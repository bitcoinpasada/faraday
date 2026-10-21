//! GPG keys in a vault (`PLAN.md` §7, F32): making one, and what it makes —
//! the public certificate, detached signatures over Inbox files, a
//! revocation certificate kept with the key, a renewed expiry, and the
//! paperkey text. The packets are `faraday-pgp`'s; the key lives in an open
//! vault as a type-7 record (`docs/VAULT.md` §7) and only its public
//! results reach the Outbox. What GnuPG does, this does: F32 lists each
//! behaviour copied and how it was checked against GnuPG 2.4.

use crate::strings::EN;
use faraday_pgp::{Armor, Hash, Key, Reason, armor};
use faraday_vault::Record;
use faraday_vault::records::kind;
use opensigner_core::strings::fill;
use zeroize::{Zeroize, Zeroizing};

use crate::backup_flow_steps::Built;
use crate::{Faraday, Item};

/// The fields of a type-7 record.
mod f {
    pub const PRIMARY: u8 = 1;
    pub const CREATED: u8 = 2;
    pub const SUBKEY: u8 = 3;
    pub const SUBKEY_CREATED: u8 = 4;
    pub const USER_ID: u8 = 5;
    pub const EXPIRY: u8 = 6;
    pub const CERTIFICATE: u8 = 7;
    /// The revocation certificate made with the key, as packets.
    pub const REVOCATION: u8 = 8;
    /// When the key was marked revoked (`u32`).
    pub const REVOKED: u8 = 9;
}

/// GnuPG's year: 365 days, as `--quick-set-expire 2y` counts it.
pub const YEAR: u32 = 31_536_000;

/// A new key's term in years, as `gpg --quick-generate-key` makes it with
/// no expiry given (GnuPG 2.4: `3y`).
pub const DEFAULT_YEARS: u32 = 3;

/// How far the clock may be before the key's last self-signature for a
/// renewal to go on, in seconds: GnuPG makes the new signature one second
/// after the last when the clock is within 4 seconds of it, and refuses
/// ("Time conflict") from 5 on.
const SKEW: u32 = 5;

/// The longest text a signing page shows of a file.
const TEXT_MAX: usize = 2048;

fn u32_of(r: &Record, n: u8) -> Option<u32> {
    Some(u32::from_le_bytes(r.field(n)?.try_into().ok()?))
}

/// The key a type-7 record holds. `None` for a record without a signing
/// subkey (field 3 is optional in the format, `docs/VAULT.md` §7): it is
/// kept and shown by its user ID alone, and signs nothing.
pub fn key_of(r: &Record) -> Option<Key> {
    let primary = r.field(f::PRIMARY)?;
    let subkey = r.field(f::SUBKEY)?;
    if primary.len() != 32 || subkey.len() != 32 {
        return None;
    }
    let created = u32_of(r, f::CREATED)?;
    let mut key = Key {
        primary_seed: [0; 32],
        created,
        subkey_seed: [0; 32],
        subkey_created: u32_of(r, f::SUBKEY_CREATED).unwrap_or(created),
        user_ids: r
            .fields
            .iter()
            .filter(|x| x.number == f::USER_ID)
            .map(|x| String::from_utf8_lossy(&x.bytes).into_owned())
            .collect(),
        expiry: u32_of(r, f::EXPIRY).unwrap_or(0),
    };
    key.primary_seed.copy_from_slice(primary);
    key.subkey_seed.copy_from_slice(subkey);
    Some(key)
}

/// A type-7 record for a key, the certificate last made from it and the
/// revocation certificate made with it.
fn record_of(key: &Key, certificate: &[u8], revocation: &[u8]) -> Record {
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
    r.push(f::REVOCATION, revocation);
    r
}

/// The time of the first signature packet in `packets`: the self-signatures
/// of a certificate are all made at one time, and a revocation is one
/// signature.
fn signature_time(packets: &[u8]) -> Option<u32> {
    let mut at = 0;
    while at + 2 < packets.len() {
        let tag = packets[at] & 0x3f;
        let first = packets[at + 1] as usize;
        let (len, head) = if first < 192 {
            (first, 2)
        } else if first < 224 {
            (((first - 192) << 8) + packets[at + 2] as usize + 192, 3)
        } else {
            return None;
        };
        let body = packets.get(at + head..at + head + len)?;
        if tag == 2 {
            // version, type, algorithm, hash, hashed length (2), then a
            // subpacket: length 5, type 2, four bytes.
            return Some(u32::from_be_bytes(body.get(8..12)?.try_into().ok()?));
        }
        at += head + len;
    }
    None
}

