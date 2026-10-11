//! The keyboard alone, from Home to a signed transaction
//! (`docs/DESIGN.md` §4.15). Nothing here touches the panel: Tab moves
//! the focus, Enter acts on what it is on, the words are typed, and the
//! hold is held with Enter until it completes.

mod common;

use common::{ABANDON, DESKTOP, Harness, PIN};
use opensigner_core::ScreenKind;
use opensigner_core::ids;
use opensigner_core::load::Step;
use opensigner_core::scan::ScanStage;
use opensigner_core::sign::{Stage, Step as SignStep};
use osk_bip::keys::Network;
use osk_shell_api::{Event, FileKind, Key};

const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");

/// A person whose touchpad the kernel does not drive loads a key and
/// signs a transaction with the keyboard and nothing else.
#[test]
fn a_key_is_loaded_and_a_transaction_signed_with_the_keyboard_alone() {
    let mut h = Harness::new(DESKTOP);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.focused(), None, "nothing has focus when Home opens");

    // Home is the launcher; Keys' empty state carries the row that
    // starts the flow.
    h.enter_on(ids::at(ids::HOME_TILE_BASE, 1));
    assert_eq!(h.app.screen(), ScreenKind::Keys);
    h.enter_on(ids::KEYS_LOAD);
    assert_eq!(h.app.screen(), ScreenKind::Load);
    assert_eq!(h.app.load_step(), Some(Step::Source));

    // Three choices: the option, then the Continue that confirms it.
    h.enter_on(ids::LOAD_SOURCE_TYPE);
    h.enter_on(ids::LOAD_SOURCE_CONTINUE);
    h.enter_on(ids::at(ids::LOAD_COUNT_BASE, 0));
    h.enter_on(ids::LOAD_COUNT_CONTINUE);
    h.enter_on(ids::at(ids::LOAD_LANG_BASE, 0));
    h.enter_on(ids::LOAD_LANG_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::Words));

    // Twelve words typed on the BIP-39 keyboard. Nothing is focused
    // while a word is being typed, so Enter is the keyboard's ✓.
    for w in ABANDON {
        assert_eq!(h.app.focused(), None);
        h.type_text(w);
        h.key(Key::Enter);
    }
    assert_eq!(h.app.load_step(), Some(Step::Checksum));

    h.enter_on(ids::LOAD_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::PassphraseOffer));
    h.enter_on(ids::LOAD_SKIP);
    h.enter_on(ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::Confirm));
    h.enter_on(ids::LOAD_HOLD);
    // The session's PIN, typed on the pad with the keyboard's own
    // digits (§16.99), twice.
    if h.app.load_step() == Some(Step::Pin) {
        for _ in 0..2 {
            h.type_text(PIN);
            h.key(Key::Enter);
        }
    }
    if h.app.screen() == ScreenKind::Keep {
        h.key(Key::Escape);
    }
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.fingerprints().len(), 1, "the key is loaded");

    // The transaction is a regtest one, so the chain is set the same way.
    h.enter_on(ids::at(ids::HOME_TILE_BASE, 5));
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    h.enter_on(ids::SETTINGS_NETWORK_ROW);
    h.enter_on(ids::at(ids::SETTINGS_NET_BASE, 3));
    h.key(Key::Escape);
    h.key(Key::Escape);
    assert_eq!(h.app.network(), Network::Regtest);
    assert_eq!(h.app.screen(), ScreenKind::Home);

    // Scan, then the file the scanner reads: a PSBT names its keys and
    // signs with the loaded one, wallet or no wallet.
    h.enter_on(ids::at(ids::HOME_TILE_BASE, 2));
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.enter_on(ids::SCAN_FILE);
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Waiting));
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Summary)));

    // The review, one screen at a time, on the same Continue.
    for _ in 0..8 {
        if h.app.sign_stage() == Some(Stage::Wizard(SignStep::Confirm)) {
            break;
        }
        h.enter_on(ids::SIGN_CONTINUE);
    }
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Confirm)));

    // The hold fills under Enter and completes when the key has been
    // down for the whole duration.
    h.focus_to(ids::SIGN_HOLD);
    h.key(Key::Enter);
    h.tick(10);
    assert!(h.app.sign_stage() == Some(Stage::Wizard(SignStep::Confirm)));
    h.tick(osk_ui::widgets::tokens::HOLD_MS);
    h.key_up(Key::Enter);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Result)));
}
