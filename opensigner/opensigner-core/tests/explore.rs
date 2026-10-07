//! Explore (`docs/DESIGN.md` §5): the two menus and the screens their
//! rows open, over a loaded key and over typed words — the key chooser,
//! the path editor, the passphrase, the Words screen, the five Secret
//! screens.

mod common;

/// One `#[test]` per reference size over a function taking the display.
/// nextest runs every test in a process of its own, so a walk written
/// this way spreads over the cores instead of holding one while the
/// rest wait (`docs/PLANNING.md` §16.119 rule 4). What each walk
/// asserts is what it asserted as one test.
macro_rules! at_every_size {
    ($at:ident => $tiny:ident, $panel:ident, $phone:ident, $desktop:ident) => {
        #[test]
        fn $tiny() {
            $at(SIZES[0]);
        }

        #[test]
        fn $panel() {
            $at(SIZES[1]);
        }

        #[test]
        fn $phone() {
            $at(SIZES[2]);
        }

        #[test]
        fn $desktop() {
            $at(SIZES[3]);
        }
    };
}

use common::{ABANDON, Harness, PANEL};
use opensigner_core::explore::{SecretKind, Source, Step};
use opensigner_core::ids;
use opensigner_core::load::Step as LoadStep;
use opensigner_core::strings;
use opensigner_core::{OpenSigner, ScreenKind};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_shell_api::{BootState, DisplayInfo, Key, SecureHardware};
use osk_ui::widgets::keyboard::KeyInput;

/// The desktop window: one fixed size, the `wide` size class (UX.md §2).
const WIDE: DisplayInfo = DisplayInfo {
    width: 960,
    height: 640,
    dpi: 160,
    inset_bottom: 0,
    inset_top: 0,
    buttons: 0,
    camera_fixed: false,
    secure: SecureHardware::None,
    boot: BootState::Unknown,
    memory_mib: None,
};

const FIRST_RECEIVE: &str = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";

fn abandon_master() -> MasterKey {
    let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
    MasterKey::from_seed(&m.to_seed(b"").unwrap(), Network::Mainnet)
}

fn load_abandon(h: &mut Harness, passphrase: Option<&str>) {
    h.start_load(&ABANDON);
    h.finish_load(passphrase);
}

fn open_explore(h: &mut Harness) {
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    assert_eq!(h.app.screen(), ScreenKind::Tools);
    h.tap(ids::TOOLS_EXPLORER);
    assert_eq!(h.app.screen(), ScreenKind::Explore);
}

/// Opens "Words and bits" from the key menu.
fn open_bits(h: &mut Harness) {
    h.tap(ids::EXPLORE_BITS);
    assert_eq!(h.app.explore_step(), Some(Step::Bits));
}

/// Types the twelve `abandon` words into Explore's own word entry.
fn type_the_words(h: &mut Harness) {
    h.tap(ids::EXPLORE_TYPE);
    h.tap(ids::SCAN_TYPE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Count));
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    for w in ABANDON {
        h.type_text(w);
        h.key(Key::Enter);
    }
    assert_eq!(h.app.load_step(), Some(LoadStep::Checksum));
    h.tap(ids::LOAD_CONTINUE);
}

fn native_segwit_address(app: &OpenSigner) -> String {
    let addresses = app.explore_addresses();
    assert_eq!(addresses.len(), 1, "BIP-84 implies one script type");
    assert_eq!(addresses[0].0, ScriptType::NativeSegwit);
    addresses[0].1.clone()
}

