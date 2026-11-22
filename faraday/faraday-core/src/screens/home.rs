//! `Screen::Home`.

use super::menu_page::{self, Page, Section};
use crate::drawing::View;
use crate::in_progress::Going;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Glyph, Ui};
use crate::ui::{Region, Shape, Slot, Style};
use crate::{Action, Screen};
use faraday_draw::tokens::{BODY_L, BODY_M, BUTTON_H, SHAPE_M, TITLE_M, TITLE_S};
use faraday_draw::tokens::{
    ELEVATION_FAB, HOME_CARD_GAP, HOME_CARD_GAP_TALL, HOME_CARD_HEAD, HOME_CARD_KEYS,
    HOME_CARD_TALL_FROM, HOME_CARD_TWO_COLS, PAD,
};
use faraday_draw::tokens::{HOME_BOX_KEYS, HOME_BOX_MIN_ROWS, MARGIN_WIDE};

use faraday_draw::tokens::{GAP_SMALL, GRID, ICON_SMALL, LIST_1, LIST_2, LIST_PAD, NAV_ICON};
use faraday_draw::tokens::{
    HOME_LEAD_H, HOME_LEAD_ICON, HOME_LEAD_INSET, HOME_LEAD_SUB_Y, HOME_LEAD_TEXT_X,
    HOME_SEARCH_MIN, MENU_GAP,
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
    /// The button's label; empty for a card that is itself the press.
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
                "",
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
/// table that applies; none when nothing waits (a key no wallet uses is a
/// row under Keys, §3). `skip_shown` drops a job that would open or start
/// over a flow whose line is above the card (`docs/FLOWS.md`, Flows in
/// progress), as the one-column Home has it. With the flow it carries on or
/// starts over, when it is one.
fn lead_job(app: &View, skip_shown: bool) -> Option<(Option<Going>, LeadCard)> {
    let shown = if skip_shown {
        app.flows_in_progress()
    } else {
        Default::default()
    };
    let hidden = |g: Option<Going>| g.is_some_and(|g| shown.iter().any(|r| r.going == g));
    if let Some(j) = home_jobs(app).into_iter().find(|j| !hidden(j.going)) {
        return Some((j.going, j.card));
    }
    None
}

/// Home's lead as the card's first item, with what it stands for: the flow
/// it carries on and the wallets whose rows it repeats.
#[derive(Clone)]
pub(crate) struct CardLead {
    card: LeadCard,
    going: Option<Going>,
    wallets: Vec<usize>,
}

/// The lead for inside the card (`docs/HOME.md` §2), when there is one.
/// `skip_shown` is the rail's windows', which skip a lead over a flow whose
/// row is in the box.
pub(crate) fn card_lead(app: &View, skip_shown: bool) -> Option<CardLead> {
    let (going, card) = lead_job(app, skip_shown)?;
    let wallets = (0..app.session.wallets.len())
        .filter(|&w| match going {
            Some(Going::Spend) => {
                app.wallets.spend().is_some_and(|s| s.wallet == Some(w))
                    || matches!(card.action, Action::StartSpend(i) if app.psbt_for(w) == Some(i))
            }
            _ => false,
        })
        .collect();
    Some(CardLead {
        card,
        going,
        wallets,
    })
}

/// A wallet's tile on Home (`docs/HOME.md` §3): its name and the one line
/// that matters now.
#[derive(Debug, Clone)]
pub(crate) struct WalletCard {
    /// The wallet's place in the session.
    pub index: usize,
    /// Its name.
    pub name: String,
    /// The first of: its spend's signatures, "A PSBT to sign", "Not
    /// backed up", its shape.
    pub line: String,
    /// Whether the line says something to do now: its spend's signatures, a
    /// PSBT to sign or a backup waiting, not its shape.
    pub needs: bool,
}

/// The wallets in the session, ranked (the ones the keys
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

/// The wallets in the session that need something now, by place, in the
/// session's order, worked out once a frame: the Wallets badge and the Pi's
/// box of what is loaded read the one answer.
pub(crate) fn wallets_needing(app: &View) -> std::rc::Rc<Vec<usize>> {
    app.drawing
        .needing
        .get_or_init(|| {
            std::rc::Rc::new(
                (0..app.session.wallets.len())
                    .filter(|&i| wallet_needs(app, i))
                    .collect(),
            )
        })
        .clone()
}

/// Whether wallet `i` needs something now: its spend is under way, Files
/// holds a PSBT for it or a key made here waits for its backup.
fn wallet_needs(app: &View, i: usize) -> bool {
    app.wallets.spend().is_some_and(|s| s.wallet == Some(i))
        || app.psbt_for(i).is_some()
        || app.wallet_waits_kept(i)
}

/// Wallet `i`'s card.
fn wallet_card(app: &View, i: usize) -> WalletCard {
    let w = &app.session.wallets[i];
    let held = app.receive_held(w);
    let spend = app.wallets.spend().filter(|s| s.wallet == Some(i));
    let needs = wallet_needs(app, i);
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
        needs,
    }
}

