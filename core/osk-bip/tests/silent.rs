//! Silent payments (BIP-352) against the published vectors, and the
//! strings BIP-392, BIP-321 and BIP-353 wrap an address in.
//!
//! `tools/vectors/bip352/send_and_receive_test_vectors.json` is the
//! BIP's own file. Every case is read from both sides: the sender's
//! outputs are built from the input private keys and the recipients'
//! addresses, and the receiver then finds those same outputs in the
//! transaction with the label each was paid to.
//!
//! The BIP's `reference.py` sits beside that file and is not run here:
//! it imports a vendored `secp256k1lab` and two helper modules that are
//! not part of the download, and the vectors it would be run against
//! are its own published output. What stands in for that cross-check is
//! the two sides below agreeing over every case.

mod common;

use common::{hex, unhex};
use osk_bip::bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use osk_bip::keys::Network;
use osk_bip::silent::{self, Receiver, ScanInput, SendKey};
use serde_json::Value;

const VECTORS: &str =
    include_str!("../../../tools/vectors/bip352/send_and_receive_test_vectors.json");

/// The outpoint as a transaction serializes it: the txid least
/// significant byte first, then the index in four bytes least
/// significant byte first.
fn outpoint(txid: &str, vout: u64) -> [u8; 36] {
    let mut bytes = unhex(txid);
    bytes.reverse();
    let mut out = [0u8; 36];
    out[..32].copy_from_slice(&bytes);
    out[32..].copy_from_slice(&(vout as u32).to_le_bytes());
    out
}

/// A serialized witness: the item count, then each item's length and
/// its bytes, all as compact sizes.
fn witness(hex: &str) -> Vec<Vec<u8>> {
    let bytes = unhex(hex);
    let mut at = 0;
    let read = |at: &mut usize| -> u64 {
        let first = bytes[*at];
        *at += 1;
        match first {
            0xfd => {
                let v = u64::from(u16::from_le_bytes([bytes[*at], bytes[*at + 1]]));
                *at += 2;
                v
            }
            0xfe => {
                let v = u64::from(u32::from_le_bytes(
                    bytes[*at..*at + 4].try_into().expect("four bytes"),
                ));
                *at += 4;
                v
            }
            0xff => {
                let v = u64::from_le_bytes(bytes[*at..*at + 8].try_into().expect("eight bytes"));
                *at += 8;
                v
            }
            n => u64::from(n),
        }
    };
    if bytes.is_empty() {
        return Vec::new();
    }
    let count = read(&mut at);
    let mut items = Vec::new();
    for _ in 0..count {
        let len = read(&mut at) as usize;
        items.push(bytes[at..at + len].to_vec());
        at += len;
    }
    items
}

/// One input of a case, as the file writes it.
struct Vin {
    outpoint: [u8; 36],
    script_sig: Vec<u8>,
    witness: Vec<Vec<u8>>,
    prevout: Vec<u8>,
    private_key: Option<SecretKey>,
}

fn read_vins(given: &Value) -> Vec<Vin> {
    given["vin"]
        .as_array()
        .expect("vin is an array")
        .iter()
        .map(|v| Vin {
            outpoint: outpoint(
                v["txid"].as_str().expect("txid"),
                v["vout"].as_u64().expect("vout"),
            ),
            script_sig: unhex(v["scriptSig"].as_str().expect("scriptSig")),
            witness: witness(v["txinwitness"].as_str().expect("txinwitness")),
            prevout: unhex(v["prevout"]["scriptPubKey"]["hex"].as_str().expect("spk")),
            private_key: v["private_key"]
                .as_str()
                .map(|k| SecretKey::from_slice(&unhex(k)).expect("a private key")),
        })
        .collect()
}

/// Those inputs in the shape the module reads them.
fn scan_inputs<'a>(vins: &'a [Vin], witnesses: &'a [Vec<&'a [u8]>]) -> Vec<ScanInput<'a>> {
    vins.iter()
        .enumerate()
        .map(|(i, v)| ScanInput {
            outpoint: v.outpoint,
            script_sig: &v.script_sig,
            witness: witnesses[i].as_slice(),
            prevout: &v.prevout,
        })
        .collect()
}

