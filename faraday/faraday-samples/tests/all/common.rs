//! What the test modules share.

use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_bip::bitcoin::{Network, NetworkKind};
use faraday_bip::keys::Network as KeyNetwork;
use faraday_core::samples::SampleKey;
use faraday_core::testkit;
use faraday_core::wallet::Session;
use faraday_core::{DESKTOP_MODES, Faraday};
use faraday_samples::SAMPLES;
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

pub const PASSPHRASE: &str = "faraday";

/// The sample whose parts are exactly `name`'s, the `n`th of that name.
pub fn key(name: &str) -> &'static SampleKey {
    SAMPLES
        .keys
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("no sample named {name}"))
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A vector file of `tools/vectors`, read when the test runs.
pub fn vector(path: &str) -> String {
    let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors")
        .join(path);
    std::fs::read_to_string(&at).unwrap_or_else(|e| panic!("{}: {e}", at.display()))
}

pub fn json(path: &str) -> serde_json::Value {
    serde_json::from_str(&vector(path)).expect("a json vector file")
}

/// The fingerprint of the BIP-32 master key of `seed`, by the `bitcoin`
/// crate alone: the first four bytes of the key's identifier.
#[expect(
    clippy::disallowed_methods,
    reason = "another implementation's master key, to check Faraday's against (docs/COMPUTE.md §7)"
)]
pub fn fingerprint_of_seed(seed: &[u8]) -> [u8; 4] {
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(NetworkKind::Main, seed).expect("a master key");
    master.fingerprint(&secp).to_bytes()
}

/// The master key of `seed`, written as an `xprv`, by the `bitcoin` crate.
#[expect(
    clippy::disallowed_methods,
    reason = "another implementation's master key, to check Faraday's against (docs/COMPUTE.md §7)"
)]
pub fn xprv_of_seed(seed: &[u8]) -> String {
    Xpriv::new_master(Network::Bitcoin, seed)
        .expect("a master key")
        .to_string()
}

/// The fingerprint BIP-39 `words` have with `passphrase`, by the `bip39`
/// crate and the `bitcoin` crate, with none of Faraday's code. `words` are
/// in `lang`, in the form (published or NFKD) the crate reads.
pub fn reference_fingerprint(lang: bip39::Language, words: &str, passphrase: &str) -> [u8; 4] {
    let m = bip39::Mnemonic::parse_in_normalized(lang, words).expect("valid words");
    fingerprint_of_seed(&m.to_seed_normalized(passphrase))
}

/// The entropy a `bip39` mnemonic encodes.
pub fn entropy_of(m: &bip39::Mnemonic) -> Vec<u8> {
    let (bytes, len) = m.to_entropy_array();
    bytes[..len].to_vec()
}

/// `app`'s display, `width` by `height` pixels at `dpi`.
pub fn show(app: &mut Faraday, width: u16, height: u16, dpi: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// The desktop's app: online, in Demo, with the samples set, in a window
/// tall enough to show every row of the sheet at once (a row out of view
/// offers no press).
pub fn demo_app() -> Faraday {
    let mut app = testkit::started();
    show(&mut app, 1280, 4000, 160);
    app.online = true;
    app.seed_line = true;
    app.session = Session::on(KeyNetwork::Testnet);
    app.session.set_demo(&SAMPLES);
    app.samples = Some(&SAMPLES);
    app.set_modes(DESKTOP_MODES);
    app
}

/// `[fingerprint/path]tpub` of the BIP-39 `words` (no passphrase) at the
/// hardened `path` (as `48h/1h/0h/2h`), by the `bip39` crate and the
/// `bitcoin` crate alone, on a test network.
#[expect(
    clippy::disallowed_methods,
    reason = "another implementation's derivation, to check Faraday's against (docs/COMPUTE.md §7)"
)]
pub fn reference_key_text(words: &str, path: &str) -> String {
    use faraday_bip::bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
    let m = bip39::Mnemonic::parse_in_normalized(bip39::Language::English, words).expect("words");
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(NetworkKind::Test, &m.to_seed_normalized("")).expect("master");
    let at: DerivationPath = format!("m/{path}").parse().expect("path");
    let key = Xpub::from_priv(&secp, &master.derive_priv(&secp, &at).expect("derive"));
    format!("[{}/{path}]{key}", master.fingerprint(&secp))
}

/// What Faraday Desktop's host strip offers for `app` as it stands, its
/// demo aids among them (`docs/DESKTOP-HOST.md` §3.1): the app held by
/// the host for one frame and given back.
pub fn strip(app: &mut Faraday) -> Vec<String> {
    faraday_core::Host::strip_for(app)
}

/// Whether the host strip (or its More menu) offers the press `a` for
/// `app` as it stands (`docs/DESKTOP-HOST.md` §3.1).
pub fn strip_offers(app: &mut Faraday, a: faraday_core::Action) -> bool {
    // The More menu is drawn open by its clock: at the very start of a
    // test's time it has not opened yet, so the clock moves on first.
    if app.now_ms() < 1000 {
        app.event(Event::Tick { now_ms: 1000 });
    }
    faraday_core::Host::strip_actions_for(app).contains(&a)
}
