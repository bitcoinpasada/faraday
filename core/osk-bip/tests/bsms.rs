//! BIP 129's own records, read out of the specification text under
//! `tools/vectors/bip129/`, plus the two things a device does with
//! them: writing its own key record, and reading a wallet that arrives
//! as a BSMS descriptor record rather than as a coordinator config.

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bsms::{self, DescriptorRecord, SignerRecord};
use osk_bip::keys::{MasterKey, MultisigScriptType, Network};
use osk_bip::multisig_config;
use osk_bip::policy::WalletPolicy;

const BIP129: &str = include_str!("../../../tools/vectors/bip129/bip-0129.mediawiki");

/// Every `<pre>` block of the specification, which is where it writes
/// its example files.
fn blocks() -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = BIP129;
    while let Some(start) = rest.find("<pre>") {
        let after = &rest[start + 5..];
        let Some(end) = after.find("</pre>") else {
            break;
        };
        out.push(after[..end].to_string());
        rest = &after[end + 6..];
    }
    out
}

/// The records of one round: the example files with that many lines.
fn records(lines: usize) -> Vec<String> {
    blocks()
        .into_iter()
        .filter(|b| b.starts_with("BSMS 1.0") && b.lines().count() == lines)
        .collect()
}

/// The specification's nine key records: one pair made with plain
/// public keys, and seven made with extended keys.
#[test]
fn every_key_record_the_bip_writes_is_read_and_its_signature_holds() {
    let records = records(5);
    assert_eq!(records.len(), 9, "the BIP writes nine key records");
    let mut verified = 0;
    for text in &records {
        let key = text.lines().nth(2).unwrap();
        // A record whose key is a plain public key names no extended
        // key, so it is not a wallet this device can derive from.
        if !key.contains("xpub") {
            assert_eq!(SignerRecord::parse(text), Err(bsms::Error::Key), "{key}");
            continue;
        }
        let record = SignerRecord::parse(text).expect(text);
        record.verify().expect(text);
        assert_eq!(record.to_text(), *text);
        assert!(record.description.ends_with(" key"));
        verified += 1;
    }
    assert_eq!(verified, 7);
}

/// A signature of anything but these exact four lines does not hold:
/// the description, the token and the key are all inside it.
#[test]
fn a_key_record_changed_after_signing_does_not_verify() {
    let text = records(5)
        .into_iter()
        .find(|t| t.contains("xpub"))
        .expect("an extended-key record");
    for (from, to) in [
        ("Signer 1 key", "Signer 2 key"),
        ("Signer 2 key", "Signer 1 key"),
        ("\n00\n", "\na54044308ceac9b7\n"),
    ] {
        if !text.contains(from) {
            continue;
        }
        let record = SignerRecord::parse(&text.replace(from, to)).expect("still a record");
        assert_eq!(record.verify(), Err(bsms::Error::Signature));
    }
}

/// The specification's four descriptor records: one over plain public
/// keys, three over extended keys. Each states its wallet's own first
/// receive address, which is the check the BIP is built around.
#[test]
fn every_descriptor_record_the_bip_writes_is_its_wallet_and_its_first_address() {
    let records = records(4);
    assert_eq!(records.len(), 4, "the BIP writes four descriptor records");
    let mut read = 0;
    for text in &records {
        let descriptor = text.lines().nth(1).unwrap();
        if !descriptor.contains("xpub") {
            assert!(DescriptorRecord::parse(text).is_err(), "{descriptor}");
            continue;
        }
        let record = DescriptorRecord::parse(text).expect(text);
        assert_eq!(record.network, Network::Mainnet);
        assert_eq!(record.paths, ["/0/*", "/1/*"]);
        assert_eq!(record.to_text(), *text);
        assert!(record.holds(&record.policy.keys()[0]));
        read += 1;
    }
    assert_eq!(read, 3);
}

