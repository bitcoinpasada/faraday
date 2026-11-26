//! The session pane in a menu page's own mode (`docs/WORKSPACE.md` §2): one
//! list in the pane's width, each group's small heading then its rows, the
//! groups the page's tools take in the order they take them. Open vaults,
//! files to write and Backups are not in it. The rows are the pane's
//! (`rows`), so a row reads the same here as on Home.

use super::rows::{Group, add_key_buttons, key_rows, progress_rows, row, wallet_rows, with_band};
use super::{SessionRow, band, card_surface, list_rows, rows_h};
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Region, Slot, Ui};
use crate::{Action, Screen};
use faraday_draw::tokens::{GRID, PAD, TITLE_M, TITLE_S};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::section_label;

/// The menu pages that have a pane of their own mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Tools: a waiting PSBT leads, else the keys.
    Tools,
    /// Check: the files first.
    Check,
    /// Wallets: what waits (a PSBT with Sign, wallet files, flows in
    /// progress), then the keys. The wallets are the page's tile.
    Wallets,
    /// Vaults: the keys, then the wallets (this session's, as the tile's
    /// vault files take Load and Save later), Waiting at the foot.
    Vaults,
}

/// The PSBTs waiting to be signed as rows with Sign, fewest signatures
/// first, as Home's Waiting group has them.
fn psbt_rows(app: &View) -> Vec<SessionRow> {
    crate::screens::home::pane_jobs(app)
        .into_iter()
        .filter(|(_, c)| matches!(c.action, Action::StartSpend(_)))
        .map(|(_, c)| {
            let mut r = row(c.icon, c.line, c.under, c.action);
            if !c.button.is_empty() {
                r.button = Some((c.button, c.enabled));
            }
            if let Action::StartSpend(k) = c.action {
                r.open = Some(Action::RowOpen(crate::row_open::Row::File(k)));
            }
            r
        })
        .collect()
}

/// Tools' and Check's band: the lead with its button, the waiting PSBTs, an
/// import put off and the locked vault files, and the flows those carry on.
fn band_rows(app: &View) -> (Vec<SessionRow>, Vec<crate::in_progress::Going>) {
    let jobs = crate::screens::home::band_jobs(app);
    let goings = jobs.iter().filter_map(|(g, _)| *g).collect();
    let rows = jobs
        .into_iter()
        .map(|(_, c)| {
            let mut r = row(c.icon, c.line, c.under, c.action);
            if !c.button.is_empty() {
                r.button = Some((c.button, c.enabled));
            }
            if let Action::StartSpend(k) = c.action {
                r.open = Some(Action::RowOpen(crate::row_open::Row::File(k)));
            }
            r
        })
        .collect();
    (rows, goings)
}

/// The files held that a tool takes, a PSBT or a transaction first, then a
/// wallet or setup file, then the rest; a vault file is a vault's, not
/// listed. A PSBT is in `psbt_rows`, not here.
fn file_rows(app: &View, keep: impl Fn(&crate::Item) -> bool) -> Vec<SessionRow> {
    let buttons = app.inbox_buttons();
    let mut held: Vec<(u8, usize, &crate::Item)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, it)| it.facts.is_none())
        .filter(|(_, it)| it.psbt().is_none())
        .filter(|(_, it)| keep(it))
        .map(|(k, it)| {
            let rank = if it.unsigned_txid.is_some() {
                0
            } else if it.wallet().is_some() {
                1
            } else {
                2
            };
            (rank, k, it)
        })
        .collect();
    held.sort_by_key(|(rank, _, _)| *rank);
    held.into_iter()
        .map(|(_, k, it)| {
            // A file whose kind has a button carries it, and the button
            // does what the file's menu does first.
            match buttons.get(k).copied().flatten() {
                Some((label, action)) => {
                    let mut r = row(Icon::File, it.name.to_string(), String::new(), action);
                    r.button = Some((label, true));
                    r.open = Some(Action::RowOpen(crate::row_open::Row::File(k)));
                    r
                }
                None => row(
                    Icon::File,
                    it.name.to_string(),
                    String::new(),
                    Action::RowOpen(crate::row_open::Row::File(k)),
                ),
            }
        })
        .collect()
}

/// The pane's groups for `mode`, in the page's order (WORKSPACE.md §2's
/// table); a group with no rows is left out.
pub(crate) fn groups(app: &View, mode: Mode) -> Vec<Group> {
    let mut groups = built_groups(app, mode);
    // Vaults has no details of its own yet: its rows keep their presses.
    if mode != Mode::Vaults {
        opening(app, &mut groups);
    }
    groups
}

