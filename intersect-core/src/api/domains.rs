use std::future::Future;

use crate::models::{
    Access, Fragment, IndexMetadata, LinkEntry, Reference, Segment, Trace, UnlockedTrace,
};
use crate::record::Record;
use crate::{veilid::with_crypto, FragmentRecord, Hash, IndexRecord, Secret, Shard};
use crate::{Identity, IntersectError, VeilidRecordKey};

use super::LinksRecord;

// domain trait

pub trait Domain
where
    Self: Clone + PartialEq + std::fmt::Debug + Copy,
{
    /// unique domain identifier (this will get hashed)
    const MAGIC: u8;
    /// record type
    type Record: RecordType + DomainRecord<Self>;
    // type Error;
    type HashData: ?Sized;

    fn compute_raw_hash(shard: &Shard, hash_data: &Self::HashData) -> Hash;

    fn new_reference(shard: &Shard, hash_data: &Self::HashData) -> Reference<Self> {
        // the raw hash as defined by the domain implementation
        let raw_hash = Self::compute_raw_hash(shard, hash_data);
        let hash = with_crypto(|crypto| {
            // hash the domain identifier so we have a 256 bit value
            let domain = crypto.generate_hash(&[Self::MAGIC]);
            // and then combine hashes to get the final domain separated hash
            crypto
                .generate_hash(
                    &[
                        shard.as_slice(),        // use shard as salt
                        domain.bytes.as_slice(), // domain separate them
                        raw_hash.as_slice(),     // and then the main hash
                    ]
                    .concat(),
                )
                .into()
        });

        Reference::<Self>::new(shard.clone(), hash)
    }

    fn open(
        reference: &Reference<Self>,
        secret: &Secret,
    ) -> impl Future<Output = Result<Self::Record, IntersectError>> {
        async {
            let key = Record::build_key(reference.shard(), reference.hash()).await;
            let record = Record::open(&key).await?;
            Self::Record::from_record(record, secret).await
        }
    }
}

// marker trait to indicate valid domains for a given recordtype
pub trait DomainRecord<D: Domain>
where
    Self: RecordType,
{
}

// trait to unify all different record handlers
// like FragmentRecord and IndexRecord etc
pub trait RecordType
where
    Self: Sized,
{
    const MAGIC: u8;

    fn from_record(
        record: Record,
        secret: &Secret,
    ) -> impl Future<Output = Result<Self, IntersectError>>;
    fn secret(&self) -> &Secret;
    fn record(&self) -> &Record;

    fn open(
        key: &VeilidRecordKey,
        secret: &Secret,
    ) -> impl Future<Output = Result<Self, IntersectError>> {
        async {
            let record = Record::open(key).await?;
            Self::from_record(record, secret).await
        }
    }

    fn reference<D: Domain>(&self) -> Reference<D>
    where
        Self: DomainRecord<D>,
    {
        Reference::<D>::new(self.record().shard().clone(), self.record().hash().clone())
    }

    fn to_trace(&self, include_secret: bool) -> Trace<Self> {
        let access = if include_secret {
            Access::Unlocked(*self.secret())
        } else {
            Access::Locked
        };

        Trace::new(self.record().record_key().clone(), access)
    }

    fn to_unlocked_trace(&self) -> UnlockedTrace<Self> {
        UnlockedTrace::new(self.record().record_key().clone(), *self.secret())
    }
}

// ==== content domain ====

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct ContentDomain;
impl Domain for ContentDomain {
    const MAGIC: u8 = 1;
    type Record = FragmentRecord;

    type HashData = [u8];
    fn compute_raw_hash(shard: &Shard, hash_data: &Self::HashData) -> Hash {
        with_crypto(|crypto| {
            crypto
                // include shard as salt
                .generate_hash(&[shard.as_slice(), hash_data].concat())
                .into()
        })
    }
}

impl ContentDomain {
    pub async fn create(
        identity: &Identity,
        fragment: &Fragment,
    ) -> Result<FragmentRecord, IntersectError> {
        FragmentRecord::create(identity, fragment).await
    }
}

// ==== index domain ====

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct IndexDomain;
impl Domain for IndexDomain {
    const MAGIC: u8 = 2;
    type Record = IndexRecord;

    type HashData = ();
    fn compute_raw_hash(shard: &Shard, _hash_data: &Self::HashData) -> Hash {
        with_crypto(|crypto| {
            // generate random 256 bit id for this index
            let identifier = crypto.random_bytes(32);
            crypto
                // include shard as salt
                .generate_hash(&[shard.as_slice(), identifier.as_slice()].concat())
                .into()
        })
    }
}

