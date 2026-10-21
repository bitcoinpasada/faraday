//! `osk_bip::coldcard` over the files Coldcards write, as BlueWallet's and
//! Sparrow's test suites carry them
//! (`faraday/faraday-core/tests/vectors/interop/`): every key rooted at
//! the file's top-level `xfp`, the stated first address checked, and a
//! SLIP-132 key whose version names another script refused.

use osk_bip::coldcard::{self, Cosigner, ReadError};
use osk_bip::keys::{MultisigScriptType, Network};
use osk_crypto::work::Compute;

const MK4: &str = include_str!(
    "../../../faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardmk4-generic.json"
);
const Q: &str =
    include_str!("../../../faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardQ.json");
const UNCHAINED: &str =
    include_str!("../../../faraday/faraday-core/tests/vectors/interop/bluewallet/unchained.json");
const CCXP: &str = include_str!(
    "../../../faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-keystore-1.json"
);

/// The Mk4's export: the master is `086EE178`; the `xfp` inside each
/// account (`7461A59E` in `bip84`) is the account key's own. The wallet
/// is rooted at the master and its first address is the one the
/// Coldcard states, as Coldcard's own descriptor in the same set has it.
#[test]
fn an_export_is_rooted_at_the_masters_xfp_and_has_its_stated_first_address() {
    let c = Compute::unchecked();
    assert!(coldcard::looks_like_export(MK4));
    let (policy, field) = coldcard::read(&c, MK4).unwrap();
    assert_eq!(field, "bip84");
    let d = policy.to_descriptor();
    assert!(d.starts_with("wpkh([086ee178/84h/0h/0h]xpub"), "{d}");
    assert_eq!(
        policy
            .address_at(&c, Network::Mainnet, false, 0)
            .unwrap()
            .to_string(),
        "bc1q5y4r767v5fzx74ez4nw36hjqrhr4ayeyut5px6"
    );
    let descriptor = include_str!(
        "../../../faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardmk4-descriptor.txt"
    );
    let theirs = osk_bip::policy::WalletPolicy::parse_any(&c, descriptor.trim()).unwrap();
    assert_eq!(theirs.to_descriptor(), d);
    let (q, _) = coldcard::read(&c, Q).unwrap();
    assert!(
        q.to_descriptor()
            .starts_with("wpkh([b68af6e4/84h/0h/0h]xpub")
    );
}

#[test]
fn an_export_whose_stated_first_address_is_not_its_wallets_is_refused() {
    let c = Compute::unchecked();
    let other = MK4.replace(
        "bc1q5y4r767v5fzx74ez4nw36hjqrhr4ayeyut5px6",
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
    );
    assert_eq!(
        coldcard::read(&c, &other),
        Err(ReadError::FirstAddress {
            account: "bip84",
            first: "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu".into(),
        })
    );
    assert!(coldcard::parse(&c, &other).is_err());
    // An account with a key and no derivation states no origin.
    let no_deriv = MK4.replace("\"deriv\": \"m/84'/0'/0'\", ", "");
    assert_ne!(no_deriv, MK4);
    assert_eq!(
        coldcard::read(&c, &no_deriv),
        Err(ReadError::NoDeriv("bip84"))
    );
}

/// Every account key a file offers, rooted at the master: the multisig
/// key file's flat fields in their SLIP-132 spellings, read as `xpub`,
/// and the generic export's account objects.
#[test]
fn every_account_key_a_file_offers_is_rooted_at_the_master() {
    let keys = coldcard::cosigner_keys(UNCHAINED);
    assert_eq!(
        keys,
        [
            "[b68af6e4/48h/0h/0h/2h]xpub6ENXu6jP3XgXCMgRq1Pb1bNXKjRZ7JnM8R17W8dH6GSmM4VokYExQX8mtSwnjqyxJ8qNdrT1Us8jNwz8Y9mvBkmLhzTzwKKYEtZFMszFScb",
            "[b68af6e4/48h/0h/0h/1h]xpub6ENXu6jP3XgX9QDKy3eSUDpuskB3UahgifC5jtTzE9zeg4uhMjmSwep6rZBkhTsTgJ4r64YEpdKd311YBFQos7H55Sio5k9GEgRXBhYNZvm",
            "[b68af6e4/45h]xpub69EKPNo9Jkd6v2h7xNKw5RdbFBoaHEcstXcRNfcQ2jg71iFpobCwcxfJjaV2ycGy218f2jM1znqs1SDkqMiR7fbyBVJwzacg2QarGt1gtJg",
        ]
    );
    // The Q's generic export offers the same three, as account objects.
    let q = coldcard::cosigner_keys(Q);
    for k in &keys {
        assert!(q.contains(k), "{k}");
    }
    assert_eq!(
        coldcard::cosigner_key(
            UNCHAINED,
            Cosigner::Multisig(MultisigScriptType::NativeSegwit)
        )
        .unwrap(),
        "[b68af6e4/48h/0h/0h/2h]xpub6ENXu6jP3XgXCMgRq1Pb1bNXKjRZ7JnM8R17W8dH6GSmM4VokYExQX8mtSwnjqyxJ8qNdrT1Us8jNwz8Y9mvBkmLhzTzwKKYEtZFMszFScb"
    );
    // Sparrow's copy of a Coldcard's key file on testnet, with the
    // nested field written `p2wsh_p2sh`.
    assert_eq!(coldcard::cosigner_keys(CCXP).len(), 3);
}

/// A flat field's key in the version of the other multisig script (the
/// `Ypub` of `p2sh_p2wsh` written under `p2wsh`) is refused, not
/// converted.
#[test]
fn a_multisig_key_in_another_scripts_version_is_refused() {
    let p2wsh = "Zpub74w9dfoeurKrKXE3SPRpFquLPTkiCuSwGuhDzBgbE42w5ShB2FxMjmJyjZpSJ6WhLt8y1PeFHQELGgq2GmktviFDH8yFWYRWg4xQiw3v335";
    let p2sh_p2wsh = "Ypub6k6tL18jmAnNRGZpk4u3WPGDmWMkdZNmx3MySYdQywCwMMHqNoKHeqLAgU6pFokHKQFdi88vAW4g3TEsCAymoq5LnFXd54RkQ8m3AD9f81J";
    assert!(UNCHAINED.contains(p2wsh) && UNCHAINED.contains(p2sh_p2wsh));
    let swapped = UNCHAINED
        .replace(p2wsh, "SWAP")
        .replace(p2sh_p2wsh, p2wsh)
        .replace("SWAP", p2sh_p2wsh);
    for script in [
        MultisigScriptType::NativeSegwit,
        MultisigScriptType::NestedSegwit,
    ] {
        assert!(
            coldcard::cosigner_key(&swapped, Cosigner::Multisig(script)).is_err(),
            "{script:?}"
        );
    }
    assert_eq!(
        coldcard::cosigner_keys(&swapped),
        [
            "[b68af6e4/45h]xpub69EKPNo9Jkd6v2h7xNKw5RdbFBoaHEcstXcRNfcQ2jg71iFpobCwcxfJjaV2ycGy218f2jM1znqs1SDkqMiR7fbyBVJwzacg2QarGt1gtJg"
        ]
    );
}
