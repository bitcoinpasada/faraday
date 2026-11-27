//! A tool or test-support crate: the counting allocator (09 §8).

/// A global allocator that counts what it hands out.
pub struct Counting {
    _private: (),
}

impl Counting {
    /// The allocations made since the last reset.
    #[must_use]
    pub fn allocations() -> u64 {
        todo!()
    }

    pub fn reset() {
        todo!()
    }
}
