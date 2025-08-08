use crate::veilid::with_crypto;
use base58::FromBase58;
use base58::ToBase58;
use binrw::binrw;
use std::str::FromStr;
use thiserror::Error;
use veilid_core::RecordKey;
use veilid_core::SecretKey;
use veilid_core::{HashDigest, KeyPair, PublicKey, SharedSecret};

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum KeyError {
    #[error("deserialisation failed")]
    DeserialisationFailed,
}

// yay macros
// see explanation below
macro_rules! wrap_key_type {
    ($new_type:ident, $wrapped_type:ty) => {
        #[binrw]
        #[brw(big)]
        #[derive(PartialEq, Debug, Clone, Hash, Eq, Copy)]
        #[bw(map = $new_type::bytes_cloned)]
        #[br(map = $new_type::from_bytes)]
        pub struct $new_type($wrapped_type);

        impl $new_type {
            pub fn bytes(&self) -> &[u8; 32] {
                &self.0.bytes
            }

            pub fn bytes_owned(self) -> [u8; 32] {
                self.0.bytes
            }

            pub fn bytes_cloned(&self) -> [u8; 32] {
                self.0.bytes
            }

            pub fn from_bytes(bytes: [u8; 32]) -> Self {
                $new_type(<$wrapped_type>::new(bytes))
            }

            pub fn as_slice(&self) -> &[u8] {
                self.bytes().as_slice()
            }

            pub fn key(&self) -> &$wrapped_type {
                &self.0
            }
        }

        impl From<$new_type> for $wrapped_type {
            fn from(value: $new_type) -> Self {
                value.0
            }
        }

        impl From<&$new_type> for $wrapped_type {
            fn from(value: &$new_type) -> Self {
                value.0
            }
        }

        impl From<$wrapped_type> for $new_type {
            fn from(value: $wrapped_type) -> Self {
                $new_type(value.clone())
            }
        }

        impl From<&$wrapped_type> for $new_type {
            fn from(value: &$wrapped_type) -> Self {
                $new_type(value.clone())
            }
        }

        impl TryFrom<&str> for $new_type {
            type Error = KeyError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value
                    .from_base58()
                    .map_err(|_| KeyError::DeserialisationFailed)
                    .and_then(|s| s.try_into().map_err(|_| KeyError::DeserialisationFailed))
                    .and_then(|s| Ok($new_type(<$wrapped_type>::new(s))))
                    .map_err(|_| KeyError::DeserialisationFailed)
            }
        }

        impl FromStr for $new_type {
            type Err = KeyError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.try_into()
            }
        }

        impl std::fmt::Display for $new_type {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(fmt, "{}", self.bytes().to_base58())
            }
        }
    };
}

// we do all this instead of just using the veilid types for a few reasons:
//   - so we can wrap it in #[binrw]
//   - so i can implement my own (de)serialisation
wrap_key_type!(Shard, PublicKey);
wrap_key_type!(PrivateKey, SecretKey);
wrap_key_type!(Secret, SharedSecret);
wrap_key_type!(Hash, HashDigest);
wrap_key_type!(VeilidRecordKey, RecordKey);

// and then some stuff that isn't shared between all the variants

impl Secret {
    pub fn random() -> Self {
        with_crypto(|crypto| crypto.random_shared_secret().into())
    }
}

// oh, and a type to replace KeyPair as well

#[derive(PartialEq, Debug, Clone, Hash, Eq, Copy)]
pub struct Identity {
    shard: Shard,
    private_key: PrivateKey,
}

impl Identity {
    pub fn new(shard: Shard, private_key: PrivateKey) -> Result<Self, InvalidKeypair> {
        // validate keypair!!
        if with_crypto(|crypto| crypto.validate_keypair(&shard.into(), &private_key.into())) {
            Ok(Identity { shard, private_key })
        } else {
            Err(InvalidKeypair)
        }
    }

    pub fn random() -> Self {
        let keypair = with_crypto(|crypto| crypto.generate_keypair());
        // if this unwrap fails we're in deep shit
        Identity::new(keypair.key.into(), keypair.secret.into()).unwrap()
    }

    pub fn shard(&self) -> &Shard {
        &self.shard
    }

    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    pub fn as_keypair(&self) -> KeyPair {
        KeyPair::new(self.shard().into(), self.private_key().into())
    }
}

#[derive(Error, Debug, Clone)]
#[error("invalid identity keypair")]
pub struct InvalidKeypair;