/// One of Home's list rows (`docs/HOME.md` §3): its leading icon, its
/// headline, the line under it (empty for none), the small text at its end
/// (empty for none; `accent` draws it as an action) and what a press
/// opens. Each ends in a chevron.
#[derive(Clone)]
struct ListRow {
    icon: Icon,
    head: String,
    sub: String,
    trailing: String,
    accent: bool,
    action: Action,
}

/// `rows` as one tinted, rounded box (the surface of the Pi's box of what
/// is loaded) of Material list items one under another from `(x, y)`, `w`
/// wide, each with a leading icon and a trailing chevron. Returns the
/// box's height.
fn list_box(ui: &mut Ui, (x, y): (f32, f32), w: f32, rows: &[ListRow]) -> f32 {
    let h = rows_h(rows);
    crate::nav::screen::loaded_surface(ui, x, y, w, h);
    list_rows(ui, (x, y), w, rows);
    h
}

/// The height of one row of the box.
fn row_h(r: &ListRow) -> f32 {
    if r.sub.is_empty() { LIST_1 } else { LIST_2 }
}

/// The height of a box holding `rows`.
fn rows_h(rows: &[ListRow]) -> f32 {
    rows.iter().map(row_h).sum::<f32>() + 2.0 * GAP_SMALL
}

/// How many of `keys` key rows (two lines each) the box shows: up to `cap`,
/// and only as many as `avail` holds besides the other rows' `rest_h`.
fn keys_fitting(avail: f32, rest_h: f32, keys: usize, cap: usize) -> usize {
    let room = ((avail - rest_h) / LIST_2).floor().max(0.0) as usize;
    keys.min(cap).min(room)
}

/// The gap between Home's search row and its card: `HOME_CARD_GAP_TALL`
/// from a window `HOME_CARD_TALL_FROM` high, `HOME_CARD_GAP` under it.
fn card_gap(window: Option<f32>) -> f32 {
    match window {
        Some(h) if h >= HOME_CARD_TALL_FROM => HOME_CARD_GAP_TALL,
        _ => HOME_CARD_GAP,
    }
}

