use thiserror::Error;
use veilid_core::{Nonce, SharedSecret};

use crate::{
    models::{Encrypted, EncryptionError},
    serialisation::{
        DeserialisationError, SerialisableV0, SerialisationError, impl_v0_proto_conversions,
    },
    veilid::with_crypto,
};

// TODO: this shouldn't derive debug.
// we may want a custom impl here with redacted secrets
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Access {
    Locked,
    Unlocked { secret: SharedSecret },
    Protected { protected_secret: ProtectedSecret },
}

impl Access {
    pub(crate) fn new_locked() -> Self {
        Self::Locked
    }

    pub(crate) fn new_unlocked(secret: &SharedSecret) -> Self {
        Self::Unlocked {
            secret: secret.clone(),
        }
    }

    pub(crate) fn new_protected(
        secret: &SharedSecret,
        password: &str,
    ) -> Result<Self, EncryptionError> {
        let protected = ProtectedSecret::new(secret, password)?;
        Ok(Self::Protected {
            protected_secret: protected,
        })
    }

    // TODO: accessors? (how) do we handle that here?
}

impl SerialisableV0 for Access {
    type Proto = crate::proto::v0::intersect::Access;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        let access_level = match self {
            Self::Locked => crate::proto::v0::intersect::access::AccessLevel::Locked(
                crate::proto::v0::intersect::access::Locked {},
            ),
            Self::Unlocked { secret } => {
                crate::proto::v0::intersect::access::AccessLevel::Unlocked(
                    crate::proto::v0::intersect::access::Unlocked {
                        secret: Some(secret.into()),
                    },
                )
            }
            Self::Protected { protected_secret } => {
                crate::proto::v0::intersect::access::AccessLevel::Protected(
                    protected_secret.to_proto()?,
                )
            }
        };
        Ok(Self::Proto {
            access_level: Some(access_level),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let access_level = proto
            .access_level
            .ok_or(DeserialisationError::MissingField(
                "access_level".to_owned(),
            ))?;
        match access_level {
            crate::proto::v0::intersect::access::AccessLevel::Locked(_) => Ok(Self::Locked),
            crate::proto::v0::intersect::access::AccessLevel::Unlocked(unlocked) => {
                Ok(Self::Unlocked {
                    secret: unlocked
                        .secret
                        .ok_or(DeserialisationError::MissingField("secret".to_owned()))?
                        .into(),
                })
            }
            crate::proto::v0::intersect::access::AccessLevel::Protected(protected) => {
                Ok(Self::Protected {
                    protected_secret: ProtectedSecret::from_proto(protected)?,
                })
            }
        }
    }
}

impl_v0_proto_conversions! {Access}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProtectedSecret {
    salt: Nonce,
    encrypted_secret: Encrypted,
}

impl ProtectedSecret {
    pub fn new(secret: &SharedSecret, password: &str) -> Result<Self, EncryptionError> {
        let salt = with_crypto(|c| c.random_nonce());
        let (encrypted, _secret) = Encrypted::encrypt_with_password(secret, password, &salt)?;
        Ok(Self {
            salt,
            encrypted_secret: encrypted,
        })
    }

    pub fn unlock(&self, password: &str) -> Result<SharedSecret, AccessError> {
        let secret = self
            .encrypted_secret
            .decrypt_with_password(password, &self.salt)
            .map_err(|_| AccessError::WrongPassword)?;
        Ok(secret)
    }
}

impl SerialisableV0 for ProtectedSecret {
    type Proto = crate::proto::v0::intersect::access::Protected;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            salt: Some((&self.salt).into()),
            encrypted_secret: Some(self.encrypted_secret.to_proto()?),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let salt: Nonce = proto
            .salt
            .ok_or(DeserialisationError::MissingField("salt".to_owned()))?
            .into();
        let encrypted_secret = proto
            .encrypted_secret
            .ok_or(DeserialisationError::MissingField(
                "encrypted_secret".to_owned(),
            ))?;
        Ok(Self {
            salt,
            encrypted_secret: Encrypted::from_proto(encrypted_secret)?,
        })
    }
}

impl_v0_proto_conversions! {ProtectedSecret}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum AccessError {
    #[error("incorrect password")]
    WrongPassword,

    #[error("encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),
}
