//! A row's details in the page's second pane (`docs/WORKSPACE.md` §7 point
//! 1): "What you can do" first (the group a wallet's page draws, one place:
//! `wallets::actions`), then the facts. A wallet's details are its page on
//! Wallets, the same drawing code. Reads `row_open` for what is open; works
//! nothing out beyond the one file's shared list.

use super::wallets::actions::{self, Offer};
use crate::Action;
use crate::drawing::View;
use crate::row_open::Opened;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Region, Ui};
use faraday_draw::tokens::{BODY_L, BODY_M, ELEVATION_SHEET, GRID, PAD, SHAPE_M, TITLE_S};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::section_label;

/// The open item's name, for the top bar; none where no row is open.
pub(crate) fn title(app: &View) -> Option<String> {
    Some(match crate::row_open::opened(app)? {
        Opened::Key(fp) => {
            let k = app.session.keys.all().find(|k| k.fingerprint.0 == fp)?;
            String::from(&*crate::wallet::fp_text(k.fingerprint))
        }
        Opened::Wallet(i) => app.session.wallets.get(i)?.name.to_string(),
        Opened::File(k) => app.inbox.get(k)?.name.to_string(),
    })
}

/// The page's top bar: the page's name, or, with a row open, its back
/// arrow (the page's name, a press returning the page) and the item's name.
/// Returns the bar's height.
pub(crate) fn bar(app: &View, ui: &mut Ui, page: &str) -> f32 {
    bar_when(app, ui, page, true)
}

/// The same bar where the details stand only if `stands`: a page whose room
/// may not hold them (Files, with the guide pane taking room) passes the
/// predicate that draws them, so the arrow and the name come with the
/// details and never without.
pub(crate) fn bar_when(app: &View, ui: &mut Ui, page: &str, stands: bool) -> f32 {
    match title(app).filter(|_| stands) {
        Some(name) => super::bar_head(ui, Some((page, Action::RowClose)), &name),
        None => super::bar_head(ui, None, page),
    }
}

/// What key `fp` offers: Explore it, and where no wallet uses it, the
/// ways on that take it (Make a wallet from this key, Start a multisig
/// wallet with this key, Restore).
fn key_offers(app: &View, fp: [u8; 4]) -> Vec<Offer> {
    let mut list = vec![actions::offer(
        EN.explore_explore,
        Some(Icon::Explore),
        Action::ExploreKey(fp),
    )];
    if app.loose_keys().0.iter().any(|k| k.0 == fp) {
        list.push(actions::offer(
            super::parts::FROM_KEY,
            Some(Icon::Wallet),
            Action::KeyWallet(fp, 1),
        ));
        list.push(actions::offer(
            EN.loose_card_start_multisig,
            Some(Icon::Add),
            Action::KeyWallet(fp, 2),
        ));
        list.push(actions::offer(
            EN.loose_card_restore,
            offer_icon(Action::RestoreWallet),
            Action::RestoreWallet,
        ));
    }
    list
}

/// The icon an action carries elsewhere (a wallet's group, the first
/// actions, Wallets' ways in, Home's rows); none where it carries none.
fn offer_icon(action: Action) -> Option<Icon> {
    use crate::file_menu::FilePress;
    use crate::vault::VaultAction;
    match action {
        Action::StartSpend(_) | Action::File(FilePress::SignMessage(_)) => Some(Icon::Sign),
        Action::SpendBackup(crate::spend_backup::SpendBackupAction::FromFile(_)) => {
            Some(Icon::Sign)
        }
        Action::SpendBackup(crate::spend_backup::SpendBackupAction::FromSeedFile(_)) => {
            Some(Icon::Sign)
        }
        Action::File(FilePress::CheckAddress(_)) => Some(Icon::Verify),
        Action::SeedCheck(crate::seed_check::SeedAction::File(_)) => Some(Icon::Checklist),
        Action::Backup(_) => Some(Icon::Shield),
        Action::CreateWallet => Some(Icon::Flag),
        Action::RestoreWallet | Action::RestoreSeedFile(_) => Some(Icon::Wallet),
        Action::Vault(VaultAction::Open(_)) => Some(Icon::Lock),
        _ => None,
    }
}

