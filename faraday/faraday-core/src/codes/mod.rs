//! The TOTP codes screen (`docs/TOTP.md` §5): one row per vault entry
//! that carries a TOTP secret, across every open vault. Reading and
//! computing is `faraday_vault::totp`; this module is what comes first
//! on the way in and what a row is.

pub(crate) mod screen;

use crate::strings::EN;
use faraday_keys::totp::NotTotp;
use faraday_vault::records::{field, kind};

use crate::vault::VaultAction;
use crate::{Action, Faraday, Screen};

/// What a row shows in place of the code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeKind {
    /// The code, grouped for display, the seconds left in its period,
    /// and the period itself (for the progress bar's fraction).
    Code {
        /// The code, grouped for display (§4.2).
        grouped: String,
        /// Seconds until the next code.
        left: u32,
        /// The period the code is valid for, seconds.
        period: u64,
    },
    /// Field 6 reads as HOTP (`docs/TOTP.md` §7): not generated here.
    Hotp,
    /// Field 6 does not read at all.
    Unreadable,
}

/// One row of the codes screen (§5.3).
pub struct CodeRow {
    /// Index into `app.vaults.open`.
    pub vault: usize,
    /// This entry's position among that vault's Entries (as
    /// `vaults.item` holds it).
    pub item: usize,
    /// Field 1.
    pub title: String,
    /// The URI label's account part, else field 2, else empty.
    pub account: String,
    /// The vault's own name, shown only when more than one is open.
    pub vault_name: String,
    /// The code, or why there is none.
    pub kind: CodeKind,
}

/// Every row, in the order §5.3 draws them: vault by vault in the order
/// they were opened, then in record order.
pub fn codes_rows(app: &Faraday, now: u64) -> Vec<CodeRow> {
    let mut out = Vec::new();
    let many = app.vaults.open.len() > 1;
    for (v, ov) in app.vaults.open.iter().enumerate() {
        for (item, (at, r)) in app
            .vaults
            .records(ov.id)
            .iter()
            .filter(|(_, r)| r.kind() == kind::ENTRY)
            .enumerate()
        {
            let Some(t) = app.vaults.store().totp(
                ov.id,
                at,
                Some(now),
                faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Totp),
            ) else {
                continue;
            };
            let title = r
                .public_text(field::TITLE)
                .unwrap_or(EN.codes_entry)
                .to_string();
            let account = t
                .account
                .or_else(|| r.public_text(field::USERNAME).map(str::to_string))
                .unwrap_or_default();
            let kind = match t.code {
                Ok(Some(code)) => CodeKind::Code {
                    grouped: code.grouped,
                    left: code.left,
                    period: code.period,
                },
                Ok(None) => CodeKind::Unreadable,
                Err(NotTotp::Hotp) => CodeKind::Hotp,
                Err(NotTotp::Unreadable) => CodeKind::Unreadable,
            };
            out.push(CodeRow {
                vault: v,
                item,
                title,
                account,
                vault_name: if many {
                    ov.label(app.vaults.store())
                } else {
                    String::new()
                },
                kind,
            });
        }
    }
    out
}

impl Faraday {
    /// Opens the codes screen (§5.2): no vault open leads to Create or
    /// Unlock first (handled by the tile and by `vault_way`); the clock
    /// not checked this power-on opens Set the time first. Both checks
    /// happen on the way in only.
    pub(crate) fn open_codes(&mut self) {
        self.screen = Screen::Codes;
        if !self.clock_checked() {
            self.open_set_time();
        }
    }

    /// A row pressed: opens that entry in its vault (Vault contents,
    /// Entries, the record selected).
    pub(crate) fn open_code_row(&mut self, vault: usize, item: usize) {
        self.vaults.current = vault;
        self.vault_act(VaultAction::Category(2));
        self.vault_act(VaultAction::Item(item));
        self.screen = Screen::VaultContents;
    }

    /// Add an entry from the codes screen: the entry form, in the
    /// current vault, the codes screen still showing behind it.
    pub(crate) fn codes_add_entry(&mut self) {
        self.vaults.category = 2;
        self.vault_act(VaultAction::Add);
    }
}

/// `va` for the codes screen's own actions, matching `vault/screen.rs`'s
/// naming.
pub(crate) fn va(a: VaultAction) -> Action {
    Action::Vault(a)
}

impl Faraday {
    /// Handles the Codes screen's actions (`OpenCodes`, `CodesOpen`, `CodesAdd`).
    pub(crate) fn codes_act(&mut self, action: Action) {
        match action {
            Action::OpenCodes => self.open_codes(),
            Action::CodesOpen(v, item) => self.open_code_row(v, item),
            Action::CodesAdd => self.codes_add_entry(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
