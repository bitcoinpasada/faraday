//! `Screen::Home`.

use crate::drawing::View;
use crate::in_progress::Going;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Glyph, Ui, W};
use crate::{Action, Screen};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{section_label, title};

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
        .chain(std::iter::once((Some(Going::Create), make_a_new_wallet())))
        .find(|(g, _)| !g.is_some_and(|g| shown.iter().any(|r| r.going == g)))
        .map(|(_, lead)| lead)
}

/// A lead: its icon, its line, the line under it, its press and whether
/// it can be pressed now.
pub(crate) type Lead = (Glyph, String, String, Action, bool);

/// Something under way or waiting (§1.2's rules 1 to 6), which leads
/// Home above its four ways in; none when nothing is.
fn home_job(app: &View) -> Option<Lead> {
    home_jobs(app).into_iter().next().map(|(_, lead)| lead)
}

/// Every lead §1.2's rules 1 to 6 give, in their order, each with the
/// flow it opens or starts over, when it is one that can be in progress.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn home_jobs(app: &View) -> Vec<(Option<Going>, Lead)> {
    use crate::vault::VaultAction as V;
    let mut v = Vec::new();
    // Rule 1: a spend under way; of a wallet loaded, as its card says it
    // (`docs/history/NEW-WALLET.md` §11.2).
    if let Some(s) = app.wallets.spend()
        && let Some(w) = s.wallet.and_then(|w| app.session.wallets.get(w))
    {
        v.push((
            Some(Going::Spend),
            (
                Glyph::Icon(Icon::Sign),
                EN.home_job_carry_on_spend.to_string(),
                fill(
                    EN.home_job_signatures,
                    &[
                        &w.name.to_string(),
                        &s.signers.len().to_string(),
                        &app.spend_needed().to_string(),
                    ],
                ),
                app.carry_on(),
                true,
            ),
        ));
    } else if let Some(s) = app.wallets.spend() {
        v.push((
            Some(Going::Spend),
            (
                Glyph::Icon(Icon::Sign),
                fill(EN.home_job_continue_signing, &[&s.spend.source.to_string()]),
                fill(
                    EN.home_job_of_signatures,
                    &[
                        &s.signers.len().to_string(),
                        &app.spend_needed().to_string(),
                    ],
                ),
                Action::Nav(Screen::Spend),
                true,
            ),
        ));
    }
    // A backup not finished: its key is made and some place is not written
    // (`docs/BACKUP-FLOW.md` §5).
    if let Some((i, f)) = app.backup_pending() {
        v.push((
            Some(app.flow_going(i)),
            (
                Glyph::Icon(Icon::Lock),
                fill(
                    crate::strings::EN.backup_flow_home_lead,
                    &[&f.subject
                        .as_ref()
                        .map_or_else(|| f.kind.name().to_string(), |s| s.name.clone())],
                ),
                String::new(),
                Action::Flow(crate::backup_flow::steps::FlowAction::Show(i)),
                true,
            ),
        ));
    }
    // Rule 2: a PSBT in Files: its spend takes the place of one under way.
    if let Some(i) = app.lead_psbt() {
        v.push((
            Some(Going::Spend),
            (
                Glyph::Icon(Icon::Sign),
                fill(EN.home_job_sign, &[&app.inbox[i].name.to_string()]),
                EN.home_job_from_files.to_string(),
                Action::StartSpend(i),
                true,
            ),
        ));
    }
    // Rule 3: an import waiting (F60). Put off after Import with a vault
    // it brought still locked, it is that vault's Unlock, on the sheet.
    if let Some(imp) = app.import.as_ref() {
        let lead = if let Some(lv) = app.import_field() {
            (
                Glyph::Icon(Icon::Lock),
                fill(EN.home_job_unlock, &[&lv.title.to_string()]),
                fill(EN.home_job_read_source, &[&imp.label.to_string()]),
                crate::boot_import::OPEN,
                true,
            )
        } else {
            (
                Glyph::Icon(Icon::Download),
                fill(EN.home_job_import_source, &[&imp.label.to_string()]),
                String::new(),
                crate::boot_import::OPEN,
                true,
            )
        };
        v.push((None, lead));
    }
    // Rule 4: a receipt from this power-on's last stick visit, and
    // nothing loaded.
    let fresh = !app.holds_secret() && app.session.wallets.is_empty();
    if fresh && let Some(r) = app.receipt.as_ref() {
        let names: Vec<&str> = r.files.iter().map(|f| f.name.as_str()).collect();
        v.push((
            None,
            (
                Glyph::Icon(Icon::Done),
                fill(EN.home_job_written_target, &[&r.label.to_string()]),
                names.join(", "),
                Action::Nav(Screen::Files),
                true,
            ),
        ));
    }
    // Rule 5: a locked vault file and nothing loaded.
    if fresh {
        let files = app.vault_files();
        if let Some(i) = files.iter().position(|f| f.open.is_none()) {
            let f = &files[i];
            // By its name, with what it held when last seen open (§3.4),
            // when it was.
            let (name, line) = match app.vault_summary(f) {
                Some(s) => (s.name.clone(), s.line(app.tz_min())),
                None => (
                    f.name.clone(),
                    fill(EN.home_job_in_files, &[&f.name.to_string()]),
                ),
            };
            let lead = if app.sticks.is_empty() {
                (
                    Glyph::Icon(Icon::Lock),
                    fill(EN.home_job_unlock, &[&name.to_string()]),
                    line,
                    Action::Vault(V::Open(i)),
                    true,
                )
            } else {
                (
                    Glyph::Icon(Icon::Lock),
                    fill(EN.home_job_unlock, &[&name.to_string()]),
                    fill(EN.home_job_pull_to_unlock, &[app.medium.noun()]),
                    Action::Nav(Screen::Home),
                    false,
                )
            };
            v.push((None, lead));
        }
    }
    // Rule 6: a vault changed since it was written.
    if let Some(name) = app.vault_changed() {
        v.push((
            None,
            (
                Glyph::Icon(Icon::Export),
                fill(
                    EN.home_job_write_target,
                    &[&name.to_string(), app.medium.a()],
                ),
                crate::vault::Currency::Changed.line(),
                Action::Nav(Screen::Files),
                true,
            ),
        ));
    }
    v
}

