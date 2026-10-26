//! Make the rest, Add a share, Check a string's worksheet, the mark's
//! line, a set's facts for a key loaded from strings, and Practice's sample
//! in place of the strings made (`docs/CODEX32.md` §8.1 to §8.4, §9.4,
//! §12): what a person typing codex32 strings into Add a key, or making
//! a set in Practice, would notice. On BIP 93's published vectors 2 and 3.

use faraday_bip::codex32::Codex32;
use faraday_bip::keys::Network;
use faraday_core::codex32::{C32, Card, Given, SPLITS, Split, WAYS, Way};
use faraday_core::codex32_correct::{Mark, Press};
use faraday_core::forms::{C32Press, Form};
use faraday_core::keygen::{Start, kstep};
use faraday_core::samples::{SampleForm, SampleKey, SampleSize, Samples};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, Event, Key};
use faraday_ui::keyboard::OskPress;

/// BIP 93's vector 2: shares A and C, D derived from them.
const SHARE_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const SHARE_C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
const SHARE_D: &str = "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg";
/// Vector 2's key: the fingerprint of its xprv.
const NAME_FP: [u8; 4] = [0xfa, 0xb6, 0x86, 0x8a];
/// BIP 93's vector 3, k = 3: shares A and C, D, E and F derived, its secret.
const CASH_A: &str = "ms13casha320zyxwvutsrqpnmlkjhgfedca2a8d0zehn8a0t";
const CASH_C: &str = "ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr";
const CASH_D: &str = "ms13cashd0wsedstcdcts64cd7wvy4m90lm28w4ffupqs7rm";
const CASH_E: &str = "ms13casheekgpemxzshcrmqhaydlp6yhms3ws7320xyxsar9";
const CASH_F: &str = "ms13cashf8jh6sdrkpyrsp5ut94pj8ktehhw2hfvyrj48704";

const BECH32: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

fn codex32_index() -> u8 {
    Form::ALL.iter().position(|f| *f == Form::Codex32).unwrap() as u8
}

/// Add a key's codex32 form, open.
fn adding(app: &mut Faraday) {
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(codex32_index()));
}

fn add(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Enter));
}

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
}

fn flow(app: &Faraday) -> &faraday_core::codex32::Codex32Make {
    app.keygen()
        .and_then(|k| k.codex32.as_ref())
        .expect("Make codex32 shares open")
}

