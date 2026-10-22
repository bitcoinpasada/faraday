//! `Screen::Backup`: backing up a wallet.

use super::parts::{
    Cells, buttons_and_next, file_row, keyed_cells, keyed_pills, made_state, never_below,
    never_real_seed, next_button, side_panel_w, ticked, typed_cells, word_keys,
};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{CoreUi, Style, Ui, W};
use crate::wallet::{Session, fp_text, key_line};
use crate::{Action, Code, Faraday, Screen, flow, guide};
use faraday_crypto::work::Compute;
use faraday_draw::widgets::Icon;
use faraday_keys::Reveal;
use faraday_ui::parts::{button_rows, section_label, title, wrap_buttons};

/// The rows of the software question.
pub(crate) const SOFTWARE_ROWS: [&str; 5] = [
    EN.app_sparrow,
    EN.app_coldcard_keystone,
    EN.app_nunchuk,
    EN.app_bitcoin_core,
    EN.app_not_sure,
];

/// A checklist item's title and the line shown while it is closed:
/// drawing's form, through the memo and the kept values.
pub(crate) fn item_title(app: &View, n: u8) -> (String, String) {
    item_title_on(app, n)
}

/// Costs what the backup's answers cost to work out: [`item_title`] for
/// a decision, from the state as it is.
pub(crate) fn item_title_now(c: &Compute, app: &Faraday, n: u8) -> (String, String) {
    item_title_on(&crate::glance::Now { app, c }, n)
}

/// [`item_title`] through `reads`.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn item_title_on(reads: &impl crate::glance::Reads, n: u8) -> (String, String) {
    use crate::plan::Item;
    let app = reads.app();
    let Some(b) = app.backup.as_ref() else {
        return (String::new(), String::new());
    };
    let Some(wallet) = app.session.wallets.get(b.wallet) else {
        return (String::new(), String::new());
    };
    let shape = reads.plan_shape(b.wallet);
    let a = &b.answers;
    let (_, keys_n) = Session::quorum(wallet);
    let item = crate::bstep::item(n).unwrap_or(Item::Envelopes);
    let done = reads.backup_item_done(item);
    match item {
        Item::Templates => {
            let k = crate::plan::templates_on(&shape, &reads.plan_mapped(&shape, a)).max(1);
            (
                fill(
                    EN.item_title_print_blank,
                    &[
                        &k.to_string(),
                        (if k == 1 {
                            EN.item_title_template_inline
                        } else {
                            EN.item_title_templates
                        }),
                    ],
                ),
                fill(EN.item_title_template_n, &[&b.words.to_string()]),
            )
        }
        Item::Copy(i) | Item::Passphrase(i) => (
            fill(
                if matches!(item, Item::Copy(_)) {
                    EN.item_title_copy_seed
                } else {
                    EN.item_title_passphrase
                },
                &[(shape.seeds.get(i).map_or("", |s| s.name.as_str()))],
            ),
            if done {
                EN.item_title_checked
            } else {
                EN.item_title_not_checked
            }
            .to_string(),
        ),
        Item::Vault(v) => {
            let names: Vec<&str> = a
                .vault_seeds(&shape, v)
                .into_iter()
                .map(|i| shape.seeds[i].name.as_str())
                .collect();
            let title = if names.is_empty() {
                fill(EN.item_title_vault_2, &[&(v + 1).to_string()])
            } else {
                fill(
                    EN.item_title_vault,
                    &[&(v + 1).to_string(), &names.join(", ").to_string()],
                )
            };
            let line = match reads.backup_vault_fits(v) {
                Some(name) => fill(EN.item_title_in, &[&name.to_string()]),
                None => EN.item_title_not_saved.to_string(),
            };
            (title, line)
        }
        Item::SeedFiles => (
            EN.item_title_the_seeds.to_string(),
            app.wallet_seeds_written().unwrap_or_else(|| {
                if done {
                    app.medium.for_box()
                } else {
                    EN.item_title_not_made
                }
                .to_string()
            }),
        ),
        Item::Sheets if a.split && shape.splits => (
            EN.item_title_the_shares.to_string(),
            app.wallet_shares_written().unwrap_or_else(|| {
                fill(
                    EN.item_title_shares_keys,
                    &[
                        &keys_n.to_string(),
                        &(keys_n - a.omit.min(shape.m.saturating_sub(1))).to_string(),
                    ],
                )
            }),
        ),
        Item::Sheets => (
            EN.item_title_the_wallet.to_string(),
            app.wallet_item_written(&[format!("{}-backup.pdf", crate::file_stem(&wallet.name))])
                .unwrap_or_else(|| {
                    if done {
                        app.medium.for_box()
                    } else {
                        EN.item_title_a_pdf_to
                    }
                    .to_string()
                }),
        ),
        Item::PublicFiles => (
            EN.item_title_the_public.to_string(),
            app.wallet_item_written(&reads.backup_public_names_for(b.wallet, a))
                .unwrap_or_else(|| ticked(&SOFTWARE_ROWS, &a.software)),
        ),
        Item::Sticks => {
            let (n, m) = app
                .wallet_flow_of_backup()
                .map_or((0, reads.wallet_stick_rows().len()), |i| {
                    app.backup_flows[i].written_of()
                });
            (
                fill(EN.backup_flow_item_sticks, &[app.medium.nouns()]),
                if n == 0 {
                    EN.backup_flow_row_none.to_string()
                } else {
                    fill(EN.backup_flow_written_of, &[&n.to_string(), &m.to_string()])
                },
            )
        }
        Item::ShowDescriptor => (
            EN.item_title_show_the.to_string(),
            if done {
                EN.item_title_shown
            } else {
                EN.item_title_not_shown
            }
            .to_string(),
        ),
        Item::Envelopes => (
            EN.item_title_one_envelope.to_string(),
            format!(
                "{} {}",
                a.places,
                if a.places == 1 {
                    EN.item_title_envelope
                } else {
                    EN.item_title_envelopes
                }
            ),
        ),
    }
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::BStage;
    let panel_w = side_panel_w(cw);
    let col_x = x0 + 40.0;
    let col_w = cw - panel_w - 72.0;
    let Some(b) = app.backup.as_ref() else {
        title(ui, col_x, 28.0, EN.backup_back_up_a_wallet);
        return;
    };
    let Some(wallet) = app.session.wallets.get(b.wallet) else {
        return;
    };
    let keys = app.backup_keys(b.wallet);
    let (m, n) = Session::quorum(wallet);
    let checklist = b.stage == BStage::Checklist;
    let steps = if checklist {
        app.backup_items()
    } else {
        app.backup_questions()
    };
    // The items after one not done wait for it (`docs/history/NEW-WALLET.md`
    // §14.1): each says which.
    let waiting = if checklist {
        app.backup_waiting()
    } else {
        None
    };
    let waiting_title = waiting.map(|n| app.item_title(n).0);
    let ctx = crate::plan::cards::PlanCtx::backup(app);
    // Each item's done mark, and whether it is reachable (every item
    // before it done), in one pass (`Faraday::backup_reachable`, for a
    // press, asks the same of one item).
    let marks: Vec<bool> = if checklist {
        steps
            .iter()
            .map(|&k| crate::bstep::item(k).is_some_and(|it| app.backup_item_done(it)))
            .collect()
    } else {
        Vec::new()
    };
    let mut before_done = true;
    let reachable: Vec<bool> = marks
        .iter()
        .map(|&d| {
            let r = before_done;
            before_done &= d;
            r
        })
        .collect();
    let cards: Vec<flow::Card> = steps
        .iter()
        .enumerate()
        .map(|(i, &k)| {
            if checklist {
                let (title, summary) = app.item_title(k);
                let summary = match (&waiting_title, waiting) {
                    (Some(t), Some(n)) if n != k && !reachable[i] && !marks[i] => {
                        fill(EN.backup_after, &[&t.to_string()])
                    }
                    _ => summary,
                };
                flow::Card {
                    title,
                    summary,
                    mono: false,
                    done: marks[i],
                    open: b.open == Some(k),
                    default: false,
                    toggle: Action::BStep(k),
                    guide: Some(guide::backup(
                        k,
                        m,
                        n,
                        keys.len(),
                        app.medium,
                        if crate::bstep::is_copy(k) {
                            app.copy_forms(b, usize::from(k - crate::bstep::COPY))
                        } else {
                            crate::plan::CopyForms::default()
                        },
                    ))
                    .filter(|g| !g.is_empty()),
                }
            } else {
                flow::Card {
                    title: ctx
                        .as_ref()
                        .map_or_else(String::new, |c| crate::plan::cards::title(c, k)),
                    summary: ctx
                        .as_ref()
                        .map_or_else(String::new, |c| crate::plan::cards::summary(c, k)),
                    mono: false,
                    done: false,
                    open: b.q == Some(k),
                    default: false,
                    toggle: Action::BQ(k),
                    guide: None,
                }
            }
        })
        .collect();
    // With more than one wallet loaded, the one backed up is a chip that
    // opens the list of them.
    let many = app.session.wallets.len() > 1;
    let heading = if many {
        EN.backup_back_up_a_wallet.to_string()
    } else {
        fill(EN.backup_back_up, &[&wallet.name.to_string()])
    };
    let pick = b.pick && many;
    // A small panel has no room for the side panel: the map is a page of
    // the plan, and the envelopes list the places.
    let compact = ui.compact;
    let col = flow::Column {
        area_x: x0,
        area_w: if compact { cw } else { cw - panel_w },
        x: col_x,
        w: col_w,
        h,
        back: Some(app.named_back(if b.from_vault {
            (EN.backup_vault_contents, Action::Nav(Screen::VaultContents))
        } else {
            (EN.backup_wallets, Action::OpenWallet(b.wallet))
        })),
        heading: &heading,
        guided: app.guided,
        switch: checklist,
        note: None,
        // From Create the wallet is the one just made, and the way on is
        // its card.
        chip: if b.from_create {
            Some(EN.backup_then_wallets)
        } else {
            many.then_some(wallet.name.as_str())
        },
        chip_tap: (many && !b.from_create).then_some(Action::BWallets),
        cancel: app.cancel_offered().is_some(),
    };
    let scroll = b.scroll;
    ui.chip_at = None;
    let all_done = checklist && marks.iter().all(|&d| d);
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            if checklist {
                backup_body(app_ref, ui, steps[i], x, y, w)
            } else {
                plan_body(app_ref, ui, ctx.as_ref(), steps[i], x, y, w)
            }
        };
        let mut foot = |ui: &mut Ui, x: f32, y: f32, w: f32| -> f32 {
            if !checklist {
                // Make the checklist is on The plan card alone
                // (`docs/BACKUP-FLOW.md` §15.1).
                return 0.0;
            }
            if all_done {
                backup_done(app_ref, ui, x, y, w)
            } else {
                wrap_buttons(
                    ui,
                    x,
                    y,
                    w,
                    40.0,
                    &[(EN.backup_change_the_plan, Style::Secondary, Action::BPlan)],
                )
            }
        };
        flow::column_foot(ui, &col, &cards, scroll, &mut body, Some(&mut foot))
    };
    app.scroll(Scrolled::Backup, next, again);
    // While the board is shown its list is the right-hand column, the
    // map's place (`docs/BACKUP-FLOW.md` §17.14).
    if !compact && app.board_shown() {
        crate::board::screen::list_panel(app, ui, x0 + cw - panel_w, panel_w, h);
    } else if !compact {
        backup_panel(app, ui, x0 + cw - panel_w, panel_w, h);
    }
    if pick && let Some((cx, cy, chip_w)) = ui.chip_at {
        let right = if compact { x0 + cw } else { x0 + cw - panel_w } - 16.0;
        wallet_pick(app, ui, cx, cy + 6.0, chip_w, right);
    }
}

