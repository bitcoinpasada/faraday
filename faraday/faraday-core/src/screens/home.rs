//! `Screen::Home`.

use crate::drawing::View;
use crate::in_progress::Going;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Glyph, Ui};
use crate::ui::{Shape, Style};
use crate::{Action, Screen};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, SHAPE_M, TITLE_M};
use faraday_draw::tokens::{
    HOME_LEAD_H, HOME_LEAD_ICON, HOME_LEAD_INSET, HOME_LEAD_SUB_Y, HOME_LEAD_TEXT_X,
    HOME_TILE_ICON, HOME_TILE_INSET, HOME_TILE_TEXT_X, LEAD_MAX, MARGIN_WIDE, TILE_H, TILE_MIN,
};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::section_label;

/// Home's lead when nothing is under way, §1.2's rule 7: Make a new
/// wallet.
fn make_a_new_wallet() -> Lead {
    (
        Glyph::Icon(Icon::Wallet),
        EN.home_lead_make_a_new.to_string(),
        EN.home_lead_single_key.to_string(),
        Action::CreateWallet,
        true,
    )
}

/// The small Home's lead job: the first rule of §1.2's ranking that
/// applies, rule 7 (Make a new wallet) when none of the others does,
/// skipping a lead that would open or start over a flow in progress,
/// whose tile is above it (`docs/FLOWS.md`, Flows in progress); none
/// when every one would. `enabled` is false for a prompt with no target
/// of its own (pull the stick first).
pub(crate) fn home_lead_beside_tiles(app: &View) -> Option<Lead> {
    let shown = app.flows_in_progress();
    home_jobs(app)
        .into_iter()
        .map(|j| (j.going, j.pi))
        .chain(std::iter::once((Some(Going::Create), make_a_new_wallet())))
        .find(|(g, _)| !g.is_some_and(|g| shown.iter().any(|r| r.going == g)))
        .map(|(_, lead)| lead)
}

/// A lead: its icon, its line, the line under it, its press and whether
/// it can be pressed now.
pub(crate) type Lead = (Glyph, String, String, Action, bool);

/// The lead as the card on the wide Home draws it (`docs/HOME.md` §2):
/// its icon, its line, the line under it, the one filled button's label
/// and press, and whether the button can be pressed now.
#[derive(Debug, Clone)]
pub(crate) struct LeadCard {
    /// The job's icon.
    pub icon: Glyph,
    /// The card's line.
    pub line: String,
    /// The line under it, empty for none.
    pub under: String,
    /// The button's label.
    pub button: &'static str,
    /// What the button does.
    pub action: Action,
    /// False for a job that cannot be done here yet; its second line
    /// then says why.
    pub enabled: bool,
}

/// One job §2 ranks: the flow it opens or starts over when it is one that
/// can be in progress, the lead as the one-column Home words it, and the
/// card as the wide Home does.
struct Job {
    going: Option<Going>,
    pi: Lead,
    card: LeadCard,
}

/// A job: `pi` as the one-column Home words it, and the card with
/// `button`, `line` and `under`.
fn job(going: Option<Going>, pi: Lead, button: &'static str, line: String, under: String) -> Job {
    let card = LeadCard {
        icon: pi.0,
        line,
        under,
        button,
        action: pi.3,
        enabled: pi.4,
    };
    Job { going, pi, card }
}

