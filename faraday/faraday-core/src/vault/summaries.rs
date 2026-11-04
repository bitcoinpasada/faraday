//! The summaries as the kept state holds them (`docs/VAULT.md` §11):
//! written into `vault-summaries` at a lock and read back by the next
//! process.

use super::{SealSeen, VaultSummary};

/// One text field of the kept summaries: no tab or line break.
fn kept_field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// The summaries as the kept state holds them: a `vault` line per
/// vault, then its `copy`, `wallet`, `key` and `map` lines, fields split
/// by tabs.
pub(crate) fn summaries_encode(all: &[VaultSummary]) -> Vec<u8> {
    let mut out = String::new();
    for s in all {
        let id = String::from_utf8_lossy(&s.id);
        let seen = s.seen.map(|t| t.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "vault\t{id}\t{}\t{}\t{}\t{seen}\n",
            kept_field(&s.file),
            kept_field(&s.name),
            s.entries
        ));
        for d in &s.copies {
            out.push_str(&format!("copy\t{}\n", crate::backups::hex32(d)));
        }
        for (name, shape) in &s.wallets {
            out.push_str(&format!(
                "wallet\t{}\t{}\n",
                kept_field(name),
                kept_field(shape)
            ));
        }
        for (i, k) in s.keys.iter().enumerate() {
            match s.ids.get(i).copied().flatten() {
                Some(id) => out.push_str(&format!(
                    "key\t{}\t{}\n",
                    kept_field(k),
                    crate::backups::hex32(&id)
                )),
                None => out.push_str(&format!("key\t{}\n", kept_field(k))),
            }
        }
        for c in &s.sums {
            out.push_str(&format!("sum\t{}\n", kept_field(c)));
        }
        for c in &s.planned {
            out.push_str(&format!("planned\t{}\n", kept_field(c)));
        }
        for m in &s.map {
            out.push_str(&format!("map\t{}\n", kept_field(m)));
        }
        if let Some(e) = &s.seal {
            out.push_str(&format!(
                "seal\t{}\t{}\t{}\n",
                e.count,
                crate::backups::hex32(&e.digest),
                kept_field(&e.planned.join(","))
            ));
        }
    }
    out.into_bytes()
}

/// The summaries read back; a line that does not read is skipped.
pub(crate) fn summaries_decode(bytes: &[u8]) -> Vec<VaultSummary> {
    let text = String::from_utf8_lossy(bytes);
    let mut out: Vec<VaultSummary> = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[..] {
            ["vault", id, file, name, entries, seen] => {
                let mut s = VaultSummary::default();
                match <[u8; 32]>::try_from(id.as_bytes()) {
                    Ok(b) if b.iter().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) => s.id = b,
                    _ => continue,
                }
                s.file = file.to_string();
                s.name = name.to_string();
                s.entries = entries.parse().unwrap_or(0);
                s.seen = seen.parse().ok();
                out.push(s);
            }
            ["copy", digest] => {
                if let (Some(s), Some(d)) = (out.last_mut(), crate::backups::read_hex32(digest)) {
                    s.copies.push(d);
                }
            }
            ["wallet", name, shape] => {
                if let Some(s) = out.last_mut() {
                    s.wallets.push((name.to_string(), shape.to_string()));
                }
            }
            ["key", fp] => {
                if let Some(s) = out.last_mut() {
                    s.keys.push(fp.to_string());
                    s.ids.push(None);
                }
            }
            ["key", fp, id] => {
                if let Some(s) = out.last_mut() {
                    s.keys.push(fp.to_string());
                    s.ids.push(crate::backups::read_hex32(id));
                }
            }
            ["sum", c] => {
                if let Some(s) = out.last_mut() {
                    s.sums.push(c.to_string());
                }
            }
            ["planned", c] => {
                if let Some(s) = out.last_mut() {
                    s.planned.push(c.to_string());
                }
            }
            ["map", m] => {
                if let Some(s) = out.last_mut() {
                    s.map.push(m.to_string());
                }
            }
            ["seal", count, digest, planned] => {
                let seen = (|| {
                    Some(SealSeen {
                        count: count.parse().ok().filter(|c| *c >= 1)?,
                        digest: crate::backups::read_hex32(digest)?,
                        planned: planned
                            .split(',')
                            .filter(|c| !c.is_empty())
                            .map(str::to_string)
                            .collect(),
                    })
                })();
                if let (Some(s), Some(seen)) = (out.last_mut(), seen) {
                    s.seal = Some(seen);
                }
            }
            _ => {}
        }
    }
    out
}
