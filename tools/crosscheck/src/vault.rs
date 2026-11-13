//! What of the vault the cross-check still asks of Faraday's code
//! (`docs/VAULT.md`): Argon2id through `faraday_crypto::argon2id`, a
//! passphrase's NFC form and the Unicode version `faraday_vault` makes it
//! under, and the payload layouts of §7 through `faraday_vault::layouts`.
//!
//! The vault file (`docs/VAULT.md` §7): `vault.create`, `vault.seal` and
//! `vault.open` through `faraday_vault::file` and the document model it
//! writes, for the outside readers of `area_vault.py` (pykeepass and
//! `keepassxc-cli`) to read and to write for.
//!
//! Passphrases travel as JSON text.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::Argon2FileCost as Cost;
use faraday_crypto::work::Compute;
use faraday_text::Text;
use faraday_vault::Record;
use faraday_vault::child::{self, ChildOf};
use faraday_vault::codex32::{SetFacts, State, set_payload};
use faraday_vault::file::{self, Change};
use faraday_vault::kdbx::items::{self, Body, GpgKey, Place};
use faraday_vault::kdbx::xml::{Document, Group};
use faraday_vault::kdbx::{self, Contents, RANDOM_LEN};
use faraday_vault::layouts::{self, Opened};
use faraday_vault::records::{BACKUP_PENDING, NO_LOAD_AT_UNLOCK, field, kind};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn cost(req: &Value) -> Result<Cost, String> {
    let n = |k: &str| u32::try_from(num(req, k)).map_err(err);
    Ok(Cost {
        memory_kib: n("memory_kib")?,
        passes: n("passes")?,
        lanes: n("lanes")?,
    })
}

/// Handles `vault.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // Argon2id, version 1.3, 32 bytes.
        "argon2id" => {
            let k = faraday_crypto::argon2id(
                &c,
                &cost(req)?,
                &bytes(req, "password"),
                &bytes(req, "salt"),
            )
            .map_err(err)?;
            Ok(json!({"key": hex(k.expose())}))
        }
        // The Unicode version the passphrase is put in NFC under.
        "unicode_version" => {
            let (a, b, v) = faraday_vault::UNICODE_VERSION;
            Ok(json!({"version": format!("{a}.{b}.{v}")}))
        }
        // A passphrase's NFC form, the bytes Argon2id stretches (§3).
        "passphrase_nfc" => {
            let p = req["passphrase"].as_str().ok_or("no passphrase")?;
            let nfc = faraday_vault::kdbx::passphrase_bytes(p).map_err(err)?;
            Ok(json!({"nfc": hex(&nfc)}))
        }
        "create" => create(&c, req),
        "export" => export(&c, req),
        "seal" => seal(&c, req),
        "open" => open(&c, req),
        _ => layout(name, req),
    }
}

const XPUB: &str = "[73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V";
const DER: &[u8] = &[0x30, 0x03, 0x02, 0x01, 0x01];
/// BIP 93's vector 2: a share of it and the fingerprint of its key.
const SHARE: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
const SHARE_KEY: [u8; 4] = [0xfa, 0xb6, 0x86, 0x8a];

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or_else(|| panic!("{k} is not text"))
}

fn opt<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

/// What an item of the request puts in the vault.
enum Put {
    Entry(Body, [u8; 16], String, String),
    Plan(Record),
    Signed(Record),
}

