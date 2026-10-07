//! BIP-85's other applications from the key page's BIP-85 row
//! (`docs/PLANNING.md` §16.114).
//!
//! The vectors the BIP publishes are for a master key given as an
//! `xprv`, which no screen here loads, so what these tests check is the
//! round trip a person makes: the screens derive, for the key this
//! harness loads, the value `osk_bip::bip85` derives for the same key,
//! the same parameters and the same index. The BIP's own vectors are
//! checked against that crate in `core/osk-bip/tests/bip85.rs`.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network};
use osk_ui::widgets::keyboard::KeyInput;

/// Row `i` of "Which application?", in the order the Choice lists them.
const WORDS: usize = 0;
const WIF: usize = 1;
const XPRV: usize = 2;
const HEX: usize = 3;
const BASE64: usize = 4;
const BASE85: usize = 5;

/// The master key of the words the harness loads, which is what the
/// screens derive from.
fn master() -> MasterKey {
    let mnemonic = Mnemonic::parse(Language::English, &ABANDON.join(" ")).expect("the test words");
    let seed = mnemonic.to_seed(b"").expect("a seed");
    MasterKey::from_seed(&seed, Network::Mainnet)
}

/// One loaded key, on its own page.
fn on_the_key_page() -> Harness {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h
}

/// The key page's BIP-85 row, and application `row` chosen on the
/// Choice it opens.
fn choose(h: &mut Harness, row: usize) {
    h.tap(ids::DETAIL_BIP85);
    h.choose(ids::at(ids::PICK_BASE, row), ids::PICK_CONTINUE);
}

/// The value on the Secret screen, read while a finger is on the panel.
fn revealed_value(h: &mut Harness) -> String {
    let point = h.press(ids::BIP85_REVEAL);
    let texts = h.app.texts();
    h.release(point);
    texts.join(" ")
}

#[test]
fn each_application_derives_what_bip85_derives_for_this_key() {
    let m = master();
    let expected: Vec<(usize, String)> = vec![
        (
            WIF,
            String::from(osk_bip::bip85::child_wif(&m, 0).unwrap().as_str()),
        ),
        (
            XPRV,
            String::from(osk_bip::bip85::child_xprv(&m, 0).unwrap().as_str()),
        ),
        (
            BASE64,
            String::from(
                osk_bip::bip85::child_password_base64(&m, 21, 0)
                    .unwrap()
                    .as_str(),
            ),
        ),
        (
            BASE85,
            String::from(
                osk_bip::bip85::child_password_base85(&m, 12, 0)
                    .unwrap()
                    .as_str(),
            ),
        ),
    ];
    for (row, value) in expected {
        let mut h = on_the_key_page();
        choose(&mut h, row);
        // A password asks its length first, with the default already in
        // the field; the other two go straight to the index.
        if row == BASE64 || row == BASE85 {
            h.type_pin(ids::BIP85_LENGTH_PAD, "");
        }
        assert_eq!(h.app.screen(), ScreenKind::Bip85, "row {row}");
        h.type_pin(ids::BIP85_INDEX_PAD, "");
        let shown = revealed_value(&mut h);
        assert!(
            shown.contains(&value),
            "row {row} shows {shown:?}, not {value:?}"
        );
    }
}

#[test]
fn hex_derives_the_byte_count_that_was_asked_for() {
    let m = master();
    for (i, bytes) in [16usize, 32, 64].into_iter().enumerate() {
        let value = osk_bip::bip85::child_hex(&m, bytes, 0).unwrap();
        let hex: String = value
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let mut h = on_the_key_page();
        choose(&mut h, HEX);
        h.choose(ids::at(ids::PICK_BASE, i), ids::PICK_CONTINUE);
        h.type_pin(ids::BIP85_INDEX_PAD, "");
        let shown = revealed_value(&mut h);
        assert!(shown.contains(&hex), "{bytes} bytes: {shown:?}");
    }
}