/// Every lead §2's table gives before the key with no wallet, in its
/// order.
fn home_jobs(app: &View) -> Vec<Job> {
    use crate::vault::VaultAction as V;
    let mut v = Vec::new();
    // A spend under way; of a wallet loaded, as its card says it.
    if let Some(s) = app.wallets.spend()
        && let Some(w) = s.wallet.and_then(|w| app.session.wallets.get(w))
    {
        let sigs = fill(
            EN.home_job_signatures,
            &[
                &w.name,
                &s.signers.len().to_string(),
                &app.spend_needed().to_string(),
            ],
        );
        v.push(job(
            Some(Going::Spend),
            (
                Glyph::Icon(Icon::Sign),
                EN.home_job_carry_on_spend.to_string(),
                sigs.clone(),
                app.carry_on(),
                true,
            ),
            EN.home_btn_carry_on,
            sigs,
            String::new(),
        ));
    } else if let Some(s) = app.wallets.spend() {
        let under = fill(
            EN.home_job_of_signatures,
            &[
                &s.signers.len().to_string(),
                &app.spend_needed().to_string(),
            ],
        );
        v.push(job(
            Some(Going::Spend),
            (
                Glyph::Icon(Icon::Sign),
                fill(EN.home_job_continue_signing, &[&s.spend.source]),
                under.clone(),
                Action::Nav(Screen::Spend),
                true,
            ),
            EN.home_btn_carry_on,
            s.spend.source.to_string(),
            under,
        ));
    }
    // A backup not finished: its key is made and some place is not written
    // (`docs/BACKUP-FLOW.md` §5).
    if let Some((i, f)) = app.backup_pending() {
        let name = f
            .subject
            .as_ref()
            .map_or_else(|| f.kind.name().to_string(), |s| String::from(&*s.name));
        v.push(job(
            Some(app.flow_going(i)),
            (
                Glyph::Icon(Icon::Lock),
                fill(crate::strings::EN.backup_flow_home_lead, &[&name]),
                String::new(),
                Action::Flow(crate::backup_flow::steps::FlowAction::Show(i)),
                true,
            ),
            EN.home_btn_finish,
            name,
            EN.home_card_backup_not_finished.to_string(),
        ));
    }
    // A PSBT in Files: its spend takes the place of one under way.
    if let Some(i) = app.lead_psbt() {
        v.push(job(
            Some(Going::Spend),
            (
                Glyph::Icon(Icon::Sign),
                fill(EN.home_job_sign, &[&app.inbox[i].name]),
                EN.home_job_from_files.to_string(),
                Action::StartSpend(i),
                true,
            ),
            EN.home_btn_sign,
            app.inbox[i].name.to_string(),
            EN.home_job_from_files.to_string(),
        ));
    }
    // An import waiting (F60). Put off after Import with a vault it
    // brought still locked, it is that vault's Unlock, on the sheet.
    if let Some(imp) = app.import.state() {
        let j = if let Some(lv) = app.import_field() {
            let under = fill(EN.home_job_read_source, &[&imp.label]);
            job(
                None,
                (
                    Glyph::Icon(Icon::Lock),
                    fill(EN.home_job_unlock, &[&lv.title]),
                    under.clone(),
                    crate::boot_import::OPEN,
                    true,
                ),
                EN.home_btn_unlock,
                lv.title.to_string(),
                under,
            )
        } else {
            let line = fill(EN.home_job_import_source, &[&imp.label]);
            job(
                None,
                (
                    Glyph::Icon(Icon::Download),
                    line.clone(),
                    String::new(),
                    crate::boot_import::OPEN,
                    true,
                ),
                EN.home_btn_import,
                line,
                String::new(),
            )
        };
        v.push(j);
    }
    // A receipt from this power-on's last stick visit, and nothing
    // loaded.
    let fresh = !app.holds_secret() && app.session.wallets.is_empty();
    if fresh && let Some(r) = app.visit.receipt().as_ref() {
        let names: Vec<&str> = r.files.iter().map(|f| f.name.as_str()).collect();
        let line = fill(EN.home_job_written_target, &[&r.label.to_string()]);
        let under = names.join(", ");
        v.push(job(
            None,
            (
                Glyph::Icon(Icon::Done),
                line.clone(),
                under.clone(),
                Action::Nav(Screen::Files),
                true,
            ),
            EN.home_btn_open,
            line,
            under,
        ));
    }
    // A locked vault file and nothing loaded.
    if fresh {
        let files = app.vault_files();
        if let Some(i) = files.iter().position(|f| f.open.is_none()) {
            let f = &files[i];
            // By its name, with what it held when last seen open (§3.4),
            // when it was.
            let (name, line) = match app.vault_summary(f) {
                Some(s) => (
                    String::from(s.name.as_str()),
                    String::from(s.line(app.tz_min()).as_str()),
                ),
                None => (
                    String::from(f.name.as_str()),
                    fill(EN.home_job_in_files, &[f.name.as_str()]),
                ),
            };
            let pi_line = fill(EN.home_job_unlock, &[&name.to_string()]);
            let j = if app.stick.is_empty() {
                job(
                    None,
                    (
                        Glyph::Icon(Icon::Lock),
                        pi_line,
                        line.clone(),
                        Action::Vault(V::Open(i)),
                        true,
                    ),
                    EN.home_btn_unlock,
                    name,
                    line,
                )
            } else {
                let under = fill(EN.home_job_pull_to_unlock, &[app.medium.noun()]);
                job(
                    None,
                    (
                        Glyph::Icon(Icon::Lock),
                        pi_line,
                        under.clone(),
                        Action::Nav(Screen::Home),
                        false,
                    ),
                    EN.home_btn_unlock,
                    name,
                    under,
                )
            };
            v.push(j);
        }
    }
    // A vault changed since it was written.
    if let Some(name) = app.vault_changed() {
        let under = String::from(crate::vault::Currency::Changed.line().as_str());
        v.push(job(
            None,
            (
                Glyph::Icon(Icon::Export),
                fill(EN.home_job_write_target, &[name.as_str(), app.medium.a()]),
                under.clone(),
                Action::Nav(Screen::Files),
                true,
            ),
            EN.home_btn_write,
            name.to_string(),
            under,
        ));
    }
    // A stick waiting unvisited: Files on it (F235).
    if app.stick.waits()
        && !app.holds_secret()
        && let Some(st) = app.stick.get(app.visit.stick)
    {
        let line = fill(EN.home_job_files_on, &[&st.label]);
        v.push(job(
            None,
            (
                app.medium.icon(),
                line.clone(),
                EN.home_ways_a_stick_attached.to_string(),
                Action::Nav(Screen::Files),
                true,
            ),
            EN.home_btn_open,
            line,
            EN.home_ways_a_stick_attached.to_string(),
        ));
    }
    v
}