/// The item a request names, as `file::seal`'s changes and
/// `items::put` take it: its body, group and title.
fn item(n: usize, v: &Value) -> Put {
    let uuid = [(n as u8) + 1; 16];
    let group = |g: &str| (uuid, g.to_string());
    let rec = |b: Record, g: &str, title: &str| {
        let (u, g) = group(g);
        Put::Entry(Body::Record(b), u, g, title.to_string())
    };
    let hexed = |k: &str| bytes(v, k);
    match s(v, "kind") {
        "entry" => {
            let mut r = Record::new(kind::ENTRY)
                .with(field::TITLE, s(v, "title").as_bytes())
                .with(field::USERNAME, s(v, "user").as_bytes())
                .with(field::PASSWORD, s(v, "password").as_bytes())
                .with(field::URL, s(v, "url").as_bytes())
                .with(field::NOTES, s(v, "notes").as_bytes());
            if let Some(t) = opt(v, "totp") {
                r = r.with(field::TOTP, t.as_bytes());
            }
            rec(r, "", "")
        }
        "note" => rec(
            Record::new(kind::NOTE).with(field::NOTE, s(v, "text").as_bytes()),
            "Notes",
            "Note",
        ),
        "seed_words" => {
            let lang = Language::ALL[num(v, "language") as usize];
            let m = Mnemonic::from_entropy(lang, &hexed("entropy")).expect("entropy");
            let mut flags = 0;
            if v["no_load"].as_bool() == Some(true) {
                flags |= NO_LOAD_AT_UNLOCK;
            }
            if v["held"].as_bool() == Some(true) {
                flags |= BACKUP_PENDING;
            }
            let mut r = Record::new(kind::KEY)
                .with(field::KEY, &layouts::words_payload(&m))
                .with(field::KEY_LABEL, s(v, "label").as_bytes())
                .with(field::KEY_FLAGS, &[flags]);
            if let Some(p) = opt(v, "passphrase") {
                r = r.with(field::KEY_PASSPHRASE, p.as_bytes());
            }
            if v["codex32"].as_bool() == Some(true) {
                let facts = SetFacts {
                    threshold: 2,
                    identifier: *b"name",
                    length: 48,
                    unknown: 1,
                    strings: vec![(b'a', State::Placed), (b'c', State::Made)],
                };
                r = r.with(field::KEY_CODEX32, &set_payload(&facts).expect("a set"));
            }
            if v["child"].as_bool() == Some(true) {
                let of = ChildOf {
                    parent: [1, 2, 3, 4],
                    language: 0,
                    words: 12,
                    index: 7,
                };
                r = r.with(field::KEY_CHILD, &child::payload(&of).expect("a child"));
            }
            rec(r, "Bitcoin keys", "")
        }
        "seed_hex" => rec(
            Record::new(kind::KEY).with(
                field::KEY,
                &layouts::seed_payload(&hexed("seed")).expect("16 to 64 bytes"),
            ),
            "Bitcoin keys",
            s(v, "title"),
        ),
        "wallet_policy" => rec(
            Record::new(kind::WALLET)
                .with(field::WALLET, format!("wpkh(@0/**)\n{XPUB}").as_bytes())
                .with(field::WALLET_NAME, s(v, "name").as_bytes()),
            "Wallets",
            "",
        ),
        "wallet_descriptor" => rec(
            Record::new(kind::WALLET)
                .with(field::WALLET, format!("wpkh({XPUB}/<0;1>/*)").as_bytes())
                .with(field::WALLET_NAME, s(v, "name").as_bytes()),
            "Wallets",
            "",
        ),
        "sheet" => {
            let p = layouts::sheet_payload(
                s(v, "descriptor").as_bytes(),
                s(v, "name").as_bytes(),
                s(v, "note").as_bytes(),
            )
            .expect("a small sheet");
            rec(
                Record::new(kind::SHEET).with(field::SHEET, &p),
                "Recovery sheets",
                "",
            )
        }
        "gpg" => {
            let (u, g) = group("GPG keys");
            Put::Entry(
                Body::Gpg(GpgKey {
                    title: Text::of(s(v, "title")),
                    key: Text::of(s(v, "key")),
                    public: opt(v, "public").map(Text::of),
                    revocation: opt(v, "revocation").map(Text::of),
                    revoked: opt(v, "revoked").map(Text::of),
                }),
                u,
                g,
                String::new(),
            )
        }
        "secure_boot" => {
            let mut r = Record::new(kind::SECURE_BOOT).with(1, &hexed("owner"));
            for n in 2..=7 {
                r.push(n, DER);
            }
            rec(r, "Secure Boot", "Secure Boot keys")
        }
        "wallet_share" => {
            let mut r = Record::new(kind::WALLET_SHARE)
                .with(field::SHARE, s(v, "text").as_bytes())
                .with(field::SHARE_WALLET_NAME, s(v, "name").as_bytes())
                .with(field::SHARE_NUMBER, &[num(v, "index") as u8])
                .with(field::SHARE_OF, &[num(v, "of") as u8]);
            if let Some(w) = opt(v, "wallet") {
                r = r.with(field::SHARE_WALLET, w.as_bytes());
            }
            rec(r, "Wallet shares", "")
        }
        "passphrase" => {
            let key = hexed("key");
            rec(
                Record::new(kind::PASSPHRASE)
                    .with(field::PASSPHRASE_KEY, &key)
                    .with(field::PASSPHRASE, s(v, "text").as_bytes()),
                "Passphrases",
                &format!("Passphrase for {}", hex(&key)),
            )
        }
        "share" => rec(
            Record::new(kind::CODEX32_SHARE)
                .with(field::CODEX32_STRING, SHARE.as_bytes())
                .with(field::CODEX32_KEY, &SHARE_KEY),
            "Codex32 shares",
            &format!("Codex32 share for {}", hex(&SHARE_KEY)),
        ),
        "plan" => Put::Plan(
            Record::new(kind::PLAN)
                .with(
                    field::PLAN_WALLET,
                    format!("wpkh({XPUB}/<0;1>/*)").as_bytes(),
                )
                .with(field::PLAN_ANSWERS, b"seeds 1\nplaces 2")
                .with(field::PLAN_PLACE, s(v, "place").as_bytes()),
        ),
        "signed" => Put::Signed(
            Record::new(kind::AMOUNTS)
                .with(field::AMOUNTS_TX, &hexed("tx"))
                .with(field::AMOUNTS_INPUT, &num(v, "first").to_le_bytes())
                .with(field::AMOUNTS_INPUT, &num(v, "second").to_le_bytes()),
        ),
        other => panic!("no item kind {other}"),
    }
}

