//! What the app keeps across a lock, outside the process (`PLAN.md` §5.2,
//! `docs/WALLETS.md` §3.3): its settings, the signed-amount memory, and
//! what each vault seen open holds (`docs/VAULT.md` §11).
//!
//! The signed-amount memory is the answer to the 2020 two-round attack: a
//! coordinator shows one amount for an input to get a signature, then
//! another for the same transaction. The amounts each input stated are
//! remembered against the unsigned transaction's SHA-256 when it is
//! signed, and the same transaction stating other amounts is refused, with
//! no override. Nothing here is secret: amounts and a hash.

use faraday_vault::Record;
use faraday_vault::records::kind;
use osk_bip::bitcoin::consensus::encode::serialize;
use osk_bip::bitcoin::hashes::{Hash, sha256};
use osk_psbt::Psbt;

use crate::Faraday;
use crate::wallet::FileKind;

/// What a picture's codes may read as, by the name the kept list gives
/// it; one not named reads as [`FileKind::Other`].
const PICTURE_KINDS: [FileKind; 18] = [
    FileKind::Psbt,
    FileKind::Wallet,
    FileKind::Transaction,
    FileKind::Sheet,
    FileKind::Message,
    FileKind::Key,
    FileKind::Words,
    FileKind::Share,
    FileKind::Vault,
    FileKind::Entries,
    FileKind::Backup,
    FileKind::EfiImage,
    FileKind::Carry,
    FileKind::Kdbx,
    FileKind::Pdf,
    FileKind::SeedPart,
    FileKind::Text,
    FileKind::Other,
];

/// The most transactions remembered; the oldest go first.
const MAX: usize = 1000;

/// One signed transaction: its hash and what each input stated.
pub type Signed = ([u8; 32], Vec<u64>);

/// The unsigned transaction's SHA-256, and the amount each input states,
/// when every input states one.
pub fn stated(psbt: &Psbt) -> Option<Signed> {
    let inner = psbt.inner();
    let hash = sha256::Hash::hash(&serialize(&inner.unsigned_tx)).to_byte_array();
    let mut amounts = Vec::new();
    for (i, input) in inner.inputs.iter().enumerate() {
        let value = match (&input.witness_utxo, &input.non_witness_utxo) {
            (Some(out), _) => out.value,
            (None, Some(tx)) => {
                let vout = inner.unsigned_tx.input.get(i)?.previous_output.vout as usize;
                tx.output.get(vout)?.value
            }
            _ => return None,
        };
        amounts.push(value.to_sat());
    }
    Some((hash, amounts))
}

/// The memory written out: per transaction, the hash, a `u16` count, and
/// the amounts as `u64`, all little-endian.
pub fn encode(entries: &[Signed]) -> Vec<u8> {
    let mut out = Vec::new();
    for (hash, amounts) in entries {
        out.extend_from_slice(hash);
        out.extend_from_slice(&(amounts.len() as u16).to_le_bytes());
        for a in amounts {
            out.extend_from_slice(&a.to_le_bytes());
        }
    }
    out
}

/// The memory read back; a truncated tail is dropped.
pub fn decode(bytes: &[u8]) -> Vec<Signed> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 34 <= bytes.len() {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[at..at + 32]);
        let n = usize::from(u16::from_le_bytes([bytes[at + 32], bytes[at + 33]]));
        at += 34;
        if at + n * 8 > bytes.len() {
            break;
        }
        let amounts = (0..n)
            .map(|k| {
                let mut b = [0u8; 8];
                b.copy_from_slice(&bytes[at + k * 8..at + k * 8 + 8]);
                u64::from_le_bytes(b)
            })
            .collect();
        at += n * 8;
        out.push((hash, amounts));
    }
    out
}

/// A type-10 vault record of one signed transaction (`docs/VAULT.md` §7).
pub fn record_of(entry: &Signed) -> Record {
    let mut r = Record::new(kind::AMOUNTS).with(1, &entry.0);
    for a in &entry.1 {
        r.push(2, &a.to_le_bytes());
    }
    r
}

/// A signed transaction from a type-10 record.
pub fn from_record(r: &Record) -> Option<Signed> {
    let hash: [u8; 32] = r.field(1)?.try_into().ok()?;
    let amounts = r
        .fields
        .iter()
        .filter(|f| f.number == 2)
        .filter_map(|f| <[u8; 8]>::try_from(f.bytes.as_slice()).ok())
        .map(u64::from_le_bytes)
        .collect();
    Some((hash, amounts))
}

impl Faraday {
    /// Whether the same transaction was signed here stating other amounts.
    pub fn amounts_refused(&self, psbt: &Psbt) -> bool {
        let Some((hash, amounts)) = stated(psbt) else {
            return false;
        };
        self.signed_amounts
            .iter()
            .any(|(h, a)| *h == hash && *a != amounts)
    }

    /// Remembers what a signed transaction's inputs stated.
    pub(crate) fn remember_amounts(&mut self, psbt: &Psbt) {
        let Some(entry) = stated(psbt) else {
            return;
        };
        if self.signed_amounts.iter().any(|(h, _)| *h == entry.0) {
            return;
        }
        self.signed_amounts.push(entry);
        if self.signed_amounts.len() > MAX {
            self.signed_amounts.remove(0);
        }
    }

