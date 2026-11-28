//! A passphrase that is being made is entered twice: the predicates the step's primary action
//! waits on (`main`'s `pass_field::pair`). The line under the second field is the owner's.

use subtle::ConstantTimeEq;

/// The two passphrases are typed, alike and not empty: the step's primary action is on.
pub fn ready(first: &str, second: &str) -> bool {
    !first.is_empty() && same(first, second)
}

/// The two passphrases are the same text: the bytes are compared in constant time, the lengths
/// are not hidden.
fn same(first: &str, second: &str) -> bool {
    first.as_bytes().ct_eq(second.as_bytes()).into()
}

/// The two passphrases differ: the line shows and the action waits. Where a passphrase is
/// optional, an empty pair is not a difference.
pub fn differ(first: &str, second: &str) -> bool {
    !same(first, second)
}
