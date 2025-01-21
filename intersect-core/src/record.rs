use futures::future::join_all;
use itertools::Itertools;
use thiserror::Error;
use veilid_core::{
    DHTRecordDescriptor, DHTReportScope, DHTSchema, DHTSchemaSMPLMember, KeyPair, ValueSubkey,
    ValueSubkeyRangeSet, VeilidAPIError,
};

use crate::{
    log,
    models::{Encrypted, EncryptionError, Reference},
    veilid::{get_routing_context, CRYPTO_KIND},
    Domain, Hash, Identity, PrivateKey, Shard, VeilidRecordKey,
};

pub static MAX_SUBKEYS: ValueSubkey = 256;
pub static SUBKEY_SIZE_BYTES: usize = 1024 * 1024 / MAX_SUBKEYS as usize;

// don't cache anything here!
// instead rely on the built in veilid local record store
pub struct Record {
    shard: Shard,
    hash: Hash,
    descriptor: DHTRecordDescriptor,
}

impl Record {
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
            .await
            .unwrap() // this should Never™ fail
            .value
            .into()
    }

    pub async fn open_key(key: &VeilidRecordKey) -> Result<Self, NetworkError> {
        let rc = get_routing_context().await;
        let key = veilid_core::TypedKey::new(CRYPTO_KIND, key.into());

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
            DHTSchema::SMPL(schema) => schema.members()[0].m_key.into(),
        };

        Ok(Self {
            shard,
            hash,
            descriptor,
        })
    }

    pub async fn open<D: Domain>(reference: &Reference<D>) -> Result<Self, NetworkError> {
        let key = Self::build_key(reference.shard(), reference.hash()).await;
        Self::open_key(&key).await
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

    async fn read_key_raw(
        &self,
        subkey: ValueSubkey,
        force_refresh: bool,
    ) -> Result<Vec<u8>, NetworkError> {
        let rc = get_routing_context().await;
        log!("reading from: [{}] {}", subkey, self.descriptor.key());
        let value = rc
            .get_dht_value(*self.descriptor.key(), subkey, force_refresh)
            .await
            .map_err(|e| NetworkError::RecordReadFailed(e))?
            .ok_or(NetworkError::MissingData)?;
        // deleted entries are empty
        // so treat them the same as a missing value
        if value.data_size() == 0 {
            Err(NetworkError::MissingData)?
        }
        log!("done reading from: [{}] {}", subkey, self.descriptor.key());
        Ok(value.data().to_vec())
    }

    pub async fn read_key(
        &self,
        subkey: ValueSubkey,
        force_refresh: bool,
    ) -> Result<Encrypted, NetworkError> {
        let rc = get_routing_context().await;
        log!("reading from: [{}] {}", subkey, self.descriptor.key());
        let value = rc
            .get_dht_value(*self.descriptor.key(), subkey, force_refresh)
            .await
            .map_err(|e| NetworkError::RecordReadFailed(e))?
            .ok_or(NetworkError::MissingData)?;
        // deleted entries are empty
        // so treat them the same as a missing value
        if value.data_size() == 0 {
            Err(NetworkError::MissingData)?
        }
        let data = Encrypted::from_bytes(value.data())?;
        log!("done reading from: [{}] {}", subkey, self.descriptor.key());
        Ok(data)
    }

    pub async fn is_unused(&self, subkey: ValueSubkey) -> Result<bool, NetworkError> {
        let rc = get_routing_context().await;
        log!("checking if empty: [{}] {}", subkey, self.descriptor.key());
        let value = rc
            .get_dht_value(*self.descriptor.key(), subkey, true)
            .await
            .map_err(|e| NetworkError::RecordReadFailed(e))?;

        Ok(value.is_none())
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
                ValueSubkeyRangeSet::single_range(0, MAX_SUBKEYS),
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
            .filter(|(_, seq)| **seq == ValueSubkey::MAX)
            // and only keep the index
            .map(|(i, _)| i as ValueSubkey)
            .collect_vec();

        Ok(unused_subkeys)
    }

    pub async fn read_all(
        &self,
        force_refresh: bool,
    ) -> Result<Vec<(ValueSubkey, Encrypted)>, NetworkError> {
        // refresh all subkeys if desired
        if force_refresh {
            self.refresh().await?;
        }

        // and now we can grab whatever we have locally
        let rc = get_routing_context().await;
        let report = rc
            .inspect_dht_record(
                *self.descriptor.key(),
                ValueSubkeyRangeSet::single_range(0, MAX_SUBKEYS),
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
            .filter(|(_, seq)| **seq != ValueSubkey::MAX)
            // and read the subkey values
            .map(|(i, _)| async move { (i as ValueSubkey, self.read_key(i as u32, false).await) })
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

    pub async fn write_key(
        &self,
        data: Encrypted,
        subkey: ValueSubkey,
        private_key: &PrivateKey,
    ) -> Result<(), NetworkError> {
        let rc = get_routing_context().await;
        log!("writing to: [{}] {}", subkey, self.descriptor.key());
        rc.set_dht_value(
            *self.descriptor.key(),
            subkey,
            data.to_bytes().to_vec(),
            Some(KeyPair::new(*self.shard.key(), *private_key.key())),
        )
        .await
        .map_err(|e| NetworkError::RecordWriteFailed(e))?;
        log!("done writing to: [{}] {}", subkey, self.descriptor.key());
        Ok(())
    }

    pub async fn write_null(
        &self,
        subkey: ValueSubkey,
        private_key: &PrivateKey,
    ) -> Result<(), NetworkError> {
        let rc = get_routing_context().await;
        log!("writing null to: [{}] {}", subkey, self.descriptor.key());
        rc.set_dht_value(
            *self.descriptor.key(),
            subkey,
            vec![],
            Some(KeyPair::new(*self.shard.key(), *private_key.key())),
        )
        .await
        .map_err(|e| NetworkError::RecordWriteFailed(e))?;
        log!("done writing to: [{}] {}", subkey, self.descriptor.key());
        Ok(())
    }

    pub async fn write_chunked(
        &self,
        data: Encrypted,
        private_key: &PrivateKey,
    ) -> Result<(), NetworkError> {
        log!("writing chunked data to: {}", self.descriptor.key());
        // split data into chunks
        let bytes = data.to_bytes();
        let chunks = bytes.chunks(SUBKEY_SIZE_BYTES);
        let rc = get_routing_context().await;

        // write number of used chunks to subkey 0
        let count: u32 = chunks.len() as u32;
        rc.set_dht_value(
            *self.descriptor.key(),
            0,
            count.to_be_bytes().to_vec(),
            Some(KeyPair::new(*self.shard.key(), *private_key.key())),
        )
        .await
        .map_err(|e| NetworkError::RecordWriteFailed(e))?;

        // and write all the chunks to subkeys 1..count
        let futures = chunks
            // add the subkey index
            .enumerate()
            // and write out the values to the respective subkeys
            .map(|(subkey, data)| async move {
                rc.set_dht_value(
                    *self.descriptor.key(),
                    // add one cause subkey zero stores the count
                    (subkey as u32) + 1,
                    data.to_vec(),
                    Some(KeyPair::new(*self.shard.key(), *private_key.key())),
                )
                .await
                .map_err(|e| NetworkError::RecordWriteFailed(e))
            })
            .collect_vec();

        let _ = join_all(futures)
            .await
            .into_iter()
            // and use this magical impl for Result to bubble it up
            .collect::<Result<Vec<_>, _>>()?;
        log!("done writing chunked to: {}", self.descriptor.key());

        Ok(())
    }

    pub async fn read_chunked(&self, force_refresh: bool) -> Result<Encrypted, NetworkError> {
        // refresh all subkeys if desired
        if force_refresh {
            self.refresh().await?;
        }

        // read the count of used subkeys
        let count_bytes = self
            .read_key_raw(0, false)
            .await?
            .try_into()
            .map_err(|_e| NetworkError::InvalidData)?;
        let count = u32::from_be_bytes(count_bytes);

        let futures = (1..=count)
            // and read the subkey values
            .map(|i| async move { (i as ValueSubkey, self.read_key_raw(i as u32, false).await) })
            .collect_vec();

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

        let data = results.into_iter().map(|(_, chunk)| chunk).concat();
        let encrypted = Encrypted::from_bytes(&data)?;
        log!("done reading chunked from: {}", self.descriptor.key());
        Ok(encrypted)
    }

    pub async fn refresh(&self) -> Result<(), NetworkError> {
        let rc = get_routing_context().await;
        log!("inspecting record: {}", self.descriptor.key());
        let report = rc
            .inspect_dht_record(
                *self.descriptor.key(),
                ValueSubkeyRangeSet::single_range(0, MAX_SUBKEYS),
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
                log!("local: {}, remote: {}", local, remote);
                true
            })
            // remove any that don't have a remote value
            .filter(|(_, (remote, _))| **remote != ValueSubkey::MAX)
            // only keep ones that are newer than our local copy
            .filter(|(_, (remote, local))| {
                // log!("local: {}, remote: {}", local, remote);
                remote > local || **local == ValueSubkey::MAX
            })
            // and collect the subkey values
            .map(|(i, (_, _))| i)
            .collect_vec();

        log!("to fetch: {:?}", subkeys);

        let results = join_all(
            subkeys
                .into_iter()
                .map(|subkey| self.read_key(subkey as u32, true)),
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
        MAX_SUBKEYS as u16,
        vec![
            // and unique identifier
            DHTSchemaSMPLMember {
                m_key: hash.into(),
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
