//! The form that makes a Krux encrypted code (`docs/KEF.md` §15.3): each
//! Option changes the code as it says, a refused version names one that
//! works, the passphrase field refuses what §9 refuses, the dice fill the
//! passphrase and Cancel stops the work.

use faraday_codec::kef::base43_decode;
use faraday_core::dice_entry::Press;
use faraday_core::kef_form::{IdKind, KefAct, KefForm, Source, Stage};
use faraday_core::{Faraday, testkit};
use faraday_keys::kef::{Envelope, unwrap};
use faraday_shell_api::{App, EntropyBytes, Event, Key};
use zeroize::Zeroizing;

const TWENTY_FOUR: &str = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";

fn app_with_seed() -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    let c = testkit::compute();
    app.session
        .keys
        .add_words_with(&c, TWENTY_FOUR, "", "seed", None)
        .unwrap();
    app
}

fn seed_form() -> KefForm {
    KefForm::new(
        Source::Seed {
            index: 0,
            bytes: 32,
            fingerprint: Some([1, 2, 3, 4]),
        },
        false,
    )
}

fn type_in(form: &mut KefForm, app: &mut Faraday, text: &str) {
    for c in text.chars() {
        form.key(app, Key::Char(c));
    }
}

fn rounds(form: &mut KefForm, app: &mut Faraday, n: &str) {
    form.act(app, KefAct::Options);
    form.act(app, KefAct::Rounds);
    for _ in 0..8 {
        form.key(app, Key::Backspace);
    }
    type_in(form, app, n);
    form.act(app, KefAct::Back);
}

/// Makes the code and returns its envelope.
fn make(form: &mut KefForm, app: &mut Faraday) -> Envelope {
    form.act(app, KefAct::Make);
    assert_eq!(form.stage(), Stage::Making, "{:?}", form.error());
    testkit::settle(app);
    assert!(form.poll(app));
    assert_eq!(form.stage(), Stage::Made, "{:?}", form.error());
    let code = form.made().unwrap();
    let view = code.view.as_ref().expect("one code");
    let bytes = base43_decode(view.whole.as_deref().unwrap()).unwrap();
    assert_eq!(bytes, code.bytes);
    unwrap(&bytes).unwrap()
}

#[test]
fn make_waits_until_the_passphrase_is_typed_twice_alike() {
    let mut app = app_with_seed();
    let mut form = seed_form();
    form.act(&mut app, KefAct::Field);
    type_in(&mut form, &mut app, "a passphrase");
    form.act(&mut app, KefAct::FieldAgain);
    type_in(&mut form, &mut app, "a passphrasf");
    form.act(&mut app, KefAct::Make);
    assert_eq!(
        form.stage(),
        Stage::Entry,
        "differing passphrases make nothing"
    );
    form.key(&mut app, Key::Backspace);
    type_in(&mut form, &mut app, "e");
    form.act(&mut app, KefAct::Make);
    assert_eq!(form.stage(), Stage::Making, "{:?}", form.error());
}

#[test]
fn each_option_changes_the_code_as_it_says() {
    let mut app = app_with_seed();
    let go = |app: &mut Faraday, set: &dyn Fn(&mut KefForm, &mut Faraday)| {
        let mut form = seed_form();
        form.act(app, KefAct::Field);
        type_in(&mut form, app, "a passphrase");
        form.act(app, KefAct::FieldAgain);
        type_in(&mut form, app, "a passphrase");
        rounds(&mut form, app, "10000");
        set(&mut form, app);
        make(&mut form, app)
    };
    let e = go(&mut app, &|f, a| f.act(a, KefAct::Version(15)));
    assert_eq!((e.version, e.count), (15, 10_000));
    let e = go(&mut app, &|f, a| f.act(a, KefAct::Id(IdKind::Random16)));
    assert_eq!(e.id.len(), 16);
    let e = go(&mut app, &|f, a| {
        f.act(a, KefAct::Id(IdKind::Label));
        type_in(f, a, "my label");
    });
    assert_eq!(e.id, b"my label");
    let e = go(&mut app, &|f, a| f.act(a, KefAct::Id(IdKind::Fingerprint)));
    assert_eq!(e.id, [1, 2, 3, 4]);
    let e = go(&mut app, &|f, a| rounds(f, a, "25000"));
    assert_eq!(e.count, 25_000);
}

#[test]
fn the_options_line_reads_the_choices_and_the_side_on_paper() {
    let mut app = app_with_seed();
    let mut form = KefForm::new(
        Source::Seed {
            index: 0,
            bytes: 32,
            fingerprint: None,
        },
        true,
    );
    assert_eq!(
        form.summary(),
        "Version 20 · 100,000 rounds · random id · 33 × 33"
    );
    form.act(&mut app, KefAct::Id(IdKind::Random16));
    assert!(form.summary().contains("16-byte random id"));
    form.act(&mut app, KefAct::Version(21));
    assert_eq!(form.side(), None);
}

