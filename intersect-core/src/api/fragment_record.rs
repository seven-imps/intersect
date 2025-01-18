use crate::{
    log,
    models::{Encrypted, Fragment},
    record::Record,
    ContentDomain, Domain, DomainRecord, Identity, IntersectError, RecordType, Secret,
};

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

        // and store the fragment in subkey 0
        record
            .write_chunked(encrypted, identity.private_key())
            .await?;

        Ok(FragmentRecord { record, secret })
    }

    pub async fn load(&self) -> Result<Fragment, IntersectError> {
        // TODO: make this look at all subkeys
        // for now, just limited to one for simplicity
        let fragment = self
            .record
            // fragments are always content addressed,
            // so we can assume there will ever only be one version
            // no need to force_refresh
            .read_chunked(false)
            .await?
            .decrypt::<Fragment>(self.secret())?;
        Ok(fragment)
    }
}
