//! End-to-end flows driven through the shell contract: `Event`s in,
//! `Command`s and the non-secret inspection API out
//! (`docs/PLANNING.md` §4.2, §16.7).

mod common;

use common::{ABANDON, Harness, PANEL, PHONE, expected_fingerprint};
use opensigner_core::ScreenKind;
use opensigner_core::ids;
use opensigner_core::load::Step;
use opensigner_core::strings;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{Network, ScriptType};
use osk_shell_api::{App, ButtonId, Command, Event, Key, TouchPhase};

#[test]
fn home_draws_after_display() {
    let mut h = Harness::new(PANEL);
    assert_eq!(
        h.drain(),
        vec![Command::RequestEntropy, Command::Draw, Command::Draw],
        "entropy is asked for before the first frame; its answer redraws"
    );
    assert_eq!(h.app.frame().rgba.len(), 640 * 480 * 4);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.fingerprints().is_empty());
    // Home is the status line and the six tiles, loaded or not.
    assert!(h.app.rect_of(ids::at(ids::HOME_TILE_BASE, 0)).is_some());
    assert!(h.app.rect_of(ids::STATUS_TIER).is_some());
}

/// §4.1: Home is the six tiles of the launcher and nothing else.
/// Settings is one of the six, so the status line carries no gear.
#[test]
fn home_is_six_tiles_and_nothing_else() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let tiles: Vec<_> = (0..6)
        .map(|i| {
            h.app
                .rect_of(ids::at(ids::HOME_TILE_BASE, i))
                .unwrap_or_else(|| panic!("tile {i}"))
        })
        .collect();
    assert!(
        h.app.rect_of(ids::at(ids::HOME_TILE_BASE, 6)).is_none(),
        "six and no more"
    );
    // Equal tiles, to the pixel a row's own rounding leaves.
    let first = tiles[0];
    for (i, t) in tiles.iter().enumerate() {
        assert!(
            (t.w - first.w).abs() <= 1 && (t.h - first.h).abs() <= 1,
            "tile {i} is not the size of the others"
        );
        assert!(t.h <= t.w, "tile {i} is taller than it is wide");
    }
    let texts = h.app.texts();
    for label in [
        s.home_wallets,
        s.home_keys,
        s.home_scan,
        s.home_tools,
        s.home_learn,
        s.home_settings,
    ] {
        assert!(texts.iter().any(|t| t == label), "{label} is a tile");
    }
    // Nothing about what is loaded: Home never changes shape.
    assert!(
        !texts.iter().any(|t| t == "73c5da0a"),
        "the key is on Keys, not on Home: {texts:?}"
    );
    h.tap(ids::at(ids::HOME_TILE_BASE, 5));
    assert_eq!(h.app.screen(), ScreenKind::Settings);
}

/// A key is reached from Keys, and its page carries neither addresses
/// nor an export: those are a wallet's. What is done with the key
/// itself is here (`docs/PLANNING.md` §16.127 rule 3).
#[test]
fn a_key_page_carries_no_wallet_screens() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    let texts = h.app.texts();
    for gone in [s.detail_addresses, s.wallet_export] {
        assert!(
            !texts.iter().any(|t| t == gone),
            "{gone} is not a key's: {texts:?}"
        );
    }
    for kept in [
        s.detail_backup,
        s.key_made_of,
        s.detail_open_passphrase,
        s.wallet_forget,
    ] {
        assert!(texts.iter().any(|t| t == kept), "{kept} stays: {texts:?}");
    }
}

#[test]
fn empty_state_tap_opens_the_load_wizard_and_escape_cancels() {
    let mut h = Harness::new(PANEL);
    h.open_load();
    assert_eq!(h.app.screen(), ScreenKind::Load);
    assert!(h.drain().is_empty(), "drained by tap");
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Keys, "back to where it started");
    assert!(h.app.fingerprints().is_empty());
}