/// Home's tiles, under the lead when there is one: the four ways in,
/// then, where they apply, Stick visit (a stick attached, nothing held)
/// and Backups (a wallet known). With no vault in Files or open, Vaults
/// is Create a vault: the list would have nothing else on it.
pub(crate) fn home_ways(app: &View) -> Vec<(Glyph, &'static str, String, Action)> {
    use crate::vault::VaultAction as V;
    let vault_files = app.vault_files().len();
    let open = app.vaults.open.len();
    let mut t = vec![
        (
            Glyph::Icon(Icon::Flag),
            EN.home_ways_make_a_new,
            EN.home_ways_single_key.to_string(),
            Action::CreateWallet,
        ),
        (
            Glyph::Icon(Icon::Wallet),
            EN.home_ways_restore_wallet,
            EN.home_ways_restore_line.to_string(),
            Action::RestoreWallet,
        ),
        (
            Glyph::Icon(Icon::Sign),
            EN.home_ways_spend_backup,
            EN.home_ways_a_stick_words.to_string(),
            Action::Nav(Screen::Family),
        ),
        if open == 0 && vault_files == 0 {
            (
                Glyph::Icon(Icon::Lock),
                EN.home_ways_create_vault,
                EN.home_ways_none_yet.to_string(),
                Action::Vault(V::Create),
            )
        } else {
            (
                Glyph::Icon(Icon::Lock),
                EN.home_ways_vaults,
                if open == 0 {
                    fill(EN.home_ways_locked, &[&vault_files.to_string()])
                } else {
                    fill(EN.home_ways_open, &[&open.to_string()])
                },
                Action::Nav(Screen::Vaults),
            )
        },
    ];
    if !app.sticks.is_empty() && !app.holds_secret() {
        t.push((
            app.medium.icon(),
            app.medium.visit(),
            EN.home_ways_a_stick_attached.to_string(),
            Action::Nav(Screen::Visit),
        ));
    }
    let known = app.backups();
    if !known.is_empty() {
        let without = known.iter().filter(|e| e.lines.is_none()).count();
        t.push((
            Glyph::Icon(Icon::Shield),
            EN.home_ways_backups,
            if without == 0 {
                fill(EN.home_ways_with_a_plan, &[&known.len().to_string()])
            } else {
                fill(EN.home_ways_not_backed, &[&without.to_string()])
            },
            Action::Nav(Screen::Backups),
        ));
    }
    t
}

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let x = x0 + 56.0;
    let width = cw - 112.0;
    let mut y = 44.0;
    title(ui, x, y, EN.home_home);
    y += 64.0;
    // The system gave no randomness: nothing seals until a restart.
    if app.seed_missing() {
        ui.icon(x - 4.0, y - 6.0, 28.0, Icon::Error, 14.0, ERR);
        let line = ui.fit(14.0, W::S, EN.seed_missing, width - 28.0);
        ui.text(x + 26.0, y, 14.0, W::S, ERR, &line);
        y += 36.0;
    }
    section_label(ui, x, y, EN.home_start);
    y += 30.0;

    let gap = 16.0;
    // The lead, full width: something under way or waiting (§1.2's
    // rules 1 to 6). With none, the four ways in are all of Start.
    if let Some((icon, label, sub, action, enabled)) = home_job(app) {
        let lead_h = 96.0;
        let pressed = enabled && ui.is_pressed(action);
        ui.fill(
            x,
            y,
            width,
            lead_h,
            14.0,
            if pressed { INNER } else { SURFACE },
        );
        ui.stroke(
            x,
            y,
            width,
            lead_h,
            14.0,
            if enabled {
                BORDER
            } else {
                WARN.with_alpha(110)
            },
        );
        let (ibg, ifg) = if enabled {
            (ACCENT.with_alpha(26), ACCENT)
        } else {
            (INNER, DIM)
        };
        ui.fill(x + 20.0, y + 24.0, 48.0, 48.0, 12.0, ibg);
        ui.icon(x + 20.0, y + 24.0, 48.0, icon, 20.0, ifg);
        let label = ui.fit(19.0, W::S, &label, width - 184.0);
        ui.text(x + 86.0, y + 22.0, 19.0, W::S, TEXT, &label);
        if !sub.is_empty() {
            let sub = ui.fit(14.0, W::R, &sub, width - 184.0);
            ui.text(
                x + 86.0,
                y + 54.0,
                14.0,
                W::R,
                if enabled { MUTED } else { WARN },
                &sub,
            );
        }
        if enabled {
            ui.hit(x, y, width, lead_h, action);
        }
        y += lead_h + gap;
    }

    // The tiles, half width each, two to a row.
    let tilew = (width - gap) / 2.0;
    let tile_h = 84.0;
    for (i, (icon, label, sub, action)) in home_ways(app).iter().enumerate() {
        let tx = x + (i % 2) as f32 * (tilew + gap);
        let ty = y + (i / 2) as f32 * (tile_h + gap);
        let pressed = ui.is_pressed(*action);
        ui.fill(
            tx,
            ty,
            tilew,
            tile_h,
            12.0,
            if pressed { INNER } else { SURFACE },
        );
        ui.stroke(tx, ty, tilew, tile_h, 12.0, BORDER);
        ui.fill(
            tx + 18.0,
            ty + 22.0,
            40.0,
            40.0,
            10.0,
            ACCENT.with_alpha(26),
        );
        ui.icon(tx + 18.0, ty + 22.0, 40.0, *icon, 17.0, ACCENT);
        let label = ui.fit(15.0, W::S, label, tilew - 96.0);
        ui.text(tx + 74.0, ty + 20.0, 15.0, W::S, TEXT, &label);
        let sub = ui.fit(13.0, W::R, sub, tilew - 96.0);
        ui.text(tx + 74.0, ty + 46.0, 13.0, W::R, MUTED, &sub);
        ui.hit(tx, ty, tilew, tile_h, *action);
    }

    // Scan floats at the foot, on the right.
    ui.fab(x + width, h - 32.0, Icon::Scan, EN.home_scan, Action::Scan);
}
