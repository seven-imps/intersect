// message AccountPublic {
//     // full public key because it can't be derived from the memberid in the schema
//     veilid.PublicKey public_key = 1;
//     // TODO: limit to something short-ish in code. maybe 64 chars max?
//     optional string name = 2; // max 64 bytes
//     // TODO: make sure this is appropriately size limited in code to avoid  the 32kb total per subkey cap. say, 8kb max?
//     optional string bio = 3;
// }

use guard_clause::guard;
use veilid_core::{PublicKey, SecretKey};

use crate::{
    models::{Trace, ValidationError},
    proto,
    serialisation::{
        DeserialisationError, SerialisableV1, SerialisationError, impl_v1_proto_conversions,
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

impl SerialisableV1 for AccountPublic {
    type Proto = proto::v1::intersect::AccountPublic;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            public_key: Some(proto::v1::veilid::PublicKey::from(self.public_key())),
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

impl_v1_proto_conversions! {AccountPublic}

// message AccountPrivate {
//     // contains the private key so that the account password can be independent from the keypair
//     veilid.SecretKey private_key = 1;
//     // Links record of bookmarked traces
//     Trace bookmarks = 2;
// }

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountPrivate {
    private_key: SecretKey,
    bookmarks: Option<Trace>,
}

impl AccountPrivate {
    pub fn new(private_key: SecretKey, bookmarks: Option<Trace>) -> Result<Self, ValidationError> {
        Ok(Self {
            private_key,
            bookmarks,
        })
    }

    pub fn private_key(&self) -> &SecretKey {
        &self.private_key
    }

    pub fn bookmarks(&self) -> Option<&Trace> {
        self.bookmarks.as_ref()
    }
}

impl SerialisableV1 for AccountPrivate {
    type Proto = proto::v1::intersect::AccountPrivate;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            private_key: Some(proto::v1::veilid::SecretKey::from(self.private_key())),
            bookmarks: self.bookmarks().map(TryInto::try_into).transpose()?,
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let private_key: SecretKey = proto
            .private_key
            .ok_or(DeserialisationError::MissingField("private_key".to_owned()))?
            .into();
        let bookmarks: Option<Trace> = proto.bookmarks.map(TryInto::try_into).transpose()?;
        Ok(Self::new(private_key, bookmarks)?)
    }
}

impl_v1_proto_conversions! {AccountPrivate}
