use veilid_core::{KeyPair, SharedSecret};

use crate::{
    api::{
        Document, DocumentError, LARGE_SUBKEYS, MutableDocument, OpenDocument, Reference,
        TypedReference,
    },
    models::{
        AccountBio, AccountName, AccountPrivate, AccountPublic, DocumentType, Encrypted, Trace,
    },
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

#[derive(PartialEq, Debug, Clone, Eq)]
pub struct AccountDocument;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountView {
    pub public: AccountPublic,
    // None if identity not loaded or not the account owner.
    pub private: Option<AccountPrivate>,
}

pub enum AccountUpdate {
    Name(Option<AccountName>),
    Bio(Option<AccountBio>),
    Home(Option<Trace>),
    // TODO: private account updates (bookmarks, etc.)
}

impl Document for AccountDocument {
    const MAX_SUBKEYS: u16 = LARGE_SUBKEYS;
    const DOCUMENT_TYPE: DocumentType = DocumentType::Account;

    type View = AccountView;

    async fn read(
        reference: &Reference,
        identity: Option<&KeyPair>,
        force: bool,
        pool: &RecordPool,
    ) -> Result<AccountView, DocumentError> {
        let public: AccountPublic = pool
            .read(reference, 0, force)
            .await?
            .decrypt(reference.secret())?;
        let owner = identity.filter(|id| id.key() == *public.public_key());
        let private = match owner {
            None => None,
            Some(id) => match pool.read(reference, 1, force).await {
                Ok(encrypted) => Some(encrypted.decrypt(&private_encryption_key(id, reference))?),
                Err(RecordError::SubkeyEmpty(_)) => None,
                Err(e) => return Err(e.into()),
            },
        };
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
}

impl MutableDocument for AccountDocument {
    type Update = AccountUpdate;

    async fn update(
        update: AccountUpdate,
        doc: &OpenDocument<Self>,
        identity: &KeyPair,
        pool: &RecordPool,
    ) -> Result<(), DocumentError> {
        // most recent view. guaranteed to be fresh (network delays aside) since an OpenDocument always has a watch on the record.
        // clone so we don't hold the lock across a bunch of network operations.
        let view = doc.updates.borrow().clone()?;
        let public = view.public;
        let reference = doc.reference.reference();

        let updated = match update {
            AccountUpdate::Name(name) => AccountPublic { name, ..public },
            AccountUpdate::Bio(bio) => AccountPublic { bio, ..public },
            AccountUpdate::Home(home) => AccountPublic { home, ..public },
        };

        let encrypted = Encrypted::encrypt(&updated, reference.secret())?;
        pool.write(reference, 0, &encrypted, identity).await?;

        Ok(())
    }
}
