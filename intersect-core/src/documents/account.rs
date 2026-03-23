use veilid_core::{KeyPair, SharedSecret};

use crate::{
    api::{Document, DocumentError, LARGE_SUBKEYS, Reference, TypedReference},
    models::{AccountPrivate, AccountPublic, Encrypted, RecordType, Trace},
    veilid::{RecordError, RecordPool, with_crypto},
};

// derive an encryption key from the identity private key.
// used for encrypting the private section of the account
fn private_encryption_key(identity: &KeyPair, reference: &Reference) -> SharedSecret {
    with_crypto(|c| {
        c.derive_shared_secret(
            identity.ref_bare_secret().bytes(),
            reference.record().ref_value().ref_key().bytes(),
        )
    })
    .expect("derive_shared_secret failed")
}

async fn read_private(
    identity: Option<&KeyPair>,
    reference: &Reference,
    pool: &RecordPool,
) -> Result<Option<AccountPrivate>, DocumentError> {
    let Some(id) = identity else { return Ok(None) };
    match pool.read(reference, 1).await {
        Ok(encrypted) => Ok(Some(
            encrypted.decrypt(&private_encryption_key(id, reference))?,
        )),
        Err(RecordError::SubkeyEmpty(_)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub struct AccountDocument;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountView {
    pub public: AccountPublic,
    // None if identity not loaded or not the account owner.
    pub private: Option<AccountPrivate>,
}

pub enum AccountUpdate {
    Name(Option<String>),
    Bio(Option<String>),
    Home(Option<Trace>),
    // TODO: private account updates (bookmarks, etc.)
}

impl Document for AccountDocument {
    const MUTABLE: bool = true;
    const MAX_SUBKEYS: u16 = LARGE_SUBKEYS;
    const RECORD_TYPE: RecordType = RecordType::Account;

    type View = AccountView;
    type Update = AccountUpdate;

    async fn read(
        reference: &Reference,
        identity: Option<&KeyPair>,
        pool: &RecordPool,
    ) -> Result<AccountView, DocumentError> {
        let public: AccountPublic = pool.read(reference, 0).await?.decrypt(reference.secret())?;
        let owner = identity.filter(|id| id.key() == *public.public_key());
        let private = read_private(owner, reference, pool).await?;
        Ok(AccountView { public, private })
    }

    async fn create(
        view: &AccountView,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> Result<TypedReference<AccountDocument>, DocumentError> {
        // validate identity keypair
        let keypair_valid =
            with_crypto(|c| c.validate_keypair(&identity.key(), &identity.secret()))
                .unwrap_or(false);
        if !keypair_valid {
            return Err(DocumentError::NotAuthorised);
        }

        // ensure private section is present
        let private = view.private.as_ref().ok_or(DocumentError::NotAuthorised)?;

        // ensure keys in view match identity
        if view.public.public_key() != &identity.key()
            || private.private_key() != &identity.secret()
        {
            return Err(DocumentError::NotAuthorised);
        }

        let record = pool.create(identity, Self::MAX_SUBKEYS).await?;
        let reference = Reference::new(record.descriptor.key(), record.secret);

        let public_encrypted = Encrypted::encrypt(&view.public, reference.secret())?;
        pool.write(&reference, 0, &public_encrypted, identity)
            .await?;

        let key = private_encryption_key(identity, &reference);
        let private_encrypted = Encrypted::encrypt(private, &key)?;
        pool.write(&reference, 1, &private_encrypted, identity)
            .await?;

        Ok(TypedReference::new(reference))
    }

    async fn update(
        update: AccountUpdate,
        reference: &Reference,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> Result<(), DocumentError> {
        // read current public, apply change, write back
        let public: AccountPublic = pool.read(reference, 0).await?.decrypt(reference.secret())?;

        let updated = match update {
            AccountUpdate::Name(name) => AccountPublic::new(
                public.public_key().clone(),
                name,
                public.bio().cloned(),
                public.home().cloned(),
            ),
            AccountUpdate::Bio(bio) => AccountPublic::new(
                public.public_key().clone(),
                public.name().cloned(),
                bio,
                public.home().cloned(),
            ),
            AccountUpdate::Home(home) => AccountPublic::new(
                public.public_key().clone(),
                public.name().cloned(),
                public.bio().cloned(),
                home,
            ),
        }
        .map_err(DocumentError::from)?;

        let encrypted = Encrypted::encrypt(&updated, reference.secret())?;
        pool.write(reference, 0, &encrypted, identity).await?;

        Ok(())
    }
}
