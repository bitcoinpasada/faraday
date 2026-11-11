//! The lists a [`crate::Action::BAnswer`] ticks, as [`Question`] names them,
//! and the question a list number stands for.

use super::Question;

/// Where the seeds go.
pub const SEEDS: u8 = 0;
/// How many places: the row is the count.
pub const PLACES: u8 = 1;
/// The whole sheet (row 0) or a share each (row 1).
pub(crate) const SPLIT: u8 = 2;
/// Where the wallet description goes.
pub const WALLET: u8 = 4;
/// The software.
pub const SOFTWARE: u8 = 5;
/// The form.
pub const FORM: u8 = 6;
/// Seed i's passphrase: `PASS + i`, a row per place, then the vault.
pub const PASS: u8 = 16;
/// A stick with vault v: `STICKS + v`, a row per place.
pub const STICKS: u8 = 128;
/// The seeds vault v holds: `VAULTS + v`, a row per seed of the
/// wallet.
pub const VAULTS: u8 = 192;

/// The question a list number stands for.
pub fn question(n: u8) -> Option<Question> {
    use Question as Q;
    Some(match n {
        SEEDS => Q::Seeds,
        PLACES => Q::Places,
        SPLIT => Q::Split,
        WALLET => Q::Wallet,
        SOFTWARE => Q::Software,
        FORM => Q::Form,
        n if n >= VAULTS => Q::Vault(usize::from(n - VAULTS)),
        n if n >= STICKS => Q::Sticks(usize::from(n - STICKS)),
        n if n >= PASS => Q::Passphrase(usize::from(n - PASS)),
        _ => return None,
    })
}
