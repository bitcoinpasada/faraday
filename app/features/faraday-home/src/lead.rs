//! Home's lead, the wallet cards and the size the Pi's Home is chosen by (`main`'s Home §2, §3).

use faraday_kernel::session::{FileKind, WaitingItem};
use faraday_kernel::{Public, View};
use faraday_organisms::Icon;
use faraday_port::values::Display;
use faraday_strings::Strings;
use faraday_text::Text;

use crate::Press;

/// A flow a lead can carry on or start over: when its line already stands above the lead, the
/// lead is the next job instead (`main`'s Flows in progress).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Going {
    /// Make a new wallet.
    Create,
    /// A spend: signing a PSBT.
    Spend,
    /// A backup not finished.
    Backup,
}

/// Whether the display is the small panel's class: under 3.5 inches across its diagonal, the
/// class whose Home is the Pi's menu (`main`'s Pi menu model, 04 V10).
pub fn small_panel(display: &Display) -> bool {
    let (w, h) = (u64::from(display.width), u64::from(display.height));
    let dpi = u64::from(display.dpi.max(1));
    // diagonal / dpi < 3.5, squared and doubled twice to stay in integers.
    (w * w + h * h) * 4 < (7 * dpi) * (7 * dpi)
}

/// The job Home leads with: its icon, its line, the line under it, the label of the button it
/// ends in and what a press does.
#[derive(Clone, PartialEq)]
pub(crate) struct Lead {
    pub going: Going,
    pub icon: Icon,
    pub line: Text,
    pub under: Text,
    pub button: &'static str,
    pub press: Press,
}

/// The first of the jobs whose flow is not shown above the lead. Where Make a new wallet is the
/// last of them it is the flow Create, as on `main`: when its own line stands above, no lead
/// stands.
pub(crate) fn pick(jobs: Vec<Lead>, shown: &[Going]) -> Option<Lead> {
    jobs.into_iter().find(|j| !shown.contains(&j.going))
}

/// Make a new wallet as a lead: the one-column Home's last job (the wide Home has no such
/// fallback: with nothing waiting no lead stands and the first actions are the page's).
pub(crate) fn make(s: &Strings) -> Lead {
    Lead {
        going: Going::Create,
        icon: Icon::Wallet,
        line: Text::of(s.home_lead_make_a_new),
        under: Text::of(s.home_lead_single_key),
        button: s.home_btn_make_a_wallet,
        press: Press::Make,
    }
}

/// The jobs the view holds the data for, in `main`'s order: a key made
/// here whose backup is not written, then a PSBT in Files. A spend under way, an import put
/// off, a stick's receipt, a locked vault file and a stick waiting are other owners' state and
/// are not here.
pub(crate) fn jobs(view: &View<'_>, s: &Strings) -> Vec<Lead> {
    let mut jobs = Vec::new();
    for item in view.session.waiting() {
        if let WaitingItem::Held(id) = item
            && let Some(key) = view.session.key(*id)
        {
            jobs.push(Lead {
                going: Going::Backup,
                icon: Icon::Lock,
                line: Text::fill(s.backup_flow_home_lead, &[key.name.as_str()]),
                under: Text::of(s.home_card_backup_not_finished),
                button: s.home_btn_finish,
                press: Press::FinishBackup(*id),
            });
            break;
        }
    }
    if let Some(file) = view
        .session
        .inbox()
        .iter()
        .find(|f| f.kind == FileKind::Psbt)
    {
        jobs.push(Lead {
            going: Going::Spend,
            icon: Icon::Sign,
            line: Text::fill(s.home_job_sign, &[file.name.as_str()]),
            under: Text::of(s.home_job_from_files),
            button: s.home_btn_sign,
            press: Press::Sign(file.id),
        });
    }
    jobs
}

