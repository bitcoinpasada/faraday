//! BIP-44/49/84/86 account vectors for the "abandon … about" mnemonic,
//! SLIP-132 encoding, descriptors and address search.

mod common;

use std::str::FromStr;

use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::{Address, NetworkKind, base58};
use osk_bip::account::AccountXpub;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::descriptor::verify_checksum;
use osk_bip::keys::{Error, Fingerprint, MasterKey, Network, ScriptType};
use osk_bip::slip132;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn master(network: Network) -> MasterKey {
    let m = Mnemonic::parse(Language::English, ABANDON).unwrap();
    MasterKey::from_seed(&m.to_seed(b"").unwrap(), network)
}

fn addr(s: &str, network: Network) -> Address {
    Address::from_str(s)
        .unwrap()
        .require_network(network.into())
        .unwrap()
}

fn check_addresses(account: &AccountXpub, receive: &[&str], change: &[&str]) {
    let network = account.network();
    for (i, &expected) in receive.iter().enumerate() {
        let a = account.address(false, i as u32).unwrap();
        assert_eq!(a, addr(expected, network), "receive {i}");
        assert_eq!(a.to_string(), expected);
    }
    for (i, &expected) in change.iter().enumerate() {
        assert_eq!(
            account.address(true, i as u32).unwrap(),
            addr(expected, network),
            "change {i}"
        );
    }
}

/// Decodes a SLIP-132 private key by swapping in the xprv/tprv version
/// bytes, and checks it against a derived key.
fn assert_slip132_xprv(s: &str, derived: &osk_bip::keys::DerivedKey, kind: NetworkKind) {
    let mut bytes = base58::decode_check(s).unwrap();
    assert_eq!(bytes.len(), 78);
    let plain: [u8; 4] = match kind {
        NetworkKind::Main => [0x04, 0x88, 0xad, 0xe4],
        NetworkKind::Test => [0x04, 0x35, 0x83, 0x94],
    };
    bytes[..4].copy_from_slice(&plain);
    let xprv = Xpriv::decode(&bytes).unwrap();
    assert_eq!(xprv.private_key, *derived.secret_key());
    let secp = bitcoin::secp256k1::Secp256k1::new();
    assert_eq!(Xpub::from_priv(&secp, &xprv), derived.to_xpub());
}

#[test]
fn master_fingerprint() {
    let m = master(Network::Mainnet);
    assert_eq!(m.fingerprint(), Fingerprint([0x73, 0xc5, 0xda, 0x0a]));
    assert_eq!(m.fingerprint().to_hex(), *b"73c5da0a");
    assert_eq!(m.fingerprint().to_string(), "73c5da0a");
    // Independently through rust-bitcoin.
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let mn = Mnemonic::parse(Language::English, ABANDON).unwrap();
    let seed = mn.to_seed(b"").unwrap();
    let xprv = Xpriv::new_master(NetworkKind::Main, seed.expose()).unwrap();
    assert_eq!(xprv.fingerprint(&secp).to_bytes(), m.fingerprint().0);
    assert_eq!(m.xpub().fingerprint().to_bytes(), m.fingerprint().0);
    // The fingerprint does not depend on the network.
    assert_eq!(master(Network::Testnet).fingerprint(), m.fingerprint());
    assert_eq!(master(Network::Signet).fingerprint(), m.fingerprint());
    assert_eq!(master(Network::Regtest).fingerprint(), m.fingerprint());
    // Round trip through rust-bitcoin's type.
    let theirs: bitcoin::bip32::Fingerprint = m.fingerprint().into();
    assert_eq!(Fingerprint::from(theirs), m.fingerprint());
}

#[test]
fn bip44_legacy_mainnet() {
    let m = master(Network::Mainnet);
    let account = m.account_xpub(ScriptType::Legacy, 0).unwrap();
    assert_eq!(
        account.path(),
        &DerivationPath::from_str("m/44h/0h/0h").unwrap()
    );
    assert_eq!(account.script_type(), ScriptType::Legacy);
    assert_eq!(account.network(), Network::Mainnet);
    assert_eq!(account.master_fingerprint(), m.fingerprint());
    check_addresses(&account, &["1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA"], &[]);
    // No SLIP-132 prefix for BIP-44.
    assert_eq!(account.slip132_string(), account.xpub_string());
    assert!(account.xpub_string().starts_with("xpub"));
    let desc = account.descriptor();
    assert!(desc.starts_with("pkh([73c5da0a/44h/0h/0h]xpub"), "{desc}");
    assert!(desc.contains("/<0;1>/*)#"), "{desc}");
    assert!(verify_checksum(&desc));
}

