use guard_clause::guard;
use veilid_core::{PublicKey, SecretKey};

use crate::{
    models::{Trace, ValidationError},
    proto,
    serialisation::{
        DeserialisationError, Deserialise, SerialisableV0, SerialisationError, Serialise,
        impl_string_conversions, impl_v0_proto_conversions,
    },
};

/// account display name, max 64 bytes
/// (note: distinct from 64 characters due to multi-byte unicode)
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountName(String);

impl AccountName {
    pub fn new(name: String) -> Result<Self, ValidationError> {
        guard!(
            name.len() <= 64,
            Err(ValidationError::TooLong(
                "name can be at most 64 bytes".to_string()
            ))
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
    pub public_key: PublicKey,
    pub name: Option<AccountName>,
    pub bio: Option<AccountBio>,
    pub home: Option<Trace>,
}

impl AccountPublic {
    pub fn new(
        public_key: PublicKey,
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

    pub fn public_key(&self) -> &PublicKey {
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
}

impl SerialisableV0 for AccountPublic {
    type Proto = proto::v0::intersect::AccountPublic;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            public_key: Some(proto::v0::veilid::PublicKey::from(self.public_key())),
            name: self.name().map(|n| n.as_ref().to_owned()),
            bio: self.bio().map(|b| b.as_ref().to_owned()),
            home: None, // TODO: implement home and add it here
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let public_key: PublicKey = proto
            .public_key
            .ok_or(DeserialisationError::MissingField("public_key".to_owned()))?
            .into();
        let name = proto.name.map(AccountName::new).transpose()?;
        let bio = proto.bio.map(AccountBio::new).transpose()?;
        let home: Option<Trace> = proto.home.map(TryInto::try_into).transpose()?;
        Ok(Self::new(public_key, name, bio, home))
    }
}

impl_v0_proto_conversions! {AccountPublic}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountPrivate {
    private_key: SecretKey,
    bookmarks: Option<Trace>,
}

impl AccountPrivate {
    pub fn new(private_key: SecretKey, bookmarks: Option<Trace>) -> Self {
        Self {
            private_key,
            bookmarks,
        }
    }

    pub fn private_key(&self) -> &SecretKey {
        &self.private_key
    }

    pub fn bookmarks(&self) -> Option<&Trace> {
        self.bookmarks.as_ref()
    }
}

impl SerialisableV0 for AccountPrivate {
    type Proto = proto::v0::intersect::AccountPrivate;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            private_key: Some(proto::v0::veilid::SecretKey::from(self.private_key())),
            bookmarks: self.bookmarks().map(TryInto::try_into).transpose()?,
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let private_key: SecretKey = proto
            .private_key
            .ok_or(DeserialisationError::MissingField("private_key".to_owned()))?
            .into();
        let bookmarks: Option<Trace> = proto.bookmarks.map(TryInto::try_into).transpose()?;
        Ok(Self::new(private_key, bookmarks))
    }
}

impl_v0_proto_conversions! {AccountPrivate}

/// an account's secret key, wrapped so it can be serialised/deserialised like other models.
/// this is what `create_account` returns and `login` accepts.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountSecret(SecretKey);

impl AccountSecret {
    pub fn new(secret: SecretKey) -> Self {
        Self(secret)
    }
}

impl AsRef<SecretKey> for AccountSecret {
    fn as_ref(&self) -> &SecretKey {
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