#[test]
fn explore_derives_from_the_loaded_key() {
    let mut h = Harness::new(PANEL);
    load_abandon(&mut h, None);
    open_explore(&mut h);
    assert_eq!(h.app.explore_source(), Some(Source::Loaded(0)));
    assert_eq!(h.app.explore_step(), Some(Step::Keys));
    assert_eq!(h.app.explore_fingerprint().unwrap().to_hex(), *b"73c5da0a");
    assert_eq!(h.app.explore_path().as_deref(), Some("m/84h/0h/0h/0/0"));
    assert_eq!(native_segwit_address(&h.app), FIRST_RECEIVE);

    // The account key's zpub is osk-bip's SLIP-132 form.
    let expected = abandon_master()
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap();
    let (xpub, slip) = h.app.explore_account().unwrap();
    assert_eq!(xpub, expected.xpub_string());
    assert_eq!(slip.as_deref(), Some(expected.slip132_string().as_str()));
    assert!(slip.unwrap().starts_with("zpub"));

    // The words are a screen of their own, and they are masked until
    // the panel is under a finger (§4.10).
    open_bits(&mut h);
    h.tap(ids::EXPLORE_WORDS);
    assert_eq!(h.app.explore_step(), Some(Step::Words));
    assert!(!h.app.explore_words_revealed());
    let point = h.press(ids::CREATE_REVEAL);
    h.tick(100);
    assert!(h.app.explore_words_revealed());
    h.release(point);
    assert!(!h.app.explore_words_revealed(), "hidden on release");
    assert!(!h.app.explore_has_input(), "no typed words");
}

#[test]
fn typed_words_derive_the_same_values_and_leave_nothing_behind() {
    let mut h = Harness::new(PANEL);
    open_explore(&mut h);
    assert_eq!(h.app.explore_source(), Some(Source::None));
    assert!(h.app.explore_fingerprint().is_none());
    type_the_words(&mut h);
    assert_eq!(
        h.app.screen(),
        ScreenKind::Explore,
        "back in Explore, not a wizard"
    );
    assert!(h.app.fingerprints().is_empty(), "no key was loaded");
    assert!(h.app.explore_has_input());
    assert_eq!(h.app.explore_source(), Some(Source::Typed));
    assert_eq!(h.app.explore_fingerprint().unwrap().to_hex(), *b"73c5da0a");
    assert_eq!(native_segwit_address(&h.app), FIRST_RECEIVE);

    // Typed words are a key, so the Addresses row opens the same list a
    // loaded key's does, derived from the words on the screen.
    h.tap(ids::EXPLORE_ADDRESSES);
    assert_eq!(h.app.screen(), ScreenKind::Addresses);
    h.tap(ids::at(ids::ADDR_ROW_BASE, 0));
    assert!(
        h.app.texts().iter().any(|t| t == FIRST_RECEIVE),
        "the first address of the typed words: {:?}",
        h.app.texts()
    );
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Explore);

    let expected = abandon_master()
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap();
    assert_eq!(
        h.app.explore_account().unwrap().1.as_deref(),
        Some(expected.slip132_string().as_str())
    );

    // A passphrase changes the key, as it does in the Load wizard.
    h.tap(ids::EXPLORE_PASSPHRASE);
    assert_eq!(h.app.explore_step(), Some(Step::Passphrase));
    h.type_text("TREZOR");
    h.key(Key::Enter);
    assert_eq!(h.app.explore_step(), Some(Step::Keys), "the tick returns");
    let with_passphrase = common::expected_fingerprint(b"TREZOR", Network::Mainnet);
    assert_eq!(h.app.explore_fingerprint(), Some(with_passphrase));

    // Leaving Explore asks before it drops the typed words, and
    // discarding them zeroizes them.
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Explore, "the confirm, not Home");
    assert_eq!(h.app.explore_step(), Some(Step::Discard));
    h.tap(ids::EXPLORE_KEEP);
    assert_eq!(h.app.explore_step(), Some(Step::Keys));
    assert!(h.app.explore_has_input(), "the words are still here");
    h.tap(ids::BACK);
    h.tap(ids::EXPLORE_DISCARD);
    assert_eq!(h.app.screen(), ScreenKind::Tools, "the explorer is a tool");
    assert!(!h.app.explore_has_input());
    assert!(h.app.fingerprints().is_empty());
}

#[test]
fn cancelling_the_word_entry_returns_to_explore_with_nothing() {
    let mut h = Harness::new(PANEL);
    open_explore(&mut h);
    h.tap(ids::EXPLORE_TYPE);
    h.tap(ids::SCAN_TYPE);
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    h.type_text("zoo");
    h.key(Key::Enter);
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Explore);
    assert!(!h.app.explore_has_input());
    // Back from the count step also cancels, since the source step is
    // not part of the sub-flow.
    h.tap(ids::EXPLORE_TYPE);
    h.tap(ids::SCAN_TYPE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Count));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Explore);
}

