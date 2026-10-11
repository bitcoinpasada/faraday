//! The launcher, Keys, Wallets and the explicit single-sig wallet
//! (`docs/PLANNING.md` §16.104): what Home is whatever is loaded, what
//! each list shows, what Add a key and Add a wallet offer, and what a
//! wallet added by hand carries.

mod common;

use common::{
    ABANDON, DESKTOP, Element, Harness, PANEL, PIN, SECURE_PHONE, TILE_KEYS, TILE_LEARN, TILE_SCAN,
    TILE_SETTINGS, TILE_TOOLS, TILE_WALLETS,
};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::{Network, ScriptType};
use osk_shell_api::Key;
use osk_ui::widgets::Icon;

/// A regtest device with one 12-word key loaded.
fn loaded(display: osk_shell_api::DisplayInfo) -> Harness {
    let mut h = Harness::new(display);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h
}

fn tile(i: usize) -> ids::Id {
    ids::at(ids::HOME_TILE_BASE, i)
}

/// Whether the screen states `text` anywhere.
fn says(h: &Harness, text: &str) -> bool {
    h.app.texts().iter().any(|t| t == text)
}

/// Home is the six tiles and nothing else, and it is the same six
/// before anything is loaded and after a key and a wallet are.
#[test]
fn home_is_six_tiles_whatever_is_loaded() {
    let s = &strings::EN;
    let labels = [
        s.home_wallets,
        s.home_keys,
        s.home_scan,
        s.home_tools,
        s.home_learn,
        s.home_settings,
    ];
    let mut h = Harness::new(PANEL);
    for stage in 0..2 {
        h.go_home();
        let texts = h.app.texts();
        for (i, label) in labels.iter().enumerate() {
            assert!(
                h.app.rect_of(tile(i)).is_some(),
                "tile {i} at stage {stage}"
            );
            assert!(
                texts.iter().any(|t| t == label),
                "{label} at stage {stage}: {texts:?}"
            );
        }
        assert!(
            h.app.rect_of(tile(6)).is_none(),
            "six tiles and no more at stage {stage}"
        );
        assert!(
            !texts.iter().any(|t| t.starts_with("73c5da0a")),
            "nothing about what is loaded at stage {stage}: {texts:?}"
        );
        if stage == 0 {
            h.start_load(&ABANDON);
            h.finish_load(None);
            h.add_single_sig(0, 2);
        }
    }
}

/// Each tile opens its own screen, and Scan opens the scanner.
#[test]
fn each_tile_opens_its_screen() {
    let mut h = loaded(PANEL);
    for (i, want) in [
        (TILE_WALLETS, ScreenKind::Wallets),
        (TILE_KEYS, ScreenKind::Keys),
        (TILE_SCAN, ScreenKind::Scan),
        (TILE_TOOLS, ScreenKind::Tools),
        (TILE_LEARN, ScreenKind::Learn),
        (TILE_SETTINGS, ScreenKind::Settings),
    ] {
        h.go_home();
        h.tap(tile(i));
        assert_eq!(h.app.screen(), want, "tile {i}");
    }
}

/// On `wide` the same six are the sidebar, and the pane is the grid.
#[test]
fn the_wide_home_is_the_same_six() {
    let h = loaded(DESKTOP);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    for i in 0..6 {
        assert!(h.app.rect_of(tile(i)).is_some(), "sidebar row {i}");
    }
    assert!(h.app.rect_of(tile(6)).is_none(), "six and no more");
}