#[test]
fn loading_abandon_about_by_physical_keys_yields_73c5da0a() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    assert!(h.app.load_suspects().is_empty());
    h.finish_load(None);
    let fps = h.app.fingerprints();
    assert_eq!(fps.len(), 1);
    assert_eq!(fps[0].to_hex(), *b"73c5da0a");
    // The key is a row on Keys, and no wallet was made up for it.
    h.open_keys();
    assert!(h.app.rect_of(ids::at(ids::KEYS_ROW_BASE, 0)).is_some());
    h.open_wallets();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none()
    );
    h.go_home();
    // Loading the same key again does not duplicate it.
    h.open_load();
    h.load_choices(0);
    for w in ABANDON {
        h.type_text(w);
        h.key(Key::Enter);
    }
    h.finish_load(None);
    assert_eq!(h.app.fingerprints().len(), 1);
}

#[test]
fn a_passphrase_makes_a_different_key() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(Some("TREZOR"));
    let fps = h.app.fingerprints();
    assert_eq!(fps.len(), 1);
    assert_eq!(fps[0], expected_fingerprint(b"TREZOR", Network::Mainnet));
    assert_ne!(fps[0].to_hex(), *b"73c5da0a");
}

#[test]
fn passphrase_characters_mask_after_a_tick() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_ADD_PASSPHRASE, ids::LOAD_PASS_CONTINUE);
    h.tick(1000);
    h.type_text("a");
    // Before the deadline a tick does not redraw; after it, it does.
    h.app.event(Event::Tick {
        now_ms: h.now_ms + 100,
    });
    assert!(h.drain().is_empty());
    h.app.event(Event::Tick {
        now_ms: h.now_ms + 600,
    });
    assert_eq!(h.drain(), vec![Command::Draw]);
}

#[test]
fn a_wrong_word_is_reported_by_position() {
    let mut h = Harness::new(PANEL);
    let right = [
        "legal", "winner", "thank", "year", "wave", "sausage", "worth", "useful", "legal",
        "winner", "thank", "yellow",
    ];
    let mut words = right;
    // Pick a similar word for position 9 that breaks the checksum.
    let lang = Language::English;
    let mut idx = [0u16; 12];
    for (i, w) in words.iter().enumerate() {
        idx[i] = lang.index_of(w).unwrap();
    }
    let wrong = ["legend", "lemon", "length", "leg"]
        .into_iter()
        .find(|w| {
            let mut t = idx;
            t[8] = lang.index_of(w).unwrap();
            Mnemonic::from_indices(lang, &t).is_err()
        })
        .unwrap();
    words[8] = wrong;
    h.start_load(&words);
    let suspects = h.app.load_suspects();
    assert!(suspects.contains(&9), "{suspects:?}");
    // §5 Result: the record names the first suspect and the secondary
    // action jumps to it; the primary starts the words over.
    let first = suspects[0] as usize - 1;
    let texts = h.app.texts();
    let label = strings::fill1(strings::EN.load_suspect_word, &format!("{}", first + 1));
    assert!(texts.contains(&label), "{texts:?}");
    h.tap(ids::at(ids::LOAD_FIX_BASE, first));
    assert_eq!(h.app.load_step(), Some(Step::Words));
    h.type_text(right[first]);
    h.key(Key::Enter);
    assert_eq!(
        h.app.load_step(),
        Some(Step::Checksum),
        "the later words kept"
    );
    // Start over and type them all correctly.
    h.tap(ids::LOAD_START_OVER);
    assert_eq!(h.app.load_step(), Some(Step::Words));
    for w in right {
        h.type_text(w);
        h.key(Key::Enter);
    }
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
    assert!(h.app.load_suspects().is_empty());
    h.finish_load(None);
    assert_eq!(h.app.fingerprints().len(), 1);
}

#[test]
fn cancelling_mid_wizard_leaves_no_keys() {
    let mut h = Harness::new(PANEL);
    h.open_load();
    h.load_choices(0);
    h.type_text("abandon");
    h.key(Key::Enter);
    h.type_text("zoo");
    // Back through the header control, step by step, then out.
    h.tap(ids::BACK);
    assert_eq!(h.app.load_step(), Some(Step::Language));
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.load_step(), Some(Step::Source));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Keys);
    assert!(h.app.fingerprints().is_empty());
}

