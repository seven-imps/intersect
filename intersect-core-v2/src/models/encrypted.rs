use guard_clause::guard;
use thiserror::Error;
use veilid_core::{Nonce, SharedSecret};

use crate::{
    proto,
    serialisation::{
        DeserialisationError, Deserialise, SerialisableV1, SerialisationError, Serialise,
        impl_v1_proto_conversions,
    },
    veilid::Connection,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Encrypted {
    nonce: Nonce,
    ciphertext: Vec<u8>,
}

impl Encrypted {
    pub fn encrypt<T: Serialise>(
        data: &T,
        shared_secret: &SharedSecret,
        connection: &Connection,
    ) -> Result<Self, EncryptionError> {
        let body = data.serialise()?;
        let nonce = connection.with_crypto(|c| c.random_nonce());
        let ciphertext = connection
            .with_crypto(|c| c.encrypt_aead(&body, &nonce, shared_secret, None))
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;
        Ok(Encrypted { nonce, ciphertext })
    }

    pub fn encrypt_with_random<T: Serialise>(
        data: &T,
        connection: &Connection,
    ) -> Result<(Self, SharedSecret), EncryptionError> {
        let key = connection.with_crypto(|c| c.random_shared_secret());
        let encrypted = Self::encrypt(data, &key, connection)?;
        Ok((encrypted, key))
    }

    pub fn encrypt_with_password<T: Serialise>(
        data: &T,
        password: &str,
        salt: &[u8],
        connection: &Connection,
    ) -> Result<(Self, SharedSecret), EncryptionError> {
        // i don't care to enforce any password rules other than length.
        // if you wanna throw multi-byte emojis or obscure scripts in there then go wild. as long as it's valid utf-8 i'm happy.
        guard!(
            // using str.chars().count() instead of str.len() to avoid overestimating the entropy of multi-byte characters.
            password.chars().count() >= 15,
            Err(EncryptionError::PasswordTooWeak(
                "must be at least 15 characters".to_string()
            ))
        );

        let key = connection
            .with_crypto(|c| c.derive_shared_secret(password.as_bytes(), salt))
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;
        let encrypted = Self::encrypt(data, &key, connection)?;
        Ok((encrypted, key))
    }

    pub fn decrypt<T: Deserialise>(
        &self,
        shared_secret: &SharedSecret,
        connection: &Connection,
    ) -> Result<T, EncryptionError> {
        let bytes = connection
            .with_crypto(|c| c.decrypt_aead(&self.ciphertext, &self.nonce, shared_secret, None))
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;
        Ok(T::deserialise(&bytes)?)
    }
}

impl SerialisableV1 for Encrypted {
    type Proto = proto::v1::intersect::Encrypted;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            nonce: Some(proto::v1::veilid::Nonce::from(&self.nonce)),
            ciphertext: self.ciphertext.clone(),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let nonce = Nonce::new(
            &proto
                .nonce
                .ok_or(DeserialisationError::MissingField("nonce".to_owned()))?
                .data,
        );
        Ok(Self {
            nonce,
            ciphertext: proto.ciphertext,
        })
    }
}

impl_v1_proto_conversions! {Encrypted}

#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EncryptionError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Serialisation failed: {0}")]
    SerialisationFailed(#[from] SerialisationError),
    #[error("deserialisation failed: {0}")]
    DeserialisationFailed(#[from] DeserialisationError),
    #[error("password is too weak: {0}")]
    PasswordTooWeak(String),
}

#[cfg(test)]
mod tests {

    // use crate::api::Intersect;

    // use super::*;

    // #[test]
    // fn sdfsdfsdf() {
    //     let payload = "[redacted]";

    //     let password = "hunter12";
    //     let salt = "salt".as_bytes();

    //     let encrypted =
    //         Encrypted::encrypt_with_password(&payload, password, salt, &connection).unwrap();
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
