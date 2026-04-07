use std::marker::PhantomData;

use veilid_core::{RecordKey, SharedSecret};

use crate::{
    api::{Document, Reference, TypedReference},
    models::{Access, AccessError, ProtectedSecret, Trace},
};

// result of opening a trace, to provide an easier surface for getting from a Trace to a usabler TypedReference
pub enum OpenedTrace<D: Document> {
    Unlocked(TypedReference<D>),
    Locked(LockedTypedReference<D>),
    Protected(ProtectedTypedReference<D>),
}

// a type-checked reference without key. needs to be unlocked to be usable
pub struct LockedTypedReference<D: Document> {
    record: RecordKey,
    _phantom: PhantomData<D>,
}

impl<D: Document> LockedTypedReference<D> {
    pub fn unlock(self, secret: SharedSecret) -> Result<TypedReference<D>, AccessError> {
        Ok(TypedReference::new(Reference::new(self.record, secret)))
    }
}

// a type-checked reference with a password-protected key. needs to be unlocked with a password to be usable
pub struct ProtectedTypedReference<D: Document> {
    record: RecordKey,
    protected_secret: ProtectedSecret,
    _phantom: PhantomData<D>,
}

impl<D: Document> ProtectedTypedReference<D> {
    pub fn unlock(self, password: &str) -> Result<TypedReference<D>, AccessError> {
        let secret = self.protected_secret.unlock(password)?;
        Ok(TypedReference::new(Reference::new(self.record, secret)))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("trace document type does not match expected type")]
pub struct WrongDocumentType;

impl Trace {
    /// opens the trace, verifying the document type and leaving access for the caller to handle.
    /// errors only if the document type doesn't match.
    pub fn open<D: Document>(self) -> Result<OpenedTrace<D>, WrongDocumentType> {
        if self.document_type() != &D::DOCUMENT_TYPE {
            return Err(WrongDocumentType);
        }
        Ok(match self.access().clone() {
            Access::Unlocked { secret } => OpenedTrace::Unlocked(TypedReference::new(
                Reference::new(self.record().clone(), secret),
            )),
            Access::Locked => OpenedTrace::Locked(LockedTypedReference {
                record: self.record().clone(),
                _phantom: PhantomData,
            }),
            Access::Protected { protected_secret } => {
                OpenedTrace::Protected(ProtectedTypedReference {
                    record: self.record().clone(),
                    protected_secret,
                    _phantom: PhantomData,
                })
            }
        })
    }
}
