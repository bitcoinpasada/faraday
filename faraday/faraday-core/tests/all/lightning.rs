//! A Lightning node's key: an LND cipher seed and a loaded key give the
//! node ids LND and ldk-node give them (`tools/reference/aezeed/`), and
//! the private key leaves only through the vault or the secret sheet.

use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_shell_api::{App, Event, Key};

/// LND's cipher seed with no passphrase, and the node id LND derives.
const CIPHER_SEED: &str = "above judge emerge veteran reform crunch system all snap please \
    shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
const NODE_KEY: &str = "024c7005923a074fd38b16ace7be4914ec9f929717692bfb585019be13290c9b1d";

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn typed(app: &mut Faraday, text: &str) {
    for c in text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
    {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn an_lnd_cipher_seed_gives_lnds_node_id() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Lightning);
    assert_eq!(app.screen, Screen::Lightning);
    app.press(Action::LAezeed);
    typed(&mut app, CIPHER_SEED);
    app.event(Event::Key(Key::Enter));
    // Drawn first, the aezeed stretched on the tick after (`docs/SPEED.md`
    // §9).
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    let l = app.lightning().unwrap();
    let n = l.node.as_ref().unwrap_or_else(|| panic!("{:?}", l.error));
    assert_eq!(hex(n.public()), NODE_KEY);
    assert!(n.birthday().is_some());
    // The private key goes out only through the secret sheet.
    app.press(Action::LOut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty());
}

#[test]
fn a_loaded_key_gives_ldk_nodes_node_id() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    typed(
        &mut app,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    app.press(Action::EntryAdd);
    app.press(Action::Lightning);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::LKey(fp));
    let n = app.lightning().unwrap().node.as_ref().unwrap();
    assert_eq!(
        hex(n.public()),
        "027cf7c81dc777e46572ff964f17650a0f6f783319fcc140fb9a69425e2fbdc93c"
    );
}

/// A letter outside printable ASCII typed into the aezeed's passphrase is
/// not taken, and the screen says it is refused.
#[test]
fn the_aezeed_passphrase_refuses_a_letter_outside_ascii_and_says_so() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    app.press(Action::LPassphrase);
    for c in "café".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert_eq!(&*app.lightning().unwrap().passphrase, "caf");
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == crate::ascii_passphrase::REFUSED),
        "{texts:?}"
    );
}

/// The node id offers its code and its picture, and both read back as the
/// node id; the picture goes For the stick as a public file, labelled with
/// the node's kind (`docs/QR.md` §7). Nothing else on the page opens a
/// code: the private key has none.
#[test]
fn the_node_id_reads_back_from_its_code_and_its_picture() {
    use crate::common::{codes_offered, png_text, qr_text};
    use faraday_core::Code;
    let mut app = faraday_core::testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    typed(&mut app, CIPHER_SEED);
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    app.press(Action::LShow);
    assert_eq!(
        codes_offered(&mut app),
        vec![
            Action::ShowCode(Code::NodeId),
            Action::CodePng(Code::NodeId)
        ]
    );
    app.press(Action::ShowCode(Code::NodeId));
    let q = app.qr().expect("no QR sheet");
    assert!(!q.secret);
    assert!(q.offers_png());
    assert!(q.label.iter().any(|l| l == "LND"), "{:?}", q.label);
    assert_eq!(qr_text(&app), NODE_KEY);
    app.press(Action::Cancel);
    app.press(Action::CodePng(Code::NodeId));
    let name = format!("lightning-{}.png", &NODE_KEY[..8]);
    let item = app
        .outbox
        .iter()
        .find(|i| i.name.as_str() == name.as_str())
        .unwrap_or_else(|| panic!("no {name}"));
    assert!(!item.secret);
    assert_eq!(png_text(&item.bytes), NODE_KEY);
}

/// Core Lightning's own test of `lightning-hsmtool encrypt`: 32 bytes of
/// 0x01, its node id, and the file sealed by libsodium under
/// `test_password`.
const ONES_ID: &str = "026f61d7ee82f937f9697f4f3e44bfaaa25849cc4f526b3a57326130eba6346002";
const ONES_SEALED: &str = "2c9ac8c7092b7c78712ceb9178be048d98c4a5871ae4eea739d8aa473e0fd750242028ee9acd86ecc6131d72cbc725c6efda4fb84ff06ba6c9b8eba2bfad6001a618eab8d79b30b911";
const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
/// The node id Core Lightning's test suite gives these words.
const ABANDON_ID: &str = "03653e90c1ce4660fd8505dd6d643356e93cfe202af109d382787639dd5890e87d";
/// SHA-256 of the seed of `ABANDON` under the empty passphrase: what
/// Core Lightning writes when a person presses Enter at the passphrase.
const ABANDON_EMPTY_HASH: &str = "62a772f85e4be6226108b56c0b1cf935c2490e434adec864fe47b189f1ed517d";

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