/// The wallet's first address is what a person compares out loud, so a
/// record carrying any other address is refused.
#[test]
fn a_descriptor_record_stating_another_address_is_refused() {
    let text = records(4)
        .into_iter()
        .find(|t| t.contains("/0/*,/1/*"))
        .expect("a template record");
    let address = text.lines().nth(3).unwrap();
    let other = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
    assert_eq!(
        DescriptorRecord::parse(&text.replace(address, other)),
        Err(bsms::Error::FirstAddress)
    );
    assert_eq!(
        DescriptorRecord::parse(&text.replace(address, "not an address")),
        Err(bsms::Error::Address)
    );
}

/// The encrypted form is hex of a MAC and a ciphertext. The device
/// cannot decrypt it, and says which thing it is rather than calling it
/// malformed.
#[test]
fn an_encrypted_record_is_refused_as_encrypted() {
    let blob = blocks()
        .into_iter()
        .find(|b| b.len() > 400 && b.bytes().all(|c| c.is_ascii_hexdigit()))
        .expect("an encrypted example file");
    assert_eq!(SignerRecord::parse(&blob), Err(bsms::Error::Encrypted));
    assert_eq!(DescriptorRecord::parse(&blob), Err(bsms::Error::Encrypted));
}

#[test]
fn a_record_of_another_version_is_refused() {
    let text = records(5).into_iter().next().unwrap();
    assert_eq!(
        SignerRecord::parse(&text.replace("BSMS 1.0", "BSMS 2.0")),
        Err(bsms::Error::Version)
    );
    assert!(!bsms::looks_like_record("Name: Demo\nPolicy: 2 of 2\n"));
    assert!(bsms::is_signer_record(&text));
    assert!(!bsms::is_signer_record(
        &records(4).into_iter().next().unwrap()
    ));
}

const KEYS: [&str; 3] = [
    "xpub6ErVmcYYHmavsMgxEcTZyzN5sqth1ZyRpFNJC26ij1wYGC2SBKYrgt9yariSbn7HLRoZUvhUhmPfsRTPrdhhGFscpPZzmch6UTdmRP1aZUj",
    "xpub6Du5Jn6eYZE96ccmAc1ZTFPzdnzrvqfG4mpamDun2qZYKywoiQJMCbS3kWWMr6U3XW6s125RLsaPABWgv2yA749ieaMe67FxkTjMsbcxCch",
    "xpub6Ex81KopPkEt9hJiWHabYy8LNsSR4A7sUQoFBk9dR8XxHrr4p9HrYWN3NCf5uwfopHnQkCG7FYnZMztKbtRtbh6tzZC4xtHPbmVVxRSN7ic",
];
const FINGERPRINTS: [&str; 3] = ["793cc70b", "b3118e52", "842bd2ed"];

/// The same 2-of-3 arriving as a coordinator config and as a BSMS
/// descriptor record is one wallet: the same keys, the same descriptor
/// and the same checksum, so a person comparing two devices that were
/// set up by different routes sees one thing.
#[test]
fn a_bsms_wallet_is_the_same_wallet_as_its_coordinator_config_twin() {
    let mut config =
        String::from("Name: Demo\nPolicy: 2 of 3\nFormat: P2WSH\nDerivation: m/48'/0'/0'/1'\n\n");
    for (fp, key) in FINGERPRINTS.iter().zip(KEYS) {
        config.push_str(&format!("{fp}: {key}\n"));
    }
    let from_config = multisig_config::parse(&config).expect("the coordinator's config");

    let address = from_config.address_at(Network::Mainnet, false, 0).unwrap();
    let record = format!(
        "BSMS 1.0\n{}\n/0/*,/1/*\n{address}",
        from_config.to_descriptor_template()
    );
    let from_bsms = DescriptorRecord::parse(&record).expect("the coordinator's BSMS record");

    assert_eq!(from_bsms.policy, from_config);
    assert_eq!(from_bsms.policy.to_text(), from_config.to_text());
    assert_eq!(from_bsms.policy.checksum(), from_config.checksum());
    assert_eq!(from_bsms.to_text(), record);
}

