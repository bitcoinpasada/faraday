//! Load samples (`docs/MODES.md` §3, §10 M6): in Demo the pill opens a
//! sheet that lists every sample, and Load ticked puts what is ticked in
//! the session the way a typed or scanned key and wallet go in. The tests
//! drive a `Faraday` with the real samples set, as the desktop does.

use crate::common::{demo_app, fingerprint_of_seed, json, show};
use faraday_bip::keys::{Fingerprint, Network};
use faraday_core::samples::{Pick, SampleKey};
use faraday_core::testkit::{self, compute};
use faraday_core::wallet::{Origin, Session};
use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_intake::{Part, part_of};
use faraday_samples::SAMPLES;
use faraday_shell_api::App;

/// The keys the sheet lists, in its order: those that load a key and that
/// the table lists.
fn listed() -> Vec<&'static SampleKey> {
    SAMPLES
        .keys
        .iter()
        .filter(|k| k.listed && k.plain.is_some())
        .collect()
}

fn wallet_pick(name: &str) -> Pick {
    let at = SAMPLES.wallets.iter().position(|w| w.name == name);
    Pick::Wallet(at.expect("a sample wallet") as u8)
}

fn key_pick(name: &str, word: &str) -> Pick {
    let at = listed()
        .iter()
        .position(|k| k.name == name && k.word == word)
        .expect("a sample key");
    Pick::Key(at as u8)
}

/// The sheet open over Home, drawn.
fn open(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(app.offers(Action::Samples), "the pill is on Home");
    app.press(Action::Samples);
    assert_eq!(app.sheet, Some(Sheet::Samples));
    let _ = app.frame();
}

/// Ticks `picks` on the open sheet and presses Load ticked.
fn load(app: &mut Faraday, picks: &[Pick]) {
    for &p in picks {
        app.press(Action::SampleTick(p));
    }
    let _ = app.frame();
    app.press(Action::SamplesLoad);
}

fn held(app: &Faraday, fp: [u8; 4]) -> bool {
    app.session.keys.position(Fingerprint(fp)).is_some()
}

fn plain_of(name: &str, word: &str) -> [u8; 4] {
    SAMPLES
        .keys
        .iter()
        .find(|k| k.name == name && k.word == word)
        .and_then(|k| k.plain)
        .expect("a sample key")
}

#[test]
fn in_demo_the_pill_opens_a_sheet_that_lists_every_sample() {
    let mut app = demo_app();
    open(&mut app);
    for i in 0..SAMPLES.wallets.len() {
        assert!(app.offers(Action::SampleTick(Pick::Wallet(i as u8))), "{i}");
    }
    let keys = listed();
    for i in 0..keys.len() {
        assert!(app.offers(Action::SampleTick(Pick::Key(i as u8))), "{i}");
    }
    // One row each, and no more: not the cipher seed, which loads no key,
    // and not the 3 of 5's keys, which load with their wallet.
    assert!(!app.offers(Action::SampleTick(
        Pick::Wallet(SAMPLES.wallets.len() as u8)
    )));
    assert!(!app.offers(Action::SampleTick(Pick::Key(keys.len() as u8))));
    assert_eq!(keys.len(), SAMPLES.keys.len() - 1 - 5);
    assert!(app.offers(Action::SamplesAll));
    assert!(app.offers(Action::Cancel), "Not now");
    // The compact panel has the pill too, and the sheet.
    let mut small = demo_app();
    show(&mut small, 480, 640, 286);
    open(&mut small);
    assert!(small.offers(Action::SampleTick(Pick::Wallet(0))));
}

