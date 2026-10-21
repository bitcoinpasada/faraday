//! `faraday_bip::slip132`'s one decoder over all ten public versions, and the
//! rule that a version is taken only where it agrees with the script or
//! path around the key: SLIP-0132's own `abandon … about` vectors, the
//! BIP-48 account keys a Coldcard writes in its multisig versions, and
//! the files other wallets write
//! (`faraday/faraday-core/tests/vectors/interop/`).

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{MasterKey, MultisigScriptType, Network, ScriptType};
use faraday_bip::slip132::{
    self, Error, Mismatch, Script, decode, decode_xpub, encode, key_line, plain_descriptor,
};
use faraday_crypto::work::Compute;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn master(network: Network) -> MasterKey {
    let c = Compute::unchecked();
    let m = Mnemonic::parse(&c, Language::English, ABANDON).unwrap();
    MasterKey::from_seed(&c, &m.to_seed(&c, b"").unwrap(), network)
}

/// SLIP-0132's test vectors for `abandon … about`, account 0.
const ZPUB: &str = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
const YPUB: &str = "ypub6Ww3ibxVfGzLrAH1PNcjyAWenMTbbAosGNB6VvmSEgytSER9azLDWCxoJwW7Ke7icmizBMXrzBx9979FfaHxHcrArf3zbeJJJUZPf663zsP";

/// The BIP-48 account keys a Coldcard writes for `abandon … about`
/// (`core/faraday-bip/src/coldcard.rs`'s test export).
const ZPUB_MULTI: &str = "Zpub74Jru6aftwwHxCUCWEvP6DgrfFsdA4U6ZRtQ5i8qJpMcC39yZGv3egBhQfV3MS9pZtH5z8iV5qWkJsK6ESs6mSzt4qvGhzJxPeeVS2e1zUG";
const YPUB_MULTI: &str = "Ypub6jUbbRukkGPp4DgJDD4HL2NKkSZ1UPk111mg59XtJRQZHvJ6XqvJzrntik9U4jCFQkgrBqevdKLPMdYZXU9KAGhKpMhW5XujwqiQ7Csmm4Z";

#[test]
fn every_version_decodes_to_the_account_key_it_names() {
    let c = Compute::unchecked();
    let m = master(Network::Mainnet);
    let account = |script| m.account_xpub(&c, script, 0).unwrap().xpub().to_string();
    let multisig = |script| {
        m.multisig_account_xpub(&c, script, 0)
            .unwrap()
            .xpub()
            .to_string()
    };
    for (text, plain, named) in [
        (
            ZPUB,
            account(ScriptType::NativeSegwit),
            Some(Script::NativeSegwit),
        ),
        (
            YPUB,
            account(ScriptType::NestedSegwit),
            Some(Script::NestedSegwit),
        ),
        (
            ZPUB_MULTI,
            multisig(MultisigScriptType::NativeSegwit),
            Some(Script::NativeMultisig),
        ),
        (
            YPUB_MULTI,
            multisig(MultisigScriptType::NestedSegwit),
            Some(Script::NestedMultisig),
        ),
    ] {
        let k = decode(text).unwrap();
        assert_eq!(k.plain, plain, "{text}");
        assert_eq!(k.named, named, "{text}");
        assert_eq!(k.xpub.to_string(), plain);
        let script = named.unwrap();
        assert_eq!(encode(&plain, script).as_deref(), Some(text));
    }
    // BIP-32's own version names no script: it is not Legacy.
    let xpub = account(ScriptType::Legacy);
    assert_eq!(decode(&xpub).unwrap().named, None);
    assert_eq!(decode(&format!("  {xpub}\n")).unwrap().plain, xpub);
    // Upstream's single-key decoder still calls it Legacy, and still
    // takes no multisig version.
    assert_eq!(decode_xpub(&xpub).unwrap().1, ScriptType::Legacy);
    assert!(matches!(
        decode_xpub(ZPUB_MULTI),
        Err(Error::UnknownVersion(_))
    ));
}

