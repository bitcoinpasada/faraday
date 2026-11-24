//! A Lightning node's key: an LND cipher seed and a loaded key give the
//! node ids LND and ldk-node give them (`tools/reference/aezeed/`), and
//! the private key leaves only through the vault or the secret sheet.

use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_shell_api::{App, Event, Key};

/// LND's cipher seed with no passphrase, and the node id LND derives at
/// coin type 1, which every network but Mainnet uses (the reference's
/// `tools/reference/aezeed/README.md` lists it with coin type 0's).
const CIPHER_SEED: &str = "above judge emerge veteran reform crunch system all snap please \
    shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
const STICK_LINE: &str = "Keys load only with no stick attached";
const NODE_KEY: &str = "0226594d21c0862a11168ab07cdbc15e7c7af5ee561b741259e311f1614f4df3b7";

/// A start on the test network, where LND's cipher seed (a public sample)
/// is taken: Mainnet refuses it.
fn test_started() -> Faraday {
    faraday_core::testkit::started()
}

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
    let mut app = test_started();
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
    faraday_core::testkit::keep_published(&mut app);
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
    assert_eq!(app.lightning().unwrap().passphrase(), "caf");
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
    let mut app = test_started();
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
    faraday_core::testkit::keep_published(&mut app);
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
    assert_eq!(app.lightning().unwrap().passphrase(), "test_password");
    faraday_core::testkit::inbox_file(&mut app, "second", unhex(ONES_SEALED));
    app.press(Action::LFile(app.inbox().len() - 1));
    assert_eq!(app.lightning().unwrap().passphrase(), "");
}

#[test]
fn a_passphrase_typed_for_a_file_is_not_used_after_switching_to_words_or_the_aezeed() {
    for to_aezeed in [false, true] {
        let mut app = with_file(unhex(ONES_SEALED));
        typed(&mut app, "test_password");
        assert_eq!(app.lightning().unwrap().passphrase(), "test_password");
        app.press(if to_aezeed {
            Action::LAezeed
        } else {
            Action::LCln(None)
        });
        assert_eq!(app.lightning().unwrap().passphrase(), "");
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
        faraday_core::testkit::keep_published(&mut app);
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
    faraday_core::testkit::keep_published(&mut app);
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

#[test]
fn done_goes_back_to_where_the_node_key_was_opened_and_another_starts_again() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::Tools));
    let from = app.screen;
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    typed(&mut app, CIPHER_SEED);
    app.press(Action::LAnother);
    assert_eq!(app.screen, Screen::Lightning);
    let l = app.lightning().unwrap();
    assert_eq!(l.back, from);
    assert!(l.source.is_none() && l.node.is_none() && l.typed.is_empty());
    app.press(Action::Nav(from));
    assert_eq!(app.screen, from);
    app.press(Action::Lightning);
    app.press(Action::Nav(Screen::Home));
    assert_eq!(app.screen, Screen::Home);
}

/// Types LND's sample cipher seed into Lightning on `network` and reads it.
fn read_cipher_seed(network: faraday_bip::keys::Network) -> Faraday {
    let mut app = faraday_core::testkit::started();
    faraday_core::testkit::move_to(&mut app.session, network).unwrap();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    typed(&mut app, CIPHER_SEED);
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    app
}

#[test]
fn lnds_public_sample_cipher_seed_is_refused_on_mainnet_and_taken_on_testnet() {
    let app = read_cipher_seed(faraday_bip::keys::Network::Mainnet);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none(), "Mainnet holds the sample's node key");
    assert_eq!(
        l.error.as_deref(),
        Some(faraday_core::strings::EN.published_sample_node)
    );
    let app = read_cipher_seed(faraday_bip::keys::Network::Testnet);
    let l = app.lightning().unwrap();
    assert_eq!(hex(l.node.as_ref().expect("a node").public()), NODE_KEY);
}

fn attach_stick(app: &mut Faraday) {
    use faraday_core::{StickInfo, StorageEvent};
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    if app.sheet == Some(Sheet::Lock) {
        app.press(Action::NotNow);
    }
}

/// With a stick attached the aezeed's words take no key, and the screen
/// says keys load only with no stick attached.
#[test]
fn the_aezeed_words_take_no_typing_with_a_stick_attached() {
    let mut app = test_started();
    attach_stick(&mut app);
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    typed(&mut app, "above judge");
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.lightning().unwrap().typed.as_str(), "");
    assert_eq!(app.toast_text(), Some(STICK_LINE));
}

