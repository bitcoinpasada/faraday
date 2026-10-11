//! A threshold wallet's group record (`docs/PLANNING.md` §16.103): what
//! it states, what it refuses, and where the wallet it makes pays.
//!
//! The fixture is the committed 2-of-3 regtest group,
//! `tools/vectors/psbt/wallet-threshold-regtest.record`, with its three
//! shares as words beside it.

use std::str::FromStr;

use bitcoin::NetworkKind;
use bitcoin::bip32::DerivationPath;
use bitcoin::secp256k1::Secp256k1;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::frost::{self, SecShare};
use osk_bip::keys::Network;
use osk_bip::miniscript::Descriptor;
use osk_bip::miniscript::descriptor::DescriptorPublicKey;
use osk_bip::policy::{Error, Template, WalletPolicy};
use osk_bip::threshold::ThresholdRecord;

const RECORD: &str = include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest.record");
const SHARES: [&str; 3] = [
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-0.txt"),
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-1.txt"),
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-2.txt"),
];

fn record() -> ThresholdRecord {
    ThresholdRecord::parse(RECORD).expect("the committed record")
}

/// The record with one line replaced, which is how every refusal below
/// is written: everything else about the file stays as it was.
fn with_line(prefix: &str, line: &str) -> String {
    let mut out = String::new();
    for l in RECORD.trim().lines() {
        out.push_str(if l.starts_with(prefix) { line } else { l });
        out.push('\n');
    }
    out
}

/// The record reads back as itself, and states the group it was dealt
/// from.
#[test]
fn the_record_round_trips_and_states_its_group() {
    let record = record();
    assert_eq!(record.t(), 2);
    assert_eq!(record.n(), 3);
    assert_eq!(format!("{}\n", record.to_text()), RECORD);
    assert_eq!(
        ThresholdRecord::parse(&record.to_text()),
        Ok(record.clone())
    );

    // Blank lines and comments are skipped, as a wallet policy's are.
    let commented = format!("# a group\n\n{RECORD}\n");
    assert_eq!(ThresholdRecord::parse(&commented), Ok(record.clone()));

    // Every participant has a fingerprint of its own, and the wallet's
    // is the extended public key's.
    let mut seen = vec![record.fingerprint()];
    for i in 0..record.n() {
        let fp = record.share_fingerprint(i).expect("a present share");
        assert!(!seen.contains(&fp), "share {i} repeats a fingerprint");
        seen.push(fp);
    }
    assert_eq!(record.share_fingerprint(3), None);
}