/// The witness items as slices, which the inputs borrow.
fn witness_refs(vins: &[Vin]) -> Vec<Vec<&[u8]>> {
    vins.iter()
        .map(|v| v.witness.iter().map(Vec::as_slice).collect())
        .collect()
}

/// Every sending case: the public key taken from each input, and the
/// outputs the sender makes for each group of recipients.
#[test]
fn the_sender_makes_the_outputs_the_vectors_publish() {
    let secp = Secp256k1::new();
    let cases: Vec<Value> = serde_json::from_str(VECTORS).expect("the vectors parse");
    for case in &cases {
        let comment = case["comment"].as_str().expect("a comment");
        for test in case["sending"].as_array().expect("sending") {
            let given = &test["given"];
            let expected = &test["expected"];
            let vins = read_vins(given);
            let witnesses = witness_refs(&vins);
            let inputs = scan_inputs(&vins, &witnesses);
            let keys: Vec<PublicKey> = inputs.iter().filter_map(silent::input_public_key).collect();
            let published: Vec<String> = expected["input_pub_keys"]
                .as_array()
                .expect("input_pub_keys")
                .iter()
                .map(|k| String::from(k.as_str().expect("hex")))
                .collect();
            assert_eq!(
                keys.iter().map(|k| hex(&k.serialize())).collect::<Vec<_>>(),
                published,
                "{comment}: the public keys taken from the inputs"
            );

            // The recipients, one entry per output, grouped by scan key
            // as BIP-352 groups them.
            let mut groups: Vec<(PublicKey, Vec<PublicKey>)> = Vec::new();
            for entry in given["recipients"].as_array().expect("recipients") {
                let address = entry["address"].as_str().expect("an address");
                let (_, scan, spend) = silent::decode_address(address).expect("the address reads");
                assert_eq!(
                    hex(&scan.serialize()),
                    entry["scan_pub_key"].as_str().expect("scan_pub_key"),
                    "{comment}: the scan key the address carries"
                );
                assert_eq!(
                    hex(&spend.serialize()),
                    entry["spend_pub_key"].as_str().expect("spend_pub_key"),
                    "{comment}: the spend key the address carries"
                );
                let count = entry["count"].as_u64().unwrap_or(1);
                for _ in 0..count {
                    match groups.iter_mut().find(|(s, _)| *s == scan) {
                        Some((_, list)) => list.push(spend),
                        None => groups.push((scan, alloc_vec(spend))),
                    }
                }
            }

            let send_keys: Vec<SendKey> = vins
                .iter()
                .enumerate()
                .filter(|(i, _)| silent::input_public_key(&inputs[*i]).is_some())
                .map(|(_, v)| SendKey {
                    key: v
                        .private_key
                        .expect("a sending case states every private key"),
                    taproot: v.prevout.len() == 34 && v.prevout[0] == 0x51,
                })
                .collect();
            let outpoints: Vec<[u8; 36]> = vins.iter().map(|v| v.outpoint).collect();
            let mut made: Vec<String> = Vec::new();
            let mut failed = send_keys.is_empty();
            for (scan, spends) in &groups {
                if spends.len() > silent::K_MAX as usize {
                    failed = true;
                    break;
                }
                match silent::sender_outputs(&secp, &send_keys, &outpoints, scan, spends) {
                    Ok(outputs) => made.extend(outputs.iter().map(|o| hex(o))),
                    Err(_) => {
                        failed = true;
                        break;
                    }
                }
            }
            // The file lists every ordering the outputs may come in;
            // one of them is the set that was made.
            let allowed = expected["outputs"].as_array().expect("outputs");
            if failed {
                assert!(
                    allowed
                        .iter()
                        .any(|set| set.as_array().expect("a set").is_empty()),
                    "{comment}: the sender made no outputs and the vector expects some"
                );
                continue;
            }
            made.sort();
            let matched = allowed.iter().any(|set| {
                let mut want: Vec<String> = set
                    .as_array()
                    .expect("a set")
                    .iter()
                    .map(|o| String::from(o.as_str().expect("hex")))
                    .collect();
                want.sort();
                want == made
            });
            assert!(
                matched,
                "{comment}: the outputs the sender made are {made:?}"
            );
        }
    }
}