/// The open card of Back up's plan (`docs/BACKUP-FLOW.md` §15.1), drawn
/// by `plan_cards`; on The plan, after the map, what holds the way on or
/// Make the checklist. Returns its height.
fn plan_body(
    app: &View,
    ui: &mut Ui,
    ctx: Option<&crate::plan::cards::PlanCtx>,
    q: u8,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let Some(ctx) = ctx else {
        return 0.0;
    };
    // On `wide` the board takes the section's whole width, under its
    // number as well as its title (`docs/BACKUP-FLOW.md` §17.15).
    let (bx, bw) = if q == crate::qstep::BOARD && !ui.compact {
        (
            x - crate::flow::BODY_INDENT,
            w + crate::flow::BODY_INDENT + crate::flow::BODY_RIGHT,
        )
    } else {
        (x, w)
    };
    let mut cy = y + crate::plan::cards::body(app, ui, ctx, q, bx, y, bw);
    if q != crate::qstep::PLAN {
        return cy - y;
    }
    cy += 8.0;
    if let Some(line) = crate::plan::cards::stickless_line(app, ctx) {
        cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, &line) + 12.0;
    }
    if let Some(line) = crate::plan::cards::held_line(app, ctx, app.backup_kept_nowhere()) {
        // On a small panel, where the way on is pinned at the foot, the
        // line takes its place there, greyed.
        if ui.pinning {
            ui.pin = Some((line, Style::Disabled, Action::BChecklist));
            return cy - y;
        }
        cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, &line) + 12.0;
        return cy - y;
    }
    if next_button(ui, x, cy, w, EN.plan_make_the_checklist, Action::BChecklist) {
        cy += 48.0;
    }
    cy - y
}

/// What a thing on the map is done by, in the checklist.
pub(crate) fn held_item(
    at: crate::plan::At,
    what: crate::plan::What,
    items: &[u8],
) -> Option<crate::plan::Item> {
    use crate::plan::{At, Item, What};
    let has = |it: Item| items.contains(&crate::bstep::of(it));
    let item = match (at, what) {
        (At::Away, _) | (_, What::VaultStick(_)) => return None,
        (At::Place(_), What::Passphrase(i)) => Item::Passphrase(i),
        (_, What::Passphrase(_)) => return None,
        (_, What::Words(i) | What::SeedQr(i)) => Item::Copy(i),
        (_, What::Sheet | What::Share(_)) => Item::Sheets,
        (At::Vault(v), _) => Item::Vault(v),
        (At::Software, _) if has(Item::ShowDescriptor) => Item::ShowDescriptor,
        (At::Files, What::SeedFile(_)) => Item::SeedFiles,
        // A file at a place is made and written with its stick
        // (`docs/BACKUP-FLOW.md` §17.12 item 6).
        (At::Place(_) | At::Stick(..), w) if crate::plan::is_file(w) => Item::Sticks,
        _ => Item::PublicFiles,
    };
    has(item).then_some(item)
}

/// Back up's map ([`crate::plan::cards::map_rows`]). Returns its height.
fn map_rows(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, boxed: bool) -> f32 {
    crate::plan::cards::PlanCtx::backup(app).map_or(0.0, |c| {
        crate::plan::cards::map_rows(app, ui, &c, x, y, w, boxed)
    })
}

/// Back up's check ([`crate::plan::cards::check_rows`]). Returns its
/// height.
fn check_rows(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    crate::plan::cards::PlanCtx::backup(app).map_or(0.0, |c| {
        crate::plan::cards::check_rows(app, ui, &c, x, y, w)
    })
}

/// The loaded wallets, one row each, under the backup's wallet chip at
/// (x, y): a press on one starts its backup, a press beside the list
/// closes it.
fn wallet_pick(app: &View, ui: &mut Ui, x: f32, y: f32, chip_w: f32, right: f32) {
    let current = app.backup.as_ref().map_or(0, |b| b.wallet);
    let lw = 340.0_f32.max(chip_w).min(right - x);
    let rh = 44.0;
    let n = app.session.wallets.len();
    let lh = n as f32 * rh + 8.0;
    ui.hit_around(x, y, lw, lh, Action::BWallets);
    ui.shadow(x, y, lw, lh, 12.0);
    ui.fill(x, y, lw, lh, 12.0, SURFACE);
    ui.stroke(x, y, lw, lh, 12.0, BORDER);
    for (k, wl) in app.session.wallets.iter().enumerate() {
        let ry = y + 4.0 + k as f32 * rh;
        let (fg, weight) = if k == current {
            (ACCENT, W::S)
        } else {
            (TEXT, W::R)
        };
        // The name first, up to half the row; the shape in what is left.
        let nw = ui.measure(14.0, weight, &wl.name).min(lw / 2.0);
        let name = ui.fit(14.0, weight, &wl.name, nw);
        let shape = ui.fit(12.0, W::R, &Session::shape(wl), lw - nw - 48.0);
        ui.text_mid(x + 14.0, ry, rh, 14.0, weight, fg, &name);
        ui.text_right(x + lw - 14.0, ry, rh, 12.0, W::R, MUTED, &shape);
        ui.hit(x + 4.0, ry, lw - 8.0, rh, Action::Backup(k));
    }
}

