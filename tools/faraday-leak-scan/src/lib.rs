//! A tool or test-support crate: the leak allocator (09 §8; 07 §6).

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
