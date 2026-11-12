//! Making a Krux encrypted code (`docs/KEF.md` §4.1, §5, §10, §15): a
//! seed's code made through the form opens to its entropy, each option
//! changes the code as it says, a refusal names a version that works, and
//! the label says nothing of the contents.

use faraday_codec::kef::{base43_decode, spec_of};
use faraday_core::worker::kef_make::{IdChoice, MakeStart, Options};
use faraday_core::{Faraday, testkit};
use faraday_keys::kef::{OpenError, unwrap};
use faraday_shell_api::{App, EntropyBytes, Event};
use zeroize::Zeroizing;

const TWENTY_FOUR: &str = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";
const FIFTEEN: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon address";
const PASS: &[u8] = b"a passphrase";

fn pass() -> Zeroizing<Vec<u8>> {
    Zeroizing::new(PASS.to_vec())
}

fn quick() -> Options {
    Options {
        count: 10_000,
        ..Options::default()
    }
}

/// An app holding `words` as its first seed, with fresh bytes to spare.
fn app_with(words: &str) -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    let c = testkit::compute();
    app.session
        .keys
        .add_words_with(&c, words, "", "seed", None)
        .unwrap();
    app
}

fn made(app: &mut Faraday, options: &Options) -> faraday_core::worker::kef_make::Made {
    testkit::settle(app);
    app.kef_take_made(options)
        .expect("an answer")
        .expect("a code")
}

#[test]
fn a_seeds_code_made_through_the_form_opens_to_its_entropy() {
    let mut app = app_with(TWENTY_FOUR);
    let c = testkit::compute();
    let entropy = app
        .session
        .keys
        .kef_entropy(&c, 0, faraday_keys::Reveal::KefCode)
        .unwrap()
        .release();
    let options = quick();
    app.kef_make_seed(0, pass(), &options).unwrap();
    let code = made(&mut app, &options);
    // What the sheet shows is the code's base43 text, as Krux reads it.
    let view = code.view.as_ref().expect("one code");
    let bytes = base43_decode(view.whole.as_deref().unwrap()).unwrap();
    assert_eq!(bytes, *code.bytes);
    let envelope = unwrap(&bytes).unwrap();
    assert_eq!(envelope.version, 20);
    assert_eq!(envelope.id.len(), 8);
    app.kef_open(pass(), envelope.clone(), "Working");
    testkit::settle(&mut app);
    assert_eq!(
        &app.kef_take_opened().unwrap().unwrap().plaintext[..],
        &entropy[..]
    );
    app.kef_open(Zeroizing::new(b"another".to_vec()), envelope, "Working");
    testkit::settle(&mut app);
    assert_eq!(
        app.kef_take_opened().unwrap().err(),
        Some(OpenError::Failed)
    );
}

#[test]
fn a_seed_of_15_words_is_not_offered_a_code() {
    let mut app = app_with(FIFTEEN);
    assert_eq!(
        app.kef_make_seed(0, pass(), &quick()),
        Err(MakeStart::NotOffered)
    );
}

#[test]
fn each_option_changes_the_code_as_it_says() {
    let mut app = app_with(TWENTY_FOUR);
    let mut make = |o: Options| {
        app.kef_make_seed(0, pass(), &o).unwrap();
        let m = made(&mut app, &o);
        let e = unwrap(&m.bytes).unwrap();
        (m, e)
    };
    let (_, e) = make(Options {
        version: 15,
        ..quick()
    });
    assert_eq!(e.version, 15);
    let (_, e) = make(Options {
        id: IdChoice::Random(16),
        ..quick()
    });
    assert_eq!(e.id.len(), 16);
    let (m, e) = make(Options {
        id: IdChoice::Label("my label".into()),
        ..quick()
    });
    assert_eq!(e.id, b"my label");
    assert!(m.view.unwrap().label.iter().any(|l| l.contains("my label")));
    // A public id adds Krux's change, under 10% of the count, to it.
    assert!(e.count >= 10_000 && e.count < 11_000, "{}", e.count);
    let (_, e) = make(Options {
        id: IdChoice::Fingerprint([1, 2, 3, 4]),
        ..quick()
    });
    assert_eq!(e.id, [1, 2, 3, 4]);
    let (_, e) = make(Options {
        count: 25_000,
        ..quick()
    });
    assert_eq!(e.count, 25_000);
    // Low error correction gives a smaller code for version 5.
    let medium = Options {
        version: 5,
        ..quick()
    };
    let low = Options {
        low: true,
        ..medium.clone()
    };
    let note = "0123456789abcdefghijklmnopqrstuv";
    let mut size = |o: Options| {
        app.kef_make_text(pass(), Zeroizing::new(note.into()), &o)
            .unwrap();
        made(&mut app, &o).view.unwrap().frames[0].size()
    };
    let size_medium = size(medium);
    let size_low = size(low);
    assert!(size_low < size_medium, "{size_low} {size_medium}");
}