/// Keys is one row per loaded key, by fingerprint, in the order they
/// were loaded, each stating what the key is made of and not where it
/// pays.
#[test]
fn keys_lists_loaded_keys_by_fingerprint_in_load_order() {
    let mut h = loaded(PANEL);
    // A passphrase key over the same words is a peer on the list.
    h.open_passphrase(0);
    h.type_text("passphrase");
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    let prints = h.app.fingerprints();
    assert_eq!(prints.len(), 2);

    h.go_home();
    h.open_keys();
    let texts = h.app.texts();
    for (i, fp) in prints.iter().enumerate() {
        assert!(h.app.rect_of(ids::at(ids::KEYS_ROW_BASE, i)).is_some());
        assert!(
            texts.iter().any(|t| *t == fp.to_string()),
            "key {i} is a row: {texts:?}"
        );
    }
    // Each row says what its key is made of: the words, and the
    // passphrase the second key was opened behind.
    let s = &strings::EN;
    for made in [
        strings::fill1(s.key_made_words, "12"),
        strings::fill1(s.key_made_passphrase, "12"),
    ] {
        assert!(says(&h, &made), "{made} is on Keys: {texts:?}");
    }
    // The network is the device's, not the key's, so no row states it.
    assert!(
        !says(&h, Network::Regtest.name()),
        "no row says where a key pays: {texts:?}"
    );
    // The rows are in load order: the parent first.
    let rows: Vec<&String> = texts
        .iter()
        .filter(|t| prints.iter().any(|fp| fp.to_string() == **t))
        .collect();
    assert_eq!(
        rows,
        vec![&prints[0].to_string(), &prints[1].to_string()],
        "as loaded"
    );
}

/// With nothing loaded Keys is the two rows that start a flow, and its
/// bottom action is there either way.
#[test]
fn an_empty_keys_offers_load_and_create() {
    let mut h = Harness::new(PANEL);
    h.open_keys();
    for row in [ids::KEYS_LOAD, ids::KEYS_CREATE, ids::KEYS_ADD] {
        assert!(h.app.rect_of(row).is_some(), "row {}", row.0);
    }
    h.tap(ids::KEYS_LOAD);
    assert_eq!(h.app.screen(), ScreenKind::Load);
}

/// Add a key is the four ways a key is made, whatever is loaded.
/// Opening one key from another is done on that key's page.
#[test]
fn add_a_key_offers_its_four_ways() {
    let s = &strings::EN;
    let rows = [
        ids::KEYS_LOAD,
        ids::KEYS_CREATE,
        ids::ADD_CREATE_SLIP39,
        ids::ADD_CREATE_CODEX32,
    ];
    let mut h = Harness::new(PANEL);
    h.open_keys();
    h.tap(ids::KEYS_ADD);
    assert_eq!(h.app.screen(), ScreenKind::Add);
    for row in rows {
        assert!(h.app.reveal(row).is_some(), "row {}", row.0);
    }

    let mut h = loaded(PANEL);
    h.open_keys();
    h.tap(ids::KEYS_ADD);
    for row in rows {
        assert!(h.app.reveal(row).is_some(), "row {}", row.0);
    }
    let texts = h.app.texts();
    for gone in [s.detail_open_passphrase, s.detail_open_child] {
        assert!(
            !texts.iter().any(|t| t == gone),
            "{gone} is a key's, not Add a key's: {texts:?}"
        );
    }
}

/// A BIP-85 child is opened from its parent's own page, and lands on
/// Keys as a peer of the key it came from.
#[test]
fn open_bip85_child_starts_on_the_parent_page_and_lands_a_peer() {
    let mut h = loaded(PANEL);
    h.open_child(0);
    assert_eq!(h.app.screen(), ScreenKind::OpenChild);
    h.choose(
        ids::at(ids::OPEN_CHILD_WORDS_BASE, 0),
        ids::OPEN_CHILD_CONTINUE,
    );
    h.type_pin(ids::OPEN_CHILD_KEYBOARD, "0");
    assert_eq!(h.app.screen(), ScreenKind::Opened);

    let prints = h.app.fingerprints();
    assert_eq!(prints.len(), 2, "the parent and the child");
    h.go_home();
    h.open_keys();
    assert!(says(&h, &prints[1].to_string()), "{:?}", h.app.texts());
}