/// An app on the Core Lightning source with `bytes` in Files, chosen.
fn with_file(bytes: Vec<u8>) -> Faraday {
    let mut app = faraday_core::testkit::started();
    faraday_core::testkit::inbox_file(&mut app, "hsm_secret", bytes);
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    app.press(Action::LFile(app.inbox().len() - 1));
    faraday_core::testkit::settle(&mut app);
    app
}

fn node_id(app: &Faraday) -> String {
    let l = app.lightning().unwrap();
    hex(l
        .node
        .as_ref()
        .unwrap_or_else(|| panic!("{:?}", l.error))
        .public())
}

#[test]
fn a_plain_hsm_secret_in_files_gives_its_node_id_with_no_passphrase_asked() {
    let app = with_file(vec![1u8; 32]);
    assert_eq!(node_id(&app), ONES_ID);
}

#[test]
fn an_encrypted_hsm_secret_asks_for_its_passphrase_and_opens_under_it() {
    let mut app = with_file(unhex(ONES_SEALED));
    assert!(app.lightning().unwrap().node.is_none());
    app.press(Action::LPassphrase);
    typed(&mut app, "test_password");
    app.event(Event::Key(Key::Enter));
    // Drawn first, the file opened on the tick after.
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    assert_eq!(node_id(&app), ONES_ID);
}

#[test]
fn a_wrong_passphrase_for_an_hsm_secret_is_one_line() {
    let mut app = with_file(unhex(ONES_SEALED));
    app.press(Action::LPassphrase);
    typed(&mut app, "test_passwore");
    app.press(Action::LResolve);
    faraday_core::testkit::settle(&mut app);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none());
    assert_eq!(
        l.error.as_deref(),
        Some("Wrong passphrase or a changed file")
    );
}

#[test]
fn a_words_form_file_whose_passphrase_was_empty_opens_with_none_typed() {
    let mut file = unhex(ABANDON_EMPTY_HASH);
    file.extend_from_slice(ABANDON.as_bytes());
    let mut app = with_file(file);
    assert!(app.lightning().unwrap().file_asks_passphrase());
    app.press(Action::LResolve);
    faraday_core::testkit::settle(&mut app);
    assert_eq!(node_id(&app), ABANDON_ID);
}

#[test]
fn a_words_form_file_with_no_passphrase_asks_nothing() {
    let mut file = vec![0u8; 32];
    file.extend_from_slice(ABANDON.as_bytes());
    let app = with_file(file);
    assert!(!app.lightning().unwrap().file_asks_passphrase());
    assert_eq!(node_id(&app), ABANDON_ID);
}

#[test]
fn a_broken_words_form_file_says_so() {
    let mut file = vec![0u8; 32];
    file.extend_from_slice(b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon");
    let app = with_file(file);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none());
    assert_eq!(
        l.error.as_deref(),
        Some("The words in the file are not English BIP-39 words in Core Lightning's form")
    );
}

#[test]
fn english_words_give_core_lightnings_node_id() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    typed(&mut app, ABANDON);
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    assert_eq!(node_id(&app), ABANDON_ID);
}

#[test]
fn words_that_are_not_a_mnemonic_are_refused_with_a_line() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    typed(&mut app, "abandon abandon");
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none());
    assert!(l.error.is_some());
}

#[test]
fn a_loaded_key_gives_core_lightnings_node_beside_ldk_nodes() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    typed(&mut app, ABANDON);
    app.press(Action::EntryAdd);
    app.press(Action::Lightning);
    let fp = app.session.keys[0].fingerprint.0;
    let _ = app.frame();
    assert!(app.offers(Action::LKey(fp)));
    assert!(app.offers(Action::LCln(Some(fp))));
    app.press(Action::LCln(Some(fp)));
    assert_eq!(node_id(&app), ABANDON_ID);
    app.press(Action::LKey(fp));
    assert_ne!(node_id(&app), ABANDON_ID);
}

#[test]
fn a_file_that_asks_for_no_passphrase_takes_no_typing() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    app.press(Action::LPassphrase);
    faraday_core::testkit::inbox_file(&mut app, "hsm_secret", vec![1u8; 32]);
    app.press(Action::LFile(app.inbox().len() - 1));
    faraday_core::testkit::settle(&mut app);
    typed(&mut app, "abc");
    app.event(Event::Key(Key::Backspace));
    assert_eq!(node_id(&app), ONES_ID);
}

#[test]
fn a_file_that_asks_takes_its_passphrase_at_once_and_the_next_file_starts_empty() {
    let mut app = with_file(unhex(ONES_SEALED));
    typed(&mut app, "test_password");
    assert_eq!(&*app.lightning().unwrap().passphrase, "test_password");
    faraday_core::testkit::inbox_file(&mut app, "second", unhex(ONES_SEALED));
    app.press(Action::LFile(app.inbox().len() - 1));
    assert_eq!(&*app.lightning().unwrap().passphrase, "");
}

