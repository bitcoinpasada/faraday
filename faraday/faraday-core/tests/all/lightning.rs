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
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no {name}"));
    assert!(!item.secret);
    assert_eq!(png_text(&item.bytes), NODE_KEY);
}
