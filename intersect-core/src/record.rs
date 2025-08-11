use futures::future::join_all;
use itertools::Itertools;
use thiserror::Error;
use veilid_core::{
    DHTRecordDescriptor, DHTReportScope, DHTSchema, DHTSchemaSMPLMember, KeyPair, PublicKey,
    SetDHTValueOptions, ValueSubkey, ValueSubkeyRangeSet, VeilidAPIError,
};

use crate::{
    log,
    models::{Encrypted, EncryptionError},
    veilid::{get_routing_context, CRYPTO_KIND},
    Hash, Identity, PrivateKey, Shard, VeilidRecordKey,
};

// don't cache anything here!
// instead rely on the built in veilid local record store
pub struct Record {
    shard: Shard,
    hash: Hash,
    descriptor: DHTRecordDescriptor,
}

impl Record {
    pub const MAX_SUBKEYS: ValueSubkey = 256;
    pub const SUBKEY_SIZE_BYTES: usize = 1024 * 1024 / Self::MAX_SUBKEYS as usize;

    pub async fn create(identity: &Identity, hash: &Hash) -> Result<Self, NetworkError> {
        let rc = get_routing_context().await;
        let schema = build_schema(hash);

        let descriptor = rc
            .create_dht_record(
                schema.clone(),
                Some(identity.as_keypair()),
                Some(CRYPTO_KIND),
            )
            .await
            .map_err(|e| NetworkError::RecordNotFound(e))?;

        log!("record created: {}", descriptor.key());

        Ok(Self {
            shard: identity.shard().clone(),
            hash: hash.clone(),
            descriptor,
        })
    }

    pub async fn build_key(shard: &Shard, hash: &Hash) -> VeilidRecordKey {
        let rc = get_routing_context().await;
        let schema = build_schema(hash);
        rc.get_dht_record_key(schema.clone(), shard.key(), Some(CRYPTO_KIND))
            .unwrap() // this should Never™ fail
            .value
            .into()
    }

    pub async fn open(key: &VeilidRecordKey) -> Result<Self, NetworkError> {
        let rc = get_routing_context().await;
        let key = veilid_core::TypedRecordKey::new(CRYPTO_KIND, key.into());

        log!("opening record: {}", key);
        let descriptor = rc
            .open_dht_record(key, None)
            .await
            // .map_err(|_| NetworkError::RecordNotFound)?;
            .inspect_err(|e| log!("network error: {e}"))
            .map_err(|_| NetworkError::RecordNotFound);

        // retry
        let descriptor = match descriptor {
            Ok(d) => d,
            Err(_) => {
                log!("retrying");
                rc.open_dht_record(key, None)
                    .await
                    // .map_err(|_| NetworkError::RecordNotFound)?;
                    .map_err(|e| NetworkError::RecordNotFound(e))?
            }
        };

        let shard: Shard = descriptor.owner().into();
        // this is some real "trust me bro" code
        let hash: Hash = match descriptor.schema() {
            DHTSchema::DFLT(_) => panic!(),
            DHTSchema::SMPL(schema) => Hash::from_bytes(schema.members()[0].m_key.bytes),
        };

        Ok(Self {
            shard,
            hash,
            descriptor,
        })
    }

    pub fn record_key(&self) -> VeilidRecordKey {
        // TODO: consider not ignoring the version field :p
        self.descriptor.key().value.into()
    }