/// Every row of the sheet and the type it shows under its name, as a
/// person reads them.
const TYPES: &[&str] = &[
    "2-of-3 multisig \u{b7} native SegWit",
    "single-sig \u{b7} native SegWit",
    "single-sig \u{b7} Taproot",
    "single-sig \u{b7} nested SegWit",
    "single-sig \u{b7} legacy",
    "2-of-3 multisig \u{b7} nested SegWit",
    "2-of-3 multisig \u{b7} legacy",
    "2-of-3 multisig \u{b7} Taproot",
    "timelock recovery \u{b7} native SegWit",
    "single-sig \u{b7} Taproot, script tree",
    "3-key MuSig2 \u{b7} Taproot",
    #[cfg(feature = "unreleased")]
    "2-of-3 FROST \u{b7} Taproot",
    "3-of-5 multisig \u{b7} native SegWit",
    "BIP-39 \u{b7} 24 words",
    "BIP-39 \u{b7} 12 words",
    "BIP-39 \u{b7} 15 words",
    "BIP-39 \u{b7} 18 words",
    "BIP-39 \u{b7} 21 words",
    "BIP-39 \u{b7} 12 words, Japanese",
    "SLIP-39 \u{b7} 1 share",
    "SLIP-39 \u{b7} 2 of 3 shares",
    "SLIP-39 \u{b7} groups, 3 shares",
    "codex32 \u{b7} secret",
    "codex32 \u{b7} 2 shares",
    "Seed XOR \u{b7} 2 parts",
];

#[test]
fn every_row_of_the_sheet_says_what_the_sample_is() {
    let mut app = demo_app();
    open(&mut app);
    let texts = app.drawn_texts();
    for want in TYPES {
        assert!(texts.iter().any(|t| t == want), "no row says {want:?}");
    }
    // One type line for every row, and every one of them is in the list.
    let rows = SAMPLES.wallets.len() + listed().len();
    let shown = texts.iter().filter(|t| TYPES.contains(&t.as_str())).count();
    assert_eq!(shown, rows);
    // A sample added later gets its line from what it records: none is
    // blank, and the sheet and the sample agree.
    for w in SAMPLES.wallets {
        assert!(texts.contains(&w.kind_label()), "{}", w.name);
    }
    for k in listed() {
        assert!(texts.contains(&k.kind_label()), "{}", k.name);
    }
}

#[test]
fn the_sheet_says_the_passphrase_is_faraday_in_a_code_box_and_never_password() {
    let mut app = demo_app();
    open(&mut app);
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "In Demo, the sample passphrase is")
    );
    assert!(texts.iter().any(|t| t == "faraday"));
    assert!(!texts.iter().any(|t| t.contains("password")));
    // The characters are drawn as they are, with no quote marks round them.
    assert!(!texts.iter().any(|t| t.contains("\"faraday\"")));
    // On the compact panel too.
    let mut small = demo_app();
    show(&mut small, 480, 640, 286);
    open(&mut small);
    assert!(small.drawn_texts().iter().any(|t| t == "faraday"));
}

#[test]
fn ticking_savings_and_loading_it_loads_the_wallet_and_its_three_keys() {
    let c = compute();
    let mut app = demo_app();
    open(&mut app);
    load(&mut app, &[wallet_pick("sample_savings")]);
    assert_eq!(app.sheet, None);
    assert_eq!(app.toast_text(), Some("Loaded 1 wallet and 3 keys"));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.wallets[0].name, "Savings");
    for word in ["bacon", "zebra", "summer"] {
        assert!(held(&app, plain_of("sample_words", word)), "{word}");
    }
    assert_eq!(app.session.keys.len(), 3);
    // Its first receive address is the testkit's Savings wallet's.
    let mut kit = Session::on(Network::Testnet);
    kit.add_wallet(&c, "Savings", &testkit::savings(), "t", Origin::Named)
        .expect("the testkit's wallet");
    let want = kit.wallets[0].address_on(&c, Network::Testnet, false, 0);
    let got = app.session.wallets[0].address_on(&c, Network::Testnet, false, 0);
    assert_eq!(got, want);
    assert!(got.starts_with("tb1"), "{got}");
}

