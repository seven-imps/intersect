use futures::future::join_all;

use crate::{
    log,
    models::{Encrypted, LinkEntry, Reference},
    record::{NetworkError, Record},
    DomainRecord, Identity, IntersectError, RecordType, Secret, ValueSubkey,
};

use super::LinksDomain;

pub struct LinksRecord {
    record: Record,
    secret: Secret,
}

impl DomainRecord<LinksDomain> for LinksRecord {}

impl RecordType for LinksRecord {
    const MAGIC: u8 = 3;

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

impl LinksRecord {
    pub(crate) async fn create(
        identity: &Identity,
        reference: &Reference<LinksDomain>,
        secret: &Secret,
        links: &[LinkEntry],
    ) -> Result<LinksRecord, IntersectError> {
        // create the record
        let record = Record::create(&identity, &reference.hash()).await?;

        // store all the links in their own subkeys
        let results = join_all(
            links
                .into_iter()
                // encrypt all the links
                .map(|link| Encrypted::encrypt(link, &secret).unwrap())
                // add a running index we can use as subkeys
                .enumerate()
                // and write the values
                .map(|(i, link)| record.write(link, i as u32, identity.private_key())),
        )
        .await;

        // handle all errors here
        for r in results {
            // TODO: consider returning a vec instead of just raising the first error
            let _ = r?;
        }

        Ok(LinksRecord {
            record,
            secret: secret.clone(),
        })
    }

    pub async fn fetch_links(
        &self,
        force_refresh: bool,
    ) -> Result<Vec<(ValueSubkey, LinkEntry)>, IntersectError> {
        // read all the used subkeys
        let values = self.record.read_all(force_refresh).await?;
        log!("found {} record entries", values.len());

        // and grab all the links
        let links = values
            .into_iter()
            // filter out None values
            .filter_map(|(i, e)| e.map(|encrypted| (i, encrypted)))
            // decrypt them all
            .map(|(i, e)| {
                e.decrypt::<LinkEntry>(self.secret())
                    .and_then(|l| Ok((i, l)))
            })
            // and so some rust magic to go from [Result<T, E>] to Result<[T], E>
            .collect::<Result<Vec<_>, _>>()?;

        log!("found {} links", links.len());

        Ok(links)
    }

    pub async fn add_link(
        &self,
        identity: &Identity,
        index_link: &LinkEntry,
    ) -> Result<(), IntersectError> {
        // build link
        // let link = IndexLink::new(name, reference, shared_secret);
        // encrypt the link
        let encrypted = Encrypted::encrypt(index_link, self.secret())?;

        // opportunistically find an unused subkey based on local cache
        // under the assumption it's very unlikely that it's been changed since lat time we fetched all the links
        log!("lazy check for unused subkeys");
        let unused_subkeys = self.record.find_unused(false).await?;
        let mut first_unused = None;
        if let Some(&subkey) = unused_subkeys.get(0) {
            // check for real, but without refreshing everything
            if self.record.is_unused(subkey).await? {
                first_unused = Some(subkey);
            }
        }

        // else do a proper refresh
        if first_unused.is_none() {
            log!("doing full refresh to find unused subkey");
            let unused_subkeys = self.record.find_unused(true).await?;
            first_unused = unused_subkeys.get(0).copied();
        }

        // fail if we still haven't found an empty subkey
        let first_unused = first_unused.ok_or(NetworkError::NoUnusedSubkey)?;
        // and store our link there!
        self.record
            .write(encrypted, first_unused, identity.private_key())
            .await?;
        Ok(())
    }

    pub async fn remove_link(
        &self,
        identity: &Identity,
        subkey: ValueSubkey,
    ) -> Result<(), IntersectError> {
        self.record
            .write_null(subkey, identity.private_key())
            .await?;
        Ok(())
    }
}
