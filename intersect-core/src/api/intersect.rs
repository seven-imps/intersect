use std::sync::{Arc, Mutex};

use guard_clause::guard;
use thiserror::Error;
use veilid_core::KeyPair;

use tokio::sync::watch;

use crate::{
    api::{Document, DocumentError, MutableDocument, OpenDocument, TypedReference},
    documents::{
        AccountDocument, AccountView, FragmentDocument, FragmentView, IndexDocument, IndexView,
    },
    models::{
        AccountBio, AccountName, AccountPrivate, AccountPublicKey, AccountSecret, EncryptionError,
        FragmentMime, IndexName, Trace, ValidationError,
    },
    serialisation::{DeserialisationError, SerialisationError},
    veilid::{
        Connection, ConnectionError, ConnectionParams, NetworkState, RecordError, RecordPool,
        WatchCoordinators, WatchRouter, watch_network_state, with_crypto,
    },
};

// derive Clone for Intersect. everything inside of it is already Arc internally or explicitly
#[derive(Clone)]
pub struct Intersect {
    connection: Connection,
    pool: Arc<RecordPool>,
    // keypair for signing.
    // set to the account keypair if logged in,
    // otherwise set to an ephemeral anonymous keypair
    keypair: Arc<Mutex<KeyPair>>,
    // None = anonymous session, Some = logged in with a persistent account
    account_tx: Arc<watch::Sender<Option<TypedReference<AccountDocument>>>>,
    watch_router: Arc<WatchRouter>,
    coordinators: WatchCoordinators,
    network_state_rx: watch::Receiver<NetworkState>,
}

impl Intersect {
    pub async fn init(connection_params: ConnectionParams) -> Result<Self, IntersectError> {
        let connection = Connection::init(connection_params).await?;

        let pool = RecordPool::new(connection.clone());
        let watch_router = Arc::new(WatchRouter::new());
        connection.add_update_handler(Box::new(Arc::clone(&watch_router)));

        let network_state_rx = watch_network_state(
            connection.attachment_state(),
            connection.network_state(),
            pool.pending_sync_watch(),
        );

        // only attach after setting up all the watchers so we avoid potential missed events or races
        connection.attach().await?;

        let keypair = with_crypto(|c| c.generate_keypair());
        let (account_tx, _) = watch::channel(None);

        crate::log!("intersect node initialised!");

        Ok(Self {
            connection,
            pool,
            keypair: Arc::new(Mutex::new(keypair)),
            account_tx: Arc::new(account_tx),
            watch_router,
            coordinators: WatchCoordinators::new(),
            network_state_rx,
        })
    }

    /// returns a receiver for the combined network state.
    /// clone it to get independent receivers
    pub fn network_watch(&self) -> watch::Receiver<NetworkState> {
        self.network_state_rx.clone()
    }

    /// waits until the node is attached and ready for public internet use.
    /// call this before performing network operations.
    pub async fn wait_for_attachment(&self) {
        self.connection.wait_for_attachment().await;
    }

    pub async fn close(self) {
        // drain pending writes before disconnecting
        self.pool.wait_for_all_pending().await;
        self.connection.close().await;
    }

    /// reads the public key from the account record,
    /// reconstructs the keypair from the provided secret,
    /// and validates it before setting the session keypair and account.
    pub async fn login(
        &self,
        account: TypedReference<AccountDocument>,
        secret: AccountSecret,
    ) -> Result<(), IntersectError> {
        // public key can't be derived from the reference alone, so read it from the record first
        let view = AccountDocument::read(&account, None, true, &self.pool).await?;
        let public_key = view.public_key().clone();

        // reconstruct and validate the keypair
        guard!(
            secret.inner().kind() == public_key.inner().kind(),
            Err(IntersectError::InvalidLogin)
        );
        let keypair = KeyPair::new_from_parts(public_key.inner().clone(), secret.inner().value());
        let is_valid = with_crypto(|c| c.validate_keypair(&keypair.key(), &keypair.secret()))
            .map_err(|_| IntersectError::InvalidLogin)?;
        if !is_valid {
            return Err(IntersectError::InvalidLogin);
        }

        // keypair first then account, so any watches of account don't potentially see a stale keypair
        *self.keypair.lock().unwrap() = keypair;
        self.account_tx.send_modify(|a| *a = Some(account));
        Ok(())
    }

    pub fn logout(&self) {
        // generate a fresh ephemeral keypair to replace the account keypair
        *self.keypair.lock().unwrap() = with_crypto(|c| c.generate_keypair());
        self.account_tx.send_modify(|a| *a = None);
        // TODO: cancel all active watch coordinators on logout
    }

    fn keypair(&self) -> KeyPair {
        self.keypair.lock().unwrap().clone()
    }

    pub fn account(&self) -> Option<TypedReference<AccountDocument>> {
        self.account_tx.borrow().clone()
    }

    /// returns a receiver for the current account state.
    /// None = anonymous session, Some = logged in with a persistent account.
    pub fn account_watch(&self) -> watch::Receiver<Option<TypedReference<AccountDocument>>> {
        self.account_tx.subscribe()
    }

