//! A passphrase that is being made is entered twice: the predicates the step's primary action
//! waits on (`main`'s `pass_field::pair`). The line under the second field is the owner's.

use faraday_text::secret::SecretText;

/// The two passphrases are typed, alike and not empty: the step's primary action is on.
pub fn ready(first: &SecretText, second: &SecretText) -> bool {
    !first.is_empty() && first.ct_eq(second)
}

/// The two passphrases differ: the line shows and the action waits. Where a passphrase is
/// optional, an empty pair is not a difference.
pub fn differ(first: &SecretText, second: &SecretText) -> bool {
    !first.ct_eq(second)
}