/// §4.6: "✓ dead while the text does not parse." The applied path does
/// not move while the editor is open, so what the key menu derives from
/// is always a path that parses.
#[test]
fn the_path_editor_is_dead_until_the_text_parses_and_applies_on_the_tick() {
    let mut h = Harness::new(PANEL);
    load_abandon(&mut h, None);
    open_explore(&mut h);
    h.tap(ids::EXPLORE_PATH);
    assert_eq!(h.app.explore_step(), Some(Step::Path));
    h.type_text("/");
    assert_eq!(
        h.app.explore_path_error(),
        Some(strings::EN.explore_path_unfinished)
    );
    assert!(
        h.app
            .key_rect(ids::EXPLORE_PATH_KEYBOARD, KeyInput::Done)
            .is_none(),
        "the tick is dead while the text does not parse"
    );
    h.key(Key::Enter);
    assert_eq!(h.app.explore_step(), Some(Step::Path), "and does nothing");
    h.type_text("/0");
    assert_eq!(
        h.app.explore_path_error(),
        Some(strings::EN.explore_path_empty)
    );
    for _ in 0..3 {
        h.key(Key::Backspace);
    }
    assert_eq!(h.app.explore_path_error(), None);
    assert_eq!(
        h.app.explore_path().as_deref(),
        Some("m/84h/0h/0h/0/0"),
        "the applied path waits for the tick"
    );
    // An index of 2^31 is out of range.
    for _ in 0..13 {
        h.key(Key::Backspace);
    }
    h.type_text("2147483648");
    assert_eq!(
        h.app.explore_path_error(),
        Some(strings::EN.explore_path_too_large)
    );
    h.key(Key::Backspace);
    assert_eq!(h.app.explore_path_error(), None);
    // `'` types as `h`; ✓ applies and returns to the key menu.
    h.type_text("'");
    h.key(Key::Enter);
    assert_eq!(h.app.explore_step(), Some(Step::Keys));
    assert_eq!(h.app.explore_path().as_deref(), Some("m/214748364h"));

    // The presets type a path; the tick applies it.
    h.tap(ids::EXPLORE_PATH);
    h.tap(ids::at(ids::EXPLORE_PRESET_BASE, 3));
    h.key(Key::Enter);
    assert_eq!(h.app.explore_path().as_deref(), Some("m/86h/0h/0h"));
    let (xpub, slip) = h.app.explore_account().unwrap();
    assert!(xpub.starts_with("xpub"));
    assert_eq!(slip, None, "taproot has no SLIP-132 prefix");
    h.tap(ids::EXPLORE_PATH);
    h.tap(ids::EXPLORE_RECEIVE);
    h.key(Key::Enter);
    assert_eq!(h.app.explore_path().as_deref(), Some("m/86h/0h/0h/0/0"));
    let addresses = h.app.explore_addresses();
    assert_eq!(addresses[0].0, ScriptType::Taproot);
    assert_eq!(
        addresses[0].1,
        abandon_master()
            .account_xpub(ScriptType::Taproot, 0)
            .unwrap()
            .address(false, 0)
            .unwrap()
            .to_string()
    );
}

