use std::marker::PhantomData;

use crate::{Domain, Hash, Shard};

// reference
// (shard and hash)

#[derive(PartialEq, Debug, Clone, Eq, Copy)]
pub struct Reference<D: Domain> {
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