/// A one-element vector, written out so the grouping above reads.
fn alloc_vec(key: PublicKey) -> Vec<PublicKey> {
    vec![key]
}

/// Every receiving case: the addresses the wallet publishes, and the
/// outputs of the transaction it finds.
#[test]
fn the_receiver_finds_the_outputs_the_vectors_publish() {
    let secp = Secp256k1::new();
    let cases: Vec<Value> = serde_json::from_str(VECTORS).expect("the vectors parse");
    for case in &cases {
        let comment = case["comment"].as_str().expect("a comment");
        for test in case["receiving"].as_array().expect("receiving") {
            let given = &test["given"];
            let expected = &test["expected"];
            let scan = SecretKey::from_slice(&unhex(
                given["key_material"]["scan_priv_key"]
                    .as_str()
                    .expect("scan"),
            ))
            .expect("a scan key");
            let spend = SecretKey::from_slice(&unhex(
                given["key_material"]["spend_priv_key"]
                    .as_str()
                    .expect("spend"),
            ))
            .expect("a spend key");
            let receiver = Receiver::new(scan, spend.public_key(&secp), Network::Mainnet);
            let labels: Vec<u32> = given["labels"]
                .as_array()
                .expect("labels")
                .iter()
                .map(|m| m.as_u64().expect("a label") as u32)
                .collect();

            // The address itself, then one per label, in the order the
            // labels are given.
            let published = expected["addresses"].as_array().expect("addresses");
            assert_eq!(
                receiver.address(&secp).expect("an address").as_str(),
                published[0].as_str().expect("the address"),
                "{comment}: the address"
            );
            for (i, m) in labels.iter().enumerate() {
                assert_eq!(
                    receiver
                        .labelled_address(&secp, *m)
                        .expect("a labelled address")
                        .as_str(),
                    published[i + 1].as_str().expect("a labelled address"),
                    "{comment}: the address for label {m}"
                );
            }

            let vins = read_vins(given);
            let witnesses = witness_refs(&vins);
            let inputs = scan_inputs(&vins, &witnesses);
            let outputs: Vec<[u8; 32]> = given["outputs"]
                .as_array()
                .expect("outputs")
                .iter()
                .map(|o| {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&unhex(o.as_str().expect("hex")));
                    key
                })
                .collect();
            let found =
                silent::scan(&secp, &receiver, &inputs, &outputs, &labels).unwrap_or_default();

            if let Some(n) = expected["n_outputs"].as_u64() {
                assert_eq!(
                    found.len(),
                    n as usize,
                    "{comment}: how many outputs the wallet found"
                );
                continue;
            }
            let mut want: Vec<String> = expected["outputs"]
                .as_array()
                .expect("outputs")
                .iter()
                .map(|o| String::from(o["pub_key"].as_str().expect("pub_key")))
                .collect();
            let mut got: Vec<String> = found.iter().map(|f| hex(&f.key)).collect();
            want.sort();
            got.sort();
            assert_eq!(got, want, "{comment}: the outputs the wallet found");
        }
    }
}

