use prost::Message;
use thiserror::Error;

const MAGIC: &[u8] = b"/?/";

#[repr(u8)]
pub enum Version {
    V1 = 1,
}

impl Version {
    // update when adding new version
    pub const LATEST: Version = Version::V1;
}

impl TryFrom<u8> for Version {
    type Error = DeserialisationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Version::V1),
            _ => Err(DeserialisationError::InvalidVersion),
        }
    }
}

pub trait Serialise {
    // fn serialise(&self) -> Result<Vec<u8>, SerialisationError> {
    fn serialise(&self) -> Result<Vec<u8>, SerialisationError> {
        // always serialise with the latest version
        let version = Version::LATEST;
        let proto_bytes = match version {
            Version::V1 => self.serialise_v1()?,
        };
        // prefix protobuf bytes with magic bytes and version number
        Ok([MAGIC, &[version as u8], proto_bytes.as_slice()].concat())
    }

    fn serialise_v1(&self) -> Result<Vec<u8>, SerialisationError> {
        // v1 uses proto, but technically a new version wouldn't even have to use proto
        Ok(self.serialise_v1_proto()?.encode_length_delimited_to_vec())
    }

    fn serialise_v1_proto(&self) -> Result<impl Message, SerialisationError>;

    // when adding a new version, just add a new `serialise_vX` function to this trait.
    // (and update the version used in `serialise`)
    // this will force you to add it to all existing impls. this is the point.
}

pub trait Deserialise
where
    Self: Sized,
{
    fn deserialise(bytes: &[u8]) -> Result<Self, DeserialisationError> {
        // validate the magic bytes
        let (magic, rest) = bytes
            .split_at_checked(MAGIC.len())
            .ok_or(DeserialisationError::UnexpectedEnd)?;
        if magic != MAGIC {
            return Err(DeserialisationError::InvalidMagic);
        }
        // deserialise whatever version we end up finding
        let version_byte = *rest.first().ok_or(DeserialisationError::UnexpectedEnd)?;
        let proto_bytes = rest.get(1..).ok_or(DeserialisationError::UnexpectedEnd)?;
        match Version::try_from(version_byte)? {
            Version::V1 => Self::deserialise_v1(proto_bytes),
        }
    }

    fn deserialise_v1(bytes: &[u8]) -> Result<Self, DeserialisationError>;

    // just a lil helper so we get consistent proto deserialisation
    fn deserialise_proto<M: Message + Default>(bytes: &[u8]) -> Result<M, DeserialisationError> {
        M::decode_length_delimited(bytes)
            .map_err(|e| DeserialisationError::InvalidProto(e.to_string()))
    }

    // same here. when adding a new version, just add a new function.
    // no need to update anything else, since deserialisation should support all versions.
    // always *write* the newest version and *read* any supported versions
}

// to make it easier on implementers, let's add a trait that we can add blanket impls for

pub trait SerialisableV1
where
    Self: Sized,
{
    type Proto: prost::Message + Default;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError>;
    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError>;
}

// blanket impl for Serialise
impl<T: SerialisableV1> Serialise for T {
    fn serialise_v1_proto(&self) -> Result<impl Message, SerialisationError> {
        self.to_proto()
    }
}
// blanket impl for Deserialise
impl<T: SerialisableV1> Deserialise for T {
    fn deserialise_v1(bytes: &[u8]) -> Result<Self, DeserialisationError> {
        let proto = Self::deserialise_proto::<T::Proto>(bytes)?;
        Self::from_proto(proto)
    }
}

// can't do blanket impls on foreign traits, so here's a macro instead
macro_rules! impl_v1_proto_conversions {
    ($t:ty) => {
        impl TryFrom<&$t> for <$t as SerialisableV1>::Proto {
            type Error = SerialisationError;
            fn try_from(value: &$t) -> Result<Self, Self::Error> {
                value.to_proto()
            }
        }

        impl TryFrom<<$t as SerialisableV1>::Proto> for $t {
            type Error = DeserialisationError;
            fn try_from(value: <$t as SerialisableV1>::Proto) -> Result<Self, Self::Error> {
                Self::from_proto(value)
            }
        }
    };
}
pub(crate) use impl_v1_proto_conversions;

use crate::models::ValidationError;

#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SerialisationError {
    #[error("serialisation failed with error: {0}")]
    Failed(String),

    #[error("missing default encryption key")]
    MissingDefaultEncryptionKey,
}

#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DeserialisationError {
    #[error("deserialisation failed with error: {0}")]
    Failed(String),

    #[error("unexpected end of input")]
    UnexpectedEnd,

    #[error("invalid version")]
    InvalidVersion,

    #[error("invalid magic bytes")]
    InvalidMagic,

    #[error("invalid proto: {0}")]
    InvalidProto(String),

    #[error("missing field: {0}")]
    MissingField(String),

    #[error("model validation error: {0}")]
    ValidationError(#[from] ValidationError),
}