    pub fn shard(&self) -> &Shard {
        &self.shard
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    pub async fn read_raw(
        &self,
        subkey: ValueSubkey,
        force_refresh: bool,
    ) -> Result<Option<Vec<u8>>, NetworkError> {
        let rc = get_routing_context().await;
        log!("reading from: [{}] {}", subkey, self.descriptor.key());
        let value = rc
            .get_dht_value(*self.descriptor.key(), subkey, force_refresh)
            .await
            .map_err(|e| NetworkError::RecordReadFailed(e))?;
        log!("done reading from: [{}] {}", subkey, self.descriptor.key());

        let data = value
            // deleted entries are empty
            // so treat them the same as a missing value
            .filter(|v| v.data_size() > 0)
            .map(|v| v.data().to_vec());
        Ok(data)
    }

    pub async fn read(
        &self,
        subkey: ValueSubkey,
        force_refresh: bool,
    ) -> Result<Option<Encrypted>, NetworkError> {
        let data = self.read_raw(subkey, force_refresh).await?;
        let encrypted = data.map(|data| Encrypted::from_bytes(&data)).transpose()?;
        Ok(encrypted)
    }

    pub async fn read_many_raw(
        &self,
        subkeys: impl IntoIterator<Item = ValueSubkey>,
        force_refresh: bool,
    ) -> Result<Vec<(ValueSubkey, Option<Vec<u8>>)>, NetworkError> {
        let futures = subkeys
            .into_iter()
            .map(|subkey| async move { (subkey, self.read_raw(subkey, force_refresh).await) });

        let results = join_all(futures)
            .await
            .into_iter()
            // make sure it's sorted after joining
            .sorted_by_key(|(i, _r)| *i)
            // some janky rearranging so we can tease out the error
            .map(|(i, r)| match r {
                Ok(r) => Ok((i, r)),
                Err(e) => Err(e),
            })
            // and use this magical impl for Result to bubble it up
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub async fn write_raw(
        &self,
        data: Vec<u8>,
        subkey: ValueSubkey,
        private_key: &PrivateKey,
    ) -> Result<(), NetworkError> {
        let rc = get_routing_context().await;
        log!("writing to: [{}] {}", subkey, self.descriptor.key());
        rc.set_dht_value(
            *self.descriptor.key(),
            subkey,
            data,
            Some(SetDHTValueOptions {
                writer: Some(KeyPair::new(*self.shard.key(), *private_key.key())),
                ..Default::default()
            }),
        )
        .await
        .map_err(|e| NetworkError::RecordWriteFailed(e))?;
        log!("done writing to: [{}] {}", subkey, self.descriptor.key());
        Ok(())
    }

    pub async fn write(
        &self,
        data: Encrypted,
        subkey: ValueSubkey,
        private_key: &PrivateKey,
    ) -> Result<(), NetworkError> {
        self.write_raw(data.to_bytes().to_vec(), subkey, private_key)
            .await
    }

    pub async fn write_null(
        &self,
        subkey: ValueSubkey,
        private_key: &PrivateKey,
    ) -> Result<(), NetworkError> {
        self.write_raw(vec![], subkey, private_key).await
    }

    pub async fn write_many_raw(
        &self,
        writes: impl IntoIterator<Item = (ValueSubkey, Vec<u8>)>,
        private_key: &PrivateKey,
    ) -> Result<(), NetworkError> {
        let futures = writes
            .into_iter()
            .map(|(subkey, data)| async move { self.write_raw(data, subkey, private_key).await });

        let _ = join_all(futures)
            .await
            .into_iter()
            // and use this magical impl for Result to bubble it up
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    pub async fn is_unused(&self, subkey: ValueSubkey) -> Result<bool, NetworkError> {
        log!("checking if empty: [{}] {}", subkey, self.descriptor.key());
        Ok(self.read_raw(subkey, true).await?.is_none())
    }

    pub async fn find_unused(&self, force_refresh: bool) -> Result<Vec<ValueSubkey>, NetworkError> {
        // refresh all subkeys if desired
        if force_refresh {
            self.refresh().await?;
        }

        // and now we can grab whatever we have locally
        let rc = get_routing_context().await;
        let report = rc
            .inspect_dht_record(
                *self.descriptor.key(),
                Some(ValueSubkeyRangeSet::single_range(0, Self::MAX_SUBKEYS)),
                DHTReportScope::Local,
            )
            .await
            .map_err(|e| NetworkError::RecordInspectFailed(e))?;

        let unused_subkeys = report
            .local_seqs()
            .into_iter()
            // add the subkey index
            .enumerate()
            // only unused subkeys
            // TODO: also allow for reuse of deleted entries
            // this would require looking at the value though, and seeing if it's empty
            .filter(|(_, seq)| **seq == None)
            // and only keep the index
            .map(|(i, _)| i as ValueSubkey)
            .collect_vec();

        Ok(unused_subkeys)
    }

    pub async fn read_all(
        &self,
        force_refresh: bool,
    ) -> Result<Vec<(ValueSubkey, Option<Encrypted>)>, NetworkError> {
        // refresh all subkeys if desired
        if force_refresh {
            self.refresh().await?;
        }

        // and now we can grab whatever we have locally
        let rc = get_routing_context().await;
        let report = rc
            .inspect_dht_record(
                *self.descriptor.key(),
                Some(ValueSubkeyRangeSet::single_range(0, Self::MAX_SUBKEYS)),
                DHTReportScope::Local,
            )
            .await
            .map_err(|e| NetworkError::RecordInspectFailed(e))?;

        let futures = report
            .local_seqs()
            .into_iter()
            // add the subkey index
            .enumerate()
            // only used subkeys
            .filter(|(_, seq)| **seq != None)
            // and read the subkey values
            .map(|(i, _)| async move { (i as ValueSubkey, self.read(i as u32, false).await) })
            .collect_vec();

        let results = join_all(futures)
            .await
            .into_iter()
            // filter out empty (deleted) entries
            .filter(|(_, result)| {
                !result
                    .as_ref()
                    .is_err_and(|e| *e == NetworkError::MissingData)
            })
            // make sure it's sorted after joining
            .sorted_by_key(|(i, _r)| *i)
            // some janky rearranging so we can tease out the error
            .map(|(i, r)| match r {
                Ok(r) => Ok((i, r)),
                Err(e) => Err(e),
            })
            // and use this magical impl for Result to bubble it up
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub async fn refresh(&self) -> Result<(), NetworkError> {
        let rc = get_routing_context().await;
        log!("inspecting record: {}", self.descriptor.key());
        let report = rc
            .inspect_dht_record(
                *self.descriptor.key(),
                Some(ValueSubkeyRangeSet::single_range(0, Self::MAX_SUBKEYS)),
                DHTReportScope::UpdateGet,
            )
            .await
            .map_err(|e| NetworkError::RecordInspectFailed(e))?;

        // find all the subkeys we need to update
        let subkeys = report
            // pair up remote and local seq numbers
            .network_seqs()
            .into_iter()
            .zip_eq(report.local_seqs())
            // and add the subkey
            .enumerate()
            .filter(|(_, (remote, local))| {
                log!("local: {:?}, remote: {:?}", local, remote);
                true
            })
            // remove any that don't have a remote value
            .filter(|(_, (remote, _))| **remote != None)
            // only keep ones that are newer than our local copy
            .filter(|(_, (remote, local))| {
                // log!("local: {}, remote: {}", local, remote);
                remote > local || **local == None
            })
            // and collect the subkey values
            .map(|(i, (_, _))| i)
            .collect_vec();

        log!("to fetch: {:?}", subkeys);

        let results = join_all(
            subkeys
                .into_iter()
                .map(|subkey| self.read(subkey as u32, true)),
        )
        .await;

        for r in results {
            let _ = r?;
        }

        Ok(())
    }

    pub async fn close(self) {
        let rc = get_routing_context().await;
        rc.close_dht_record(*self.descriptor.key()).await.unwrap();
        log!("record closed: {}", self.descriptor.key());
    }
}

#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NetworkError {
    #[error("encryption error ({0})")]
    EncryptionError(#[from] EncryptionError),

    #[error("couldn't open record ({0})")]
    RecordNotFound(VeilidAPIError),

    #[error("couldn't read record ({0})")]
    RecordReadFailed(VeilidAPIError),

    #[error("couldn't write to record ({0})")]
    RecordWriteFailed(VeilidAPIError),

    #[error("couldn't inspect record ({0})")]
    RecordInspectFailed(VeilidAPIError),

    #[error("unexpected missing data")]
    MissingData,

    #[error("invalid data")]
    InvalidData,

    #[error("no unused subkey")]
    NoUnusedSubkey,
}

fn build_schema(hash: &Hash) -> DHTSchema {
    // we're building a schema where we we use one of the writer keys as a "tag"
    // by giving it no subkeys and storing the hash of the data there instead of the public key
    // thankfully they're conveniently the same size

    DHTSchema::smpl(
        Record::MAX_SUBKEYS as u16,
        vec![
            // and unique identifier
            DHTSchemaSMPLMember {
                m_key: PublicKey::new(*hash.bytes()),
                m_cnt: 0,
            },
        ],
    )
    .unwrap()
}

// mod tests {
//     use super::*;
//     use crate::init;

//     #[test]
//     fn it_works() {
//         tokio_test::block_on(init());

//         // ...
//     }
// }
