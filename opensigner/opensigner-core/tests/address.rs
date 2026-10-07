//! The address keyboard (`docs/DESIGN.md` §4.3): the bech32 alphabet
//! where QWERTY puts it, base58 behind the shift, and a key that cannot
//! continue an address on this network dimmed.
//!
//! Every character is tapped on the screen rather than sent as a
//! physical key, so what these tests prove is that the key was there and
//! was live.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::verify::AddressResult;
use opensigner_core::{ScreenKind, ids};
use osk_bip::keys::{Network, ScriptType};
use osk_ui::widgets::keyboard::KeyInput;

/// A loaded key, and the address field of Verify open over its wallet.
fn typing(network: usize) -> Harness {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(network);
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    h.tap(ids::SCAN_TYPE);
    assert_eq!(h.app.screen(), ScreenKind::Verify);
    h
}

/// Whether the key for `c` is on the keyboard and live.
fn live(h: &Harness, c: char) -> bool {
    h.app
        .key_rect(ids::VERIFY_KEYBOARD, KeyInput::Char(c))
        .is_some()
}

/// Taps `address` out one character at a time, changing the case of the
/// base58 layer where a character asks for it.
fn tap_address(h: &mut Harness, address: &str) {
    for c in address.chars() {
        if !live(h, c) {
            h.pad(ids::VERIFY_KEYBOARD, KeyInput::Symbols);
        }
        h.pad(ids::VERIFY_KEYBOARD, KeyInput::Char(c));
    }
}

/// A `bc1…` address is typed on the layer the field opens with, and ✓
/// answers for it.
#[test]
fn a_bech32_address_is_typed_on_the_layer_the_field_opens_with() {
    let mut h = typing(0);
    assert_eq!(h.app.network(), Network::Mainnet);
    let address = h.app.addresses(0, ScriptType::NativeSegwit, false, 1)[0].clone();
    assert!(address.starts_with("bc1q"), "{address}");
    tap_address(&mut h, &address);
    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Done);
    // The key's SegWit wallet is registered, so the address is that
    // wallet's rather than a key's alone.
    assert_eq!(
        h.app.verify_result(),
        Some(&AddressResult::Wallet {
            wallet: 0,
            change: false,
            index: 0,
        })
    );
}

/// A legacy address on regtest is typed on the base58 layer, which the
/// first character switches to, and ✓ answers for it.
#[test]
fn a_base58_address_is_typed_on_the_shift_layer() {
    let mut h = typing(3);
    assert_eq!(h.app.network(), Network::Regtest);
    let address = h.app.addresses(0, ScriptType::Legacy, false, 1)[0].clone();
    let first = address.chars().next().expect("an address");
    assert!(matches!(first, 'm' | 'n' | '2'), "{address}");
    tap_address(&mut h, &address);
    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Done);
    assert_eq!(
        h.app.verify_result(),
        Some(&AddressResult::Yours {
            fingerprint: h.app.fingerprints()[0],
            script: ScriptType::Legacy,
            change: false,
            index: 0,
        })
    );
}

/// `1` is not a bech32 character, and the keyboard still has it: it is
/// the separator between the human-readable part and the rest.
#[test]
fn the_separator_has_a_key_where_it_belongs() {
    let mut h = typing(0);
    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Char('b'));
    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Char('c'));
    assert!(live(&h, '1'), "the separator follows `bc`");
    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Char('1'));
    assert_eq!(h.app.verify_input(), "bc1");
}

/// A key that cannot continue the address is dim, as the BIP-39
/// keyboard dims a letter that cannot continue a word.
#[test]
fn keys_that_cannot_follow_are_dim() {
    let mut h = typing(0);
    // Nothing typed: `b` begins `bc1…` and `1` and `3` begin a base58
    // address; `q` begins neither.
    assert!(live(&h, 'b') && live(&h, '1') && live(&h, '3'));
    assert!(!live(&h, 'q'), "no address on mainnet begins with q");

    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Char('b'));
    assert!(live(&h, 'c') && !live(&h, 'b'), "`bc` and nothing else");

    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Char('c'));
    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Char('1'));
    h.pad(ids::VERIFY_KEYBOARD, KeyInput::Char('q'));
    // Past the separator only the bech32 alphabet continues, and `b`
    // and `1` are not in it.
    assert!(live(&h, 'q') && live(&h, '0'));
    assert!(!live(&h, 'b') && !live(&h, '1'));
}