/// Every way a record can be wrong, each refused with its own reason.
#[test]
fn a_record_that_states_something_untrue_is_refused() {
    const OTHER_KEY: &str = "0376c80f9704bbf33f96b9fcf464deec0548997110fc1e0fed81c9bea5e0ea8500";
    let record = record();
    let group = record.info.thresh_pk;
    let shares: Vec<String> = RECORD
        .lines()
        .filter(|l| l.starts_with("share "))
        .map(String::from)
        .collect();

    for (text, expected, why) in [
        (
            RECORD.replacen("osk-threshold 1", "osk-threshold 2", 1),
            Error::Template,
            "another version of the format",
        ),
        (
            RECORD.replacen("osk-threshold 1", "osk-group 1", 1),
            Error::Template,
            "another format",
        ),
        (
            with_line("threshold ", "threshold 4"),
            Error::Threshold,
            "a threshold above the participants",
        ),
        (
            with_line("threshold ", "threshold 0"),
            Error::Threshold,
            "a threshold of none",
        ),
        (
            RECORD.replacen(&shares[1], "", 1),
            Error::Share,
            "a share missing from the sequence",
        ),
        (
            RECORD.replacen("share 1 ", "share 2 ", 1),
            Error::Share,
            "the shares out of order",
        ),
        (
            with_line("share 0 ", &format!("share 0 {OTHER_KEY}")),
            Error::Key,
            "a share that is no point",
        ),
        (
            with_line(
                "group ",
                &format!("group {}", shares[0]["share 0 ".len()..].trim()),
            ),
            Error::Group,
            "a group key the shares do not give",
        ),
    ] {
        assert_eq!(ThresholdRecord::parse(&text), Err(expected), "{why}");
    }

    // Sixteen participants: more than one QR and one slot carry.
    let secp = Secp256k1::verification_only();
    let chosen: Vec<(u32, SecShare)> = (0..2u32)
        .map(|i| {
            let mut bytes = [1u8; 32];
            bytes[31] = i as u8 + 1;
            (i, SecShare::from_bytes(&bytes).expect("a scalar"))
        })
        .collect();
    let big = frost::deal(&secp, 16, 2, &chosen).expect("a group of sixteen");
    let big = ThresholdRecord::new(big.info, NetworkKind::Test);
    assert_eq!(
        ThresholdRecord::parse(&big.to_text()),
        Err(Error::Threshold),
        "sixteen participants"
    );

    // Another group's shares under this group key, which is the same
    // refusal by another road.
    let others = frost::deal(&secp, 3, 2, &chosen).expect("another group");
    let mut mixed = ThresholdRecord::new(others.info, NetworkKind::Test);
    mixed.info.thresh_pk = group;
    let mixed = mixed.to_text().replacen(
        &mixed.descriptor_checksummed(),
        &record.descriptor_checksummed(),
        1,
    );
    assert_eq!(ThresholdRecord::parse(&mixed), Err(Error::Group));

    // The extended public key is the group key's own: another group's
    // key over these shares is no wallet of theirs.
    let theirs = ThresholdRecord::new(
        frost::deal(&secp, 3, 2, &chosen)
            .expect("another group")
            .info,
        NetworkKind::Test,
    );
    let wrong_xpub = RECORD.replacen(
        &record.descriptor_checksummed(),
        &theirs.descriptor_checksummed(),
        1,
    );
    assert_eq!(ThresholdRecord::parse(&wrong_xpub), Err(Error::Xpub));

    // The network is what the version bytes state, so the same group
    // written as mainnet is another extended public key and another
    // wallet; a `tpub` record read as mainnet is not this one.
    let mainnet = ThresholdRecord::new(record.info.clone(), NetworkKind::Main);
    assert_ne!(mainnet.xpub, record.xpub);
    assert_eq!(
        ThresholdRecord::parse(&mainnet.to_text())
            .expect("a mainnet record of the same group")
            .xpub
            .network,
        NetworkKind::Main
    );

    // A checksum that does not hold, and a descriptor with none at all.
    let bad = record.descriptor_checksummed().replace("#q", "#p");
    let wrong_checksum = RECORD.replacen(&record.descriptor_checksummed(), &bad, 1);
    assert_eq!(
        ThresholdRecord::parse(&wrong_checksum),
        Err(Error::Checksum)
    );
    let none = RECORD.replacen(&record.descriptor_checksummed(), &record.descriptor(), 1);
    assert_eq!(ThresholdRecord::parse(&none), Err(Error::Checksum));
}

/// The record is a wallet: `parse_any` reads it, writes it back
/// unchanged, and names the kind and the script it pays to.
#[test]
fn the_record_is_a_wallet_policy() {
    let policy = WalletPolicy::parse_any(RECORD).expect("the committed record");
    assert_eq!(
        policy.template(),
        Template::Threshold {
            threshold: 2,
            participants: 3
        }
    );
    assert_eq!(policy.quorum(), None);
    assert_eq!(policy.script_type(), osk_bip::keys::ScriptType::Taproot);
    assert_eq!(policy.keys().len(), 1);
    assert_eq!(policy.keys()[0].fingerprint(), None, "no master exists");
    assert_eq!(policy.template_text(), "tr(@0/**)");
    assert_eq!(policy.to_descriptor(), record().descriptor());
    assert_eq!(
        policy.to_descriptor_checksummed(),
        record().descriptor_checksummed()
    );
    assert_eq!(format!("{}\n", policy.to_text()), RECORD);
    assert_eq!(
        WalletPolicy::parse_any(&policy.to_text()),
        Ok(policy.clone())
    );
    assert!(policy.spend_paths().is_empty(), "one key, one signature");
    assert_eq!(policy.record().map(|r| r.n()), Some(3));

    // The two-part form and the plain-descriptor form are not records.
    assert!(WalletPolicy::parse(RECORD).is_err());
    assert!(WalletPolicy::from_descriptor(RECORD).is_err());
}