/// §2.7: the chooser checks a row and Continue confirms it.
#[test]
fn the_chooser_checks_then_confirms_the_key() {
    let mut h = Harness::new(PANEL);
    load_abandon(&mut h, None);
    h.open_load();
    h.load_choices(0);
    for w in ABANDON {
        h.type_text(w);
        h.key(Key::Enter);
    }
    h.finish_load(Some("TREZOR"));
    assert_eq!(h.app.fingerprints().len(), 2);
    open_explore(&mut h);
    assert_eq!(h.app.explore_fingerprint().unwrap().to_hex(), *b"73c5da0a");
    h.tap(ids::EXPLORE_USING);
    assert_eq!(h.app.explore_step(), Some(Step::Chooser));
    h.tap(ids::at(ids::EXPLORE_KEY_BASE, 1));
    assert_eq!(
        h.app.explore_source(),
        Some(Source::Loaded(0)),
        "the check alone changes nothing"
    );
    h.tap(ids::EXPLORE_CHOOSE_CONTINUE);
    assert_eq!(h.app.explore_source(), Some(Source::Loaded(1)));
    assert_eq!(h.app.explore_step(), Some(Step::Keys));
    assert_eq!(
        h.app.explore_fingerprint(),
        Some(common::expected_fingerprint(b"TREZOR", Network::Mainnet))
    );
    assert_ne!(native_segwit_address(&h.app), FIRST_RECEIVE);
    h.tap(ids::EXPLORE_USING);
    h.choose(
        ids::at(ids::EXPLORE_KEY_BASE, 0),
        ids::EXPLORE_CHOOSE_CONTINUE,
    );
    assert_eq!(native_segwit_address(&h.app), FIRST_RECEIVE);
}

/// §2.8: "Long secrets and long strings live on their own screen,
/// reached by a row." Each row opens the Secret screen for its value,
/// and the chevron returns to the menu that holds the row.
#[test]
fn every_secret_row_opens_its_own_screen() {
    let mut h = Harness::new(PANEL);
    load_abandon(&mut h, None);
    open_explore(&mut h);
    h.tap(ids::EXPLORE_XPRV);
    assert_eq!(h.app.explore_step(), Some(Step::Secret(SecretKind::Xprv)));
    assert!(h.app.rect_of(ids::CREATE_REVEAL).is_some(), "the panel");
    h.tap(ids::BACK);
    assert_eq!(h.app.explore_step(), Some(Step::Keys));

    open_bits(&mut h);
    for (row, secret) in [
        (ids::EXPLORE_ENTROPY, SecretKind::Entropy),
        (ids::EXPLORE_CHECKSUM_BITS, SecretKind::ChecksumBits),
        (ids::EXPLORE_SEED, SecretKind::Seed),
        (ids::EXPLORE_MASTER_XPRV, SecretKind::MasterXprv),
    ] {
        h.tap(row);
        assert_eq!(h.app.explore_step(), Some(Step::Secret(secret)));
        assert!(h.app.rect_of(ids::CREATE_REVEAL).is_some(), "{secret:?}");
        // The panel masks until a finger lands on it, and it keeps its
        // rectangle either way (§4.10).
        let masked = h.app.rect_of(ids::CREATE_REVEAL).unwrap();
        let point = h.press(ids::CREATE_REVEAL);
        assert_eq!(h.app.rect_of(ids::CREATE_REVEAL), Some(masked));
        h.release(point);
        h.tap(ids::BACK);
        assert_eq!(h.app.explore_step(), Some(Step::Bits));
    }
}

#[test]
fn wide_shows_a_sidebar_beside_the_same_hub_and_one_screen_at_a_time() {
    let mut h = Harness::new(WIDE);
    assert_eq!(h.app.class(), osk_ui::geom::SizeClass::Wide);
    // Home: the sidebar carries the areas and the pane the key list, so
    // the eight areas are on the screen once (`docs/DESIGN.md` §4.1).
    assert!(h.app.rect_of(ids::SIDEBAR).is_some());
    assert!(h.app.rect_of(ids::STATUS_TIER).is_some());
    let sidebar = h.app.rect_of(ids::SIDEBAR).unwrap();
    let tier = h.app.rect_of(ids::STATUS_TIER).unwrap();
    assert!(sidebar.right() <= tier.x, "the sidebar is left of the pane");
    load_abandon(&mut h, None);
    // The sidebar reaches every area from every screen.
    h.tap(ids::at(ids::HOME_TILE_BASE, 5));
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    assert_eq!(h.app.screen(), ScreenKind::Tools);
    h.tap(ids::TOOLS_EXPLORER);
    assert_eq!(h.app.screen(), ScreenKind::Explore);
    assert_eq!(h.app.explore_step(), Some(Step::Keys));
    assert!(
        h.app.rect_of(ids::CREATE_REVEAL).is_none(),
        "no secret panel on a menu"
    );
    assert!(h.app.rect_of(ids::EXPLORE_XPRV).is_some());
    open_bits(&mut h);
    assert!(h.app.rect_of(ids::EXPLORE_SEED).is_some());
    assert!(
        h.app.rect_of(ids::EXPLORE_XPRV).is_none(),
        "the key menu's rows are not on this one"
    );
    // The path keyboard's keys tap.
    h.tap(ids::BACK);
    h.tap(ids::EXPLORE_PATH);
    h.pad(ids::EXPLORE_PATH_KEYBOARD, KeyInput::Backspace);
    h.pad(ids::EXPLORE_PATH_KEYBOARD, KeyInput::Char('7'));
    h.pad(ids::EXPLORE_PATH_KEYBOARD, KeyInput::Done);
    assert_eq!(h.app.explore_path().as_deref(), Some("m/84h/0h/0h/0/7"));
    assert_eq!(h.app.explore_step(), Some(Step::Keys));
    h.tap(ids::at(ids::HOME_TILE_BASE, 4));
    assert_eq!(h.app.screen(), ScreenKind::Learn);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(!h.app.explore_has_input());
}

