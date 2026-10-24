//! Codex32 strings typed, checked and scanned on Add a key's codex32 form
//! (`docs/CODEX32.md` §8.1, §8.3, §8.5, §10.1), and the form's error
//! correction wired in (§8.4): a hand-made set typed string by string,
//! a failing share named, the same string again and another at an index
//! already in, Check a string loading nothing, the Codex32QR profile's
//! pictures read to the key they state, and a corrected key keeping its
//! mark. On BIP 93's published strings and the Codex32QR profile's
//! vectors (`tools/vectors/codex32qr/`).

use faraday_bip::codex32::Codex32;
use faraday_core::codex32_correct::{Mark, Press};
use faraday_core::forms::{C32Press, Form};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, Event, Key};

/// BIP 93's vector 2, two shares of one set (fingerprint from its xprv).
const SHARE_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const SHARE_C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
/// BIP 93's vector 1, the secret alone.
const SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";

const BECH32: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

fn codex32_index() -> u8 {
    Form::ALL.iter().position(|f| *f == Form::Codex32).unwrap() as u8
}

fn adding() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(codex32_index()));
    app
}

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn enter(app: &mut Faraday) {
    app.event(Event::Key(Key::Enter));
}

fn add(app: &mut Faraday, text: &str) {
    typed(app, text);
    enter(app);
}

/// `s` with box `n` (from 1) changed to another bech32 character.
fn changed(s: &str, n: usize) -> String {
    let mut b = s.as_bytes().to_vec();
    let at = BECH32.iter().position(|&c| c == b[n - 1]).unwrap_or(0);
    b[n - 1] = BECH32[(at + 7) % 32];
    String::from_utf8(b).unwrap()
}

fn error(app: &Faraday) -> Option<String> {
    testkit::entry(app).error.clone()
}

/// The fingerprint of the key `strings` make, loaded on a form of its own.
fn key_of(strings: &[&str]) -> [u8; 4] {
    let mut app = adding();
    for s in strings {
        add(&mut app, s);
    }
    enter(&mut app);
    assert_eq!(app.session.keys.len(), 1, "{:?}", error(&app));
    app.session.keys[0].fingerprint.0
}

/// A share of another set with vector 2's identifier and threshold, at
/// its index `a`: what a second paper with the same header would carry.
fn other_share_a() -> String {
    let secret = Codex32::from_seed(0, *b"name", &[7u8; 16]).unwrap();
    let shares = secret.split(2, 3, &[3u8; 64]).unwrap();
    let a = shares
        .iter()
        .find(|s| s.share_index() == b'a')
        .expect("a share at index a");
    a.encode().as_str().to_string()
}

/// §8.1: shares made by hand are typed one at a time, each checked at
/// once; one whose checksum fails is not taken and is named by its
/// index, and once enough are in the key loads.
#[test]
fn a_hand_made_set_typed_share_by_share_loads_its_key() {
    let mut app = adding();
    add(&mut app, &changed(SHARE_C, 30));
    assert_eq!(error(&app).as_deref(), Some("Share C · checksum fails"));
    assert!(testkit::entry(&app).parts.codex32.is_empty());
    // The typing is kept to fix: cleared here and typed right.
    for _ in 0..SHARE_C.len() {
        app.event(Event::Key(Key::Backspace));
    }
    add(&mut app, SHARE_C);
    assert_eq!(error(&app), None);
    assert!(app.session.keys.is_empty(), "one share of two is not a key");
    // The list names the share as Check a string does, its index in
    // capitals.
    let shown = app.drawn_texts();
    assert!(
        shown.iter().any(|t| t == "Share C · id name · 2 needed"),
        "{shown:?}"
    );
    add(&mut app, SHARE_A);
    enter(&mut app);
    assert_eq!(app.session.keys.len(), 1, "{:?}", error(&app));
    assert_eq!(
        app.session.keys[0].fingerprint.0,
        key_of(&[SHARE_A, SHARE_C])
    );
    // The secret's own string, failing, is named as the secret.
    let mut app = adding();
    add(&mut app, &changed(SECRET, 20));
    assert_eq!(error(&app).as_deref(), Some("Secret · checksum fails"));
}

