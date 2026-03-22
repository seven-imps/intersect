use std::sync::{Arc, Mutex};

use thiserror::Error;
use tokio::sync::watch;
use veilid_core::KeyPair;

use crate::{
    api::{Document, DocumentError, TypedReference},
    documents::{AccountDocument, AccountView},
    models::{AccountPrivate, AccountPublic, EncryptionError, Trace, ValidationError},
    serialisation::{DeserialisationError, SerialisationError},
    veilid::{
        Connection, ConnectionError, ConnectionParams, RecordError, RecordPool, WatchCoordinators,
        WatchRouter, with_crypto,
    },
};

pub struct Intersect {
    connection: Connection,
    pool: Arc<RecordPool>,
    identity: Arc<Mutex<Option<KeyPair>>>,
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

    pub fn login(&self, identity: KeyPair) -> Result<(), IntersectError> {
        let is_valid = with_crypto(|c| c.validate_keypair(&identity.key(), &identity.secret()))
            .map_err(|_| IntersectError::InvalidLogin)?;
        if !is_valid {
            return Err(IntersectError::InvalidLogin);
        }
        *self.identity.lock().unwrap() = Some(identity);
        Ok(())
    }

    pub fn identity(&self) -> Option<KeyPair> {
        self.identity.lock().unwrap().clone()
    }

    pub async fn open<D: Document>(
        &self,
        typed_ref: &TypedReference<D>,
    ) -> Result<
        (
            TypedReference<D>,
            watch::Receiver<Result<D::View, DocumentError>>,
        ),
        IntersectError,
    > {
        let reference = &typed_ref.reference;
        let identity = self.identity();

        // immutable — single read, tx dropped immediately so changed() returns Err right away.
        // caller reads the value via borrow().
        if !D::MUTABLE {
            let view = D::read(reference, identity.as_ref(), &self.pool).await?;
            let (tx, rx) = watch::channel(Ok(view));
            drop(tx);
            return Ok((TypedReference::new(reference.clone()), rx));
        }

        // if a coordinator is already running for this record, subscribe for free —
        // the receiver starts with the latest cached view, no read needed
        if let Some(rx) = self.coordinators.try_subscribe::<D>(reference.record()) {
            return Ok((TypedReference::new(reference.clone()), rx));
        }

        // first open for this record — do the initial read, then create the coordinator
        let initial = D::read(reference, identity.as_ref(), &self.pool).await?;
        self.pool.watch(reference).await?;
        let notify_rx = self.watch_router.subscribe(reference.record().clone());

        let rx = self.coordinators.create::<D>(
            reference.clone(),
            initial,
            Arc::clone(&self.pool),
            identity,
            notify_rx,
            Arc::clone(&self.watch_router),
        );

        Ok((TypedReference::new(reference.clone()), rx))
    }

    pub async fn update<D: Document>(
        &self,
        typed_ref: &TypedReference<D>,
        update: D::Update,
    ) -> Result<(), IntersectError> {
        if !D::MUTABLE {
            return Err(DocumentError::NotMutable)?;
        }
        let identity = self.identity().ok_or(IntersectError::InvalidLogin)?;
        D::update(update, &typed_ref.reference, &identity, &self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn create_account(
        &self,
        name: Option<String>,
        bio: Option<String>,
        home: Option<Trace>,
    ) -> Result<TypedReference<AccountDocument>, IntersectError> {
        let identity = self.identity().ok_or(IntersectError::InvalidLogin)?;
        let public = AccountPublic::new(identity.key(), name, bio, home)?;
        let private = AccountPrivate::new(identity.secret(), None).unwrap();
        let view = AccountView {
            public,
            private: Some(private),
        };
        AccountDocument::create(&view, &identity, &self.pool)
            .await
            .map_err(Into::into)
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
}
