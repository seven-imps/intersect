use std::sync::{Arc, Mutex};

use guard_clause::guard;
use thiserror::Error;
use veilid_core::KeyPair;

use crate::{
    api::{Document, DocumentError, MutableDocument, OpenDocument, TypedReference},
    documents::{AccountDocument, AccountView, FragmentDocument, FragmentView},
    models::{
        AccountBio, AccountName, AccountPrivate, AccountPublic, AccountSecret, EncryptionError,
        FragmentMime, Trace, ValidationError,
    },
    serialisation::{DeserialisationError, SerialisationError},
    veilid::{
        Connection, ConnectionError, ConnectionParams, RecordError, RecordPool, WatchCoordinators,
        WatchRouter, with_crypto,
    },
};

pub struct Intersect {
    connection: Connection,
    pool: Arc<RecordPool>,
    identity: Arc<Mutex<Option<Identity>>>,
    watch_router: Arc<WatchRouter>,
    coordinators: WatchCoordinators,
}

impl Intersect {
    pub async fn init(connection_params: ConnectionParams) -> Result<Self, IntersectError> {
        let connection = Connection::init(connection_params).await?;
        connection.wait_for_attachment().await;

        let pool = Arc::new(RecordPool::new(connection.clone()));
        let watch_router = Arc::new(WatchRouter::new());

        connection.add_update_handler(Box::new(Arc::clone(&watch_router)));

        crate::log!("intersect node initialised!");

        Ok(Self {
            connection,
            pool,
            identity: Arc::new(Mutex::new(None)),
            watch_router,
            coordinators: WatchCoordinators::new(),
        })
    }

    pub async fn close(self) {
        self.connection.close().await;
    }

    /// two-pass login: reads public key from the account record, reconstructs the keypair,
    /// then verifies the stored private key matches before setting identity.
    pub async fn login(
        &self,
        account: TypedReference<AccountDocument>,
        secret: AccountSecret,
    ) -> Result<(), IntersectError> {
        let reference = &account.reference;

        // public key can't be derived from the reference alone, so read it from the record first
        let public_view = AccountDocument::read(reference, None, true, &self.pool).await?;
        let public_key = public_view.public.public_key;

        // reconstruct and validate the keypair
        guard!(
            secret.as_ref().kind() == public_key.kind(),
            Err(IntersectError::InvalidLogin)
        );
        let keypair = KeyPair::new_from_parts(public_key, secret.as_ref().value());
        let is_valid = with_crypto(|c| c.validate_keypair(&keypair.key(), &keypair.secret()))
            .map_err(|_| IntersectError::InvalidLogin)?;
        if !is_valid {
            return Err(IntersectError::InvalidLogin);
        }

        // second pass: read with identity to verify stored private key matches
        let full_view = AccountDocument::read(reference, Some(&keypair), false, &self.pool).await?;
        let private = full_view.private.ok_or(IntersectError::InvalidLogin)?;
        if private.private_key() != &keypair.secret() {
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
        // always force — immutable implementations ignore this and use cache internally anyway
        D::read(&typed_ref.reference, identity.as_ref(), true, &self.pool)
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
        let reference = &typed_ref.reference;
        let identity = self.identity();

        // if a coordinator is already running for this record, subscribe for free
        // the receiver starts with the latest cached view, no read needed
        if let Some(updates) = self.coordinators.try_subscribe::<D>(reference.record()) {
            return Ok(OpenDocument {
                reference: typed_ref.clone(),
                updates,
            });
        }

        // first open for this record. do the initial read, then create the coordinator
        let initial = D::read(reference, identity.as_ref(), false, &self.pool).await?;
        self.pool.watch(reference).await?;
        let notify_rx = self.watch_router.subscribe(reference.record().clone());

        let updates = self.coordinators.create::<D>(
            reference.clone(),
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

    /// upload a fragment with a given mimetype to the network
    pub async fn create_fragment(
        &self,
        data: Vec<u8>,
        mime: FragmentMime,
    ) -> Result<TypedReference<FragmentDocument>, IntersectError> {
        let identity = self.identity().ok_or(IntersectError::InvalidLogin)?;
        let view = FragmentView::new(data, mime);
        FragmentDocument::create(view, &identity, &self.pool)
            .await
            .map_err(Into::into)
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
        let public = AccountPublic::new(
            keypair.key(),
            name.map(AccountName::new).transpose()?,
            bio.map(AccountBio::new).transpose()?,
            home,
        );
        let private = AccountPrivate::new(keypair.secret(), None);
        let view = AccountView {
            public,
            private: Some(private),
        };
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