#[test]
fn forget_removes_the_key() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEY_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Forget);
    h.hold(ids::DETAIL_FORGET);
    assert!(h.app.fingerprints().is_empty());
    // The row went with the key.
    h.open_keys();
    assert!(h.app.rect_of(ids::at(ids::KEYS_ROW_BASE, 0)).is_none());
}

#[test]
fn network_setting_changes_the_first_address() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let mainnet = h.app.addresses(0, ScriptType::NativeSegwit, false, 1);
    assert_eq!(mainnet, vec!["bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"]);
    h.open_settings();
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    h.tap(ids::SETTINGS_NETWORK_ROW);
    assert_eq!(h.app.screen(), ScreenKind::Setting);
    h.tap(ids::at(ids::SETTINGS_NET_BASE, 2));
    assert_eq!(h.app.network(), Network::Signet);
    // §4.2: the tap applied it and the Choice is still on screen.
    assert_eq!(h.app.screen(), ScreenKind::Setting);
    h.tap(ids::BACK);
    let signet = h.app.addresses(0, ScriptType::NativeSegwit, false, 1);
    assert_ne!(signet, mainnet);
    assert!(signet[0].starts_with("tb1q"), "{}", signet[0]);
    // The fingerprint does not depend on the network.
    assert_eq!(h.app.fingerprints()[0].to_hex(), *b"73c5da0a");
    assert_eq!(
        h.app.fingerprints()[0],
        expected_fingerprint(b"", Network::Signet)
    );
    // The key's wallet derives on the new network too.
    h.tap(ids::BACK);
    h.open_single_sig(0);
    h.tap(ids::WALLET_ADDRESSES);
    assert_eq!(h.app.screen(), ScreenKind::Addresses);
    // §5 Addresses is the list; a row opens the one address whole.
    assert!(h.app.rect_of(ids::at(ids::ADDR_ROW_BASE, 0)).is_some());
    h.tap(ids::at(ids::ADDR_ROW_BASE, 0));
    assert!(h.app.rect_of(ids::ADDR_TEXT).is_some());
}

#[test]
fn wipe_is_a_tap_to_its_own_screen_and_then_a_hold() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_settings();
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    assert!(
        h.app.rect_of(ids::SETTINGS_WIPE).is_none(),
        "the hold is not on the settings page"
    );
    h.tap(ids::SETTINGS_WIPE_ROW);
    assert_eq!(h.app.screen(), ScreenKind::WipeAll);
    assert_eq!(h.app.fingerprints().len(), 1, "the tap wipes nothing");
    // The app-bar back is the way out; there is no Cancel.
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    assert_eq!(h.app.fingerprints().len(), 1);
    h.tap(ids::SETTINGS_WIPE_ROW);
    h.hold(ids::SETTINGS_WIPE);
    assert!(h.app.fingerprints().is_empty());
    // §5 Result: what the wipe removed, which is the confirmation the
    // empty Home used to have to carry (UX review 2026-09-07, §3.8).
    assert_eq!(h.app.screen(), ScreenKind::Wiped);
    let texts = h.app.texts();
    let title = strings::EN.settings_wiped_title;
    assert!(texts.iter().any(|t| t == title), "{texts:?}");
    assert!(texts.iter().any(|t| t == "1"), "the key count: {texts:?}");
    h.tap(ids::SETTINGS_WIPED_DONE);
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

#[test]
fn back_reaches_home_within_two_steps_from_everywhere() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    // Home → Keys → the key is the deepest document path.
    h.open_key(0);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Keys);
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    // Settings.
    h.open_settings();
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    h.send(Event::Button(ButtonId::Back));
    assert_eq!(h.app.screen(), ScreenKind::Home);
    // The wizard opened from Add a key: Escape cancels, Back returns.
    h.open_keys();
    h.tap(ids::KEYS_ADD);
    h.tap(ids::KEYS_LOAD);
    h.choose(ids::LOAD_SOURCE_TYPE, ids::LOAD_SOURCE_CONTINUE);
    h.tap(ids::at(ids::LOAD_COUNT_BASE, 4));
    assert_eq!(h.app.screen(), ScreenKind::Load);
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Add);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Keys);
}

