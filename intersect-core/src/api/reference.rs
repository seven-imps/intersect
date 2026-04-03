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

// typed handle for a document. wraps a Reference with the document type baked in.
// eliminates the need for explicit type annotations when calling open/write/update.
// convert to/from Trace for serialising and sharing.
#[derive(PartialEq, Debug, Eq)]
pub struct TypedReference<D: Document> {
    pub(crate) reference: Reference,
    _phantom: PhantomData<D>,
}

// manual impl to avoid the `D: Clone` bound that #[derive(Clone)] would generate.
// D is only used as a marker (PhantomData), so it doesn't need to be Clone itself.
impl<D: Document> Clone for TypedReference<D> {
    fn clone(&self) -> Self {
        Self { reference: self.reference.clone(), _phantom: PhantomData }
    }
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
            D::DOCUMENT_TYPE,
            self.reference.record(),
            self.reference.secret(),
        )
    }

    pub fn to_locked_trace(&self) -> Trace {
        Trace::locked(D::DOCUMENT_TYPE, self.reference.record())
    }

    pub fn to_protected_trace(&self, password: &str) -> Result<Trace, EncryptionError> {
        Trace::protected(
            D::DOCUMENT_TYPE,
            self.reference.record(),
            self.reference.secret(),
            password,
        )
    }

    pub fn from_trace(trace: Trace) -> Result<Self, TraceConversionError> {
        trace.try_into()
    }
}

impl<D: Document> TryFrom<Trace> for TypedReference<D> {
    type Error = TraceConversionError;

    fn try_from(trace: Trace) -> Result<Self, Self::Error> {
        if trace.document_type() != &D::DOCUMENT_TYPE {
            return Err(TraceConversionError::WrongDocumentType);
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
    WrongDocumentType,
    #[error("trace access is locked or protected")]
    LockedAccess,
}