/// A key's page states what it is made of and offers Backup and Forget,
/// and nothing of a wallet.
#[test]
fn the_key_page_states_what_it_is_made_of() {
    let s = &strings::EN;
    let mut h = loaded(PANEL);
    h.open_key(0);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    assert!(says(&h, s.key_made_of), "{:?}", h.app.texts());
    assert!(
        says(&h, &strings::fill1(s.key_made_words, "12")),
        "12 words: {:?}",
        h.app.texts()
    );
    for row in [ids::DETAIL_BACKUP, ids::KEY_FORGET] {
        assert!(h.app.rect_of(row).is_some(), "row {}", row.0);
    }
    for gone in [
        ids::WALLET_ADDRESSES,
        ids::WALLET_EXPORT,
        ids::WALLET_SIGN,
        ids::WALLET_KEYS,
    ] {
        assert!(
            h.app.rect_of(gone).is_none(),
            "row {} is a wallet's",
            gone.0
        );
    }

    // A passphrase key says so on its own page, and nowhere else.
    h.go_home();
    h.open_passphrase(0);
    h.type_text("passphrase");
    h.key(Key::Enter);
    h.go_home();
    h.open_key(1);
    assert!(
        says(&h, &strings::fill1(s.key_made_passphrase, "12")),
        "12 words and a passphrase: {:?}",
        h.app.texts()
    );
}

/// "Keep on this device" is a row of the key's page on Tier B alone.
#[test]
fn keep_is_a_row_of_the_key_page_on_tier_b() {
    let mut h = loaded(PANEL);
    h.open_key(0);
    assert!(h.app.rect_of(ids::KEEP_ROW).is_none(), "not on Tier C");

    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    assert!(h.app.rect_of(ids::KEEP_ROW).is_some(), "on Tier B");
}

/// A loaded key is no wallet. Adding a single-sig wallet by hand lists
/// one row with the key glyph, and SegWit and Taproot of one key are two
/// wallets.
#[test]
fn single_sig_wallets_are_added_by_hand_one_per_script_type() {
    let mut h = loaded(PANEL);
    h.open_wallets();
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "a key alone is no wallet: {:?}",
        h.app.texts()
    );
    for row in [ids::WALLETS_ADD, ids::WALLETS_LOAD] {
        assert!(h.app.rect_of(row).is_some(), "empty-state row {}", row.0);
    }

    h.add_single_sig(0, 2);
    h.open_wallets();
    let glyphs = h.app.row_glyphs();
    assert_eq!(glyphs.len(), 1, "one row: {glyphs:?}");
    assert_eq!(glyphs[0].1, Icon::Wallet, "this device holds its key");
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.starts_with("73c5da0a") && t.contains(strings::EN.script_segwit)),
        "the row names the key and the script: {:?}",
        h.app.texts()
    );

    // The Taproot wallet of the same key is a second wallet.
    h.go_home();
    h.add_single_sig(0, 3);
    h.open_wallets();
    assert_eq!(h.app.wallet_count(), 2, "two wallets of one key");
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 1))
            .is_some()
    );
}

/// The same descriptor added twice is the one row: the review opens the
/// wallet already registered.
#[test]
fn the_same_descriptor_twice_is_one_row() {
    let mut h = loaded(PANEL);
    h.add_single_sig(0, 2);
    h.go_home();
    h.open_add_wallet(ids::BUILD_NEW);
    h.choose(ids::at(ids::BUILD_KIND_BASE, 0), ids::BUILD_KIND_CONTINUE);
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 0));
    h.tap(ids::BUILD_CONTINUE);
    h.choose(
        ids::at(ids::BUILD_SCRIPT_BASE, 2),
        ids::BUILD_SCRIPT_CONTINUE,
    );
    assert_eq!(h.app.screen(), ScreenKind::Build);
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.wallet_count(), 1, "one wallet, not two");
}