/// Whether nothing is loaded: no key held, no wallet in the session and
/// no vault open. Files brought in, a stick attached and a vault file not
/// yet unlocked do not count (`docs/HOME.md` §1).
pub(crate) fn nothing_loaded(app: &View) -> bool {
    !app.holds_secret()
        && app.session.keys.all().next().is_none()
        && app.session.wallets.is_empty()
        && app.vaults.open.is_empty()
}

/// The lead of the wide Home (`docs/HOME.md` §2): the first job of the
/// table that applies, a key no wallet uses making its wallet, and Make a
/// new wallet only while nothing is loaded; none when something is loaded
/// and nothing waits. `skip_shown` drops a job that would open or start
/// over a flow whose line is above the card (`docs/FLOWS.md`, Flows in
/// progress), as the one-column Home has it.
pub(crate) fn home_lead(app: &View, skip_shown: bool) -> Option<LeadCard> {
    let shown = if skip_shown {
        app.flows_in_progress()
    } else {
        Default::default()
    };
    let hidden = |g: Option<Going>| g.is_some_and(|g| shown.iter().any(|r| r.going == g));
    if let Some(j) = home_jobs(app).into_iter().find(|j| !hidden(j.going)) {
        return Some(j.card);
    }
    let (loose, picked) = app.loose_keys();
    if let Some(fp) = picked.and_then(|p| loose.iter().find(|f| f.0 == p).copied()) {
        return Some(LeadCard {
            icon: Glyph::Icon(Icon::Wallet),
            line: fill(EN.home_card_key, &[&crate::wallet::fp_text(fp)]),
            under: EN.home_card_no_wallet_uses_it.to_string(),
            button: EN.home_btn_make_a_wallet,
            action: Action::KeyWallet(fp.0, 1),
            enabled: true,
        });
    }
    nothing_loaded(app).then(|| LeadCard {
        icon: Glyph::Icon(Icon::Wallet),
        line: EN.home_lead_make_a_new.to_string(),
        under: EN.home_lead_single_key.to_string(),
        button: EN.home_btn_make,
        action: Action::CreateWallet,
        enabled: true,
    })
}

/// A wallet's card on Home (`docs/HOME.md` §3): its name, the one line
/// that matters now and what its Sign button does.
#[derive(Debug, Clone)]
pub(crate) struct WalletCard {
    /// The wallet's place in the session.
    pub index: usize,
    /// Its name.
    pub name: String,
    /// The first of: its spend's signatures, "A PSBT to sign", "Not
    /// backed up", its shape.
    pub line: String,
    /// What Sign does: the wallet card's first button.
    pub sign: Action,
}

/// The wallets in the session, in the sidebar's order (the ones the keys
/// here sign for alone first, then those with some keys here, then
/// watch-only), each as its Home card.
pub(crate) fn home_wallets(app: &View) -> Vec<WalletCard> {
    let mut ranked: Vec<(usize, u8)> = (0..app.session.wallets.len())
        .map(|i| (i, app.readiness(i)))
        .collect();
    ranked.sort_by_key(|&(i, r)| (std::cmp::Reverse(r), i));
    ranked
        .into_iter()
        .map(|(i, _)| wallet_card(app, i))
        .collect()
}

