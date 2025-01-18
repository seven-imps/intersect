use super::{Segment, Trace};
use crate::{rw_helpers::RWOption, FragmentRecord, IndexRecord, LinksRecord, Shard};
use binrw::binrw;
// use veilid_core::Timestamp;

#[binrw]
#[brw(big)]
#[derive(PartialEq, Debug, Clone)]
pub struct IndexMetadata {
    shard: Shard,

    // "folder" name
    name: Segment,

    // reference to the fragment for this index
    #[bw(map = RWOption::from)]
    #[br(map = RWOption::into)]
    // using traces here lets us have unlocked, pwd protected, or locked links
    fragment: Option<Trace<FragmentRecord>>,

    // reference to the links for this index
    #[bw(map = RWOption::from)]
    #[br(map = RWOption::into)]
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

#[binrw]
#[brw(big)]
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
