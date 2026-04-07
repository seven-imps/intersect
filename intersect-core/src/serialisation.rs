use prost::Message;
use thiserror::Error;

use crate::models::ValidationError;

/// fourcc-style version tag, used to prefix both string and byte serialisations
/// binary: <fourcc bytes><proto bytes>
/// string: <fourcc string>:<base58(proto bytes)>
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Version {
    V0,
}

const V0_FOURCC: &[u8; 4] = b"ISC0";

impl Version {
    // update when adding a new version
    pub const LATEST: Version = Version::V0;

    fn as_str(self) -> &'static str {
        match self {
            Version::V0 => std::str::from_utf8(V0_FOURCC).unwrap(), // infallible unwrap, it's ok
        }
    }

    fn as_bytes(self) -> [u8; 4] {
        match self {
            Version::V0 => *V0_FOURCC,
        }
    }

    fn from_fourcc(fourcc: &[u8; 4]) -> Option<Self> {
        if fourcc == V0_FOURCC {
            Some(Version::V0)
        } else {
            None
        }
    }
}

pub trait Serialise {
    fn serialise(&self) -> Result<Vec<u8>, SerialisationError> {
        let version = Version::LATEST;
        let proto_bytes = match version {
            Version::V0 => self.serialise_v0()?,
        };
        Ok([&version.as_bytes(), proto_bytes.as_slice()].concat())
    }

    fn serialise_to_string(&self) -> Result<String, SerialisationError> {
        let version = Version::LATEST;
        let proto_bytes = match version {
            Version::V0 => self.serialise_v0()?,
        };
        // TODO: TIL that base58 encoding runs in O(n^2) time...
        // _should_ be fine here cause nothing we convert to a string will be huge,
        // but look into https://carlmastrangelo.com/blog/a-better-base-58-encoding if this ever becomes a problem.
        Ok(format!(
            "{}:{}",
            version.as_str(),
            bs58::encode(&proto_bytes).into_string()
        ))
    }

    fn serialise_v0(&self) -> Result<Vec<u8>, SerialisationError> {
        // v0 uses proto, but technically a new version wouldn't even have to use proto
        Ok(self.serialise_v0_proto()?.encode_length_delimited_to_vec())
    }

    fn serialise_v0_proto(&self) -> Result<impl Message, SerialisationError>;

    // when adding a new version, just add a new `serialise_vX` function to this trait.
    // (and update the version used in `serialise` and `serialise_to_string`)
    // this will force you to add it to all existing impls. this is the point.
}

pub trait Deserialise
where
    Self: Sized,
{
    fn deserialise(bytes: &[u8]) -> Result<Self, DeserialisationError> {
        let (fourcc, proto_bytes) = bytes
            .split_at_checked(4)
            .ok_or(DeserialisationError::UnexpectedEnd)?;
        let fourcc: &[u8; 4] = fourcc.try_into().unwrap();
        let version = Version::from_fourcc(fourcc).ok_or(DeserialisationError::InvalidMagic)?;
        match version {
            Version::V0 => Self::deserialise_v0(proto_bytes),
        }
    }

    fn deserialise_from_str(s: &str) -> Result<Self, DeserialisationError> {
        // accept both : and _ as separators
        let sep = s
            .find([':', '_'])
            .ok_or(DeserialisationError::InvalidMagic)?;
        let (prefix, rest) = (&s[..sep], &s[sep + 1..]);
        let fourcc: &[u8; 4] = prefix
            .as_bytes()
            .try_into()
            .map_err(|_| DeserialisationError::InvalidMagic)?;
        let version = Version::from_fourcc(fourcc).ok_or(DeserialisationError::InvalidMagic)?;
        let proto_bytes = bs58::decode(rest)
            .into_vec()
            .map_err(|_| DeserialisationError::Failed("invalid base58 encoding".to_string()))?;
        match version {
            Version::V0 => Self::deserialise_v0(&proto_bytes),
        }
    }

    fn deserialise_v0(bytes: &[u8]) -> Result<Self, DeserialisationError>;

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

pub trait SerialisableV0
where
    Self: Sized,
{
    type Proto: prost::Message + Default;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError>;
    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError>;
}

// blanket impl for Serialise
impl<T: SerialisableV0> Serialise for T {
    fn serialise_v0_proto(&self) -> Result<impl Message, SerialisationError> {
        self.to_proto()
    }
}
// blanket impl for Deserialise
impl<T: SerialisableV0> Deserialise for T {
    fn deserialise_v0(bytes: &[u8]) -> Result<Self, DeserialisationError> {
        let proto = Self::deserialise_proto::<T::Proto>(bytes)?;
        Self::from_proto(proto)
    }
}

// can't do blanket impls on foreign traits, so here's a macro instead
macro_rules! impl_v0_proto_conversions {
    ($t:ty) => {
        impl TryFrom<&$t> for <$t as SerialisableV0>::Proto {
            type Error = SerialisationError;
            fn try_from(value: &$t) -> Result<Self, Self::Error> {
                value.to_proto()
            }
        }

        impl TryFrom<<$t as SerialisableV0>::Proto> for $t {
            type Error = DeserialisationError;
            fn try_from(value: <$t as SerialisableV0>::Proto) -> Result<Self, Self::Error> {
                Self::from_proto(value)
            }
        }
    };
}
pub(crate) use impl_v0_proto_conversions;

macro_rules! impl_string_conversions {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}",
                    self.serialise_to_string().map_err(|_| std::fmt::Error)?
                )
            }
        }

        impl std::str::FromStr for $t {
            type Err = DeserialisationError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::deserialise_from_str(s)
            }
        }
    };
}
pub(crate) use impl_string_conversions;

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

    #[error("invalid magic bytes")]
    InvalidMagic,

    #[error("invalid proto: {0}")]
    InvalidProto(String),

    #[error("missing field: {0}")]
    MissingField(String),

    #[error("model validation error: {0}")]
    ValidationError(#[from] ValidationError),
}
