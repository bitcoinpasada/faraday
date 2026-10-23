//! Secure Boot keys in a vault (`docs/history/PLAN.md` §8): making an owner's PK, KEK
//! and db, the enrolment files, signing `BOOTX64.EFI` from the Inbox, and
//! checking an image's signature. The cryptography is `faraday-sb`'s; the
//! keys live in an open vault as a type-8 record (`docs/VAULT.md` §7) and
//! only certificates, signed updates and signed images reach the Outbox.

pub mod state;

use crate::strings::EN;
use crate::strings::fill;
use faraday_keys::vault::{RecordView, VaultId};
use faraday_sb::{Check, Policy};
use faraday_vault::records::kind;

use crate::Faraday;
use crate::backup_flow::steps::Built;

/// The owner GUID's field of a type-8 record.
pub(crate) const OWNER_FIELD: u8 = f::OWNER;

use faraday_keys::vault::sb_field as f;

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
pub fn shown(r: RecordView<'_>) -> Option<Shown> {
    let name = |n: u8| {
        r.public(n)
            .and_then(faraday_sb::common_name)
            .unwrap_or_default()
    };
    Some(Shown {
        owner: guid_text(r.public(f::OWNER)?),
        names: [name(f::PK_CERT), name(f::KEK_CERT), name(f::DB_CERT)],
    })
}

/// Whether an Inbox file is an EFI image, by its name.
pub fn is_image(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".efi")
}

/// An image's SHA-256 `h`, in groups of four for comparing by eye.
pub fn image_sha256(h: &[u8; 32]) -> String {
    h.chunks(2)
        .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
        .collect::<Vec<_>>()
        .join(" ")
}

impl Faraday {
    /// The Secure Boot keys selected in the open vault: the vault and the
    /// record's place.
    fn sb_selected(&self) -> Option<(VaultId, usize)> {
        let at = self.vault_selected_index()?;
        let id = self.vaults.open.get(self.vaults.current)?.id;
        let r = self.vaults.records(id).get(at)?;
        (r.kind() == kind::SECURE_BOOT).then_some((id, at))
    }

    /// Makes an owner's three keys for the backup flow: the record every
    /// vault of the plan takes.
    pub(crate) fn sb_build(&mut self, owner: &str) -> Result<Built, String> {
        let Some(now) = self.clock_now() else {
            return Err(EN.secureboot_this_computer.to_string());
        };
        let Some(seed) = self.fresh(b"secure boot") else {
            return Err(EN.secureboot_no_randomness.to_string());
        };
        let owner = owner.trim();
        // The enrolment files are made with the keys, as the policy chosen
        // says, and written in the flow with the other public files.
        let policy = if self.vaults.sb_own_only {
            Policy::OwnKeysOnly
        } else {
            Policy::Windows
        };
        let blind_seed = self
            .fresh(b"secure boot enrolment")
            .ok_or_else(|| EN.secureboot_no_randomness.to_string())?;
        let made = faraday_keys::secure_boot::create(&seed, &blind_seed, owner, policy, now)
            .map_err(|e| e.reason().to_string())?;
        Ok(Built {
            record: made.key,
            subject: made.owner.to_vec(),
            name: if owner.is_empty() {
                guid_text(&made.owner)
            } else {
                owner.to_string()
            },
            outbox: made.enrolment,
        })
    }

    /// The enrolment files for the selected keys to the Outbox.
    pub(crate) fn sb_enrol(&mut self) {
        if self.selected_waits().is_some() {
            return;
        }
        let Some((id, at)) = self.sb_selected() else {
            return;
        };
        let Some(now) = self.clock_now() else {
            return self.toast(EN.secureboot_this_computer);
        };
        let policy = if self.vaults.sb_own_only {
            Policy::OwnKeysOnly
        } else {
            Policy::Windows
        };
        // The signing is blinded with fresh bytes of the system's.
        let Some(seed) = self.fresh(b"secure boot enrolment") else {
            return self.toast(EN.secureboot_no_randomness);
        };
        match self.vaults.store().sb_enrolment(id, at, &seed, policy, now) {
            Some(Ok(files)) => {
                let n = files.len();
                for (name, bytes) in files {
                    self.put_outbox(&name, bytes);
                }
                self.toast_out(&fill(EN.secureboot_enrolment, &[&n.to_string()]));
            }
            Some(Err(e)) => self.toast(e.reason()),
            None => {}
        }
    }

    /// Signs Inbox image `k` with the selected db key; the signed image
    /// goes to the Outbox under the same name.
    pub(crate) fn sb_sign(&mut self, k: usize) {
        if self.selected_waits().is_some() {
            return;
        }
        let Some((id, at)) = self.sb_selected() else {
            return;
        };
        if self.inbox.get(k).is_none() {
            return;
        }
        // The signing is blinded with fresh bytes of the system's.
        let Some(seed) = self.fresh(b"secure boot signing") else {
            return self.toast(EN.secureboot_no_randomness);
        };
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let name = item.name.clone();
        match self
            .vaults
            .store()
            .sb_sign_image(id, at, &seed, &item.bytes)
        {
            Some(Ok(signed)) => {
                self.put_outbox(&name, signed);
                self.vaults.sb_sign = None;
                self.vaults.sb_images = false;
                self.toast_out(&fill(EN.secureboot_signed, &[&name.to_string()]));
            }
            Some(Err(e)) => self.toast(e.reason()),
            None => {}
        }
    }

    /// What Inbox image `k`'s signature is, against the selected db key.
    pub(crate) fn sb_check(&mut self, k: usize) {
        let Some((id, at)) = self.sb_selected() else {
            return;
        };
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let Some(db_cert) = self
            .vaults
            .records(id)
            .get(at)
            .and_then(|r| r.public(f::DB_CERT))
        else {
            return;
        };
        let said = match faraday_sb::pe::check(&item.bytes, db_cert) {
            Ok(Check::Valid) => EN.secureboot_signed_by_vault,
            Ok(Check::OtherSigner) => EN.secureboot_signed_by_other,
            Ok(Check::Altered) => EN.secureboot_changed,
            Ok(Check::Unsigned) => EN.secureboot_not_signed,
            Ok(Check::Unreadable) => EN.secureboot_its_signature,
            Err(e) => e.reason(),
        };
        let line = format!("{}: {said}", item.name);
        self.toast(&line);
        self.vaults.sb_checked = Some(line);
    }
}