/// A payment to a labelled address is found under that label, and a
/// payment to the address itself under none. The outputs come from the
/// sending side of the same module, so the two halves are held against
/// each other rather than against one set of numbers.
#[test]
fn a_labelled_payment_is_found_under_its_label() {
    let secp = Secp256k1::new();
    let cases: Vec<Value> = serde_json::from_str(VECTORS).expect("the vectors parse");
    // The first case's inputs: two P2PKH spends with their keys.
    let given = &cases[0]["sending"][0]["given"];
    let vins = read_vins(given);
    let witnesses = witness_refs(&vins);
    let inputs = scan_inputs(&vins, &witnesses);
    let outpoints: Vec<[u8; 36]> = vins.iter().map(|v| v.outpoint).collect();
    let keys: Vec<SendKey> = vins
        .iter()
        .map(|v| SendKey {
            key: v.private_key.expect("a private key"),
            taproot: false,
        })
        .collect();

    let receiving = &cases[0]["receiving"][0]["given"]["key_material"];
    let scan = SecretKey::from_slice(&unhex(receiving["scan_priv_key"].as_str().expect("scan")))
        .expect("a scan key");
    let spend = SecretKey::from_slice(&unhex(receiving["spend_priv_key"].as_str().expect("spend")))
        .expect("a spend key");
    let receiver = Receiver::new(scan, spend.public_key(&secp), Network::Mainnet);
    let scan_key = receiver.scan_public_key(&secp);

    for label in [1u32, 2, 1_001_337] {
        let spend_key = receiver
            .labelled_spend_key(&secp, label)
            .expect("a labelled spend key");
        let outputs = silent::sender_outputs(&secp, &keys, &outpoints, &scan_key, &[spend_key])
            .expect("paid");
        let found = silent::scan(
            &secp,
            &receiver,
            &inputs,
            &outputs,
            &[silent::CHANGE_LABEL, label],
        )
        .expect("the wallet scans");
        assert_eq!(found.len(), 1, "one output pays this wallet");
        assert_eq!(found[0].key, outputs[0]);
        assert_eq!(found[0].label, Some(label));
    }

    // The same sender paying the address itself.
    let outputs = silent::sender_outputs(
        &secp,
        &keys,
        &outpoints,
        &scan_key,
        &[receiver.spend_public_key()],
    )
    .expect("paid");
    let found = silent::scan(&secp, &receiver, &inputs, &outputs, &[silent::CHANGE_LABEL])
        .expect("the wallet scans");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].label, None);
}

/// A transaction with no output for this wallet is not paid here, and
/// one whose inputs BIP-352 takes nothing from is refused rather than
/// reported as unpaid.
#[test]
fn a_transaction_that_does_not_pay_this_wallet_finds_nothing() {
    let secp = Secp256k1::new();
    let cases: Vec<Value> = serde_json::from_str(VECTORS).expect("the vectors parse");
    let given = &cases[23]["receiving"][0]["given"];
    let scan = SecretKey::from_slice(&unhex(
        given["key_material"]["scan_priv_key"]
            .as_str()
            .expect("scan"),
    ))
    .expect("a scan key");
    // A different wallet's spend key: the same transaction, read by
    // someone it does not pay.
    let other = SecretKey::from_slice(&[7u8; 32]).expect("a key");
    let receiver = Receiver::new(scan, other.public_key(&secp), Network::Mainnet);
    let vins = read_vins(given);
    let witnesses = witness_refs(&vins);
    let inputs = scan_inputs(&vins, &witnesses);
    let outputs: Vec<[u8; 32]> = given["outputs"]
        .as_array()
        .expect("outputs")
        .iter()
        .map(|o| {
            let mut key = [0u8; 32];
            key.copy_from_slice(&unhex(o.as_str().expect("hex")));
            key
        })
        .collect();
    let found = silent::scan(&secp, &receiver, &inputs, &outputs, &[silent::CHANGE_LABEL])
        .expect("the wallet scans");
    assert!(found.is_empty());

    // The case whose inputs carry no key BIP-352 reads.
    let given = &cases[24]["receiving"][0]["given"];
    let bare = read_vins(given);
    let bare_witnesses = witness_refs(&bare);
    let bare_inputs = scan_inputs(&bare, &bare_witnesses);
    assert_eq!(
        silent::input_sum_and_hash(&bare_inputs),
        Err(silent::Error::NoInputs)
    );
}

/// The address examples BIP-352 publishes read back to the two public
/// keys they carry, and an address is refused where its checksum, its
/// human-readable part or its length is wrong.
#[test]
fn an_address_reads_back_to_the_keys_it_carries() {
    let secp = Secp256k1::new();
    let cases: Vec<Value> = serde_json::from_str(VECTORS).expect("the vectors parse");
    let address = cases[0]["receiving"][0]["expected"]["addresses"][0]
        .as_str()
        .expect("an address");
    let (network, scan, spend) = silent::decode_address(address).expect("the address reads");
    assert_eq!(network, Network::Mainnet);
    assert_eq!(address.len(), 116);
    assert_eq!(
        silent::encode_address(Network::Mainnet, &scan, &spend)
            .expect("written back")
            .as_str(),
        address
    );

    assert!(silent::looks_like_address(address));
    assert!(!silent::looks_like_address(
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"
    ));

    let mut broken = String::from(address);
    broken.pop();
    broken.push('q');
    assert!(silent::decode_address(&broken).is_err());
    assert!(silent::decode_address(&address.replace("sp1", "tsp1")).is_err());

    // A test-network wallet writes the three-character part.
    let key = SecretKey::from_slice(&[3u8; 32]).expect("a key");
    let receiver = Receiver::new(key, key.public_key(&secp), Network::Testnet);
    let text = receiver.address(&secp).expect("an address");
    assert!(text.as_str().starts_with("tsp1q"));
    assert_eq!(text.as_str().len(), 117);
}

