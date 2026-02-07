use super::{Segment, Trace};
use crate::{
    proto,
    serialisation::{DeserialisationError, Deserialise, Serialise},
    FragmentRecord, IndexRecord, LinksRecord, Shard,
};
// use veilid_core::Timestamp;

#[derive(PartialEq, Debug, Clone)]
pub struct IndexMetadata {
    shard: Shard,

    // "folder" name
    name: Segment,

    // reference to the fragment for this index
    // using traces here lets us have unlocked, pwd protected, or locked links
    fragment: Option<Trace<FragmentRecord>>,

    // reference to the links for this index
    links: Option<Trace<LinksRecord>>,
    // timestamps
    // #[bw(map = |t| Timestamp::as_u64(t.clone()))]
    // #[br(map = Timestamp::new)]
    // created_at: Timestamp,
    // #[bw(map = |t| Timestamp::as_u64(t.clone()))]
    // #[br(map = Timestamp::new)]
    // last_updated: Timestamp,
}

impl IndexMetadata {
    pub fn new(shard: &Shard, name: &Segment) -> Self {
        IndexMetadata {
            shard: shard.clone(),
            name: name.clone(),
            fragment: None,
            links: None,
        }
    }

    pub fn shard(&self) -> &Shard {
        &self.shard
    }

    pub fn with_shard(&self, shard: &Shard) -> Self {
        IndexMetadata {
            shard: shard.clone(),
            ..self.clone()
        }
    }

    pub fn name(&self) -> &Segment {
        &self.name
    }

    pub fn with_name(&self, name: &Segment) -> Self {
        IndexMetadata {
            name: name.clone(),
            ..self.clone()
        }
    }

    pub fn fragment(&self) -> Option<&Trace<FragmentRecord>> {
        self.fragment.as_ref()
    }

    pub fn with_fragment(&self, fragment: &Trace<FragmentRecord>) -> Self {
        IndexMetadata {
            fragment: Some(fragment.clone()),
            ..self.clone()
        }
    }

    pub fn without_fragment(&self) -> Self {
        IndexMetadata {
            fragment: None,
            ..self.clone()
        }
    }

    pub fn links(&self) -> Option<&Trace<LinksRecord>> {
        self.links.as_ref()
    }

    pub fn with_links(&self, links: &Trace<LinksRecord>) -> Self {
        IndexMetadata {
            links: Some(links.clone()),
            ..self.clone()
        }
    }

    pub fn without_links(&self) -> Self {
        IndexMetadata {
            links: None,
            ..self.clone()
        }
    }
}

impl From<&IndexMetadata> for proto::intersect::v1::IndexMetadata {
    fn from(value: &IndexMetadata) -> Self {
        proto::intersect::v1::IndexMetadata {
            // nonce: Some(self.nonce.into()),
            // ciphertext: Some(self.ciphertext.clone()),
            shard: Some(value.shard.key().into()),
            name: Some(value.name.to_string()),
            fragment: value.fragment.map(Into::into),
            links: value.links.into(),
        }
    }
}

impl Serialise for IndexMetadata {
    fn serialise_v1_proto(&self) -> impl prost::Message {
        Into::<proto::intersect::v1::IndexMetadata>::into(self)
    }
}

impl Deserialise for IndexMetadata {
    fn deserialise_v1(bytes: &[u8]) -> Result<Self, DeserialisationError> {
        let proto = Self::deserialise_proto::<proto::intersect::v1::IndexMetadata>(bytes)?;

        let shard = Shard::from(
            proto
                .shard
                .ok_or(DeserialisationError::Failed("missing shard".to_owned()))?
                .into(),
        );

        let name = Segment::new(
            proto
                .name
                .ok_or(DeserialisationError::Failed("missing name".to_owned()))?,
        )
        .map_err(|_| DeserialisationError::Failed("invalid name".to_owned()))?;

        Ok(IndexMetadata {
            shard,
            name,
            fragment: proto.fragment.into(),
            links: proto.links.into(),
        })
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct LinkEntry {
    // "filename"
    name: Segment,
    // points at another index
    trace: Trace<IndexRecord>,
    // // metadata
    // #[bw(map = |t| Timestamp::as_u64(t.clone()))]
    // #[br(map = Timestamp::new)]
    // created_at: Timestamp,
    // #[bw(map = |t| Timestamp::as_u64(t.clone()))]
    // #[br(map = Timestamp::new)]
    // last_updated: Timestamp,
}

impl LinkEntry {
    pub fn new(name: &Segment, trace: &Trace<IndexRecord>) -> Self {
        LinkEntry {
            name: name.clone(),
            trace: trace.clone(),
        }
    }

    pub fn name(&self) -> &Segment {
        &self.name
    }

    pub fn trace(&self) -> &Trace<IndexRecord> {
        &self.trace
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::init;

//     use super::*;

//     #[test]
//     fn it_works() {
//         tokio_test::block_on(init());
//     }
// }
