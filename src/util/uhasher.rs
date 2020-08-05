/// For hashing things where the order doesn't matter (e.g.
/// when using a Set or Map as a key)
/// Roughly based on
/// https://stackoverflow.com/questions/20832279/python-frozenset-hashing-algorithm-implementation
pub struct UnorderedHasher {
    hash: u64,
}

impl UnorderedHasher {
    pub fn new(size: u64) -> UnorderedHasher {
        UnorderedHasher {
            hash: 1927868237u64.wrapping_mul(size + 1),
        }
    }

    pub fn add(&mut self, h: u64) {
        self.hash ^= (h ^ (h << 16) ^ 89869747).wrapping_mul(3644798167);
    }

    pub fn finish(self) -> u64 {
        let hash = self.hash.wrapping_mul(69069).wrapping_add(907133923);
        hash
    }
}
