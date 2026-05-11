use guard_clause::guard;
use veilid_core::{HashDigest, RecordKey};

use crate::{
    models::ValidationError,
    proto,
    serialisation::{
        DeserialisationError, SerialisableV0, SerialisationError, impl_v0_proto_conversions,
    },
};

// fragment records use 32 subkeys (same as LARGE_SUBKEYS in api/document).
// veilid allocates 1MiB per record split evenly across subkeys, giving 32KiB per subkey.
pub const FRAGMENT_SUBKEYS: u16 = 32;
pub const MAX_CHUNK_BYTES: usize = 1024 * 1024 / FRAGMENT_SUBKEYS as usize;
// arbitrary limit, may be relaxed if needed in practice
pub const MAX_FRAGMENT_BYTES: usize = 32 * 1024 * 1024;

// RFC 6838 limits type and subtype names to 127 characters each (255 for type/subtype combined).
// 512 gives comfortable headroom for parameters (e.g. '; charset=UTF-8') on top of that.
const MIME_MAX_BYTES: usize = 512;

/// content type for a fragment, e.g. 'text/markdown;charset=UTF-8'
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FragmentMime(String);

impl FragmentMime {
    pub fn new(mime: String) -> Result<Self, ValidationError> {
        guard!(
            mime.len() <= MIME_MAX_BYTES,
            Err(ValidationError::TooLong(format!(
                "mime type can be at most {MIME_MAX_BYTES} bytes"
            )))
        );
        // basic type/subtype format check per RFC 6838: must be "type/subtype" with both non-empty
        let (type_part, subtype_part) = mime.split_once('/').ok_or_else(|| {
            ValidationError::Invalid("mime type must contain both a type and subtype".to_string())
        })?;
        guard!(
            !type_part.is_empty() && !subtype_part.is_empty(),
            Err(ValidationError::Invalid(
                "mime type and subtype must both be non-empty".to_string()
            ))
        );
        Ok(Self(mime))
    }
}

impl AsRef<str> for FragmentMime {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FragmentHeader {
    // hash of the reassembled *encrypted* data, for integrity verification
    hash: HashDigest,
    // total data size in bytes. must be > 0.
    fragment_size: u32,
    mime: FragmentMime,
    // overflow records sharing the same writer key and encryption
    overflow_keys: Vec<RecordKey>,
}

impl FragmentHeader {
    pub fn new(
        hash: HashDigest,
        fragment_size: u32,
        mime: FragmentMime,
        overflow_keys: Vec<RecordKey>,
    ) -> Result<Self, ValidationError> {
        guard!(
            fragment_size > 0 && fragment_size as usize <= MAX_FRAGMENT_BYTES,
            Err(ValidationError::Invalid(format!(
                "fragment size must be between 1 and {MAX_FRAGMENT_BYTES} bytes"
            )))
        );
        Ok(Self {
            hash,
            fragment_size,
            mime,
            overflow_keys,
        })
    }

    pub fn hash(&self) -> &HashDigest {
        &self.hash
    }

    pub fn fragment_size(&self) -> u32 {
        self.fragment_size
    }

    pub fn mime(&self) -> &FragmentMime {
        &self.mime
    }

    pub fn overflow_keys(&self) -> &[RecordKey] {
        &self.overflow_keys
    }
}

impl SerialisableV0 for FragmentHeader {
    type Proto = proto::v0::intersect::FragmentHeader;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            hash: Some(proto::v0::veilid::HashDigest::from(&self.hash)),
            fragment_size: self.fragment_size,
            mime: self.mime.as_ref().to_owned(),
            overflow_keys: self
                .overflow_keys
                .iter()
                .map(|k| k.try_into())
                .collect::<Result<_, _>>()?,
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let hash = HashDigest::from(
            proto
                .hash
                .ok_or(DeserialisationError::MissingField("hash".to_owned()))?,
        );
        let mime = FragmentMime::new(proto.mime)?;
        let overflow_keys = proto
            .overflow_keys
            .into_iter()
            .map(RecordKey::from)
            .collect();
        Self::new(hash, proto.fragment_size, mime, overflow_keys)
            .map_err(|e| DeserialisationError::Failed(e.to_string()))
    }
}

impl_v0_proto_conversions! {FragmentHeader}

/// the reassembled and decrypted content of a fragment.
/// this is what gets encrypted and chunked for storage.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FragmentContent(Vec<u8>);

impl FragmentContent {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn into_data(self) -> Vec<u8> {
        self.0
    }
}

impl SerialisableV0 for FragmentContent {
    type Proto = proto::v0::intersect::FragmentContent;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            data: self.0.clone(),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        Ok(Self(proto.data))
    }
}

impl_v0_proto_conversions! {FragmentContent}
