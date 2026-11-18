//! The multisig-check tour (`docs/FLOWS.md`, Check a multisig setup):
//! the screen with nothing read; a setup scanned and its Result; a
//! descriptor whose checksum does not hold, refused on the scanner; and
//! Check a seed against it: the Seed and Passphrase steps under the rows,
//! a seed that is a cosigner's key, and one that is no key of the wallet;
//! and the added wallet's own sheet with a key loaded here: its key block
//! with a key Yours, then one ticked, then every key checked.

use crate::tour::Tour;
use faraday_core::glance::Press;
use faraday_core::glance::sheet::{ChartAction as C, Target};
use faraday_core::multisig_check::MsAction as M;
use faraday_core::seed_check::SeedAction as S;
use faraday_core::testkit;
use faraday_core::wallet::KeyBlockOf;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event};

/// The words typed on the Seed step and taken, no passphrase.
fn seed(t: &mut Tour, words: &str) {
    t.press(Action::SeedCheck(S::Type));
    testkit::type_words(&mut t.app, words);
    t.press(Action::SeedCheck(S::Words));
}

/// A descriptor of the test wallet, scanned.
fn scan(t: &mut Tour, text: &str) {
    t.press(Action::MultisigCheck(M::Scan));
    t.app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    t.tick();
}

const ABOUT: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

pub(crate) fn multisig_check_tour(t: &mut Tour) -> Result<(), String> {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .ok_or("no savings wallet")?;
    t.press(Action::Nav(Screen::Tools));
    t.press(Action::MultisigCheck(M::Open));
    t.shot("multisig-check-empty")?;
    scan(t, &kit.descriptor);
    t.shot("multisig-check-result")?;
    crate::tour::scroll_to(t, Action::KeyTick(KeyBlockOf::Check, 2));
    t.shot("multisig-check-keys")?;
    t.press(Action::MultisigCheck(M::CheckSeed));
    t.shot("multisig-check-seed")?;
    seed(t, &testkit::test_words("zebra"));
    t.shot("multisig-check-passphrase")?;
    t.press(Action::SeedCheck(S::NoPass));
    t.shot("multisig-check-seed-is-key")?;
    t.press(Action::MultisigCheck(M::CheckSeed));
    seed(t, ABOUT);
    t.press(Action::SeedCheck(S::NoPass));
    t.shot("multisig-check-seed-not-a-key")?;
    t.press(Action::MultisigCheck(M::Another));
    let (body, sum) = kit.descriptor.split_once('#').ok_or("no checksum")?;
    let last = if sum.ends_with('q') { 'p' } else { 'q' };
    let bad = format!("{body}#{}{last}", &sum[..sum.len() - 1]);
    scan(t, &bad);
    t.shot("multisig-check-refused")?;
    t.press(Action::MultisigCheck(M::Another));
    scan(t, &kit.descriptor);
    t.press(Action::MultisigCheck(M::Add));
    t.press(Action::WalletHold);
    t.shot("multisig-check-added")?;
    // The wallet's own sheet with a key loaded here: its key block, a
    // key Yours and two not checked; a press on the second; the third.
    let w = t.app.session.wallets.len() - 1;
    let of = KeyBlockOf::Wallet(u16::try_from(w).map_err(|e| e.to_string())?);
    t.press(Action::Entry(None));
    t.type_key(0);
    t.press(Action::OpenWallet(w));
    // A press on a tick closes a chart's sheet on a narrow window, so the
    // sheet is opened again, as a person would, before each frame.
    let sheet = |t: &mut Tour, name: &str| {
        if t.app.sheet.is_none() {
            t.press(Action::Chart(C::Open(Press::Loaded(w), Target::Wallet)));
        }
        crate::tour::scroll_to(t, Action::KeyTick(of, 2));
        crate::tour::scroll(t, 120);
        t.shot(name)
    };
    sheet(t, "multisig-wallet-keys")?;
    t.press(Action::KeyTick(of, 1));
    sheet(t, "multisig-wallet-keys-ticked")?;
    t.press(Action::KeyTick(of, 2));
    sheet(t, "multisig-wallet-keys-all")?;
    Ok(())
}
