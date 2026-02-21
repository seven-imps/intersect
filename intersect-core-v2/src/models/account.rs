// message AccountPublic {
//     // full public key because it can't be derived from the memberid in the schema
//     veilid.PublicKey public_key = 1;
//     // TODO: limit to something short-ish in code. maybe 64 chars max?
//     optional string name = 2; // max 64 bytes
//     // TODO: make sure this is appropriately size limited in code to avoid  the 32kb total per subkey cap. say, 8kb max?
//     optional string bio = 3;
// }

use guard_clause::guard;
use veilid_core::PublicKey;

use crate::{
    models::{Trace, ValidationError},
    proto,
    serialisation::{
        DeserialisationError, SerialisableV1, SerialisationError, impl_v1_proto_conversions,
    },
};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct AccountPublic {
    public_key: PublicKey,
    name: Option<String>,
    bio: Option<String>,
    home: Option<Trace>,
}

impl AccountPublic {
    pub fn new(
        public_key: PublicKey,
        name: Option<String>,
        bio: Option<String>,
        home: Option<Trace>,
    ) -> Result<Self, ValidationError> {
        // max 64 bytes for name
        // (note: this is distinct from 64 characters because of multi-byte characters!)
        guard!(
            name.as_ref().is_none_or(|n| n.len() <= 64),
            Err(ValidationError::TooLong(
                "name can be at most 64 bytes".to_string()
            ))
        );

        // max 8KiB for bio
        guard!(
            bio.as_ref().is_none_or(|n| n.len() <= 8 * 1024),
            Err(ValidationError::TooLong(
                "bio can be at most 8 kilobytes".to_string()
            ))
        );

        Ok(Self {
            public_key,
            name,
            bio,
            home,
        })
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    pub fn bio(&self) -> Option<&String> {
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
            name: self.name().cloned(),
            bio: self.bio().cloned(),
            home: None, // TODO: implement home and add it here
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        let public_key: PublicKey = proto
            .public_key
            .ok_or(DeserialisationError::MissingField("public_key".to_owned()))?
            .into();
        let home: Option<Trace> = proto.home.map(TryInto::try_into).transpose()?;
        Ok(Self::new(public_key, proto.name, proto.bio, home)?)
    }
}

impl_v1_proto_conversions! {AccountPublic}

// message AccountPrivate {
//     // contains the private key so that the account password can be independent from the keypair
//     veilid.SecretKey private_key = 1;
//     // Links record of bookmarked traces
//     Trace bookmarks = 2;
// }

// TODO: implement AccountPrivate