/// What held file `k` offers: its shared list.
fn file_offers(app: &View, k: usize) -> Vec<Offer> {
    let found = std::cell::OnceCell::new();
    let loaded_at = |k: usize| found.get_or_init(|| app.inbox_found()).loaded_at_file(k);
    crate::items::offer(app, k, &|seen| app.spends_from(seen), &loaded_at)
        .list
        .into_iter()
        .map(|(label, action)| actions::offer(&label, offer_icon(action), action))
        .collect()
}

/// The group, then the facts, from `(x, y)`, `w` wide. Returns the height.
fn body(app: &View, ui: &mut Ui, opened: Opened, (x, y): (f32, f32), w: f32) -> f32 {
    let (offers, facts) = match opened {
        Opened::Key(fp) => (
            key_offers(app, fp),
            super::session_pane::key_facts(app, fp).map(|(head, sub)| (head, sub, true)),
        ),
        Opened::File(k) => (
            file_offers(app, k),
            // As Files' own row says it: the name, then its kind and size.
            app.inbox.get(k).map(|it| {
                let line = super::kind_line(it.kind, it.bytes.len());
                (it.name.to_string(), String::from(&*line), false)
            }),
        ),
        Opened::Wallet(_) => return 0.0,
    };
    let mut cy = y;
    if !offers.is_empty() {
        cy += actions::draw(ui, (x, cy, w), &offers) + faraday_draw::tokens::ACTIONS_UNDER;
    }
    if let Some((head, sub, mono)) = facts {
        section_label(ui, x, cy, EN.row_details_facts);
        cy += ui.line_as(TITLE_S) + GRID;
        let style = if mono { BODY_L.mono() } else { BODY_L };
        cy += ui.wrap_as((x, cy, w), style, Role::OnSurface, &head) + GRID;
        if !sub.is_empty() {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurfaceVariant, &sub);
        }
    }
    cy - y
}

/// What the open item's details take inside their padding, `w` wide: the
/// room the stack asks for under This session's card. A wallet's page is
/// measured from its own drawing, at the card's width `w` plus the padding
/// the stack leaves, and less its own padding that `placed` adds.
pub(crate) fn content_h(app: &View, ui: &mut Ui, w: f32) -> f32 {
    match crate::row_open::opened(app) {
        None => 0.0,
        Some(Opened::Wallet(_)) => {
            // The page is a card that takes its whole room: its need less
            // the padding the stack adds around a tile's content.
            let need = crate::menu::measure(ui, &mut |ui| {
                super::wallets::wallet_page(
                    app,
                    ui,
                    (0.0, w + 2.0 * PAD),
                    (0.0, 0.0),
                    (false, true),
                )
            });
            (need - 2.0 * PAD).max(0.0)
        }
        Some(opened) => crate::menu::measure(ui, &mut |ui| body(app, ui, opened, (0.0, 0.0), w)),
    }
}

/// The open item's details in `(x, y, w, h)`, the tile's place. False,
/// drawing nothing, where no row is open.
pub(crate) fn draw(app: &View, ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) -> bool {
    let Some(opened) = crate::row_open::opened(app) else {
        return false;
    };
    if let Opened::Wallet(_) = opened {
        super::wallets::wallet_page(app, ui, (x, w), (y, h), (false, false));
        return true;
    }
    ui.shadow(x, y, w, h, SHAPE_M, ELEVATION_SHEET);
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, w, h, SHAPE_M, Role::Outline);
    }
    let inner = w - 2.0 * PAD;
    let need =
        crate::menu::measure(ui, &mut |ui| body(app, ui, opened, (0.0, 0.0), inner)) + 2.0 * PAD;
    let clip = ui.rect(x, y, w, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    body(app, ui, opened, (x + PAD, y + PAD), inner);
    app.scroll.content_h().set(need);
    ui.region_close(open, (need - h).max(0.0));
    true
}
