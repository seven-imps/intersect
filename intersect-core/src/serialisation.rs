use prost::Message;
use thiserror::Error;

const MAGIC: &[u8] = b"/?/";

#[repr(u8)]
pub enum Version {
    V1 = 1,
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
    fn serialise(&self) -> Vec<u8> {
        // always serialise with the latest version
        let version = Version::V1; // update when adding new version
        let proto_bytes = match version {
            Version::V1 => self.serialise_v1(),
        };
        // prefix protobuf bytes with magic bytes and version number
        [MAGIC, &[version as u8], proto_bytes.as_slice()].concat()
    }

    fn serialise_v1(&self) -> Vec<u8> {
        // v1 uses proto, but technically a new version wouldn't even have to use proto
        self.serialise_v1_proto().encode_length_delimited_to_vec()
    }

    fn serialise_v1_proto(&self) -> impl Message;

    // when adding a new version, just add a new `serialise_vX` function to this trait.
    // (and update the version used in `serialise`)
    // this will force you to add it to all existing impls. this is the point
}

pub trait Deserialise
where
    Self: Sized,
{
    fn deserialise(bytes: &[u8]) -> Result<Self, DeserialisationError> {
        // validate the magic bytes
        let (magic, rest) = bytes.split_at(MAGIC.len());
        if magic != MAGIC {
            return Err(DeserialisationError::InvalidMagic);
        }
        // deserialise whatever version we end up finding
        let version_byte = *rest.first().ok_or(DeserialisationError::InvalidVersion)?;
        let proto_bytes = &rest[1..];
        match Version::try_from(version_byte)? {
            Version::V1 => Self::deserialise_v1(proto_bytes),
        }
    }

    fn deserialise_v1(bytes: &[u8]) -> Result<Self, DeserialisationError>;

    // just a lil helper so we get consistent proto deserialisation
    fn deserialise_proto<M: Message + Default>(bytes: &[u8]) -> Result<M, DeserialisationError> {
        M::decode_length_delimited(bytes).map_err(|_| DeserialisationError::InvalidProto)
    }

    // same here. when adding a new version, just add a new function.
    // no need to update anything else, since deserialisation should support all versions.
    // always *write* the newest version and *read* any supported versions
}

// #[derive(Error, Debug, Clone)]
// #[non_exhaustive]
// pub enum SerialisationError {
//     #[error("serialisation failed: {0}")]
//     SerialisationFailed(String),
// }

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum DeserialisationError {
    #[error("failed with error: {0}")]
    Failed(String),
    #[error("invalid version")]
    InvalidVersion,
    #[error("invalid magic bytes")]
    InvalidMagic,
    #[error("invalid proto")]
    InvalidProto,
}