/// `rows` from the top of a box at `(x, y)`, `w` wide: the rows alone.
fn list_rows(ui: &mut Ui, (x, y): (f32, f32), w: f32, rows: &[ListRow]) {
    let mut cy = y + GAP_SMALL;
    let text_x = x + LIST_PAD + ICON_SMALL + LIST_PAD;
    let chevron_x = x + w - LIST_PAD - ICON_SMALL;
    for r in rows {
        let rh = row_h(r);
        ui.press(
            Shape::new(x + GAP_SMALL, cy, w - 2.0 * GAP_SMALL, rh, SHAPE_M),
            Role::OnSurface.paint(),
            r.action,
        );
        ui.icon(
            x + LIST_PAD,
            cy + (rh - ICON_SMALL) / 2.0,
            ICON_SMALL,
            r.icon,
            NAV_ICON + GAP_SMALL,
            if r.icon == Icon::Warning {
                Role::Warning
            } else {
                Role::OnSurfaceVariant
            },
        );
        ui.icon(
            chevron_x,
            cy + (rh - ICON_SMALL) / 2.0,
            ICON_SMALL,
            Icon::ChevronRight,
            NAV_ICON + GAP_SMALL,
            Role::OnSurfaceVariant,
        );
        let mut right = chevron_x - GRID;
        if !r.trailing.is_empty() {
            let tone = if r.accent {
                Role::Primary
            } else {
                Role::OnSurfaceVariant
            };
            let tw = ui.measure_as(BODY_M, &r.trailing);
            ui.text_right_as((right, cy, rh), BODY_M, tone, &r.trailing);
            right -= tw + GRID;
        }
        let room = (right - text_x).max(0.0);
        let (head_y, head_h) = if r.sub.is_empty() {
            (cy, rh)
        } else {
            let line = ui.line_as(BODY_L);
            (cy + (rh - line - ui.line_as(BODY_M)) / 2.0, line)
        };
        let head = ui.fit_as(BODY_L, &r.head, room);
        ui.text_mid_as((text_x, head_y, head_h), BODY_L, Role::OnSurface, &head);
        if !r.sub.is_empty() {
            let line = ui.line_as(BODY_M);
            let sub = ui.fit_as(BODY_M, &r.sub, (right - text_x).max(0.0));
            ui.text_mid_as(
                (text_x, head_y + head_h, line),
                BODY_M,
                Role::OnSurfaceVariant,
                &sub,
            );
        }
        cy += rh;
    }
}

/// The wallets' rows (`docs/HOME.md` §3), the ones that need something and
/// the rest: one for each wallet that needs something now, then "All wallets" opening Wallets, then Backups once a
/// wallet is known.
fn wallet_rows(app: &View, lead: Option<&CardLead>) -> (Vec<ListRow>, Vec<ListRow>) {
    let rows: Vec<ListRow> = home_wallets(app)
        .into_iter()
        .filter(|c| c.needs && !lead.is_some_and(|l| l.wallets.contains(&c.index)))
        .map(|c| ListRow {
            icon: Icon::Warning,
            head: c.name,
            sub: c.line,
            trailing: String::new(),
            accent: false,
            action: Action::OpenWallet(c.index),
        })
        .collect();
    let mut rest = Vec::new();
    // Backups only while something is not backed up.
    let (_, without) = app.backup_counts();
    if without > 0 {
        rest.push(ListRow {
            icon: Icon::Shield,
            head: EN.home_ways_backups.to_string(),
            sub: fill(EN.home_ways_not_backed, &[&without.to_string()]),
            trailing: String::new(),
            accent: false,
            action: Action::Nav(Screen::Backups),
        });
    }
    (rows, rest)
}

/// The box's first rows (`docs/HOME.md` §3): each flow in progress, its name
/// and its open step under it, resuming it, and while files wait to be
/// written, their count, opening Files.
fn going_rows(app: &View, lead: Option<&CardLead>) -> Vec<ListRow> {
    let mut rows: Vec<ListRow> = app
        .flows_in_progress()
        .iter()
        .filter(|r| !lead.is_some_and(|l| l.going == Some(r.going)))
        .map(|r| ListRow {
            icon: match r.tab {
                Screen::Vaults => Icon::Lock,
                Screen::Catalog => Icon::Tools,
                _ => Icon::Wallet,
            },
            head: r.name.to_string(),
            sub: r.step.to_string(),
            trailing: String::new(),
            accent: false,
            action: Action::Resume(r.going),
        })
        .collect();
    if !app.outbox.is_empty() {
        rows.push(ListRow {
            icon: Icon::File,
            head: app.files_count().to_string(),
            sub: String::new(),
            trailing: String::new(),
            accent: false,
            action: Action::Nav(Screen::Files),
        });
    }
    rows
}

/// The keys' rows (`docs/HOME.md` §3): one for each key no wallet uses,
/// offering to make a wallet from it, then "All keys" opening Explore.
fn key_rows(app: &View) -> (Vec<ListRow>, Option<ListRow>) {
    let n = app.session.keys.all().count();
    if n == 0 {
        return (Vec::new(), None);
    }
    let loose = app.session.loose_keys();
    let rows: Vec<ListRow> = app
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
            ListRow {
                icon: Icon::Keys,
                head: String::from(&*fp),
                sub,
                trailing: EN.home_btn_make_a_wallet.to_string(),
                accent: true,
                action: Action::KeyWallet(k.fingerprint.0, 1),
            }
        })
        .collect();
    let all = ListRow {
        icon: Icon::Keys,
        head: EN.home_all_keys.to_string(),
        sub: String::new(),
        trailing: n.to_string(),
        accent: false,
        action: Action::Explore,
    };
    (rows, Some(all))
}

