use guard_clause::guard;

use crate::{
    models::{Trace, ValidationError},
    proto,
    serialisation::{
        DeserialisationError, SerialisableV0, SerialisationError, impl_v0_proto_conversions,
    },
};

const INDEX_NAME_MAX_BYTES: usize = 256;

/// display name for an index document with length validation
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IndexName(String);

impl IndexName {
    pub fn new(name: String) -> Result<Self, ValidationError> {
        guard!(
            name.len() <= INDEX_NAME_MAX_BYTES,
            Err(ValidationError::TooLong(format!(
                "name can be at most {INDEX_NAME_MAX_BYTES} bytes"
            )))
        );
        Ok(Self(name))
    }
}

impl AsRef<str> for IndexName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IndexHeader {
    // user-readable name for the index, max 256 bytes
    name: IndexName,
    // author's account trace, unset for anonymous indexes
    author: Option<Trace>,
    // reference to the content fragment, if any
    fragment: Option<Trace>,
    // reference to the links record, if any
    links: Option<Trace>,
}

impl IndexHeader {
    pub fn new(
        name: IndexName,
        author: Option<Trace>,
        fragment: Option<Trace>,
        links: Option<Trace>,
    ) -> Self {
        Self {
            name,
            author,
            fragment,
            links,
        }
    }

    pub fn name(&self) -> &IndexName {
        &self.name
    }
    pub fn author(&self) -> Option<&Trace> {
        self.author.as_ref()
    }
    pub fn fragment(&self) -> Option<&Trace> {
        self.fragment.as_ref()
    }
    pub fn links(&self) -> Option<&Trace> {
        self.links.as_ref()
    }
}

impl SerialisableV0 for IndexHeader {
    type Proto = proto::v0::intersect::IndexHeader;

    fn to_proto(&self) -> Result<Self::Proto, SerialisationError> {
        Ok(Self::Proto {
            name: self.name.as_ref().to_owned(),
            author: self.author.as_ref().map(TryInto::try_into).transpose()?,
            fragment: self.fragment.as_ref().map(TryInto::try_into).transpose()?,
            links: self.links.as_ref().map(TryInto::try_into).transpose()?,
        })
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, DeserialisationError> {
        Ok(Self {
            name: IndexName::new(proto.name)?,
            author: proto.author.map(TryInto::try_into).transpose()?,
            fragment: proto.fragment.map(TryInto::try_into).transpose()?,
            links: proto.links.map(TryInto::try_into).transpose()?,
        })
    }
}

impl_v0_proto_conversions! {IndexHeader}