impl IndexDomain {
    pub async fn create(
        identity: &Identity,
        meta: &IndexMetadata,
    ) -> Result<IndexRecord, IntersectError> {
        let secret = Secret::random();
        let reference = IndexDomain::new_reference(identity.shard(), &());
        IndexRecord::create(identity, &reference, &secret, meta).await
    }
}

// ==== root domain ====

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct RootDomain;
impl Domain for RootDomain {
    const MAGIC: u8 = 3;
    type Record = IndexRecord;

    // root hashes are based on a segment name
    // e.g. "home", "account"
    // a given shard, segment pair will always have one unique root domain hash
    type HashData = Segment;
    fn compute_raw_hash(shard: &Shard, hash_data: &Self::HashData) -> Hash {
        with_crypto(|crypto| {
            crypto
                // include shard as salt
                .generate_hash(&[shard.as_slice(), hash_data.to_string().as_bytes()].concat())
                .into()
        })
    }
}

impl RootDomain {
    pub async fn open_public(
        shard: &Shard,
        root_name: &Segment,
    ) -> Result<<Self as Domain>::Record, IntersectError> {
        let reference = Self::new_reference(shard, root_name);
        let secret = Self::public_secret(shard, root_name);
        Self::open(&reference, &secret).await
    }

    pub async fn open_private(
        identity: &Identity,
        root_name: &Segment,
    ) -> Result<<Self as Domain>::Record, IntersectError> {
        let reference = Self::new_reference(identity.shard(), root_name);
        let secret = Self::private_secret(identity, root_name);
        Self::open(&reference, &secret).await
    }

    pub async fn create_public(
        identity: &Identity,
        root_name: &Segment,
        meta: &IndexMetadata,
    ) -> Result<<Self as Domain>::Record, IntersectError> {
        let reference = Self::new_reference(identity.shard(), root_name);
        let secret: Secret = Self::public_secret(identity.shard(), root_name);
        IndexRecord::create(identity, &reference, &secret, meta).await
    }

    pub async fn create_private(
        identity: &Identity,
        root_name: &Segment,
        meta: &IndexMetadata,
    ) -> Result<<Self as Domain>::Record, IntersectError> {
        let reference = Self::new_reference(identity.shard(), root_name);
        let secret: Secret = Self::private_secret(identity, root_name);
        IndexRecord::create(identity, &reference, &secret, meta).await
    }

    fn public_secret(shard: &Shard, root_name: &Segment) -> Secret {
        // derive secret from public key and root name
        // this means it's essentially public knowledge!!
        // it's mostly intended for any roots that are supposed to be visible to anyone with just the shard
        with_crypto(|crypto| {
            crypto
                .derive_shared_secret(
                    root_name.to_string().as_bytes(),
                    // include shard and unique string for domain separation
                    &[shard.as_slice(), b"public secret".as_slice()].concat(),
                )
                .unwrap()
                .into()
        })
    }

    fn private_secret(identity: &Identity, root_name: &Segment) -> Secret {
        with_crypto(|crypto| {
            crypto
                .derive_shared_secret(
                    // derive password from the name and private key
                    &[
                        root_name.to_string().as_bytes(),
                        identity.private_key().as_slice(),
                    ]
                    .concat(),
                    // include shard and unique string for domain separation
                    &[identity.shard().as_slice(), b"private secret".as_slice()].concat(),
                )
                .unwrap()
                .into()
        })
    }
}

// ==== links domain ====

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct LinksDomain;
impl Domain for LinksDomain {
    const MAGIC: u8 = 4;
    type Record = LinksRecord;

    type HashData = ();
    fn compute_raw_hash(shard: &Shard, _hash_data: &Self::HashData) -> Hash {
        with_crypto(|crypto| {
            // generate random 256 bit id for this index
            let identifier = crypto.random_bytes(32);
            crypto
                // include shard as salt
                .generate_hash(&[shard.as_slice(), identifier.as_slice()].concat())
                .into()
        })
    }
}

impl LinksDomain {
    pub async fn create(
        identity: &Identity,
        links: &[LinkEntry],
    ) -> Result<LinksRecord, IntersectError> {
        let secret = Secret::random();
        let reference = LinksDomain::new_reference(identity.shard(), &());
        LinksRecord::create(identity, &reference, &secret, links).await
    }
}
