// fewer subkeys = larger max size per subkey (32kb each)
pub const LARGE_SUBKEYS: u16 = 32;
// more subkeys = smaller max size per subkey (4kb each)
pub const MANY_SUBKEYS: u16 = 256;

use veilid_core::KeyPair;

use crate::{
    api::{Reference, TypedReference},
    models::RecordType,
    veilid::RecordPool,
};

#[allow(async_fn_in_trait)]
pub trait Document: Sized {
    // if false, open() does a single read and closes the channel.
    // immutable document types (e.g. fragments) should set this to false.
    const MUTABLE: bool;

    // number of subkeys on the root record. affects max subkey size.
    const MAX_SUBKEYS: u16;

    // the record type used when serialising a TypedReference to a Trace.
    const RECORD_TYPE: RecordType;

    type View: PartialEq + Clone + Send + Sync + 'static;

    // partial write intent. expresses what to update, not how.
    type Update;

    async fn read(
        reference: &Reference,
        identity: Option<&KeyPair>,
        pool: &RecordPool,
    ) -> Result<Self::View, DocumentError>;

    async fn create(
        view: &Self::View,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> Result<TypedReference<Self>, DocumentError>;

    // immutable document types (MUTABLE = false) should return Err(DocumentError::NotMutable) here.
    // TODO: updates should use some kind of builder pattern and then apply
    // all changes inside of a veilid transaction instead.
    // that way we also avoid the potential issues of doing mutiple updates
    // in parallel and overwriting each other.
    async fn update(
        update: Self::Update,
        reference: &Reference,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> Result<(), DocumentError>;
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

    #[error("document is not mutable")]
    NotMutable,
}