/// The wallet pays where any descriptor wallet given the record's own
/// descriptor line pays.
#[test]
fn the_wallet_pays_where_its_descriptor_says() {
    let policy = WalletPolicy::parse_any(RECORD).expect("the committed record");
    let descriptor = Descriptor::<DescriptorPublicKey>::from_str(&record().descriptor())
        .expect("the record's descriptor line");
    let chains = descriptor
        .into_single_descriptors()
        .expect("a receive chain and a change chain");
    let secp = Secp256k1::verification_only();
    for (chain, descriptor) in chains.iter().enumerate() {
        for index in [0u32, 1, 7] {
            let theirs = descriptor
                .clone()
                .at_derivation_index(index)
                .expect("index")
                .derived_descriptor(&secp)
                .expect("derivation")
                .address(bitcoin::Network::Regtest)
                .expect("an address");
            let ours = policy
                .address_at(Network::Regtest, chain == 1, index)
                .expect("an address");
            assert_eq!(ours, theirs, "chain {chain} index {index}");
        }
    }
}

/// A coordinator holding `tr(XPUB/<0;1>/*)` writes the group key's own
/// fingerprint and two unhardened steps for each of the wallet's keys,
/// and that is the output the wallet claims.
#[test]
fn an_output_of_the_wallet_is_named_by_its_chain_and_index() {
    let policy = WalletPolicy::parse_any(RECORD).expect("the committed record");
    let fp = record().fingerprint();
    for (path, expected) in [
        ("0/0", Some((false, 0))),
        ("1/5", Some((true, 5))),
        ("0", None),
        ("0/0/0", None),
        ("2/0", None),
        ("0h/0", None),
    ] {
        let path = DerivationPath::from_str(path).expect("a path");
        assert_eq!(policy.leaf_of(fp, &path), expected, "{path}");
    }
    let other = osk_bip::keys::Fingerprint([0, 0, 0, 0]);
    assert_eq!(
        policy.leaf_of(other, &DerivationPath::from_str("0/0").unwrap()),
        None,
        "another wallet's key names nothing here"
    );
}

/// The fixture's three share files are the shares of the fixture's own
/// group: any two of them rebuild the record every one of them belongs
/// to.
#[test]
fn the_share_files_are_the_shares_of_the_fixture_group() {
    let secp = Secp256k1::new();
    let record = record();
    record.info.validate(&secp).expect("one polynomial");

    let shares: Vec<SecShare> = SHARES
        .iter()
        .map(|words| {
            let m = Mnemonic::parse(Language::English, words.trim()).expect("24 English words");
            let entropy = m.entropy();
            SecShare::from_bytes(
                &<[u8; 32]>::try_from(entropy.expose().as_bytes()).expect("32 bytes"),
            )
            .expect("a scalar")
        })
        .collect();
    for (i, share) in shares.iter().enumerate() {
        assert_eq!(
            record.info.pubshares[i],
            Some(share.public_share(&secp)),
            "share {i} belongs to participant {i}"
        );
    }
    for pair in [[0usize, 1], [0, 2], [1, 2]] {
        let held: Vec<(u32, &SecShare)> = pair.iter().map(|i| (*i as u32, &shares[*i])).collect();
        let rebuilt = frost::recover_info(&secp, 3, 2, &held).expect("any two shares");
        assert_eq!(
            ThresholdRecord::new(rebuilt, NetworkKind::Test),
            record,
            "{pair:?} rebuild the record"
        );
    }
}