/// §8.5: the same string again changes nothing and says "Already in"; a
/// different string at an index already in shows both and asks.
#[test]
fn a_share_index_already_in_is_kept_or_replaced_as_asked() {
    // The same string twice.
    let mut app = adding();
    add(&mut app, SHARE_A);
    add(&mut app, SHARE_A);
    let e = testkit::entry(&app);
    assert!(e.codex32.already, "{:?}", e.error);
    assert_eq!(e.parts.codex32.len(), 1);
    assert!(e.typed.is_empty());

    // Another string at index a: both shown, the differing boxes named.
    let other = other_share_a();
    add(&mut app, &other);
    let clash = testkit::entry(&app).codex32.clash.as_ref().expect("asked");
    assert_eq!(clash.held.as_str(), SHARE_A);
    assert_eq!(clash.text.as_str(), other);
    assert!(!clash.differ.is_empty());
    assert!(
        clash.differ.iter().all(|&n| n > 9),
        "the headers are the same"
    );
    // Keep the one held: the vector's key.
    app.press(Action::Codex32Form(C32Press::Keep));
    assert!(testkit::entry(&app).codex32.clash.is_none());
    add(&mut app, SHARE_C);
    enter(&mut app);
    assert_eq!(app.session.keys.len(), 1, "{:?}", error(&app));
    assert_eq!(
        app.session.keys[0].fingerprint.0,
        key_of(&[SHARE_A, SHARE_C])
    );

    // Replace it: the string typed is the one in.
    let mut app = adding();
    add(&mut app, SHARE_A);
    add(&mut app, &other);
    app.press(Action::Codex32Form(C32Press::Replace));
    let e = testkit::entry(&app);
    assert!(e.codex32.clash.is_none());
    assert_eq!(e.parts.codex32.len(), 1);
    assert_eq!(e.parts.codex32[0].encode().as_str(), other);
    assert!(app.session.keys.is_empty());
}

/// §8.5: a string from a scan is read as given: one case is taken, mixed
/// case refused; typing on the keyboard types one case.
#[test]
fn a_string_is_taken_in_one_case_and_refused_in_two() {
    let mut app = adding();
    app.press(Action::ScanPart);
    app.event(Event::Scanned {
        bytes: SHARE_A.to_uppercase().into_bytes(),
    });
    assert_eq!(testkit::entry(&app).parts.codex32.len(), 1);
    let mut mixed = SHARE_C.to_string();
    mixed.replace_range(0..2, "MS");
    app.press(Action::ScanPart);
    app.event(Event::Scanned {
        bytes: mixed.into_bytes(),
    });
    assert_eq!(testkit::entry(&app).parts.codex32.len(), 1);
    assert!(app.scan().is_some_and(|s| s.note.is_some()));
    // Typed in capitals on a keyboard: one case, taken.
    let mut app = adding();
    add(&mut app, &SHARE_C.to_uppercase());
    assert_eq!(testkit::entry(&app).parts.codex32.len(), 1);
}

/// §8.3: Check a string says whether a string's checksum holds and
/// whether it is of the set typed in, and loads, keeps and files
/// nothing; the string goes on leaving.
#[test]
fn check_a_string_says_whether_it_holds_and_belongs() {
    let mut app = adding();
    add(&mut app, SHARE_A);
    app.press(Action::Codex32Form(C32Press::Check));
    assert!(testkit::entry(&app).codex32.checking);

    add(&mut app, SHARE_C);
    let checked = testkit::entry(&app)
        .codex32
        .checked
        .clone()
        .expect("checked");
    assert!(checked.holds);
    assert_eq!(checked.what, "Share C · id name · 2 needed");
    assert_eq!(checked.of_the_set, Some(true));

    // Another set's string: its checksum holds, it is not of this set.
    for _ in 0..SHARE_C.len() {
        app.event(Event::Key(Key::Backspace));
    }
    add(&mut app, SECRET);
    let checked = testkit::entry(&app)
        .codex32
        .checked
        .clone()
        .expect("checked");
    assert!(checked.holds);
    assert_eq!(checked.of_the_set, Some(false));

    // A box changed: the checksum fails.
    for _ in 0..SECRET.len() {
        app.event(Event::Key(Key::Backspace));
    }
    add(&mut app, &changed(SHARE_C, 25));
    let checked = testkit::entry(&app)
        .codex32
        .checked
        .clone()
        .expect("checked");
    assert!(!checked.holds);

    // Nothing was loaded or added, and leaving drops the string.
    assert!(app.session.keys.is_empty());
    assert_eq!(testkit::entry(&app).parts.codex32.len(), 1);
    app.press(Action::Codex32Form(C32Press::Check));
    let e = testkit::entry(&app);
    assert!(!e.codex32.checking);
    assert!(e.typed.is_empty());
    assert!(e.codex32.checked.is_none());
    // Back to adding: the share it checked makes the key.
    add(&mut app, SHARE_C);
    enter(&mut app);
    assert_eq!(app.session.keys.len(), 1, "{:?}", error(&app));
}