/// When the certificate in a record was last signed: the time of its
/// self-signatures.
pub fn certified_at(r: &Record) -> Option<u32> {
    signature_time(r.field(f::CERTIFICATE)?)
}

/// When a record's key was marked revoked.
fn revoked_at(r: &Record) -> Option<u32> {
    u32_of(r, f::REVOKED)
}

/// SHA-256 of a file, as the signing page shows and ticks it.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    crate::sha256_of(bytes)
}

/// [`sha256`] in lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    sha256(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// A Unix time as a date, `YYYY-MM-DD`.
pub fn date(unix: u64) -> String {
    faraday_pgp::utc_date(unix)
}

/// A key's ID as GnuPG prints it: the last 16 hex digits of its
/// fingerprint.
pub fn key_id(fp: &[u8; 20]) -> String {
    fp[12..].iter().map(|b| format!("{b:02X}")).collect()
}

/// A short name for files: the user ID's name part, lower-case, and the
/// fingerprint's last eight digits.
fn file_stem(key: &Key) -> String {
    let name = name_part(key);
    let fp = key.fingerprint();
    let tail: String = fp[16..].iter().map(|b| format!("{b:02X}")).collect();
    if name.is_empty() {
        tail
    } else {
        format!("{name}-{tail}")
    }
}

/// The user ID's name, as a file name part: lower-case letters and digits
/// with dashes between.
fn name_part(key: &Key) -> String {
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
    name.trim_matches('-').to_string()
}

/// A revocation file's name, `{name}-{FPR}.rev`: the name GnuPG gives
/// `openpgp-revocs.d/<FPR>.rev`, with the name before it.
fn rev_name(key: &Key) -> String {
    let fpr: String = key
        .fingerprint()
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();
    match name_part(key) {
        n if n.is_empty() => format!("{fpr}.rev"),
        n => format!("{n}-{fpr}.rev"),
    }
}

/// How long ago, as GnuPG says it: seconds below a day, then days.
fn span(secs: u32) -> String {
    match secs {
        1 => fill(EN.gpg_second, &["1"]),
        s if s < 86_400 => fill(EN.gpg_seconds, &[&s.to_string()]),
        s if s / 86_400 == 1 => fill(EN.gpg_day, &["1"]),
        s => fill(EN.gpg_days, &[&(s / 86_400).to_string()]),
    }
}

/// Whether a key can sign at `now`: not revoked, not expired, and made
/// before now. GnuPG's refusals: a key it cannot use is "Unusable secret
/// key"; one made in the future says so, with its ID.
fn usable(key: &Key, revoked: Option<u32>, now: u32) -> Result<(), String> {
    if let Some(t) = revoked {
        return Err(fill(EN.gpg_revoked, &[&date(u64::from(t))]));
    }
    if key.expiry > 0 {
        let end = key.created.saturating_add(key.expiry);
        if now >= end {
            return Err(fill(EN.gpg_expired, &[&date(u64::from(end))]));
        }
    }
    for (created, fp) in [
        (key.created, key.fingerprint()),
        (key.subkey_created, key.subkey_fingerprint()),
    ] {
        if created > now {
            return Err(fill(EN.gpg_future, &[&key_id(&fp), &span(created - now)]));
        }
    }
    Ok(())
}

/// An Inbox file the signing page lists: what is signed is what is shown.
#[derive(Debug, Clone)]
pub struct GpgFile {
    /// Its place in the Inbox.
    pub index: usize,
    /// Its name.
    pub name: String,
    /// Its size.
    pub size: usize,
    /// SHA-256 of its bytes.
    pub hash: [u8; 32],
    /// Its text, when it is a small text file.
    pub text: Option<String>,
}

/// What the signing page remembers of a file so that it is hashed once: the
/// file's name, size, place in memory and both ends. A file that a stick
/// visit replaced differs in one of them; the hash is taken again at
/// signing in any case.
#[derive(Debug, Clone)]
pub struct Seen {
    name: String,
    probe: (usize, usize, Vec<u8>),
    hash: [u8; 32],
    text: Option<String>,
}