fn random_of(req: &Value) -> [u8; RANDOM_LEN] {
    bytes(req, "random").try_into().expect("140 random bytes")
}

fn now_of(req: &Value) -> Option<u64> {
    req.get("now").and_then(Value::as_u64)
}

fn place<'a>(uuid: [u8; 16], group: &'a str, title: &'a str, now: Option<u64>) -> Place<'a> {
    Place {
        uuid,
        group,
        group_uuid: [uuid[0].wrapping_add(100); 16],
        title,
        now: now.map(|n| n as i64),
    }
}

/// Costs the vault's own Argon2id run: `file::create` of an empty vault at the
/// cost asked (its size follows its contents); with `items`, the same document the app seals (each
/// put through `items::put`, the plan and amounts through theirs) sealed by
/// `kdbx::seal` at that cost, since `file::seal` always takes the 64 MiB step.
/// Gives the file as hex.
fn create(c: &Compute, req: &Value) -> Answer {
    let id: [u8; 16] = bytes(req, "vault_id").try_into().expect("16 bytes");
    let label = s(req, "label");
    let passphrase = s(req, "passphrase");
    let cost = cost(req)?;
    let random = random_of(req);
    let now = now_of(req);
    let list: Vec<Put> = req["items"]
        .as_array()
        .map(|a| a.iter().enumerate().map(|(n, v)| item(n, v)).collect())
        .unwrap_or_default();
    if list.is_empty() {
        let file = file::create(c, id, label, cost, passphrase, &random, now).map_err(err)?;
        return Ok(json!({"file": hex(&file)}));
    }
    let mut doc = Document::new(id, label);
    items::stamp(&mut doc, 1, now.and_then(items::utc_time).as_deref()).map_err(err)?;
    let mut groups: Vec<(String, [u8; 16])> = vec![];
    let mut plans = 0;
    for p in &list {
        match p {
            Put::Entry(body, uuid, group, title) => {
                let mut at = place(*uuid, group, title, now);
                match groups.iter().find(|(g, _)| g == group) {
                    Some((_, u)) => at.group_uuid = *u,
                    None => groups.push((group.clone(), at.group_uuid)),
                }
                items::put(c, &mut doc, &at, body).map_err(err)?;
            }
            Put::Plan(r) => {
                plans += 1;
                items::put_plan(&mut doc, plans, r).map_err(err)?;
            }
            Put::Signed(r) => items::put_signed(&mut doc, r).map_err(err)?,
        }
    }
    let xml = doc.write(kdbx::inner_key_of(&random));
    // The size follows the contents, as a seal's does: the smallest multiple
    // of a step that holds them.
    let size = kdbx::plain_needs(&kdbx::Binaries::all(&[]), xml.len())
        .div_ceil(file::STEP)
        .max(1)
        * file::STEP;
    let file = kdbx::seal(
        c,
        core::str::from_utf8(&kdbx::passphrase_bytes(passphrase).map_err(err)?).map_err(err)?,
        cost,
        &random,
        &Contents {
            document: &xml,
            binaries: faraday_vault::kdbx::Binaries::all(&[]),
            public_custom_data: None,
        },
        size,
    )
    .map_err(err)?;
    Ok(json!({"file": hex(&file)}))
}

/// Costs an Argon2id run at the cost asked and a document write for each item:
/// `file::create_holding`, the function an export is made by, over the entries
/// of the request, each put as `file::seal` puts one. Gives the file as hex.
fn export(c: &Compute, req: &Value) -> Answer {
    let id: [u8; 16] = bytes(req, "vault_id").try_into().expect("16 bytes");
    let now = now_of(req);
    let list: Vec<Put> = req["items"]
        .as_array()
        .map(|a| a.iter().enumerate().map(|(n, v)| item(n, v)).collect())
        .unwrap_or_default();
    let mut changes = vec![];
    for p in &list {
        match p {
            Put::Entry(body, u, g, t) => changes.push(Change::Put {
                place: place(*u, g, t, now),
                body,
            }),
            _ => return Err("an export holds entries".into()),
        }
    }
    let file = file::create_holding(
        c,
        id,
        s(req, "label"),
        cost(req)?,
        s(req, "passphrase"),
        &kdbx::Components::none(),
        &random_of(req),
        now,
        &changes,
    )
    .map_err(err)?;
    Ok(json!({"file": hex(&file)}))
}