    /// one-time document retrieval guaranteed to return the most recent version on the network
    pub async fn fetch<D: Document>(
        &self,
        typed_ref: &TypedReference<D>,
    ) -> Result<D::View, IntersectError> {
        let keypair = self.keypair();
        // always force. immutable implementations ignore this and use cache internally anyway
        D::read(typed_ref, Some(&keypair), true, &self.pool)
            .await
            .map_err(Into::into)
    }

    /// document retrieval with background watch
    /// initial return may be stale local cache, but will continually return newer versions to the receiver
    /// (usually faster than `fetch` if you don't need the most up-to-date version right away)
    pub async fn open<D: MutableDocument>(
        &self,
        typed_ref: &TypedReference<D>,
    ) -> Result<OpenDocument<D>, IntersectError> {
        let keypair = self.keypair();

        // if a coordinator is already running for this record, subscribe for free
        // the receiver starts with the latest cached view, no read needed
        if let Some(updates) = self
            .coordinators
            .try_subscribe::<D>(typed_ref.reference().record())
        {
            return Ok(OpenDocument {
                reference: typed_ref.clone(),
                updates,
            });
        }

        // first open for this record. do the initial read, then create the coordinator
        let initial = D::read(typed_ref, Some(&keypair), false, &self.pool).await?;
        self.pool.watch(typed_ref.reference()).await?;
        let notify_rx = self
            .watch_router
            .subscribe(typed_ref.reference().record().clone());

        let updates = self.coordinators.create::<D>(
            typed_ref.clone(),
            initial,
            Arc::clone(&self.pool),
            Some(keypair),
            notify_rx,
            Arc::clone(&self.watch_router),
        );

        Ok(OpenDocument {
            reference: typed_ref.clone(),
            updates,
        })
    }

    pub async fn update<D: MutableDocument>(
        &self,
        doc: &OpenDocument<D>,
        update: D::Update,
    ) -> Result<(), IntersectError> {
        let keypair = self.keypair();
        D::update(update, doc, &keypair, &self.pool)
            .await
            .map_err(Into::into)
    }

    /// create a new index document.
    /// the author account is automatically pulled from the current session (absent for anon logins).
    pub async fn create_index(
        &self,
        name: String,
        fragment: Option<Trace>,
        links: Option<Trace>,
    ) -> Result<TypedReference<IndexDocument>, IntersectError> {
        let keypair = self.keypair();
        // convert the account reference to an unlocked trace so the reader can follow it
        let author = self.account().map(|r| r.to_unlocked_trace());
        let view = IndexView::new(IndexName::new(name)?, author, fragment, links);
        Ok(IndexDocument::create(view, &keypair, &self.pool).await?)
    }

    /// upload a fragment with a given mimetype to the network
    pub async fn create_fragment(
        &self,
        data: Vec<u8>,
        mime: FragmentMime,
    ) -> Result<TypedReference<FragmentDocument>, IntersectError> {
        let keypair = self.keypair();
        let view = FragmentView::new(data, mime);
        Ok(FragmentDocument::create(view, &keypair, &self.pool).await?)
    }

    /// creates a new account, generating a keypair internally.
    /// returns the account reference and the secret key (save it to log in later).
    /// errors if already logged in with a persistent account.
    pub async fn create_account(
        &self,
        name: Option<String>,
        bio: Option<String>,
        home: Option<Trace>,
    ) -> Result<(TypedReference<AccountDocument>, AccountSecret), IntersectError> {
        if self.account().is_some() {
            return Err(IntersectError::AlreadyLoggedIn);
        }
        let keypair = with_crypto(|c| c.generate_keypair());
        let private = AccountPrivate::new(None);
        let view = AccountView::new(
            AccountPublicKey::new(keypair.key()),
            name.map(AccountName::new).transpose()?,
            bio.map(AccountBio::new).transpose()?,
            home,
            Some(private),
        );
        let reference = AccountDocument::create(view, &keypair, &self.pool).await?;
        let secret = AccountSecret::new(keypair.secret());
        *self.keypair.lock().unwrap() = keypair;
        self.account_tx
            .send_modify(|a| *a = Some(reference.clone()));
        Ok((reference, secret))
    }
}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum IntersectError {
    #[error("connection error: {0}")]
    ConnectionError(#[from] ConnectionError),

    #[error("document error: {0}")]
    DocumentError(#[from] DocumentError),

    #[error("serialisation error: {0}")]
    SerialisationError(#[from] SerialisationError),

    #[error("deserialisation error: {0}")]
    DeserialisationError(#[from] DeserialisationError),

    #[error("encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),

    #[error("validation error: {0}")]
    ValidationError(#[from] ValidationError),

    #[error("record error: {0}")]
    RecordError(#[from] RecordError),

    #[error("invalid login")]
    InvalidLogin,

    #[error("already logged in")]
    AlreadyLoggedIn,
}