/// A wallet whose descriptor carries a `/**` on every key but says it
/// has no path restrictions, or the other way round, states two
/// different wallets and is refused.
#[test]
fn a_descriptor_record_whose_paths_and_template_disagree_is_refused() {
    let text = records(4)
        .into_iter()
        .find(|t| t.contains("/0/*,/1/*"))
        .expect("a template record");
    assert_eq!(
        DescriptorRecord::parse(&text.replace("/0/*,/1/*", "No path restrictions")),
        Err(bsms::Error::Paths)
    );
    assert_eq!(
        DescriptorRecord::parse(&text.replace("/0/*,/1/*", "/0/*,/1/*,/2/*")),
        Err(bsms::Error::Paths)
    );
}

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// The record this device writes for one of its own multisig account
/// keys is a record the other side reads: it parses back, its signature
/// holds against the key it names, and that key is the account's.
#[test]
fn the_record_this_device_writes_parses_back_and_verifies() {
    let mnemonic = Mnemonic::parse(Language::English, ABANDON).unwrap();
    let master = MasterKey::from_seed(&mnemonic.to_seed(b"").unwrap(), Network::Mainnet);
    let account = master
        .multisig_account_xpub(MultisigScriptType::NativeSegwit, 0)
        .unwrap();
    let derived = master.derive(account.path());

    let written = bsms::signer_record(
        master.secp(),
        &account,
        derived.secret_key(),
        bsms::NO_ENCRYPTION,
        "OpenSigner key",
    )
    .expect("a record for this account");

    let text = written.to_text();
    assert!(
        text.starts_with("BSMS 1.0\n00\n[73c5da0a/48'/0'/0'/2']xpub"),
        "{text}"
    );
    let read = SignerRecord::parse(&text).expect("what it wrote");
    assert_eq!(read, written);
    read.verify().expect("its own signature");
    assert_eq!(read.key.xpub(), account.xpub());

    // The coordinator puts that key into the wallet, and the wallet
    // that comes back names this key whole.
    let policy =
        WalletPolicy::from_parts("wsh(sortedmulti(1,@0/**))", &[&read.key.key_text()]).unwrap();
    let address = policy.address_at(Network::Mainnet, false, 0).unwrap();
    let back = DescriptorRecord::parse(&format!(
        "BSMS 1.0\n{}\n/0/*,/1/*\n{address}",
        policy.to_descriptor_template()
    ))
    .expect("the descriptor record");
    assert!(back.holds(&read.key));
}

/// A description longer than the eighty characters the BIP allows, and
/// a token that is neither `00` nor a nonce, are refused on the way in
/// and on the way out.
#[test]
fn a_record_outside_the_bips_limits_is_refused() {
    let mnemonic = Mnemonic::parse(Language::English, ABANDON).unwrap();
    let master = MasterKey::from_seed(&mnemonic.to_seed(b"").unwrap(), Network::Mainnet);
    let account = master
        .multisig_account_xpub(MultisigScriptType::NativeSegwit, 0)
        .unwrap();
    let derived = master.derive(account.path());
    let write = |token: &str, description: &str| {
        bsms::signer_record(
            master.secp(),
            &account,
            derived.secret_key(),
            token,
            description,
        )
    };
    assert_eq!(write("0", "fine"), Err(bsms::Error::Token));
    assert_eq!(write("zz", "fine"), Err(bsms::Error::Token));
    assert_eq!(
        write("a54044308ceac9b7", "fine").map(|r| r.token),
        Ok(String::from("a54044308ceac9b7"))
    );
    assert_eq!(write("00", &"x".repeat(81)), Err(bsms::Error::Description));

    let text = records(5)
        .into_iter()
        .find(|t| t.contains("xpub"))
        .expect("an extended-key record");
    let long = text.replace("Signer 1 key", &"x".repeat(81));
    assert_eq!(SignerRecord::parse(&long), Err(bsms::Error::Description));
}
