use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex, time::Duration};

use thiserror::Error;
use tokio::sync::watch;
use veilid_core::{
    DHTRecordDescriptor, DHTReportScope, DHTSchema, DHTSchemaSMPLMember, KeyPair, RecordKey,
    SetDHTValueOptions,
};

use crate::{
    api::Reference,
    debug,
    models::Encrypted,
    serialisation::{DeserialisationError, Deserialise, SerialisationError, Serialise},
    veilid::{CRYPTO_KIND, Connection, ConnectionError, PendingSync, with_crypto},
};

const PENDING_SYNC_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct OpenRecord {
    descriptor: DHTRecordDescriptor,
    reference: Reference,
    // updates: flume::Receiver<T::Update>,
}

impl OpenRecord {
    pub fn descriptor(&self) -> &DHTRecordDescriptor {
        &self.descriptor
    }

    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn key(&self) -> RecordKey {
        self.descriptor.key()
    }
}

pub struct RecordPool {
    // mutex for interior mutability,
    // otherwise get_or_open would need `&mut self` which would make it unusable in most contexts
    open_records: Mutex<HashMap<RecordKey, OpenRecord>>,
    connection: Connection,
    pending_sync_tx: watch::Sender<PendingSync>,
}

impl RecordPool {
    pub fn new(connection: Connection) -> Arc<Self> {
        let (pending_sync_tx, _) = watch::channel(PendingSync::default());
        let pool = Arc::new(Self {
            open_records: Mutex::new(HashMap::new()),
            connection,
            pending_sync_tx,
        });
        // poll offline subkeys across all open records and broadcast the total.
        // uses a weak ref so the task exits naturally when the pool is dropped.
        let weak = Arc::downgrade(&pool);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(PENDING_SYNC_POLL_INTERVAL).await;
                let Some(pool) = weak.upgrade() else { break };
                let keys: Vec<RecordKey> =
                    pool.open_records.lock().unwrap().keys().cloned().collect();
                let mut pending = PendingSync::default();
                // break if we don't have a routing context so we don't panic
                // if this loses a race during shutdown.
                let Ok(ctx) = pool.connection.routing_context() else {
                    break;
                };
                // TODO: we may want to add a dirty flag to open records to avoid unnecessary checks.
                // they're pretty cheap so probably ok for now, but it'd be much cleaner to avoid inspect calls if possible
                for key in keys {
                    if let Ok(report) = ctx
                        .inspect_dht_record(key, None, DHTReportScope::Local)
                        .await
                    {
                        let offline = report.offline_subkeys().len() as usize;
                        if offline > 0 {
                            pending.records += 1;
                            pending.subkeys += offline;
                        }
                    }
                }
                pool.pending_sync_tx.send_replace(pending);
            }
        });
        pool
    }

    /// returns a receiver that tracks total offline subkeys across all open records.
    /// updates approximately every 250ms.
    pub fn pending_sync_watch(&self) -> watch::Receiver<PendingSync> {
        self.pending_sync_tx.subscribe()
    }

    pub async fn get_or_open(&self, reference: &Reference) -> Result<OpenRecord, RecordError> {
        // fast path: already open
        if let Some(record) = self.open_records.lock().unwrap().get(reference.record()) {
            return Ok(record.clone());
        }

        // slow path: open the record outside the lock (network call)
        let descriptor = self
            .connection
            .routing_context()?
            .open_dht_record(reference.record().clone(), None)
            .await
            .map_err(|e| RecordError::OpenError(e.to_string()))?;

        let record = OpenRecord {
            reference: reference.clone(),
            descriptor,
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
            .routing_context()?
            .create_dht_record(CRYPTO_KIND, schema, None)
            .await
            .map_err(|e| RecordError::CreateError(e.to_string()))?;

        let key = descriptor.key();
        let secret = with_crypto(|c| c.random_shared_secret());
        let record = OpenRecord {
            reference: Reference::new(key.clone(), secret),
            descriptor,
        };

        // grab the lock as late as possible to avoid blocking while doing network operations
        self.open_records
            .lock()
            .unwrap()
            .insert(key, record.clone());

        Ok(record)
    }

    pub async fn read_raw(
        &self,
        reference: &Reference,
        subkey: u32,
        force: bool,
    ) -> Result<Vec<u8>, RecordError> {
        let record = self.get_or_open(reference).await?;
        let data = self
            .connection
            .routing_context()?
            .get_dht_value(record.descriptor.key(), subkey, force)
            .await
            .map_err(|e| RecordError::ReadError(e.to_string()))?
            .ok_or(RecordError::SubkeyEmpty(subkey))?;
        debug!("read from record with key {}", record.descriptor.key());
        Ok(data.data().to_vec())
    }

    /// read a subkey on a given record
    /// if `force` is true, will force a network refresh, bypassing local cache
    pub async fn read(
        &self,
        reference: &Reference,
        subkey: u32,
        force: bool,
    ) -> Result<Encrypted, RecordError> {
        let data = self.read_raw(reference, subkey, force).await?;
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
            .routing_context()?
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
            .routing_context()?
            .watch_dht_values(record.descriptor.key(), None, None, None)
            .await
            .map_err(|e| RecordError::WatchError(e.to_string()))?;
        Ok(())
    }

    pub async fn cancel_watch(&self, reference: &Reference) -> Result<(), RecordError> {
        let record = self.get_or_open(reference).await?;
        self.connection
            .routing_context()?
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

    /// waits until all offline subkeys across all open records have been flushed to the network.
    pub async fn wait_for_all_pending(&self) {
        let mut rx = self.pending_sync_tx.subscribe();
        // wait_for checks the current value first, so no race if already synced
        let _ = rx.wait_for(|p| p.subkeys == 0).await;
    }

    /// waits until all pending subkeys on a record have been flushed to the network.
    pub async fn wait_for_pending(&self, reference: &Reference) -> Result<(), RecordError> {
        let record = self.get_or_open(reference).await?;
        loop {
            let report = self
                .connection
                .routing_context()?
                .inspect_dht_record(record.key(), None, DHTReportScope::Local)
                .await
                .map_err(|e| RecordError::ReadError(e.to_string()))?;
            if report.offline_subkeys().is_empty() {
                return Ok(());
            }
            debug!(
                "waiting for record with key {} to sync, {} subkeys still offline",
                record.key(),
                report.offline_subkeys().len()
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
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

    #[error("{0}")]
    ConnectionError(#[from] ConnectionError),
}