/// Home's card on the wide layout: a held file's row (a PSBT with Sign)
/// opens its details on a press beyond its button, then as every page.
pub(super) fn home_opening(app: &View, groups: &mut [Group]) {
    for r in groups.iter_mut().flat_map(|g| g.rows.iter_mut()) {
        if let (Action::StartSpend(k), Some(_)) = (r.action, r.button) {
            r.open = Some(Action::RowOpen(crate::row_open::Row::File(k)));
        }
    }
    opening(app, groups);
}

/// A press on a key's, a wallet's or a held file's row opens it in the
/// page's second pane (`docs/WORKSPACE.md` §7), the open one marked.
fn opening(app: &View, groups: &mut [Group]) {
    use crate::row_open::{Opened, Row};
    let opened = crate::row_open::opened(app);
    for r in groups.iter_mut().flat_map(|g| g.rows.iter_mut()) {
        // A row with a button opens its details on a press beyond it.
        if let Some(Action::RowOpen(crate::row_open::Row::File(k))) = r.open {
            r.selected = opened == Some(Opened::File(k));
            continue;
        }
        if let Some(Action::OpenWallet(i)) = r.open {
            r.open = Some(Action::RowOpen(Row::Wallet(i)));
            r.selected = opened == Some(Opened::Wallet(i));
            continue;
        }
        let (row, now) = match r.action {
            Action::ExploreKey(fp) => (Row::Key(fp), Opened::Key(fp)),
            Action::OpenWallet(i) => (Row::Wallet(i), Opened::Wallet(i)),
            Action::RowOpen(Row::File(k)) => (Row::File(k), Opened::File(k)),
            _ => continue,
        };
        r.action = Action::RowOpen(row);
        r.selected = opened == Some(now);
    }
}

