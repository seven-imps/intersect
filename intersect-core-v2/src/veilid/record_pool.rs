use std::{collections::HashMap, sync::Mutex};

use thiserror::Error;
use veilid_core::{
    DHTRecordDescriptor, DHTSchema, DHTSchemaSMPLMember, KeyPair, RecordKey, SetDHTValueOptions,
    SharedSecret,
};

use crate::{
    api::Reference,
    debug,
    models::Encrypted,
    serialisation::{DeserialisationError, Deserialise, SerialisationError, Serialise},
    veilid::{CRYPTO_KIND, Connection, with_crypto},
};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct OpenRecord {
    pub descriptor: DHTRecordDescriptor,
    pub secret: SharedSecret,
    // updates: flume::Receiver<T::Update>,
}

pub struct RecordPool {
    // mutex for interior mutability,
    // otherwise get_or_open would need `&mut self` which would make it unusable in most contexts
    open_records: Mutex<HashMap<RecordKey, OpenRecord>>,
    connection: Connection,
}

impl RecordPool {
    pub fn new(connection: Connection) -> Self {
        Self {
            open_records: Mutex::new(HashMap::new()),
            connection,
        }
    }

    pub async fn get_or_open(&self, reference: &Reference) -> Result<OpenRecord, RecordError> {
        // fast path: already open
        if let Some(record) = self.open_records.lock().unwrap().get(reference.record()) {
            return Ok(record.clone());
        }

        // slow path: open the record outside the lock (network call)
        let descriptor = self
            .connection
            .routing_context()
            .open_dht_record(reference.record().clone(), None)
            .await
            .map_err(|e| RecordError::OpenError(e.to_string()))?;

        let record = OpenRecord {
            descriptor,
            secret: reference.secret().clone(),
        };

        // use entry to avoid clobbering a concurrent insert
        // open_dht_record is idempotent so the duplicate call is harmless.
        // we just discard its result if we lost the race.
        Ok(self
            .open_records
            .lock()
            .unwrap()
            .entry(reference.record().clone())
            .or_insert(record)
            .clone())
    }

    pub async fn create(
        &self,
        identity: &KeyPair,
        num_subkeys: u16,
    ) -> Result<OpenRecord, RecordError> {
        let schema = DHTSchema::smpl(
            0, // no owner subkeys
            vec![DHTSchemaSMPLMember {
                m_key: self.connection.generate_member_id(&identity.key()).value(),
                m_cnt: num_subkeys, // only writer subkeys
            }],
        )
        .map_err(|e| RecordError::SchemaError(e.to_string()))?;

        let descriptor = self
            .connection
            .routing_context()
            .create_dht_record(CRYPTO_KIND, schema, None)
            .await
            .map_err(|e| RecordError::CreateError(e.to_string()))?;

        let record = OpenRecord {
            descriptor: descriptor.clone(),
            secret: with_crypto(|c| c.random_shared_secret()),
        };

        // grab the lock as late as possible to avoid blocking while doing network operations
        self.open_records
            .lock()
            .unwrap()
            .insert(descriptor.key(), record.clone());

        Ok(record)
    }

    pub async fn read_raw(
        &self,
        reference: &Reference,
        subkey: u32,
    ) -> Result<Vec<u8>, RecordError> {
        let record = self.get_or_open(reference).await?;
        let data = self
            .connection
            .routing_context()
            .get_dht_value(record.descriptor.key(), subkey, true)
            .await
            .map_err(|e| RecordError::ReadError(e.to_string()))?
            .ok_or(RecordError::SubkeyEmpty(subkey))?;
        debug!("read from record with key {}", record.descriptor.key());
        Ok(data.data().to_vec())
    }

    pub async fn read(&self, reference: &Reference, subkey: u32) -> Result<Encrypted, RecordError> {
        let data = self.read_raw(reference, subkey).await?;
        let encrypted = Encrypted::deserialise(&data)?;
        Ok(encrypted)
    }

    pub async fn write_raw(
        &self,
        reference: &Reference,
        subkey: u32,
        value: &[u8],
        writer: &KeyPair,
    ) -> Result<(), RecordError> {
        let record = self.get_or_open(reference).await?;
        self.connection
            .routing_context()
            .set_dht_value(
                record.descriptor.key(),
                subkey,
                value.to_vec(),
                Some(SetDHTValueOptions {
                    writer: Some(writer.clone()),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| RecordError::WriteError(e.to_string()))?;
        debug!("wrote record with key {}", record.descriptor.key());
        Ok(())
    }

    pub async fn watch(&self, reference: &Reference) -> Result<(), RecordError> {
        let record = self.get_or_open(reference).await?;
        self.connection
            .routing_context()
            .watch_dht_values(record.descriptor.key(), None, None, None)
            .await
            .map_err(|e| RecordError::WatchError(e.to_string()))?;
        Ok(())
    }

    pub async fn cancel_watch(&self, reference: &Reference) -> Result<(), RecordError> {
        let record = self.get_or_open(reference).await?;
        self.connection
            .routing_context()
            .cancel_dht_watch(record.descriptor.key(), None)
            .await
            .map_err(|e| RecordError::WatchError(e.to_string()))?;
        Ok(())
    }

    pub async fn write(
        &self,
        reference: &Reference,
        subkey: u32,
        value: &Encrypted,
        writer: &KeyPair,
    ) -> Result<(), RecordError> {
        let serialised = value.serialise()?;
        self.write_raw(reference, subkey, &serialised, writer).await
    }
}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum RecordError {
    #[error("failed to open record: {0}")]
    OpenError(String),

    #[error("failed to create record: {0}")]
    CreateError(String),

    #[error("failed to build schema: {0}")]
    SchemaError(String),

    #[error("failed to read record: {0}")]
    ReadError(String),

    #[error("failed to write record: {0}")]
    WriteError(String),

    #[error("failed to watch record: {0}")]
    WatchError(String),

    #[error("subkey {0} has no value")]
    SubkeyEmpty(u32),

    #[error("serialisation error: {0}")]
    SerialisationError(#[from] SerialisationError),

    #[error("deserialisation error: {0}")]
    DeserialisationError(#[from] DeserialisationError),
}