fn probe_of(bytes: &[u8]) -> (usize, usize, Vec<u8>) {
    let mut ends = bytes[..bytes.len().min(32)].to_vec();
    ends.extend_from_slice(&bytes[bytes.len().saturating_sub(32)..]);
    (bytes.len(), bytes.as_ptr() as usize, ends)
}

/// A small text file's text: UTF-8, printable, a few lines.
fn small_text(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() || bytes.len() > TEXT_MAX {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    text.chars()
        .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .then(|| text.to_string())
}

impl Faraday {
    fn clock(&self) -> Result<u32, String> {
        self.clock_now()
            .map(|t| t as u32)
            .ok_or_else(|| EN.gpg_this_computer_s.to_string())
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

    /// The selected key's record.
    fn gpg_record(&self, at: usize) -> Option<&Record> {
        self.vaults
            .open
            .get(self.vaults.current)?
            .contents
            .records
            .get(at)
    }

    /// Puts a file in the Outbox unless a file of that name is there that
    /// `replaces` does not allow over: GnuPG's batch mode does not
    /// overwrite ("File exists") either.
    fn put_unless_taken(
        &mut self,
        name: &str,
        bytes: Vec<u8>,
        replaces: impl Fn(&Item) -> bool,
    ) -> bool {
        if self.outbox.iter().any(|i| i.name == name && !replaces(i)) {
            return false;
        }
        self.put_outbox(name, bytes);
        true
    }

    /// Makes a GPG key for the backup flow: an Ed25519 certification key
    /// and signing subkey from fresh entropy, one user ID, the chosen
    /// expiry. The record every vault of the plan takes and the public
    /// certificate For the stick; the revocation certificate, as GnuPG 2.1
    /// and later do, is kept with the key.
    pub(crate) fn gpg_build(&mut self, name: &str, email: &str) -> Result<Built, String> {
        let now = self.clock()?;
        // `gpg --quick-generate-key` takes one string, trims it, and
        // refuses only an empty one ("Invalid user ID"); it checks no
        // address and no length.
        let uid = match (name.trim(), email.trim()) {
            ("", "") => return Err(EN.gpg_a_key_needs_a_name.to_string()),
            (n, "") => n.to_string(),
            ("", e) => format!("<{e}>"),
            (n, e) => format!("{n} <{e}>"),
        };
        let (Some(primary), Some(subkey)) = (self.fresh(b"gpg primary"), self.fresh(b"gpg subkey"))
        else {
            return Err(EN.gpg_no_randomness_from.to_string());
        };
        let primary = Zeroizing::new(primary);
        let subkey = Zeroizing::new(subkey);
        let key = Key {
            primary_seed: *primary,
            created: now,
            subkey_seed: *subkey,
            subkey_created: now,
            user_ids: vec![uid.clone()],
            expiry: self.vaults.gpg_years * YEAR,
        };
        let cert = key.certificate(now);
        let stem = file_stem(&key);
        let revocation = key.revocation(now, Reason::Unspecified, "");
        let record = record_of(&key, &cert, &revocation);
        Ok(Built {
            record,
            subject: key.fingerprint().to_vec(),
            name: if name.trim().is_empty() {
                uid
            } else {
                name.trim().to_string()
            },
            outbox: vec![(
                format!("{stem}.asc"),
                armor(Armor::PublicKey, &cert).into_bytes(),
            )],
        })
    }

    /// The public key to the Outbox, unless a file of that name is there
    /// that is not a public key.
    pub(crate) fn gpg_put_public(&mut self, name: &str, bytes: Vec<u8>) {
        self.put_unless_taken(name, bytes, |i| {
            faraday_pgp::read_public_armor(&i.bytes) == Some(Armor::PublicKey)
        });
    }

    /// Makes the revocation certificate a record lacks, with the clock as
    /// it is: a key made before revocations were kept gets its one when
    /// its page is first used. Nothing is made while the clock is not
    /// known or not checked.
    pub(crate) fn gpg_keep_revocation(&mut self) {
        let Some((at, key)) = self.gpg_selected() else {
            return;
        };
        if self
            .gpg_record(at)
            .is_none_or(|r| r.field(f::REVOCATION).is_some())
            || !self.clock_checked()
        {
            return;
        }
        let Ok(now) = self.clock() else {
            return;
        };
        let rev = key.revocation(now, Reason::Unspecified, "");
        if let Some(v) = self.vaults.open.get_mut(self.vaults.current) {
            v.contents.records[at].set(f::REVOCATION, &rev);
            v.changes += 1;
        }
    }

    /// The selected key's public certificate, armoured, with the key.
    fn gpg_certificate(&self) -> Option<(Key, String)> {
        let (at, key) = self.gpg_selected()?;
        let r = self.gpg_record(at)?;
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
        if self.put_unless_taken(&name, text.into_bytes(), |i| {
            faraday_pgp::read_public_armor(&i.bytes) == Some(Armor::PublicKey)
        }) {
            self.toast_out(&name);
        } else {
            self.toast(&fill(EN.gpg_exists, &[&name.to_string()]));
        }
    }

    /// The QR view of the selected key's certificate or its signature
    /// over an Inbox file, with the name and label of its picture.
    pub(crate) fn gpg_code(&self, code: crate::Code) -> Result<crate::QrView, String> {
        use crate::Code;
        let (at, key) = self.gpg_selected().ok_or(EN.gpg_no_gpg_key_is_selected)?;
        let stem = file_stem(&key);
        let fp = fill(
            EN.gpg_fingerprint,
            &[&faraday_pgp::fingerprint_text(&key.fingerprint()).to_string()],
        );
        let who = key.user_ids.first().cloned().unwrap_or_default();
        let (title, text, name, lines) = match code {
            Code::GpgKey => {
                let (_, text) = self
                    .gpg_certificate()
                    .ok_or(EN.gpg_no_gpg_key_is_selected)?;
                (
                    EN.gpg_gpg_public_key.to_string(),
                    text,
                    format!("{stem}.png"),
                    vec![who, fp, EN.gpg_public_checks_signatures.to_string()],
                )
            }
            Code::GpgSignature(prefix) => {
                // A key that waits for its backup signs nothing, a code of a
                // signature included (`docs/BACKUP-FLOW.md` §8).
                if self.selected_waits().is_some() {
                    return Err(EN.backup_flow_first.to_string());
                }
                let now = self.clock()?;
                usable(&key, self.gpg_record(at).and_then(revoked_at), now)?;
                let item = self
                    .gpg_files()
                    .into_iter()
                    .find(|f| f.hash[..8] == prefix)
                    .and_then(|f| self.inbox.get(f.index))
                    .ok_or(EN.gpg_that_file_is_no)?;
                let sig = key.sign(&item.bytes, now, Hash::Sha512);
                (
                    fill(EN.gpg_gpg_signature, &[&item.name.to_string()]),
                    armor(Armor::Signature, &sig),
                    format!("{}.asc.png", item.name),
                    vec![
                        fill(EN.gpg_signs, &[&item.name.to_string()]),
                        fill(EN.gpg_by, &[&who.to_string()]),
                        fp,
                    ],
                )
            }
            _ => return Err(EN.gpg_not_a_gpg_file.to_string()),
        };
        Ok(crate::QrView::text(&title, &text)?.public(&name, lines))
    }

    /// Offers a revocation file through the secret sheet: whoever has it
    /// can revoke the key, so it is never a public file, a code or a
    /// picture.
    fn gpg_offer_revocation(&mut self, key: &Key, revocation: &[u8]) {
        let mut out = crate::secrets::SecretOut::new(
            rev_name(key),
            Zeroizing::new(key.revocation_file(revocation).into_bytes()),
            EN.gpg_gpg_revocation,
            EN.gpg_whoever_has_it,
        );
        // It is in the vault already.
        out.keep = crate::secrets::Keep::None;
        self.offer_secret(out);
    }

    /// The revocation certificate kept with the selected key, through the
    /// secret sheet as `{name}-{FPR}.rev`.
    pub(crate) fn gpg_export_revocation(&mut self) {
        self.gpg_keep_revocation();
        let Some((at, key)) = self.gpg_selected() else {
            return;
        };
        let Some(rev) = self
            .gpg_record(at)
            .and_then(|r| r.field(f::REVOCATION))
            .map(<[u8]>::to_vec)
        else {
            return self.toast(EN.gpg_no_revocation_kept);
        };
        self.gpg_offer_revocation(&key, &rev);
    }

    /// A new revocation certificate for the selected key, with the reason
    /// chosen (`gpg --generate-revocation`), through the secret sheet.
    /// The kept one is not replaced.
    pub(crate) fn gpg_make_revocation(&mut self, reason: Reason, text: &str) {
        let Some((_, key)) = self.gpg_selected() else {
            return;
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        let rev = key.revocation(now, reason, text.trim());
        self.gpg_offer_revocation(&key, &rev);
    }

    /// The key is marked revoked, as importing its own revocation
    /// certificate marks it in GnuPG: it signs nothing and renews
    /// nothing after.
    pub(crate) fn gpg_mark_revoked(&mut self) {
        let Some((at, _)) = self.gpg_selected() else {
            return;
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        if let Some(v) = self.vaults.open.get_mut(self.vaults.current)
            && v.contents.records[at].field(f::REVOKED).is_none()
        {
            v.contents.records[at].set(f::REVOKED, &now.to_le_bytes());
            v.changes += 1;
        }
        self.toast(EN.gpg_marked_revoked);
    }

    /// New self-signatures with the term chosen on the key's page,
    /// counted from now as `gpg --quick-set-expire` counts it; the
    /// record's certificate and the Outbox's change together.
    pub(crate) fn gpg_renew(&mut self) {
        if self.selected_waits().is_some() {
            return;
        }
        let Some((at, mut key)) = self.gpg_selected() else {
            return;
        };
        let Some(years) = self.vaults.renew_years else {
            return self.toast(EN.gpg_choose_the_term);
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        let Some(r) = self.gpg_record(at) else {
            return;
        };
        if revoked_at(r).is_some() {
            return self.toast(&fill(
                EN.gpg_revoked,
                &[&date(u64::from(revoked_at(r).unwrap_or(0)))],
            ));
        }
        // GnuPG signs one second after the last signature when the clock
        // is within 4 seconds of it, and refuses a clock earlier than
        // that ("Time conflict"): an older signature would be ignored.
        let last = certified_at(r).unwrap_or(key.created);
        if last > now && last - now >= SKEW {
            return self.toast(EN.gpg_time_conflict);
        }
        let at_time = now.max(last.saturating_add(1));
        key.expiry = match years {
            0 => 0,
            y => now.saturating_sub(key.created) + y * YEAR,
        };
        let cert = key.certificate(at_time);
        let name = format!("{}.asc", file_stem(&key));
        if let Some(v) = self.vaults.open.get_mut(self.vaults.current) {
            let r = &mut v.contents.records[at];
            r.set(f::EXPIRY, &key.expiry.to_le_bytes());
            r.set(f::CERTIFICATE, &cert);
            v.changes += 1;
        }
        self.vaults.renew_years = None;
        if self.put_unless_taken(&name, armor(Armor::PublicKey, &cert).into_bytes(), |i| {
            faraday_pgp::read_public_armor(&i.bytes) == Some(Armor::PublicKey)
        }) {
            self.toast(&fill(
                EN.gpg_renewed_waits,
                &[&name.to_string(), self.medium.for_the()],
            ));
        } else {
            self.toast(&fill(EN.gpg_exists, &[&name.to_string()]));
        }
    }

    /// The Inbox files a GPG key can sign, each with its hash and, when
    /// small, its text: a `.asc` file is a signature already, not
    /// something to sign again.
    pub(crate) fn gpg_files(&self) -> Vec<GpgFile> {
        let mut seen = self.vaults.sign_seen.borrow_mut();
        let mut out = Vec::new();
        let mut keep = Vec::new();
        for (index, it) in self.inbox.iter().enumerate() {
            if it.name.ends_with(".asc") {
                continue;
            }
            let probe = probe_of(&it.bytes);
            let (hash, text) = match seen.iter().find(|s| s.name == it.name && s.probe == probe) {
                Some(s) => (s.hash, s.text.clone()),
                // Kept on the state (`sign_seen`) once worked out.
                None => crate::work::keeping(|| (it.hash(), small_text(&it.bytes))),
            };
            keep.push(Seen {
                name: it.name.clone(),
                probe,
                hash,
                text: text.clone(),
            });
            out.push(GpgFile {
                index,
                name: it.name.clone(),
                size: it.bytes.len(),
                hash,
                text,
            });
        }
        *seen = keep;
        out
    }

    /// The code of the selected key's signature over the Inbox file
    /// called `name`: it names the file by its hash, as the signing
    /// page's buttons do.
    pub fn signature_code(&self, name: &str) -> Option<crate::Code> {
        let f = self.gpg_files().into_iter().find(|f| f.name == name)?;
        Some(crate::Code::GpgSignature(f.hash[..8].try_into().ok()?))
    }

    /// Whether a listed file is ticked: by its name and hash, so a file a
    /// stick visit replaced is not.
    pub(crate) fn gpg_ticked(&self, f: &GpgFile) -> bool {
        self.vaults
            .sign_ticked
            .iter()
            .any(|(n, h)| *n == f.name && *h == f.hash)
    }

    /// The signing page's ticks at their default: the only signable file
    /// ticked, or none of several.
    pub(crate) fn gpg_default_ticks(&mut self) {
        let files = self.gpg_files();
        self.vaults.sign_ticked = match files.as_slice() {
            [one] => vec![(one.name.clone(), one.hash)],
            _ => Vec::new(),
        };
    }

    /// Ticks or unticks the listed file at `k`.
    pub(crate) fn gpg_tick(&mut self, k: usize) {
        let Some(f) = self.gpg_files().into_iter().nth(k) else {
            return;
        };
        let tick = (f.name, f.hash);
        match self.vaults.sign_ticked.iter().position(|t| *t == tick) {
            Some(at) => {
                self.vaults.sign_ticked.remove(at);
            }
            None => self.vaults.sign_ticked.push(tick),
        }
    }

    /// Ticks every listed file, or none when every one is ticked.
    pub(crate) fn gpg_tick_all(&mut self) {
        let files = self.gpg_files();
        if files.iter().all(|f| self.gpg_ticked(f)) {
            self.vaults.sign_ticked.clear();
        } else {
            self.vaults.sign_ticked = files.into_iter().map(|f| (f.name, f.hash)).collect();
        }
    }

    /// Clears the signing page, its ticks and the other GPG pages when the
    /// vault, the category or the key selected changes.
    pub(crate) fn gpg_page_reset(&mut self) {
        if self.sheet == Some(crate::Sheet::GpgSign) {
            self.sheet = None;
        }
        self.vaults.signing = false;
        self.vaults.sign_ticked.clear();
        self.vaults.sign_key = None;
        self.vaults.renew_years = None;
        self.vaults.revoke_reason = 0;
    }

    /// The selected key's fingerprint.
    pub(crate) fn gpg_fingerprint(&self) -> Option<[u8; 20]> {
        self.gpg_selected().map(|(_, k)| k.fingerprint())
    }

    /// The selected key's first user ID, which Sign with {key} names it
    /// by.
    pub(crate) fn gpg_user_id(&self) -> Option<String> {
        let at = self.vault_selected_index()?;
        let r = self
            .vaults
            .open
            .get(self.vaults.current)?
            .contents
            .records
            .get(at)?;
        let id = r.fields.iter().find(|x| x.number == f::USER_ID)?;
        Some(String::from_utf8_lossy(&id.bytes).into_owned())
    }

    /// The listed files ticked to sign, as Sign's sheet lists them.
    /// Hashes a file the page has not hashed yet.
    pub(crate) fn gpg_chosen(&self) -> Vec<GpgFile> {
        self.gpg_files()
            .into_iter()
            .filter(|f| self.gpg_ticked(f))
            .collect()
    }

    /// Sign a file's **Sign**: opens **Sign with {key}** over the page
    /// when a listed file is ticked. Signs nothing and asks nothing: the
    /// sheet shows the time the signatures will carry (F63).
    pub(crate) fn gpg_sign_sheet_open(&mut self) {
        if !self.vaults.signing || self.vaults.sign_key != self.gpg_fingerprint() {
            return;
        }
        if self.gpg_chosen().is_empty() {
            return;
        }
        self.sheet = Some(crate::Sheet::GpgSign);
    }

    /// The hold on **Sign with {key}**: signs the ticked files at the
    /// time the sheet shows, then closes it. With the clock not known
    /// the hold is disabled and signs nothing (F63). Nothing else signs:
    /// pressed with no sheet open, it does nothing.
    pub(crate) fn gpg_sign_sheet_hold(&mut self) {
        if self.sheet != Some(crate::Sheet::GpgSign) || self.clock_now().is_none() {
            return;
        }
        self.sheet = None;
        self.gpg_sign_ticked();
    }

    /// Signs every ticked file in one go, after the hold: a detached
    /// signature each, armoured, `{file}.asc` in the Outbox as `gpg
    /// --armor --detach-sign` names it. A file whose bytes are no longer
    /// those ticked is not signed. A signature is not written over a file
    /// of that name. One toast says how many were written.
    pub(crate) fn gpg_sign_ticked(&mut self) {
        if self.selected_waits().is_some() {
            return;
        }
        let Some((at, key)) = self.gpg_selected() else {
            return;
        };
        // The page was opened for another key: nothing is signed.
        if self.vaults.sign_key != Some(key.fingerprint()) {
            self.gpg_page_reset();
            return;
        }
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        if let Err(e) = usable(&key, self.gpg_record(at).and_then(revoked_at), now) {
            return self.toast(&e);
        }
        let ticked = std::mem::take(&mut self.vaults.sign_ticked);
        let (mut written, mut changed) = (0usize, 0usize);
        let mut failure: Option<String> = None;
        for (name, hash) in ticked {
            let found = self
                .inbox
                .iter()
                .position(|i| i.name == name && crate::sha256_of(&i.bytes) == hash);
            let Some(k) = found else {
                changed += 1;
                failure.get_or_insert_with(|| fill(EN.gpg_changed, &[&name.to_string()]));
                continue;
            };
            let out = format!("{name}.asc");
            if self.outbox.iter().any(|i| i.name == out) {
                failure.get_or_insert_with(|| fill(EN.gpg_exists, &[&out.to_string()]));
                continue;
            }
            let sig = key.sign(&self.inbox[k].bytes, now, Hash::Sha512);
            self.put_outbox(&out, armor(Armor::Signature, &sig).into_bytes());
            written += 1;
        }
        let _ = changed;
        let text = match (written, failure) {
            (0, Some(e)) => e,
            (0, None) => return,
            (1, _) => fill(EN.gpg_1_signature_waits, &[self.medium.for_the()]),
            (n, _) => fill(
                EN.gpg_signatures_wait,
                &[&n.to_string(), self.medium.for_the()],
            ),
        };
        self.toast(&text);
    }
}

/// What the detail column shows of a GPG key: its fingerprints, user IDs,
/// dates and revocation. The paperkey text is not here: it is made when
/// it is shown ([`paperkey`]).
pub struct Shown {
    /// The primary fingerprint, as GnuPG prints it.
    pub fingerprint: String,
    /// The signing subkey's.
    pub subkey: String,
    /// The signing subkey's ID.
    pub subkey_id: String,
    /// The primary key's ID.
    pub key_id: String,
    /// The user IDs.
    pub user_ids: Vec<String>,
    /// Created, as a date.
    pub created: String,
    /// Expires, as a date, or "Never".
    pub expires: String,
    /// The date a revocation certificate kept with the key was made.
    pub revocation_kept: Option<String>,
    /// The date the key was marked revoked.
    pub revoked: Option<String>,
}

/// What a type-7 record shows. The dates are local (`docs/TOTP.md`
/// §2.3); the key's own creation and expiry stay UTC (`key.created`).
pub fn shown(r: &Record, tz_min: i16) -> Option<Shown> {
    let key = key_of(r)?;
    let local = |t: u64| date(crate::vaults::local_secs(t, tz_min));
    Some(Shown {
        fingerprint: faraday_pgp::fingerprint_text(&key.fingerprint()),
        subkey: faraday_pgp::fingerprint_text(&key.subkey_fingerprint()),
        subkey_id: key_id(&key.subkey_fingerprint()),
        key_id: key_id(&key.fingerprint()),
        user_ids: key.user_ids.clone(),
        created: local(u64::from(key.created)),
        expires: if key.expiry == 0 {
            EN.gpg_never.to_string()
        } else {
            local(u64::from(key.created) + u64::from(key.expiry))
        },
        revocation_kept: r
            .field(f::REVOCATION)
            .and_then(signature_time)
            .map(|t| local(u64::from(t))),
        revoked: revoked_at(r).map(|t| local(u64::from(t))),
    })
}

/// The paperkey text of a type-7 record: secret, made only to be shown,
/// wiped when dropped.
pub fn paperkey(r: &Record) -> Option<Zeroizing<String>> {
    let mut key = key_of(r)?;
    let text = key.paperkey();
    key.primary_seed.zeroize();
    key.subkey_seed.zeroize();
    Some(text)
}
