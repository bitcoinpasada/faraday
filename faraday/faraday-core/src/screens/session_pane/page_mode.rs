//! The session pane in a menu page's own mode (`docs/WORKSPACE.md` §2): one
//! list in the pane's width, each group's small heading then its rows, the
//! groups the page's tools take in the order they take them. Open vaults,
//! files to write and Backups are not in it. The rows are the pane's
//! (`rows`), so a row reads the same here as on Home.

use super::rows::{Group, key_rows, progress_rows, row, wallet_rows};
use super::{SessionRow, card_surface, list_rows, rows_h};
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
            r
        })
        .collect()
}

/// The files held that a tool takes, a PSBT or a transaction first, then a
/// wallet or setup file, then the rest; a vault file is a vault's, not
/// listed. A PSBT is in `psbt_rows`, not here.
fn file_rows(app: &View) -> Vec<SessionRow> {
    let mut held: Vec<(u8, &crate::Item)> = app
        .inbox
        .iter()
        .filter(|it| it.facts.is_none())
        .filter(|it| it.psbt().is_none())
        .map(|it| {
            let rank = if it.unsigned_txid.is_some() {
                0
            } else if it.wallet().is_some() {
                1
            } else {
                2
            };
            (rank, it)
        })
        .collect();
    held.sort_by_key(|(rank, _)| *rank);
    held.into_iter()
        .map(|(_, it)| {
            row(
                Icon::File,
                it.name.to_string(),
                String::new(),
                Action::Nav(Screen::Files),
            )
        })
        .collect()
}

/// The pane's groups for `mode`, in the page's order (WORKSPACE.md §2's
/// table); a group with no rows is left out.
pub(crate) fn groups(app: &View, mode: Mode) -> Vec<Group> {
    let psbts = psbt_rows(app);
    let waiting = !psbts.is_empty();
    let mut files = psbts;
    files.extend(file_rows(app));
    let wallets: Vec<SessionRow> = wallet_rows(app)
        .into_iter()
        .filter(|r| r.action != Action::Nav(Screen::Backups))
        .collect();
    let shown = |g| waiting && g == crate::in_progress::Going::Spend;
    let order = match (mode, waiting) {
        (Mode::Tools, true) => [Slotted::Files, Slotted::Keys, Slotted::Wallets],
        (Mode::Tools, false) => [Slotted::Keys, Slotted::Files, Slotted::Wallets],
        (Mode::Check, _) => [Slotted::Files, Slotted::Wallets, Slotted::Keys],
    };
    let mut files = Some(files);
    let mut wallets = Some(wallets);
    let mut out: Vec<(&'static str, Vec<SessionRow>)> = order
        .into_iter()
        .filter_map(|s| match s {
            Slotted::Files => files.take().map(|r| (EN.session_group_files, r)),
            Slotted::Wallets => wallets.take().map(|r| (EN.session_group_wallets, r)),
            Slotted::Keys => Some((EN.session_group_keys, key_rows(app))),
        })
        .collect();
    out.push((EN.session_group_progress, progress_rows(app, shown)));
    out.into_iter()
        .filter(|(_, rows)| !rows.is_empty())
        .map(|(label, rows)| Group { label, rows })
        .collect()
}

/// A group's place in a page's order.
#[derive(Clone, Copy)]
enum Slotted {
    Files,
    Wallets,
    Keys,
}

/// The height of the pane's content: the heading, then each group's
/// heading and rows, one under another, `w` wide with its padding.
pub(crate) fn content_h(ui: &Ui, w: f32, groups: &[Group]) -> f32 {
    let head = ui.line_as(TITLE_M) + GRID;
    let label_h = ui.line_as(TITLE_S) + GRID;
    let body = if groups.is_empty() {
        ui.line_as(faraday_draw::tokens::BODY_L)
    } else {
        groups
            .iter()
            .map(|g| label_h + rows_h(ui, w - 2.0 * PAD, &g.rows))
            .sum::<f32>()
    };
    2.0 * PAD + head + body
}

/// The pane at `(x, y)`, `w` wide and `h` tall: its surface, the heading
/// "This session", the groups, scrolling within where `h` cuts them,
/// `offset` down. Returns how far it can scroll.
pub(crate) fn draw(
    ui: &mut Ui,
    (x, y, w, h): (f32, f32, f32, f32),
    offset: f32,
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
    let view = (h - (top - y) - PAD / 2.0).max(0.0);
    let clip = ui.rect(x, top, w, view);
    let total = content_h(ui, w, groups) - 2.0 * PAD - head_h;
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
    for g in groups {
        section_label(ui, x + PAD + faraday_draw::tokens::LIST_PAD, cy, g.label);
        cy += ui.line_as(TITLE_S) + GRID;
        list_rows(ui, (x + PAD, cy), w - 2.0 * PAD, &g.rows);
        cy += rows_h(ui, w - 2.0 * PAD, &g.rows);
    }
    ui.clip_presses_since(mark, clip);
    let max = (total - view).max(0.0);
    ui.region_close(open, max);
    max
}