/// Wallet `i`'s card.
fn wallet_card(app: &View, i: usize) -> WalletCard {
    let w = &app.session.wallets[i];
    let held = app.receive_held(w);
    let spend = app.wallets.spend().filter(|s| s.wallet == Some(i));
    let line = if let Some(s) = spend {
        fill(
            EN.home_card_signatures,
            &[
                &s.signers.len().to_string(),
                &app.spend_needed().to_string(),
            ],
        )
    } else if app.psbt_for(i).is_some() {
        EN.home_card_a_psbt_to_sign.to_string()
    } else if held {
        EN.home_card_not_backed_up.to_string()
    } else {
        // As the sidebar's row gives its shape. A key not of the wallet
        // spends it alone: no "k of n" (F132).
        let (m, n) = crate::wallet::Session::quorum(w);
        if crate::wallet::another_key_spends(w) {
            EN.sidebar_another_key.to_string()
        } else if n > 1 || w.policy().keys().len() > 1 {
            fill(EN.sidebar_count, &[&m.to_string(), &n.to_string()])
        } else {
            EN.sidebar_single_key.to_string()
        }
    };
    WalletCard {
        index: i,
        name: w.name.to_string(),
        line,
        sign: app.spend_button(i).1,
    }
}

/// Home's tiles, under the lead and the wallets: Start's tiles while
/// nothing is loaded (Make a new wallet among them unless it is the
/// lead, `make_leads`), then Backups once a wallet is known. Check is no
/// tile here: the sidebar lists it (F275).
pub(crate) fn home_ways(
    app: &View,
    make_leads: bool,
) -> Vec<(Glyph, &'static str, String, Action)> {
    let mut t = Vec::new();
    if nothing_loaded(app) {
        if !make_leads {
            t.push((
                Glyph::Icon(Icon::Flag),
                EN.home_ways_make_a_new,
                EN.home_ways_single_key.to_string(),
                Action::CreateWallet,
            ));
        }
        t.push((
            Glyph::Icon(Icon::Wallet),
            EN.home_ways_restore_wallet,
            EN.home_ways_restore_line.to_string(),
            Action::RestoreWallet,
        ));
        t.push((
            Glyph::Icon(Icon::Sign),
            EN.home_ways_spend_backup,
            EN.home_ways_a_stick_words.to_string(),
            Action::Nav(Screen::SpendBackup),
        ));
    }
    let (known, without) = app.backup_counts();
    if known > 0 {
        t.push((
            Glyph::Icon(Icon::Shield),
            EN.home_ways_backups,
            if without == 0 {
                fill(EN.home_ways_with_a_plan, &[&known.to_string()])
            } else {
                fill(EN.home_ways_not_backed, &[&without.to_string()])
            },
            Action::Nav(Screen::Backups),
        ));
    }
    t
}

/// Home's tile grid in a row `width` wide with `gap` between tiles:
/// how many columns fit at `min` or `TILE_MIN`, whichever is wider (at
/// least one), and the width each
/// tile takes, which is the row's share, never under `TILE_MIN` unless
/// the row itself is narrower. A lookup.
pub(crate) fn tile_grid(width: f32, gap: f32, min: f32) -> (usize, f32) {
    let min = min.max(TILE_MIN);
    let cols = (((width + gap) / (min + gap)).floor() as usize).max(1);
    (cols, (width - gap * (cols - 1) as f32) / cols as f32)
}