/// §8.4, wired: with the tick on, a string with errors within the bound
/// gives its proposal in the form, the changed boxes typed again from
/// the paper take it, and the key the strings make carries its mark.
#[test]
fn a_corrected_string_goes_in_and_its_key_carries_the_mark() {
    let mut app = adding();
    // Off: `?` is not typed, a failing string is refused as before.
    typed(&mut app, "?");
    assert!(testkit::entry(&app).typed.is_empty());
    add(&mut app, &changed(SHARE_A, 20));
    assert!(error(&app).is_some());
    assert!(!testkit::entry(&app).correction.is_open());
    for _ in 0..SHARE_A.len() {
        app.event(Event::Key(Key::Backspace));
    }

    app.press(Action::Codex32Correct(Press::Tick));
    assert!(testkit::entry(&app).correction.on());
    add(&mut app, &changed(changed(SHARE_A, 20).as_str(), 31));
    let c = &testkit::entry(&app).correction;
    assert!(c.is_open(), "{:?}", error(&app));
    let edits: Vec<(usize, u8)> = c.edits().iter().map(|e| (e.position(), e.now())).collect();
    assert_eq!(edits.len(), 2);
    assert_eq!(edits.iter().map(|e| e.0).collect::<Vec<_>>(), vec![20, 31]);
    // I mistyped: the changed boxes typed again from the paper.
    app.press(Action::Codex32Correct(Press::Mistyped));
    for (n, _) in &edits {
        let ch = char::from(SHARE_A.as_bytes()[n - 1]);
        app.event(Event::Key(Key::Char(ch)));
    }
    let e = testkit::entry(&app);
    assert!(!e.correction.is_open());
    assert_eq!(e.parts.codex32.len(), 1);
    assert_eq!(e.parts.codex32[0].encode().as_str(), SHARE_A);
    assert_eq!(
        e.correction.mark_of(&e.parts.codex32[0]),
        Some(Mark::Corrected)
    );

    add(&mut app, SHARE_C);
    enter(&mut app);
    assert_eq!(app.session.keys.len(), 1, "{:?}", error(&app));
    let fp = app.session.keys[0].fingerprint.0;
    assert_eq!(fp, key_of(&[SHARE_A, SHARE_C]));
    assert_eq!(app.session.key_marks.of(fp), Some(Mark::Corrected));
}

/// §8.4, wired: a box marked `?` on the keyboard, filled from the
/// checksum as Unreadable on the paper, marks the string "Repaired, not
/// checked" and its key "Share repairs unchecked".
#[test]
fn an_unreadable_box_filled_from_the_checksum_is_marked_repaired() {
    let mut app = adding();
    app.press(Action::Codex32Correct(Press::Tick));
    let mut damaged = SECRET.as_bytes().to_vec();
    damaged[24] = b'?';
    add(&mut app, std::str::from_utf8(&damaged).unwrap());
    assert!(testkit::entry(&app).correction.offers_unreadable());
    app.press(Action::Codex32Correct(Press::Unreadable));
    app.press(Action::Codex32Correct(Press::Take));
    // The secret is the key at once.
    assert_eq!(app.session.keys.len(), 1, "{:?}", error(&app));
    let fp = app.session.keys[0].fingerprint.0;
    assert_eq!(fp, key_of(&[SECRET]));
    assert_eq!(app.session.key_marks.of(fp), Some(Mark::Repaired));
}

