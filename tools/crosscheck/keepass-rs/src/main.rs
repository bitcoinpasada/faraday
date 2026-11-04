//! keepass-rs as an outside reader and writer of the vault's KDBX file, for
//! the cross-check's `vault` area (`tools/crosscheck/area_vault.py`; F193
//! point 1: it judges the file format only). Never part of an app.
//!
//!     keepass-rs read  FILE    the passphrase is the first line of standard input
//!     keepass-rs write FILE    the passphrase, a line feed, then the JSON description
//!
//! `read` prints the database as one JSON document: the shape the area
//! compares with pykeepass's view (`vault_files.py`), every list sorted
//! because keepass-rs keeps fields and custom data in hash maps. A file it
//! will not open is a message on standard error and exit status 3.
//! `write` makes a KDBX 4 file from the description (see `area_vault.py`'s
//! `rs_description`).

use std::collections::HashMap;
use std::io::{Read, Write};

use keepass::config::{CompressionConfig, InnerCipherConfig, KdfConfig, OuterCipherConfig};
use keepass::db::{
    CustomDataItem, CustomDataValue, Database, EntryRef, GroupId, GroupRef, Icon, Value,
};
use keepass::DatabaseKey;
use serde_json::{Value as Json, json};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Standard Base64 with padding. keepass-rs decodes a custom data item's value
/// as Base64 when it can, so a text such as `note` comes back as bytes; encoding
/// them again gives the text back when it was canonical Base64 (a known difference
/// of keepass-rs otherwise, listed in `area_vault.py`).
fn base64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn custom_list(items: &HashMap<String, CustomDataItem>) -> Json {
    let mut out: Vec<(String, String)> = items
        .iter()
        .map(|(k, item)| {
            let v = match &item.value {
                Some(CustomDataValue::String(s)) => s.clone(),
                Some(CustomDataValue::Binary(b)) => base64(b),
                None => String::new(),
            };
            (k.clone(), v)
        })
        .collect();
    out.sort();
    json!(out)
}

fn entry_json(e: &EntryRef, path: &[String]) -> Json {
    let mut fields: Vec<(String, String, bool)> = e
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), v.get().clone(), v.is_protected()))
        .collect();
    fields.sort();
    let mut tags = e.tags.clone();
    tags.sort();
    let mut attachments: Vec<(String, String)> = e
        .attachments_named()
        .map(|(name, a)| (name.to_string(), hex(a.data.get())))
        .collect();
    attachments.sort();
    let icon = match e.icon() {
        Some(Icon::BuiltIn(n)) => json!(n),
        _ => Json::Null,
    };
    json!({
        "uuid": e.id().uuid().simple().to_string(),
        "path": path,
        "fields": fields,
        "custom": custom_list(&e.custom_data),
        "tags": tags,
        "icon": icon,
        "attachments": attachments,
    })
}

fn walk(g: &GroupRef, path: &mut Vec<String>, entries: &mut Vec<Json>, groups: &mut Vec<Vec<String>>) {
    for e in g.entries() {
        entries.push(entry_json(&e, path));
    }
    for sub in g.groups() {
        path.push(sub.name.clone());
        groups.push(path.clone());
        walk(&sub, path, entries, groups);
        path.pop();
    }
}

fn read(path: &str, passphrase: &str) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let db = Database::parse(&data, DatabaseKey::new().with_password(passphrase))
        .map_err(|e| e.to_string())?;
    let (mut entries, mut groups) = (Vec::new(), Vec::new());
    walk(&db.root(), &mut Vec::new(), &mut entries, &mut groups);
    entries.sort_by_key(|e| e["uuid"].as_str().unwrap_or("").to_string());
    groups.sort();
    let out = json!({
        "label": db.meta.database_name.clone().unwrap_or_default(),
        "id": db.root().id().uuid().simple().to_string(),
        "custom": custom_list(&db.meta.custom_data),
        "groups": groups,
        "entries": entries,
    });
    println!("{out}");
    Ok(())
}

fn text(v: &Json, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_string()
}

fn string_item(v: &str) -> CustomDataItem {
    CustomDataItem {
        value: Some(CustomDataValue::String(v.to_string())),
        last_modification_time: None,
    }
}