#[test]
fn a_small_panel_opens_one_screen_at_a_time() {
    let mut h = Harness::new(PANEL);
    load_abandon(&mut h, None);
    open_explore(&mut h);
    // The key menu carries the row into the second menu and the row
    // into this key's addresses.
    assert!(h.app.rect_of(ids::EXPLORE_BITS).is_some());
    assert!(h.app.rect_of(ids::EXPLORE_ADDRESSES).is_some());
    assert!(h.app.rect_of(ids::CREATE_REVEAL).is_none());
    open_bits(&mut h);
    h.tap(ids::EXPLORE_WORDS);
    assert!(h.app.rect_of(ids::CREATE_REVEAL).is_some());
    // Done and Back both return to the menu the row is on, then to the
    // key menu, then out of the explorer and on to Tools.
    h.tap(ids::EXPLORE_WORDS_DONE);
    assert_eq!(h.app.explore_step(), Some(Step::Bits));
    h.tap(ids::BACK);
    assert_eq!(h.app.explore_step(), Some(Step::Keys));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Tools);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

/// The four reference sizes (UX.md §2): the smallest panel, the 2.8"
/// reference, a phone and the desktop window.
const SIZES: [DisplayInfo; 4] = [
    DisplayInfo {
        width: 240,
        height: 320,
        dpi: 143,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 1,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    },
    DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 1,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    },
    DisplayInfo {
        width: 1080,
        height: 2340,
        dpi: 420,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    },
    WIDE,
];

/// Walks to the Explore screen that holds the secret `secret` and
/// returns a harness on it.
fn explore_secret(display: DisplayInfo, secret: SecretKind) -> Harness {
    let mut h = Harness::new(display);
    load_abandon(&mut h, None);
    open_explore(&mut h);
    let row = match secret {
        SecretKind::Xprv => ids::EXPLORE_XPRV,
        SecretKind::Entropy => ids::EXPLORE_ENTROPY,
        SecretKind::ChecksumBits => ids::EXPLORE_CHECKSUM_BITS,
        SecretKind::Seed => ids::EXPLORE_SEED,
        SecretKind::MasterXprv => ids::EXPLORE_MASTER_XPRV,
    };
    if secret != SecretKind::Xprv {
        open_bits(&mut h);
    }
    h.tap(row);
    h
}

/// A secret panel never holds a paged string: the panel's own surface is
/// the reveal target, so the pager would sit under the finger that
/// reveals it and could not be pressed.
fn no_secret_panel_pages_its_own_content_at(display: DisplayInfo) {
    for secret in SecretKind::ALL {
        let mut h = explore_secret(display, secret);
        for held in [false, true] {
            let point = held.then(|| h.press(ids::CREATE_REVEAL));
            let rect = h.app.rect_of(ids::CREATE_REVEAL).expect("the panel");
            for (id, chunk) in h.app.paged_chunks() {
                assert!(
                    chunk.intersect(&rect).is_empty(),
                    "a paged string ({}) is inside the {secret:?} panel at {}x{} (held: {held})",
                    id.0,
                    display.width,
                    display.height
                );
            }
            if let Some(p) = point {
                h.release(p);
            }
        }
    }
}