/// Whether nothing is loaded: no key held, no wallet in the session and no vault open. Files
/// brought in and a stick attached do not count (`main`'s Home §1).
pub(crate) fn nothing_loaded(view: &View<'_>) -> bool {
    !view.session.keys().holds_secret()
        && view.session.keys().is_empty()
        && view.session.wallets().is_empty()
        && view.vaults.current().is_none()
}

/// A wallet's card: its name and the one line that matters now.
pub(crate) struct WalletCard {
    pub name: Text,
    pub line: Text,
    /// The line says something to do now, not only the wallet's shape.
    pub needs: bool,
}

/// The session's wallets as cards: the ones that need something now first, then the session's
/// order.
pub(crate) fn wallets(view: &View<'_>, s: &Strings) -> Vec<WalletCard> {
    let session = view.session;
    let mut cards: Vec<WalletCard> = session
        .wallets()
        .iter()
        .map(|w| {
            let psbt = session
                .waiting()
                .iter()
                .any(|i| matches!(i, WaitingItem::Psbt(id) if *id == w.id));
            let unsaved = session.waiting().iter().any(|i| match i {
                WaitingItem::Held(k) => session.key(*k).is_some_and(|k| {
                    w.policy
                        .keys()
                        .iter()
                        .any(|p| p.fingerprint() == Some(k.fingerprint))
                }),
                WaitingItem::Psbt(_) => false,
            });
            let (line, needs) = if psbt {
                (Text::of(s.home_card_a_psbt_to_sign), true)
            } else if unsaved {
                (Text::of(s.home_card_not_backed_up), true)
            } else {
                let keys = w.policy.keys().len();
                match w.policy.quorum() {
                    Some((m, n)) if n > 1 || keys > 1 => (
                        Text::fill(s.sidebar_count, &[&m.to_string(), &n.to_string()]),
                        false,
                    ),
                    _ => (Text::of(s.sidebar_single_key), false),
                }
            };
            WalletCard {
                name: Text::of(w.name.as_str()),
                line,
                needs,
            }
        })
        .collect();
    cards.sort_by_key(|c| !c.needs);
    cards
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(going: Going) -> Lead {
        Lead {
            going,
            icon: Icon::Wallet,
            line: Text::of("x"),
            under: Text::of(""),
            button: "",
            press: Press::Make,
        }
    }

    /// Invariant: The lead is the first job whose flow is not already shown above it, and Make a new wallet is the last; when Create's own flow is shown too, no lead stands.
    /// Regression: The lead repeats a flow whose line is already on the page, or falls back to Make a new wallet while its own flow is under way.
    /// Owner: app
    #[test]
    fn the_lead_skips_a_flow_shown_above_it_and_falls_back_to_create() {
        let jobs = || vec![job(Going::Backup), job(Going::Spend), job(Going::Create)];
        assert!(pick(jobs(), &[]).is_some_and(|l| l.going == Going::Backup));
        assert!(pick(jobs(), &[Going::Backup]).is_some_and(|l| l.going == Going::Spend));
        assert!(
            pick(jobs(), &[Going::Backup, Going::Spend]).is_some_and(|l| l.going == Going::Create)
        );
        assert!(pick(jobs(), &[Going::Backup, Going::Spend, Going::Create]).is_none());
    }

    /// Invariant: The small panel's Home is chosen by the display's physical size: under 3.5 inches across the diagonal.
    /// Regression: The Pi's panel gets the wide Home, or a phone-size window gets the Pi's.
    /// Owner: app
    #[test]
    fn the_small_panel_is_under_three_and_a_half_inches() {
        let display = |width, height, dpi| Display {
            width,
            height,
            dpi,
            orientation: faraday_port::values::Orientation::Portrait,
            rotation_offered: None,
            external: false,
            preview: None,
        };
        assert!(small_panel(&display(480, 640, 286)));
        assert!(!small_panel(&display(1280, 800, 96)));
        assert!(!small_panel(&display(1080, 2400, 420)));
    }
}
