//! A Lightning node's key: an LND cipher seed and a loaded key give the
//! node ids LND and ldk-node give them (`tools/reference/aezeed/`), and
//! the private key leaves only through the vault or the secret sheet.

use faraday_core::{Action, Faraday, Screen, Sheet};
use osk_shell_api::{App, Event, Key};

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
    let l = app.lightning.as_ref().unwrap();
    let n = l.node.as_ref().unwrap_or_else(|| panic!("{:?}", l.error));
    assert_eq!(hex(&n.public), NODE_KEY);
    assert!(n.birthday.is_some());
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
    let fp = app.session.keys[0].master.fingerprint().0;
    app.press(Action::LKey(fp));
    let n = app.lightning.as_ref().unwrap().node.as_ref().unwrap();
    assert_eq!(
        hex(&n.public),
        "027cf7c81dc777e46572ff964f17650a0f6f783319fcc140fb9a69425e2fbdc93c"
    );
}