/// The strings that carry an address elsewhere: BIP-392's key
/// expression and descriptor, BIP-321's URI and BIP-353's record.
#[test]
fn an_address_travels_as_a_descriptor_a_uri_and_a_record() {
    let secp = Secp256k1::new();
    let scan = SecretKey::from_slice(&unhex(
        "0f694e068028a717f8af6b9411f9a133dd3565258714cc226594b34db90c1f2c",
    ))
    .expect("a scan key");
    let spend = SecretKey::from_slice(&unhex(
        "9d6ad855ce3417ef84e836892e5a56392bfba05fa5d97ccea30e266f540e08b3",
    ))
    .expect("a spend key");
    let receiver = Receiver::new(scan, spend.public_key(&secp), Network::Mainnet);

    let key = receiver.scan_key_text().expect("the key expression");
    assert!(key.as_str().starts_with("spscan1q"));
    let descriptor = receiver
        .descriptor(Some((
            osk_bip::keys::Fingerprint([0xde, 0xad, 0xbe, 0xef]),
            0,
        )))
        .expect("the descriptor");
    assert_eq!(
        descriptor.as_str(),
        format!("sp([deadbeef/352h/0h/0h]{})", key.as_str())
    );
    let plain = receiver.descriptor(None).expect("the descriptor");
    assert_eq!(plain.as_str(), format!("sp({})", key.as_str()));

    let address = receiver.address(&secp).expect("an address");
    let uri = silent::uri(address.as_str()).expect("the uri");
    assert_eq!(uri.as_str(), format!("bitcoin:?sp={}", address.as_str()));
    let record = silent::dns_record("matt", "example.com", address.as_str()).expect("the record");
    assert_eq!(
        record.as_str(),
        format!(
            "matt.user._bitcoin-payment.example.com. IN TXT \"bitcoin:?sp={}\"",
            address.as_str()
        )
    );

    // A test-network wallet writes the test-network parts.
    let receiver = Receiver::new(scan, spend.public_key(&secp), Network::Signet);
    assert!(
        receiver
            .scan_key_text()
            .expect("the key expression")
            .as_str()
            .starts_with("tspscan1q")
    );
}

/// The scan and spend keys come from BIP-352's paths, hardened to the
/// account and unhardened at the last step.
#[test]
fn the_keys_come_from_the_paths_the_bip_names() {
    use osk_bip::bip39::{Language, Mnemonic};
    use osk_bip::keys::MasterKey;
    let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let m = Mnemonic::parse(Language::English, words).expect("the words");
    let master = MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), Network::Mainnet);
    let receiver = Receiver::derive(&master, 0);
    assert_eq!(
        silent::scan_path(Network::Mainnet, 0).to_string(),
        "352'/0'/0'/1'/0"
    );
    assert_eq!(
        silent::spend_path(Network::Mainnet, 0).to_string(),
        "352'/0'/0'/0'/0"
    );
    let scan = master.derive(&silent::scan_path(Network::Mainnet, 0));
    let spend = master.derive(&silent::spend_path(Network::Mainnet, 0));
    let secp = Secp256k1::new();
    assert_eq!(
        receiver.scan_public_key(&secp),
        scan.secret_key().public_key(&secp)
    );
    assert_eq!(
        receiver.spend_public_key(),
        spend.secret_key().public_key(&secp)
    );
    // The same words on another network give another wallet.
    let testnet = MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), Network::Testnet);
    let other = Receiver::derive(&testnet, 0);
    assert_ne!(other.spend_public_key(), receiver.spend_public_key());
}