at_every_size!(
    no_secret_panel_pages_its_own_content_at =>
        no_secret_panel_pages_its_own_content_on_tiny,
        no_secret_panel_pages_its_own_content_on_panel,
        no_secret_panel_pages_its_own_content_on_phone,
        no_secret_panel_pages_its_own_content_on_desktop
);

/// Revealing a secret moves nothing: the panel keeps its rectangle and
/// every widget outside it keeps its own.
fn revealing_a_secret_moves_nothing_on_the_screen_at(display: DisplayInfo) {
    for secret in SecretKind::ALL {
        let mut h = explore_secret(display, secret);
        let point = h.press(ids::CREATE_REVEAL);
        let held_panel = h.app.rect_of(ids::CREATE_REVEAL).expect("the panel");
        let held: Vec<_> = h
            .app
            .placed_rects()
            .into_iter()
            .filter(|r| !held_panel.contains(r.x, r.y) || *r == held_panel)
            .collect();
        h.release(point);
        let masked_panel = h.app.rect_of(ids::CREATE_REVEAL).expect("the panel");
        let masked: Vec<_> = h
            .app
            .placed_rects()
            .into_iter()
            .filter(|r| !masked_panel.contains(r.x, r.y) || *r == masked_panel)
            .collect();
        assert_eq!(
            masked_panel, held_panel,
            "the {secret:?} panel resizes at {}x{}",
            display.width, display.height
        );
        assert_eq!(
            masked, held,
            "{secret:?} moves on reveal at {}x{}",
            display.width, display.height
        );
    }
}

at_every_size!(
    revealing_a_secret_moves_nothing_on_the_screen_at =>
        revealing_a_secret_moves_nothing_on_the_screen_on_tiny,
        revealing_a_secret_moves_nothing_on_the_screen_on_panel,
        revealing_a_secret_moves_nothing_on_the_screen_on_phone,
        revealing_a_secret_moves_nothing_on_the_screen_on_desktop
);

/// §4.5 and §2.11: the "Account key" row is drawn only where the account
/// key is a key of its own. At a path no deeper than the account it is
/// the leaf, which the "Extended public key" row already shows, so the
/// screen does not carry the same string twice with nothing to say why.
#[test]
fn the_account_key_row_is_absent_where_it_would_repeat_the_leaf() {
    let mut h = Harness::new(PANEL);
    load_abandon(&mut h, None);
    open_explore(&mut h);
    // The account itself: three levels, so the leaf is the account.
    h.tap(ids::EXPLORE_PATH);
    for _ in 0..15 {
        h.key(Key::Backspace);
    }
    h.type_text("84h/0h/0h");
    h.key(Key::Enter);
    assert_eq!(h.app.explore_path().as_deref(), Some("m/84h/0h/0h"));
    assert!(
        h.app.rect_of(ids::EXPLORE_ACCOUNT_XPUB).is_none(),
        "the account key repeats the leaf at an account path"
    );
    assert!(h.app.rect_of(ids::EXPLORE_XPUB).is_some(), "the leaf");

    // A level below it: the two are different keys, so both are rows.
    h.tap(ids::EXPLORE_PATH);
    h.type_text("/0");
    h.key(Key::Enter);
    assert_eq!(h.app.explore_path().as_deref(), Some("m/84h/0h/0h/0"));
    assert!(h.app.rect_of(ids::EXPLORE_ACCOUNT_XPUB).is_some());
    assert!(h.app.rect_of(ids::EXPLORE_XPUB).is_some());
    // Two rows, two keys: the elided values differ, which is what the
    // account-key row is on the screen to say.
    let texts = h.app.texts();
    let elided: Vec<&String> = texts.iter().filter(|t| t.starts_with("xpub ")).collect();
    assert_eq!(elided.len(), 2, "both key rows: {texts:?}");
    assert_ne!(elided[0], elided[1], "two rows, two keys: {texts:?}");
}
