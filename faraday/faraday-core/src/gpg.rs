//! GPG keys in a vault (`PLAN.md` §7): making one, and what it makes —
//! the public certificate, detached signatures over Inbox files, a
//! revocation certificate, a renewed expiry, and the paperkey text. The
//! packets are `faraday-pgp`'s; the key lives in an open vault as a type-7
//! record (`docs/VAULT.md` §7) and only its public results reach the
//! Outbox.

use faraday_pgp::{Armor, Hash, Key, Reason, armor};
use faraday_vault::Record;
use faraday_vault::records::kind;
use zeroize::Zeroize;

use crate::Faraday;

/// The fields of a type-7 record.
mod f {
    pub const PRIMARY: u8 = 1;
    pub const CREATED: u8 = 2;
    pub const SUBKEY: u8 = 3;
    pub const SUBKEY_CREATED: u8 = 4;
    pub const USER_ID: u8 = 5;
    pub const EXPIRY: u8 = 6;
    pub const CERTIFICATE: u8 = 7;
}

/// A Julian year in seconds: what "expires in n years" counts.
const YEAR: u32 = 31_557_600;

fn u32_of(r: &Record, n: u8) -> Option<u32> {
    Some(u32::from_le_bytes(r.field(n)?.try_into().ok()?))
}

/// The key a type-7 record holds.
pub fn key_of(r: &Record) -> Option<Key> {
    let mut primary = [0u8; 32];
    primary.copy_from_slice(r.field(f::PRIMARY)?);
    let mut subkey = [0u8; 32];
    subkey.copy_from_slice(r.field(f::SUBKEY)?);
    let created = u32_of(r, f::CREATED)?;
    Some(Key {
        primary_seed: primary,
        created,
        subkey_seed: subkey,
        subkey_created: u32_of(r, f::SUBKEY_CREATED).unwrap_or(created),
        user_ids: r
            .fields
            .iter()
            .filter(|x| x.number == f::USER_ID)
            .map(|x| String::from_utf8_lossy(&x.bytes).into_owned())
            .collect(),
        expiry: u32_of(r, f::EXPIRY).unwrap_or(0),
    })
}

/// A type-7 record for a key and the certificate last made from it.
fn record_of(key: &Key, certificate: &[u8]) -> Record {
    let mut r = Record::new(kind::GPG)
        .with(f::PRIMARY, &key.primary_seed)
        .with(f::CREATED, &key.created.to_le_bytes())
        .with(f::SUBKEY, &key.subkey_seed)
        .with(f::SUBKEY_CREATED, &key.subkey_created.to_le_bytes());
    for uid in &key.user_ids {
        r.push(f::USER_ID, uid.as_bytes());
    }
    r.push(f::EXPIRY, &key.expiry.to_le_bytes());
    r.push(f::CERTIFICATE, certificate);
    r
}

/// When the certificate in a record was made: its self-signatures' time,
/// read from the first one.
fn certified_at(r: &Record) -> Option<u32> {
    let cert = r.field(f::CERTIFICATE)?;
    // The first signature packet's first hashed subpacket is its creation
    // time, as `faraday-pgp` writes it.
    let mut at = 0;
    while at + 2 < cert.len() {
        let tag = cert[at] & 0x3f;
        let first = cert[at + 1] as usize;
        let (len, head) = if first < 192 {
            (first, 2)
        } else if first < 224 {
            (((first - 192) << 8) + cert[at + 2] as usize + 192, 3)
        } else {
            return None;
        };
        let body = cert.get(at + head..at + head + len)?;
        if tag == 2 {
            // version, type, algorithm, hash, hashed length (2), then a
            // subpacket: length 5, type 2, four bytes.
            return Some(u32::from_be_bytes(body.get(8..12)?.try_into().ok()?));
        }
        at += head + len;
    }
    None
}