/// Costs `file::open` and `file::seal` (an Argon2id run each, the seal at the 64 MiB
/// step): opens `file`, puts the request's items (and `label`) and seals it.
fn seal(c: &Compute, req: &Value) -> Answer {
    let file = bytes(req, "file");
    let opened = file::open(c, &file, s(req, "passphrase")).map_err(err)?;
    let now = now_of(req);
    let list: Vec<Put> = req["items"]
        .as_array()
        .map(|a| {
            a.iter()
                .enumerate()
                .map(|(n, v)| item(n + 100, v))
                .collect()
        })
        .unwrap_or_default();
    let places: Vec<Place<'_>> = list
        .iter()
        .map(|p| match p {
            Put::Entry(_, u, g, t) => place(*u, g, t, now),
            _ => place([0; 16], "", "", now),
        })
        .collect();
    let mut changes = vec![];
    let mut plans = items::next_plan(opened.document()).map_err(err)?;
    for (p, at) in list.iter().zip(&places) {
        match p {
            Put::Entry(body, ..) => changes.push(Change::Put {
                place: place(at.uuid, at.group, at.title, now),
                body,
            }),
            Put::Plan(r) => {
                changes.push(Change::Plan {
                    n: plans,
                    record: r,
                });
                plans += 1;
            }
            Put::Signed(r) => changes.push(Change::Signed(r)),
        }
    }
    if let Some(l) = opt(req, "label") {
        changes.push(Change::Label(l));
    }
    let out = file::seal(c, &opened, &changes, &random_of(req), now).map_err(err)?;
    Ok(json!({"file": hex(&out)}))
}

fn pairs<'a>(it: impl Iterator<Item = (&'a str, &'a str)>) -> Vec<Value> {
    it.map(|(k, v)| json!([k, v])).collect()
}

fn walk(g: &Group, path: &mut Vec<String>, out: &mut Vec<Value>) {
    for e in g.entries() {
        out.push(json!({
            "path": path,
            "uuid": hex(&e.uuid()),
            "fields": e.fields().map(|p| json!([p.key(), p.value(), p.protected()])).collect::<Vec<_>>(),
            "custom": pairs(e.custom().items()),
        }));
    }
    for sub in g.groups() {
        path.push(sub.name().to_string());
        walk(sub, path, out);
        path.pop();
    }
}

/// Costs `file::open`: what Faraday read of `file`: the label, the
/// identifier, the vault's own custom data and every entry with its
/// group path, fields (protected or not) and Faraday's custom data.
fn open(c: &Compute, req: &Value) -> Answer {
    let file = bytes(req, "file");
    let opened = file::open(c, &file, s(req, "passphrase")).map_err(err)?;
    let doc = opened.document();
    let mut entries = vec![];
    walk(doc.root(), &mut vec![], &mut entries);
    Ok(json!({
        "label": doc.label(),
        "id": hex(&doc.root().uuid()),
        "custom": pairs(doc.custom().items()),
        "entries": entries,
        "read_only": opened.read_only().is_some(),
        "next_seal": opened.read().next_seal(),
    }))
}

/// Handles the payload layouts of §7: `vault.words_payload`,
/// `seed_payload`, `sheet_payload` and `payload_of`.
fn layout(name: &str, req: &Value) -> Answer {
    match name {
        "words_payload" => {
            let lang = *Language::ALL
                .get(num(req, "language") as usize)
                .ok_or("no such language")?;
            let m = Mnemonic::from_entropy(lang, &bytes(req, "entropy")).map_err(err)?;
            Ok(json!({"payload": hex(&layouts::words_payload(&m))}))
        }
        "seed_payload" => {
            let p = layouts::seed_payload(&bytes(req, "seed")).ok_or("not 16 to 64 bytes")?;
            Ok(json!({"payload": hex(&p)}))
        }
        "sheet_payload" => {
            let p = layouts::sheet_payload(
                &bytes(req, "descriptor"),
                &bytes(req, "name"),
                &bytes(req, "note"),
            )
            .ok_or("too long")?;
            Ok(json!({"payload": hex(&p)}))
        }
        "payload_of" => {
            let kind = u8::try_from(num(req, "kind")).map_err(err)?;
            match layouts::payload_of(kind, &bytes(req, "payload")).ok_or("not that layout")? {
                Opened::Words(m) => {
                    let lang = Language::ALL
                        .iter()
                        .position(|l| *l == m.language())
                        .expect("a listed language");
                    Ok(json!({"words": {
                        "language": lang,
                        "entropy": hex(m.entropy().expose().as_bytes()),
                    }}))
                }
                Opened::Seed(s) => Ok(json!({"seed": hex(&s)})),
                Opened::Sheet(s) => Ok(json!({"sheet": [
                    hex(&s.descriptor), hex(&s.name), hex(&s.note)
                ]})),
            }
        }
        _ => Err(format!("unknown vault op {name}")),
    }
}
