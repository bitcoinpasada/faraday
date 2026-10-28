//! The pictures of `docs/guide/walk/more-from-a-key.md`.

use super::key_session;
use crate::tour::Tour;
use faraday_core::fresh::OwnSource;
use faraday_core::fresh::page::Act;
use faraday_core::{Action, Screen};

/// Costs what the tool screens cost: an address, a child seed and a node
/// key derived. A fresh stick session with the guide's key typed in:
/// Tools, then each tool that works from a key opened on it, and Add
/// randomness with ten dice rolls.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    t.press(Action::Nav(Screen::Catalog));
    t.shot("more-from-a-key-tools")?;
    on_the_key(t, "more-from-a-key")?;
    t.press(Action::Nav(Screen::Randomness));
    t.press(Action::Own(Act::Source(OwnSource::Dice)));
    for f in [3, 1, 4, 1, 5, 2, 6, 5, 3, 5] {
        t.press(Action::Own(Act::Roll(f)));
    }
    t.shot("more-from-a-key-randomness")
}

/// Costs what the tool screens cost. Explore, BIP-85, Silent payments,
/// Lightning and Vanity, each opened on the first key loaded, the shots
/// named `<prefix>-explore` and so on, or by the screen alone with no
/// prefix (the screen reference's).
pub(crate) fn on_the_key(t: &mut Tour, prefix: &str) -> Result<(), String> {
    let fp = t
        .app
        .session
        .keys
        .first()
        .ok_or("no key loaded")?
        .fingerprint
        .0;
    let name = |what: &str| {
        if prefix.is_empty() {
            what.to_string()
        } else {
            format!("{prefix}-{what}")
        }
    };
    t.press(Action::Explore);
    t.press(Action::XKey(fp));
    t.shot(&name("explore"))?;
    t.press(Action::Bip85);
    t.press(Action::PKey(fp));
    t.press(Action::PNext);
    t.press(Action::PNext);
    t.shot(&name(if prefix.is_empty() {
        "child-seeds"
    } else {
        "bip85"
    }))?;
    t.press(Action::Silent);
    t.press(Action::SKey(fp));
    t.shot(&name(if prefix.is_empty() {
        "silent-payments"
    } else {
        "silent"
    }))?;
    t.press(Action::Lightning);
    t.press(Action::LKey(fp));
    t.shot(&name("lightning"))?;
    t.press(Action::Nav(Screen::Vanity));
    t.shot(&name("vanity"))
}
