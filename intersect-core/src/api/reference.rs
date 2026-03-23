use std::marker::PhantomData;

use veilid_core::{RecordKey, SharedSecret};

use crate::{
    api::Document,
    models::{Access, EncryptionError, Trace},
};

#[derive(PartialEq, Debug, Clone, Eq)]
pub struct Reference {
    record: RecordKey,
    secret: SharedSecret,
}

impl Reference {
    pub(crate) fn new(record: RecordKey, secret: SharedSecret) -> Self {
        Self { record, secret }
    }

    pub fn record(&self) -> &RecordKey {
        &self.record
    }

    pub fn secret(&self) -> &SharedSecret {
        &self.secret
    }
}

// typed handle for a document — wraps a Reference with the document type baked in.
// eliminates the need for explicit type annotations when calling open/write/update.
// convert to/from Trace for serialising and sharing.
pub struct TypedReference<D: Document> {
    pub(crate) reference: Reference,
    _phantom: PhantomData<D>,
}

impl<D: Document> TypedReference<D> {
    pub(crate) fn new(reference: Reference) -> Self {
        Self {
            reference,
            _phantom: PhantomData,
        }
    }

    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn to_unlocked_trace(&self) -> Trace {
        Trace::unlocked(
            D::RECORD_TYPE.clone(),
            self.reference.record(),
            self.reference.secret(),
        )
    }

    pub fn to_locked_trace(&self) -> Trace {
        Trace::locked(D::RECORD_TYPE.clone(), self.reference.record())
    }

    pub fn to_protected_trace(&self, password: &str) -> Result<Trace, EncryptionError> {
        Trace::protected(
            D::RECORD_TYPE.clone(),
            self.reference.record(),
            self.reference.secret(),
            password,
        )
    }
}

impl<D: Document> TryFrom<Trace> for TypedReference<D> {
    type Error = TraceConversionError;

    fn try_from(trace: Trace) -> Result<Self, Self::Error> {
        if trace.record_type() != &D::RECORD_TYPE {
            return Err(TraceConversionError::WrongRecordType);
        }
        let Access::Unlocked { secret } = trace.access() else {
            return Err(TraceConversionError::LockedAccess);
        };
        Ok(Self::new(Reference::new(
            trace.record().clone(),
            secret.clone(),
        )))
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TraceConversionError {
    #[error("trace record type does not match document type")]
    WrongRecordType,
    #[error("trace access is locked or protected")]
    LockedAccess,
}
