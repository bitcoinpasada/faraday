//! A tool or test-support crate: the leak allocator (09 §8; 07 §6). It lists the live blocks of
//! the armed threads and reads freed ones only inside `drop_checked` (07 §6.1).

/// A global allocator that finds a form of a secret in a live block, and in a block that
/// `drop_checked` frees; it lists live blocks and reads freed ones only there (07 §6.1).
pub struct Leak {
    _private: (),
}

impl Leak {
    /// Watches for `form`, a secret's bytes in any form the test names.
    pub fn watch(_form: &[u8]) {
        todo!()
    }

    /// Whether a watched form is in a live block of the armed threads.
    #[must_use]
    pub fn live() -> bool {
        todo!()
    }

    /// Whether a watched form is in a block that dropping `value` freed.
    #[must_use]
    pub fn drop_checked<T>(_value: T) -> bool {
        todo!()
    }
}