/// A single-sig wallet's page carries every row §16.104 rule 2 names,
/// its addresses are the key's at that script type, and Forget takes the
/// wallet away and leaves the key.
#[test]
fn a_single_sig_wallet_page_carries_its_rows() {
    let mut h = loaded(PANEL);
    let expected = h.app.addresses(0, ScriptType::NativeSegwit, false, 3);
    h.add_single_sig(0, 2);
    for row in [
        ids::WALLET_SIGN,
        ids::WALLET_SIGN_MESSAGE,
        ids::WALLET_ADDRESSES,
        ids::WALLET_CHECK,
        ids::WALLET_EXPORT,
        ids::WALLET_KEYS,
        ids::WALLET_FORGET,
    ] {
        assert!(h.app.reveal(row).is_some(), "row {}", row.0);
    }

    h.tap(ids::WALLET_ADDRESSES);
    let texts = h.app.texts();
    for address in &expected {
        let head = &address[..8];
        assert!(
            texts.iter().any(|t| t.replace(' ', "").starts_with(head)),
            "{address} is in the list: {texts:?}"
        );
    }
    h.tap(ids::BACK);

    h.tap(ids::WALLET_EXPORT);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.contains("73c5da0a")),
        "the descriptor names the master: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.starts_with("m/84h/0h/0h")),
        "and the account path it was taken at: {texts:?}"
    );
    h.tap(ids::BACK);

    h.tap(ids::WALLET_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    assert_eq!(h.app.wallet_count(), 0, "the wallet is gone");
    h.go_home();
    h.open_keys();
    assert!(
        h.app.rect_of(ids::at(ids::KEYS_ROW_BASE, 0)).is_some(),
        "the key stays: {:?}",
        h.app.texts()
    );
}

/// A wallet's Keys row is the Keys list filtered to its members, with
/// the glyphs: a member not loaded says so, tapping it opens Add a key,
/// and loading that key comes back to the review with the key glyph.
#[test]
fn a_not_loaded_member_opens_add_a_key_and_comes_back() {
    const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(osk_shell_api::Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(osk_shell_api::Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: POLICY.as_bytes().to_vec(),
    });
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);

    let glyphs = h.app.row_glyphs();
    assert_eq!(glyphs.len(), 3, "one row per key: {glyphs:?}");
    assert!(
        glyphs.iter().all(|(_, icon)| *icon == Icon::Eye),
        "no key of this device is loaded: {glyphs:?}"
    );
    assert!(says(&h, strings::EN.key_not_loaded), "{:?}", h.app.texts());

    // The row this device's own key belongs to opens Add a key, and the
    // key loaded there comes back to the review.
    let row = glyphs
        .iter()
        .position(|(label, _)| label == "73c5da0a")
        .expect("the test key is a member");
    h.tap(ids::at(ids::WALLET_KEY_ROW_BASE, row));
    assert_eq!(h.app.screen(), ScreenKind::Add);
    h.tap(ids::KEYS_LOAD);
    h.load_choices(0);
    for w in ABANDON {
        h.type_text(w);
        h.key(Key::Enter);
    }
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(
        h.app.screen(),
        ScreenKind::WalletKeys,
        "back to the review it was left from"
    );
    let glyphs = h.app.row_glyphs();
    assert_eq!(glyphs[row].1, Icon::Wallet, "now loaded: {glyphs:?}");
}

/// Tier B: a single-sig wallet added by hand comes back after a restart
/// with the key it is over.
#[test]
fn a_single_sig_wallet_comes_back_after_a_restart() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept());
    h.go_home();
    h.add_single_sig(0, 2);

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    assert_eq!(h.app.fingerprints().len(), 1, "the key came back");
    h.open_wallets();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_some(),
        "and the wallet with it: {:?}",
        h.app.texts()
    );
    assert_eq!(
        h.app.single_sig_wallet(0, ScriptType::NativeSegwit),
        Some(0),
        "the same wallet over the same key"
    );
}

/// Signing needs keys and never a wallet: a PSBT read at Scan is signed
/// with the loaded key alone (§16.104 rule 5).
#[test]
fn a_psbt_signs_with_a_loaded_key_and_no_wallet() {
    use opensigner_core::sign::{Stage, Step};
    const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");
    let mut h = loaded(PANEL);
    h.set_network(3);
    assert_eq!(h.app.wallet_count(), 0, "no wallet is registered");

    h.go_home();
    h.tap(ids::HOME_SCAN);
    h.send(osk_shell_api::Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(osk_shell_api::Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
    for _ in 0..8 {
        if h.app.sign_stage() == Some(Stage::Wizard(Step::Confirm)) {
            break;
        }
        h.tap(ids::SIGN_CONTINUE);
    }
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Confirm)));
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Result)));
}
