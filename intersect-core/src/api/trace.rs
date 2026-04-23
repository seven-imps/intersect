use std::marker::PhantomData;

use veilid_core::RecordKey;

use crate::{
    api::{Document, Reference, TypedReference},
    models::{Access, AccessError, ProtectedSecret, Trace, TraceSecret},
};

// result of opening a trace, to provide an easier surface for getting from a Trace to a usabler TypedReference
#[derive(Clone)]
pub enum TypedTrace<D: Document> {
    Unlocked(TypedReference<D>),
    Locked(LockedTypedReference<D>),
    Protected(ProtectedTypedReference<D>),
}

// a type-checked reference without key. needs to be unlocked to be usable
#[derive(Clone)]
pub struct LockedTypedReference<D: Document> {
    record: RecordKey,
    _phantom: PhantomData<D>,
}

impl<D: Document> LockedTypedReference<D> {
    pub fn unlock(&self, secret: TraceSecret) -> Result<TypedReference<D>, AccessError> {
        Ok(TypedReference::new(Reference::new(self.record.clone(), secret.inner().clone())))
    }
}

// a type-checked reference with a password-protected key. needs to be unlocked with a password to be usable
#[derive(Clone)]
pub struct ProtectedTypedReference<D: Document> {
    record: RecordKey,
    protected_secret: ProtectedSecret,
    _phantom: PhantomData<D>,
}

impl<D: Document> ProtectedTypedReference<D> {
    pub fn unlock(&self, password: &str) -> Result<TypedReference<D>, AccessError> {
        let secret = self.protected_secret.unlock(password)?;
        Ok(TypedReference::new(Reference::new(self.record.clone(), secret)))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("trace document type does not match expected type")]
pub struct WrongDocumentType;

#[derive(Debug, thiserror::Error)]
pub enum NotUnlocked {
    #[error("trace requires a secret key")]
    Locked,
    #[error("trace requires a password")]
    Protected,
}

impl<D: Document> TypedTrace<D> {
    /// extracts the inner TypedReference if the trace is unlocked (key already embedded).
    /// errors if locked or password-protected.
    pub fn into_unlocked(self) -> Result<TypedReference<D>, NotUnlocked> {
        match self {
            TypedTrace::Unlocked(typed_ref) => Ok(typed_ref),
            TypedTrace::Locked(_) => Err(NotUnlocked::Locked),
            TypedTrace::Protected(_) => Err(NotUnlocked::Protected),
        }
    }
}

impl Trace {
    /// type-checks the trace and converts it to a typed TypedTrace<D>,
    /// leaving access handling to the caller.
    /// errors only if the document type doesn't match.
    pub fn into_typed<D: Document>(self) -> Result<TypedTrace<D>, WrongDocumentType> {
        if self.document_type() != &D::DOCUMENT_TYPE {
            return Err(WrongDocumentType);
        }
        Ok(match self.access().clone() {
            Access::Unlocked { secret } => TypedTrace::Unlocked(TypedReference::new(
                Reference::new(self.record().clone(), secret),
            )),
            Access::Locked => TypedTrace::Locked(LockedTypedReference {
                record: self.record().clone(),
                _phantom: PhantomData,
            }),
            Access::Protected { protected_secret } => {
                TypedTrace::Protected(ProtectedTypedReference {
                    record: self.record().clone(),
                    protected_secret,
                    _phantom: PhantomData,
                })
            }
        })
    }
}
