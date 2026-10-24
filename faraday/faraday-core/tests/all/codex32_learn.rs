//! Codex32 in Learn and in the demo (`docs/CODEX32.md` §12, §13.3):
//! Make codex32 shares and the codex32 form name the codex32 page first,
//! and in the demo the codex32 form takes BIP 93's published set,
//! refuses a set that is not public and checks any string.

use faraday_bip::codex32::Codex32;
use faraday_bip::keys::Network;
use faraday_core::forms::{C32Press, Form};
use faraday_core::keygen::Start;
use faraday_core::samples::{SampleForm, SampleKey, SampleSize, Samples};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, Event, Key};

/// The Learn page codex32 is on.
const CODEX32_PAGE: &str = "Backups in other forms";

/// BIP 93's vector 2, two shares of one set: the demo's public set.
const SHARE_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const SHARE_C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";

fn first_page(app: &Faraday) -> Option<&'static str> {
    app.learn_pages().first().map(|p| p.title)
}

/// Add a key with the codex32 form open.
fn codex32_form(app: &mut Faraday) {
    let i = Form::ALL.iter().position(|f| *f == Form::Codex32).unwrap();
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(i as u8));
}

fn add(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Enter));
}

fn clear(app: &mut Faraday, n: usize) {
    for _ in 0..n {
        app.event(Event::Key(Key::Backspace));
    }
}

/// Two shares of a set no one has published, at vector 2's header.
fn unpublished_set() -> [String; 2] {
    let secret = Codex32::from_seed(0, *b"name", &[7u8; 16]).unwrap();
    let shares = secret.split(2, 3, &[3u8; 64]).unwrap();
    let at = |i: u8| {
        shares
            .iter()
            .find(|s| s.share_index() == i)
            .expect("a share at that index")
            .encode()
            .as_str()
            .to_string()
    };
    [at(b'a'), at(b'c')]
}

/// The demo's public set with its codex32 sample, BIP 93's vector 2, as
/// `faraday-samples` lists it (`docs/MODES.md` §7), beside the words of
/// core's tests.
fn samples() -> &'static Samples {
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
            parts: &[SHARE_A, SHARE_C],
            combined: "",
            // The fingerprint of vector 2's xprv.
            plain: Some([0xfa, 0xb6, 0x86, 0x8a]),
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

/// The app as the demo has it: online, on testnet, the public seeds set.
fn demo() -> Faraday {
    let mut app = testkit::started();
    app.online = true;
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    app.session.set_demo(samples());
    app
}

#[test]
fn make_codex32_shares_and_the_codex32_form_open_learn_on_the_codex32_page() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    assert_ne!(first_page(&app), Some(CODEX32_PAGE), "BIP-39 words first");
    app.press(Action::KStep(faraday_core::keygen::kstep::START));
    let _ = app.frame();
    app.press(Action::KStart(Start::MakeCodex32.index()));
    assert_eq!(first_page(&app), Some(CODEX32_PAGE));

    let mut app = testkit::started();
    codex32_form(&mut app);
    assert_eq!(first_page(&app), Some(CODEX32_PAGE));
    // The page says where the codex32 guide and the booklet are.
    let page = app.learn_pages()[0];
    let text: String = page
        .sections
        .iter()
        .flat_map(|s| s.paragraphs.iter().copied())
        .collect();
    assert!(text.contains("https://github.com/FractalEncrypt/FractalEncrypt_seedsigner/blob/codex/codex32-error-correction/docs/codex32_qr/Codex32_User_Flow_Guide.md"));
    assert!(text.contains("https://secretcodex32.com/"));
    assert!(text.contains("Copyright (c) 2022 Blockstream"));
}

#[test]
fn in_the_demo_the_codex32_form_takes_the_published_set_only() {
    let mut app = demo();
    codex32_form(&mut app);
    let [a, c] = unpublished_set();
    add(&mut app, &a);
    add(&mut app, &c);
    app.event(Event::Key(Key::Enter));
    assert!(
        app.session.keys.is_empty(),
        "a set not published is refused"
    );

    let mut app = demo();
    codex32_form(&mut app);
    add(&mut app, SHARE_A);
    add(&mut app, SHARE_C);
    app.event(Event::Key(Key::Enter));
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
}

#[test]
fn in_the_demo_check_a_string_answers_for_any_string_and_loads_nothing() {
    let mut app = demo();
    codex32_form(&mut app);
    app.press(Action::Codex32Form(C32Press::Check));
    let [a, _] = unpublished_set();
    add(&mut app, &a);
    let checked = testkit::entry(&app).codex32.checked.clone();
    assert!(checked.expect("checked").holds);
    clear(&mut app, a.len());
    add(&mut app, SHARE_C);
    assert!(
        testkit::entry(&app)
            .codex32
            .checked
            .clone()
            .expect("checked")
            .holds
    );
    assert!(app.session.keys.is_empty());
}
