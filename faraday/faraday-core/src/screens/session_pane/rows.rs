//! The session pane's rows as data: what the session holds, in groups
//! (`docs/SESSION-PANE.md`). No geometry here, so a menu page can build the
//! same rows without Home's columns.

use super::SessionRow;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::Glyph;
use crate::{Action, Screen};
use faraday_draw::widgets::Icon;

/// One group of the pane: its heading and its rows.
pub(crate) struct Group {
    /// The small heading over the rows.
    pub label: &'static str,
    /// The rows, never none.
    pub rows: Vec<SessionRow>,
    /// The Waiting band at the pane's foot: always the last group.
    pub band: bool,
}

/// The pane's groups in their order: Waiting, Wallets (Backups last), Keys.
/// A group with no rows is left out.
pub(crate) fn groups(app: &View) -> Vec<Group> {
    let mut groups = home_groups(app);
    if crate::row_open::home_opens(app) {
        super::page_mode::home_opening(app, &mut groups);
    }
    groups
}

fn home_groups(app: &View) -> Vec<Group> {
    with_band(
        [
            (EN.session_group_keys, key_rows(app)),
            (EN.session_group_wallets, wallet_rows(app)),
        ],
        waiting_rows(app),
    )
}

/// The pane's groups: `groups` in their order, those with no rows left
/// out, then Waiting as the band at the foot, left out with no rows.
pub(super) fn with_band(
    groups: impl IntoIterator<Item = (&'static str, Vec<SessionRow>)>,
    waiting: Vec<SessionRow>,
) -> Vec<Group> {
    groups
        .into_iter()
        .map(|(label, rows)| (label, rows, false))
        .chain(std::iter::once((EN.session_group_waiting, waiting, true)))
        .filter(|(_, rows, _)| !rows.is_empty())
        .map(|(label, rows, band)| Group { label, rows, band })
        .collect()
}

/// A row of the wide card: wrapping its second line.
pub(super) fn row(icon: impl Into<Glyph>, head: String, sub: String, action: Action) -> SessionRow {
    SessionRow {
        icon: icon.into(),
        head,
        sub,
        trailing: String::new(),
        accent: false,
        action,
        button: None,
        wrap: true,
        selected: false,
        open: None,
    }
}

/// Waiting: each job (the spend under way, a backup not finished, each
/// PSBT to sign fewest signatures first, an import put off, the last write,
/// each locked vault file, a vault changed since it was written, a stick
/// waiting), then each flow in progress not already one of them, and the
/// files to write. A job with a button carries it under its two lines.
fn waiting_rows(app: &View) -> Vec<SessionRow> {
    let jobs = super::super::home::pane_jobs(app);
    let mut rows: Vec<SessionRow> = jobs
        .iter()
        .map(|(_, c)| {
            let mut r = row(c.icon, c.line.clone(), c.under.clone(), c.action);
            if !c.button.is_empty() {
                r.button = Some((c.button, c.enabled));
            }
            r
        })
        .collect();
    rows.extend(progress_rows(app, |g| {
        jobs.iter().any(|(j, _)| *j == Some(g))
    }));
    if !app.outbox.is_empty() {
        rows.push(row(
            Icon::File,
            app.files_count().to_string(),
            String::new(),
            Action::Nav(Screen::Files),
        ));
    }
    rows
}

/// The flows in progress that `shown` does not already name, a Resume
/// button under each.
pub(super) fn progress_rows(
    app: &View,
    shown: impl Fn(crate::in_progress::Going) -> bool,
) -> Vec<SessionRow> {
    app.flows_in_progress()
        .iter()
        .filter(|f| !shown(f.going))
        .map(|f| {
            let icon = match f.tab {
                Screen::Vaults => Icon::Lock,
                Screen::Catalog => Icon::Tools,
                _ => Icon::Wallet,
            };
            let mut r = row(
                icon,
                f.name.to_string(),
                f.step.to_string(),
                Action::Resume(f.going),
            );
            r.button = Some((EN.home_btn_resume, true));
            r
        })
        .collect()
}

/// Wallets: every loaded wallet, its need in orange, then Backups while
/// something is not backed up.
pub(super) fn wallet_rows(app: &View) -> Vec<SessionRow> {
    let mut rows: Vec<SessionRow> = super::super::home::home_wallets(app)
        .into_iter()
        .map(|c| {
            let icon = if c.needs { Icon::Warning } else { Icon::Wallet };
            row(icon, c.name, c.line, Action::OpenWallet(c.index))
        })
        .collect();
    let (_, without) = app.backup_counts();
    if without > 0 {
        rows.push(row(
            Icon::Shield,
            EN.home_ways_backups.to_string(),
            fill(EN.home_ways_not_backed, &[&without.to_string()]),
            Action::Nav(Screen::Backups),
        ));
    }
    rows
}

/// The facts of key `fp` as its row says them, whole: its name or
/// fingerprint, then the line under it.
pub(crate) fn key_facts(app: &View, fp: [u8; 4]) -> Option<(String, String)> {
    key_rows(app)
        .into_iter()
        .find_map(|r| (r.action == Action::ExploreKey(fp)).then_some((r.head, r.sub)))
}

/// Keys: every key held, its name in full, the wallet that uses it named in
/// its line ("No wallet uses it" otherwise). A press opens the key.
pub(super) fn key_rows(app: &View) -> Vec<SessionRow> {
    let session = &app.session;
    session
        .keys
        .all()
        .map(|k| {
            let fp = crate::wallet::fp_text(k.fingerprint);
            let name = crate::bip85::key_name(k);
            let mut sub = String::new();
            if !name.eq_ignore_ascii_case(&fp) {
                sub.push_str(&name);
                sub.push_str(" · ");
            }
            if k.has_passphrase {
                sub.push_str(EN.home_key_passphrase);
                sub.push_str(" · ");
            }
            // A key that went in by a codex32 correction says so
            // (`docs/CODEX32.md` §8.4).
            if let Some(mark) = session.key_marks.of(k.fingerprint.0) {
                sub.push_str(mark.fingerprint_line());
                sub.push_str(" · ");
            }
            let user = session.wallets.iter().find(|w| {
                session
                    .slots(w)
                    .iter()
                    .any(|s| s.fingerprint == Some(k.fingerprint))
            });
            match user {
                Some(w) => sub.push_str(&fill(EN.home_key_used_by, &[&w.name])),
                None => sub.push_str(EN.home_card_no_wallet_uses_it),
            }
            row(
                Icon::Keys,
                String::from(&*fp),
                sub,
                Action::ExploreKey(k.fingerprint.0),
            )
        })
        .collect()
}

/// The phone's and the Pi's rows (`docs/HOME.md` §3), as they have been:
/// what needs you first (flows, files, wallets that need something,
/// Backups), the keys no wallet uses with a Make a wallet button, All keys.
pub(crate) fn compact_rows(
    app: &View,
    lead: Option<&super::super::home::CardLead>,
) -> (Vec<SessionRow>, Vec<SessionRow>, Option<SessionRow>) {
    let plain = |icon: Icon, head: String, sub: String, action: Action| SessionRow {
        wrap: false,
        ..row(icon, head, sub, action)
    };
    let mut rest: Vec<SessionRow> = app
        .flows_in_progress()
        .iter()
        .filter(|r| !lead.is_some_and(|l| l.going() == Some(r.going)))
        .map(|r| {
            let icon = match r.tab {
                Screen::Vaults => Icon::Lock,
                Screen::Catalog => Icon::Tools,
                _ => Icon::Wallet,
            };
            plain(
                icon,
                r.name.to_string(),
                r.step.to_string(),
                Action::Resume(r.going),
            )
        })
        .collect();
    if !app.outbox.is_empty() {
        rest.push(plain(
            Icon::File,
            app.files_count().to_string(),
            String::new(),
            Action::Nav(Screen::Files),
        ));
    }
    rest.extend(
        super::super::home::home_wallets(app)
            .into_iter()
            .filter(|c| c.needs && !lead.is_some_and(|l| l.wallets().contains(&c.index)))
            .map(|c| plain(Icon::Warning, c.name, c.line, Action::OpenWallet(c.index))),
    );
    let (_, without) = app.backup_counts();
    if without > 0 {
        rest.push(plain(
            Icon::Shield,
            EN.home_ways_backups.to_string(),
            fill(EN.home_ways_not_backed, &[&without.to_string()]),
            Action::Nav(Screen::Backups),
        ));
    }
    let n = app.session.keys.all().count();
    if n == 0 {
        return (rest, Vec::new(), None);
    }
    let loose = app.session.loose_keys();
    let keys: Vec<SessionRow> = app
        .session
        .keys
        .all()
        .filter(|k| loose.contains(&k.fingerprint))
        .map(|k| {
            let fp = crate::wallet::fp_text(k.fingerprint);
            let name = crate::bip85::key_name(k);
            let mut sub = String::new();
            if !name.eq_ignore_ascii_case(&fp) {
                sub.push_str(&name);
                sub.push_str(" · ");
            }
            if k.has_passphrase {
                sub.push_str(EN.home_key_passphrase);
                sub.push_str(" · ");
            }
            sub.push_str(EN.home_card_no_wallet_uses_it);
            SessionRow {
                trailing: EN.home_btn_make_a_wallet.to_string(),
                accent: true,
                ..plain(
                    Icon::Keys,
                    String::from(&*fp),
                    sub,
                    Action::KeyWallet(k.fingerprint.0, 1),
                )
            }
        })
        .collect();
    let all = SessionRow {
        trailing: n.to_string(),
        ..plain(
            Icon::Keys,
            EN.home_all_keys.to_string(),
            String::new(),
            Action::Explore,
        )
    };
    (rest, keys, Some(all))
}
