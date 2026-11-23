//! `Screen::Home`.

use super::menu_page::{self, Page};
use crate::drawing::View;
use crate::in_progress::Going;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Glyph, Ui};
use crate::ui::{Shape, Style};
use crate::{Action, Screen};
use faraday_draw::tokens::{BODY_M, BUTTON_H, SHAPE_M, TITLE_M};
use faraday_draw::tokens::{
    HOME_LEAD_H, HOME_LEAD_ICON, HOME_LEAD_INSET, HOME_LEAD_SUB_Y, HOME_LEAD_TEXT_X,
    HOME_SEARCH_MIN, MENU_GAP,
};
use faraday_draw::widgets::Icon;

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
    home_jobs(app, false)
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
/// With `all`, every job the session pane's Waiting group shows
/// (`docs/SESSION-PANE.md`): one for each waiting PSBT and each locked vault
/// file, the last write kept with keys and wallets loaded; without it the
/// one lead the one-column Home has.
fn home_jobs(app: &View, all: bool) -> Vec<Job> {
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
    let psbts = waiting_psbts(app);
    for &i in psbts.iter().take(if all { usize::MAX } else { 1 }) {
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
    if (fresh || all)
        && let Some(r) = app.visit.receipt().as_ref()
    {
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
    // A locked vault file, and nothing loaded (every one with `all`).
    if fresh || all {
        let locked = app
            .vault_files()
            .iter()
            .enumerate()
            .filter(|(_, f)| f.open.is_none())
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        for &i in locked.iter().take(if all { usize::MAX } else { 1 }) {
            v.push(locked_job(app, i));
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

/// The PSBTs in Files that wait to be signed, fewest signatures first (the
/// first is [`View::lead_psbt`]'s).
fn waiting_psbts(app: &View) -> Vec<usize> {
    let mut order: Vec<(usize, usize)> = app
        .inbox
        .iter()
        .enumerate()
        .filter_map(|(i, it)| Some((it.psbt()?.sigs, i)))
        .collect();
    order.sort_unstable();
    order.into_iter().map(|(_, i)| i).collect()
}

/// The job for the locked vault file `i`: Unlock with what it held when
/// last seen open, or why it cannot be unlocked yet.
fn locked_job(app: &View, i: usize) -> Job {
    use crate::vault::VaultAction as V;
    let files = app.vault_files();
    let f = &files[i];
    // By its name, with what it held when last seen open (§3.4), when it
    // was.
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
    if app.stick.is_empty() {
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
    }
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
    if let Some(j) = home_jobs(app, false).into_iter().find(|j| !hidden(j.going)) {
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

impl CardLead {
    /// The lead as the card draws it.
    pub(super) fn card(&self) -> &LeadCard {
        &self.card
    }

    /// The flow it carries on.
    pub(super) fn going(&self) -> Option<Going> {
        self.going
    }

    /// The wallets whose rows it repeats.
    pub(super) fn wallets(&self) -> &[usize] {
        &self.wallets
    }
}

/// Every job the session pane's Waiting group shows, with the flow each
/// carries on (`docs/SESSION-PANE.md`): all waiting PSBTs and locked vault
/// files, the last write with keys and wallets loaded too.
pub(super) fn pane_jobs(app: &View) -> Vec<(Option<Going>, LeadCard)> {
    home_jobs(app, true)
        .into_iter()
        .map(|j| (j.going, j.card))
        .collect()
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
pub(super) fn lead_card(
    ui: &mut Ui,
    card: &LeadCard,
    (x, y): (f32, f32),
    width: f32,
    inside: bool,
) -> f32 {
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
    let sections = super::session_pane::loaded_sections(app, Some(h), card_lead(app, false));
    let page = Page {
        name: EN.home_home,
        back: false,
        search: Some(Bar::Home),
        most_columns: usize::MAX,
        fixed_columns: None,
        above: None,
        bar_still: false,
        ways: true,
        pane: None,
    };
    menu_page::draw(app, ui, (x0, cw, h), &page, &sections);
}