#[test]
fn bip49_nested_segwit_testnet() {
    let m = master(Network::Testnet);
    let account = m.account_xpub(ScriptType::NestedSegwit, 0).unwrap();
    assert_eq!(
        account.path(),
        &DerivationPath::from_str("m/49h/1h/0h").unwrap()
    );
    check_addresses(&account, &["2Mww8dCYPUpKHofjgcXcBCEGmniw9CoaiD2"], &[]);

    let upub = "upub5EFU65HtV5TeiSHmZZm7FUffBGy8UKeqp7vw43jYbvZPpoVsgU93oac7Wk3u6moKegAEWtGNF8DehrnHtv21XXEMYRUocHqguyjknFHYfgY";
    assert_eq!(account.slip132_string(), upub);
    assert!(account.xpub_string().starts_with("tpub"));
    let (decoded, script_type) = slip132::decode_xpub(upub).unwrap();
    assert_eq!(&decoded, account.xpub());
    assert_eq!(script_type, ScriptType::NestedSegwit);
    assert_eq!(decoded.to_string(), account.xpub_string());

    let uprv = "uprv91G7gZkzehuMVxDJTYE6tLivdF8e4rvzSu1LFfKw3b2Qx1Aj8vpoFnHdfUZ3hmi9jsvPifmZ24RTN2KhwB8BfMLTVqaBReibyaFFcTP1s9n";
    assert_slip132_xprv(uprv, &m.derive(account.path()), NetworkKind::Test);
    let root = "uprv8tXDerPXZ1QsVNjUJWTurs9kA1KGfKUAts74GCkcXtU8GwnH33GDRbNJpEqTvipfCyycARtQJhmdfWf8oKt41X9LL1zeD2pLsWmxEk3VAwd";
    assert_slip132_xprv(
        root,
        &m.derive(&DerivationPath::master()),
        NetworkKind::Test,
    );

    let desc = account.descriptor();
    assert!(
        desc.starts_with("sh(wpkh([73c5da0a/49h/1h/0h]tpub"),
        "{desc}"
    );
    assert!(desc.contains("/<0;1>/*))#"), "{desc}");
    assert!(verify_checksum(&desc));

    // Signet and regtest share testnet's key encoding; regtest addresses
    // differ only for bech32.
    let signet = master(Network::Signet)
        .account_xpub(ScriptType::NestedSegwit, 0)
        .unwrap();
    assert_eq!(signet.xpub_string(), account.xpub_string());
    assert_eq!(
        signet.address(false, 0).unwrap().to_string(),
        "2Mww8dCYPUpKHofjgcXcBCEGmniw9CoaiD2"
    );
}

