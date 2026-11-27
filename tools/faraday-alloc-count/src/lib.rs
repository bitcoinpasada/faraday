//! A tool or test-support crate: the counting allocator (09 §8).

// Spec: 09 §8 names the counting allocator without a shape. It is a global allocator, which is
// `unsafe impl GlobalAlloc`; U2 allows the unsafe where a later port needs it, which a skeleton
// does not, so the `GlobalAlloc` impl comes with the bodies.
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