#[test]
fn a_refused_version_names_one_that_works() {
    let mut app = app_with(TWENTY_FOUR);
    // 40 equal bytes: two equal blocks, which ECB would show.
    let options = Options {
        version: 5,
        ..quick()
    };
    app.kef_make_text(pass(), Zeroizing::new("a".repeat(40)), &options)
        .unwrap();
    testkit::settle(&mut app);
    let refused = app.kef_take_made(&options).unwrap().err().expect("refused");
    let named: u8 = refused
        .split_whitespace()
        .find_map(|w| w.trim_end_matches('.').parse().ok())
        .expect("a version is named");
    assert!(spec_of(named).is_some());
    // The version named makes the code.
    let again = Options {
        version: named,
        ..quick()
    };
    app.kef_make_text(pass(), Zeroizing::new("a".repeat(40)), &again)
        .unwrap();
    assert!(made(&mut app, &again).view.is_ok());
}

#[test]
fn a_passphrase_the_keypads_lack_is_refused_before_any_work() {
    let mut app = app_with(TWENTY_FOUR);
    let started = app.kef_make_seed(0, Zeroizing::new("caf\u{e9}".as_bytes().to_vec()), &quick());
    assert!(matches!(started, Err(MakeStart::Refused(_))));
    assert!(app.busy().is_none());
}

#[test]
fn the_label_and_the_files_say_nothing_of_the_contents() {
    let mut app = app_with(TWENTY_FOUR);
    let options = quick();
    app.kef_make_text(pass(), Zeroizing::new("hunter two".into()), &options)
        .unwrap();
    let code = made(&mut app, &options);
    let view = code.view.as_ref().unwrap();
    let said = format!("{} {} {:?}", view.title, view.subtitle, view.label).to_lowercase();
    for word in [
        "seed",
        "words",
        "passphrase",
        "share",
        "note",
        "wallet",
        "hunter",
    ] {
        assert!(!said.contains(word), "{word} in {said}");
    }
    for name in [&code.png_name, &code.file_name] {
        for word in ["seed", "words", "passphrase", "hunter"] {
            assert!(!name.contains(word), "{word} in {name}");
        }
    }
}

#[test]
fn the_picture_and_the_file_go_to_the_outbox() {
    let mut app = app_with(TWENTY_FOUR);
    let options = quick();
    app.kef_make_seed(0, pass(), &options).unwrap();
    let code = made(&mut app, &options);
    app.kef_save_made(&code);
    let names: Vec<_> = app.outbox.iter().map(|i| i.name.clone()).collect();
    assert!(names.contains(&code.png_name), "{names:?}");
    let file = app
        .outbox
        .iter()
        .find(|i| i.name == code.file_name)
        .unwrap();
    assert_eq!(*file.bytes, *code.bytes);
}

#[test]
fn a_second_code_with_the_same_label_keeps_the_first_in_the_outbox() {
    let mut app = app_with(TWENTY_FOUR);
    let options = Options {
        id: IdChoice::Label("cold".into()),
        ..quick()
    };
    app.kef_make_seed(0, pass(), &options).unwrap();
    let first = made(&mut app, &options);
    let first_name = app.kef_save_made(&first);
    app.kef_make_seed(0, pass(), &options).unwrap();
    let second = made(&mut app, &options);
    let second_name = app.kef_save_made(&second);
    assert_ne!(first_name, second_name);
    for code in [&first, &second] {
        assert!(
            app.outbox.iter().any(|i| *i.bytes == *code.bytes),
            "a code's file is missing from the Outbox"
        );
    }
}

#[test]
fn the_size_a_code_will_be_is_known_before_it_is_made() {
    use faraday_core::worker::kef_make::side;
    let at = |version, low| Options {
        version,
        low,
        ..Options::default()
    };
    assert_eq!(side(&at(20, false), 32), Some(33));
    assert_eq!(side(&at(20, false), 16), Some(33));
    assert_eq!(side(&at(5, false), 16), Some(29));
    assert_eq!(side(&at(5, false), 32), Some(33));
    assert_eq!(side(&at(5, true), 16), Some(25));
    assert_eq!(side(&at(5, true), 32), Some(29));
    let long_id = Options {
        id: IdChoice::Random(16),
        ..Options::default()
    };
    assert_eq!(side(&long_id, 32), Some(37));
}

#[test]
fn the_size_predicted_is_the_size_of_the_code_made() {
    use faraday_core::worker::kef_make::{MODES, side, versions_in};
    let mut app = app_with(TWENTY_FOUR);
    let mut cases = vec![(10u8, false, 16usize, 49usize)];
    for mode in MODES {
        for v in versions_in(mode) {
            for id in [8, 16] {
                for len in [1, 16, 32, 49, 120] {
                    cases.push((v, false, id, len));
                    cases.push((v, true, id, len));
                }
            }
        }
    }
    for (version, low, id, len) in cases {
        let options = Options {
            version,
            low,
            count: 10_000,
            id: IdChoice::Random(id),
        };
        let Some(predicted) = side(&options, len) else {
            continue;
        };
        // No two blocks alike, which the ECB versions refuse.
        let counted: String = (0..len / 8 + 1).map(|i| format!("{i:08}")).collect();
        let text = Zeroizing::new(counted[..len].to_string());
        for i in 0..2u8 {
            app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
        }
        // A text some version refuses (ECB takes whole blocks) makes no code.
        if let Err(e) = app.kef_make_text(pass(), text, &options) {
            assert_ne!((version, low, id, len), (10, false, 16, 49), "{e:?}");
            continue;
        }
        let code = made(&mut app, &options);
        let view = code.view.as_ref().expect("one code");
        assert_eq!(
            view.frames[0].size(),
            predicted,
            "version {version}, low {low}, id {id}, text {len}"
        );
    }
}