/// What is loaded as one section of the menu page (`docs/HOME.md` §3): the
/// window's card of what needs you and the keys here, or on the small
/// panel a tinted box of list rows, first one for each wallet that needs
/// something and each key no wallet uses, then All wallets, Backups and
/// All keys. The wide layout and the rail's windows both draw it.
pub(crate) fn loaded_sections(
    app: &View,
    window: Option<f32>,
    lead: Option<CardLead>,
) -> Vec<Section<'static>> {
    let mut rest = going_rows(app, lead.as_ref());
    let (wallets_first, wallets) = wallet_rows(app, lead.as_ref());
    rest.extend(wallets_first);
    rest.extend(wallets);
    let needs = rest.clone();
    let (keys, all_keys) = key_rows(app);
    rest.extend(all_keys.clone());
    if rest.is_empty() && keys.is_empty() && lead.is_none() {
        return Vec::new();
    }
    let offset = app.scroll.home_box_offset();
    vec![Section::block(None, move |ui, (x, y), w| {
        if !ui.compact {
            let parts = CardRows {
                needs: &needs,
                keys: &keys,
                all_keys: all_keys.as_ref(),
            };
            return home_card(ui, (x, y), w, (window, offset), &parts, lead.as_ref());
        }
        let bw = w;
        let bx = x;
        section_label(ui, bx, y, EN.home_box_heading);
        let head = ui.line_as(TITLE_S) + GRID;
        let by = y + head;
        let lead_h = lead
            .as_ref()
            .map_or(0.0, |l| lead_card(ui, &l.card, (bx, by), bw, true) + GRID);
        let by = by + lead_h;
        let head = head + lead_h;
        let rest_h = rows_h(&rest);
        // Without a window height the page scrolls as a whole: up to three
        // keys. With one, the box's room runs to a margin above the foot;
        // where the fixed part leaves less than two rows the page scrolls
        // as one.
        let avail = window.map_or(f32::INFINITY, |h| (h - by - MARGIN_WIDE).floor());
        let n = keys_fitting(avail, rest_h, keys.len(), HOME_BOX_KEYS);
        let all_h = rest_h + n as f32 * LIST_2;
        let mut rows: Vec<ListRow> = rest.clone();
        rows.extend(keys.iter().take(n).cloned());
        if all_h <= avail || avail < HOME_BOX_MIN_ROWS * LIST_2 {
            return head + list_box(ui, (bx, by), bw, &rows);
        }
        // Scrolls within itself: the surface stays, the rows move.
        crate::nav::screen::loaded_surface(ui, bx, by, bw, avail);
        let clip = ui.rect(bx, by, bw, avail);
        let mark = ui.presses_mark();
        let open = ui.region_open(Region::scrolled(Slot::HomeBox, clip, offset));
        list_rows(ui, (bx, by), bw, &rows);
        ui.clip_presses_since(mark, clip);
        ui.region_close(open, (all_h - avail).max(0.0));
        head + avail
    })]
}

/// The rows of Home's card, by meaning: what needs you, the keys no wallet
/// uses and All keys.
struct CardRows<'a> {
    needs: &'a [ListRow],
    keys: &'a [ListRow],
    all_keys: Option<&'a ListRow>,
}

/// The card's surface: a raised card of the surface-container-low fill with
/// the Scan button's shadow.
fn card_surface(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.shadow(x, y, w, h, SHAPE_M, ELEVATION_FAB);
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
}

