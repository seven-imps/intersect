// fewer subkeys = larger max size per subkey (32kb each)
pub const LARGE_SUBKEYS: u16 = 32;
// more subkeys = smaller max size per subkey (4kb each)
pub const MANY_SUBKEYS: u16 = 256;

use std::fmt::Debug;
use tokio::sync::watch;
use veilid_core::KeyPair;

use crate::{
    api::{Reference, TypedReference},
    models::DocumentType,
    veilid::RecordPool,
};

pub trait Document: Sized {
    /// number of subkeys on the root record. affects max subkey size.
    const MAX_SUBKEYS: u16;

    /// the document type used when serialising a TypedReference to a Trace.
    const DOCUMENT_TYPE: DocumentType;

    type View: PartialEq + Clone + Debug + Send + Sync + 'static;

    /// read the entire document and assemble it.
    /// everything should be discoverable from the root reference, put could potentially read from more records
    ///  if `force` is true, `read` should guarantee the most recent network version is returned
    /// for mutable documents that means a force_refresh should be done when reading subkeys,
    /// immutable records are always guaranteed to be fresh, so force can be ignored.
    fn read<'a>(
        reference: &'a Reference,
        identity: Option<&'a KeyPair>,
        force: bool,
        pool: &'a RecordPool,
        // (gotta be Send so it can be called from the WatchCoordinator task)
    ) -> impl Future<Output = Result<Self::View, DocumentError>> + Send + 'a;

    // create takes an owned view to avoid unnnecessary cloning
    fn create(
        view: Self::View,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> impl Future<Output = Result<TypedReference<Self>, DocumentError>> + Send;
}

pub trait MutableDocument: Document {
    /// represents partial write intent. expresses what to update, not how.
    type Update;

    // TODO: updates should use some kind of builder pattern and then apply
    // all changes inside of a veilid transaction instead.
    // that way we also avoid the potential issues of doing mutiple updates
    // in parallel and overwriting each other.
    fn update(
        update: Self::Update,
        document: &OpenDocument<Self>,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> impl Future<Output = Result<(), DocumentError>> + Send;
}

pub struct OpenDocument<D: MutableDocument> {
    pub reference: TypedReference<D>,
    pub updates: watch::Receiver<Result<D::View, DocumentError>>,
}

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum DocumentError {
    #[error("record error: {0}")]
    RecordError(#[from] crate::veilid::RecordError),

    #[error("serialisation error: {0}")]
    SerialisationError(#[from] crate::serialisation::SerialisationError),

    #[error("deserialisation error: {0}")]
    DeserialisationError(#[from] crate::serialisation::DeserialisationError),

    #[error("encryption error: {0}")]
    EncryptionError(#[from] crate::models::EncryptionError),

    #[error("validation error: {0}")]
    ValidationError(#[from] crate::models::ValidationError),

    #[error("not authorised")]
    NotAuthorised,

    #[error("hash mismatch: fragment data is corrupt or tampered")]
    HashMismatch,

    #[error("corrupt document: {0}")]
    Corrupt(String),
}