fn strings(app: &Faraday) -> Vec<String> {
    flow(app)
        .strings()
        .map(|s| s.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

/// Each string on the Strings card skipped, behind its caution: Strings
/// made open.
fn past_strings(app: &mut Faraday) {
    c32(app, C32::Next);
    assert_eq!(flow(app).open(), Some(Card::Strings));
    while flow(app).open() == Some(Card::Strings) {
        c32(app, C32::Skip);
        c32(app, C32::Skip);
    }
    assert_eq!(flow(app).open(), Some(Card::Made));
}

/// §8.1: Make the rest pressed after the first share; once the second is
/// in, New key opens on the strings typed, asks how many in all, and makes
/// the rest on their translation worksheets; Lock in loads the key with
/// the set's facts.
#[test]
fn make_the_rest_from_two_shares_typed_makes_the_published_third() {
    let mut app = testkit::started();
    adding(&mut app);
    add(&mut app, SHARE_A);
    assert!(crate::common::offered(
        &mut app,
        Action::Codex32Form(C32Press::Rest)
    ));
    app.press(Action::Codex32Form(C32Press::Rest));
    add(&mut app, SHARE_C);
    assert!(
        app.session.keys.is_empty(),
        "the strings make the rest, not the key"
    );
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(app.keygen().unwrap().open, Some(kstep::CODEX32));
    let m = flow(&app);
    assert_eq!(m.given_kind(), Some(Given::Rest));
    assert_eq!(m.open(), Some(Card::Scheme));
    assert_eq!(m.identifier(), "name");
    assert_eq!(m.k_n(), (2, 3));
    // Fewer strings in all than those typed and one more are not offered.
    c32(&mut app, C32::N(-1));
    assert_eq!(flow(&app).k_n(), (2, 3));
    let _ = app.frame();
    c32(&mut app, C32::Next);
    let m = flow(&app);
    assert_eq!(m.open(), Some(Card::Sheets), "{:?}", m.note());
    assert_eq!(strings(&app), [SHARE_D]);
    let tabs: Vec<String> = flow(&app).tabs().into_iter().map(|t| t.0).collect();
    assert_eq!(
        tabs.len(),
        4,
        "A's and C's checksums as typed, share D and the secret: {tabs:?}"
    );
    let _ = app.frame();
    past_strings(&mut app);
    let _ = app.frame();
    c32(&mut app, C32::Lock);
    assert_eq!(app.session.keys.len(), 1);
    let key = &app.session.keys[0];
    assert_eq!(key.fingerprint.0, NAME_FP);
    let set = key.codex32.clone().expect("the set's facts");
    assert_eq!(set.threshold, 2);
    assert_eq!(&set.identifier, b"name");
    assert_eq!(set.indices(), b"acd");
}

/// §8.2: Add a share from vector 3's three strings, the secret not among
/// them; the index offered is the next unused one, another typed over it;
/// an index typed in is refused.
#[test]
fn add_a_share_at_an_index_typed_is_the_published_string() {
    let mut app = testkit::started();
    adding(&mut app);
    add(&mut app, CASH_A);
    app.press(Action::Codex32Form(C32Press::AddShare));
    add(&mut app, CASH_C);
    add(&mut app, CASH_E);
    let m = flow(&app);
    assert_eq!(m.given_kind(), Some(Given::Add));
    assert_eq!(m.add_at(), b'd', "the next index none of the three has");
    // An index typed in is refused.
    c32(&mut app, C32::Key(OskPress::Char('c')));
    assert!(flow(&app).add_at_taken());
    c32(&mut app, C32::Next);
    assert_eq!(flow(&app).open(), Some(Card::Scheme));
    assert!(flow(&app).note().is_some());
    c32(&mut app, C32::Key(OskPress::Char('f')));
    let _ = app.frame();
    c32(&mut app, C32::Next);
    assert_eq!(flow(&app).open(), Some(Card::Sheets));
    assert_eq!(strings(&app), [CASH_F]);
    past_strings(&mut app);
    assert!(app.session.keys.is_empty(), "nothing loads without Lock in");
    let _ = CASH_D;
}

/// §9.4: a key loaded from strings keeps its set's facts: k, the
/// identifier, the length and the indices typed; how many strings there
/// are is the plan's question, so none more is counted.
#[test]
fn a_key_loaded_from_strings_keeps_its_sets_facts() {
    let mut app = testkit::started();
    adding(&mut app);
    add(&mut app, SHARE_A);
    add(&mut app, SHARE_C);
    app.event(Event::Key(Key::Enter));
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    let set = app.session.keys[0]
        .codex32
        .clone()
        .expect("the set's facts");
    assert_eq!(set.threshold, 2);
    assert_eq!(&set.identifier, b"name");
    assert_eq!(set.length, 48);
    assert_eq!(set.indices(), b"ac");
    assert_eq!(set.unknown, 0);
}

/// §6.3, §5.3: Make codex32 shares' Entropy card goes on with shares
/// made by hand: they are typed on the codex32 form, Make the rest on and
/// the strings in all the Scheme card's; the Start card names a key the
/// person has.
#[test]
fn shares_made_by_hand_go_on_into_make_the_rest() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    app.press(Action::KStep(kstep::START));
    let _ = app.frame();
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let other = SPLITS.iter().position(|s| *s == Split::Other).unwrap();
    c32(&mut app, C32::Split(other as u8));
    c32(&mut app, C32::N(1));
    c32(&mut app, C32::N(1));
    c32(&mut app, C32::Next);
    for ch in "name".chars() {
        c32(&mut app, C32::Key(OskPress::Char(ch)));
    }
    c32(&mut app, C32::Next);
    let hand = WAYS.iter().position(|w| *w == Way::Shares).unwrap();
    c32(&mut app, C32::Way(hand as u8));
    c32(&mut app, C32::Next);
    let k = app.keygen().unwrap();
    assert_eq!(k.open, Some(kstep::TYPED), "the codex32 form");
    assert!(k.codex32.is_none());
    assert_eq!(testkit::entry(&app).form, Form::Codex32);
    add(&mut app, SHARE_A);
    add(&mut app, SHARE_C);
    let m = flow(&app);
    assert_eq!(m.given_kind(), Some(Given::Rest));
    assert_eq!(m.k_n(), (2, 5), "the strings in all the Scheme card chose");
    assert!(crate::common::has_part(&mut app, "Add a key you have"));
    c32(&mut app, C32::Next);
    assert_eq!(flow(&app).open(), Some(Card::Sheets));
    assert_eq!(strings(&app).len(), 3, "D, E and F");
    assert_eq!(strings(&app)[0], SHARE_D);
}

/// §6, §8.3: Tools' codex32 tile opens the codex32 form, from which Make
/// the rest goes on.
#[test]
fn tools_codex32_tile_reaches_make_the_rest() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Tools));
    let _ = app.frame();
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "codex32")
        .expect("the codex32 tile");
    app.press(Action::Catalog(tile as u8));
    assert_eq!(testkit::entry(&app).form, Form::Codex32);
    add(&mut app, SHARE_A);
    assert!(crate::common::offered(
        &mut app,
        Action::Codex32Form(C32Press::Rest)
    ));
    assert!(crate::common::offered(
        &mut app,
        Action::Codex32Form(C32Press::Check)
    ));
    app.press(Action::Codex32Form(C32Press::Rest));
    add(&mut app, SHARE_C);
    assert_eq!(flow(&app).given_kind(), Some(Given::Rest));
}

