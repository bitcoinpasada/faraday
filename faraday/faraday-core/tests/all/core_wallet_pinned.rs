//! What a person opening a Bitcoin Core wallet file notices does not change
//! when the reader is reorganised: every wallet file of the repository's
//! vectors and samples, and the refusals of a changed one, give the same
//! wallet or the same refusal. Each case's outcome is written out in full in
//! `core_wallet_pinned.txt` (public data only: no key value is in a wallet
//! or in a refusal), and a case that differs is named with both outcomes.
//! A reader changed on purpose has the file written again by running the
//! test with `FARADAY_PIN_WRITE` set, and the file's diff is the change.

use faraday_bip::keys::Network;
use faraday_core::core_wallet::{Refusal, Wallet, read, read_file};
use faraday_intake::secret_buf::SecretText;

type Records = Vec<(Vec<u8>, Vec<u8>)>;

const V: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/vectors/core-wallet"
);

fn hex_of(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn dump(name: &str) -> (u32, Records) {
    let text = std::fs::read_to_string(format!("{V}/{name}.json")).expect("dump");
    let json: serde_json::Value = serde_json::from_str(&text).expect("json");
    let records = json["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|r| {
            (
                hex_of(r[0].as_str().expect("key")),
                hex_of(r[1].as_str().expect("value")),
            )
        })
        .collect();
    (json["application_id"].as_u64().expect("id") as u32, records)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn names(list: &[(faraday_text::Text, usize)]) -> String {
    let mut s = String::new();
    for (n, k) in list {
        s += &format!(" {:?} x{k}", n.as_str());
    }
    s
}

/// The outcome as text, every field of it: the wallet or the refusal.
fn outcome(got: Result<Wallet, Refusal>) -> String {
    let w = match got {
        Ok(w) => w,
        Err(e) => {
            return match e {
                Refusal::Legacy(found) => format!("refused Legacy{}\n", names(&found)),
                Refusal::NoOrigin(r) => format!("refused NoOrigin {:?}\n", r.as_str()),
                Refusal::Beside(r) => format!("refused Beside {:?}\n", r.as_str()),
                other => format!("refused {other:?}\n"),
            };
        }
    };
    let mut s = format!(
        "accepted {:?} version {:?} flags {:#x} passphrase {:?}\n",
        w.network, w.version, w.flags, w.passphrase
    );
    for d in &w.descriptors {
        s += &format!(
            "descriptor {} time {} next {} range {:?} private {} encrypted {}\n  text {:?}\n  listed {:?}\n",
            hex(&d.id),
            d.creation_time,
            d.next_index,
            d.range,
            d.private_keys,
            d.encrypted_keys,
            d.text.as_str(),
            d.listed.as_str()
        );
    }
    for k in &w.kept {
        s += &format!("kept {} {:?}\n", hex(&k.id), k.text.as_str());
    }
    for a in &w.active {
        s += &format!(
            "active internal {} {:?} {}\n",
            a.internal,
            a.output_type,
            hex(&a.id)
        );
    }
    for l in &w.labels {
        s += &format!(
            "label {:?} name {:?} purpose {:?}\n",
            l.address.as_str(),
            l.name.as_ref().map(|t| t.as_str()),
            l.purpose.as_ref().map(|t| t.as_str())
        );
    }
    for m in &w.master_keys {
        s += &format!("master {m:?}\n");
    }
    s += &format!(
        "keys passed {} unchecked {} tx {} locator {:?}\n",
        w.keys_passed, w.keys_unchecked, w.tx_count, w.block_locator
    );
    s += &format!("unread{}\n", names(&w.unread));
    s += &format!("unknown{} more {}\n", names(&w.unknown), w.unknown_more);
    for f in &w.failures {
        s += &format!("failure {:?} {:?}\n", f.fault, f.record.as_str());
    }
    s += &format!("failures more {}\n", w.failures_more);
    s
}

fn records_read(id: u32, records: &Records, mode: Network, pass: Option<&str>) -> String {
    let c = faraday_core::testkit::compute();
    let p = pass.map(SecretText::of);
    outcome(read(&c, id, records, mode, p.as_ref()))
}

fn file_read(bytes: &[u8], beside: &[String], mode: Network, pass: Option<&str>) -> String {
    let c = faraday_core::testkit::compute();
    let p = pass.map(SecretText::of);
    outcome(read_file(&c, "wallet.dat", bytes, beside, mode, p.as_ref()))
}

/// Whether a record key is of the type `name`: its length byte and its name.
fn is_named(key: &[u8], name: &str) -> bool {
    key.first() == Some(&(name.len() as u8)) && key.get(1..=name.len()) == Some(name.as_bytes())
}

fn find(records: &Records, name: &str) -> Option<usize> {
    records.iter().position(|(k, _)| is_named(k, name))
}

fn named(records: &Records, name: &str) -> usize {
    find(records, name).unwrap_or_else(|| panic!("no {name} record"))
}

const PASSES: [Option<&str>; 3] = [None, Some("faraday"), Some("faradaz")];
const MODES: [(&str, Network); 2] = [("regtest", Network::Regtest), ("mainnet", Network::Mainnet)];

/// Every case's name and outcome.
fn cases() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut add = |name: String, text: String| out.push((name, text));
    let mut dumps: Vec<String> = std::fs::read_dir(V)
        .expect("vectors")
        .filter_map(|e| {
            let n = e.expect("entry").file_name().into_string().expect("name");
            n.strip_suffix(".json").map(str::to_string)
        })
        .collect();
    dumps.sort();
    for name in &dumps {
        let (id, records) = dump(name);
        for (m, mode) in MODES {
            for pass in PASSES {
                add(
                    format!("records {name} {m} {pass:?}"),
                    records_read(id, &records, mode, pass),
                );
            }
        }
    }
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for name in ["encrypted", "migrated", "multisig", "plain", "watchonly"] {
        let path = format!("{V}/{name}.dat");
        files.push((name.to_string(), std::fs::read(path).expect("dat")));
    }
    for name in ["encrypted", "plain"] {
        let path = format!(
            "{}/../faraday-samples/data/core-wallet-{name}.dat",
            env!("CARGO_MANIFEST_DIR")
        );
        files.push((
            format!("sample-{name}"),
            std::fs::read(path).expect("sample"),
        ));
    }
    for (name, bytes) in &files {
        for (m, mode) in MODES {
            for pass in PASSES {
                add(
                    format!("file {name} {m} {pass:?}"),
                    file_read(bytes, &[], mode, pass),
                );
            }
        }
    }
    // Refusals and changed records.
    let plain = std::fs::read(format!("{V}/plain.dat")).expect("dat");
    for cut in [10, 99, 4096, plain.len() - 4096, plain.len() - 1] {
        add(
            format!("truncated {cut}"),
            file_read(&plain[..cut], &[], Network::Regtest, None),
        );
    }
    for beside in [
        "wallet.dat-journal",
        "WALLET.DAT-WAL",
        "wallet.dat-shm",
        "other.dat",
    ] {
        add(
            format!("beside {beside}"),
            file_read(&plain, &[beside.to_string()], Network::Regtest, None),
        );
    }
    let text = b"Not a wallet at all, only text that is longer than a SQLite header is.".repeat(4);
    add(
        "not sqlite".into(),
        file_read(&text, &[], Network::Regtest, None),
    );
    let berkeley = [vec![0u8; 12], vec![0x62, 0x31, 0x05, 0x00], vec![0u8; 4096]].concat();
    add(
        "berkeley".into(),
        file_read(&berkeley, &[], Network::Regtest, None),
    );
    let other = std::fs::read(format!(
        "{}/../faraday-sqlite/tests/fixtures/other_table.db",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("database");
    add(
        "other table".into(),
        file_read(&other, &[], Network::Regtest, None),
    );
    let mut version = plain.clone();
    version[60..64].copy_from_slice(&7u32.to_be_bytes());
    add(
        "schema version".into(),
        file_read(&version, &[], Network::Regtest, None),
    );

    for (base, pass) in [
        ("plain", None),
        ("encrypted", Some("faraday")),
        ("migrated", None),
    ] {
        let (id, records) = dump(base);
        add(
            format!("{base} unknown application"),
            records_read(0x1234_5678, &records, Network::Regtest, pass),
        );
        let key_type = if base == "encrypted" {
            "walletdescriptorckey"
        } else {
            "walletdescriptorkey"
        };
        // A form that changes nothing in this base is not a case of it.
        let mut run = |label: &str, edit: &dyn Fn(&mut Records)| {
            let mut r = records.clone();
            edit(&mut r);
            if r == records {
                return;
            }
            add(
                format!("{base} {label}"),
                records_read(id, &r, Network::Regtest, pass),
            );
        };
        run("legacy", &|r| r.push((b"\x03key".to_vec(), vec![0])));
        run("legacy pool", &|r| r.push((b"\x04pool".to_vec(), vec![0])));
        run("no flags", &|r| r.retain(|(k, _)| !is_named(k, "flags")));
        run("flags short", &|r| {
            let i = named(r, "flags");
            r[i].1.pop();
        });
        run("flags unknown bit", &|r| {
            let i = named(r, "flags");
            r[i].1[7] = 0x80;
        });
        run("flags not descriptors", &|r| {
            let i = named(r, "flags");
            r[i].1 = vec![0; 8];
        });
        run("flags with a tail", &|r| {
            let (mut k, v) = r[named(r, "flags")].clone();
            k.push(1);
            r.push((k, v));
        });
        run("repeated first", &|r| r.push(r[0].clone()));
        run("repeated flags", &|r| {
            let one = r[named(r, "flags")].clone();
            r.push(one);
        });
        run("empty key", &|r| r.push((Vec::new(), vec![1])));
        run("version short", &|r| {
            let i = named(r, "version");
            r[i].1.pop();
        });
        run("minversion short", &|r| {
            let Some(i) = find(r, "minversion") else {
                return;
            };
            r[i].1.pop();
        });
        run("unknown type", &|r| {
            r.push((b"\x05zzzzz".to_vec(), vec![1, 2]))
        });
        run("unread type", &|r| {
            r.push((b"\x08settings".to_vec(), vec![1]))
        });
        run("tx", &|r| r.push((b"\x02tx\x01".to_vec(), vec![1])));
        run("bestblock layout", &|r| {
            let i = named(r, "bestblock");
            r[i].1 = vec![0; 7];
        });
        run("bestblock two hashes", &|r| {
            let i = named(r, "bestblock");
            r[i].1 = [vec![0; 4], vec![2], vec![0; 64]].concat();
        });
        run("bestblock with a tail", &|r| {
            let i = named(r, "bestblock");
            r[i].0.push(1);
        });
        run("bestblock dropped", &|r| {
            r.retain(|(k, _)| !is_named(k, "bestblock"))
        });
        run("bestblock nomerkle layout", &|r| {
            let i = named(r, "bestblock_nomerkle");
            r[i].1.pop();
        });
        run("bestblock nomerkle dropped", &|r| {
            r.retain(|(k, _)| !is_named(k, "bestblock_nomerkle"))
        });
        run("key flipped", &|r| {
            let Some(i) = find(r, key_type) else { return };
            r[i].1[20] ^= 1;
        });
        run("descriptor flipped", &|r| {
            let i = named(r, "walletdescriptor");
            r[i].1[100] ^= 1;
        });
        run("descriptor id flipped", &|r| {
            let i = named(r, "walletdescriptor");
            let last = r[i].0.len() - 1;
            r[i].0[last] ^= 1;
        });
        run("cache dropped", &|r| {
            r.retain(|(k, _)| !is_named(k, "walletdescriptorlhcache"))
        });
        run("cache layout", &|r| {
            let i = named(r, "walletdescriptorlhcache");
            r[i].1.pop();
        });
        run("active short", &|r| {
            let i = named(r, "activeexternalspk");
            r[i].1.truncate(3);
        });
        run("label odd", &|r| {
            r.push((b"\x04name\x01".to_vec(), vec![0xff, 0xfe]))
        });
        run("purpose odd", &|r| {
            r.push((b"\x07purpose\x02".to_vec(), vec![0xc3]))
        });
    }

    // The master keys. A master key's value is a 48-byte vector, an 8-byte
    // salt, then the method and the rounds as 32-bit numbers.
    let (id, records) = dump("encrypted");
    let mk = named(&records, "mkey");
    let at = 1 + 48 + 1 + 8;
    for (label, edit) in [
        ("mkey layout", 0usize),
        ("mkey method", 1),
        ("mkey rounds zero", 2),
        ("mkey rounds past", 3),
        ("mkey second", 4),
        ("mkey two layouts", 5),
        ("mkey dropped", 6),
        ("mkey id changed", 7),
        ("mkey key tail", 8),
    ] {
        let mut r = records.clone();
        let value = r[mk].1.clone();
        let n = value.len();
        match edit {
            0 => r[mk].1.truncate(n - 3),
            1 => r[mk].1[at..at + 4].copy_from_slice(&1u32.to_le_bytes()),
            2 => r[mk].1[at + 4..at + 8].copy_from_slice(&0u32.to_le_bytes()),
            3 => r[mk].1[at + 4..at + 8].copy_from_slice(&10_000_001u32.to_le_bytes()),
            4 => {
                let mut k = r[mk].0.clone();
                let last = k.len() - 1;
                k[last] ^= 0x02;
                r.push((k, value));
            }
            5 => {
                let mut k = r[mk].0.clone();
                let last = k.len() - 1;
                k[last] ^= 0x02;
                r.push((k, vec![0; 5]));
            }
            6 => {
                r.remove(mk);
            }
            7 => {
                let last = r[mk].0.len() - 1;
                r[mk].0[last] ^= 0x04;
            }
            _ => r[mk].0.push(0),
        }
        for pass in PASSES {
            add(
                format!("{label} {pass:?}"),
                records_read(id, &r, Network::Regtest, pass),
            );
        }
    }
    // A plain key of another wallet beside the master key, a plain key of
    // this wallet there, and an encrypted key without a master key.
    let (_, plain_records) = dump("plain");
    let mut foreign = records.clone();
    foreign.push(plain_records[named(&plain_records, "walletdescriptorkey")].clone());
    let mut mixed = records.clone();
    let (mut k, _) = records[named(&records, "walletdescriptorckey")].clone();
    // The record type is `walletdescriptorckey`, 20 bytes: drop the c.
    k.splice(0..21, [19].into_iter().chain(*b"walletdescriptorkey"));
    mixed.push((k, vec![0]));
    let mut stripped = records.clone();
    stripped.retain(|(k, _)| !is_named(k, "mkey"));
    for (label, r) in [
        ("foreign plain key in encrypted", &foreign),
        ("plain key in encrypted", &mixed),
        ("encrypted without mkey", &stripped),
    ] {
        for pass in PASSES {
            add(
                format!("{label} {pass:?}"),
                records_read(id, r, Network::Regtest, pass),
            );
        }
    }
    // A key value past the limit, a key whose pubkey does not read, and a
    // key whose descriptor the file does not hold.
    for base in ["plain", "encrypted"] {
        let (id, records) = dump(base);
        let name = if base == "plain" {
            "walletdescriptorkey"
        } else {
            "walletdescriptorckey"
        };
        let i = named(&records, name);
        let mut big = records.clone();
        big[i].1 = vec![7; 70_000];
        add(
            format!("{base} key value huge"),
            records_read(id, &big, Network::Regtest, Some("faraday")),
        );
        let mut short = records.clone();
        short[i].0.truncate(1 + name.len() + 32);
        add(
            format!("{base} key tail short"),
            records_read(id, &short, Network::Regtest, Some("faraday")),
        );
        let mut orphan = records.clone();
        orphan[i].0[1 + name.len()] ^= 1;
        add(
            format!("{base} key orphan"),
            records_read(id, &orphan, Network::Regtest, Some("faraday")),
        );
    }
    out
}

const EXPECTED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/all/core_wallet_pinned.txt"
);

/// The cases as a file: a `## name` line, then the outcome.
fn written(got: &[(String, String)]) -> String {
    got.iter().map(|(n, t)| format!("## {n}\n{t}")).collect()
}

#[test]
fn every_wallet_file_and_refusal_reads_as_it_did() {
    let got = cases();
    let all = written(&got);
    if std::env::var_os("FARADAY_PIN_WRITE").is_some() {
        std::fs::write(EXPECTED, &all).expect("write");
        return;
    }
    let expected = std::fs::read_to_string(EXPECTED).expect("the pinned outcomes");
    if all == expected {
        return;
    }
    let mut was: Vec<(&str, &str)> = Vec::new();
    for part in expected.split("## ").skip(1) {
        let (name, text) = part.split_once('\n').expect("case");
        was.push((name, text));
    }
    let mut differ = Vec::new();
    for (n, t) in &got {
        match was.iter().find(|(m, _)| m == n) {
            Some((_, u)) if u == t => {}
            Some((_, u)) => differ.push(format!("{n}\nwas:\n{u}now:\n{t}")),
            None => differ.push(format!("{n}\nnew case:\n{t}")),
        }
    }
    for (n, _) in &was {
        if !got.iter().any(|(m, _)| m == n) {
            differ.push(format!("{n}\nno longer a case"));
        }
    }
    differ.truncate(5);
    panic!(
        "{} cases differ; the first:\n{}",
        differ.len(),
        differ.join("\n")
    );
}
