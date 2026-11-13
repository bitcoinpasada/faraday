//! The TOTP codes screen (`docs/TOTP.md` §5): one row per vault entry
//! that carries a TOTP secret, across every open vault. Reading and
//! computing is `faraday_vault::totp`; this module is what comes first
//! on the way in and what a row is.
//!
//! Its presses are the vault area's glue, in `vault/mod.rs`.

pub(crate) mod screen;

use crate::strings::EN;
use faraday_keys::totp::NotTotp;
use faraday_text::Text;
use faraday_vault::records::field;

use crate::vault::VaultAction;
use crate::{Action, Faraday};

/// What a row shows in place of the code.
pub enum CodeKind {
    /// The code, grouped for display, the seconds left in its period,
    /// and the period itself (for the progress bar's fraction).
    Code {
        /// The code, grouped for display (§4.2).
        grouped: Text,
        /// Seconds until the next code.
        left: u32,
        /// The period the code is valid for, seconds.
        period: u64,
    },
    /// Field 6 reads as HOTP (`docs/TOTP.md` §7): not generated here.
    Hotp,
    /// Field 6 does not read at all.
    Unreadable,
    /// The clock is not known: no code is worked out until the time is
    /// set (`docs/TOTP.md` §1.3).
    NoClock,
    /// No code is held for the row: nothing is drawn but the row itself.
    NotHeld,
}

/// Compares the code in constant time and the rest as it is.
impl PartialEq for CodeKind {
    fn eq(&self, other: &Self) -> bool {
        use subtle::ConstantTimeEq;
        match (self, other) {
            (
                Self::Code {
                    grouped,
                    left,
                    period,
                },
                Self::Code {
                    grouped: g2,
                    left: l2,
                    period: p2,
                },
            ) => bool::from(grouped.as_bytes().ct_eq(g2.as_bytes())) && left == l2 && period == p2,
            (Self::Hotp, Self::Hotp)
            | (Self::Unreadable, Self::Unreadable)
            | (Self::NoClock, Self::NoClock)
            | (Self::NotHeld, Self::NotHeld) => true,
            _ => false,
        }
    }
}

impl Eq for CodeKind {}

/// Prints which kind it is and the seconds left, never the code.
impl std::fmt::Debug for CodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code { left, period, .. } => f
                .debug_struct(stringify!(Code))
                .field("left", left)
                .field("period", period)
                .finish_non_exhaustive(),
            Self::Hotp => f.write_str(stringify!(Hotp)),
            Self::Unreadable => f.write_str(stringify!(Unreadable)),
            Self::NoClock => f.write_str(stringify!(NoClock)),
            Self::NotHeld => f.write_str(stringify!(NotHeld)),
        }
    }
}

/// One row of the codes screen (§5.3).
pub struct CodeRow {
    /// Index into `app.vaults.open`.
    pub vault: usize,
    /// The entry's place among that vault's records.
    pub item: usize,
    /// Field 1.
    pub title: Text,
    /// The URI label's account part, else field 2, else empty.
    pub account: Text,
    /// The vault's own name, shown only when more than one is open.
    pub vault_name: Text,
    /// The code, or why there is none.
    pub kind: CodeKind,
}

/// Every row the search bar's query holds, in the order §5.3 draws them:
/// vault by vault in the order they were opened, then in record order.
/// Costs one HMAC for each row, and the query's pass over the titles
/// ([`Vaults::shown_codes`]): for callers outside drawing, which reads
/// [`codes_rows_held`].
pub fn codes_rows(app: &Faraday, now: u64) -> Vec<CodeRow> {
    rows(app, Some(now), false)
}

/// The rows as drawing reads them: the codes the vault holds for the
/// screen (`Faraday::vault_codes_refresh`), the seconds left worked out
/// from the clock. Nothing costs.
pub(crate) fn codes_rows_held(app: &Faraday) -> Vec<CodeRow> {
    rows(app, app.clock_now(), true)
}

/// What an entry gives: its code with the seconds left and the period,
/// Set the time, HOTP or Unreadable.
type Given = Result<Option<(Text, u32, u64)>, NotTotp>;

fn rows(app: &Faraday, now: Option<u64>, held: bool) -> Vec<CodeRow> {
    let mut out = Vec::new();
    let many = app.vaults.open.len() > 1;
    let hold = app.vaults.codes_held().filter(|_| held);
    for (v, ov) in app.vaults.open.iter().enumerate() {
        let records = app.vaults().records(ov.id);
        for &at in app.vaults().shown_codes(ov.id) {
            let Some(r) = records.get(at) else {
                continue;
            };
            let (account, given): (Option<Text>, Option<Given>) = if held {
                // A record the hold says has no row has none; one it
                // does not hold yet keeps its row, with no code.
                let l = hold.and_then(|h| h.of(ov.id, at));
                if l.is_some_and(|l| l.result.is_none()) {
                    continue;
                }
                let given = l
                    .and_then(|l| l.result.as_ref())
                    .map(|result| match (result, now) {
                        (Ok(Some(s)), Some(n)) => Ok(Some((
                            s.grouped.clone(),
                            (s.period - n % s.period.max(1)) as u32,
                            s.period,
                        ))),
                        (Ok(_), _) => Ok(None),
                        (Err(e), _) => Err(*e),
                    });
                (l.and_then(|l| l.account.clone()), given)
            } else {
                let Some(t) = app.vaults.store().totp(
                    ov.id,
                    at,
                    now,
                    faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Totp),
                ) else {
                    continue;
                };
                (
                    t.account,
                    Some(t.code.map(|c| c.map(|s| (s.grouped, s.left, s.period)))),
                )
            };
            let title = Text::of(r.public_text(field::TITLE).unwrap_or(EN.codes_entry));
            let account = account
                .or_else(|| r.public_text(field::USERNAME).map(Text::of))
                .unwrap_or_default();
            let kind = match given {
                None => CodeKind::NotHeld,
                Some(Ok(_)) if now.is_none() => CodeKind::NoClock,
                Some(Ok(Some((grouped, left, period)))) => CodeKind::Code {
                    grouped,
                    left,
                    period,
                },
                Some(Ok(None)) => CodeKind::Unreadable,
                Some(Err(NotTotp::Hotp)) => CodeKind::Hotp,
                Some(Err(NotTotp::Unreadable)) => CodeKind::Unreadable,
            };
            out.push(CodeRow {
                vault: v,
                item: at,
                title,
                account,
                vault_name: if many {
                    Text::of(ov.label(app.vaults.store()).as_str())
                } else {
                    Text::new()
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