/// Home's card in the window `w` wide from `x`, `HOME_CARD_GAP` under what
/// is above it: its heading at the top left, then the rows in two columns
/// (what needs you, the keys here) from `HOME_CARD_TWO_COLS`, or in one
/// list in `docs/HOME.md` §3's order below it. The card is as wide as the
/// menu pages and centred. It scrolls within itself, as one, when the
/// window is short. Returns its height with the gap.
fn home_card(
    ui: &mut Ui,
    (x, y): (f32, f32),
    w: f32,
    (window, offset): (Option<f32>, f32),
    parts: &CardRows,
    lead: Option<&CardLead>,
) -> f32 {
    let cw = w;
    let cx = x + (w - cw) / 2.0;
    let gap = card_gap(window);
    let top = y + gap;
    let head_h = ui.line_as(HOME_CARD_HEAD);
    let inner = cw - 2.0 * PAD;
    let lead_h = lead.map_or(0.0, |_| HOME_LEAD_H + GRID);
    let rows_y = top + PAD + head_h + GRID + lead_h;
    let two = cw >= HOME_CARD_TWO_COLS;
    let avail = window.map_or(f32::INFINITY, |h| (h - rows_y - MARGIN_WIDE).floor());
    let (cols, widths): (Vec<Vec<ListRow>>, Vec<f32>) = if two {
        let all: Vec<ListRow> = parts.all_keys.cloned().into_iter().collect();
        let n = keys_fitting(
            avail,
            rows_h(&all),
            parts.keys.len().min(HOME_CARD_KEYS),
            HOME_CARD_KEYS,
        );
        let mut right: Vec<ListRow> = parts.keys.iter().take(n).cloned().collect();
        right.extend(all);
        let col_w = (inner - MENU_GAP) / 2.0;
        (vec![parts.needs.to_vec(), right], vec![col_w, col_w])
    } else {
        let mut one = parts.needs.to_vec();
        one.extend(parts.all_keys.cloned());
        let n = keys_fitting(avail, rows_h(&one), parts.keys.len(), HOME_BOX_KEYS);
        one.extend(parts.keys.iter().take(n).cloned());
        (vec![one], vec![inner])
    };
    let rows_h_max = cols
        .iter()
        .filter(|c| !c.is_empty())
        .map(|c| rows_h(c))
        .fold(0.0, f32::max);
    let scrolls = rows_h_max > avail && avail >= HOME_BOX_MIN_ROWS * LIST_2;
    let shown = if scrolls { avail } else { rows_h_max };
    let card_h = rows_y - top + shown + GRID;
    card_surface(ui, (cx, top, cw, card_h));
    ui.text_as(
        cx + PAD + LIST_PAD,
        top + PAD,
        HOME_CARD_HEAD,
        Role::OnSurface,
        EN.home_box_heading,
    );
    if let Some(l) = lead {
        lead_card(ui, &l.card, (cx + PAD, rows_y - lead_h), inner, true);
    }
    let draw_cols = |ui: &mut Ui| {
        let mut colx = cx + PAD;
        for (c, cw_) in cols.iter().zip(&widths) {
            list_rows(ui, (colx, rows_y), *cw_, c);
            colx += cw_ + MENU_GAP;
        }
    };
    if scrolls {
        let clip = ui.rect(cx, rows_y, cw, shown);
        let mark = ui.presses_mark();
        let open = ui.region_open(Region::scrolled(Slot::HomeBox, clip, offset));
        draw_cols(ui);
        ui.clip_presses_since(mark, clip);
        ui.region_close(open, (rows_h_max - avail).max(0.0));
    } else {
        draw_cols(ui);
    }
    gap + card_h
}