    /// Takes in the signed amounts an open vault holds.
    pub(crate) fn amounts_from_vault(&mut self, records: &[Record]) {
        for r in records.iter().filter(|r| r.kind == kind::AMOUNTS) {
            if let Some(e) = from_record(r)
                && !self.signed_amounts.iter().any(|(h, _)| *h == e.0)
            {
                self.signed_amounts.push(e);
            }
        }
    }

    /// Whether a wallet of this descriptor checksum has been checked
    /// this power-on (`docs/WALLETS.md` §4).
    pub fn is_checked(&self, checksum: &str) -> bool {
        self.checked_wallets.iter().any(|c| c == checksum)
    }

    /// Remembers that a wallet of this descriptor checksum has been
    /// checked.
    pub fn mark_checked(&mut self, checksum: &str) {
        if !self.is_checked(checksum) {
            self.checked_wallets.push(checksum.to_string());
        }
    }

    /// What is kept across a lock: the settings and the memory.
    pub(crate) fn kept(&self) -> Vec<(String, Vec<u8>)> {
        let settings = format!(
            "{}seal-amounts={}\n",
            self.settings_body(),
            u8::from(self.seal_amounts)
        );
        let mut out = vec![
            ("settings".to_string(), settings.into_bytes()),
            ("signed-amounts".to_string(), encode(&self.signed_amounts)),
        ];
        if !self.checked_wallets.is_empty() {
            out.push((
                "checked-wallets".to_string(),
                self.checked_wallets.join("\n").into_bytes(),
            ));
        }
        // Which Outbox files are secrets let out: read again by the next
        // process, a key's text would otherwise pass for public.
        let secret_out: Vec<&str> = self
            .outbox
            .iter()
            .filter(|i| i.secret)
            .map(|i| i.name.as_str())
            .collect();
        if !secret_out.is_empty() {
            out.push(("secret-out".to_string(), secret_out.join("\n").into_bytes()));
        }
        // What each picture in the Inbox holds: the next process does not
        // decode it, and only the disk process could.
        let pictures: Vec<String> = self
            .inbox
            .iter()
            .filter_map(|i| Some(format!("{:?}\t{}", i.picture?, i.name)))
            .collect();
        if !pictures.is_empty() {
            out.push(("pictures".to_string(), pictures.join("\n").into_bytes()));
        }
        // What the boot stick holds, so the next process neither reads it
        // again over settings changed since nor loses what to compare.
        if let Some(b) = &self.stick_settings {
            out.push(("stick-settings".to_string(), b.clone().into_bytes()));
        }
        // What each vault seen open holds, for its locked row and Home
        // (`docs/VAULT.md` §11).
        if !self.vaults.summaries.is_empty() {
            out.push((
                "vault-summaries".to_string(),
                crate::vaults::summaries_encode(&self.vaults.summaries),
            ));
        }
        if let Some(f) = self.family_kept() {
            out.push(("family".to_string(), f));
        }
        // A lock for idleness tells the next process how long the person
        // has been away.
        if let Some(ms) = self.idle_locked_after() {
            out.push(("idle".to_string(), ms.to_string().into_bytes()));
        }
        // Locking for the upgrade (`PLAN.md` §5.5): the next process, clean,
        // opens on it.
        if self.upgrade_after_lock || self.upgrade.is_some() {
            out.push(("upgrade".to_string(), b"1".to_vec()));
        }
        out
    }

    /// Reads back what the previous process kept.
    pub(crate) fn restore_kept(&mut self, kept: &[(String, Vec<u8>)]) {
        for (name, bytes) in kept {
            match name.as_str() {
                "settings" => {
                    for line in String::from_utf8_lossy(bytes).lines() {
                        match line.split_once('=') {
                            Some(("seal-amounts", v)) => self.seal_amounts = v == "1",
                            Some((k, v)) => self.apply_setting(k, v, false),
                            None => {}
                        }
                    }
                }
                "stick-settings" => {
                    self.stick_settings = Some(String::from_utf8_lossy(bytes).into_owned());
                }
                "signed-amounts" => self.signed_amounts = decode(bytes),
                "checked-wallets" => {
                    self.checked_wallets = String::from_utf8_lossy(bytes)
                        .lines()
                        .map(str::to_string)
                        .collect();
                }
                "family" => self.family_restore(bytes),
                "vault-summaries" => {
                    self.vaults.summaries = crate::vaults::summaries_decode(bytes);
                }
                "secret-out" => {
                    let text = String::from_utf8_lossy(bytes);
                    let names: Vec<&str> = text.lines().collect();
                    for i in self.outbox.iter_mut() {
                        if names.contains(&i.name.as_str()) {
                            i.secret = true;
                        }
                    }
                }
                "pictures" => {
                    let text = String::from_utf8_lossy(bytes);
                    for line in text.lines() {
                        let Some((k, name)) = line.split_once('\t') else {
                            continue;
                        };
                        let kind = PICTURE_KINDS
                            .iter()
                            .copied()
                            .find(|c| format!("{c:?}") == k)
                            .unwrap_or(FileKind::Other);
                        for i in self.inbox.iter_mut().filter(|i| i.name == name) {
                            i.picture = Some(kind);
                        }
                    }
                }
                "upgrade" => {
                    if !self.online {
                        self.upgrade_start();
                    }
                }
                "idle" => {
                    if let Ok(ms) = String::from_utf8_lossy(bytes).parse() {
                        self.idle_locked(ms);
                    }
                }
                _ => {}
            }
        }
    }
}