/// §8.3: Check a string shows its worksheet's final row, SECRETSHARE32
/// or where it differs, and the whole sheet on Show the worksheet;
/// nothing loads.
#[test]
fn check_a_string_shows_its_final_row_and_its_worksheet() {
    let mut app = testkit::started();
    adding(&mut app);
    app.press(Action::Codex32Form(C32Press::Check));
    add(&mut app, SHARE_A);
    let sheet = testkit::entry(&app)
        .codex32
        .sheet
        .as_ref()
        .expect("its worksheet");
    let rows = sheet.worksheet(faraday_keys::Reveal::PaperForm).unwrap();
    assert!(rows.holds());
    // Its final row, `=` and the booklet's capitals.
    let want: Vec<String> = std::iter::once("=".to_string())
        .chain("SECRETSHARE32".chars().map(String::from))
        .collect();
    let texts = app.drawn_texts();
    assert!(
        texts.windows(want.len()).any(|w| w == want.as_slice()),
        "{texts:?}"
    );
    assert!(crate::common::offered(
        &mut app,
        Action::Codex32Form(C32Press::Sheet)
    ));
    // One box changed: the final row differs, and says where.
    for _ in 0..SHARE_A.len() {
        app.event(Event::Key(Key::Backspace));
    }
    let mut bad = SHARE_A.as_bytes().to_vec();
    let at = BECH32.iter().position(|&c| c == bad[19]).unwrap();
    bad[19] = BECH32[(at + 5) % 32];
    add(&mut app, std::str::from_utf8(&bad).unwrap());
    assert!(!testkit::entry(&app).codex32.checked.clone().unwrap().holds);
    assert!(crate::common::has_part(
        &mut app,
        "Final row differs at box"
    ));
    app.press(Action::Codex32Form(C32Press::Sheet));
    assert!(testkit::entry(&app).codex32.sheet_open);
    let _ = app.frame();
    assert!(app.session.keys.is_empty());
}

/// §8.4: a key of strings that went in by a correction says so beside
/// its fingerprint on Wallets.
#[test]
fn a_corrected_keys_line_is_on_wallets() {
    let mut app = testkit::started();
    adding(&mut app);
    app.press(Action::Codex32Correct(Press::Tick));
    let mut b = SHARE_A.as_bytes().to_vec();
    let at = BECH32.iter().position(|&c| c == b[19]).unwrap();
    b[19] = BECH32[(at + 7) % 32];
    add(&mut app, std::str::from_utf8(&b).unwrap());
    assert!(testkit::entry(&app).correction.is_open());
    app.press(Action::Codex32Correct(Press::Mistyped));
    app.event(Event::Key(Key::Char(char::from(SHARE_A.as_bytes()[19]))));
    add(&mut app, SHARE_C);
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.key_marks.of(NAME_FP), Some(Mark::Corrected));
    app.press(Action::Nav(Screen::Wallets));
    assert!(crate::common::has_part(&mut app, "Share entry corrected"));
}

