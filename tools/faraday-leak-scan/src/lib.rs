//! A tool or test-support crate: the leak allocator (09 §8; 07 §6).

// Spec: 09 §8 names the leak allocator without a shape. It is a global allocator, which is
// `unsafe impl GlobalAlloc`; U2 allows the unsafe where a later port needs it, which a skeleton
// does not, so the `GlobalAlloc` impl comes with the bodies.
/// A global allocator that finds a form of a secret in a block it frees.
pub struct Leak {
    _private: (),
}

impl Leak {
    /// Watches for `form`, a secret's bytes in any form the test names.
    pub fn watch(_form: &[u8]) {
        todo!()
    }

    /// Whether a freed block held a watched form.
    #[must_use]
    pub fn found() -> bool {
        todo!()
    }
}