#[test]
fn the_index_that_was_typed_is_the_child_that_is_shown() {
    let m = master();
    let mut h = on_the_key_page();
    choose(&mut h, WIF);
    h.pad(ids::BIP85_INDEX_PAD, KeyInput::Backspace);
    h.type_pin(ids::BIP85_INDEX_PAD, "7");
    let shown = revealed_value(&mut h);
    let seven = String::from(osk_bip::bip85::child_wif(&m, 7).unwrap().as_str());
    let zero = String::from(osk_bip::bip85::child_wif(&m, 0).unwrap().as_str());
    assert!(shown.contains(&seven), "index 7: {shown:?}");
    assert!(!shown.contains(&zero), "index 0 is still on screen");
    // The title names the application and the index it derived.
    let title = strings::fill(
        strings::EN.bip85_value_title,
        &[strings::EN.bip85_app_wif, "7"],
    );
    assert!(h.app.texts().contains(&title), "{:?}", h.app.texts());
}

#[test]
fn the_derived_value_is_masked_until_the_panel_is_held() {
    let m = master();
    let value = String::from(osk_bip::bip85::child_wif(&m, 0).unwrap().as_str());
    let mut h = on_the_key_page();
    choose(&mut h, WIF);
    h.type_pin(ids::BIP85_INDEX_PAD, "");
    let masked = h.app.texts().join(" ");
    assert!(!masked.contains(&value), "the value is on screen unheld");
    let point = h.press(ids::BIP85_REVEAL);
    assert!(h.app.texts().join(" ").contains(&value), "held");
    h.release(point);
    assert!(
        !h.app.texts().join(" ").contains(&value),
        "the value stays on screen after the finger lifts"
    );
}

#[test]
fn a_derived_secret_is_not_offered_to_the_clipboard() {
    let mut h = on_the_key_page();
    choose(&mut h, BASE64);
    h.type_pin(ids::BIP85_LENGTH_PAD, "");
    h.type_pin(ids::BIP85_INDEX_PAD, "");
    assert!(
        h.app.rect_of(ids::COPY).is_none(),
        "the Secret screen offers Copy"
    );
}

#[test]
fn a_password_takes_only_the_lengths_its_application_defines() {
    let mut h = on_the_key_page();
    choose(&mut h, BASE85);
    // The default, 12, is in the field and ✓ is live.
    assert!(
        h.app
            .key_rect(ids::BIP85_LENGTH_PAD, KeyInput::Done)
            .is_some(),
        "the default length is refused"
    );
    // Nine is under the ten base85 takes, so ✓ is dead until the field
    // holds a length the application defines.
    h.pad(ids::BIP85_LENGTH_PAD, KeyInput::Backspace);
    h.pad(ids::BIP85_LENGTH_PAD, KeyInput::Backspace);
    h.pad(ids::BIP85_LENGTH_PAD, KeyInput::Char('9'));
    assert!(
        h.app
            .key_rect(ids::BIP85_LENGTH_PAD, KeyInput::Done)
            .is_none(),
        "nine characters is accepted"
    );
    h.pad(ids::BIP85_LENGTH_PAD, KeyInput::Char('0'));
    assert!(
        h.app
            .key_rect(ids::BIP85_LENGTH_PAD, KeyInput::Done)
            .is_none(),
        "ninety characters is accepted"
    );
    h.pad(ids::BIP85_LENGTH_PAD, KeyInput::Backspace);
    h.pad(ids::BIP85_LENGTH_PAD, KeyInput::Backspace);
    h.type_pin(ids::BIP85_LENGTH_PAD, "15");
    h.type_pin(ids::BIP85_INDEX_PAD, "");
    let m = master();
    let value = String::from(
        osk_bip::bip85::child_password_base85(&m, 15, 0)
            .unwrap()
            .as_str(),
    );
    let shown = revealed_value(&mut h);
    assert!(shown.contains(&value), "{shown:?}");
}

#[test]
fn words_opens_the_child_flow() {
    let mut h = on_the_key_page();
    choose(&mut h, WORDS);
    assert_eq!(h.app.screen(), ScreenKind::OpenChild);
    let title = String::from(strings::EN.load_count_title);
    assert!(h.app.texts().contains(&title), "{:?}", h.app.texts());
}