/// The demo's public set with its codex32 sample, BIP 93's vector 2, as
/// `faraday-samples` lists it (`docs/MODES.md` §7).
pub(crate) fn samples() -> &'static Samples {
    static SAMPLES: std::sync::OnceLock<Samples> = std::sync::OnceLock::new();
    SAMPLES.get_or_init(|| {
        let words = testkit::demo_samples();
        let mut keys = words.keys.to_vec();
        keys.push(SampleKey {
            name: "sample_codex32_shares",
            word: "",
            count: 0,
            form: SampleForm::Codex32,
            size: SampleSize::Shares(2),
            listed: true,
            parts: &[
                "MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM",
                "MS12NAMECACDEFGHJKLMNPQRSTUVWXYZ023FTR2GDZMPY6PN",
            ],
            combined: "",
            plain: Some(NAME_FP),
            with_passphrase: None,
            node: None,
        });
        Samples {
            keys: Box::leak(keys.into_boxed_slice()),
            wallets: words.wallets,
            inputs: words.inputs,
        }
    })
}

/// The app as the demo has it, with the shell's samples.
fn demo() -> Faraday {
    let mut app = testkit::started();
    app.online = true;
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    app.session.set_demo(samples());
    app.samples = Some(samples());
    // Tall enough that an open card is drawn whole.
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// Make codex32 shares open on its entry: 2 of 3 at 48 characters, the
/// identifier `d3m0`, `way` chosen.
fn entry(app: &mut Faraday, way: Way) {
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    app.press(Action::KStep(kstep::START));
    let _ = app.frame();
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(app, C32::Split(split as u8));
    c32(app, C32::Next);
    for ch in "d3m0".chars() {
        c32(app, C32::Key(OskPress::Char(ch)));
    }
    c32(app, C32::Next);
    let w = WAYS.iter().position(|w| *w == way).unwrap();
    c32(app, C32::Way(w as u8));
    c32(app, C32::Next);
    assert_eq!(
        flow(app).open(),
        Some(Card::Enter),
        "{:?}",
        flow(app).note()
    );
}

/// §12: in Practice the person rolls and watches the sheets; once the entry
/// is done the sample stands in place of the strings made, every later
/// card says so, no dice of theirs is read back, and Lock in loads the
/// sample's key. Use a sample fills the characters left.
#[test]
fn in_the_demo_the_sample_stands_in_place_of_the_strings_made() {
    let mut app = demo();
    entry(&mut app, Way::Dice);
    // One character rolled, the rest from Use a sample.
    for f in [4, 3, 4, 3, 4, 3, 4, 3, 4, 3] {
        c32(&mut app, C32::Key(OskPress::Char(char::from(b'0' + f))));
    }
    assert!(
        crate::common::strip(&mut app)
            .iter()
            .any(|t| t == "Use a sample")
    );
    c32(&mut app, C32::Sample);
    let own = strings(&app);
    assert_eq!(own.len(), 3, "the entry finished");
    assert!(own.iter().all(|s| s.starts_with("ms12d3m0")));
    c32(&mut app, C32::Next);
    let m = flow(&app);
    assert_eq!(m.open(), Some(Card::Sheets));
    assert!(m.sample());
    assert_eq!(
        strings(&app),
        [SHARE_A, SHARE_C, SHARE_D],
        "the sample's strings"
    );
    assert!(
        m.tabs().iter().all(|t| !t.0.contains("Dice")),
        "no dice of the person's is read back"
    );
    assert!(
        crate::common::strip(&mut app)
            .iter()
            .any(|t| t == "Practice puts sample NAME in place of these strings")
    );
    past_strings(&mut app);
    assert!(
        crate::common::strip(&mut app)
            .iter()
            .any(|t| t == "Practice puts sample NAME in place of these strings")
    );
    c32(&mut app, C32::Lock);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].fingerprint.0, NAME_FP);
}

/// §12: in Practice, Make the rest takes the published strings and refuses a
/// set that is not public, as any key is refused.
#[test]
fn in_the_demo_make_the_rest_takes_only_a_published_set() {
    let secret = Codex32::from_seed(0, *b"w0za", &[7u8; 16]).unwrap();
    let set = secret.split(2, 3, &[3u8; 64]).unwrap();
    let mut app = demo();
    adding(&mut app);
    add(&mut app, set[0].encode().as_str());
    app.press(Action::Codex32Form(C32Press::Rest));
    add(&mut app, set[1].encode().as_str());
    assert!(app.keygen().is_none(), "a set not public is refused");
    assert!(testkit::entry(&app).error.is_some());

    let mut app = demo();
    adding(&mut app);
    add(&mut app, SHARE_A);
    app.press(Action::Codex32Form(C32Press::Rest));
    add(&mut app, SHARE_C);
    c32(&mut app, C32::Next);
    assert_eq!(strings(&app), [SHARE_D]);
}