#[test]
fn ticking_the_slip39_set_loads_its_key_with_the_vectors_fingerprint() {
    let c = compute();
    let mut app = demo_app();
    open(&mut app);
    load(&mut app, &[key_pick("sample_slip39_two_of_three", "")]);
    assert_eq!(app.session.keys.len(), 1);
    // The published shares of the vector's fourth case, recovered with no
    // passphrase, by the code a typed share goes through.
    let case = &json("slip39/vectors.json")[3];
    let shares: Vec<_> = case[1]
        .as_array()
        .expect("mnemonics")
        .iter()
        .map(|m| match part_of(m.as_str().expect("text")) {
            Some(Part::Slip39(s)) => s,
            _ => panic!("a share"),
        })
        .collect();
    let seed = faraday_keys::slip39_seed(&c, &shares, b"").expect("recovers");
    assert!(held(&app, fingerprint_of_seed(&seed)));
}

#[test]
fn a_set_loads_as_one_key_and_a_vector_with_a_passphrase_loads_without_it() {
    let mut app = demo_app();
    open(&mut app);
    let picks = [
        key_pick("sample_codex32_shares", ""),
        key_pick("sample_xor", ""),
        key_pick("sample_abandon", ""),
    ];
    load(&mut app, &picks);
    assert_eq!(app.session.keys.len(), 3);
    for name in ["sample_codex32_shares", "sample_xor", "sample_abandon"] {
        let k = SAMPLES.keys.iter().find(|k| k.name == name).expect(name);
        assert!(held(&app, k.plain.expect("a key")), "{name}");
    }
}

#[test]
fn what_is_loaded_is_ticked_and_dimmed_when_the_sheet_comes_back() {
    let mut app = demo_app();
    open(&mut app);
    let (savings, bacon) = (
        wallet_pick("sample_savings"),
        key_pick("sample_words", "bacon"),
    );
    load(&mut app, &[savings]);
    open(&mut app);
    // No press ticks it: it is done.
    assert!(!app.offers(Action::SampleTick(savings)));
    assert!(!app.offers(Action::SampleTick(bacon)));
    assert!(app.offers(Action::SampleTick(key_pick("sample_words", "flag"))));
    app.press(Action::SampleTick(savings));
    app.press(Action::SamplesLoad);
    assert_eq!(
        app.sheet,
        Some(Sheet::Samples),
        "nothing was ticked to load"
    );
    // A wallet loaded some other way shows the same.
    let c = compute();
    let mut other = demo_app();
    other
        .session
        .add_wallet(&c, "Mine", &testkit::savings(), "t", Origin::Named)
        .expect("the wallet");
    open(&mut other);
    assert!(!other.offers(Action::SampleTick(savings)));
    assert!(other.offers(Action::SampleTick(wallet_pick("sample_spending"))));
}

#[test]
fn a_key_already_loaded_is_skipped_without_an_error_and_the_rest_loads() {
    let mut app = demo_app();
    open(&mut app);
    load(&mut app, &[key_pick("sample_words", "bacon")]);
    assert_eq!(app.toast_text(), Some("Loaded 1 key"));
    open(&mut app);
    load(&mut app, &[wallet_pick("sample_savings")]);
    // Savings brought zebra and summer; bacon was there.
    assert_eq!(app.toast_text(), Some("Loaded 1 wallet and 2 keys"));
    assert_eq!(app.session.keys.len(), 3);
}

#[test]
fn select_all_ticks_every_sample_not_loaded_and_unselect_all_clears_them() {
    let mut app = demo_app();
    open(&mut app);
    app.press(Action::SamplesAll);
    app.press(Action::SamplesAll);
    // Nothing is ticked: Load ticked does nothing, the sheet stays.
    app.press(Action::SamplesLoad);
    assert_eq!(app.sheet, Some(Sheet::Samples));
    assert!(app.session.wallets.is_empty() && app.session.keys.is_empty());
    // Not now closes it and loads nothing.
    app.press(Action::SamplesAll);
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.is_empty() && app.session.keys.is_empty());
}

