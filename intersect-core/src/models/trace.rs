use std::marker::PhantomData;

use base58::{FromBase58, ToBase58};
use binrw::binrw;
use thiserror::Error;

use crate::{
    record::Record,
    rw_helpers::{BinReadAlloc, BinWriteAlloc},
    Domain, DomainRecord, IntersectError, RecordType, Secret, VeilidRecordKey,
};

use super::Access;

// traces are essentially "links"
// they're what you would share with someone to show them a page

#[binrw]
#[brw(big)]
pub struct Trace<T: RecordType> {
    #[bw(map = |_| T::MAGIC)]
    #[br(try_map = |d: u8| (d == T::MAGIC).then_some(PhantomData).ok_or("invalid record type magic"))]
    _domain: PhantomData<T>,
    key: VeilidRecordKey,
    access: Access,
}

// manual clone impl to avoid the RecordType bound
impl<T: RecordType> Clone for Trace<T> {
    fn clone(&self) -> Self {
        Self {
            _domain: PhantomData,
            key: self.key.clone(),
            access: self.access.clone(),
        }
    }
}

// manual comparison impls to avoid the RecordType bound
impl<T: RecordType> PartialEq for Trace<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.access == other.access
    }
}
impl<T: RecordType> Eq for Trace<T> {}

// core trace impl
impl<T: RecordType> Trace<T> {
    pub(crate) fn new(key: VeilidRecordKey, access: Access) -> Self {
        Trace {
            _domain: PhantomData,
            key,
            access,
        }
    }

    pub fn from_str(trace: &str) -> Result<Self, TraceError> {
        trace.try_into()
    }

    pub async fn try_open<D: Domain<Record = T>>(&self) -> Result<T, IntersectError>
    where
        T: DomainRecord<D>,
    {
        let Access::Unlocked(secret) = self.access else {
            return Err(IntersectError::LockedTrace);
        };

        let record = Record::open(&self.key).await?;
        Ok(T::from_record(record, &secret).await?)
    }

    pub fn key(&self) -> &VeilidRecordKey {
        &self.key
    }

    pub fn access(&self) -> &Access {
        &self.access
    }
}

impl<T: RecordType> std::fmt::Debug for Trace<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        // use display impl
        write!(fmt, "{}", self)
    }
}

impl<T: RecordType> std::fmt::Display for Trace<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.serialise().to_base58())
    }
}

impl<T: RecordType> TryFrom<&str> for Trace<T> {
    type Error = TraceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = value
            .from_base58()
            .map_err(|_| TraceError::DeserialisationFailed("invalid base58".to_owned()))?;

        Self::deserialise(bytes.as_slice())
            .map_err(|e| TraceError::DeserialisationFailed(e.to_string()))
    }
}

// this is intentionally not serialisable!!
// only for internal use in an application
pub struct UnlockedTrace<T: RecordType> {
    _domain: PhantomData<T>,
    key: VeilidRecordKey,
    secret: Secret,
}

impl<T: RecordType> UnlockedTrace<T> {
    pub fn new(key: VeilidRecordKey, secret: Secret) -> Self {
        Self {
            _domain: PhantomData,
            key,
            secret,
        }
    }

    pub fn key(&self) -> &VeilidRecordKey {
        &self.key
    }

    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    pub async fn open(&self) -> Result<T, IntersectError> {
        T::open(&self.key, &self.secret).await
    }
}

impl<T: RecordType> From<UnlockedTrace<T>> for Trace<T> {
    fn from(value: UnlockedTrace<T>) -> Self {
        Trace::new(value.key, Access::Unlocked(value.secret))
    }
}

impl<T: RecordType> TryFrom<Trace<T>> for UnlockedTrace<T> {
    type Error = IntersectError;

    fn try_from(value: Trace<T>) -> Result<Self, Self::Error> {
        let secret = match value.access() {
            Access::Locked => Err(IntersectError::Unauthorized)?,
            Access::Protected(_protected_secret) => Err(IntersectError::Unauthorized)?,
            Access::Unlocked(secret) => secret,
        };

        Ok(UnlockedTrace::new(*value.key(), secret.clone()))
    }
}

// manual clone impl to avoid the RecordType bound
impl<T: RecordType> Clone for UnlockedTrace<T> {
    fn clone(&self) -> Self {
        Self {
            _domain: PhantomData,
            key: self.key.clone(),
            secret: self.secret.clone(),
        }
    }
}

impl<T: RecordType> Copy for UnlockedTrace<T> {}

// manual comparison impls to avoid the RecordType bound
impl<T: RecordType> PartialEq for UnlockedTrace<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.secret == other.secret
    }
}
impl<T: RecordType> Eq for UnlockedTrace<T> {}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum TraceError {
    #[error("missing key")]
    MissingKey,
    #[error("invalid key")]
    InvalidKey,
    #[error("missing secret")]
    MissingSecret,
    #[error("invalid secret")]
    InvalidSecret,
    #[error("deserialisation failed: {0}")]
    DeserialisationFailed(String),
}
