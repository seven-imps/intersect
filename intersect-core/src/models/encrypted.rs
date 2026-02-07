use base58::ToBase58;
use prost::Message;
use thiserror::Error;
use veilid_core::Nonce;

use crate::{
    proto,
    serialisation::{DeserialisationError, Deserialise, Serialise},
    veilid::get_crypto,
    Secret,
};

#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EncryptionError {
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("deserialisation failed: {0}")]
    DeserialisationFailed(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Encrypted {
    nonce: Nonce,
    ciphertext: Vec<u8>,
}

impl Encrypted {
    pub fn encrypt<T: Serialise>(
        data: &T,
        shared_secret: &Secret,
    ) -> Result<Self, EncryptionError> {
        let body = data.serialise();
        let nonce = get_crypto().random_nonce();
        let ciphertext = get_crypto()
            .encrypt_aead(&body, &nonce, shared_secret.key(), None)
            .map_err(|_| EncryptionError::EncryptionFailed)?;
        Ok(Encrypted { nonce, ciphertext })
    }

    pub fn encrypt_with_random<T: Serialise>(data: &T) -> Result<(Self, Secret), EncryptionError> {
        let key = Secret::random();
        let encrypted = Self::encrypt(data, &key)?;
        Ok((encrypted, key))
    }

    pub fn decrypt<T: Deserialise>(&self, shared_secret: &Secret) -> Result<T, EncryptionError> {
        let bytes = get_crypto()
            .decrypt_aead(&self.ciphertext, &self.nonce, shared_secret.key(), None)
            .map_err(|_| EncryptionError::DecryptionFailed)?;
        T::deserialise(&bytes).map_err(|e| EncryptionError::DeserialisationFailed(e.to_string()))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EncryptionError> {
        Self::deserialise(bytes).map_err(|e| EncryptionError::DeserialisationFailed(e.to_string()))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.serialise()
    }

    pub fn nonce(&self) -> &Nonce {
        &self.nonce
    }
}

impl Serialise for Encrypted {
    fn serialise_v1_proto(&self) -> impl Message {
        proto::intersect::v1::Encrypted {
            nonce: Some(self.nonce().into()),
            ciphertext: Some(self.ciphertext.clone()),
        }
    }
}

impl Deserialise for Encrypted {
    fn deserialise_v1(bytes: &[u8]) -> Result<Self, DeserialisationError> {
        let proto = Self::deserialise_proto::<proto::intersect::v1::Encrypted>(bytes)?;

        Ok(Encrypted {
            nonce: proto
                .nonce
                .ok_or(DeserialisationError::Failed("missing nonce".to_owned()))?
                .into(),
            ciphertext: proto
                .ciphertext
                .ok_or(DeserialisationError::Failed(
                    "missing ciphertext".to_owned(),
                ))?
                .into(),
        })
    }
}

impl std::fmt::Display for Encrypted {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = self.to_bytes().to_base58();
        write!(fmt, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    // use crate::init;

    // use super::*;

    // #[test]
    // fn sdfsdfsdf() {
    //     tokio_test::block_on(init());

    //     let payload = Secret::random();

    //     let password = "hunter12";
    //     let key = get_crypto()
    //         .derive_shared_secret(
    //             password.as_bytes(),
    //             get_crypto().generate_hash("salt".as_bytes()).as_slice(),
    //         )
    //         .unwrap()
    //         .into();
    //     let encrypted = Encrypted::encrypt(&payload, &key).unwrap();
    //     println!("encrypted: {}", encrypted);
    //     let serialised = encrypted.serialise();
    //     println!("encrypted len (bytes): {}", serialised.len());

    //     // assert!(encrypted.validate());

    //     let deserialised = Encrypted::from_bytes(&serialised).unwrap();
    //     let decrypted = deserialised.decrypt(&key).unwrap();
    //     println!("decrypted: {}", decrypted);

    //     assert_eq!(payload, decrypted);
    // }

    // #[test]
    // fn it_works() {
    //     tokio_test::block_on(init());

    //     let text: NullString = "hello!".into();

    //     let key = Secret::random();
    //     let encrypted = Encrypted::encrypt(&text, &key).unwrap();
    //     let serialised = encrypted.serialise();
    //     println!("encrypted (base64): {}", encrypted);

    //     // assert!(encrypted.validate());

    //     let deserialised = Encrypted::from_bytes(&serialised).unwrap();
    //     let decrypted = deserialised.decrypt(&key).unwrap();
    //     println!("decrypted: {:?}", decrypted);

    //     assert_eq!(text, decrypted);
    // }
}