#[test]
fn a_passphrase_typed_for_a_file_is_not_used_after_switching_to_words_or_the_aezeed() {
    for to_aezeed in [false, true] {
        let mut app = with_file(unhex(ONES_SEALED));
        typed(&mut app, "test_password");
        assert_eq!(&*app.lightning().unwrap().passphrase, "test_password");
        app.press(if to_aezeed {
            Action::LAezeed
        } else {
            Action::LCln(None)
        });
        assert_eq!(&*app.lightning().unwrap().passphrase, "");
        assert_eq!(&*app.lightning().unwrap().typed, "");
    }
}

#[test]
fn the_aezeed_and_a_loaded_key_leave_a_chosen_file() {
    let mut app = with_file(unhex(ONES_SEALED));
    app.press(Action::LAezeed);
    typed(&mut app, "above judge");
    assert_eq!(&*app.lightning().unwrap().typed, "above judge");
    assert!(!app.lightning().unwrap().file_asks_passphrase());
    for key_button in [true, false] {
        let mut app = faraday_core::testkit::started();
        app.press(Action::Entry(None));
        typed(&mut app, ABANDON);
        app.press(Action::EntryAdd);
        faraday_core::testkit::inbox_file(&mut app, "hsm_secret", unhex(ONES_SEALED));
        app.press(Action::Lightning);
        app.press(Action::LCln(None));
        app.press(Action::LFile(app.inbox().len() - 1));
        let fp = app.session.keys[0].fingerprint.0;
        assert!(app.lightning().unwrap().file_asks_passphrase());
        app.press(if key_button {
            Action::LKey(fp)
        } else {
            Action::LCln(Some(fp))
        });
        assert!(!app.lightning().unwrap().file_asks_passphrase());
        assert!(
            app.lightning().unwrap().node.is_some(),
            "{:?}",
            app.lightning().unwrap().error
        );
    }
}

#[test]
fn enter_on_a_files_passphrase_keeps_the_field_for_the_next_try() {
    let mut app = with_file(unhex(ONES_SEALED));
    typed(&mut app, "wrong");
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    assert!(app.lightning().unwrap().node.is_none());
    assert!(app.lightning().unwrap().error.is_some());
    for _ in 0.."wrong".len() {
        app.event(Event::Key(Key::Backspace));
    }
    typed(&mut app, "test_password");
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    assert_eq!(node_id(&app), ONES_ID);
}

/// The picture and the saved file name the program the node is for, and
/// the key's fingerprint for a loaded key's.
#[test]
fn a_nodes_code_names_core_lightning_for_its_sources_and_lnd_only_for_the_aezeed() {
    use faraday_core::Code;
    let label = |app: &mut Faraday| {
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        faraday_core::testkit::settle(app);
        assert!(
            app.lightning().unwrap().node.is_some(),
            "{:?}",
            app.lightning().unwrap().error
        );
        if !app.offers(Action::ShowCode(Code::NodeId)) {
            app.press(Action::LShow);
            let _ = app.frame();
        }
        app.press(Action::ShowCode(Code::NodeId));
        let l = app.qr().expect("no QR sheet").label.clone();
        app.press(Action::Cancel);
        l
    };
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    typed(&mut app, ABANDON);
    app.press(Action::EntryAdd);
    app.press(Action::Lightning);
    let fp = app.session.keys[0].fingerprint.0;
    let _ = app.frame();
    app.press(Action::LCln(Some(fp)));
    let l = label(&mut app);
    assert!(l[0].starts_with("Core Lightning · "), "{l:?}");
    app.press(Action::LKey(fp));
    let l = label(&mut app);
    assert!(l[0].starts_with("ldk-node · "), "{l:?}");
    app.press(Action::LCln(None));
    typed(&mut app, ABANDON);
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    assert_eq!(label(&mut app)[0], "Core Lightning");
    let mut app = faraday_core::testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    typed(&mut app, CIPHER_SEED);
    app.event(Event::Key(Key::Enter));
    assert_eq!(label(&mut app)[0], "LND");
}

#[test]
fn enter_on_typed_words_is_drawn_first() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    typed(&mut app, ABANDON);
    app.event(Event::Key(Key::Enter));
    assert!(app.lightning().unwrap().node.is_none());
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    assert_eq!(node_id(&app), ABANDON_ID);
}

#[test]
fn a_chosen_file_is_found_by_name_when_files_change_and_says_so_when_gone() {
    let mut app = with_file(unhex(ONES_SEALED));
    // Another file in front of it moves its place.
    faraday_core::testkit::inbox_file_at(&mut app, 0, "other", vec![2u8; 32]);
    typed(&mut app, "test_password");
    app.press(Action::LResolve);
    faraday_core::testkit::settle(&mut app);
    assert_eq!(node_id(&app), ONES_ID);
    // Gone from Files.
    faraday_core::testkit::inbox_retain(&mut app, |i| i.name != "hsm_secret");
    app.press(Action::LResolve);
    faraday_core::testkit::settle(&mut app);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none());
    assert_eq!(l.error.as_deref(), Some("hsm_secret is no longer in Files"));
}