#[test]
fn touch_drives_the_bip39_keyboard_and_candidates() {
    let mut h = Harness::new(PHONE);
    h.open_load();
    h.load_choices(0);
    assert_eq!(h.app.load_step(), Some(Step::Words));
    assert!(h.app.candidates().is_empty(), "nothing typed");
    // Tap the "z" key: bottom-left letter of the QWERTY layout, right of
    // the backspace key which spans 1.5 units of 10.
    let kb = h.app.rect_of(ids::LOAD_KEYBOARD).unwrap();
    let unit = kb.w as f32 / 10.0;
    let z = (
        (kb.x as f32 + 1.5 * unit + unit / 2.0) as u16,
        (kb.y + kb.h - kb.h / 6) as u16,
    );
    h.send(Event::Touch {
        x: z.0,
        y: z.1,
        phase: TouchPhase::Down,
    });
    h.send(Event::Touch {
        x: z.0,
        y: z.1,
        phase: TouchPhase::Up,
    });
    let strip = h
        .app
        .rect_of(ids::LOAD_CANDIDATES)
        .expect("z → zebra zero zone zoo");
    // Tap the first candidate ("zebra") and the word commits.
    let x = (strip.x + strip.w / 8) as u16;
    let y = strip.center().y as u16;
    h.send(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    h.send(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    assert!(h.app.candidates().is_empty(), "prefix cleared");
    // Physical backspace un-commits it again; a second one is harmless.
    h.key(Key::Backspace);
    h.key(Key::Backspace);
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Keys);
}

/// Explore over words typed into it: the Load wizard's own steps hand
/// their indices to the workspace, no key is added, the path applies on
/// ✓, and leaving asks before the words go.
#[test]
fn exploring_typed_words_adds_no_key_and_asks_before_dropping_them() {
    use opensigner_core::explore::{Source, Step as ExploreStep};
    let mut h = Harness::new(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    assert_eq!(h.app.screen(), ScreenKind::Explore);
    assert_eq!(h.app.explore_source(), Some(Source::None));
    h.tap(ids::EXPLORE_TYPE);
    h.tap(ids::SCAN_TYPE);
    assert_eq!(h.app.load_step(), Some(Step::Count), "the source step goes");
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    for w in ABANDON {
        h.type_text(w);
        h.key(Key::Enter);
    }
    // The checksum Result's action is what it does here.
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
    assert!(
        h.app.labels().iter().any(|l| l == strings::EN.home_explore),
        "the checksum's action is Explore: {:?}",
        h.app.labels()
    );
    h.tap(ids::LOAD_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Explore);
    assert!(h.app.fingerprints().is_empty(), "no key was added");
    assert_eq!(h.app.explore_fingerprint().unwrap().to_hex(), *b"73c5da0a");

    // The path editor applies on ✓ and the key menu comes back with it.
    h.tap(ids::EXPLORE_PATH);
    assert_eq!(h.app.explore_step(), Some(ExploreStep::Path));
    h.tap(ids::at(ids::EXPLORE_PRESET_BASE, 0));
    h.key(Key::Enter);
    assert_eq!(h.app.explore_step(), Some(ExploreStep::Keys));
    // §4.6: the Purpose group and the Chain pair are two independent
    // choices, so tapping Legacy rewrites the account levels and leaves
    // the chain and index the path already had.
    assert_eq!(h.app.explore_path().as_deref(), Some("m/44h/0h/0h/0/0"));

    // Escape asks, and keeping the words stays in the area.
    h.key(Key::Escape);
    assert_eq!(h.app.explore_step(), Some(ExploreStep::Discard));
    h.tap(ids::EXPLORE_KEEP);
    assert_eq!(h.app.screen(), ScreenKind::Explore);
    h.key(Key::Escape);
    h.tap(ids::EXPLORE_DISCARD);
    assert_eq!(h.app.screen(), ScreenKind::Tools, "the explorer is a tool");
    assert!(!h.app.explore_has_input());
}

/// A tap on the second row of candidates commits the word shown there,
/// not the word above it. On the phone the strips hold four words a row,
/// so "a" offers "abandon ability able about" and then "above …".
#[test]
fn a_candidate_in_the_second_row_is_the_one_tapped() {
    let mut h = Harness::new(PHONE);
    h.open_load();
    h.load_choices(0);
    assert_eq!(h.app.load_step(), Some(Step::Words));
    // The "a" key: first of the middle letter row, which starts half a
    // unit in.
    let kb = h.app.rect_of(ids::LOAD_KEYBOARD).unwrap();
    let unit = kb.w as f32 / 10.0;
    let a = ((kb.x as f32 + unit) as u16, (kb.y + kb.h / 2) as u16);
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        h.send(Event::Touch {
            x: a.0,
            y: a.1,
            phase,
        });
    }
    let offered = h.app.candidates();
    assert!(offered.len() > 4, "a → more than one row: {offered:?}");
    let second_row_first = offered[4].clone();
    let strip = h
        .app
        .rect_of(ids::LOAD_CANDIDATES_2)
        .expect("a second row of candidates on the phone");
    let (x, y) = ((strip.x + strip.w / 8) as u16, strip.center().y as u16);
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        h.send(Event::Touch { x, y, phase });
    }
    assert!(h.app.candidates().is_empty(), "the word committed");
    // The committed word is masked on screen, so read it back the way
    // a person would: Backspace un-commits it into the field, and the
    // candidates for that prefix are the word itself.
    h.key(Key::Backspace);
    let back = h.app.candidates();
    assert_eq!(
        back,
        vec![second_row_first.clone()],
        "the word tapped in the second row is the one that committed"
    );
    assert_ne!(back, vec![offered[0].clone()], "not the first row's word");
}

