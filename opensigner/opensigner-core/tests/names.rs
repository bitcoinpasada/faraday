//! Wallet names: what a person types so that two wallets of one shape
//! read apart, where the name shows, and what it replaces.

mod common;

use common::{ABANDON, Element, Harness, PANEL, PIN, SECURE_PHONE};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_shell_api::{Event, FileKind, Key};

/// The 2-of-3 wallet of the loaded key and two fixed cosigners.
const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");
/// The same wallet as a coordinator's text config, whose `Name:` line
/// is "OpenSignerKit demo".
const CONFIG: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.txt");
/// That config's name.
const CONFIG_NAME: &str = "OpenSignerKit demo";

/// A single-key wallet over the test key's own account, on mainnet.
const WALLET: &str = "wpkh([73c5da0a/84'/0'/0']xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)";

fn policy() -> WalletPolicy {
    WalletPolicy::parse(POLICY).expect("the committed policy")
}

/// "2 of 3", the shape the 2-of-3 wallet's label begins with.
fn quorum() -> String {
    strings::fill(strings::EN.wallet_quorum, &["2", "3"])
}

/// Add › "Scan a wallet", `bytes` read from a file there, and the
/// wallet put in use.
fn use_wallet(h: &mut Harness, bytes: &[u8]) {
    h.go_home();
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: bytes.to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
}

/// Types `name` into the Name field of the wallet page that is on, over
/// whatever was there.
fn name_it(h: &mut Harness, name: &str) {
    h.tap(ids::WALLET_NAME);
    assert_eq!(h.app.screen(), ScreenKind::WalletName);
    for _ in 0..32 {
        h.key(Key::Backspace);
    }
    h.type_text(name);
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
}

/// Every row label Wallets draws, which is the kind and the network.
fn home_labels(h: &mut Harness) -> Vec<String> {
    h.open_wallets();
    h.app
        .row_glyphs()
        .into_iter()
        .map(|(label, _)| label)
        .collect()
}

/// Everything Wallets draws, which includes each row's value: what the
/// wallet is called.
fn home_texts(h: &mut Harness) -> Vec<String> {
    h.open_wallets();
    h.app.texts()
}

/// The bold line of a wallet's row is what the wallet is called: its
/// name where it has one, its shape where it has none. The label above
/// is the kind and the network, and the checksum is on neither line.
#[test]
fn a_named_wallet_reads_by_its_name_and_its_shape() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    assert_eq!(h.app.network(), Network::Regtest);
    use_wallet(&mut h, POLICY.as_bytes());
    let checksum = policy().checksum();

    name_it(&mut h, "Savings");
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.wallet_name) && texts.iter().any(|t| t == "Savings"),
        "the page states the name: {texts:?}"
    );

    let texts = home_texts(&mut h);
    assert!(
        texts.iter().any(|t| t == "Savings"),
        "the row reads by its name: {texts:?}"
    );
    let labels = home_labels(&mut h);
    assert!(
        labels
            .iter()
            .any(|l| l.starts_with(strings::EN.wallet_kind_multisig)),
        "the kind and the network are the label: {labels:?}"
    );
    assert!(
        !texts.contains(&checksum),
        "the page states the checksum, so Home does not: {texts:?}"
    );

    // An empty name gives the wallet back its shape.
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    name_it(&mut h, "");
    let texts = home_texts(&mut h);
    assert!(
        texts.iter().any(|t| t.starts_with(&quorum())),
        "the shape is what it is called again: {texts:?}"
    );
    assert!(
        !texts.contains(&checksum),
        "and the checksum is still nowhere on Home: {texts:?}"
    );
}

/// A single-sig wallet added by hand takes a name the same way, and its
/// fingerprint leaves the row.
#[test]
fn a_single_sig_wallet_takes_a_name_too() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_single_sig(0);
    name_it(&mut h, "Everyday");
    let texts = home_texts(&mut h);
    assert!(texts.iter().any(|t| t == "Everyday"), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t.starts_with("73c5da0a")),
        "the name replaced the fingerprint: {texts:?}"
    );
}

/// A coordinator's config names the wallet it registers.
#[test]
fn a_configs_name_line_is_the_wallets_name() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    use_wallet(&mut h, CONFIG.as_bytes());
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == CONFIG_NAME),
        "the page states the name the file gave it: {texts:?}"
    );
    let texts = home_texts(&mut h);
    assert!(texts.iter().any(|t| t == CONFIG_NAME), "{texts:?}");
}

/// A device that keeps keys keeps the names with the wallets, so a
/// restart brings back what the person typed.
#[test]
fn a_name_comes_back_after_a_lock_and_an_unlock() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept());
    use_wallet(&mut h, WALLET.as_bytes());
    name_it(&mut h, "Savings");
    // The Taproot wallet of the same key is a second wallet, and it
    // takes a name of its own.
    h.go_home();
    h.add_single_sig(0, 3);
    name_it(&mut h, "Everyday");
    h.go_home();

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    let texts = home_texts(&mut h);
    assert!(
        texts.iter().any(|t| t == "Savings") && texts.iter().any(|t| t == "Everyday"),
        "both names came back: {texts:?}"
    );
}

/// A blob written before names existed carries slots that are policy
/// text and nothing else, and it still opens.
#[test]
fn a_slot_with_no_name_line_is_a_wallet_with_no_name() {
    let policy = policy();
    let text = policy.to_text();
    let mut body = vec![0u8; osk_keep::wallets::BODY_LEN];
    body[0] = 1;
    body[1..1 + text.len()].copy_from_slice(text.as_bytes());

    let (wallets, names) = osk_keep::wallets::wallets(&body);
    assert_eq!(wallets, vec![policy.clone()]);
    assert_eq!(names.policy_name(&policy), None);
}