#[test]
fn select_all_and_load_ticked_loads_every_wallet_and_every_key() {
    let mut app = demo_app();
    open(&mut app);
    app.press(Action::SamplesAll);
    app.press(Action::SamplesLoad);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), SAMPLES.wallets.len());
    // Every key the sheet lists, and the five the 3 of 5 uses.
    assert_eq!(app.session.keys.len(), listed().len() + 5);
    for k in listed() {
        assert!(held(&app, k.plain.expect("a key")), "{}", k.name);
    }
    let toast = app.toast_text().expect("a toast").to_string();
    let loaded = format!("Loaded {} wallets and ", SAMPLES.wallets.len());
    assert!(toast.starts_with(&loaded), "{toast}");
    // Everything is loaded: the sheet has nothing left to tick.
    open(&mut app);
    app.press(Action::SamplesAll);
    app.press(Action::SamplesLoad);
    assert_eq!(app.sheet, Some(Sheet::Samples));
}

#[test]
fn new_keys_next_sample_skips_one_the_sheet_loaded() {
    let mut app = demo_app();
    let first = |app: &Faraday| app.session.keys.next_handout(24).map(|(w, _)| w);
    assert_eq!(first(&app), Some("bacon"));
    open(&mut app);
    load(&mut app, &[key_pick("sample_words", "bacon")]);
    assert_eq!(first(&app), Some("flag"));
}

#[test]
fn on_testnet_and_on_the_stick_no_pill_shows_and_the_action_does_nothing() {
    // Testnet: samples are set, Demo is not on.
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Testnet).expect("testnet");
    app.samples = Some(&SAMPLES);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(!app.offers(Action::Samples));
    app.press(Action::Samples);
    assert_eq!(app.sheet, None);
    // The stick: Mainnet, no samples.
    let mut stick = testkit::started();
    let _ = stick.frame();
    assert!(!stick.offers(Action::Samples));
    stick.press(Action::Samples);
    assert_eq!(stick.sheet, None);
    // Demo with no samples set has no pill either.
    let mut bare = demo_app();
    bare.samples = None;
    let _ = bare.frame();
    assert!(!bare.offers(Action::Samples));
    bare.press(Action::Samples);
    assert_eq!(bare.sheet, None);
}

#[test]
fn every_sample_has_a_name_a_person_reads() {
    use faraday_core::samples::label;
    for k in SAMPLES.keys {
        assert!(label(k.name, k.word, k.count).is_some(), "{}", k.name);
    }
    for w in SAMPLES.wallets {
        assert!(label(w.name, "", 0).is_some(), "{}", w.name);
    }
    assert_eq!(
        label("sample_words", "bacon", 24).as_deref(),
        Some("bacon ×24")
    );
}

/// The Load samples sheet on a 1280 x 800 window, longer than the window.
fn long_sheet() -> faraday_core::Faraday {
    let mut app = demo_app();
    crate::common::show(&mut app, 1280, 800, 160);
    app.press(Action::Samples);
    let _ = app.frame();
    app
}

#[test]
fn the_load_sheet_shows_a_scrollbar_with_the_pointer_over_its_edge_and_it_can_be_dragged() {
    use faraday_shell_api::{Event, TouchPhase};
    let mut app = long_sheet();
    let rest = app.frame().rgba.to_vec();
    // The sheet is 640 wide, centred: its right edge is at 960.
    app.event(Event::Hover { x: 955, y: 400 });
    assert!(app.frame().rgba.to_vec() != rest, "the bar shows");
    let first = |app: &faraday_core::Faraday| {
        app.hit_box(Action::SampleTick(Pick::Wallet(0)))
            .map(|b| b.1)
    };
    let at_rest = first(&app);
    let touch = |app: &mut faraday_core::Faraday, y: u16, phase| {
        app.event(Event::Touch { x: 955, y, phase });
        let _ = app.frame();
    };
    touch(&mut app, 650, TouchPhase::Down);
    let low = first(&app);
    touch(&mut app, 0, TouchPhase::Move);
    let top = first(&app);
    touch(&mut app, 0, TouchPhase::Up);
    assert!(
        low.is_none_or(|y| Some(y) < at_rest),
        "pressed low on the track, the list went there"
    );
    assert_eq!(top, at_rest, "dragged to the top, it followed");
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Samples));
}