/// With a stick attached the passphrase takes no key either, and the
/// screen says keys load only with no stick attached.
#[test]
fn the_aezeed_passphrase_takes_no_typing_with_a_stick_attached() {
    let mut app = test_started();
    attach_stick(&mut app);
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    app.press(Action::LPassphrase);
    typed(&mut app, "hunter2");
    assert_eq!(app.lightning().unwrap().passphrase(), "");
    assert_eq!(app.toast_text(), Some(STICK_LINE));
}

/// Words and a passphrase typed, then a stick arrives: both are empty.
#[test]
fn a_stick_arriving_empties_the_aezeed_words_and_passphrase() {
    let mut app = test_started();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    typed(&mut app, "above judge");
    app.press(Action::LPassphrase);
    typed(&mut app, "hunter2");
    assert_eq!(app.lightning().unwrap().typed.as_str(), "above judge");
    attach_stick(&mut app);
    assert_eq!(app.screen, Screen::Lightning);
    assert_eq!(app.lightning().unwrap().typed.as_str(), "");
    assert_eq!(app.lightning().unwrap().passphrase(), "");
}

/// A started app on a 1366 × 768 window.
fn wide_window() -> Faraday {
    window(1366, 768)
}

/// A started app on a `width` × `height` window.
fn window(width: u16, height: u16) -> Faraday {
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};
    let mut app = test_started();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// The Lightning page on the wide window with `n` keys loaded.
fn lightning_many(n: u8) -> Faraday {
    lightning_keys(wide_window(), n)
}

/// The Lightning page of `app` with `n` keys loaded.
fn lightning_keys(mut app: Faraday, n: u8) -> Faraday {
    add_keys(&mut app, n);
    app.press(Action::Lightning);
    assert_eq!(app.screen, Screen::Lightning);
    app
}

/// `n` words keys loaded into `app`.
fn add_keys(app: &mut Faraday, n: u8) {
    use faraday_bip::bip39::Language;
    let c = faraday_core::testkit::compute();
    for i in 0..n {
        let m = faraday_keys::words::from_entropy(Language::English, &[i + 1; 16]).unwrap();
        let words: Vec<&str> = m
            .indices()
            .iter()
            .map(|&w| Language::English.words()[usize::from(w)])
            .collect();
        app.session
            .keys
            .add_words(&c, &words.join(" "), &format!("Seed {i}"), None)
            .unwrap();
    }
}

fn key_at(app: &Faraday, i: usize) -> [u8; 4] {
    app.session.keys.all().nth(i).unwrap().fingerprint.0
}

/// A press and its release at the middle of what offers `a`.
fn touch(app: &mut Faraday, a: Action) {
    let (x, y) = app.where_offered(a).expect("offered");
    for phase in [
        faraday_shell_api::TouchPhase::Down,
        faraday_shell_api::TouchPhase::Up,
    ] {
        app.event(Event::Touch { x, y, phase });
    }
}

#[test]
fn a_wide_window_lists_every_key_in_a_pane_and_a_press_picks_one() {
    let mut app = lightning_many(2);
    let (first, second) = (key_at(&app, 0), key_at(&app, 1));
    assert!(crate::common::offered(&mut app, Action::LKey(first)));
    // The card begins where the aezeed's button does.
    let (card, ..) = app.hit_box(Action::LAezeed).expect("the card's button");
    for a in [
        Action::LKey(first),
        Action::LCln(Some(first)),
        Action::LKey(second),
        Action::LCln(Some(second)),
    ] {
        let (x, _, w, _) = app.hit_box(a).expect("a row");
        assert!(x + w < card, "the row ends left of the card");
    }
    touch(&mut app, Action::LKey(second));
    assert_eq!(
        app.lightning().unwrap().source,
        Some(faraday_core::lightning::NodeSource::Key(second))
    );
    let _ = app.frame();
    touch(&mut app, Action::LCln(Some(first)));
    assert_eq!(
        app.lightning().unwrap().source,
        Some(faraday_core::lightning::NodeSource::KeyCln(first))
    );
}

#[test]
fn the_card_follows_the_key_picked_in_the_pane() {
    let mut app = lightning_many(2);
    let id = |app: &mut Faraday, k: [u8; 4]| {
        app.press(Action::LKey(k));
        app.lightning()
            .unwrap()
            .node
            .as_ref()
            .map(|n| hex(n.public()))
    };
    let (a, b) = (key_at(&app, 0), key_at(&app, 1));
    let (ida, idb) = (id(&mut app, a), id(&mut app, b));
    assert!(ida.is_some() && idb.is_some());
    assert_ne!(ida, idb);
    let shown = crate::common::texts(&mut app).concat().replace(' ', "");
    assert!(shown.contains(&idb.unwrap()), "the card shows the node id");
}