/// The tile grid with its top left at `at`, `width` wide.
fn tiles(
    ways: &[(Glyph, &'static str, String, Action)],
    ui: &mut Ui,
    (x, y): (f32, f32),
    width: f32,
    gap: f32,
) -> f32 {
    // The tiles in a grid: as many columns as `TILE_MIN` leaves room
    // for, a row with fewer tiles than columns leaving the rest empty.
    // A tile is wide enough for its longest line, so no name is cut.
    let need = ways
        .iter()
        .map(|(_, label, sub, _)| {
            ui.measure_as(TITLE_M, label)
                .max(ui.measure_as(BODY_S, sub))
        })
        .fold(0.0, f32::max)
        + HOME_TILE_TEXT_X
        + 2.0 * HOME_TILE_INSET;
    let (cols, tilew) = tile_grid(width, gap, need);
    let tile_h = TILE_H;
    for (i, (icon, label, sub, action)) in ways.iter().enumerate() {
        let tx = x + (i % cols) as f32 * (tilew + gap);
        let ty = y + (i / cols) as f32 * (tile_h + gap);
        ui.fill(tx, ty, tilew, tile_h, SHAPE_M, Role::SurfaceContainerLow);
        ui.stroke(tx, ty, tilew, tile_h, SHAPE_M, Role::OutlineVariant);
        ui.press(
            Shape::new(tx, ty, tilew, tile_h, SHAPE_M),
            Role::OnSurface.paint(),
            *action,
        );
        let (ix, ib) = (HOME_TILE_INSET, HOME_TILE_ICON);
        let iy = (tile_h - ib) / 2.0;
        ui.fill(
            tx + ix,
            ty + iy,
            ib,
            ib,
            faraday_draw::tokens::SHAPE_M,
            Role::SecondaryContainer,
        );
        ui.icon(
            tx + ix,
            ty + iy,
            ib,
            *icon,
            17.0,
            Role::OnSecondaryContainer,
        );
        let room = tilew - HOME_TILE_TEXT_X - ix - ix;
        let label = ui.fit_as(TITLE_M, label, room);
        ui.text_as(
            tx + HOME_TILE_TEXT_X,
            ty + 20.0,
            TITLE_M,
            Role::OnSurface,
            &label,
        );
        let sub = ui.fit_as(BODY_S, sub, room);
        ui.text_as(
            tx + HOME_TILE_TEXT_X,
            ty + 46.0,
            BODY_S,
            Role::OnSurfaceVariant,
            &sub,
        );
    }
    ways.len().div_ceil(cols) as f32 * (tile_h + gap)
}

/// The lead's card at `(x, y)`, at most `LEAD_MAX` wide inside `width`:
/// the job's icon, its line and the line under it, and at its end the one
/// filled button (`docs/HOME.md` §2). Returns its height. The card is not
/// a press; its button is.
fn lead_card(ui: &mut Ui, card: &LeadCard, (x, y): (f32, f32), width: f32) -> f32 {
    let lead_h = HOME_LEAD_H;
    let lead_w = width.min(LEAD_MAX);
    ui.fill(x, y, lead_w, lead_h, SHAPE_M, Role::SurfaceContainerLow);
    ui.stroke(x, y, lead_w, lead_h, SHAPE_M, Role::OutlineVariant);
    let (ibg, ifg) = if card.enabled {
        (
            Role::SecondaryContainer.paint(),
            Role::OnSecondaryContainer.paint(),
        )
    } else {
        (
            Role::SurfaceContainerHighest.paint(),
            Role::OnSurfaceVariant.paint(),
        )
    };
    ui.fill(
        x + HOME_LEAD_INSET,
        y + (lead_h - HOME_LEAD_ICON) / 2.0,
        HOME_LEAD_ICON,
        HOME_LEAD_ICON,
        SHAPE_M,
        ibg,
    );
    ui.icon(
        x + HOME_LEAD_INSET,
        y + (lead_h - HOME_LEAD_ICON) / 2.0,
        HOME_LEAD_ICON,
        card.icon,
        HOME_LEAD_INSET,
        ifg,
    );
    let style = if card.enabled {
        Style::Primary
    } else {
        Style::PrimaryOff
    };
    let bw = ui.button_width(card.button, style);
    let by = y + (lead_h - BUTTON_H) / 2.0;
    ui.button(
        x + lead_w - HOME_LEAD_INSET - bw,
        by,
        Some(bw),
        BUTTON_H,
        card.button,
        style,
        card.action,
    );
    let room = lead_w - HOME_LEAD_TEXT_X - bw - 2.0 * HOME_LEAD_INSET;
    let line = ui.fit_as(TITLE_M, &card.line, room);
    ui.text_as(
        x + HOME_LEAD_TEXT_X,
        y + HOME_LEAD_INSET,
        TITLE_M,
        Role::OnSurface,
        &line,
    );
    if !card.under.is_empty() {
        let under = ui.fit_as(BODY_M, &card.under, room);
        let tone = if card.enabled {
            Role::OnSurfaceVariant
        } else {
            Role::Warning
        };
        ui.text_as(
            x + HOME_LEAD_TEXT_X,
            y + HOME_LEAD_SUB_Y,
            BODY_M,
            tone,
            &under,
        );
    }
    lead_h
}

/// The wallets' cards in a grid at `(x, y)`, `width` wide: the name, the
/// line under it, and Sign at the card's end (`docs/HOME.md` §3). The card
/// opens the wallet on Wallets. Returns the grid's height.
fn wallet_cards(
    ui: &mut Ui,
    cards: &[WalletCard],
    (x, y): (f32, f32),
    width: f32,
    gap: f32,
) -> f32 {
    let sign = EN.home_card_sign;
    let bw = ui.button_width(sign, Style::Secondary);
    let need = cards
        .iter()
        .map(|c| {
            ui.measure_as(TITLE_M, &c.name)
                .max(ui.measure_as(BODY_S, &c.line))
        })
        .fold(0.0, f32::max)
        + bw
        + 3.0 * HOME_TILE_INSET;
    let (cols, w) = tile_grid(width, gap, need);
    for (i, c) in cards.iter().enumerate() {
        let cx = x + (i % cols) as f32 * (w + gap);
        let cy = y + (i / cols) as f32 * (TILE_H + gap);
        ui.fill(cx, cy, w, TILE_H, SHAPE_M, Role::SurfaceContainerLow);
        ui.stroke(cx, cy, w, TILE_H, SHAPE_M, Role::OutlineVariant);
        let room = w - bw - 3.0 * HOME_TILE_INSET;
        ui.press(
            Shape::new(cx, cy, w - bw - 2.0 * HOME_TILE_INSET, TILE_H, SHAPE_M),
            Role::OnSurface.paint(),
            Action::OpenWallet(c.index),
        );
        let name = ui.fit_as(TITLE_M, &c.name, room);
        ui.text_as(
            cx + HOME_TILE_INSET,
            cy + 20.0,
            TITLE_M,
            Role::OnSurface,
            &name,
        );
        let line = ui.fit_as(BODY_S, &c.line, room);
        ui.text_as(
            cx + HOME_TILE_INSET,
            cy + 46.0,
            BODY_S,
            Role::OnSurfaceVariant,
            &line,
        );
        ui.button(
            cx + w - HOME_TILE_INSET - bw,
            cy + (TILE_H - BUTTON_H) / 2.0,
            Some(bw),
            BUTTON_H,
            sign,
            Style::Secondary,
            c.sign,
        );
    }
    let rows = cards.len().div_ceil(cols);
    rows as f32 * (TILE_H + gap)
}

/// Home on the wide layout, named by the top app bar (`docs/HOME.md`):
/// the lead, the wallets loaded each with Sign, and the tiles (Start's
/// while nothing is loaded, then Check and Backups), a feed across the
/// work width (`docs/LAYOUT.md` §4, §17): as many columns as fit, from
/// the page's start to `MARGIN_WIDE` from its end.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32) {
    let x = x0 + 56.0;
    let width = cw - (x - x0) - MARGIN_WIDE;
    let mut y = super::bar_head(ui, None, EN.home_home);
    let gap = 16.0;
    // Home's search bar stands under the buttons, centred; what is
    // typed in it shows the matching tiles under it, the buttons staying
    // (the page scrolls as Tools does).
    let h = ui.c.height() as f32 / ui.f;
    let find = !app.tools.home_find().is_empty();
    let open = find.then(|| {
        let clip = ui.rect(x0, 0.0, cw, h);
        ui.region_open(crate::ui::Region::page(clip).from_zero(app.list_offset))
    });
    let top = y;
    let lead = home_lead(app, false);
    let makes = lead
        .as_ref()
        .is_some_and(|l| l.action == Action::CreateWallet);
    if let Some(card) = &lead {
        y += lead_card(ui, card, (x, y), width) + gap;
    }
    let wallets = home_wallets(app);
    if !wallets.is_empty() {
        section_label(ui, x, y, EN.home_wallets);
        y += ui.advance_as(faraday_draw::tokens::TITLE_S, 30.0);
        y += wallet_cards(ui, &wallets, (x, y), width, gap);
    }
    if nothing_loaded(app) {
        section_label(ui, x, y, EN.home_start);
        y += ui.advance_as(faraday_draw::tokens::TITLE_S, 30.0);
    }
    y += tiles(&home_ways(app, makes), ui, (x, y), width, gap);
    let sw = width.min(LEAD_MAX);
    y += super::catalog::home_search_bar(app, ui, (x0 + (cw - sw) / 2.0, y, sw));
    if let Some(open) = open {
        y += super::catalog::home_results(app, ui, (x, y), width, false);
        app.scroll
            .content_h()
            .set(y - top + faraday_draw::tokens::HOME_RESULTS_FOOT);
        ui.region_close(open, app.scroll.content_h().get() - h);
    }
}