fn built_groups(app: &View, mode: Mode) -> Vec<Group> {
    if mode == Mode::Wallets {
        return wallets_groups(app);
    }
    if mode == Mode::Vaults {
        return vaults_groups(app);
    }
    // A waiting PSBT is the band's, not the files'; each flow in progress
    // that is not the band's is the group after the others.
    let (waiting, band_goings) = band_rows(app);
    let progress = progress_rows(app, |g| band_goings.contains(&g));
    let wallets: Vec<SessionRow> = wallet_rows(app)
        .into_iter()
        .filter(|r| r.action != Action::Nav(Screen::Backups))
        .collect();
    let wallets = add_key_buttons(app, wallets);
    let order = match mode {
        Mode::Tools => [Slotted::Keys, Slotted::Wallets, Slotted::Files],
        Mode::Wallets | Mode::Vaults => unreachable!("their groups are built before"),
        Mode::Check => [Slotted::Files, Slotted::Keys, Slotted::Wallets],
    };
    let mut files = Some(file_rows(app, |_| true));
    let mut wallets = Some(wallets);
    let groups: Vec<(&'static str, Vec<SessionRow>)> = order
        .into_iter()
        .filter_map(|s| match s {
            Slotted::Files => files.take().map(|r| (EN.session_group_files, r)),
            Slotted::Wallets => wallets.take().map(|r| (EN.session_group_wallets, r)),
            Slotted::Keys => Some((EN.session_group_keys, key_rows(app))),
        })
        .chain(std::iter::once((EN.session_group_progress, progress)))
        .collect();
    with_band(groups, waiting)
}

/// Wallets' groups: Keys, then Waiting at the foot (a waiting PSBT with
/// Sign, the wallet files held, the flows in progress); the wallets are
/// the tile's.
fn wallets_groups(app: &View) -> Vec<Group> {
    let mut waiting = psbt_rows(app);
    let signing = waiting_spend(&waiting);
    waiting.extend(file_rows(app, |it| it.wallet().is_some()));
    waiting.extend(progress_rows(app, |g| {
        signing && g == crate::in_progress::Going::Spend
    }));
    with_band([(EN.session_group_keys, key_rows(app))], waiting)
}

/// Whether a row of `rows` is a PSBT's Sign, which names the spend flow.
fn waiting_spend(rows: &[SessionRow]) -> bool {
    rows.iter()
        .any(|r| matches!(r.action, Action::StartSpend(_)))
}

/// A group's place in a page's order.
#[derive(Clone, Copy)]
enum Slotted {
    Files,
    Wallets,
    Keys,
}

/// The height of the pane's content: the heading, then each group's
/// heading and rows, one under another, the band last, `w` wide with its
/// padding.
pub(crate) fn content_h(ui: &Ui, w: f32, groups: &[Group]) -> f32 {
    let head = ui.line_as(TITLE_M) + GRID;
    let label_h = ui.line_as(TITLE_S) + GRID;
    let (rest, band) = band::split(groups);
    let body = if groups.is_empty() {
        ui.line_as(faraday_draw::tokens::BODY_L)
    } else {
        rest.iter()
            .map(|g| label_h + rows_h(ui, w - 2.0 * PAD, &g.rows))
            .sum::<f32>()
    };
    let band = band.map_or(0.0, |g| band::layout(ui, w, g).need());
    2.0 * PAD + head + body + band
}

/// The pane at `(x, y)`, `w` wide and `h` tall: its surface, the heading
/// "This session", the groups, scrolling within where `h` cuts them,
/// `offset` down, and the Waiting band at its foot, `band_offset` down.
/// Returns how far the groups can scroll.
pub(crate) fn draw(
    ui: &mut Ui,
    (x, y, w, h): (f32, f32, f32, f32),
    (offset, band_offset): (f32, f32),
    groups: &[Group],
) -> f32 {
    card_surface(ui, (x, y, w, h));
    let head_h = ui.line_as(TITLE_M) + GRID;
    ui.text_as(
        x + PAD + faraday_draw::tokens::LIST_PAD,
        y + PAD,
        TITLE_M,
        Role::OnSurface,
        EN.home_box_heading,
    );
    let top = y + PAD + head_h;
    let (rest, band) = band::split(groups);
    let band = band.map(|g| (g, band::layout(ui, w, g)));
    let shown = band.as_ref().map_or(0.0, |(_, lay)| {
        band::shown_h(ui, lay.need(), h).min(lay.need())
    });
    let view = if band.is_some() {
        (h - (top - y) - shown).max(0.0)
    } else {
        (h - (top - y) - PAD / 2.0).max(0.0)
    };
    let clip = ui.rect(x, top, w, view);
    let label_h = ui.line_as(TITLE_S) + GRID;
    let total: f32 = rest
        .iter()
        .map(|g| label_h + rows_h(ui, w - 2.0 * PAD, &g.rows))
        .sum();
    let mark = ui.presses_mark();
    let open = ui.region_open(Region::scrolled(Slot::HomeBox, clip, offset));
    if groups.is_empty() {
        ui.text_as(
            x + PAD + faraday_draw::tokens::LIST_PAD,
            top,
            faraday_draw::tokens::BODY_L,
            Role::OnSurfaceVariant,
            EN.session_nothing_loaded,
        );
    }
    let mut cy = top;
    for g in rest {
        section_label(ui, x + PAD + faraday_draw::tokens::LIST_PAD, cy, g.label);
        cy += ui.line_as(TITLE_S) + GRID;
        list_rows(ui, (x + PAD, cy), w - 2.0 * PAD, &g.rows);
        cy += rows_h(ui, w - 2.0 * PAD, &g.rows);
    }
    ui.clip_presses_since(mark, clip);
    let max = (total - view).max(0.0);
    ui.region_close(open, max);
    if let Some((g, lay)) = &band {
        band::draw(ui, (x, y + h - shown, w, shown), band_offset, g, lay);
    }
    max
}

/// Vaults' groups: Keys, then Wallets, then Waiting at the foot (a waiting
/// PSBT with Sign, the flows in progress); the vault files are the tile's.
fn vaults_groups(app: &View) -> Vec<Group> {
    let mut waiting = psbt_rows(app);
    // The file's page is no page of Vaults: the row has Sign and no open.
    waiting.iter_mut().for_each(|r| r.open = None);
    let signing = waiting_spend(&waiting);
    waiting.extend(progress_rows(app, |g| {
        signing && g == crate::in_progress::Going::Spend
    }));
    let mut wallets: Vec<SessionRow> = wallet_rows(app)
        .into_iter()
        .filter(|r| r.action != Action::Nav(Screen::Backups))
        .collect();
    let mut keys = key_rows(app);
    super::vault_marks::mark(app, &mut keys, &mut wallets);
    with_band(
        [
            (EN.session_group_keys, keys),
            (EN.session_group_wallets, wallets),
        ],
        waiting,
    )
}