/// What the page draws for the private key while it is hidden.
const MASK: &str = "••••••••••••";

/// Whether the page drawn hides the private key (its mask is on it).
fn key_hidden(app: &mut Faraday) -> bool {
    crate::common::texts(app).iter().any(|t| t.contains(MASK))
}

#[test]
fn a_shown_private_key_hides_when_the_key_picked_changes() {
    let mut app = lightning_many(2);
    let (a, b) = (key_at(&app, 0), key_at(&app, 1));
    app.press(Action::LKey(a));
    app.press(Action::LShow);
    assert!(!key_hidden(&mut app));
    app.press(Action::LKey(b));
    assert!(key_hidden(&mut app));
    app.press(Action::LShow);
    app.press(Action::LCln(Some(b)));
    assert!(key_hidden(&mut app));
    app.press(Action::LShow);
    app.press(Action::LAezeed);
    assert!(!app.lightning().unwrap().shown);
    // Core Lightning's entry, left from a node that was shown.
    app.press(Action::LKey(a));
    app.press(Action::LShow);
    app.press(Action::LCln(None));
    assert!(!app.lightning().unwrap().shown);
}

#[test]
fn a_second_hsm_secret_file_does_not_show_its_private_key() {
    let mut app = faraday_core::testkit::started();
    faraday_core::testkit::inbox_file(&mut app, "hsm_secret", vec![1u8; 32]);
    faraday_core::testkit::inbox_file(&mut app, "hsm_secret2", vec![2u8; 32]);
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    app.press(Action::LFile(0));
    faraday_core::testkit::settle(&mut app);
    app.press(Action::LShow);
    assert!(!key_hidden(&mut app));
    app.press(Action::LFile(1));
    faraday_core::testkit::settle(&mut app);
    assert!(app.lightning().unwrap().node.is_some());
    assert!(key_hidden(&mut app), "the second file's key is drawn");
}

#[test]
fn an_aezeed_edited_and_read_again_does_not_show_its_private_key() {
    let mut app = read_cipher_seed(faraday_bip::keys::Network::Testnet);
    assert!(app.lightning().unwrap().node.is_some());
    app.press(Action::LShow);
    assert!(!key_hidden(&mut app));
    // An edit and its undo: the same words, read again.
    app.event(Event::Key(Key::Backspace));
    app.event(Event::Key(Key::Char('y')));
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    faraday_core::testkit::settle(&mut app);
    assert!(app.lightning().unwrap().node.is_some());
    assert!(key_hidden(&mut app), "the node read again is drawn hidden");
}

#[test]
fn a_key_picked_clears_the_words_and_passphrase_typed_before() {
    let mut app = lightning_many(1);
    let a = key_at(&app, 0);
    for pick in [Action::LKey(a), Action::LCln(Some(a))] {
        app.press(Action::LAezeed);
        typed(&mut app, "above judge");
        app.press(Action::LPassphrase);
        typed(&mut app, "hunter2");
        assert_eq!(app.lightning().unwrap().typed.as_str(), "above judge");
        app.press(pick);
        assert_eq!(app.lightning().unwrap().typed.as_str(), "");
        assert_eq!(app.lightning().unwrap().passphrase(), "");
    }
}

/// Scrolls until the pane's last row is whole in the window (its box as
/// tall as the first row's and ends inside it).
#[test]
fn a_pane_of_many_keys_scrolls_to_its_last_row() {
    let mut app = lightning_many(20);
    let last = key_at(&app, 19);
    let (w, h) = app.size();
    let _ = app.frame();
    let full = app
        .hit_box(Action::LCln(Some(key_at(&app, 0))))
        .expect("the first row")
        .3;
    for _ in 0..400 {
        if app
            .hit_box(Action::LCln(Some(last)))
            .is_some_and(|(_, y, _, bh)| bh == full && f32::from(y + bh) <= h)
        {
            return;
        }
        app.event(Event::Scroll {
            x: (w * 0.5) as u16,
            y: (h / 2.0) as u16,
            dy: 120,
        });
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    }
    panic!("the last key's row never came whole into the window");
}

#[test]
fn the_four_rows_of_two_keys_share_one_left_edge() {
    let mut app = lightning_many(2);
    let _ = app.frame();
    let xs: Vec<u16> = (0..2)
        .flat_map(|i| {
            let k = key_at(&app, i);
            [Action::LKey(k), Action::LCln(Some(k))]
        })
        .map(|a| app.hit_box(a).expect("a row").0)
        .collect();
    assert!(xs.iter().all(|&x| x == xs[0]), "{xs:?}");
}