fn config(db: &mut Database, d: &Json) -> Result<(), String> {
    let c = &mut db.config;
    c.outer_cipher_config = match d["cipher"].as_str().unwrap_or("aes256") {
        "aes256" => OuterCipherConfig::AES256,
        "chacha20" => OuterCipherConfig::ChaCha20,
        "twofish" => OuterCipherConfig::Twofish,
        other => return Err(format!("cipher {other}")),
    };
    c.inner_cipher_config = match d["inner"].as_str().unwrap_or("chacha20") {
        "chacha20" => InnerCipherConfig::ChaCha20,
        "salsa20" => InnerCipherConfig::Salsa20,
        other => return Err(format!("inner cipher {other}")),
    };
    c.compression_config = if d["compression"].as_bool().unwrap_or(true) {
        CompressionConfig::GZip
    } else {
        CompressionConfig::None
    };
    let kib = d["memory_kib"].as_u64().unwrap_or(8192);
    let iterations = d["iterations"].as_u64().unwrap_or(1);
    let parallelism = d["parallelism"].as_u64().unwrap_or(1) as u32;
    let KdfConfig::Argon2 { version, .. } = c.kdf_config.clone() else {
        return Err("the default KDF is not Argon2".into());
    };
    c.kdf_config = match d["kdf"].as_str().unwrap_or("argon2d") {
        "argon2d" => KdfConfig::Argon2 { iterations, memory: kib * 1024, parallelism, version },
        "argon2id" => KdfConfig::Argon2id { iterations, memory: kib * 1024, parallelism, version },
        "aes" => KdfConfig::Aes { rounds: d["rounds"].as_u64().unwrap_or(1000) },
        other => return Err(format!("kdf {other}")),
    };
    Ok(())
}

fn write(path: &str, passphrase: &str, d: &Json) -> Result<(), String> {
    let mut db = Database::new();
    config(&mut db, d)?;
    db.meta.database_name = Some(text(d, "label"));
    for pair in d["custom"].as_array().into_iter().flatten() {
        let (k, v) = (pair[0].as_str().unwrap_or(""), pair[1].as_str().unwrap_or(""));
        db.meta.custom_data.insert(k.to_string(), string_item(v));
    }
    // Groups by path, parents before children.
    let mut ids: HashMap<Vec<String>, GroupId> = HashMap::new();
    ids.insert(Vec::new(), db.root().id());
    for g in d["groups"].as_array().into_iter().flatten() {
        let path: Vec<String> = g
            .as_array()
            .ok_or("a group path")?
            .iter()
            .map(|n| n.as_str().unwrap_or("").to_string())
            .collect();
        let parent = *ids.get(&path[..path.len() - 1]).ok_or("a group before its parent")?;
        let mut pg = db.group_mut(parent).ok_or("no parent group")?;
        let mut sub = pg.add_group();
        sub.name = path[path.len() - 1].clone();
        ids.insert(path, sub.id());
    }
    for e in d["entries"].as_array().into_iter().flatten() {
        let path: Vec<String> = e["path"]
            .as_array()
            .ok_or("an entry path")?
            .iter()
            .map(|n| n.as_str().unwrap_or("").to_string())
            .collect();
        let parent = *ids.get(&path).ok_or("an entry in no group")?;
        let mut group = db.group_mut(parent).ok_or("no group")?;
        let mut entry = group.add_entry();
        entry.set_unprotected("Title", text(e, "title"));
        entry.set_unprotected("UserName", text(e, "user"));
        entry.set_protected("Password", text(e, "password"));
        entry.set_unprotected("URL", text(e, "url"));
        entry.set_unprotected("Notes", text(e, "notes"));
        for p in e["props"].as_array().into_iter().flatten() {
            let (k, v) = (p[0].as_str().unwrap_or(""), p[1].as_str().unwrap_or(""));
            if p[2].as_bool().unwrap_or(false) {
                entry.set_protected(k, v);
            } else {
                entry.set_unprotected(k, v);
            }
        }
        for t in e["tags"].as_array().into_iter().flatten() {
            entry.tags.push(t.as_str().unwrap_or("").to_string());
        }
        if let Some(n) = e["icon"].as_u64() {
            entry.set_icon_builtin(n as usize);
        }
        for a in e["attachments"].as_array().into_iter().flatten() {
            let name = a[0].as_str().unwrap_or("").to_string();
            entry.add_attachment(name, Value::unprotected(unhex(a[1].as_str().unwrap_or(""))));
        }
        for c in e["custom"].as_array().into_iter().flatten() {
            let (k, v) = (c[0].as_str().unwrap_or(""), c[1].as_str().unwrap_or(""));
            entry.custom_data.insert(k.to_string(), string_item(v));
        }
    }
    let mut file = Vec::new();
    db.save(&mut file, DatabaseKey::new().with_password(passphrase))
        .map_err(|e| e.to_string())?;
    std::fs::write(path, file).map_err(|e| e.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("standard input");
    let (passphrase, rest) = input.split_once('\n').unwrap_or((&input, ""));
    let result = match (args.get(1).map(String::as_str), args.get(2)) {
        (Some("read"), Some(path)) => read(path, passphrase),
        (Some("write"), Some(path)) => serde_json::from_str::<Json>(rest)
            .map_err(|e| e.to_string())
            .and_then(|d| write(path, passphrase, &d)),
        _ => Err("usage: keepass-rs read|write FILE".into()),
    };
    if let Err(message) = result {
        let _ = writeln!(std::io::stderr(), "keepass-rs: {message}");
        std::process::exit(3);
    }
}