#[test]
fn a_refused_version_names_one_that_works() {
    let mut app = app_with_seed();
    // Two equal blocks, which ECB would show.
    let mut form = KefForm::new(Source::Text(Zeroizing::new("a".repeat(32))), false);
    type_in(&mut form, &mut app, "a passphrase");
    form.act(&mut app, KefAct::FieldAgain);
    type_in(&mut form, &mut app, "a passphrase");
    rounds(&mut form, &mut app, "10000");
    form.act(&mut app, KefAct::Version(5));
    form.act(&mut app, KefAct::Make);
    testkit::settle(&mut app);
    assert!(form.poll(&mut app));
    assert_eq!(form.stage(), Stage::Entry);
    let line = form.error().expect("a refusal").to_string();
    let version: u8 = line
        .split(|c: char| !c.is_ascii_digit())
        .find_map(|n| n.parse().ok())
        .expect("a version in the line");
    assert_ne!(version, 5);
    form.act(&mut app, KefAct::Version(version));
    assert_eq!(make(&mut form, &mut app).version, version);
}

#[test]
fn the_passphrase_field_refuses_what_the_rule_refuses() {
    let mut app = app_with_seed();
    let mut form = seed_form();
    type_in(&mut form, &mut app, "caf\u{e9} \u{1f600}\tx");
    assert_eq!(form.passphrase(), "caf x");
    type_in(&mut form, &mut app, &"y".repeat(300));
    assert_eq!(form.passphrase().len(), 200);
    // An empty passphrase is refused when the code is asked for.
    let mut empty = seed_form();
    empty.act(&mut app, KefAct::Make);
    assert_eq!(empty.stage(), Stage::Entry);
    assert!(empty.error().is_some());
}

#[test]
fn dice_fill_the_passphrase() {
    let mut app = app_with_seed();
    let mut form = seed_form();
    form.act(&mut app, KefAct::Dice);
    for f in [1, 2, 3, 4, 5] {
        form.dice_press(&mut app, Press::Face(f));
    }
    form.dice_press(&mut app, Press::Use);
    assert!(!form.passphrase().is_empty());
    assert!(faraday_keys::kef::writable_passphrase(form.passphrase()));
}

#[test]
fn cancel_stops_the_work() {
    let mut app = app_with_seed();
    let mut form = seed_form();
    type_in(&mut form, &mut app, "a passphrase");
    form.act(&mut app, KefAct::FieldAgain);
    type_in(&mut form, &mut app, "a passphrase");
    rounds(&mut form, &mut app, "500000");
    form.act(&mut app, KefAct::Make);
    assert_eq!(form.stage(), Stage::Making);
    form.cancel(&mut app);
    assert_eq!(form.stage(), Stage::Entry);
    assert!(app.busy().is_none());
    assert!(form.made().is_none());
    assert!(app.kef_sealed().is_none());
    // The form makes a code afterwards.
    rounds(&mut form, &mut app, "10000");
    assert_eq!(make(&mut form, &mut app).version, 20);
}

/// A form that has made its code, with the passphrase `pass`.
fn made_form(app: &mut Faraday, pass: &str) -> KefForm {
    let mut form = seed_form();
    form.act(app, KefAct::Field);
    type_in(&mut form, app, pass);
    form.act(app, KefAct::FieldAgain);
    type_in(&mut form, app, pass);
    rounds(&mut form, app, "10000");
    make(&mut form, app);
    form
}

fn compare(form: &mut KefForm, app: &mut Faraday, pass: &str) {
    form.act(app, KefAct::Check);
    assert_eq!(form.stage(), Stage::Check);
    type_in(form, app, pass);
    form.act(app, KefAct::Compare);
    assert_eq!(form.stage(), Stage::Comparing);
    testkit::settle(app);
    assert!(form.poll(app));
}

#[test]
fn a_right_passphrase_reads_checked() {
    let mut app = app_with_seed();
    let mut form = made_form(&mut app, "a passphrase");
    assert!(!form.checked());
    compare(&mut form, &mut app, "a passphrase");
    assert!(form.checked());
    assert_eq!(form.stage(), Stage::Made);
}