#[test]
fn the_divider_stays_while_the_page_has_no_node_to_show() {
    let mut app = lightning_many(2);
    assert!(app.lightning().unwrap().node.is_none());
    let first = key_at(&app, 0);
    let _ = app.frame();
    let (card, ..) = app.hit_box(Action::LAezeed).expect("the card's button");
    let (x, _, w, _) = app.hit_box(Action::LKey(first)).expect("a row");
    let (from, to) = (usize::from(x + w) + 1, usize::from(card));
    let (width, _) = app.size();
    let rgba = app.frame().rgba.to_vec();
    let row = 330 * width as usize;
    let at = |col: usize| &rgba[(row + col) * 4..(row + col) * 4 + 4];
    assert!(
        (from..to).any(|col| at(col) != at(from)),
        "a divider between the pane and the card"
    );
}

#[test]
fn a_narrow_window_keeps_the_keys_as_buttons_in_the_card() {
    let mut app = lightning_keys(window(1000, 800), 1);
    let k = key_at(&app, 0);
    assert!(crate::common::offered(&mut app, Action::LKey(k)));
    let (kx, ky, ..) = app.hit_box(Action::LKey(k)).expect("the key's button");
    let (ax, ay, ..) = app.hit_box(Action::LAezeed).expect("the aezeed's button");
    assert!(
        kx == ax || ky == ay,
        "in a column or a line with the card's"
    );
    // Not a pane: nothing to its left, the buttons wrap in the card.
    let (cx, ..) = app
        .hit_box(Action::LCln(Some(k)))
        .expect("Core Lightning's");
    assert!(cx >= ax.min(kx), "no row stands left of the card");
}

#[test]
fn the_page_opened_from_convert_has_no_pane() {
    let mut app = wide_window();
    let c = faraday_core::testkit::compute();
    app.session
        .keys
        .add_words(&c, ABANDON, "Seed", None)
        .unwrap();
    app.press(Action::Convert(
        faraday_core::convert::ConvertAction::Aezeed,
    ));
    assert_eq!(app.screen, Screen::Lightning);
    let _ = app.frame();
    assert!(!app.offers(Action::LKey(key_at(&app, 0))));
}

#[test]
fn a_page_opened_from_a_scrolled_tools_page_starts_at_the_top() {
    for open in [Action::Lightning, Action::Explore] {
        let mut app = lightning_keys(window(1366, 500), 20);
        app.press(Action::Nav(Screen::Tools));
        let (w, h) = app.size();
        for _ in 0..30 {
            app.event(Event::Scroll {
                x: (w * 0.5) as u16,
                y: (h / 2.0) as u16,
                dy: 120,
            });
            let _ = app.frame();
        }
        app.press(open);
        let _ = app.frame();
        let first = key_at(&app, 0);
        let a = if open == Action::Lightning {
            Action::LKey(first)
        } else {
            Action::XKey(first)
        };
        let (_, y, ..) = app.hit_box(a).expect("the first row");
        assert!(
            f32::from(y) < h / 2.0,
            "the first row is in view at the top"
        );
    }
}

/// Scrolls to the page's end, then asks that the outline's foot (a
/// padding below the pane's last row) stands in the window.
fn foot_in_view(app: &mut Faraday, last: Action) {
    let (w, h) = app.size();
    let _ = app.frame();
    for _ in 0..200 {
        app.event(Event::Scroll {
            x: (w * 0.5) as u16,
            y: (h / 2.0) as u16,
            dy: 120,
        });
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    }
    for _ in 0..60 {
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    }
    let (_, y, _, bh) = app.hit_box(last).expect("the last row");
    assert!(
        f32::from(y + bh) + 2.0 * faraday_draw::tokens::PAD <= h,
        "the foot is below the window: row ends {} of {h}",
        y + bh
    );
}

#[test]
fn the_outline_foot_can_be_scrolled_into_view() {
    let mut app = window(1366, 768);
    add_keys(&mut app, 40);
    app.press(Action::Explore);
    let last = key_at(&app, 39);
    foot_in_view(&mut app, Action::XKey(last));

    let mut app = lightning_many(20);
    let last = key_at(&app, 19);
    foot_in_view(&mut app, Action::LCln(Some(last)));
}

#[test]
fn the_page_opened_from_a_scrolled_convert_page_starts_at_the_top() {
    let mut app = window(1366, 300);
    add_keys(&mut app, 1);
    app.screen = Screen::Convert;
    app.list_offset = 400.0;
    app.press(Action::Convert(
        faraday_core::convert::ConvertAction::Aezeed,
    ));
    assert_eq!(app.screen, Screen::Lightning);
    assert_eq!(app.list_offset, 0.0);
}
