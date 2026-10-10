//! Secure Boot keys in a vault (`PLAN.md` §8): making an owner's PK, KEK
//! and db, the enrolment files, signing `BOOTX64.EFI` from the Inbox, and
//! checking an image's signature. The cryptography is `faraday-sb`'s; the
//! keys live in an open vault as a type-8 record (`docs/VAULT.md` §7) and
//! only certificates, signed updates and signed images reach the Outbox.

use faraday_sb::{Check, Keys, Pair, Policy};
use faraday_vault::Record;
use faraday_vault::records::kind;
use osk_bip::bitcoin::hashes::{Hash, sha256};

use crate::Faraday;

/// The fields of a type-8 record.
mod f {
    pub const OWNER: u8 = 1;
    pub const PK_KEY: u8 = 2;
    pub const PK_CERT: u8 = 3;
    pub const KEK_KEY: u8 = 4;
    pub const KEK_CERT: u8 = 5;
    pub const DB_KEY: u8 = 6;
    pub const DB_CERT: u8 = 7;
}

fn pair(r: &Record, key: u8, cert: u8) -> Option<Pair> {
    Some(Pair {
        key: zeroize::Zeroizing::new(r.field(key)?.to_vec()),
        cert: r.field(cert)?.to_vec(),
    })
}

/// The keys a type-8 record holds.
pub fn keys_of(r: &Record) -> Option<Keys> {
    Some(Keys {
        owner: r.field(f::OWNER)?.try_into().ok()?,
        pk: pair(r, f::PK_KEY, f::PK_CERT)?,
        kek: pair(r, f::KEK_KEY, f::KEK_CERT)?,
        db: pair(r, f::DB_KEY, f::DB_CERT)?,
    })
}

fn record_of(k: &Keys) -> Record {
    Record::new(kind::SECURE_BOOT)
        .with(f::OWNER, &k.owner)
        .with(f::PK_KEY, &k.pk.key)
        .with(f::PK_CERT, &k.pk.cert)
        .with(f::KEK_KEY, &k.kek.key)
        .with(f::KEK_CERT, &k.kek.cert)
        .with(f::DB_KEY, &k.db.key)
        .with(f::DB_CERT, &k.db.cert)
}

/// A GUID as text.
pub fn guid_text(g: &[u8]) -> String {
    if g.len() != 16 {
        return String::new();
    }
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{}",
        u32::from_le_bytes([g[0], g[1], g[2], g[3]]),
        u16::from_le_bytes([g[4], g[5]]),
        u16::from_le_bytes([g[6], g[7]]),
        g[8],
        g[9],
        g[10..]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

/// What the detail column shows of an owner's keys.
pub struct Shown {
    /// The owner GUID.
    pub owner: String,
    /// The three certificates' common names: PK, KEK, db.
    pub names: [String; 3],
}

/// What a type-8 record shows.
pub fn shown(r: &Record) -> Option<Shown> {
    let name = |n: u8| {
        r.field(n)
            .and_then(faraday_sb::common_name)
            .unwrap_or_default()
    };
    Some(Shown {
        owner: guid_text(r.field(f::OWNER)?),
        names: [name(f::PK_CERT), name(f::KEK_CERT), name(f::DB_CERT)],
    })
}

/// Whether an Inbox file is an EFI image, by its name.
pub fn is_image(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".efi")
}

/// An image's SHA-256, in groups of four for comparing by eye.
pub fn image_sha256(bytes: &[u8]) -> String {
    let h = sha256::Hash::hash(bytes).to_byte_array();
    h.chunks(2)
        .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
        .collect::<Vec<_>>()
        .join(" ")
}

impl Faraday {
    /// The Secure Boot keys selected in the open vault.
    fn sb_selected(&self) -> Option<Keys> {
        let at = self.vault_selected_index()?;
        let r = self
            .vaults
            .open
            .get(self.vaults.current)?
            .contents
            .records
            .get(at)?;
        (r.kind == kind::SECURE_BOOT).then(|| keys_of(r))?
    }

    /// Makes an owner's three keys into the open vault.
    pub(crate) fn sb_make(&mut self) {
        let Some(now) = self.vaults.unix_secs else {
            return self.toast("This computer's clock is not known");
        };
        let Some(seed) = self.fresh(b"secure boot") else {
            return self.toast("No randomness from the system yet. Try again");
        };
        let owner = self.vaults.sb_owner.clone();
        match faraday_sb::make(&seed, &owner, now) {
            Ok(keys) => {
                self.vault_push(record_of(&keys), "PK, KEK and db are in the vault");
                let n = self
                    .vaults
                    .open
                    .get(self.vaults.current)
                    .map_or(0, |v| v.contents.of(kind::SECURE_BOOT).count());
                self.vaults.item[self.vaults.category] = n.saturating_sub(1);
            }
            Err(e) => self.toast(e.reason()),
        }
    }

    /// The enrolment files for the selected keys to the Outbox.
    pub(crate) fn sb_enrol(&mut self) {
        let Some(keys) = self.sb_selected() else {
            return;
        };
        let Some(now) = self.vaults.unix_secs else {
            return self.toast("This computer's clock is not known");
        };
        let policy = if self.vaults.sb_own_only {
            Policy::OwnKeysOnly
        } else {
            Policy::Windows
        };
        match faraday_sb::enrolment(&keys, policy, now) {
            Ok(files) => {
                let n = files.len();
                for (name, bytes) in files {
                    self.put_outbox(&name, bytes);
                }
                self.toast_out(&format!("{n} enrolment files"));
            }
            Err(e) => self.toast(e.reason()),
        }
    }

    /// Signs Inbox image `k` with the selected db key; the signed image
    /// goes to the Outbox under the same name.
    pub(crate) fn sb_sign(&mut self, k: usize) {
        let Some(keys) = self.sb_selected() else {
            return;
        };
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let name = item.name.clone();
        match faraday_sb::pe::sign(&item.bytes, &keys.db) {
            Ok(signed) => {
                self.put_outbox(&name, signed);
                self.vaults.sb_sign = None;
                self.vaults.sb_images = false;
                self.toast_out(&format!("{name}, signed,"));
            }
            Err(e) => self.toast(e.reason()),
        }
    }

    /// What Inbox image `k`'s signature is, against the selected db key.
    pub(crate) fn sb_check(&mut self, k: usize) {
        let Some(keys) = self.sb_selected() else {
            return;
        };
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let said = match faraday_sb::pe::check(&item.bytes, &keys.db.cert) {
            Ok(Check::Valid) => "Signed by this vault's db key",
            Ok(Check::OtherSigner) => "Signed by another key",
            Ok(Check::Altered) => "Changed since it was signed",
            Ok(Check::Unsigned) => "Not signed",
            Ok(Check::Unreadable) => "Its signature could not be read",
            Err(e) => e.reason(),
        };
        let line = format!("{}: {said}", item.name);
        self.toast(&line);
        self.vaults.sb_checked = Some(line);
    }
}
