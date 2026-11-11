//! Bitcoin Core's descriptor wallets, made by Bitcoin Core
//! (`tools/vectors/core-wallet/`), read as Core says
//! (`listdescriptors`, `getwalletinfo`).

use faraday_bip::keys::Network;
use faraday_core::core_wallet::{Fault, Passphrase, Refusal, Wallet, read, read_file};
use faraday_intake::secret_buf::SecretText;

type Records = Vec<(Vec<u8>, Vec<u8>)>;

fn dump(name: &str) -> (u32, Records, serde_json::Value) {
    let path = format!(
        "{}/../../tools/vectors/core-wallet/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("dump")).expect("json");
    let hex = |v: &serde_json::Value| {
        let s = v.as_str().expect("hex");
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
            .collect::<Vec<u8>>()
    };
    let records = json["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|r| (hex(&r[0]), hex(&r[1])))
        .collect();
    (
        json["application_id"].as_u64().expect("id") as u32,
        records,
        json,
    )
}

fn load(name: &str, passphrase: Option<&str>) -> Wallet {
    let c = faraday_core::testkit::compute();
    let (id, records, _) = dump(name);
    let p = passphrase.map(SecretText::of);
    read(&c, id, &records, Network::Regtest, p.as_ref()).expect(name)
}

fn bare(text: &str) -> &str {
    text.split('#').next().expect("text")
}

#[test]
fn each_wallet_reads_as_core_lists_it() {
    for name in [
        "plain",
        "encrypted",
        "watchonly",
        "blank",
        "labels",
        "imported",
    ] {
        let wallet = load(name, Some("faraday"));
        let (_, _, core) = dump(name);
        assert!(wallet.is_whole(), "{name}: {:?}", wallet.failures);
        let listed = core["listdescriptors"]["descriptors"]
            .as_array()
            .expect("list");
        assert_eq!(wallet.descriptors.len(), listed.len(), "{name}");
        for d in listed {
            let text = d["desc"].as_str().expect("desc");
            // Core lists private descriptors with their keys; compare the
            // shape of the public text through the range and next index.
            let found = wallet
                .descriptors
                .iter()
                .find(|w| {
                    d["timestamp"].as_u64() == Some(w.creation_time)
                        && d["next_index"].as_i64().unwrap_or(0) == i64::from(w.next_index)
                        && d["range"].as_array().map(|r| r[1].as_i64())
                            == w.range.map(|r| Some(i64::from(r.1)))
                        && bare(text).len() >= 4
                        && w.text.split('(').next() == text.split('(').next()
                })
                .unwrap_or_else(|| panic!("{name}: {text}"));
            let active = wallet.active.iter().any(|a| a.id == found.id);
            assert_eq!(active, d["active"].as_bool().expect("active"), "{name}");
        }
        let info = &core["getwalletinfo"];
        assert_eq!(
            wallet.is_encrypted(),
            info.get("unlocked_until").is_some(),
            "{name}"
        );
    }
}

#[test]
fn labels_and_the_block_are_counted() {
    let wallet = load("labels", None);
    assert!(
        wallet
            .labels
            .iter()
            .any(|l| l.name.as_deref() == Some("savings"))
    );
    assert!(
        wallet
            .labels
            .iter()
            .any(|l| l.name.as_deref() == Some("rent"))
    );
    assert!(wallet.block_locator.is_some());
    assert_eq!(load("plain", None).tx_count, 1);
}

#[test]
fn keys_pass_and_an_encrypted_wallet_needs_its_passphrase() {
    let plain = load("plain", None);
    assert_eq!(plain.keys_passed, plain.descriptors.len());
    let locked = load("encrypted", None);
    assert_eq!(locked.passphrase, Passphrase::NotGiven);
    assert!(locked.keys_unchecked > 0);
    let opened = load("encrypted", Some("faraday"));
    assert_eq!(opened.passphrase, Passphrase::Opened);
    assert_eq!(opened.keys_passed, opened.descriptors.len());
    let wrong = load("encrypted", Some("faradaz"));
    assert_eq!(wrong.passphrase, Passphrase::Wrong);
    assert!(!wrong.is_whole());
}

fn changed(name: &str, record: &str, at: usize, in_value: bool) -> Wallet {
    let c = faraday_core::testkit::compute();
    let (id, mut records, _) = dump(name);
    let (k, v) = records
        .iter_mut()
        .find(|(k, _)| k.get(1..=record.len()) == Some(record.as_bytes()))
        .expect("record");
    if in_value {
        v[at] ^= 1;
    } else {
        let last = k.len() - 1 - at;
        k[last] ^= 1;
    }
    read(&c, id, &records, Network::Regtest, None).expect("read")
}

#[test]
fn a_changed_byte_is_refused_with_its_record_named() {
    let key = changed("plain", "walletdescriptorkey", 20, true);
    assert!(
        key.failures
            .iter()
            .any(|f| f.record.starts_with("walletdescriptorkey") && f.fault == Fault::KeyHash)
    );
    let hash = changed("plain", "walletdescriptorkey", 230, true);
    assert!(hash.failures.iter().any(|f| f.fault == Fault::KeyHash));
    let sum = changed("plain", "walletdescriptor", 100, true);
    assert!(
        sum.failures
            .iter()
            .any(|f| f.record.starts_with("walletdescriptor "))
    );
    let id = changed("plain", "walletdescriptor", 0, false);
    assert!(
        id.failures
            .iter()
            .any(|f| f.fault == Fault::Id || f.fault == Fault::NotHeld)
    );
    assert!(!id.is_whole());
}

#[test]
fn refusals() {
    let c = faraday_core::testkit::compute();
    let (id, records, _) = dump("plain");
    let net = read(&c, id, &records, Network::Mainnet, None);
    assert!(matches!(net, Err(Refusal::Network { .. })));
    let mut legacy = records.clone();
    legacy.push((b"\x03key".to_vec(), vec![0]));
    assert!(matches!(
        read(&c, id, &legacy, Network::Regtest, None),
        Err(Refusal::Legacy(_))
    ));
    let none: Records = records
        .iter()
        .filter(|(k, _)| k.get(1..6) != Some(b"flags"))
        .cloned()
        .collect();
    assert_eq!(
        read(&c, id, &none, Network::Regtest, None).err(),
        Some(Refusal::NotDescriptors)
    );
}

fn read_with(name: &str, passphrase: Option<&str>, edit: impl FnOnce(&mut Records)) -> Wallet {
    let c = faraday_core::testkit::compute();
    let (id, mut records, _) = dump(name);
    edit(&mut records);
    let p = passphrase.map(SecretText::of);
    read(&c, id, &records, Network::Regtest, p.as_ref()).expect("read")
}

fn record_of(records: &Records, name: &str) -> (Vec<u8>, Vec<u8>) {
    records
        .iter()
        .find(|(k, _)| k.get(1..=name.len()) == Some(name.as_bytes()))
        .cloned()
        .unwrap_or_else(|| panic!("no {name} record"))
}

fn failed(wallet: &Wallet, record: &str, fault: Fault) -> bool {
    wallet
        .failures
        .iter()
        .any(|f| f.record.starts_with(record) && f.fault == fault)
}

#[test]
fn a_record_whose_key_is_in_the_file_twice_is_refused() {
    // Bitcoin Core's table holds each key once: the second is named and
    // the wallet is not whole, for each kind of record.
    for (wallet, record, passphrase) in [
        ("plain", "flags", None),
        ("plain", "version", None),
        ("plain", "walletdescriptor", None),
        ("plain", "walletdescriptorkey", None),
        ("plain", "activeexternalspk", None),
        ("plain", "tx", None),
        ("plain", "bestblock", None),
        ("labels", "name", None),
        ("labels", "purpose", None),
        ("encrypted", "mkey", Some("faraday")),
        ("encrypted", "walletdescriptorckey", Some("faraday")),
    ] {
        let once = read_with(wallet, passphrase, |_| {});
        let twice = read_with(wallet, passphrase, |records| {
            let mut again = record_of(records, record);
            if record == "flags" {
                // The second value is the one a last-wins reader would keep.
                again.1 = vec![0; 8];
            }
            records.push(again);
        });
        assert!(failed(&twice, record, Fault::Repeated), "{wallet} {record}");
        assert!(!twice.is_whole(), "{wallet} {record}");
        // The repeat is neither read nor counted again.
        assert_eq!(twice.flags, once.flags, "{record}");
        assert_eq!(twice.keys_passed, once.keys_passed, "{record}");
        assert_eq!(twice.tx_count, once.tx_count, "{record}");
        assert_eq!(twice.descriptors, once.descriptors, "{record}");
    }
    // A record type Faraday does not know is counted once.
    let unknown = read_with("plain", None, |records| {
        records.push((b"\x06mystic".to_vec(), vec![1]));
        records.push((b"\x06mystic".to_vec(), vec![2]));
    });
    assert!(failed(&unknown, "mystic", Fault::Repeated));
    assert_eq!(unknown.unknown, vec![("mystic".to_string(), 1)]);
    // A repeated minversion record.
    let min = read_with("plain", None, |records| {
        records.push((b"\x0aminversion".to_vec(), vec![0; 4]));
        records.push((b"\x0aminversion".to_vec(), vec![1, 0, 0, 0]));
    });
    assert!(failed(&min, "minversion", Fault::Repeated));
}

#[test]
fn a_block_locator_with_a_count_past_the_file_is_refused() {
    // 2^59 + 1 hashes: 32 times the count wraps to 32 in 64 bits.
    let wallet = read_with("plain", None, |records| {
        let (k, v) = records
            .iter_mut()
            .find(|(k, _)| k.get(1..=18) == Some(b"bestblock_nomerkle".as_slice()))
            .expect("locator");
        let _ = k;
        let mut value = vec![0u8; 4];
        value.push(0xff);
        value.extend_from_slice(&((1u64 << 59) + 1).to_le_bytes());
        value.extend_from_slice(&[0; 32]);
        *v = value;
    });
    assert!(failed(&wallet, "bestblock_nomerkle", Fault::Layout));
    assert!(!wallet.is_whole());
    assert!(wallet.block_locator.is_none_or(|n| n < 1_000));
}

#[test]
fn a_creation_time_past_i64_is_read_unsigned_as_core_lists_it() {
    // Bitcoin Core writes the time as a uint64_t and lists it as one: its
    // top bit set is a time past 2^63, not a negative one.
    let once = read_with("plain", None, |_| {});
    let wallet = read_with("plain", None, |records| {
        let (_, v) = records
            .iter_mut()
            .find(|(k, _)| k.get(1..=16) == Some(b"walletdescriptor".as_slice()) && k[0] == 16)
            .expect("descriptor");
        // The text, then the time (8 bytes), the next index and the range.
        let at = v.len() - 20 + 7;
        v[at] |= 0x80;
    });
    assert!(wallet.is_whole(), "{:?}", wallet.failures);
    let raised: Vec<u64> = wallet.descriptors.iter().map(|d| d.creation_time).collect();
    let before: Vec<u64> = once.descriptors.iter().map(|d| d.creation_time).collect();
    assert!(
        raised
            .iter()
            .any(|t| *t >= 1 << 63 && before.contains(&(t - (1 << 63))))
    );
}

#[test]
fn a_purpose_that_is_not_one_whole_utf8_string_fails_its_record() {
    // Bitcoin Core loads such a record and keeps the address with no
    // purpose; Faraday names it and does not call the wallet whole.
    for (what, change) in [
        (
            "not UTF-8",
            (|v: &mut Vec<u8>| v[3] |= 0x80) as fn(&mut Vec<u8>),
        ),
        ("bytes left over", |v: &mut Vec<u8>| v[0] -= 1),
    ] {
        let wallet = read_with("labels", None, |records| {
            let (_, v) = records
                .iter_mut()
                .find(|(k, _)| k.get(1..=7) == Some(b"purpose".as_slice()))
                .expect("purpose");
            assert_eq!(v.as_slice(), b"\x07receive");
            change(v);
        });
        assert!(failed(&wallet, "purpose", Fault::Layout), "{what}");
        assert!(!wallet.is_whole(), "{what}");
    }
}

#[test]
fn an_encrypted_wallet_holds_one_master_key_and_no_plain_key() {
    let two = read_with("encrypted", Some("faraday"), |records| {
        let (mut k, v) = record_of(records, "mkey");
        let last = k.len() - 4;
        k[last] = k[last].wrapping_add(1);
        records.push((k, v));
    });
    assert!(failed(&two, "mkey", Fault::ManyMasterKeys));
    assert_eq!(two.master_keys.len(), 2);
    assert!(!two.is_whole());

    // A plain private key beside the master key: Bitcoin Core does not
    // unlock such a wallet.
    let mixed = read_with("encrypted", Some("faraday"), |records| {
        let (mut k, _) = record_of(records, "walletdescriptorckey");
        // The record type is `walletdescriptorckey`, 20 bytes: drop the c.
        k.splice(0..21, [19].into_iter().chain(*b"walletdescriptorkey"));
        records.push((k, vec![0]));
    });
    assert!(failed(
        &mixed,
        "walletdescriptorkey",
        Fault::PlainKeyEncrypted
    ));
    assert!(!mixed.is_whole());
}

#[test]
fn a_master_key_that_cannot_open_is_refused_with_or_without_a_passphrase() {
    // The record is a 48-byte vector, an 8-byte salt, then the method and
    // the rounds as 32-bit numbers.
    let at = 1 + 48 + 1 + 8;
    for (method, rounds, fault) in [
        (1u32, 25_000u32, Fault::MasterMethod),
        (0, 0, Fault::MasterRounds),
        (0, 10_000_001, Fault::MasterRounds),
    ] {
        for passphrase in [None, Some("faraday")] {
            let wallet = read_with("encrypted", passphrase, |records| {
                let (_, v) = records
                    .iter_mut()
                    .find(|(k, _)| k.get(1..=4) == Some(b"mkey".as_slice()))
                    .expect("mkey");
                v[at..at + 4].copy_from_slice(&method.to_le_bytes());
                v[at + 4..at + 8].copy_from_slice(&rounds.to_le_bytes());
            });
            assert!(failed(&wallet, "mkey", fault), "{method} {rounds}");
            assert!(!wallet.is_whole(), "{method} {rounds}");
        }
    }
}

#[test]
fn a_wallet_with_an_uncompressed_key_is_read_as_core_wrote_it() {
    let wallet = load("uncompressed", None);
    assert!(wallet.is_whole(), "{:?}", wallet.failures);
    assert_eq!(wallet.keys_passed, wallet.descriptors.len());
    assert!(
        wallet
            .descriptors
            .iter()
            .any(|d| d.text.starts_with("pkh([d34db33f/44h/1h/0h]04"))
    );
    // A changed byte in the 65-byte public key's DER copy or the hash fails.
    let bad = read_with("uncompressed", None, |records| {
        let (_, v) = records
            .iter_mut()
            .find(|(k, v)| {
                k.get(1..=19) == Some(b"walletdescriptorkey".as_slice()) && v.len() > 300
            })
            .expect("uncompressed key");
        v[100] ^= 1;
    });
    assert!(!bad.is_whole());
    assert!(
        bad.failures
            .iter()
            .any(|f| f.record.starts_with("walletdescriptorkey"))
    );
}

fn dat(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../tools/vectors/core-wallet/{name}.dat",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("wallet.dat")
}

fn whole_file(
    bytes: &[u8],
    beside: &[String],
    passphrase: Option<&str>,
) -> Result<Wallet, Refusal> {
    let c = faraday_core::testkit::compute();
    let p = passphrase.map(SecretText::of);
    read_file(
        &c,
        "wallet.dat",
        bytes,
        beside,
        Network::Regtest,
        p.as_ref(),
    )
}

#[test]
fn a_wallet_dat_read_whole_gives_what_core_lists() {
    for name in ["plain", "encrypted"] {
        let wallet = whole_file(&dat(name), &[], Some("faraday")).expect(name);
        let (_, _, core) = dump(name);
        assert!(wallet.is_whole(), "{name}: {:?}", wallet.failures);
        let listed = core["listdescriptors"]["descriptors"]
            .as_array()
            .expect("list");
        assert_eq!(wallet.descriptors.len(), listed.len(), "{name}");
        for d in listed {
            assert!(
                wallet.descriptors.iter().any(|w| {
                    d["timestamp"].as_u64() == Some(w.creation_time)
                        && d["next_index"].as_i64().unwrap_or(0) == i64::from(w.next_index)
                        && d["range"].as_array().map(|r| r[1].as_i64())
                            == w.range.map(|r| Some(i64::from(r.1)))
                }),
                "{name}: {}",
                d["desc"]
            );
        }
        // The same wallet as the dump of its table reads.
        let mut from_dump = load(name, Some("faraday"));
        let mut from_file = wallet;
        for w in [&mut from_dump, &mut from_file] {
            w.descriptors.sort_by_key(|d| d.id);
        }
        assert_eq!(from_file.descriptors, from_dump.descriptors, "{name}");
        assert_eq!(from_file.keys_passed, from_dump.keys_passed, "{name}");
        assert_eq!(from_file.flags, from_dump.flags, "{name}");
        assert_eq!(from_file.is_encrypted(), name == "encrypted");
    }
}

#[test]
fn a_truncated_file_is_refused_and_named() {
    let bytes = dat("plain");
    for cut in [10, 99, 4096, bytes.len() - 4096, bytes.len() - 1] {
        let got = whole_file(&bytes[..cut], &[], None);
        let Err(Refusal::File(e)) = &got else {
            panic!("{cut}: {got:?}");
        };
        assert!(!got.as_ref().expect_err("refused").message().is_empty());
        if cut < 100 {
            assert_eq!(*e, faraday_sqlite::Error::NotSqlite);
        }
    }
}

#[test]
fn a_second_table_holding_rows_is_refused() {
    let bytes = std::fs::read(format!(
        "{}/../faraday-sqlite/tests/fixtures/other_table.db",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("database");
    assert_eq!(
        whole_file(&bytes, &[], None).err(),
        Some(Refusal::File(faraday_sqlite::Error::OtherTable(
            "extra".to_string()
        )))
    );
}

#[test]
fn a_file_that_is_not_sqlite_is_refused() {
    let text = b"Not a wallet at all, only text that is longer than a SQLite header is.".repeat(4);
    let got = whole_file(&text, &[], None).err();
    assert_eq!(got, Some(Refusal::File(faraday_sqlite::Error::NotSqlite)));
    assert!(got.expect("refused").message().contains("not a SQLite"));
}

#[test]
fn a_journal_or_log_beside_the_file_is_refused_by_its_name() {
    let bytes = dat("plain");
    for suffix in ["-journal", "-wal", "-shm"] {
        let name = format!("wallet.dat{suffix}");
        let got = whole_file(&bytes, &["other.txt".to_string(), name.clone()], None).err();
        assert_eq!(got, Some(Refusal::Beside(name.clone())));
        assert!(got.expect("refused").message().contains(&name));
    }
}

#[test]
fn a_database_version_other_than_cores_is_refused() {
    let mut bytes = dat("plain");
    assert!(whole_file(&bytes, &[], None).is_ok());
    bytes[63] = 1;
    let got = whole_file(&bytes, &[], None).err();
    assert_eq!(got, Some(Refusal::SchemaVersion(1)));
    assert!(got.expect("refused").message().contains("version 1"));
}

#[test]
fn a_journal_beside_the_file_is_refused_whatever_its_case() {
    let bytes = dat("plain");
    let got = whole_file(&bytes, &["Wallet.dat-JOURNAL".to_string()], None).err();
    assert_eq!(got, Some(Refusal::Beside("wallet.dat-journal".to_string())));
    // A name that only starts with it is another file.
    let other = whole_file(&bytes, &["wallet.dat-journal.bak".to_string()], None);
    assert!(other.is_ok());
}

#[test]
fn a_record_that_only_starts_with_flags_does_not_change_the_flags() {
    let real = read_with("plain", None, |_| {});
    let w = read_with("plain", None, |records| {
        records.push((b"\x05flags\x00".to_vec(), vec![0; 7]));
    });
    assert_eq!(w.flags, real.flags);
    assert!(!w.is_whole());
}

#[test]
fn failed_records_and_unknown_types_past_a_few_hundred_are_counted_not_kept() {
    let wallet = read_with("plain", None, |records| {
        for i in 0u32..2000 {
            // A key that is no record layout, a record type no one knows
            // (each its own, with a long name), and an `mkey` of no layout.
            let mut layout = vec![0xff];
            layout.extend([b'x'; 1000]);
            layout.extend(i.to_le_bytes());
            records.push((layout, vec![]));
            let mut name = format!("unknown{i}").into_bytes();
            name.extend([b'u'; 1000]);
            let mut key = vec![0xfd];
            key.extend((name.len() as u16).to_le_bytes());
            key.extend(name);
            records.push((key, vec![]));
        }
    });
    assert!(!wallet.is_whole());
    assert_eq!(wallet.failures.len(), 256);
    assert_eq!(wallet.failures_more, 2000 - 256);
    assert!(wallet.failures.iter().all(|f| f.record.len() < 400));
    assert_eq!(wallet.unknown_more, 2000);
}

#[test]
fn unknown_types_with_short_names_past_the_cap_are_counted_not_kept() {
    let wallet = read_with("plain", None, |records| {
        for i in 0u32..300 {
            let name = format!("t{i}").into_bytes();
            let mut key = vec![name.len() as u8];
            key.extend(name);
            records.push((key, vec![]));
        }
    });
    assert!(wallet.is_whole());
    assert_eq!(wallet.unknown.len(), 256);
    assert_eq!(wallet.unknown_more, 300 - 256);
}

/// The descriptors Core's `listdescriptors` gives without private keys,
/// sorted.
fn core_public(name: &str) -> Vec<String> {
    let (_, _, core) = dump(name);
    let mut out: Vec<String> = core["listdescriptors_public"]["descriptors"]
        .as_array()
        .expect("list")
        .iter()
        .map(|d| d["desc"].as_str().expect("desc").to_string())
        .collect();
    out.sort();
    out
}

fn listed(wallet: &Wallet) -> Vec<String> {
    let mut out: Vec<String> = wallet
        .descriptors
        .iter()
        .map(|d| d.listed.clone())
        .collect();
    out.sort();
    out
}

#[test]
fn each_descriptor_is_listed_with_its_origin_and_account_key_as_core_lists_it() {
    for name in [
        "plain",
        "encrypted",
        "watchonly",
        "blank",
        "labels",
        "imported",
        "uncompressed",
        "multisig",
    ] {
        let wallet = load(name, Some("faraday"));
        assert!(wallet.is_whole(), "{name}: {:?}", wallet.failures);
        assert_eq!(listed(&wallet), core_public(name), "{name}");
    }
    // An encrypted wallet without its passphrase still gives them.
    let locked = load("encrypted", None);
    assert_eq!(locked.passphrase, Passphrase::NotGiven);
    assert_eq!(listed(&locked), core_public("encrypted"));
    // A file's descriptors are the same as the dump's.
    let file = whole_file(&dat("plain"), &[], None).expect("plain");
    assert_eq!(listed(&file), core_public("plain"));
}

fn cache_record(records: &mut Records) -> &mut (Vec<u8>, Vec<u8>) {
    records
        .iter_mut()
        .find(|(k, _)| k.get(1..=23) == Some(b"walletdescriptorlhcache".as_slice()))
        .expect("a cache record")
}

#[test]
fn a_changed_cached_account_key_fails_where_the_keys_are_checked() {
    for (name, passphrase) in [("plain", None), ("encrypted", Some("faraday"))] {
        // A changed chain code and a changed public key.
        for at in [20, 60] {
            let wallet = read_with(name, passphrase, |records| {
                cache_record(records).1[at] ^= 1;
            });
            assert!(
                failed(&wallet, "walletdescriptorlhcache", Fault::CacheKey),
                "{name} {at}"
            );
            assert!(!wallet.is_whole(), "{name} {at}");
        }
    }
    // Not checked, the cached key is taken as it is: the wallet reads
    // whole and lists the changed key.
    let taken = read_with("encrypted", None, |records| {
        cache_record(records).1[20] ^= 1;
    });
    assert!(taken.is_whole(), "{:?}", taken.failures);
    assert_ne!(listed(&taken), core_public("encrypted"));
}

#[test]
fn a_descriptor_whose_account_key_is_not_in_the_file_is_refused_and_named() {
    let wallet = read_with("plain", None, |records| {
        let key = cache_record(records).0.clone();
        records.retain(|(k, _)| *k != key);
    });
    assert!(failed(&wallet, "walletdescriptor ", Fault::CacheMissing));
    assert!(!wallet.is_whole());
    assert_eq!(wallet.descriptors.len(), 7);
    // A descriptor with its origin needs no cached key.
    let watch = read_with("watchonly", None, |records| {
        records.retain(|(k, _)| k.get(1..=23) != Some(b"walletdescriptorlhcache".as_slice()));
    });
    assert!(watch.is_whole(), "{:?}", watch.failures);
    assert_eq!(listed(&watch), core_public("watchonly"));
}

#[test]
fn a_cached_key_no_key_record_checks_fails_where_the_keys_are_checked() {
    let drop_keys = |records: &mut Records| {
        records.retain(|(k, _)| k.get(1..=19) != Some(b"walletdescriptorkey".as_slice()));
    };
    let wallet = read_with("plain", None, drop_keys);
    assert!(failed(
        &wallet,
        "walletdescriptorlhcache",
        Fault::CacheUnchecked
    ));
    assert!(!wallet.is_whole());
    // An unencrypted watch-only wallet counts as checked, so a cached key with
    // no key record would fail CacheUnchecked; this fixture has no cached
    // keys, so it stays whole. It is not a test of that refusal.
    let watch = read_with("watchonly", None, drop_keys);
    assert!(watch.is_whole(), "{:?}", watch.failures);
    // The bit that says the wallet holds no private keys is no way past it.
    let flagged = read_with("plain", None, |records| {
        drop_keys(records);
        let flags = records
            .iter_mut()
            .find(|(k, _)| k.get(1..6) == Some(b"flags".as_slice()))
            .expect("flags");
        flags.1[4] |= 1;
    });
    assert!(failed(
        &flagged,
        "walletdescriptorlhcache",
        Fault::CacheUnchecked
    ));
    assert!(!flagged.is_whole());
}

#[test]
fn a_descriptor_of_two_master_keys_with_one_key_record_fails() {
    let whole = load("multisig", None);
    assert!(whole.is_whole(), "{:?}", whole.failures);
    // The second key of the 2-of-2 is another master key, which has no key record.
    let tpub = |records: &Records| -> Vec<u8> {
        let (_, v) = record_of(records, "walletdescriptor");
        let at = v.windows(4).position(|w| w == b"tpub").expect("tpub");
        v[at..at + 111].to_vec()
    };
    let other = tpub(&dump("plain").1);
    let wallet = read_with("multisig", None, |records| {
        let root = tpub(records);
        let mut renamed = None;
        for (k, v) in records.iter_mut() {
            let named = k.get(1..=16) == Some(b"walletdescriptor".as_slice());
            if !named || k[0] != 16 || !v.windows(6).any(|w| w == b"multi(") {
                continue;
            }
            let at = v.windows(111).rposition(|w| w == root).expect("root");
            v[at..at + 111].copy_from_slice(&other);
            // The text ends in its checksum, which is made again.
            let start = v.windows(4).position(|w| w == b"wsh(").expect("text");
            let hash = at + v[at..].iter().position(|b| *b == b'#').expect("checksum");
            let sum = faraday_bip::descriptor::descriptor_checksum(
                std::str::from_utf8(&v[start..hash]).expect("text"),
            )
            .expect("checksum");
            v[hash + 1..hash + 9].copy_from_slice(&sum);
            // Its id is the hash of the text, and every record of it is named by the id.
            // (the hash is of the text with apostrophes for hardened steps).
            let mut compat = std::str::from_utf8(&v[start..hash])
                .expect("text")
                .replace("h/", "'/")
                .replace("h]", "']");
            let apostrophes = faraday_bip::descriptor::descriptor_checksum(&compat).expect("sum");
            compat.push('#');
            compat.push_str(std::str::from_utf8(&apostrophes).expect("sum"));
            let new = faraday_crypto::sha256(compat.as_bytes());
            let old: [u8; 32] = k[17..49].try_into().expect("id");
            renamed = Some((old, new));
        }
        let (old, new) = renamed.expect("the descriptor");
        for (k, _) in records.iter_mut() {
            let at = 1 + usize::from(k[0]);
            if k.get(at..at + 32) == Some(old.as_slice()) && k[1..].starts_with(b"walletdescriptor")
            {
                k[at..at + 32].copy_from_slice(&new);
            }
        }
    });
    assert!(!wallet.is_whole());
    assert!(
        failed(&wallet, "walletdescriptorlhcache", Fault::CacheUnchecked),
        "{:?}",
        wallet.failures
    );
}

#[test]
fn a_cached_key_that_disagrees_with_its_path_or_the_curve_fails_where_keys_are_not_checked() {
    // The depth of the cached key.
    let deep = read_with("encrypted", None, |records| {
        cache_record(records).1[1] ^= 1;
    });
    assert!(
        failed(&deep, "walletdescriptor ", Fault::CacheKey),
        "{:?}",
        deep.failures
    );
    assert!(!deep.is_whole());
    // The child number.
    let child = read_with("encrypted", None, |records| {
        cache_record(records).1[9] ^= 1;
    });
    assert!(
        failed(&child, "walletdescriptor ", Fault::CacheKey),
        "{:?}",
        child.failures
    );
    // A key whose x is not on the curve: some value of the last byte is
    // not, and every one that is not fails the descriptor.
    let mut off = 0;
    for last in 0..=255u8 {
        let wallet = read_with("encrypted", None, |records| {
            cache_record(records).1[74] = last;
        });
        if !wallet.is_whole() {
            off += 1;
            assert!(
                failed(&wallet, "walletdescriptor ", Fault::CacheKey),
                "{last}"
            );
        }
    }
    assert!(off > 0);
}

#[test]
fn a_cache_record_that_is_not_the_shape_core_writes_fails_its_record() {
    for edit in [
        |r: &mut (Vec<u8>, Vec<u8>)| r.1.truncate(40),
        |r: &mut (Vec<u8>, Vec<u8>)| r.1.push(0),
        |r: &mut (Vec<u8>, Vec<u8>)| r.1[42] = 7,
        |r: &mut (Vec<u8>, Vec<u8>)| {
            r.0.pop();
        },
    ] {
        let wallet = read_with("plain", None, |records| edit(cache_record(records)));
        assert!(
            failed(&wallet, "walletdescriptorlhcache", Fault::Layout),
            "{:?}",
            wallet.failures
        );
        assert!(!wallet.is_whole());
    }
    // A cache record of a descriptor the file does not hold.
    let stray = read_with("plain", None, |records| {
        let mut again = cache_record(records).clone();
        again.0[29] ^= 1;
        records.push(again);
    });
    assert!(failed(&stray, "walletdescriptorlhcache", Fault::NotHeld));
}

#[test]
fn a_private_key_record_of_a_descriptor_the_file_does_not_hold_fails_its_record() {
    let stray = read_with("plain", None, |records| {
        let mut again = records
            .iter()
            .find(|(k, _)| k.get(1..20) == Some(b"walletdescriptorkey".as_slice()))
            .expect("a key record")
            .clone();
        again.0[21] ^= 1;
        records.push(again);
    });
    assert!(failed(&stray, "walletdescriptorkey", Fault::NotHeld));
    assert!(!stray.is_whole());
}

#[test]
fn a_descriptor_that_is_not_ranged_has_no_range() {
    let wallet = load("uncompressed", None);
    let single = wallet
        .descriptors
        .iter()
        .find(|d| d.text.starts_with("pkh([d34db33f"))
        .expect("the imported key");
    assert_eq!(single.range, None);
    assert!(wallet.descriptors.iter().any(|d| d.range.is_some()));
}

fn is_combo(text: &str) -> bool {
    text.starts_with("combo(")
}

#[test]
fn a_wallet_migrated_by_core_reads_whole_with_its_combo_descriptors_kept_not_taken() {
    let wallet = load("migrated", None);
    assert!(wallet.is_whole(), "{:?}", wallet.failures);
    let (_, _, core) = dump("migrated");
    let public: Vec<String> = core_public("migrated");
    let combos = public.iter().filter(|t| is_combo(t)).count();
    assert!(combos >= 2, "{public:?}");
    assert_eq!(wallet.kept.len(), combos);
    // The kept texts are Core's own combo list, checksums included.
    let mut kept: Vec<&str> = wallet.kept.iter().map(|k| k.text.as_str()).collect();
    kept.sort_unstable();
    let want: Vec<&str> = public
        .iter()
        .filter(|t| is_combo(t))
        .map(String::as_str)
        .collect();
    assert_eq!(kept, want);
    // Only public keys are listed: none of Core's private descriptors'
    // key text is in what was kept.
    for d in core["listdescriptors"]["descriptors"]
        .as_array()
        .expect("list")
    {
        let text = d["desc"].as_str().expect("desc");
        if is_combo(text) {
            let key = text
                .trim_start_matches("combo(")
                .split(['/', ')'])
                .next()
                .expect("key");
            let key = key.rsplit(']').next().expect("key");
            assert!(
                wallet.kept.iter().all(|k| !k.text.contains(key)),
                "a private key is listed: {key}"
            );
        }
    }
    // The other descriptors are as Core lists them, and none is a combo.
    let others: Vec<String> = public.iter().filter(|t| !is_combo(t)).cloned().collect();
    assert_eq!(listed(&wallet), others);
    assert_eq!(wallet.descriptors.len(), others.len());
    assert!(!wallet.active.is_empty());
}

#[test]
fn a_combo_descriptor_with_a_wrong_checksum_fails() {
    let wallet = read_with("migrated", None, |records| {
        let (key, value) = records
            .iter_mut()
            .find(|(k, v)| {
                k.get(1..17) == Some(b"walletdescriptor".as_slice())
                    && k.len() == 49
                    && v.get(1..7) == Some(b"combo(".as_slice())
            })
            .expect("a combo record");
        let _ = key;
        let last = usize::from(value[0]);
        value[last] ^= 1;
    });
    assert!(!wallet.is_whole());
    assert!(wallet.failures.iter().any(|f| f.fault == Fault::Checksum));
}

#[test]
fn the_private_key_of_a_combo_descriptor_is_still_checked() {
    let whole = load("migrated", None);
    let id = whole.kept[0].id;
    let wallet = read_with("migrated", None, |records| {
        let (_, value) = records
            .iter_mut()
            .find(|(k, _)| {
                k.get(1..20) == Some(b"walletdescriptorkey".as_slice()) && k[20..52] == id
            })
            .expect("a key record of the combo descriptor");
        let last = value.len() - 1;
        value[last] ^= 1;
    });
    assert!(!wallet.is_whole());
    assert!(
        wallet
            .failures
            .iter()
            .any(|f| f.record.starts_with("walletdescriptorkey") && f.fault == Fault::KeyHash)
    );
}

#[test]
fn a_descriptor_with_a_second_hash_sign_in_its_text_is_not_whole() {
    let whole = load("migrated", None);
    let old = whole.kept[0].text.clone();
    let (body, _) = old.split_once('#').expect("checksum");
    let id = whole.kept[0].id;
    // Text after a first `#` that carries a valid checksum of everything
    // before the last `#`: Bitcoin Core refuses it.
    let junk = format!("{body}#junk");
    let sum = faraday_bip::descriptor::descriptor_checksum(&junk).expect("checksum");
    let text = format!("{junk}#{}", std::str::from_utf8(&sum).expect("ascii"));
    let wallet = read_with("migrated", None, |records| {
        let (_, value) = records
            .iter_mut()
            .find(|(k, v)| {
                k.get(1..17) == Some(b"walletdescriptor".as_slice())
                    && k.len() == 49
                    && k[17..49] == id
                    && v.get(1..7) == Some(b"combo(".as_slice())
            })
            .expect("the combo record");
        let mut out = vec![u8::try_from(text.len()).expect("short")];
        out.extend(text.as_bytes());
        out.extend(&value[1 + usize::from(value[0])..]);
        *value = out;
    });
    assert!(!wallet.is_whole());
    assert!(failed(&wallet, "walletdescriptor", Fault::Checksum));
}

#[test]
fn an_active_record_naming_a_combo_descriptor_is_not_held() {
    let whole = load("migrated", None);
    let id = whole.kept[0].id;
    let wallet = read_with("migrated", None, |records| {
        let (_, value) = records
            .iter_mut()
            .find(|(k, _)| k.starts_with(b"\x11activeexternalspk"))
            .expect("an active record");
        value.copy_from_slice(&id);
    });
    assert!(!wallet.is_whole());
    assert!(wallet.failures.iter().any(|f| f.fault == Fault::NotHeld));
    assert!(wallet.active.iter().all(|a| a.id != id));
}

/// Reads "uncompressed" with the next index, range start and range end of the
/// record whose text starts with `prefix` set as given.
fn with_stored_range(prefix: &str, next: i32, start: i32, end: i32) -> Wallet {
    read_with("uncompressed", None, |records| {
        let (_, value) = records
            .iter_mut()
            .find(|(k, v)| {
                k.get(1..17) == Some(b"walletdescriptor".as_slice())
                    && k.len() == 49
                    && v.get(1..=prefix.len()) == Some(prefix.as_bytes())
            })
            .expect("the descriptor record");
        let at = value.len() - 12;
        value[at..at + 4].copy_from_slice(&next.to_le_bytes());
        value[at + 4..at + 8].copy_from_slice(&start.to_le_bytes());
        value[at + 8..].copy_from_slice(&end.to_le_bytes());
    })
}

#[test]
fn a_descriptor_with_no_wildcard_that_stores_a_range_of_many_indexes_is_refused() {
    let wallet = with_stored_range("pkh([d34db33f", 0, 0, 5);
    assert!(!wallet.is_whole());
    assert!(failed(&wallet, "walletdescriptor", Fault::Range));
    assert!(wallet.descriptors.iter().all(|d| !d.text.starts_with("pkh([d34db33f")));
    // One index, or none, is not refused.
    for end in [0, 1] {
        assert!(with_stored_range("pkh([d34db33f", 0, 0, end).is_whole());
    }
}

#[test]
fn a_range_of_a_million_indexes_or_a_next_index_of_a_million_is_refused() {
    // The stored end is one past the last index: Core's size is end - 1 - start.
    for (next, start, end) in [(0, 0, 1_000_001), (1_000_000, 0, 10), (0, 5, 1_000_006)] {
        let wallet = with_stored_range("wpkh(", next, start, end);
        assert!(failed(&wallet, "walletdescriptor", Fault::Range), "{next} {start} {end}");
    }
    // A range of 999,999 and a small one that starts high are read whole.
    for (next, start, end) in [(999_999, 0, 1_000_000), (0, 5_000_000, 5_000_010)] {
        let wallet = with_stored_range("wpkh(", next, start, end);
        assert!(!failed(&wallet, "walletdescriptor", Fault::Range), "{next} {start} {end}");
    }
}
