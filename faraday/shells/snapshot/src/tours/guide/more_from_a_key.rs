//! The pictures of `docs/guide/walk/more-from-a-key.md`.

use super::{DICE_WORDS, key_session};
use crate::tour::{Tour, type_text};
use faraday_core::fresh::OwnSource;
use faraday_core::fresh::page::Act;
use faraday_core::{Action, Screen};

/// Costs what the tool screens cost: an address, a child seed and a node
/// key derived. A fresh stick session with the guide's key typed in:
/// Tools, then each tool that works from a key opened on it, and Add
/// randomness with ten dice rolls; Explore's Addresses on the key's first
/// account; Add a key's Seed XOR parts with two parts typed.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    t.press(Action::Nav(Screen::Catalog));
    t.shot("more-from-a-key-tools")?;
    on_the_key(t, "more-from-a-key")?;
    t.press(Action::Nav(Screen::Settings));
    t.press(Action::Own(Act::Open));
    t.press(Action::Own(Act::Source(OwnSource::Dice)));
    for f in [3, 1, 4, 1, 5, 2, 6, 5, 3, 5] {
        t.press(Action::Own(Act::Roll(f)));
    }
    t.shot("more-from-a-key-randomness")?;
    addresses(t)?;
    seed_xor(t)
}

/// Explore on the guide's key, at its native SegWit account: Addresses.
fn addresses(t: &mut Tour) -> Result<(), String> {
    use faraday_core::explore::{Page, XAction};
    t.press(Action::Explore);
    t.press(Action::XKey(super::DICE_FP));
    t.press(Action::X(XAction::Page(Page::Addresses)));
    t.shot("more-from-a-key-addresses")
}

/// Costs a mnemonic's checksum. Add a key on Seed XOR parts: two parts
/// of the guide's key typed and added, the first BIP-39's
/// `legal winner …` vector, the second what it leaves of the key.
fn seed_xor(t: &mut Tour) -> Result<(), String> {
    use faraday_bip::bip39::{Language, Mnemonic};
    use faraday_core::forms::Form;
    let c = faraday_core::testkit::compute();
    let first = "legal winner thank year wave sausage worth useful legal winner thank yellow";
    let key = Mnemonic::parse(&c, Language::English, DICE_WORDS).map_err(|e| format!("{e:?}"))?;
    let part = Mnemonic::parse(&c, Language::English, first).map_err(|e| format!("{e:?}"))?;
    let rest: Vec<u8> = key
        .entropy()
        .expose()
        .as_bytes()
        .iter()
        .zip(part.entropy().expose().as_bytes())
        .map(|(a, b)| a ^ b)
        .collect();
    let second = Mnemonic::from_entropy(Language::English, &rest).map_err(|e| format!("{e:?}"))?;
    let second: Vec<&str> = second
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let at = Form::ALL
        .iter()
        .position(|f| *f == Form::Xor)
        .ok_or("no Seed XOR form")?;
    t.press(Action::Entry(None));
    t.press(Action::EntryForm(at as u8));
    for words in [first.to_string(), second.join(" ")] {
        type_text(t, &words);
        t.press(Action::EntryPart);
    }
    t.shot("more-from-a-key-seed-xor")
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
    vanity(t, fp)?;
    t.shot(&name("vanity"))
}

/// Costs an address derived a try. Vanity address on key `fp`: Account
/// number, Native SegWit, `fa` typed after `bc1q`, and the search run
/// until it finds an address.
fn vanity(t: &mut Tour, fp: [u8; 4]) -> Result<(), String> {
    use faraday_core::vanity::{VanityAction as Va, vstep};
    use faraday_shell_api::{App, Event};
    t.press(Action::Vanity(Va::Open));
    t.press(Action::Vanity(Va::Key(fp)));
    t.press(Action::Vanity(Va::Next(vstep::DIAL)));
    t.press(Action::Vanity(Va::Next(vstep::SCRIPT)));
    type_text(t, "fa");
    t.press(Action::Vanity(Va::Next(vstep::PREFIX)));
    t.press(Action::Vanity(Va::Start));
    // Ticks a tenth of a second apart, as a shell sends them; the tour's
    // clock is set back after, so the pictures after these keep the
    // times their pages give.
    let start = t.now;
    let mut found = false;
    for _ in 0..4000 {
        t.now += 100;
        t.app.event(Event::Tick { now_ms: t.now });
        t.pump();
        if t.app.vanity().is_some_and(|v| v.grind.find.is_some()) {
            found = true;
            break;
        }
    }
    t.now = start;
    if found {
        Ok(())
    } else {
        Err("the vanity search found nothing".into())
    }
}