#[test]
fn bip84_native_segwit_mainnet() {
    let m = master(Network::Mainnet);
    let account = m.account_xpub(ScriptType::NativeSegwit, 0).unwrap();
    assert_eq!(
        account.path(),
        &DerivationPath::from_str("m/84h/0h/0h").unwrap()
    );
    check_addresses(
        &account,
        &[
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
            "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g",
        ],
        &["bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el"],
    );

    let zpub = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
    assert_eq!(account.slip132_string(), zpub);
    assert!(account.xpub_string().starts_with("xpub"));
    let (decoded, script_type) = slip132::decode_xpub(zpub).unwrap();
    assert_eq!(&decoded, account.xpub());
    assert_eq!(script_type, ScriptType::NativeSegwit);
    assert_eq!(decoded.encode(), account.xpub().encode());

    let zprv = "zprvAdG4iTXWBoARxkkzNpNh8r6Qag3irQB8PzEMkAFeTRXxHpbF9z4QgEvBRmfvqWvGp42t42nvgGpNgYSJA9iefm1yYNZKEm7z6qUWCroSQnE";
    assert_slip132_xprv(zprv, &m.derive(account.path()), NetworkKind::Main);
    let root_zprv = "zprvAWgYBBk7JR8Gjrh4UJQ2uJdG1r3WNRRfURiABBE3RvMXYSrRJL62XuezvGdPvG6GFBZduosCc1YP5wixPox7zhZLfiUm8aunE96BBa4Kei5";
    assert_slip132_xprv(
        root_zprv,
        &m.derive(&DerivationPath::master()),
        NetworkKind::Main,
    );
    let root_zpub = "zpub6jftahH18ngZxLmXaKw3GSZzZsszmt9WqedkyZdezFtWRFBZqsQH5hyUmb4pCEeZGmVfQuP5bedXTB8is6fTv19U1GQRyQUKQGUTzyHACMF";
    assert_eq!(
        slip132::encode_xpub(&m.xpub(), ScriptType::NativeSegwit),
        root_zpub
    );

    let desc = account.descriptor();
    assert!(desc.starts_with("wpkh([73c5da0a/84h/0h/0h]xpub"), "{desc}");
    assert!(verify_checksum(&desc));

    // Testnet variants use vpub / tb1q; regtest uses bcrt1q.
    let t = master(Network::Testnet)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap();
    assert!(t.slip132_string().starts_with("vpub"));
    assert!(t.address(false, 0).unwrap().to_string().starts_with("tb1q"));
    let r = master(Network::Regtest)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap();
    assert_eq!(r.xpub_string(), t.xpub_string());
    assert!(
        r.address(false, 0)
            .unwrap()
            .to_string()
            .starts_with("bcrt1q")
    );
}

#[test]
fn bip86_taproot_mainnet() {
    let m = master(Network::Mainnet);
    let account = m.account_xpub(ScriptType::Taproot, 0).unwrap();
    assert_eq!(
        account.path(),
        &DerivationPath::from_str("m/86h/0h/0h").unwrap()
    );
    let xpub = "xpub6BgBgsespWvERF3LHQu6CnqdvfEvtMcQjYrcRzx53QJjSxarj2afYWcLteoGVky7D3UKDP9QyrLprQ3VCECoY49yfdDEHGCtMMj92pReUsQ";
    assert_eq!(account.xpub_string(), xpub);
    assert_eq!(
        account.slip132_string(),
        xpub,
        "no SLIP-132 prefix for BIP-86"
    );
    let xprv = "xprv9xgqHN7yz9MwCkxsBPN5qetuNdQSUttZNKw1dcYTV4mkaAFiBVGQziHs3NRSWMkCzvgjEe3n9xV8oYywvM8at9yRqyaZVz6TYYhX98VjsUk";
    assert_eq!(
        Xpriv::from_str(xprv).unwrap().private_key,
        *m.derive(account.path()).secret_key()
    );
    let root = "xpub661MyMwAqRbcFkPHucMnrGNzDwb6teAX1RbKQmqtEF8kK3Z7LZ59qafCjB9eCRLiTVG3uxBxgKvRgbubRhqSKXnGGb1aoaqLrpMBDrVxga8";
    assert_eq!(m.xpub().to_string(), root);

    check_addresses(
        &account,
        &[
            "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
            "bc1p4qhjn9zdvkux4e44uhx8tc55attvtyu358kutcqkudyccelu0was9fqzwh",
        ],
        &["bc1p3qkhfews2uk44qtvauqyr2ttdsw7svhkl9nkm9s9c3x4ax5h60wqwruhk7"],
    );

    let desc = account.descriptor();
    assert_eq!(
        &desc[..desc.len() - 9],
        format!("tr([73c5da0a/86h/0h/0h]{xpub}/<0;1>/*)")
    );
    assert!(verify_checksum(&desc));
}

