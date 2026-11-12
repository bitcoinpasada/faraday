//! The forms a test seed can be held in (`docs/SECRET-LEAKS.md` §3),
//! worked out with faraday-bip's own functions: what the leak scan looks
//! for. Every secret here is a published test vector and long enough that
//! no form of it matches by chance.

use std::str::FromStr;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_bip::keys::{MasterKey, Network};
use faraday_core::testkit;
use faraday_leak_scan::Form;
use unicode_normalization::UnicodeNormalization;

/// The 24-word BIP-39 test vector whose entropy is
/// `f585c11aec520db57dd353c69554b21a89b20fb0650966fa0a9d6f74fd989d8f`
/// (the Trezor vectors).
pub const PHRASE: &str = "void come effort suffer camp survey warrior heavy shoot primary clutch crush open amazing screen patrol group space point ten exist slush involve unfold";

/// A long, unusual passphrase: printable ASCII, as the device takes one.
pub const PASSPHRASE: &str = "Quixotic-Marmot-Anvil-7731-Ember-Kestrel-Lattice-902";

/// A vault's passphrase, as typed at its unlock.
pub const VAULT_PASSPHRASE: &str = "Velvet-Sundial-Parsnip-4408-Obelisk-Thrush-Quill";

/// Every form of the seed made of `PHRASE` with `passphrase` (empty for
/// none), and of the phrase and the passphrase themselves.
pub fn seed_forms(passphrase: &str) -> Vec<Form> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, PHRASE).expect("the vector's words");
    let mut forms = vec![Form::new("phrase text", PHRASE.as_bytes().to_vec())];
    let indices = m.indices();
    forms.push(Form::new(
        "indices (u16, little endian)",
        indices.iter().flat_map(|i| i.to_le_bytes()).collect(),
    ));
    forms.push(Form::new(
        "indices (u16, big endian)",
        indices.iter().flat_map(|i| i.to_be_bytes()).collect(),
    ));
    let entropy = m.entropy();
    let entropy = entropy.expose().as_bytes().to_vec();
    assert_eq!(
        entropy[..4],
        [0xf5, 0x85, 0xc1, 0x1a],
        "the vector's entropy"
    );
    let (check, bits) = m.checksum_bits();
    let mut packed = entropy.clone();
    packed.push(check << (8 - bits));
    forms.push(Form::new("entropy", entropy));
    forms.push(Form::new("entropy with its checksum bits", packed));
    let seed = m
        .to_seed(&c, passphrase.as_bytes())
        .expect("an ASCII passphrase");
    forms.push(Form::new("BIP-39 seed", seed.expose().to_vec()));
    let master = MasterKey::from_seed(&c, &seed, Network::Mainnet);
    for (network, name, text) in [
        (Network::Mainnet, "master key as xprv", "xprv text"),
        (Network::Testnet, "master key as tprv", "tprv text"),
    ] {
        let key = master.for_network(network);
        let xprv = Xpriv::from_str(key.xpriv_ascii().as_str()).expect("its own text");
        if network == Network::Mainnet {
            forms.push(Form::new(
                "master private key",
                xprv.private_key.secret_bytes().to_vec(),
            ));
            forms.push(Form::new(
                "master chain code",
                xprv.chain_code.as_bytes().to_vec(),
            ));
        }
        forms.push(Form::new(name, xprv.encode().to_vec()));
        forms.push(Form::new(
            text,
            key.xpriv_ascii().as_str().as_bytes().to_vec(),
        ));
    }
    if !passphrase.is_empty() {
        let nfc: String = passphrase.nfc().collect();
        let nfkd: String = passphrase.nfkd().collect();
        forms.push(Form::new("passphrase (NFC)", nfc.into_bytes()));
        forms.push(Form::new("passphrase (NFKD)", nfkd.into_bytes()));
    }
    forms
}

/// The vault passphrase as a form, in NFC and NFKD.
pub fn vault_passphrase_forms() -> Vec<Form> {
    vec![
        Form::new(
            "vault passphrase (NFC)",
            VAULT_PASSPHRASE.nfc().collect::<String>().into_bytes(),
        ),
        Form::new(
            "vault passphrase (NFKD)",
            VAULT_PASSPHRASE.nfkd().collect::<String>().into_bytes(),
        ),
    ]
}
