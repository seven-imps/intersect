use base58::{FromBase58, ToBase58};
use binrw::binrw;
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    rw_helpers::{BinReadAlloc, BinWriteAlloc},
    veilid::get_crypto,
    Secret, Shard,
};

use super::{Encrypted, EncryptionError};

#[binrw]
#[brw(big)]
#[derive(PartialEq, Clone, Eq)]
pub enum Access {
    #[brw(magic = 1u8)]
    Locked,

    #[brw(magic = 2u8)]
    Unlocked(Secret),

    #[brw(magic = 3u8)]
    Protected(ProtectedSecret),
}

#[binrw]
#[derive(PartialEq, Clone, Eq)]
pub struct ProtectedSecret(Encrypted);

impl ProtectedSecret {
    pub fn new(shard: &Shard, password: &str, secret: &Secret) -> Result<Self, AccessError> {
        let password_hash = Self::password_hash(shard, password)?;
        let encrypted = Encrypted::encrypt(secret, &password_hash)?;
        Ok(Self(encrypted))
    }

    pub fn unlock(self, shard: &Shard, password: &str) -> Result<Secret, AccessError> {
        let password_hash = Self::password_hash(shard, password)?;
        let secret = self
            .0
            .decrypt(&password_hash)
            .map_err(|_| AccessError::InvalidPassword)?;
        Ok(secret)
    }

    fn password_hash(shard: &Shard, password: &str) -> Result<Secret, AccessError> {
        // validate
        // (nothing fancy, just don't make them too short or oops-some-parser-is-oom long)
        let len = password.graphemes(true).count();
        if len < 15 || len > 64 {
            return Err(AccessError::InvalidPassword);
        }
        // hash
        let hash = get_crypto()
            .derive_shared_secret(password.as_bytes(), shard.as_slice())
            .unwrap();
        Ok(hash.into())
    }
}

impl std::fmt::Display for Access {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.serialise().to_base58())
    }
}

impl TryFrom<&str> for Access {
    type Error = AccessError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = value
            .from_base58()
            .map_err(|_| AccessError::DeserialisationFailed("invalid base58".to_owned()))?;

        Self::deserialise(bytes.as_slice())
            .map_err(|e| AccessError::DeserialisationFailed(e.to_string()))
    }
}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum AccessError {
    #[error("missing key")]
    WrongPassword,

    #[error("invalid key")]
    InvalidPassword,

    #[error("encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),

    #[error("deserialisation failed: {0}")]
    DeserialisationFailed(String),
}
