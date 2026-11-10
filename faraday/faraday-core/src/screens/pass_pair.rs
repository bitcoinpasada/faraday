//! A passphrase that is being made is entered twice (`docs/DESIGN.md`
//! §4.10): the two fields one under the other, a live line under the
//! second while they differ, and the predicates the step's primary action
//! waits on. The fields themselves are the caller's, since a screen may
//! mask with dots or with an eye and carries its own Practice chip; this
//! draws what is shared.

use crate::strings::EN;
use crate::ui::Ui;
use crate::ui::pal::*;

/// The two passphrases are typed, alike and not empty: the step's primary
/// action is on (`Style::Primary.when(..)`).
pub(crate) fn ready(first: &str, second: &str) -> bool {
    !first.is_empty() && first == second
}

/// The two passphrases differ: the line shows and the action waits.
/// Where a passphrase is optional, an empty pair is not a difference.
pub(crate) fn differ(first: &str, second: &str) -> bool {
    first != second
}

/// The two fields from (x, y), `w` wide, then the line "The two
/// passphrases differ" while they do and the second is begun (an empty
/// second field is still to be typed, not a mistake; the action waits
/// either way). `field(ui, n, y)` draws field `n`
/// (0 the passphrase, 1 the one again) at `y` and returns its height.
/// Returns the height used.
pub(crate) fn pass_pair(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (first, second): (&str, &str),
    mut field: impl FnMut(&mut Ui, u8, f32) -> f32,
) -> f32 {
    let mut cy = y;
    cy += field(ui, 0, cy);
    cy += field(ui, 1, cy);
    if !second.is_empty() && differ(first, second) {
        cy += ui
            .field_supporting((x, cy, w), Role::Error, EN.pass_pair_differ, false)
            .max(22.0);
    }
    cy - y
}