/// Under the backup's cards once every one is done
/// (`docs/history/NEW-WALLET.md` §14.6): the plan's three checks as a strip, the
/// wallet's chart of the plan just completed, what waits for the stick,
/// and Write to a stick, Open the wallet, Change the plan. Returns its
/// height.
fn backup_done(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::glance::{self, Backup, Direction};
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    ui.icon(x, cy, 24.0, Icon::Success, 13.0, OK);
    ui.text_mid(x + 32.0, cy, 24.0, 15.0, W::S, OK, EN.backup_done_backup);
    cy += 40.0;
    if let Some(g) = app.glance_of(b.wallet) {
        if let Backup::Plan { check, .. } = &g.backup {
            cy += glance::check_lines(ui, x, cy, w, *check, true, &glance::names_of(&g)) + 16.0;
        }
        cy += if ui.compact {
            glance::draw_column_with(ui, &g, Direction::WalletFirst, x, cy, w, false)
        } else {
            glance::draw_with(ui, &g, Direction::WalletFirst, x, cy, w, false)
        };
        cy += 16.0;
    }
    cy += extra_copies(app, ui, x, cy, w);
    // What the checklist made was written inside it: nothing waits For the
    // stick (`docs/BACKUP-FLOW.md` §10.2).
    let items: Vec<(&str, Style, Action)> = vec![
        (
            EN.backup_done_open_the,
            Style::Primary,
            Action::OpenWallet(b.wallet),
        ),
        (EN.backup_done_change, Style::Secondary, Action::BPlan),
    ];
    cy += wrap_buttons(ui, x, cy, w, 40.0, &items);
    cy - y
}

/// The backup's last card, in Guided only (`docs/BACKUP-FLOW.md` §17.19
/// item 7): copies made outside the plan, and the Learn page on them.
/// Returns its height; nothing in Steps only.
pub(crate) fn extra_copies(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if !app.guided {
        return 0.0;
    }
    let mut cy = y;
    cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, EN.backup_extra_copies) + 6.0;
    let label = EN.backup_extra_copies_link;
    let bw = (ui.measure(13.0, W::S, label) + 24.0).min(w);
    ui.button(
        x,
        cy,
        Some(bw),
        32.0,
        label,
        Style::Ghost,
        Action::LearnExtraCopies,
    );
    cy += 32.0 + 12.0;
    cy - y
}

/// A multisig's shares: **Keys left off each share** on a slider from 0
/// to the most the quorum allows, the shares and their keys'
/// fingerprints, and what the split audit finds. The plan's Places
/// question and the checklist's Shares card both draw it. Returns the
/// height used.
pub(crate) fn shares_card(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    i: usize,
    omit: usize,
) -> f32 {
    let Some(wallet) = app.session.wallets.get(i) else {
        return 0.0;
    };
    let (m, keys_n) = Session::quorum(wallet);
    let slots = app.session.slots(wallet);
    let fp = |j: usize| crate::wallet_backup::fp_of(slots.get(j).and_then(|s| s.fingerprint));
    shares_table(
        ui,
        (x, y, w),
        (m, keys_n, omit),
        &fp,
        crate::seeds::SLIDE_OMIT,
    )
}