#[test]
fn other_accounts_and_index_limits() {
    let m = master(Network::Mainnet);
    let a0 = m.account_xpub(ScriptType::NativeSegwit, 0).unwrap();
    let a1 = m.account_xpub(ScriptType::NativeSegwit, 1).unwrap();
    assert_ne!(a0.xpub(), a1.xpub());
    assert_eq!(a1.path(), &DerivationPath::from_str("m/84h/0h/1h").unwrap());
    assert!(
        m.account_xpub(ScriptType::NativeSegwit, 0x7fff_ffff)
            .is_ok()
    );
    assert_eq!(
        m.account_xpub(ScriptType::NativeSegwit, 0x8000_0000).err(),
        Some(Error::IndexOutOfRange)
    );
    assert!(a0.address(false, 0x7fff_ffff).is_ok());
    assert_eq!(
        a0.address(true, 0x8000_0000).err(),
        Some(Error::IndexOutOfRange)
    );
    assert_eq!(
        a0.address(true, u32::MAX).err(),
        Some(Error::IndexOutOfRange)
    );
}

#[test]
fn find_address_scans_both_chains() {
    let m = master(Network::Mainnet);
    let account = m.account_xpub(ScriptType::NativeSegwit, 0).unwrap();
    let receive1 = addr(
        "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g",
        Network::Mainnet,
    );
    let change0 = addr(
        "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el",
        Network::Mainnet,
    );
    assert_eq!(account.find_address(&receive1, 10), Some((false, 1)));
    assert_eq!(account.find_address(&change0, 10), Some((true, 0)));
    assert_eq!(account.find_address(&receive1, 0), None, "beyond max_index");
    assert_eq!(account.find_address(&change0, 0), Some((true, 0)));

    let deep = account.address(true, 37).unwrap();
    assert_eq!(account.find_address(&deep, 36), None);
    assert_eq!(account.find_address(&deep, 37), Some((true, 37)));

    // Wrong account, wrong script type, wrong network: not found.
    let other = m.account_xpub(ScriptType::NativeSegwit, 1).unwrap();
    assert_eq!(other.find_address(&receive1, 50), None);
    let taproot = m.account_xpub(ScriptType::Taproot, 0).unwrap();
    assert_eq!(taproot.find_address(&receive1, 50), None);
    let testnet = master(Network::Testnet)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap();
    assert_eq!(testnet.find_address(&receive1, 50), None);
    // An unrelated address.
    let foreign = addr(
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
        Network::Mainnet,
    );
    assert_eq!(account.find_address(&foreign, 100), None);
    // max_index past the hardened boundary is tolerated.
    assert_eq!(account.find_address(&change0, u32::MAX), Some((true, 0)));
}

#[test]
fn slip132_round_trip_all_types_and_networks() {
    for network in Network::ALL {
        let m = master(network);
        for script_type in ScriptType::ALL {
            let account = m.account_xpub(script_type, 3).unwrap();
            let s = account.slip132_string();
            let prefix = &s[..4];
            let expected_prefix = match (script_type, network.is_mainnet()) {
                (ScriptType::Legacy | ScriptType::Taproot, true) => "xpub",
                (ScriptType::Legacy | ScriptType::Taproot, false) => "tpub",
                (ScriptType::NestedSegwit, true) => "ypub",
                (ScriptType::NestedSegwit, false) => "upub",
                (ScriptType::NativeSegwit, true) => "zpub",
                (ScriptType::NativeSegwit, false) => "vpub",
            };
            assert_eq!(prefix, expected_prefix, "{script_type} on {network}");

            let (decoded, decoded_type) = slip132::decode_xpub(&s).unwrap();
            assert_eq!(decoded.encode(), account.xpub().encode());
            let expected_type = match script_type {
                ScriptType::Taproot => ScriptType::Legacy,
                other => other,
            };
            assert_eq!(decoded_type, expected_type);
            assert_eq!(
                slip132::version_bytes(script_type, network.kind()),
                base58::decode_check(&s).unwrap()[..4]
            );

            // The plain form decodes too and always reports Legacy.
            let (plain, plain_type) = slip132::decode_xpub(&account.xpub_string()).unwrap();
            assert_eq!(&plain, account.xpub());
            assert_eq!(plain_type, ScriptType::Legacy);
        }
    }
}

