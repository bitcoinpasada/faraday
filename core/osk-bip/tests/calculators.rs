//! What the Tools calculators and a scanned document's review work out
//! from text: the hashes of some bytes, a descriptor's checksum, script
//! type and key origins, and every spelling of an extended public key.

use bitcoin::NetworkKind;
use bitcoin::hex::FromHex;
use osk_bip::descriptor::{checksum_facts, origin_fingerprints, script_type};
use osk_bip::hashes::hashes;
use osk_bip::keys::{Network, ScriptType};
use osk_bip::slip132::{KeyReading, decode_xpub, encode_xpub, key_facts, network_kind};

/// BIP-32 test vector 1: the master extended public key of seed
/// `000102030405060708090a0b0c0d0e0f`.
const VECTOR_1_XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";

/// The same vector's master public key, compressed.
const VECTOR_1_PUBKEY: &str = "0339a36013301597daef41fbe593a02cc513d0b55527ec2df1050e2e8ff49c85c2";

/// BIP-32 test vector 1: the identifier of that key, which is HASH160
/// of the public key above. Its first four bytes are the fingerprint
/// `3442193e` the vector states.
const VECTOR_1_IDENTIFIER: &str = "3442193e1bb70916e914552172cd4e2dbc9df811";

/// Hex of some bytes, for comparing against a published vector.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// FIPS 180-4 and BIP-32: the hashes a person compares against a
/// published value.
#[test]
fn the_hashes_match_the_published_vectors() {
    let abc = hashes(b"abc");
    assert_eq!(
        hex(&abc.sha256),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let empty = hashes(b"");
    assert_eq!(
        hex(&empty.sha256),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&empty.sha256d),
        "5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456"
    );
    let key = Vec::<u8>::from_hex(VECTOR_1_PUBKEY).unwrap();
    assert_eq!(hex(&hashes(&key).hash160), VECTOR_1_IDENTIFIER);
}

/// BIP-380: the checksum of a descriptor, and the verdict on the one it
/// arrived with.
#[test]
fn the_descriptor_checksum_matches_bip_380() {
    let f = checksum_facts("raw(deadbeef)").expect("a descriptor");
    assert_eq!(f.with_checksum, "raw(deadbeef)#89f8spxm");
    assert_eq!(f.given, None);

    let holds = checksum_facts("raw(deadbeef)#89f8spxm").expect("a descriptor");
    assert_eq!(
        holds.given.as_ref().map(|(g, ok)| (g.as_str(), *ok)),
        Some(("89f8spxm", true))
    );

    // BIP-380's invalid examples: a checksum of the wrong length, and
    // one whose characters are not the ones this descriptor produces.
    for wrong in ["raw(deadbeef)#89f8spxmx", "raw(deadbeef)#89f8spxn"] {
        let f = checksum_facts(wrong).expect("a descriptor");
        assert_eq!(f.with_checksum, "raw(deadbeef)#89f8spxm");
        assert!(!f.given.expect("a checksum arrived").1, "{wrong} held");
    }

    // Only a descriptor this device could derive from is one it offers
    // to load: a raw script is a descriptor and not a wallet.
    assert!(
        checksum_facts("raw(deadbeef)")
            .expect("a descriptor")
            .wallet
            .is_none()
    );
    let wpkh = format!("wpkh([3442193e/84h/0h/0h]{VECTOR_1_XPUB}/<0;1>/*)");
    assert!(
        checksum_facts(&wpkh)
            .expect("a descriptor")
            .wallet
            .is_some()
    );
}

/// SLIP-132: one key, five spellings, and nothing invented. The key
/// material is the same in each, so every spelling decodes back to the
/// key that was typed.
#[test]
fn one_key_is_shown_in_every_spelling() {
    let KeyReading::Public(f) = key_facts(VECTOR_1_XPUB, Network::Mainnet) else {
        panic!("the BIP-32 vector 1 master key is an extended public key");
    };
    assert_eq!(f.bip32, VECTOR_1_XPUB);
    assert_eq!(f.depth, 0);
    assert_eq!(f.fingerprint, "3442193e");
    assert_eq!(f.child, "0");
    assert_eq!(f.network, Network::Mainnet);
    let prefixes = ["xpub", "ypub", "zpub", "xpub"];
    for ((_, spelling), prefix) in f.slip132.iter().zip(prefixes) {
        assert!(spelling.starts_with(prefix), "{spelling} is not a {prefix}");
        let (decoded, _) = decode_xpub(spelling).expect("a key");
        assert_eq!(
            hex(&decoded.public_key.serialize()),
            VECTOR_1_PUBKEY,
            "{spelling} is another key"
        );
    }

    // A private key is refused: the key explorer is where those are
    // seen.
    let xprv = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
    assert_eq!(key_facts(xprv, Network::Mainnet), KeyReading::Private);
    assert_eq!(key_facts("nonsense", Network::Mainnet), KeyReading::None);
}

/// A descriptor's outer function names its script type, and its key
/// origins name the masters its keys come from: BIP-389's two-key
/// example and a single-key `tr`.
#[test]
fn a_descriptor_states_its_script_type_and_its_masters() {
    let wsh = format!(
        "wsh(sortedmulti(2,[3442193e/48h/0h/0h/2h]{VECTOR_1_XPUB}/<0;1>/*,[73c5da0a/48h/0h/0h/2h]{VECTOR_1_XPUB}/<2;3>/*))"
    );
    assert_eq!(script_type(&wsh), Some(ScriptType::NativeSegwit));
    assert_eq!(origin_fingerprints(&wsh), ["3442193e", "73c5da0a"]);

    assert_eq!(
        script_type(&format!(
            "sh(wpkh([3442193e/49h/0h/0h]{VECTOR_1_XPUB}/0/*))"
        )),
        Some(ScriptType::NestedSegwit)
    );
    assert_eq!(
        script_type(&format!("tr([3442193e]{VECTOR_1_XPUB}/0/*)")),
        Some(ScriptType::Taproot)
    );
    assert_eq!(
        origin_fingerprints(&format!("tr([3442193e]{VECTOR_1_XPUB}/0/*)")),
        ["3442193e"]
    );
    assert_eq!(script_type("raw(deadbeef)"), None);
    assert!(origin_fingerprints("raw(deadbeef)").is_empty());
}

/// An extended public key's version bytes say whether it is a mainnet
/// or a test-network key, in the BIP-32 spelling and in every SLIP-132
/// one. The test networks share their version bytes, so a test key
/// read while set to mainnet is called testnet, and while set to
/// signet, signet.
#[test]
fn an_extended_key_says_which_chain_it_is_for() {
    let KeyReading::Public(f) = key_facts(VECTOR_1_XPUB, Network::Mainnet) else {
        panic!("the BIP-32 vector 1 master key is an extended public key");
    };
    for (_, spelling) in &f.slip132 {
        assert_eq!(network_kind(spelling), Some(NetworkKind::Main));
    }

    let (mut xpub, _) = decode_xpub(VECTOR_1_XPUB).unwrap();
    xpub.network = NetworkKind::Test;
    let tpub = encode_xpub(&xpub, ScriptType::Legacy);
    assert!(tpub.starts_with("tpub"));
    assert_eq!(network_kind(&tpub), Some(NetworkKind::Test));
    for (setting, called) in [
        (Network::Mainnet, Network::Testnet),
        (Network::Signet, Network::Signet),
    ] {
        let KeyReading::Public(f) = key_facts(&tpub, setting) else {
            panic!("a tpub is an extended public key");
        };
        assert_eq!(f.network, called);
    }
    assert_eq!(network_kind("nonsense"), None);
}