/// The shares of an `m`-of-`keys_n` description with `omit` keys left off
/// each: the slider `slider` from 0 to the most the quorum allows, each
/// share with its keys (`fp` names key `j`), and what the split audit
/// finds. Back up's, from the wallet loaded, and a wallet flow's, from
/// its shape (`docs/BACKUP-FLOW.md` §15.7). Returns the height used.
pub(crate) fn shares_table(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (m, keys_n, omit): (usize, usize, usize),
    fp: &dyn Fn(usize) -> String,
    slider: u8,
) -> f32 {
    let most = m.saturating_sub(1).min(keys_n.saturating_sub(1));
    let omit = omit.min(most);
    let mut cy = y;
    let label = EN.shares_card_keys_left;
    if most == 0 {
        ui.text_mid(x, cy, 36.0, 13.0, W::R, MUTED, label);
        ui.text_mid(x + w - 20.0, cy, 36.0, 14.0, W::S, TEXT, "0");
        cy += 44.0;
    } else {
        // On a small panel the slider goes under the label.
        let (sx, sw) = if ui.compact {
            ui.text(x, cy, 13.0, W::S, MUTED, label);
            cy += 20.0;
            (x, w)
        } else {
            ui.text_mid(x, cy, 40.0, 13.0, W::R, MUTED, label);
            (x + 190.0, (w - 190.0).min(320.0))
        };
        let top = u8::try_from(most).unwrap_or(u8::MAX);
        let at = u8::try_from(omit).unwrap_or(top);
        cy += ui.slider(sx, cy, sw, slider, 0, top, at) + 4.0;
    }
    let plan = crate::wallet_backup::split_plan(keys_n, m, omit);
    for (k, row) in plan.iter().enumerate() {
        let fps: Vec<String> = row.iter().map(|&j| fp(j)).collect();
        ui.text(
            x,
            cy,
            12.0,
            W::R,
            MUTED,
            &fill(EN.shares_card_share, &[&(k + 1).to_string()]),
        );
        // On a narrow panel a share's keys go over more than one line.
        let gap = ui.measure(13.0, W::M, "  ");
        let mut fx = x + 70.0;
        for fp in &fps {
            let fw = ui.measure(13.0, W::M, fp);
            if fx > x + 70.0 && fx + fw > x + w {
                fx = x + 70.0;
                cy += 20.0;
            }
            ui.text(fx, cy, 13.0, W::M, TEXT, fp);
            fx += fw + gap;
        }
        cy += 24.0;
    }
    let a = crate::wallet_backup::audit(&plan, m);
    cy += 8.0;
    let lines = [
        (
            if a.every_quorum_rebuilds {
                fill(EN.shares_card_any_shares, &[&m.to_string()])
            } else {
                fill(EN.shares_card_some_shares, &[&m.to_string()])
            },
            a.every_quorum_rebuilds,
        ),
        (
            fill(
                EN.shares_card_the_fewest,
                &[&a.smallest_rebuild.to_string()],
            ),
            true,
        ),
        (
            fill(EN.shares_card_each_key, &[&a.copies_per_key.to_string()]),
            true,
        ),
        (
            if a.keys_per_sheet < keys_n {
                EN.shares_card_no_single.to_string()
            } else {
                EN.shares_card_each_share.to_string()
            },
            a.keys_per_sheet < keys_n,
        ),
    ];
    for (l, ok) in lines.iter() {
        cy += ui.wrap(x, cy, w, 13.0, W::R, if *ok { OK } else { WARN }, l) + 4.0;
    }
    cy + 10.0 - y
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn backup_body(app: &View, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::bstep;
    let b = app.backup.as_ref().expect("backup");
    let wallet = &app.session.wallets[b.wallet];
    let mut cy = y;
    match n {
        bstep::BLANK => {
            ui.text_mid(x, cy, 38.0, 13.0, W::R, MUTED, EN.backup_words_on_the);
            let (mut bx, bw) = if ui.compact {
                (x + w - 112.0, 52.0)
            } else {
                (x + 140.0, 64.0)
            };
            for words in [12usize, 24] {
                let style = if b.words == words {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                bx += ui.button(
                    bx,
                    cy,
                    Some(bw),
                    38.0,
                    &words.to_string(),
                    style,
                    Action::BWords(words),
                ) + 8.0;
            }
            cy += 52.0;
            // Printed from Faraday Starter, on the online computer, before
            // the copies by hand: nothing is made here.
            let shape = app.plan_shape(b.wallet);
            let copy = match (
                b.answers.seeds[crate::plan::seeds::WORDS],
                b.answers.seeds[crate::plan::seeds::SEEDQR],
            ) {
                (true, true) => EN.item_templates_both,
                (false, true) => EN.item_templates_seedqr,
                _ => EN.item_templates_words,
            };
            let line = fill(
                EN.item_templates_starter,
                &[
                    &crate::plan::templates_on(&shape, &app.plan_mapped(&shape, &b.answers))
                        .max(1)
                        .to_string(),
                    &b.words.to_string(),
                    copy,
                ],
            );
            cy += ui.wrap(x, cy, w, 14.0, W::R, TEXT, &line) + 16.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::STICKS => {
            cy += sticks_item(app, ui, x, cy, w);
            cy += 8.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        n if bstep::is_copy(n) => {
            let keys = app.backup_keys(b.wallet);
            if keys.is_empty() {
                ui.wrap(x, cy, w, 13.0, W::R, MUTED, EN.backup_no_key_of_this);
                cy += 30.0;
                // The small panel has no side panel: where each is kept
                // is on this page.
                if ui.compact {
                    cy += kept_rows(app, ui, x, cy, w);
                }
                let drawn = item_next(app, ui, x, cy, w, n);
                return cy + (if drawn { 48.0 } else { 8.0 }) - y;
            }
            let key = &app.session.keys[b.key];
            let word_count = usize::from(key.word_count.unwrap_or(0));
            // The seed's copies and the forms they take: the grid is drawn
            // for a SeedQR copy only (`docs/history/NEW-WALLET.md` §14.7).
            let forms = app.copy_forms(b, usize::from(n - bstep::COPY));
            let bw = ui.button(
                x,
                cy,
                None,
                38.0,
                if b.reveal {
                    EN.backup_hide_the_seed
                } else {
                    EN.backup_show_the_seed
                },
                if b.reveal {
                    Style::Secondary
                } else {
                    Style::Primary
                },
                Action::BReveal,
            );
            // Beside the button; on a small panel under it.
            let (ix, iy) = if ui.compact {
                (x, cy + 44.0)
            } else {
                (x + bw + 14.0, cy)
            };
            let info = fill(
                EN.backup_words,
                &[
                    &key_line(key.fingerprint, &key.label).to_string(),
                    &word_count.to_string(),
                ],
            );
            let info = ui.fit(13.0, W::R, &info, x + w - ix);
            ui.text_mid(ix, iy, 38.0, 13.0, W::R, MUTED, &info);
            cy = iy + 52.0;
            // The seed is not drawn while the typed check is open.
            if b.reveal && b.check.is_none() {
                let shown = app.reveal(b.key, Reveal::CopyByHand);
                let list: Vec<&str> = shown.split_whitespace().collect();
                // The words, with each word's index as SeedQR writes it.
                let lang = key.language;
                // Two columns on a small panel, the index at each one's end.
                let cols = if ui.compact { 2 } else { 4 };
                let colw = w / cols as f32;
                let rows = list.len().div_ceil(cols);
                for (i, wd) in list.iter().enumerate() {
                    let cx = x + (i / rows) as f32 * colw;
                    let ry = cy + (i % rows) as f32 * 26.0;
                    let idx = lang
                        .index_of(wd)
                        .map(|v| format!("{v:04}"))
                        .unwrap_or_default();
                    ui.text(cx, ry, 12.0, W::R, DIM, &format!("{:>2}", i + 1));
                    if ui.compact {
                        ui.text(cx + 20.0, ry, 13.0, W::M, TEXT, wd);
                        ui.text_right(cx + colw - 8.0, ry - 3.0, 20.0, 11.0, W::M, MUTED, &idx);
                    } else {
                        ui.text(cx + 26.0, ry, 14.0, W::M, TEXT, wd);
                        ui.text(cx + 104.0, ry, 12.0, W::M, MUTED, &idx);
                    }
                }
                cy += rows as f32 * 26.0 + 16.0;
                // SeedQR numbers the English list's words: another list's
                // words have no SeedQR another signer would read right.
                if forms.seedqr && lang != faraday_bip::bip39::Language::English {
                    cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, EN.backup_seedqr_is_for) + 12.0;
                } else if forms.seedqr {
                    // The SeedQR as a ruled grid to copy.
                    let items: Vec<(&str, Style, Action)> = [
                        (EN.backup_standard_seedqr, false),
                        (EN.backup_compact_seedqr, true),
                    ]
                    .into_iter()
                    .map(|(label, c)| {
                        let style = if b.compact == c {
                            Style::Primary
                        } else {
                            Style::Secondary
                        };
                        (label, style, Action::BCompact(c))
                    })
                    .collect();
                    cy += wrap_buttons(ui, x, cy, w, 34.0, &items) + 6.0;
                    // A secret shown: drawn from the words each frame.
                    let matrix =
                        app.seedqr_shown(lang, &app.reveal(b.key, Reveal::SeedQr), b.compact);
                    if let Some(mx) = matrix {
                        cy += seed_grid(ui, &mx, x, cy, w.min(560.0), b.pin) + 12.0;
                    }
                }
                cy += paper_section(app, ui, x, cy, w);
            }
            // The copy checked: its words typed back, or its SeedQR
            // scanned; the foot's Check my copy starts it.
            cy += copy_check(app, ui, x, cy, w);
            if ui.compact {
                cy += 8.0;
                cy += kept_rows(app, ui, x, cy, w);
            }
            cy += copy_foot(app, ui, x, cy, w, n);
        }
        n if bstep::is_passphrase(n) => {
            cy += passphrase_item(app, ui, x, cy, w);
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::PUBLIC => {
            cy += public_rows_for(app, ui, x, cy, w, b.wallet, Some(&app.backup_public()));
            cy += 12.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        n if n >= bstep::VAULT => {
            cy += vault_item(app, ui, x, cy, w, usize::from(n - bstep::VAULT));
            cy += 8.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::FILES => {
            cy += seed_files(app, ui, x, cy, w);
            cy += 8.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::SHOW => {
            let shown = if b.shown {
                EN.backup_shown
            } else {
                EN.backup_not_shown
            };
            ui.text_mid(
                x,
                cy,
                38.0,
                13.0,
                W::S,
                if b.shown { OK } else { MUTED },
                shown,
            );
            cy += 50.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::SHEETS if !(b.answers.split && crate::wallet_backup::splits(wallet)) => {
            let stem = crate::file_stem(&wallet.name);
            let file = format!("{stem}-backup.pdf");
            let (state, button) = made_state(
                app,
                &file,
                Some(Action::PublicRemove(b.wallet, 3)),
                (EN.backup_make_it, Action::BOut(3)),
            );
            cy += file_row(
                ui,
                x,
                cy,
                w,
                EN.backup_backup_sheet,
                EN.backup_a_pdf_to_print,
                state.as_deref(),
                button,
                &[],
            );
            cy += 12.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::SHEETS => {
            cy += shares_card(app, ui, x, cy, w, b.wallet, b.answers.omit);
            // Each share's sheet, text and picture, made For the stick
            // with the checklist; Remove takes them all.
            let stem = crate::file_stem(&wallet.name);
            let file = format!("{stem}-share-1-of-{}.pdf", Session::quorum(wallet).1);
            let (state, button) = made_state(
                app,
                &file,
                Some(Action::PublicRemove(b.wallet, 4)),
                (EN.backup_make_them, Action::BOut(4)),
            );
            cy += file_row(
                ui,
                x,
                cy,
                w,
                EN.backup_the_shares,
                EN.backup_sheets_to_print,
                state.as_deref(),
                button,
                &[],
            );
            cy += 12.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        _ => {
            // On a small panel the map is here, as a list of places.
            if ui.compact {
                cy += map_rows(app, ui, x, cy, w, false);
            } else {
                let shape = app.plan_shape(b.wallet);
                for spot in &app.plan_mapped(&shape, &b.answers).places {
                    let crate::plan::At::Place(p) = spot.at else {
                        continue;
                    };
                    let head = ui.fit(13.0, W::S, &app.place_name(p), w);
                    ui.text(x, cy, 13.0, W::S, TEXT, &head);
                    cy += 22.0;
                    let holds: Vec<String> = spot
                        .holds
                        .iter()
                        .map(|(h, _)| h.label(&shape, app.medium))
                        .collect();
                    let line = if holds.is_empty() {
                        EN.backup_nothing.to_string()
                    } else {
                        holds.join(" · ")
                    };
                    cy += ui.wrap(x + 12.0, cy, w - 12.0, 12.0, W::R, MUTED, &line) + 10.0;
                }
            }
            cy += 10.0;
            cy += buttons_and_next(
                ui,
                x,
                cy,
                w,
                &[(
                    EN.backup_back_to_the,
                    Style::Secondary,
                    Action::OpenWallet(b.wallet),
                )],
                Some((EN.backup_done, Action::BNext(n))),
            );
        }
    }
    cy - y
}

/// The SeedQR as a ruled grid to copy by hand: numbers every five
/// squares, the standard's fixed squares grey, the pinned row marked.
/// Returns its height.
/// The key's other paper forms: a Seed XOR split and codex32 shares,
/// shown to copy by hand like the words.
/// The copy of the seed checked: what scanning it found, and the typed
/// check while it is open (`docs/history/NEW-WALLET.md` §14.7): the word being
/// typed, the words taken masked, and how the last comparison went.
/// Returns its height.
fn copy_check(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let Some(key) = app.session.keys.info(b.key) else {
        return 0.0;
    };
    let mut cy = y;
    let pass = b.open.is_some_and(crate::bstep::is_passphrase);
    if let Some(found) = b.scanned.as_ref().filter(|_| !pass) {
        let c = if *found == crate::wallet_backup::CopyCheck::Matches {
            OK
        } else {
            ERR
        };
        let line = crate::wallet_backup::copy_scan_line(found);
        cy += ui.wrap(x, cy, w, 13.0, W::S, c, &line) + 12.0;
    }
    // A SeedQR copy kept as the words instead (§14.7 item 3): what stays
    // of the grid, then Keep the words only, or the scans without a match
    // once it has moved to the foot.
    if let Some(i) = b
        .open
        .filter(|&n| crate::bstep::is_copy(n))
        .map(|n| usize::from(n - crate::bstep::COPY))
    {
        if b.answers.part.iter().any(|&(_, s)| s == i) {
            let line = fill(EN.copy_check_drawn_in_part, &[&key.fingerprint.to_string()]);
            cy += ui.wrap(x, cy, w, 13.0, W::R, WARN, &line) + 12.0;
        }
        let misses = b.misses_of(key.fingerprint.0);
        if keep_words_offered(app, i) {
            if misses >= crate::copy_check::MISSES {
                let line = fill(EN.copy_check_scans_without, &[&misses.to_string()]);
                cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, &line) + 12.0;
            } else if b.check.is_none() {
                ui.button(
                    x,
                    cy,
                    None,
                    36.0,
                    EN.copy_check_keep_the_words,
                    Style::Secondary,
                    Action::BKeepWords,
                );
                cy += 36.0 + 12.0;
            }
        }
    }
    let Some(c) = b.check.as_ref() else {
        let (form, line) = if pass {
            (
                crate::plan::CopyForm::Passphrase,
                EN.copy_check_passphrase_matches,
            )
        } else {
            (crate::plan::CopyForm::Words, EN.copy_check_your_copy)
        };
        if b.checked.contains(&(key.fingerprint.0, form)) {
            cy += ui.wrap(x, cy, w, 13.0, W::S, OK, line) + 12.0;
        }
        return cy - y;
    };
    if c.passphrase {
        // The passphrase as typed, as bullets; the comparison's line.
        cy += ui.wrap(
            x,
            cy,
            w,
            12.0,
            W::R,
            DIM,
            EN.copy_check_your_copys_passphrase,
        ) + 4.0;
        ui.fill(x, cy, w, 42.0, 8.0, BG);
        ui.stroke(x, cy, w, 42.0, 8.0, ACCENT);
        let dots = "•".repeat(c.typed.chars().count());
        let dots = ui.fit(14.0, W::M, &dots, w - 30.0);
        ui.text_mid(x + 12.0, cy, 42.0, 14.0, W::M, TEXT, &dots);
        cy += 42.0 + 12.0 + never_below(app, ui, x, cy + 42.0, w);
    } else {
        cy = word_check(app, ui, x, cy, w, c);
    }
    // How the last comparison went, and Clear.
    let line = c.error.as_deref().unwrap_or("");
    let lh = ui.wrap(x, cy, w - 88.0, 13.0, W::S, ERR, line);
    ui.button(
        x + w - 80.0,
        cy - 8.0,
        Some(80.0),
        32.0,
        EN.copy_check_clear,
        Style::Ghost,
        Action::BCheckClear,
    );
    cy += lh.max(18.0) + 12.0;
    cy - y
}

/// The typed words of the open check: the word being typed and its
/// number, the words taken masked, the keys. Returns where it ends.
fn word_check(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    c: &crate::copy_check::WordCheck,
) -> f32 {
    let mut cy = y;
    let lang = c.lang;
    if !ui.compact {
        cy += ui.wrap(x, cy, w, 12.0, W::R, DIM, EN.copy_check_your_copys_words) + 4.0;
    }
    let at = (c.taken() + 1).min(c.count);
    let label = fill(
        EN.copy_check_word_of,
        &[&at.to_string(), &c.count.to_string()],
    );
    let error = c.error.as_deref();
    match c.keys.as_deref() {
        None => {
            cy += ui.wrap(x, cy, w, 13.0, W::S, MUTED, &label) + 8.0;
            let cells = Cells {
                boxes: c.count,
                masked: true,
                caret: true,
                never: app.online,
            };
            cy = typed_cells(ui, (x, cy, w), lang, &c.typed, cells);
        }
        Some(k) if ui.compact => {
            let status = error.map(|e| (e, ERR));
            cy = crate::compact::screen::keyed_head(
                ui,
                (x, cy, w),
                k,
                lang,
                &label,
                status,
                (true, c.shift, false),
            );
            cy += never_real_seed(app, ui, x, cy, w);
        }
        Some(k) => {
            cy += ui.wrap(x, cy, w, 13.0, W::S, MUTED, &label) + 8.0;
            let taken: Vec<u16> = k.committed_indices().collect();
            let prefix: String = k.prefix().iter().collect();
            let cells = Cells {
                boxes: c.count,
                masked: true,
                caret: true,
                never: app.online,
            };
            cy += keyed_cells(ui, (x, cy, w), lang, &taken, &prefix, cells);
            cy += keyed_pills(ui, x, cy, w, k, lang, false);
            cy = word_keys(c.shift, k, ui, x, cy, w, 30.0);
        }
    }
    cy
}

/// A passphrase on paper's item (`docs/history/NEW-WALLET.md` §14.7 item 5):
/// Show the passphrase, and the passphrase while shown and no check is
/// open; then the check. Returns its height.
fn passphrase_item(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let Some(key) = app.session.keys.info(b.key) else {
        ui.wrap(x, y, w, 13.0, W::R, MUTED, EN.backup_no_key_of_this);
        return 30.0;
    };
    let mut cy = y;
    let bw = ui.button(
        x,
        cy,
        None,
        38.0,
        if b.reveal {
            EN.backup_hide_the_passphrase
        } else {
            EN.backup_show_the_passphrase
        },
        if b.reveal {
            Style::Secondary
        } else {
            Style::Primary
        },
        Action::BReveal,
    );
    let (ix, iy) = if ui.compact {
        (x, cy + 44.0)
    } else {
        (x + bw + 14.0, cy)
    };
    let info = fill(
        EN.backup_passphrase_of,
        &[&key_line(key.fingerprint, &key.label).to_string()],
    );
    let info = ui.fit(13.0, W::R, &info, x + w - ix);
    ui.text_mid(ix, iy, 38.0, 13.0, W::R, MUTED, &info);
    cy = iy + 52.0;
    // The passphrase is not drawn while the check is open.
    if b.reveal && b.check.is_none() {
        if key.has_passphrase {
            let shown = app.reveal(b.key, Reveal::Passphrase);
            cy += ui.wrap(x, cy, w, 16.0, W::M, TEXT, &shown) + 16.0;
        } else {
            cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, EN.backup_no_passphrase_kept) + 12.0;
        }
    }
    cy += copy_check(app, ui, x, cy, w);
    if ui.compact {
        cy += 8.0;
        cy += kept_rows(app, ui, x, cy, w);
    }
    cy - y
}

/// A checklist item's next action (`docs/history/NEW-WALLET.md` §14.1): its own
/// until it is done, then Continue. A vault item: Make a vault (or the
/// way to one), then Save into the open vault, then Continue.
fn item_action(app: &View, n: u8) -> (String, Action) {
    use crate::bstep;
    use crate::plan::Item;
    let item = bstep::item(n).unwrap_or(Item::Envelopes);
    let next = (EN.item_action_continue.to_string(), Action::BNext(n));
    if item == Item::Envelopes || app.backup_item_done(item) {
        return next;
    }
    let Some(b) = app.backup.as_ref() else {
        return next;
    };
    let split = b.answers.split
        && app
            .session
            .wallets
            .get(b.wallet)
            .is_some_and(crate::wallet_backup::splits);
    match item {
        Item::Templates => (EN.item_action_done.to_string(), Action::BNext(n)),
        Item::Sticks => (
            EN.backup_flow_lock_and_write.to_string(),
            Action::Flow(crate::backup_flow::steps::FlowAction::WalletLock),
        ),
        // The words typed back first; then, for a SeedQR copy, the scan
        // (`docs/history/NEW-WALLET.md` §14.7 item 1).
        Item::Copy(i) => {
            use crate::plan::CopyForm;
            let words_left = app.session.keys.info(b.key).is_none_or(|k| {
                let fp = k.fingerprint.0;
                let has = |f| b.checked.contains(&(fp, f));
                app.copy_checks(b, i, k).typed
                    && !(has(CopyForm::Words)
                        || (!app.copy_forms(b, i).words && has(CopyForm::SeedQr)))
            });
            let action = if words_left {
                Action::BCheck
            } else {
                Action::BScan
            };
            (EN.item_action_check_my.to_string(), action)
        }
        // Check my copy opens the field; with it open, Compare.
        Item::Passphrase(_) if b.check.is_some() => {
            (EN.item_action_compare.to_string(), Action::BCheck)
        }
        Item::Passphrase(_) => (EN.item_action_check_my.to_string(), Action::BCheck),
        Item::Vault(v) => match app.backup_vault_target(v) {
            Some(cur) if app.backup_vault_taken(v, cur) => (
                EN.item_action_lock_it.to_string(),
                Action::BNewVault(v.min(255) as u8),
            ),
            Some(cur) => (
                fill(
                    EN.item_action_save_into,
                    &[&app.vaults.open[cur].name.to_string()],
                ),
                Action::BVaultSave(v.min(255) as u8),
            ),
            // A vault of the plan with none set up for it, whether or not
            // another is open: a new one on an empty stick (written at the
            // write step as an added copy is).
            None => app.vault_way(Screen::Backup).unwrap_or((
                EN.vaults_make_a_vault.to_string(),
                Action::Vault(crate::vault::VaultAction::CreateFrom(Screen::Backup)),
            )),
        },
        Item::SeedFiles => (EN.item_action_make_it.to_string(), Action::BFile),
        Item::Sheets if split => (EN.item_action_make_them.to_string(), Action::BOut(4)),
        Item::Sheets => (EN.item_action_make_it.to_string(), Action::BOut(3)),
        Item::PublicFiles => (EN.item_action_make_them.to_string(), Action::BFilesMake),
        Item::ShowDescriptor => (
            EN.item_action_show_as.to_string(),
            Action::QrWallet(b.wallet),
        ),
        Item::Envelopes => next,
    }
}

/// Whether seed `i`'s copy item offers Keep the words only
/// (`docs/history/NEW-WALLET.md` §14.7 item 3): the seed has a SeedQR copy and
/// the item is not done.
fn keep_words_offered(app: &View, i: usize) -> bool {
    app.backup.as_ref().is_some_and(|b| {
        app.copy_forms(b, i).seedqr && !app.backup_item_done(crate::plan::Item::Copy(i))
    })
}

/// A copy item's foot: its next action, and after three scans without a
/// match Keep the words only beside it (`docs/history/NEW-WALLET.md` §14.7 item
/// 3). Returns the height used in the column.
fn copy_foot(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, n: u8) -> f32 {
    let i = usize::from(n - crate::bstep::COPY);
    let missed = app.backup.as_ref().is_some_and(|b| {
        app.session
            .keys
            .info(b.key)
            .is_some_and(|k| b.misses_of(k.fingerprint.0) >= crate::copy_check::MISSES)
    });
    if !(missed && keep_words_offered(app, i)) {
        return if item_next(app, ui, x, y, w, n) {
            48.0
        } else {
            0.0
        };
    }
    let (label, action) = item_action(app, n);
    let typing = app.backup.as_ref().and_then(|b| b.check.as_ref()).is_some();
    let next = (!(typing && action == Action::BCheck)).then_some((label.as_str(), action));
    buttons_and_next(
        ui,
        x,
        y,
        w,
        &[(
            EN.copy_check_keep_the_words,
            Style::Secondary,
            Action::BKeepWords,
        )],
        next,
    )
}

/// The item's next action as its foot button. Returns whether it was
/// drawn in the column (on a small panel it is pinned instead).
fn item_next(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, n: u8) -> bool {
    let (label, action) = item_action(app, n);
    // While the typed check is open it compares on the last word: the
    // foot's Check my copy would do nothing, so it is not drawn.
    if action == Action::BCheck
        && app
            .backup
            .as_ref()
            .and_then(|b| b.check.as_ref())
            .is_some_and(|c| !c.passphrase)
    {
        return false;
    }
    let label = ui.fit(14.0, W::S, &label, w - 36.0);
    next_button(ui, x, y, w, &label, action)
}

/// The checklist's Write to sticks: what the lock keeps and wipes, then a
/// row for each stick by place, ticked once written, its files under it.
/// Its one button is the foot's. Returns the height used.
fn sticks_item(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let written = app.wallet_sticks_done();
    if !written {
        let kw = app.kept_wiped(false);
        for (title, lines) in [
            (
                EN.backup_flow_kept,
                kw.kept
                    .iter()
                    .map(|l| (l.clone(), TEXT))
                    .collect::<Vec<_>>(),
            ),
            (
                EN.backup_flow_wiped,
                kw.wiped
                    .iter()
                    .map(|(l, nowhere)| (l.clone(), if *nowhere { ERR } else { TEXT }))
                    .collect(),
            ),
        ] {
            if lines.is_empty() {
                continue;
            }
            ui.text(x, cy, 12.0, W::R, MUTED, title);
            cy += 22.0;
            for (l, tone) in lines {
                cy += ui.wrap(x, cy, w, 13.0, W::R, tone, &l) + 6.0;
            }
            cy += 6.0;
        }
    }
    for (name, files, done) in app.wallet_stick_rows() {
        let name = {
            let mut c = name.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        };
        ui.dot(
            x + 4.0,
            cy + 9.0,
            3.5,
            if done.is_some() { OK } else { DIM },
        );
        let tail = done.clone().unwrap_or_default();
        let room = w - ui.measure(12.0, W::M, &tail) - 30.0;
        let head = ui.fit(13.0, W::S, &name, room);
        ui.text_mid(x + 18.0, cy, 18.0, 13.0, W::S, TEXT, &head);
        if done.is_some() {
            ui.text_right(x + w, cy, 18.0, 12.0, W::M, OK, &tail);
        }
        cy += 24.0;
        if !files.is_empty() {
            cy += ui.wrap(x + 18.0, cy, w - 18.0, 12.0, W::M, MUTED, &files.join(", ")) + 8.0;
        }
        cy += 4.0;
    }
    cy - y
}

/// A vault's checklist item: what Save into the open vault saves, a row
/// each, marked once the vault holds it; with the open vault holding
/// another vault's seed, that. Its one button is the foot's. Returns its
/// height.
fn vault_item(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, v: usize) -> f32 {
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    // The vault this plan vault is saved into: the one set up for it, else
    // the current one if no other plan vault is its.
    let target = app.backup_vault_target(v);
    let cur = target.unwrap_or(usize::MAX);
    let open = target.and_then(|t| app.vaults.open.get(t));
    if let Some(o) = open
        && app.backup_vault_taken(v, cur)
    {
        let line = fill(EN.vault_item_holds_another, &[&o.name.to_string()]);
        let line = ui.fit(13.0, W::S, &line, w);
        ui.text(x, cy + 4.0, 13.0, W::S, WARN, &line);
        cy += 32.0;
    }
    let head = match open {
        Some(o) => fill(EN.vault_item_into, &[&o.name.to_string()]),
        None => EN.vault_item_into_the.to_string(),
    };
    let head = ui.fit(13.0, W::S, &head, w);
    section_label(ui, x, cy, &head);
    cy += 28.0;
    let (own, _) = app.backup_vault_seeds(v);
    let mut rows: Vec<(String, bool)> = own
        .iter()
        .filter_map(|&(k, with)| {
            let key = app.session.keys.info(k)?;
            let name = fp_text(key.fingerprint);
            let label = if with {
                fill(EN.vault_item_seed_with, &[&name.to_string()])
            } else {
                fill(EN.vault_item_seed, &[&name.to_string()])
            };
            let has = open.is_some() && app.vault_holds_seed(cur, k, with);
            Some((label, has))
        })
        .collect();
    if b.answers.wallet_in_vault(v) {
        let has = open.is_some()
            && app
                .session
                .wallets
                .get(b.wallet)
                .is_some_and(|wl| app.vault_has_wallet(cur, wl));
        rows.push((EN.vault_item_wallet_description.to_string(), has));
    }
    for (label, has) in rows {
        ui.dot(x + 4.0, cy + 9.0, 3.5, if has { OK } else { DIM });
        let label = ui.fit(13.0, W::R, &label, w - 18.0);
        ui.text_mid(x + 18.0, cy, 18.0, 13.0, W::R, TEXT, &label);
        cy += 24.0;
    }
    // The wallet's record loads at unlock unless turned off here (F62).
    if b.answers.wallet_in_vault(v) {
        let on = !b.no_load.contains(&v);
        let top = cy;
        cy += 4.0;
        ui.checkbox(x, cy, on, true);
        let label = ui.fit(13.0, W::S, EN.vault_load_at_unlock, w - 28.0);
        ui.text_mid(x + 28.0, cy - 2.0, 22.0, 13.0, W::S, TEXT, &label);
        cy += 28.0;
        ui.hit(
            x - 4.0,
            top,
            w + 8.0,
            cy - top,
            Action::BVaultLoad(v.min(255) as u8),
        );
    }
    // No vault yet for this one while another is open: a vault already on
    // a stick read this power-on is the other way to it.
    if target.is_none()
        && let Some((label, action)) = app.vault_unlock_way(Screen::Backup)
    {
        cy += 4.0;
        let label = ui.fit(14.0, W::S, &label, w - 36.0);
        ui.button(x, cy, None, 40.0, &label, Style::Secondary, action);
        cy += 48.0;
    }
    cy + 4.0 - y
}

/// The seeds' files, on their checklist item: a row per seed here,
/// marked once its file is made. The foot's Make it makes the next one.
/// Returns the height used.
fn seed_files(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let Some(wallet) = app.session.wallets.get(b.wallet) else {
        return 0.0;
    };
    let stem = crate::file_stem(&wallet.name);
    let mut cy = y + 4.0;
    for k in app.backup_keys(b.wallet) {
        let key = &app.session.keys[k];
        let made = app.seed_file_made(&stem, key);
        ui.dot(x + 4.0, cy + 9.0, 3.5, if made { OK } else { DIM });
        let name = key_line(key.fingerprint, &key.label);
        let state = if made {
            app.medium.for_box()
        } else {
            EN.seed_files_not_made
        };
        let sw = ui.measure(12.0, W::R, state);
        let name = ui.fit(13.0, W::S, &name, w - sw - 30.0);
        ui.text_mid(x + 18.0, cy, 18.0, 13.0, W::S, TEXT, &name);
        ui.text_right(
            x + w,
            cy,
            18.0,
            12.0,
            W::R,
            if made { OK } else { MUTED },
            state,
        );
        cy += 26.0;
    }
    cy + 4.0 - y
}

fn paper_section(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::paper::PaperForm;
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let mut cy = y + 8.0;
    section_label(ui, x, cy, EN.paper_section_other);
    cy += 28.0;
    match &b.paper {
        None => {
            let row: Vec<(String, Style, Action)> = vec![
                (
                    EN.paper_section_seed_xor.into(),
                    Style::Secondary,
                    Action::BXor(2),
                ),
                (
                    EN.paper_section_seed.into(),
                    Style::Secondary,
                    Action::BXor(3),
                ),
                (
                    EN.paper_section_codex32_2.into(),
                    Style::Secondary,
                    Action::BCodex32(2, 3),
                ),
                (
                    EN.paper_section_codex32.into(),
                    Style::Secondary,
                    Action::BCodex32(3, 5),
                ),
            ];
            cy += button_rows(ui, x, cy, w, &row);
        }
        Some(form) => {
            let (lines, line) = match form {
                PaperForm::Xor(parts) => (
                    parts,
                    fill(EN.paper_section_parts, &[&parts.len().to_string()]),
                ),
                PaperForm::Codex32 { k, strings } => (
                    strings,
                    fill(
                        EN.paper_section_shares,
                        &[&strings.len().to_string(), &k.to_string()],
                    ),
                ),
            };
            ui.text(x, cy, 13.0, W::S, WARN, &line);
            cy += 26.0;
            for (i, l) in lines.iter().enumerate() {
                ui.text(x, cy, 12.0, W::R, MUTED, &format!("{}", i + 1));
                cy += ui
                    .wrap(x + 28.0, cy, w - 28.0, 13.0, W::M, TEXT, l)
                    .max(18.0)
                    + 10.0;
            }
            ui.button(
                x,
                cy,
                None,
                34.0,
                EN.paper_section_put_them,
                Style::Ghost,
                Action::BPaperHide,
            );
            cy += 44.0;
        }
    }
    cy - y
}

fn seed_grid(
    ui: &mut Ui,
    m: &faraday_codec::qr::QrMatrix,
    x: f32,
    y: f32,
    w: f32,
    pin: usize,
) -> f32 {
    let n = m.size();
    let margin = 26.0;
    let cell = ((w - margin) / n as f32).floor().max(6.0);
    let gx = x + margin;
    let gy = y + margin;
    let side = cell * n as f32;
    let pin = pin.min(n - 1);
    ui.fill(gx, gy, side, side, 0.0, faraday_draw::Color::WHITE);
    ui.fill(
        gx,
        gy + pin as f32 * cell,
        side,
        cell,
        0.0,
        ACCENT.with_alpha(90),
    );
    for row in 0..n {
        for col in 0..n {
            if m.module(col, row) {
                let c = if crate::wallet_backup::structural(col, row, n) {
                    faraday_draw::Color::rgb(0x8a, 0x94, 0x9e)
                } else {
                    faraday_draw::Color::BLACK
                };
                ui.fill(
                    gx + col as f32 * cell,
                    gy + row as f32 * cell,
                    cell,
                    cell,
                    0.0,
                    c,
                );
            }
        }
        ui.hit(gx, gy + row as f32 * cell, side, cell, Action::BPin(row));
    }
    // The ruling: light every square, darker every five.
    for i in 0..=n {
        let c = if i % 5 == 0 {
            faraday_draw::Color::rgb(0x6f, 0x82, 0x91)
        } else {
            faraday_draw::Color::rgb(0xc8, 0xd0, 0xd6)
        };
        ui.fill(gx + i as f32 * cell, gy, 1.0, side, 0.0, c);
        ui.fill(gx, gy + i as f32 * cell, side, 1.0, 0.0, c);
        if i % 5 == 0 && i < n {
            ui.text(
                gx + i as f32 * cell + 2.0,
                y + 4.0,
                11.0,
                W::M,
                MUTED,
                &i.to_string(),
            );
            ui.text(
                x,
                gy + i as f32 * cell + 2.0,
                11.0,
                W::M,
                MUTED,
                &i.to_string(),
            );
        }
    }
    // Beside the pinned row; on a small panel, under the grid.
    if ui.compact {
        ui.text(
            gx,
            gy + side + 8.0,
            12.0,
            W::S,
            ACCENT,
            &fill(EN.seed_grid_row, &[&pin.to_string()]),
        );
        return margin + side + 34.0;
    }
    ui.text(
        gx + side + 10.0,
        gy + pin as f32 * cell - 3.0,
        12.0,
        W::S,
        ACCENT,
        &fill(EN.seed_grid_row, &[&pin.to_string()]),
    );
    margin + side + 10.0
}

/// The backup's side panel: the map, its boxes listing what each holds,
/// in the checklist each line marked by its item's state; the check at
/// its foot.
fn backup_panel(app: &View, ui: &mut Ui, px: f32, pw: f32, h: f32) {
    if app.backup.is_none() {
        return;
    }
    ui.fill(px, 0.0, pw, h, 0.0, SURFACE);
    ui.fill(px, 0.0, 1.0, h, 0.0, LINE);
    let x = px + 22.0;
    let w = pw - 44.0;
    // The check is measured first, so the map stops above it.
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let mark = ui.hits.len();
    let ch = check_rows(app, ui, x, 0.0, w);
    ui.hits.truncate(mark);
    ui.c.pop_clip();
    let foot = h - ch - 16.0;
    ui.text(x, 28.0, 13.0, W::S, MUTED, EN.backup_panel_map);
    let clip = ui.rect(px, 52.0, pw, foot - 60.0);
    ui.c.push_clip(clip);
    map_rows(app, ui, x, 56.0, w, true);
    ui.c.pop_clip();
    ui.rule(x, foot - 4.0, w, INNER);
    check_rows(app, ui, x, foot + 8.0, w);
}

/// The colour of a line of [`crate::wallet_backup::Kept`].
fn tone(t: crate::wallet_backup::Tone) -> faraday_draw::Color {
    use crate::wallet_backup::Tone;
    match t {
        Tone::Ok => OK,
        Tone::Warn => WARN,
        Tone::Err => ERR,
        Tone::Dim => DIM,
    }
}

/// Where the wallet and each of its seeds are kept: the wallet's line,
/// then "Keys" and a row per key, its fingerprint and its lines. The
/// backup's side panel and the small panel's seeds page draw it. Returns
/// its height.
fn kept_rows(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(kept) = app.backup_kept() else {
        return 0.0;
    };
    let mut cy = y;
    let (line, t) = &kept.wallet;
    cy += ui.wrap(x, cy, w, 13.0, W::S, tone(*t), line) + 16.0;
    ui.text(x, cy, 13.0, W::S, MUTED, EN.kept_rows_keys);
    cy += 26.0;
    for seed in &kept.seeds {
        let name = ui.fit(13.0, W::M, &seed.name, w);
        ui.text(x, cy, 13.0, W::M, TEXT, &name);
        cy += 22.0;
        for (line, t) in &seed.lines {
            cy += ui.wrap(x + 12.0, cy, w - 12.0, 12.0, W::R, tone(*t), line) + 4.0;
        }
        cy += 8.0;
    }
    cy - y
}

/// A file offered on a card: its name, what reads it, the file's name,
/// its own button's action, and Show as QR and PNG when it has them.
type FileRow<'a, N> = (N, &'a str, String, Action, Vec<(&'a str, Action)>);

/// Wallet `i`'s public files, as the backup's public step offers them:
/// first the descriptor, the wallet file, the multisig config, the
/// backup sheet, the BSMS record, Bitcoin Core's import for Core 29 or
/// later, and each key held here with its BSMS record, for the
/// cosigners; then the files under More options (F64), where the wallet
/// has each: Bitcoin Core before 29, the Electrum wallet file, the
/// Caravan config, the Wasabi file. Each has its own button, and Show as
/// QR and PNG beside the ones a wallet or a person reads from a code.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn public_files(
    app: &View,
    i: usize,
) -> (Vec<FileRow<'static, String>>, Vec<FileRow<'static, String>>) {
    use faraday_bip::policy::{Template, Wrapper};
    let Some(wl) = app.session.wallets.get(i) else {
        return (Vec::new(), Vec::new());
    };
    let stem = crate::file_stem(&wl.name);
    let codes = |show: Action, png: Action| vec![(EN.public_files_show_as, show), ("PNG", png)];
    let mut rows: Vec<FileRow<String>> = vec![
        (
            EN.public_files_descriptor.to_string(),
            EN.public_files_any_wallet,
            format!("{stem}-descriptor.txt"),
            Action::PublicOut(i, 1),
            codes(Action::QrWallet(i), Action::PublicOut(i, 8)),
        ),
        (
            EN.public_files_wallet.to_string(),
            EN.public_files_sparrow,
            format!("{stem}-wallet.json"),
            Action::PublicOut(i, 5),
            Vec::new(),
        ),
    ];
    if crate::wallet_backup::multisig_config(wl, None).is_some() {
        rows.push((
            EN.public_files_multisig.to_string(),
            EN.public_files_coldcard,
            format!("{stem}-multisig-config.txt"),
            Action::PublicOut(i, 2),
            codes(
                Action::ShowCode(Code::MultisigConfig(i)),
                Action::CodePng(Code::MultisigConfig(i)),
            ),
        ));
    }
    rows.push((
        EN.public_files_backup.to_string(),
        EN.public_files_a_pdf,
        format!("{stem}-backup.pdf"),
        Action::PublicOut(i, 3),
        Vec::new(),
    ));
    // BIP 129 covers wsh and sh(wsh) multisig.
    if matches!(
        wl.policy().template(),
        Template::Multi {
            wrapper: Wrapper::Wsh | Wrapper::ShWsh,
            ..
        }
    ) {
        rows.push((
            EN.public_files_bsms_descriptor.to_string(),
            EN.public_files_bip_129,
            format!("{stem}-bsms.txt"),
            Action::PublicOut(i, 6),
            codes(
                Action::ShowCode(Code::Bsms(i)),
                Action::CodePng(Code::Bsms(i)),
            ),
        ));
    }
    // A silent payments wallet has no descriptor Bitcoin Core imports.
    if wl.policy().silent().is_none() {
        rows.push((
            EN.public_files_bitcoin.to_string(),
            EN.public_files_bitcoin_core,
            format!("{stem}-bitcoin-core.json"),
            Action::PublicOut(i, 7),
            Vec::new(),
        ));
    }
    // The account key of each seed held here, for the cosigners.
    for slot in app.wallet_keys_here(i) {
        let Ok(k) = app.wallet_key(i, slot) else {
            continue;
        };
        let fp = k.fp.clone();
        let record = matches!(
            k.standard,
            Some(crate::create::NewKind::Multi | crate::create::NewKind::MultiNested)
        );
        rows.push((
            fill(EN.public_files_key, &[&fp.to_string()]),
            EN.public_files_for_the,
            format!("xpub-{fp}.txt"),
            Action::WalletKeyOut(i, slot),
            codes(
                Action::ShowCode(Code::WalletKey(i, slot)),
                Action::CodePng(Code::WalletKey(i, slot)),
            ),
        ));
        if record {
            rows.push((
                fill(EN.public_files_key_bsms, &[&fp.to_string()]),
                EN.public_files_signed,
                format!("xpub-{fp}-bsms.txt"),
                Action::WalletKeyBsms(i, slot),
                codes(
                    Action::ShowCode(Code::WalletKeyBsms(i, slot)),
                    Action::CodePng(Code::WalletKeyBsms(i, slot)),
                ),
            ));
        }
    }
    let more = crate::wallet_backup::public_more_files(wl)
        .into_iter()
        .map(|(what, file)| {
            let (name, detail) = match what {
                9 => (EN.public_files_core_before_29, EN.public_files_bitcoin_core),
                10 => (
                    EN.public_files_electrum,
                    EN.public_files_electrum_watch_only,
                ),
                11 => (EN.public_files_caravan, EN.public_files_caravan_unchained),
                _ => (EN.public_files_wasabi, EN.public_files_wasabi_wallet),
            };
            (
                name.to_string(),
                detail,
                file,
                Action::PublicOut(i, what),
                Vec::new(),
            )
        })
        .collect();
    (rows, more)
}

/// Draws [`public_files`] of wallet `i` on the backup's checklist, each
/// marked once its file is in the Outbox: only the files `only`
/// names (as `public_out` numbers them) and the keys for the cosigners,
/// the codes beside them only where the plan's form has QR pictures;
/// then a ghost More options, or, once it is pressed, the files under
/// it (F64).
fn public_rows_for(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    i: usize,
    only: Option<&[u8]>,
) -> f32 {
    let qr = app
        .backup
        .as_ref()
        .is_none_or(|b| only.is_none() || b.answers.form[crate::plan::form::QR]);
    let mut cy = y;
    let (primary, more) = public_files(app, i);
    let open = app
        .backup
        .as_ref()
        .is_some_and(|b| b.wallet == i && b.public_more);
    let shown = primary
        .iter()
        .filter(|(_, _, _, action, _)| match (only, action) {
            (Some(o), Action::PublicOut(_, what)) => o.contains(what),
            _ => true,
        })
        .chain(more.iter().filter(|_| open));
    for (name, detail, file, action, extra) in shown {
        let extra: &[(&str, Action)] = if qr { extra } else { &[] };
        // Made For the stick with the checklist: Remove while it waits.
        let remove = match action {
            Action::PublicOut(w, what) => Some(Action::PublicRemove(*w, *what)),
            _ => None,
        };
        let (state, button) = made_state(app, file, remove, (EN.public_rows_for_make, *action));
        cy += file_row(ui, x, cy, w, name, detail, state.as_deref(), button, extra);
    }
    if !open && !more.is_empty() {
        cy += 8.0;
        ui.button(
            x,
            cy,
            None,
            36.0,
            EN.public_rows_for_more_options,
            Style::Ghost,
            Action::PublicMore(i),
        );
        cy += 44.0;
    }
    cy - y
}
