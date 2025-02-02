use crate::{
    log,
    models::{Encrypted, IndexMetadata, Reference},
    record::{NetworkError, Record},
    Domain, DomainRecord, Identity, IndexDomain, IntersectError, RecordType, RootDomain, Secret,
};

use super::LinksRecord;

// TODO:
//   - potentially wrap the record field in an arc mutex so we can derive clone
//   - add a method that checks for fragment updates
//     - inspect the metadata
//     - fetch updated version if it exists
//     - fetch new fragment and return it
//     - return option<metadata>  (only some if it changed)

pub struct IndexRecord {
    record: Record,
    secret: Secret,
}

impl DomainRecord<IndexDomain> for IndexRecord {}
impl DomainRecord<RootDomain> for IndexRecord {}

impl RecordType for IndexRecord {
    const MAGIC: u8 = 2;

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

impl IndexRecord {
    pub async fn meta(&self, force_refresh: bool) -> Result<IndexMetadata, IntersectError> {
        // subkey 0 is the metadata
        // must always be present
        let meta = self
            .record
            .read(0, force_refresh)
            .await?
            .ok_or_else(|| NetworkError::MissingData)?
            .decrypt::<IndexMetadata>(&self.secret())?;
        Ok(meta)
    }

    pub(crate) async fn create<D: Domain>(
        identity: &Identity,
        reference: &Reference<D>,
        secret: &Secret,
        meta: &IndexMetadata,
    ) -> Result<IndexRecord, IntersectError>
    where
        Self: DomainRecord<D>,
    {
        log!("building new index record");

        // make sure the identity matches the metadata pased in
        if identity.shard() != meta.shard() {
            return Err(IntersectError::Unauthorized);
        }

        // and then encrypt it
        let encrypted = Encrypted::encrypt(meta, secret)?;

        // create the record
        let record = Record::create(&identity, &reference.hash()).await?;

        // and store the metadata there
        record.write(encrypted, 0, identity.private_key()).await?;

        Ok(IndexRecord {
            record,
            secret: secret.clone(),
        })
    }

    pub async fn update_meta(
        &mut self,
        identity: &Identity,
        new_metadata: &IndexMetadata,
    ) -> Result<(), IntersectError> {
        let encrypted = Encrypted::encrypt(new_metadata, self.secret())?;
        self.record
            .write(encrypted, 0, identity.private_key())
            .await?;

        Ok(())
    }

    // pub async fn fetch_fragment(
    //     &self,
    //     force_refresh: bool,
    // ) -> Result<Option<FragmentRecord>, IntersectError> {
    //     let meta = self.meta(force_refresh).await?;

    //     if let Some(trace) = meta.fragment() {
    //         Ok(Some(trace.open().await?))
    //     } else {
    //         Ok(None)
    //     }
    // }

    pub async fn try_fetch_links(
        &self,
        force_refresh: bool,
    ) -> Result<Option<LinksRecord>, IntersectError> {
        let meta = self.meta(force_refresh).await?;

        if let Some(link) = meta.links() {
            Ok(Some(link.try_open().await?))
        } else {
            Ok(None)
        }
    }

    // pub async fn fetch_or_new_links(
    //     &mut self,
    //     identity: &Identity,
    //     force_refresh: bool,
    // ) -> Result<LinksRecord, IntersectError> {
    //     let record = match self.fetch_links(force_refresh).await? {
    //         Some(record) => record,
    //         None => {
    //             // create empty links record
    //             let record = LinksDomain::create(identity, &[]).await?;
    //             // and save it to the metadata
    //             let new_meta = self
    //                 // no need to refresh meta, fetch_links already did
    //                 .meta(false)
    //                 .await?
    //                 .with_links(&record.to_trace(true));
    //             self.update_meta(identity, &new_meta).await?;

    //             record
    //         }
    //     };

    //     Ok(record)
    // }
}
