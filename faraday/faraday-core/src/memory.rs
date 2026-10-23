//! What the app keeps across a lock, outside the process (`docs/history/PLAN.md` §5.2,
//! `docs/WALLETS.md` §3.3): its settings, the signed-amount memory, and
//! what each vault seen open holds (`docs/VAULT.md` §11).
//!
//! The signed-amount memory is the answer to the 2020 two-round attack: a
//! coordinator shows one amount for an input to get a signature, then
//! another for the same transaction. The amounts each input stated are
//! remembered against the unsigned transaction's SHA-256 when it is
//! signed, and the same transaction stating other amounts is refused, with
//! no override. Nothing here is secret: amounts and a hash.

use faraday_bip::bitcoin::consensus::encode::serialize;
use faraday_bip::bitcoin::hashes::{Hash, sha256};
use faraday_psbt::Psbt;
use faraday_vault::Record;
use faraday_vault::records::kind;

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
pub fn from_record(r: faraday_keys::vault::RecordView<'_>) -> Option<Signed> {
    let hash: [u8; 32] = r.public(1)?.try_into().ok()?;
    let amounts = r
        .public_all(2)
        .filter_map(|b| <[u8; 8]>::try_from(b).ok())
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
    pub(crate) fn amounts_from_vault(&mut self, id: faraday_keys::vault::VaultId) {
        let held: Vec<Signed> = self
            .vaults
            .records(id)
            .of(kind::AMOUNTS)
            .filter_map(|(_, r)| from_record(r))
            .collect();
        for e in held {
            if !self.signed_amounts.iter().any(|(h, _)| *h == e.0) {
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
        // The files For the stick the person renamed and no visit has
        // written yet (`docs/FLOWS.md` decision 61).
        if !self.visit.files_menu().renamed.is_empty() {
            let names: Vec<&str> = self
                .visit
                .files_menu()
                .renamed
                .iter()
                .map(String::as_str)
                .collect();
            out.push(("renamed".to_string(), names.join("\n").into_bytes()));
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
        if let Some(b) = self.stick.settings() {
            out.push(("stick-settings".to_string(), b.clone().into_bytes()));
        }
        // What each vault seen open holds, for its locked row and Home
        // (`docs/VAULT.md` §11).
        if !self.vaults.summaries.is_empty() {
            out.push((
                "vault-summaries".to_string(),
                crate::vault::summaries_encode(&self.vaults.summaries),
            ));
        }
        // What was seen of each wallet's backup, for Backups and the
        // wallet card (`docs/history/SIMPLIFY.md` §5).
        if !self.backups_area.records().is_empty() {
            out.push((
                "backups".to_string(),
                crate::backups::encode(self.backups_area.records()),
            ));
        }
        // What the last visit wrote, for Files, Home and each vault's
        // currency (§4.3).
        if let Some(r) = &self.visit.receipt() {
            out.push(("receipt".to_string(), r.encode()));
        }
        // The backup flows: the plan, what each stick read, the step and
        // the written copies' SHA-256. Nothing in it is secret; gone at
        // power-off (`docs/BACKUP-FLOW.md` §7).
        if !self.backup_flows.is_empty() {
            out.push((
                "backup-flow".to_string(),
                crate::backup_flow::encode(&self.backup_flows),
            ));
        }
        // The keys that wait for their backup, apart from the flows: a flow
        // that does not read cannot lift the gate.
        let gate = crate::backup_flow::gate_encode(&self.backup_flows);
        if !gate.is_empty() {
            out.insert(0, ("backup-gate".to_string(), gate));
        }
        // Which stick each vault file came from, for its currency (§3.5).
        if !self.visit.vault_from().is_empty() {
            let lines: Vec<String> = self
                .visit
                .vault_from()
                .iter()
                .map(|(name, hash, label)| {
                    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
                    format!("{hex}\t{label}\t{name}")
                })
                .collect();
            out.push(("vault-from".to_string(), lines.join("\n").into_bytes()));
        }
        if let Some(f) = self.family_kept() {
            out.push(("family".to_string(), f));
        }
        // A time set by hand (`docs/TOTP.md` §1.2 and §1.3): the offset
        // against the shell's own clock, and whether it has been checked
        // this power-on. Lost at power-off, when the Pi's clock restarts
        // at 1970 and the offset would no longer mean anything.
        if self.vaults.clock_checked || self.vaults.clock_offset.is_some() {
            let mut text = format!("checked={}\n", u8::from(self.vaults.clock_checked));
            if let Some(off) = self.vaults.clock_offset {
                text.push_str(&format!("offset={off}\n"));
            }
            out.push(("clock".to_string(), text.into_bytes()));
        }
        // A lock for idleness tells the next process how long the person
        // has been away.
        if let Some(ms) = self.idle_locked_after() {
            out.push(("idle".to_string(), ms.to_string().into_bytes()));
        }
        // Locking for the upgrade (`docs/history/PLAN.md` §5.5): the next process, clean,
        // opens on it.
        if self.upgrade.after_lock() || self.upgrade.state().is_some() {
            out.push(("upgrade".to_string(), b"1".to_vec()));
        }
        out
    }

    /// Reads back what the previous process kept.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn restore_kept(&mut self, kept: &[(String, Vec<u8>)]) {
        // The flows first, then the keys that wait: each key's gate is
        // read against the flows whatever order the entries are listed in.
        let mut gate: Option<&Vec<u8>> = None;
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
                    self.stick
                        .set_settings(Some(String::from_utf8_lossy(bytes).into_owned()));
                }
                "signed-amounts" => self.signed_amounts = decode(bytes),
                "checked-wallets" => {
                    self.checked_wallets = String::from_utf8_lossy(bytes)
                        .lines()
                        .map(str::to_string)
                        .collect();
                }
                "family" => self.family_restore(bytes),
                "clock" => {
                    for line in String::from_utf8_lossy(bytes).lines() {
                        match line.split_once('=') {
                            Some(("offset", v)) => self.vaults.clock_offset = v.parse().ok(),
                            Some(("checked", v)) => self.vaults.clock_checked = v == "1",
                            _ => {}
                        }
                    }
                }
                "receipt" => *self.visit.receipt_mut() = crate::Receipt::decode(bytes),
                "renamed" => {
                    self.visit.files_menu_mut().renamed = String::from_utf8_lossy(bytes)
                        .lines()
                        .map(str::to_string)
                        .collect();
                }
                "backup-flow" => self.flow_restore(bytes),
                "backup-gate" => gate = Some(bytes),
                "backups" => self.backups_area.set_records(crate::backups::decode(bytes)),
                "vault-from" => {
                    for line in String::from_utf8_lossy(bytes).lines() {
                        let mut parts = line.splitn(3, '\t');
                        let (Some(hex), Some(label), Some(name)) =
                            (parts.next(), parts.next(), parts.next())
                        else {
                            continue;
                        };
                        let mut hash = [0u8; 32];
                        let ok = hex.len() == 64
                            && (0..32).all(|i| {
                                u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)
                                    .map(|b| hash[i] = b)
                                    .is_ok()
                            });
                        if ok {
                            self.visit.vault_from_mut().push((
                                name.to_string(),
                                hash,
                                label.to_string(),
                            ));
                        }
                    }
                }
                "vault-summaries" => {
                    self.vaults.summaries = crate::vault::summaries_decode(bytes);
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
        if let Some(bytes) = gate {
            self.flow_gate_restore(bytes);
        }
    }
}