/// A Unix time as a date, `YYYY-MM-DD`.
pub fn date(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    // Howard Hinnant's days-to-civil.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// A short name for files: the user ID's name part, lower-case, and the
/// fingerprint's last eight digits.
fn file_stem(key: &Key) -> String {
    let name = key
        .user_ids
        .first()
        .map(|u| u.split('<').next().unwrap_or(u).trim().to_string())
        .unwrap_or_default();
    let name: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let fp = key.fingerprint();
    let tail: String = fp[16..].iter().map(|b| format!("{b:02X}")).collect();
    if name.trim_matches('-').is_empty() {
        tail
    } else {
        format!("{}-{tail}", name.trim_matches('-'))
    }
}

impl Faraday {
    fn clock(&self) -> Result<u32, String> {
        self.vaults
            .unix_secs
            .map(|t| t as u32)
            .ok_or_else(|| "This computer's clock is not known".to_string())
    }

    /// The GPG key selected in the open vault, with its record's place.
    fn gpg_selected(&self) -> Option<(usize, Key)> {
        let at = self.vault_selected_index()?;
        let r = self
            .vaults
            .open
            .get(self.vaults.current)?
            .contents
            .records
            .get(at)?;
        (r.kind == kind::GPG).then(|| key_of(r).map(|k| (at, k)))?
    }

    /// Makes a GPG key in the open vault: an Ed25519 certification key and
    /// signing subkey from fresh entropy, one user ID, the chosen expiry.
    /// The public certificate and a revocation certificate go to the
    /// Outbox.
    pub(crate) fn gpg_make(&mut self, name: &str, email: &str) -> Result<(), String> {
        let now = self.clock()?;
        let uid = match (name.trim(), email.trim()) {
            ("", "") => return Err("A key needs a name or an email".to_string()),
            (n, "") => n.to_string(),
            ("", e) => format!("<{e}>"),
            (n, e) => format!("{n} <{e}>"),
        };
        let (Some(primary), Some(subkey)) = (self.fresh(b"gpg primary"), self.fresh(b"gpg subkey"))
        else {
            return Err("No randomness from the system yet. Try again".to_string());
        };
        let key = Key {
            primary_seed: primary,
            created: now,
            subkey_seed: subkey,
            subkey_created: now,
            user_ids: vec![uid],
            expiry: self.vaults.gpg_years * YEAR,
        };
        let cert = key.certificate(now);
        let stem = file_stem(&key);
        let revocation = key.revocation(now, Reason::Unspecified, "");
        self.vault_push(record_of(&key, &cert), "The key is in the vault");
        self.put_outbox(
            &format!("{stem}.asc"),
            armor(Armor::PublicKey, &cert).into_bytes(),
        );
        self.put_outbox(
            &format!("{stem}-revocation.asc"),
            armor(Armor::PublicKey, &revocation).into_bytes(),
        );
        Ok(())
    }

    /// The selected key's public certificate, armoured, with the key.
    fn gpg_certificate(&self) -> Option<(Key, String)> {
        let (at, key) = self.gpg_selected()?;
        let r = &self.vaults.open[self.vaults.current].contents.records[at];
        let cert = match r.field(f::CERTIFICATE) {
            Some(c) => c.to_vec(),
            None => key.certificate(certified_at(r).unwrap_or(key.created)),
        };
        Some((key, armor(Armor::PublicKey, &cert)))
    }

    /// The selected key's public certificate to the Outbox.
    pub(crate) fn gpg_export(&mut self) {
        let Some((key, text)) = self.gpg_certificate() else {
            return;
        };
        let name = format!("{}.asc", file_stem(&key));
        self.put_outbox(&name, text.into_bytes());
        self.toast(&format!("{name} is in the Outbox"));
    }

    /// The QR view of the selected key's certificate, a revocation of it
    /// or its signature over an Inbox file, with the name and label of
    /// its picture.
    pub(crate) fn gpg_code(&self, code: crate::Code) -> Result<crate::QrView, String> {
        use crate::Code;
        let (_, key) = self.gpg_selected().ok_or("No GPG key is selected")?;
        let stem = file_stem(&key);
        let fp = format!(
            "Fingerprint {}",
            faraday_pgp::fingerprint_text(&key.fingerprint())
        );
        let who = key.user_ids.first().cloned().unwrap_or_default();
        let (title, text, name, lines) = match code {
            Code::GpgKey => {
                let (_, text) = self.gpg_certificate().ok_or("No GPG key is selected")?;
                (
                    "GPG public key".to_string(),
                    text,
                    format!("{stem}.png"),
                    vec![
                        who,
                        fp,
                        "Public: checks signatures, signs nothing".to_string(),
                    ],
                )
            }
            Code::GpgRevocation => {
                let rev = key.revocation(self.clock()?, Reason::Unspecified, "");
                (
                    "GPG revocation certificate".to_string(),
                    armor(Armor::PublicKey, &rev),
                    format!("{stem}-revocation.png"),
                    vec![
                        format!("Revokes {who}"),
                        fp,
                        "Whoever has it can revoke the key".to_string(),
                    ],
                )
            }
            Code::GpgSignature(k) => {
                let item = self.inbox.get(k).ok_or("That file is no longer in Files")?;
                let sig = key.sign(&item.bytes, self.clock()?, Hash::Sha512);
                (
                    format!("GPG signature · {}", item.name),
                    armor(Armor::Signature, &sig),
                    format!("{}-signature.png", item.name),
                    vec![format!("Signs {}", item.name), format!("By {who}"), fp],
                )
            }
            _ => return Err("Not a GPG file".to_string()),
        };
        Ok(crate::QrView::text(&title, &text)?.public(&name, lines))
    }

    /// A revocation certificate for the selected key to the Outbox.
    pub(crate) fn gpg_revoke(&mut self) {
        let Some((_, key)) = self.gpg_selected() else {
            return;
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        let name = format!("{}-revocation.asc", file_stem(&key));
        let rev = key.revocation(now, Reason::Unspecified, "");
        self.put_outbox(&name, armor(Armor::PublicKey, &rev).into_bytes());
        self.toast(&format!("{name} is in the Outbox"));
    }

    /// New self-signatures with the chosen expiry counted from now; the
    /// updated certificate to the Outbox.
    pub(crate) fn gpg_renew(&mut self) {
        let Some((at, mut key)) = self.gpg_selected() else {
            return;
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        key.expiry = match self.vaults.gpg_years {
            0 => 0,
            y => now.saturating_sub(key.created) + y * YEAR,
        };
        let cert = key.certificate(now);
        let name = format!("{}.asc", file_stem(&key));
        if let Some(v) = self.vaults.open.get_mut(self.vaults.current) {
            v.contents.records[at] = record_of(&key, &cert);
            v.changes += 1;
        }
        self.put_outbox(&name, armor(Armor::PublicKey, &cert).into_bytes());
        self.toast(&format!("Renewed · {name} is in the Outbox"));
    }

    /// A detached signature over Inbox file `k` by the selected key, to
    /// the Outbox as `NAME.asc`.
    pub(crate) fn gpg_sign(&mut self, k: usize) {
        let Some((_, key)) = self.gpg_selected() else {
            return;
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let name = format!("{}.asc", item.name);
        let sig = key.sign(&item.bytes, now, Hash::Sha512);
        self.put_outbox(&name, armor(Armor::Signature, &sig).into_bytes());
        self.vaults.signing = false;
        self.toast(&format!("{name} is in the Outbox"));
    }
}

/// What the detail column shows of a GPG key: its fingerprints, user IDs,
/// dates and paperkey text.
pub struct Shown {
    /// The primary fingerprint, as GnuPG prints it.
    pub fingerprint: String,
    /// The signing subkey's.
    pub subkey: String,
    /// The user IDs.
    pub user_ids: Vec<String>,
    /// Created, as a date.
    pub created: String,
    /// Expires, as a date, or "Never".
    pub expires: String,
    /// The paperkey text: secret.
    pub paperkey: zeroize::Zeroizing<String>,
}

/// What a type-7 record shows.
pub fn shown(r: &Record) -> Option<Shown> {
    let key = key_of(r)?;
    let mut paperkey = key.paperkey();
    let out = Shown {
        fingerprint: faraday_pgp::fingerprint_text(&key.fingerprint()),
        subkey: faraday_pgp::fingerprint_text(&key.subkey_fingerprint()),
        user_ids: key.user_ids.clone(),
        created: date(u64::from(key.created)),
        expires: if key.expiry == 0 {
            "Never".to_string()
        } else {
            date(u64::from(key.created) + u64::from(key.expiry))
        },
        paperkey: zeroize::Zeroizing::new(paperkey.clone()),
    };
    paperkey.zeroize();
    Some(out)
}