/// The Codex32QR profile's pictures (§10.3), each read as the disk
/// process reads a PNG and scanned into the codex32 form: every set's
/// shares, and every secret alone, load the key of the stated
/// fingerprint.
#[test]
fn codex32qr_pictures_scanned_load_the_stated_keys() {
    const DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/codex32qr/"
    );
    // Every picture in the profile's img/ and test_vectors/ folders, as
    // sets of strings enough for the key, with the fingerprint
    // vectors.json and bip93_128bit_vectors.json state.
    let vectors: [(&[&[&str]], &str); 7] = [
        (
            &[
                &[
                    "img/split-shares/love_share_a.png",
                    "img/split-shares/love_share_c.png",
                ],
                &["img/s-shares/codex32_s_share_2_l0ve.png"],
            ],
            "a6c4da6d",
        ),
        (
            &[
                &[
                    "img/split-shares/wsfp_share_a.png",
                    "img/split-shares/wsfp_share_c.png",
                ],
                &["img/s-shares/codex32_s_share_3_wfsp.png"],
            ],
            "c17858e9",
        ),
        (
            &[&[
                "img/split-shares/f0ur_share_a.png",
                "img/split-shares/f0ur_share_c.png",
                "img/split-shares/f0ur_share_d.png",
                "img/split-shares/f0ur_share_e.png",
            ]],
            "04b3384e",
        ),
        (&[&["img/s-shares/codex32_s_share_1_seed.png"]], "0e549ffd"),
        (&[&["test_vectors/bip93-v1-secret.png"]], "3f3521a6"),
        (
            &[
                &[
                    "test_vectors/bip93-v2-share-a.png",
                    "test_vectors/bip93-v2-share-c.png",
                ],
                &[
                    "test_vectors/bip93-v2-share-d.png",
                    "test_vectors/bip93-v2-share-c.png",
                ],
                &["test_vectors/bip93-v2-secret.png"],
            ],
            "fab6868a",
        ),
        (
            &[
                &[
                    "test_vectors/bip93-v3-share-a.png",
                    "test_vectors/bip93-v3-share-c.png",
                    "test_vectors/bip93-v3-share-d.png",
                ],
                &[
                    "test_vectors/bip93-v3-share-e.png",
                    "test_vectors/bip93-v3-share-f.png",
                    "test_vectors/bip93-v3-share-d.png",
                ],
                &["test_vectors/bip93-v3-secret.png"],
            ],
            "1e50c111",
        ),
    ];
    let c = testkit::compute();
    let read = |path: &str| -> Vec<u8> {
        let png = std::fs::read(format!("{DIR}{path}")).expect(path);
        let payloads = faraday_files::qr_in_png(&c, &png).expect("a PNG");
        assert_eq!(payloads.len(), 1, "{path}: one code");
        let text = std::str::from_utf8(&payloads[0]).expect("text");
        assert!(text.len() == 48 && text == text.to_uppercase(), "{path}");
        payloads[0].clone()
    };
    let hex = |fp: [u8; 4]| fp.iter().map(|b| format!("{b:02x}")).collect::<String>();
    for (ways, want) in vectors {
        for pictures in ways.iter().copied() {
            let mut app = adding();
            for p in pictures {
                app.press(Action::ScanPart);
                app.event(Event::Scanned { bytes: read(p) });
                assert!(
                    app.scan().is_none(),
                    "{p}: {:?}",
                    app.scan().and_then(|s| s.note.clone())
                );
            }
            if app.session.keys.is_empty() {
                enter(&mut app);
            }
            assert_eq!(app.session.keys.len(), 1, "{pictures:?}: {:?}", error(&app));
            assert_eq!(hex(app.session.keys[0].fingerprint.0), want, "{pictures:?}");
        }
    }
}

/// §10.1: Add a key's Scan a SeedQR takes a Codex32QR code too, and opens
/// the codex32 form with the string in, waiting for the rest of the set.
#[test]
fn scan_a_seedqr_takes_a_codex32qr_code_into_the_codex32_form() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::ScanSeed);
    app.event(Event::Scanned {
        bytes: SHARE_A.to_uppercase().into_bytes(),
    });
    assert!(app.scan().is_none(), "the camera stayed open");
    assert_eq!(app.screen, Screen::Entry);
    let e = testkit::entry(&app);
    assert_eq!(e.form, Form::Codex32);
    assert_eq!(e.parts.codex32.len(), 1);
    assert!(app.session.keys.is_empty(), "nothing loads on one share");
    add(&mut app, SHARE_C);
    enter(&mut app);
    assert_eq!(app.session.keys.len(), 1, "{:?}", error(&app));
}