#[test]
fn a_wrong_passphrase_reads_not_matching_and_can_be_tried_again() {
    let mut app = app_with_seed();
    let mut form = made_form(&mut app, "a passphrase");
    compare(&mut form, &mut app, "another one");
    assert!(!form.checked());
    assert_eq!(form.stage(), Stage::Check);
    assert!(form.error().is_some_and(|e| e.starts_with("Not matching")));
    for _ in 0..11 {
        form.key(&mut app, Key::Backspace);
    }
    type_in(&mut form, &mut app, "a passphrase");
    form.act(&mut app, KefAct::Compare);
    testkit::settle(&mut app);
    assert!(form.poll(&mut app));
    assert!(form.checked());
}

#[test]
fn a_typed_text_is_checked_against_the_text_the_form_holds() {
    let mut app = app_with_seed();
    let mut form = KefForm::new(Source::Text(Zeroizing::new("a note".to_string())), false);
    form.act(&mut app, KefAct::Field);
    type_in(&mut form, &mut app, "a passphrase");
    form.act(&mut app, KefAct::FieldAgain);
    type_in(&mut form, &mut app, "a passphrase");
    rounds(&mut form, &mut app, "10000");
    make(&mut form, &mut app);
    compare(&mut form, &mut app, "a passphrase");
    assert!(form.checked());
}

#[test]
fn make_another_starts_over_with_the_field_empty() {
    let mut app = app_with_seed();
    let mut form = made_form(&mut app, "a passphrase");
    form.act(&mut app, KefAct::Another);
    assert_eq!(form.stage(), Stage::Entry);
    assert_eq!(form.passphrase(), "");
    assert!(form.made().is_none());
    assert!(!form.checked());
}

#[test]
fn cancel_during_compare_stops_it() {
    let mut app = app_with_seed();
    let mut form = made_form(&mut app, "a passphrase");
    form.act(&mut app, KefAct::Check);
    type_in(&mut form, &mut app, "a passphrase");
    form.act(&mut app, KefAct::Compare);
    form.cancel(&mut app);
    testkit::settle(&mut app);
    assert!(!form.poll(&mut app));
    assert_eq!(form.stage(), Stage::Check);
    assert!(!form.checked());
}

fn seed_form_of(bytes: usize) -> KefForm {
    KefForm::new(
        Source::Seed {
            index: 0,
            bytes,
            fingerprint: None,
        },
        false,
    )
}

fn sides(form: &KefForm) -> Vec<usize> {
    form.sizes().iter().map(|(s, _)| *s).collect()
}

#[test]
fn options_offers_the_sizes_the_contents_can_take() {
    // `docs/KEF.md` §5's table: 12 words fill 25, 29 and 33 (a 16-byte id
    // under version 11 also reaches 37); 24 words 29, 33 and 37.
    assert_eq!(sides(&seed_form_of(16)), [25, 29, 33, 37]);
    assert_eq!(sides(&seed_form_of(32)), [29, 33, 37]);
    let long = KefForm::new(Source::Text(Zeroizing::new("x".repeat(300))), false);
    assert!(sides(&long).is_empty());
}

#[test]
fn the_options_line_and_size_follow_the_choice_off_paper_too() {
    let mut app = app_with_seed();
    let mut form = seed_form_of(16);
    assert!(form.summary().ends_with("· 33 × 33"));
    form.act(&mut app, KefAct::Size(25));
    assert_eq!(form.side(), Some(25));
    assert!(form.summary().ends_with("· 25 × 25"));
    form.act(&mut app, KefAct::Version(21));
    assert_eq!(form.side(), None);
}

#[test]
fn picking_a_size_makes_a_code_of_that_size() {
    let mut app = app_with_seed();
    // A text of 16 distinct bytes, as long as a 12-word seed's entropy.
    let mut form = KefForm::new(
        Source::Text(Zeroizing::new("0123456789abcdef".into())),
        false,
    );
    form.act(&mut app, KefAct::Field);
    type_in(&mut form, &mut app, "a passphrase");
    form.act(&mut app, KefAct::FieldAgain);
    type_in(&mut form, &mut app, "a passphrase");
    rounds(&mut form, &mut app, "10000");
    form.act(&mut app, KefAct::Size(25));
    assert_eq!(form.error(), None);
    make(&mut form, &mut app);
    let v = form.made().unwrap().view.as_ref().unwrap();
    assert_eq!(v.frames[0].size(), 25);
}

#[test]
fn the_sizes_offered_follow_a_typed_label() {
    let mut app = app_with_seed();
    let mut form = seed_form_of(32);
    let before = sides(&form);
    form.act(&mut app, KefAct::Options);
    form.act(&mut app, KefAct::Id(IdKind::Label));
    type_in(&mut form, &mut app, &"x".repeat(30));
    let after = sides(&form);
    assert_ne!(after, before);
    for side in after {
        form.act(&mut app, KefAct::Size(side));
        assert_eq!(form.side(), Some(side));
    }
}