/// The three first-run actions while nothing is loaded, in their order:
/// their icon, their heading and their press (`docs/HOME.md` §5).
pub(crate) fn first_actions() -> [(Glyph, &'static str, Action); 3] {
    [
        (
            Glyph::Icon(Icon::Flag),
            EN.home_ways_make_a_new,
            Action::CreateWallet,
        ),
        (
            Glyph::Icon(Icon::Wallet),
            EN.home_ways_restore_wallet,
            Action::RestoreWallet,
        ),
        (
            Glyph::Icon(Icon::Sign),
            EN.home_ways_spend_backup,
            Action::Nav(Screen::SpendBackup),
        ),
    ]
}

/// The most lines any first-run heading takes at `role` in a column
/// `col` wide, with `text_x` before the heading and `inset` after it.
pub(crate) fn actions_lines(
    ui: &Ui,
    role: faraday_draw::tokens::TypeRole,
    col: f32,
    (text_x, inset): (f32, f32),
) -> usize {
    let room = (col - text_x - inset).max(0.0);
    first_actions()
        .iter()
        .map(|(_, label, _)| ui.wrap_lines_as(role, label, room))
        .max()
        .unwrap_or(1)
}

/// The width of one of three equal columns `width` wide with `gap`
/// between them.
pub(crate) fn actions_col(width: f32, gap: f32) -> f32 {
    (width - 2.0 * gap) / 3.0
}

/// Whether the first-run actions stand on one row `width` wide with `gap`
/// between them: every heading wraps within two lines at `role`, with
/// `text_x` before each heading and `inset` after it. The rail's windows
/// choose a row or a stack with it; the wide layout always has the row.
pub(crate) fn actions_fit_a_row(
    ui: &Ui,
    role: faraday_draw::tokens::TypeRole,
    (width, gap): (f32, f32),
    insets: (f32, f32),
) -> bool {
    actions_lines(ui, role, actions_col(width, gap), insets) <= 2
}

/// The height of one first-run button whose longest heading takes `lines`
/// lines of `role`: `base`, grown where the lines need more, with `pad`
/// above and below. All three buttons share it.
pub(crate) fn action_h(
    ui: &Ui,
    role: faraday_draw::tokens::TypeRole,
    lines: usize,
    (base, pad): (f32, f32),
) -> f32 {
    base.max(lines as f32 * ui.line_as(role) + 2.0 * pad)
}

/// Home's top row from `(x, y)`, `w` wide: the search bar, taking what
/// the three tonal buttons (icon and short label, as tall as the bar)
/// leave, and the buttons after it, each as wide as its icon and label
/// need. Where the bar would be left under `HOME_SEARCH_MIN` the buttons
/// stand on a second row under it, three equal columns across `w`.
/// Returns the height it takes, the gap under it counted (`docs/HOME.md`
/// §5).
pub(super) fn top_row(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    use super::catalog::Bar;
    use faraday_draw::tokens::{GAP, SEARCH_BAR_H};
    let ways = [
        (Icon::Flag, EN.home_ways_new_short, Action::CreateWallet),
        (
            Icon::Wallet,
            EN.home_ways_restore_short,
            Action::RestoreWallet,
        ),
        (
            Icon::Sign,
            EN.home_ways_spend_short,
            Action::Nav(Screen::SpendBackup),
        ),
    ];
    let widths = ways.map(|(_, label, _)| ui.button_icon_width(label, Style::Tonal));
    let beside = widths.iter().map(|bw| MENU_GAP + bw).sum::<f32>();
    let one_row = w - beside >= HOME_SEARCH_MIN;
    let sw = if one_row { w - beside } else { w };
    super::catalog::home_search_bar(app, ui, (x, y, sw), None, Bar::Home);
    let (mut bx, by, step) = if one_row {
        (x + sw + MENU_GAP, y, None)
    } else {
        let col = (w - 2.0 * MENU_GAP) / 3.0;
        (x, y + SEARCH_BAR_H + MENU_GAP, Some(col))
    };
    for ((icon, label, action), own) in ways.into_iter().zip(widths) {
        let bw = step.unwrap_or(own);
        ui.button_icon(
            (bx, by, Some(bw), SEARCH_BAR_H),
            icon,
            label,
            Style::Tonal,
            action,
        );
        bx += bw + MENU_GAP;
    }
    (if one_row {
        SEARCH_BAR_H
    } else {
        2.0 * SEARCH_BAR_H + MENU_GAP
    }) + GAP
}

/// The lead's card at `(x, y)`, `width` wide:
/// the job's icon, its line and the line under it, and at its end the one
/// filled button (`docs/HOME.md` §2). Returns its height. The card is not
/// a press; its button is.
fn lead_card(ui: &mut Ui, card: &LeadCard, (x, y): (f32, f32), width: f32, inside: bool) -> f32 {
    let lead_h = HOME_LEAD_H;
    let lead_w = width;
    // Inside Home's card the lead is the card's featured first item, in
    // the secondary container, the warning container while it cannot be
    // done yet.
    let on = match (inside, card.enabled) {
        (false, _) => Role::OnSurface,
        (true, true) => Role::OnSecondaryContainer,
        (true, false) => Role::OnWarningContainer,
    };
    if inside {
        let bg = if card.enabled {
            Role::SecondaryContainer
        } else {
            Role::WarningContainer
        };
        ui.fill(x, y, lead_w, lead_h, SHAPE_M, bg);
    } else {
        ui.fill(x, y, lead_w, lead_h, SHAPE_M, Role::SurfaceContainerLow);
        ui.stroke(x, y, lead_w, lead_h, SHAPE_M, Role::OutlineVariant);
    }
    let (ibg, ifg) = if inside {
        (None, on.paint())
    } else if card.enabled {
        (
            Some(Role::SecondaryContainer.paint()),
            Role::OnSecondaryContainer.paint(),
        )
    } else {
        (
            Some(Role::SurfaceContainerHighest.paint()),
            Role::OnSurfaceVariant.paint(),
        )
    };
    if let Some(ibg) = ibg {
        ui.fill(
            x + HOME_LEAD_INSET,
            y + (lead_h - HOME_LEAD_ICON) / 2.0,
            HOME_LEAD_ICON,
            HOME_LEAD_ICON,
            SHAPE_M,
            ibg,
        );
    }
    ui.icon(
        x + HOME_LEAD_INSET,
        y + (lead_h - HOME_LEAD_ICON) / 2.0,
        HOME_LEAD_ICON,
        card.icon,
        HOME_LEAD_INSET,
        ifg,
    );
    // A card with no button is the press itself.
    let bw = if card.button.is_empty() {
        ui.press(
            Shape::new(x, y, lead_w, lead_h, SHAPE_M),
            Role::OnSurface.paint(),
            card.action,
        );
        0.0
    } else {
        let style = if card.enabled {
            Style::Primary
        } else {
            Style::PrimaryOff
        };
        let bw = ui.button_width(card.button, style);
        ui.button(
            x + lead_w - HOME_LEAD_INSET - bw,
            y + (lead_h - BUTTON_H) / 2.0,
            Some(bw),
            BUTTON_H,
            card.button,
            style,
            card.action,
        );
        bw
    };
    let room = lead_w - HOME_LEAD_TEXT_X - bw - 2.0 * HOME_LEAD_INSET;
    let line = ui.fit_as(TITLE_M, &card.line, room);
    ui.text_as(
        x + HOME_LEAD_TEXT_X,
        y + HOME_LEAD_INSET,
        TITLE_M,
        on,
        &line,
    );
    if !card.under.is_empty() {
        let under = ui.fit_as(BODY_M, &card.under, room);
        let tone = if inside {
            on
        } else if card.enabled {
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

/// Whether the page down to Home's search bar stays still while a search
/// is typed, only the results under it scrolling: where `above` (what
/// stands over the bar, units) and the bar leave `window` room for
/// `HOME_RESULTS_ROWS` rows of `row` under them. Where they do not, the
/// whole page scrolls as one, the same content in the same order. Only the
/// phone-size window chooses: the wide layout and the rail's windows always
/// keep the bar still.
pub(crate) fn home_pinned(above: f32, window: f32, row: f32) -> bool {
    use faraday_draw::tokens::{HOME_RESULTS_ROWS, SEARCH_BAR_H};
    above + SEARCH_BAR_H + HOME_RESULTS_ROWS * row <= window
}

/// Home on the wide layout, named by the top app bar (`docs/HOME.md`),
/// drawn by the menu page container: the search bar first, centred and
/// standing still, and under it the lead, the three first actions as
/// blocks across the page, the loaded wallets beside Backups and the Keys
/// section as tiles, scrolling. While something is typed its results take
/// the page.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use super::catalog::Bar;
    let sections = loaded_sections(app, Some(h), card_lead(app, false));
    let page = Page {
        name: EN.home_home,
        back: false,
        search: Some(Bar::Home),
        most_columns: usize::MAX,
        fixed_columns: None,
        above: None,
        bar_still: false,
        ways: true,
    };
    menu_page::draw(app, ui, (x0, cw, h), &page, &sections);
}