#[test]
fn slip132_decode_errors() {
    use slip132::Error as E;
    assert_eq!(
        slip132::decode_xpub("not base58 0OIl").err(),
        Some(E::Base58)
    );
    assert_eq!(slip132::decode_xpub("").err(), Some(E::Base58));
    // Valid base58check, wrong length.
    let short = base58::encode_check(&[1, 2, 3]);
    assert_eq!(slip132::decode_xpub(&short).err(), Some(E::Length(3)));
    // Valid length, unknown version (a private key's version bytes).
    let m = master(Network::Mainnet);
    let mut bytes = m.xpub().encode();
    bytes[..4].copy_from_slice(&[0x04, 0x88, 0xad, 0xe4]);
    assert_eq!(
        slip132::decode_xpub(&base58::encode_check(&bytes)).err(),
        Some(E::UnknownVersion([0x04, 0x88, 0xad, 0xe4]))
    );
    // Right version, corrupt key.
    let mut bytes = m.xpub().encode();
    bytes[45] = 0x04;
    assert!(matches!(
        slip132::decode_xpub(&base58::encode_check(&bytes)),
        Err(E::Key(_))
    ));
}

#[test]
fn coin_types_and_purposes_decide_the_path() {
    assert_eq!(Network::Mainnet.coin_type(), 0);
    for n in [Network::Testnet, Network::Signet, Network::Regtest] {
        assert_eq!(n.coin_type(), 1);
    }
    assert_eq!(ScriptType::ALL.map(ScriptType::purpose), [44, 49, 84, 86]);
}

#[test]
fn passphrase_changes_everything() {
    let mn = Mnemonic::parse(Language::English, ABANDON).unwrap();
    let a = MasterKey::from_seed(&mn.to_seed(b"").unwrap(), Network::Mainnet);
    let b = MasterKey::from_seed(&mn.to_seed(b"TREZOR").unwrap(), Network::Mainnet);
    assert_ne!(a.fingerprint(), b.fingerprint());
    assert_ne!(
        a.account_xpub(ScriptType::Taproot, 0).unwrap().xpub(),
        b.account_xpub(ScriptType::Taproot, 0).unwrap().xpub()
    );
}

/// `MasterKey` and `DerivedKey` must not implement `Debug`, `Clone` or
/// `Display`; `AccountXpub` may. Checked with the inherent-shadows-trait
/// trick: the inherent `IMPLS` is chosen only when its bound holds.
mod no_leaky_traits {
    use core::fmt::{Debug, Display};
    use core::marker::PhantomData;

    pub trait DoesNotImpl {
        const IMPLS: bool = false;
    }
    impl<T> DoesNotImpl for T {}

    pub struct IsDebug<T>(PhantomData<T>);
    impl<T: Debug> IsDebug<T> {
        pub const IMPLS: bool = true;
    }
    pub struct IsClone<T>(PhantomData<T>);
    impl<T: Clone> IsClone<T> {
        pub const IMPLS: bool = true;
    }
    pub struct IsDisplay<T>(PhantomData<T>);
    impl<T: Display> IsDisplay<T> {
        pub const IMPLS: bool = true;
    }
}

#[test]
// The probes resolve at compile time; asserting on them is the point.
#[allow(clippy::assertions_on_constants)]
fn secret_key_types_have_no_debug_clone_or_display() {
    use no_leaky_traits::{DoesNotImpl, IsClone, IsDebug, IsDisplay};
    use osk_bip::keys::DerivedKey;

    // Sanity: the probe reports true for a type that does implement.
    assert!(<IsDebug<AccountXpub>>::IMPLS);
    assert!(<IsClone<AccountXpub>>::IMPLS);
    assert!(<IsDisplay<Fingerprint>>::IMPLS);
    assert!(!<IsDisplay<AccountXpub>>::IMPLS);

    assert!(!<IsDebug<MasterKey>>::IMPLS);
    assert!(!<IsClone<MasterKey>>::IMPLS);
    assert!(!<IsDisplay<MasterKey>>::IMPLS);
    assert!(!<IsDebug<DerivedKey>>::IMPLS);
    assert!(!<IsClone<DerivedKey>>::IMPLS);
    assert!(!<IsDisplay<DerivedKey>>::IMPLS);
}
