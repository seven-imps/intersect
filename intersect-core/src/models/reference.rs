use binrw::binrw;
use std::marker::PhantomData;

use crate::{Domain, Hash, Shard};

// reference
// (shard and hash)

#[binrw]
#[derive(PartialEq, Debug, Clone, Eq, Copy)]
pub struct Reference<D: Domain> {
    // convert the domain to a magic byte
    // and include it in the serialisation
    // so we can make sure all references are strongly typechecked
    #[bw(map = |_| D::MAGIC)]
    #[br(try_map = |d: u8| (d == D::MAGIC).then_some(PhantomData).ok_or("invalid domain magic"))]
    _domain: PhantomData<D>,
    shard: Shard,
    hash: Hash,
}

impl<D: Domain> Reference<D> {
    pub(crate) fn new(shard: Shard, hash: Hash) -> Self {
        Self {
            shard,
            hash,
            _domain: PhantomData,
        }
    }

    pub fn shard(&self) -> &Shard {
        &self.shard
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }
}
