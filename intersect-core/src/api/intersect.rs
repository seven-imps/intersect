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
    identity: Arc<Mutex<Option<Identity>>>,
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

        // only attach aftter setting up all the watchers so we avoid potential missed events or races
        connection.attach().await?;

        crate::log!("intersect node initialised!");

        Ok(Self {
            connection,
            pool,
            identity: Arc::new(Mutex::new(None)),
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
    /// and validates it before setting identity.
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

        *self.identity.lock().unwrap() = Some(Identity::Account { keypair, account });
        Ok(())
    }

    pub fn logout(&self) {
        *self.identity.lock().unwrap() = None;
        // TODO: cancel all active watch coordinators on logout
    }

    fn identity(&self) -> Option<KeyPair> {
        self.identity
            .lock()
            .unwrap()
            .as_ref()
            .map(|i| i.keypair().clone())
    }

    pub fn account(&self) -> Option<TypedReference<AccountDocument>> {
        self.identity
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|i| i.account().cloned())
    }

    /// one-time document retrieval guaranteed to return the most recent version on the network
    pub async fn fetch<D: Document>(
        &self,
        typed_ref: &TypedReference<D>,
    ) -> Result<D::View, IntersectError> {
        let identity = self.identity();
        // always force. immutable implementations ignore this and use cache internally anyway
        D::read(typed_ref, identity.as_ref(), true, &self.pool)
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
        let identity = self.identity();

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
        let initial = D::read(typed_ref, identity.as_ref(), false, &self.pool).await?;
        self.pool.watch(typed_ref.reference()).await?;
        let notify_rx = self
            .watch_router
            .subscribe(typed_ref.reference().record().clone());

        let updates = self.coordinators.create::<D>(
            typed_ref.clone(),
            initial,
            Arc::clone(&self.pool),
            identity,
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
        let identity = self.identity().ok_or(IntersectError::InvalidLogin)?;
        D::update(update, doc, &identity, &self.pool)
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
        let identity = self.identity().ok_or(IntersectError::InvalidLogin)?;
        // convert the account reference to an unlocked trace so the reader can follow it
        let author = self.account().map(|r| r.to_unlocked_trace());
        let view = IndexView::new(IndexName::new(name)?, author, fragment, links);
        Ok(IndexDocument::create(view, &identity, &self.pool).await?)
    }

    /// upload a fragment with a given mimetype to the network
    pub async fn create_fragment(
        &self,
        data: Vec<u8>,
        mime: FragmentMime,
    ) -> Result<TypedReference<FragmentDocument>, IntersectError> {
        let identity = self.identity().ok_or(IntersectError::InvalidLogin)?;
        let view = FragmentView::new(data, mime);
        Ok(FragmentDocument::create(view, &identity, &self.pool).await?)
    }

    /// generates a fresh ephemeral keypair and sets it as the current identity.
    /// allows signing/creating things without a persistent account record.
    pub fn login_anonymous(&self) -> Result<(), IntersectError> {
        if self.account().is_some() {
            return Err(IntersectError::AlreadyLoggedIn);
        }
        let keypair = with_crypto(|c| c.generate_keypair());
        *self.identity.lock().unwrap() = Some(Identity::Anonymous(keypair));
        Ok(())
    }

    /// creates a new account, generating a keypair internally.
    /// returns the account reference and the secret key (save it to log in later).
    /// errors if an account (non-anonymous) is already logged in.
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
        *self.identity.lock().unwrap() = Some(Identity::Account {
            keypair,
            account: reference.clone(),
        });
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

#[derive(Clone)]
enum Identity {
    Anonymous(KeyPair),
    Account {
        keypair: KeyPair,
        account: TypedReference<AccountDocument>,
    },
}

impl Identity {
    fn keypair(&self) -> &KeyPair {
        match self {
            Identity::Anonymous(keypair) => keypair,
            Identity::Account { keypair, .. } => keypair,
        }
    }

    fn account(&self) -> Option<&TypedReference<AccountDocument>> {
        match self {
            Identity::Anonymous(_) => None,
            Identity::Account { account, .. } => Some(account),
        }
    }
}
