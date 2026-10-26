//! The TOTP codes screen (`docs/TOTP.md` §5): one row per vault entry
//! that carries a TOTP secret, across every open vault. Reading and
//! computing is `faraday_vault::totp`; this module is what comes first
//! on the way in and what a row is.
//!
//! Its presses are the vault area's glue, in `vault/mod.rs`.

pub(crate) mod screen;

use crate::strings::EN;
use faraday_keys::totp::NotTotp;
use faraday_vault::records::field;

use crate::vault::VaultAction;
use crate::{Action, Faraday};

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
    /// The entry's place among that vault's records.
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

/// Every row the search bar's query holds, in the order §5.3 draws them:
/// vault by vault in the order they were opened, then in record order.
pub fn codes_rows(app: &Faraday, now: u64) -> Vec<CodeRow> {
    let mut out = Vec::new();
    let many = app.vaults.open.len() > 1;
    for (v, ov) in app.vaults.open.iter().enumerate() {
        let records = app.vaults().records(ov.id);
        for &at in app.vaults().shown_codes(ov.id) {
            let Some(r) = records.get(at) else {
                continue;
            };
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
                item: at,
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

/// `va` for the codes screen's own actions, matching `vault/screen.rs`'s
/// naming.
pub(crate) fn va(a: VaultAction) -> Action {
    Action::Vault(a)
}
