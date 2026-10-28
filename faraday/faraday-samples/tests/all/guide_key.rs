//! The user guide's key (`docs/guide/walk/new-key.md`, `docs/MODES.md`
//! §5.1) is a sample: it is the key EntropyLab's published dice transcript
//! names, Practice takes its words in Add a key, New key from the guide's
//! rolls ends with it rather than a handout, and Load samples lists it
//! first by its name. A key that is no sample is still refused.

use crate::common::{
    PASSPHRASE, demo_app, entropy_of, json, key, reference_fingerprint, strip, unhex,
};
use faraday_bip::keys::Fingerprint;
use faraday_core::keygen::{Way, kstep};
use faraday_core::samples::Pick;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_samples::SAMPLES;
use faraday_shell_api::{App, Event, Key};

/// The guide's key's fingerprint.
const GUIDE: [u8; 4] = [0x3d, 0xca, 0x68, 0x05];

/// The guide's rolls, as `docs/guide/walk/new-key.md` prints them.
const ROLLS: &str = "1234112341223412334123441234152341263412314123421234132341243412354321";

fn held(app: &Faraday, fp: [u8; 4]) -> bool {
    app.session.keys.position(Fingerprint(fp)).is_some()
}

/// New key at twelve words, dice naming the words, `rolls` entered, and
/// the Key card open.
fn made_from(app: &mut Faraday, rolls: &str) {
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KNext);
    app.press(Action::KWay(Way::DiceWords.index()));
    app.press(Action::KNext);
    for c in rolls.bytes() {
        app.press(Action::KRoll(c - b'0'));
    }
    app.press(Action::KNext);
    assert_eq!(app.keygen().unwrap().open, Some(kstep::KEY));
    let _ = app.frame();
}

/// Types `text` into Add a key's words and presses Add.
fn add_typed(app: &mut Faraday, text: &str) {
    app.press(Action::Entry(None));
    let _ = app.frame();
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

#[test]
fn the_guide_key_is_the_words_the_published_transcript_names() {
    let sample = key("sample_guide");
    let vector = json("entropy/dice.json")["vectors"]
        .as_array()
        .expect("vectors")
        .iter()
        .find(|v| v["procedure"] == "words")
        .cloned()
        .expect("a words vector");
    let lang = bip39::Language::English;
    let m = bip39::Mnemonic::parse_in_normalized(lang, sample.parts[0]).expect("words");
    assert_eq!(
        entropy_of(&m),
        unhex(vector["entropy"].as_str().expect("entropy"))
    );
    // The guide's rolls are the vector's, but for the two it does not take.
    let rolls = vector["rolls"].as_str().expect("rolls");
    assert!(rolls.starts_with(ROLLS));
    assert_eq!(ROLLS.len(), 70);
    assert_eq!(sample.plain, Some(GUIDE));
    assert_eq!(
        sample.plain,
        Some(reference_fingerprint(lang, sample.parts[0], ""))
    );
    assert_eq!(
        sample.with_passphrase,
        Some(reference_fingerprint(lang, sample.parts[0], PASSPHRASE))
    );
}

#[test]
fn in_practice_the_guides_words_typed_in_add_a_key_load_key_3dca6805() {
    let mut app = demo_app();
    add_typed(&mut app, key("sample_guide").parts[0]);
    assert!(held(&app, GUIDE), "{:?}", testkit::entry(&app).error);
}

#[test]
fn in_practice_a_key_that_is_no_sample_is_still_refused_in_add_a_key() {
    let mut app = demo_app();
    // The transcript's other vector: valid words, in no sample.
    add_typed(
        &mut app,
        "brand hobby ranch shoulder brass hockey ranch short brand hockey random above",
    );
    assert!(app.session.keys.is_empty());
    assert!(testkit::entry(&app).error.is_some());
}

#[test]
fn in_practice_new_key_from_the_guides_rolls_ends_with_key_3dca6805() {
    let mut app = demo_app();
    made_from(&mut app, ROLLS);
    // Faraday Demo's strip names the sample the words are; no handout.
    let lines = strip(&mut app);
    assert!(
        lines
            .iter()
            .any(|t| t.contains("These words are sample Guide key · 3dca6805")),
        "{lines:?}"
    );
    assert!(!lines.iter().any(|t| t.contains("Lock in adds sample")));
    assert!(app.offers(Action::KLock));
    app.press(Action::KLock);
    let k = app.keygen().unwrap();
    assert!(k.locked, "{:?}", k.note);
    assert_eq!(k.fingerprint, Some(GUIDE));
    assert_eq!(k.handed, None);
    assert!(held(&app, GUIDE));
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("These words are sample Guide key · 3dca6805")),
        "the Key card names the sample"
    );
}

#[test]
fn in_practice_new_key_from_other_rolls_still_ends_with_a_handout() {
    let mut app = demo_app();
    // The transcript with a last word of `abandon` rolled: no sample's words.
    made_from(
        &mut app,
        "123411234122341233412344123415234126341231412342123413234124341235111111",
    );
    app.press(Action::KLock);
    let k = app.keygen().unwrap();
    assert!(k.locked, "{:?}", k.note);
    assert_eq!(k.handed, Some(("twelve", 12)));
    let twelve = SAMPLES
        .keys
        .iter()
        .find(|s| s.word == "twelve")
        .and_then(|s| s.plain);
    assert_eq!(k.fingerprint, twelve);
    assert!(!held(&app, GUIDE));
}

#[test]
fn load_samples_lists_the_guide_key_first_by_its_name_and_loads_it() {
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app.press(Action::Samples);
    assert_eq!(app.sheet, Some(Sheet::Samples));
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Guide key · 3dca6805"),
        "the sheet names the guide's key"
    );
    let first = SAMPLES
        .keys
        .iter()
        .find(|k| k.listed && k.plain.is_some())
        .expect("a key");
    assert_eq!(first.name, "sample_guide");
    app.press(Action::SampleTick(Pick::Key(0)));
    let _ = app.frame();
    app.press(Action::SamplesLoad);
    assert!(held(&app, GUIDE));
}