#[test]
fn the_test_network_versions_decode_to_tpub() {
    let c = Compute::unchecked();
    let m = master(Network::Testnet);
    let tpub = m
        .multisig_account_xpub(&c, MultisigScriptType::NativeSegwit, 0)
        .unwrap()
        .xpub()
        .to_string();
    assert!(tpub.starts_with("tpub"));
    for (script, prefix) in [
        (Script::NativeSegwit, "vpub"),
        (Script::NestedSegwit, "upub"),
        (Script::NativeMultisig, "Vpub"),
        (Script::NestedMultisig, "Upub"),
    ] {
        let spelled = encode(&tpub, script).unwrap();
        assert!(spelled.starts_with(prefix), "{spelled}");
        let k = decode(&spelled).unwrap();
        assert_eq!((k.plain.as_str(), k.named), (tpub.as_str(), Some(script)));
    }
    assert_eq!(decode(&tpub).unwrap().named, None);
    // A type SLIP-132 does not name leaves the key as it is.
    assert_eq!(
        encode(&tpub, Script::Taproot).as_deref(),
        Some(tpub.as_str())
    );
}

#[test]
fn a_version_that_disagrees_with_its_path_or_script_is_a_mismatch() {
    assert!(matches!(
        key_line(&format!("[73c5da0a/84'/0'/0']{ZPUB}")),
        Some(Ok(_))
    ));
    assert_eq!(
        key_line(&format!("[73c5da0a/49h/0h/0h]{ZPUB}")),
        Some(Err(Mismatch {
            key: ZPUB.into(),
            named: Script::NativeSegwit,
            around: Script::NestedSegwit,
        }))
    );
    assert!(matches!(
        key_line(&format!("[73C5DA0A/48'/0'/0'/1']{ZPUB_MULTI}")),
        Some(Err(Mismatch {
            named: Script::NativeMultisig,
            around: Script::NestedMultisig,
            ..
        }))
    ));
    // An origin with no purpose this list names tells nothing; the
    // origin is written in lower case.
    let line = key_line(&format!("[73C5DA0A/0h]{ZPUB}")).unwrap().unwrap();
    assert!(line.text.starts_with("[73c5da0a/0h]xpub"));
    assert_eq!(key_line("not a key"), None);

    let wsh = format!("wsh(sortedmulti(1,[73c5da0a/48h/0h/0h/2h]{ZPUB_MULTI}/<0;1>/*))");
    let plain = plain_descriptor(&wsh).unwrap();
    assert!(!slip132::mentions(&plain), "{plain}");
    for (descriptor, named, around) in [
        (
            format!("wpkh([73c5da0a/84h/0h/0h]{ZPUB_MULTI}/<0;1>/*)"),
            Script::NativeMultisig,
            Script::NativeSegwit,
        ),
        (
            format!("sh(wsh(sortedmulti(1,[73c5da0a/48h/0h/0h/2h]{ZPUB_MULTI}/<0;1>/*)))"),
            Script::NativeMultisig,
            Script::NestedMultisig,
        ),
        (
            format!("wsh(sortedmulti(1,[73c5da0a/84h/0h/0h]{ZPUB}/<0;1>/*))"),
            Script::NativeSegwit,
            Script::NativeMultisig,
        ),
    ] {
        let m = plain_descriptor(&descriptor).unwrap_err();
        assert_eq!((m.named, m.around), (named, around), "{descriptor}");
    }
}

/// Casa's descriptor of `Ypub`s (BlueWallet's test strings) is read as
/// the nested multisig it states, with a checksum made for the new text.
#[test]
fn casas_descriptor_of_ypubs_is_its_nested_multisig_in_xpubs() {
    let c = Compute::unchecked();
    let casa = include_str!(
        "../../../../faraday/faraday-core/tests/vectors/interop/bluewallet/casa-descriptor.txt"
    )
    .trim();
    assert!(slip132::mentions(casa));
    let plain = plain_descriptor(casa).unwrap();
    assert!(plain.starts_with("sh(wsh(sortedmulti(3,"), "{plain}");
    assert!(!slip132::mentions(&plain));
    assert_eq!(
        faraday_bip::policy::WalletPolicy::parse_any(&c, &plain)
            .unwrap()
            .keys()
            .len(),
        5
    );
}
