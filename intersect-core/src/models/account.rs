use guard_clause::guard;
use veilid_core::{PublicKey, SecretKey};

use crate::{
    models::{Trace, ValidationError},
    proto,
    serialisation::{
        DeserialisationError, Deserialise, SerialisableV0, SerialisationError, Serialise,
        impl_string_conversions, impl_v0_proto_conversions,
    },
    veilid::with_crypto,
};

/// the public half of an account's signing keypair.
/// wrapper so we don't expose the raw veilid PublicKey type in our public API.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountPublicKey(PublicKey);

impl AccountPublicKey {
    pub(crate) fn new(key: PublicKey) -> Self {
        Self(key)
    }

    pub(crate) fn inner(&self) -> &PublicKey {
        &self.0
    }

    /// hashes the crypto kind + bare key bytes to produce a stable 128-bit fingerprint.
    /// intended for display alongside usernames (which are not unique) to aid disambiguation.
    pub fn fingerprint_bytes(&self) -> [u8; 16] {
        let mut data = Vec::with_capacity(36);
        data.extend_from_slice(self.0.kind().bytes()); // 4 bytes: crypto kind (domain separation)
        data.extend_from_slice(&self.0.value()); // 32 bytes: bare public key
        let hash = with_crypto(|c| c.generate_hash(&data));
        hash.value()[..16].try_into().unwrap()
    }

    /// base58-encoded fingerprint for display
    pub fn fingerprint(&self) -> String {
        bs58::encode(self.fingerprint_bytes()).into_string()
    }
}

impl SerialisableV0 for AccountPublicKey {
    type Proto = proto::v0::intersect::AccountPublicKey;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            key: Some(proto::v0::veilid::PublicKey::from(&self.0)),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let key = proto
            .key
            .ok_or(DeserialisationError::MissingField("key".to_owned()))?;
        Ok(Self(key.into()))
    }
}

impl_v0_proto_conversions! {AccountPublicKey}
impl_string_conversions! {AccountPublicKey}

const ACCOUNT_NAME_MAX_BYTES: usize = 64;

/// account display name, max 64 bytes
/// (note: distinct from 64 characters due to multi-byte unicode)
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountName(String);

impl AccountName {
    pub fn new(name: String) -> Result<Self, ValidationError> {
        guard!(
            name.len() <= ACCOUNT_NAME_MAX_BYTES,
            Err(ValidationError::TooLong(format!(
                "name can be at most {ACCOUNT_NAME_MAX_BYTES} bytes"
            )))
        );
        Ok(Self(name))
    }
}

impl AsRef<str> for AccountName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// account bio, max 8KiB
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountBio(String);

impl AccountBio {
    pub fn new(bio: String) -> Result<Self, ValidationError> {
        guard!(
            bio.len() <= 8 * 1024,
            Err(ValidationError::TooLong(
                "bio can be at most 8 kilobytes".to_string()
            ))
        );
        Ok(Self(bio))
    }
}

impl AsRef<str> for AccountBio {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountPublic {
    public_key: AccountPublicKey,
    name: Option<AccountName>,
    bio: Option<AccountBio>,
    home: Option<Trace>,
}

impl AccountPublic {
    pub fn new(
        public_key: AccountPublicKey,
        name: Option<AccountName>,
        bio: Option<AccountBio>,
        home: Option<Trace>,
    ) -> Self {
        Self {
            public_key,
            name,
            bio,
            home,
        }
    }

    pub fn public_key(&self) -> &AccountPublicKey {
        &self.public_key
    }
    pub fn name(&self) -> Option<&AccountName> {
        self.name.as_ref()
    }
    pub fn bio(&self) -> Option<&AccountBio> {
        self.bio.as_ref()
    }
    pub fn home(&self) -> Option<&Trace> {
        self.home.as_ref()
    }

    pub fn with_name(self, name: Option<AccountName>) -> Self {
        Self { name, ..self }
    }
    pub fn with_bio(self, bio: Option<AccountBio>) -> Self {
        Self { bio, ..self }
    }
    pub fn with_home(self, home: Option<Trace>) -> Self {
        Self { home, ..self }
    }
}

impl SerialisableV0 for AccountPublic {
    type Proto = proto::v0::intersect::AccountPublic;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            public_key: Some(proto::v0::veilid::PublicKey::from(self.public_key.inner())),
            name: self.name().map(|n| n.as_ref().to_owned()),
            bio: self.bio().map(|b| b.as_ref().to_owned()),
            home: None, // TODO: implement home and add it here
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let public_key = AccountPublicKey::new(
            proto
                .public_key
                .ok_or(DeserialisationError::MissingField("public_key".to_owned()))?
                .into(),
        );
        let name = proto.name.map(AccountName::new).transpose()?;
        let bio = proto.bio.map(AccountBio::new).transpose()?;
        let home: Option<Trace> = proto.home.map(TryInto::try_into).transpose()?;
        Ok(Self::new(public_key, name, bio, home))
    }
}

impl_v0_proto_conversions! {AccountPublic}

#[derive(PartialEq, Eq, Debug, Clone)]
// TODO: private account data is currently exposed as-is in AccountView.
// ideally we'd either inline its fields directly into the view or gate access more carefully.
pub struct AccountPrivate {
    bookmarks: Option<Trace>,
}

impl AccountPrivate {
    pub fn new(bookmarks: Option<Trace>) -> Self {
        Self { bookmarks }
    }

    pub fn bookmarks(&self) -> Option<&Trace> {
        self.bookmarks.as_ref()
    }
}

impl SerialisableV0 for AccountPrivate {
    type Proto = proto::v0::intersect::AccountPrivate;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            bookmarks: self.bookmarks().map(TryInto::try_into).transpose()?,
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let bookmarks: Option<Trace> = proto.bookmarks.map(TryInto::try_into).transpose()?;
        Ok(Self::new(bookmarks))
    }
}

impl_v0_proto_conversions! {AccountPrivate}

/// an account's secret key, wrapped so it can be serialised/deserialised like other models.
/// this is what `create_account` returns and `login` accepts.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountSecret(SecretKey);

impl AccountSecret {
    pub(crate) fn new(secret: SecretKey) -> Self {
        Self(secret)
    }

    pub(crate) fn inner(&self) -> &SecretKey {
        &self.0
    }
}

impl SerialisableV0 for AccountSecret {
    type Proto = proto::v0::intersect::AccountSecret;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            secret: Some(proto::v0::veilid::SecretKey::from(&self.0)),
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let secret: SecretKey = proto
            .secret
            .ok_or(DeserialisationError::MissingField("secret".to_owned()))?
            .into();
        Ok(Self(secret))
    }
}

impl_v0_proto_conversions! {AccountSecret}
impl_string_conversions! {AccountSecret}
