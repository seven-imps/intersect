use thiserror::Error;
use veilid_core::{DHTSchema, RecordKey};

use crate::{
    debug, log,
    models::{
        Access, AccountPublic, Encrypted, EncryptionError, RecordType, Trace, ValidationError,
    },
    serialisation::{DeserialisationError, Deserialise, SerialisationError, Serialise},
    veilid::{CRYPTO_KIND, Connection, ConnectionError},
};

pub struct Intersect {
    // TODO: make this private once i don't need access to this to access the crypto system when testing anymore
    pub connection: Connection,
}

impl Intersect {
    pub async fn init() -> Result<Self, IntersectError> {
        let mut veilid = Connection::init().await?;
        // wait for the node to be fully attached before we continue
        veilid.wait_for_attachment().await;
        log!("intersect node initialised!");

        Ok(Self { connection: veilid })
    }

    // /// Gets the underlying Veilid crypto context.
    // pub fn crypto(&self) -> VeilidComponentGuard<'_, veilid_core::Crypto> {
    //     self.veilid.crypto()
    // }

    pub async fn close(self) -> () {
        self.connection.close().await;
    }

    // TODO: remove and replace with higher level abstractions.
    // this is just for testing
    pub async fn write_raw(&self, value: &[u8]) -> Result<RecordKey, IntersectError> {
        let rc = self.connection.routing_context();
        // create
        let record = rc
            .create_dht_record(CRYPTO_KIND, DHTSchema::dflt(1).unwrap(), None)
            .await
            .unwrap();
        // write subkey
        let secret = record.key().encryption_key();
        debug!("writing record with secret {:?}", secret);
        rc.set_dht_value(record.key(), 0, value.to_vec(), None)
            .await
            .unwrap();
        debug!("wrote record with key {}", record.key());
        Ok(record.key())
    }

    pub async fn write(&self, value: &Encrypted) -> Result<RecordKey, IntersectError> {
        let serialised = value.serialise()?;
        self.write_raw(&serialised).await
    }

    // TODO: remove and replace with higher level abstractions.
    // this is just for testing
    pub async fn read_raw(&self, key: RecordKey) -> Result<Vec<u8>, IntersectError> {
        let rc = self.connection.routing_context();
        // open
        let record = rc.open_dht_record(key, None).await.unwrap();
        // read subkey
        let data = rc
            .get_dht_value(record.key(), 0, true)
            .await
            .unwrap()
            .unwrap();

        Ok(data.data().to_vec())
    }

    pub async fn read(&self, key: RecordKey) -> Result<Encrypted, IntersectError> {
        let data = self.read_raw(key).await?;
        let encrypted = Encrypted::deserialise(&data)?;
        Ok(encrypted)
    }

    pub async fn create_account(&self, public: AccountPublic) -> Result<Trace, IntersectError> {
        let serialised = public.serialise()?;
        debug!(
            "serialised value ({} bytes): {}",
            serialised.len(),
            hex::encode_upper(&serialised)
        );

        let (encrypted, secret) = Encrypted::encrypt_with_random(&public, &self.connection)?;

        let key = self.write(&encrypted).await?;
        let access = Access::new_unlocked(&secret);

        let trace = Trace::new(RecordType::Account, &key, access)?;
        debug!("created account at trace: {}", trace);

        Ok(trace)
    }

    pub async fn read_account(&self, trace: Trace) -> Result<AccountPublic, IntersectError> {
        let key = trace.record();
        let access = trace.access();

        let encrypted = self.read(key.clone()).await?;
        let account_public: AccountPublic = match access {
            Access::Locked => todo!(),
            Access::Unlocked { secret } => encrypted.decrypt(secret, &self.connection)?,
            Access::Protected {
                protected_secret: _,
            } => todo!(),
        };

        Ok(account_public)
    }
}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum IntersectError {
    #[error("connection error: {0}")]
    ConnectionError(#[from] ConnectionError),

    #[error("serialisation error: {0}")]
    SerialisationError(#[from] SerialisationError),

    #[error("deserialisation error: {0}")]
    DeserialisationError(#[from] DeserialisationError),

    #[error("encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),

    #[error("validation error: {0}")]
    ValidationError(#[from] ValidationError),
}