/// §15 item 35: there is no SLIP-132 prefix for P2TR and there will not
/// be one, so the format Choice leaves the row off for a Taproot key.
/// Every script type that has the form still offers it.
#[test]
fn the_export_format_choice_offers_slip132_only_where_the_form_exists() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_single_sig(0);
    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::EXPORT_FORMAT);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.export_slip132),
        "SegWit has a SLIP-132 form: {texts:?}"
    );
    h.tap(ids::BACK);

    h.go_home();
    h.add_single_sig(0, 3);
    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::EXPORT_FORMAT);
    let texts = h.app.texts();
    assert!(
        !texts.iter().any(|t| t == strings::EN.export_slip132),
        "Taproot has none: {texts:?}"
    );
}

/// A format the script type has no form of does not stay picked when the
/// script type changes: SLIP-132 chosen on a SegWit key becomes the
/// account key on Taproot, so the row never names a format the Choice
/// does not offer.
#[test]
fn a_taproot_wallet_names_no_format_it_has_no_form_of() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_single_sig(0);
    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, 2), ids::PICK_CONTINUE);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.export_slip132),
        "SLIP-132 is the picked format: {texts:?}"
    );

    // The Taproot wallet of the same key is a wallet of its own, and
    // its export never names a form Taproot has none of.
    h.go_home();
    h.add_single_sig(0, 3);
    h.tap(ids::WALLET_EXPORT);
    let texts = h.app.texts();
    assert!(
        !texts.iter().any(|t| t == strings::EN.export_slip132),
        "Taproot has no SLIP-132 form: {texts:?}"
    );
    h.tap(ids::EXPORT_FORMAT);
    let texts = h.app.texts();
    assert!(
        !texts.iter().any(|t| t == strings::EN.export_slip132),
        "and the Choice does not offer it: {texts:?}"
    );
}
