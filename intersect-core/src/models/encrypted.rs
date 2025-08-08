use base58::ToBase58;
use binrw::{
    binrw,
    meta::{ReadEndian, WriteEndian},
    BinRead, BinWrite,
};
use thiserror::Error;
use veilid_core::{Nonce, NONCE_LENGTH};

use crate::{
    rw_helpers::{BinReadAlloc, BinWriteAlloc},
    veilid::with_crypto,
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

#[binrw]
#[brw(big, magic = b"/??/")]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Encrypted {
    #[bw(map = |x| x.bytes)]
    #[br(map = |x: [u8; NONCE_LENGTH]| Nonce::new(x))]
    nonce: Nonce,

    // include ciphertext length in serialised format
    // this ensures unambigous decoding when many sections are concatenated
    #[br(temp)]
    #[bw(calc = ciphertext.len() as u32)]
    length: u32,

    #[br(count = length as usize)]
    ciphertext: Vec<u8>,
}

impl Encrypted {
    pub fn encrypt<T>(data: &T, shared_secret: &Secret) -> Result<Self, EncryptionError>
    where
        T: BinWrite + WriteEndian,
        for<'a> <T as BinWrite>::Args<'a>: Default,
    {
        let body = data.serialise();
        let (ciphertext, nonce) = with_crypto(|crypto| {
            let nonce = crypto.random_nonce();
            let ciphertext = crypto
                .encrypt_aead(&body, &nonce, shared_secret.key(), None)
                .map_err(|_| EncryptionError::EncryptionFailed)
                .unwrap();

            (ciphertext, nonce)
        });
        Ok(Encrypted { nonce, ciphertext })
    }

    pub fn encrypt_with_random<T>(data: &T) -> Result<(Self, Secret), EncryptionError>
    where
        T: BinWrite + WriteEndian,
        for<'a> <T as BinWrite>::Args<'a>: Default,
    {
        let key = Secret::random();
        let encrypted = Self::encrypt(data, &key)?;
        Ok((encrypted, key))
    }

    pub fn decrypt<T>(&self, shared_secret: &Secret) -> Result<T, EncryptionError>
    where
        T: BinRead + ReadEndian,
        for<'a> <T as BinRead>::Args<'a>: Default,
    {
        let bytes = with_crypto(|crypto| {
            crypto
                .decrypt_aead(&self.ciphertext, &self.nonce, shared_secret.key(), None)
                .map_err(|_| EncryptionError::DecryptionFailed)
                .unwrap()
        });
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

impl std::fmt::Display for Encrypted {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = self.to_bytes().to_base58();
        write!(fmt, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use binrw::NullString;

    use crate::init;

    use super::*;

    #[test]
    fn sdfsdfsdf() {
        tokio_test::block_on(init());

        let payload = Secret::random();

        let password = "hunter12";
        let key = with_crypto(|crypto| {
            crypto
                .derive_shared_secret(
                    password.as_bytes(),
                    crypto.generate_hash("salt".as_bytes()).bytes.as_slice(),
                )
                .unwrap()
                .into()
        });
        let encrypted = Encrypted::encrypt(&payload, &key).unwrap();
        println!("encrypted: {}", encrypted);
        let serialised = encrypted.serialise();
        println!("encrypted len (bytes): {}", serialised.len());

        // assert!(encrypted.validate());

        let deserialised = Encrypted::from_bytes(&serialised).unwrap();
        let decrypted = deserialised.decrypt(&key).unwrap();
        println!("decrypted: {}", decrypted);

        assert_eq!(payload, decrypted);
    }

    #[test]
    fn it_works() {
        tokio_test::block_on(init());

        let text: NullString = "hello!".into();

        let key = Secret::random();
        let encrypted = Encrypted::encrypt(&text, &key).unwrap();
        let serialised = encrypted.serialise();
        println!("encrypted (base64): {}", encrypted);

        // assert!(encrypted.validate());

        let deserialised = Encrypted::from_bytes(&serialised).unwrap();
        let decrypted = deserialised.decrypt(&key).unwrap();
        println!("decrypted: {:?}", decrypted);

        assert_eq!(text, decrypted);
    }
}
