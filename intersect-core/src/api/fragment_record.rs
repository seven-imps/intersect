use crate::{
    log,
    models::{Encrypted, Fragment},
    record::{NetworkError, Record},
    ContentDomain, Domain, DomainRecord, Identity, IntersectError, RecordType, Secret,
};
use itertools::Itertools;

// since fragments are content addressed we don't need to
// keep track of an open record. we can just rely on the veilid cache
// we know the value will never change
pub struct FragmentRecord {
    record: Record,
    secret: Secret,
}

impl DomainRecord<ContentDomain> for FragmentRecord {}

impl RecordType for FragmentRecord {
    const MAGIC: u8 = 1;

    async fn from_record(record: Record, secret: &Secret) -> Result<Self, IntersectError> {
        Ok(Self {
            record,
            secret: secret.clone(),
        })
    }

    fn secret(&self) -> &Secret {
        &self.secret
    }

    fn record(&self) -> &Record {
        &self.record
    }
}

impl FragmentRecord {
    pub(crate) async fn create(
        identity: &Identity,
        fragment: &Fragment,
    ) -> Result<FragmentRecord, IntersectError> {
        // encrypt and build reference
        log!("building new fragment record");
        let (encrypted, secret) = Encrypted::encrypt_with_random(fragment)?;
        log!("making ref");
        let reference = ContentDomain::new_reference(identity.shard(), &encrypted.to_bytes());

        // create the record
        let record = Record::create(identity, &reference.hash()).await?;

        // split data into chunks
        let bytes = encrypted.to_bytes();
        let chunks = bytes.chunks(Record::SUBKEY_SIZE_BYTES);

        // write number of used chunks to subkey 0
        let count: u32 = chunks.len() as u32;
        record
            .write_raw(count.to_be_bytes().to_vec(), 0, identity.private_key())
            .await?;

        // and write all the chunks to subkeys 1..count
        let writes = chunks
            // add the subkey index
            .enumerate()
            // and offset the index by one to account for the subkey with the count
            .map(|(subkey, data)| ((subkey as u32) + 1, data.to_vec()));

        record
            .write_many_raw(writes, identity.private_key())
            .await?;

        Ok(FragmentRecord { record, secret })
    }

    pub async fn load(&self) -> Result<Fragment, IntersectError> {
        // read the count of used subkeys
        let count_bytes = self
            .record
            .read_raw(0, false)
            .await?
            .ok_or_else(|| NetworkError::MissingData)?
            .try_into()
            .map_err(|_e| NetworkError::InvalidData)?;
        let count = u32::from_be_bytes(count_bytes);

        // fragments are always content addressed,
        // so we can assume there will ever only be one version
        // no need to force_refresh
        let results = self.record.read_many_raw(1..=count, false).await?;

        let data = results.into_iter().filter_map(|(_, chunk)| chunk).concat();
        let fragment = Encrypted::from_bytes(&data)?.decrypt::<Fragment>(self.secret())?;
        Ok(fragment)
    }
}
