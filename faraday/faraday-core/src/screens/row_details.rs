//! A row's details in the page's second pane (`docs/WORKSPACE.md` §7 point
//! 1): "What you can do" first (the group a wallet's page draws, one place:
//! `wallets::actions`), then the facts. A wallet's details are its page on
//! Wallets, the same drawing code. Reads `row_open` for what is open; works
//! nothing out beyond the one file's shared list.

use super::wallets::actions::{self, Offer};
use crate::drawing::View;
use crate::row_open::Opened;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Region, Ui};
use crate::{Action, KeyTool};
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

/// What key `fp` offers, every action that can open with it picked, whether
/// or not a wallet uses it: Explore it, derive from it, Convert it, make a
/// Krux code of its words (where one can be made), search a vanity address,
/// its silent payments, its Lightning node key (all in the order of
/// Tools) and Sign a message with a single-key wallet of it (not while the
/// key waits for its backup), Make a wallet from this key, Start a multisig wallet with this
/// key, Restore with it, Add to vault where the open vault can take it,
/// check a copy of its words (a key with words) and show its account xpub or
/// put it in a file (a key in a wallet). A tool that would not take the key
/// is left out.
fn key_offers(app: &View, fp: [u8; 4]) -> Vec<Offer> {
    let convert = Action::Convert(crate::convert::ConvertAction::OpenOn(fp));
    let tool = |label, tool| actions::offer(label, None, Action::KeyTool(tool, fp));
    let info = app
        .session
        .keys
        .position_of(fp)
        .and_then(|k| app.session.keys.info(k));
    let mut offers = vec![
        actions::offer(
            EN.explore_explore,
            Some(Icon::Explore),
            Action::ExploreKey(fp),
        ),
        tool(EN.catalog_child_seeds, KeyTool::Derive),
        actions::offer(EN.convert_way_in, offer_icon(convert), convert),
    ];
    if info.is_some_and(|k| k.kef_offered()) {
        let kef = Action::KefTool(crate::kef_tool::KefToolAction::OpenOn(fp));
        offers.push(actions::offer(EN.catalog_krux_code, None, kef));
    }
    let vanity = Action::Vanity(crate::vanity::VanityAction::OpenOn(fp));
    offers.push(actions::offer(EN.catalog_vanity_address, None, vanity));
    // A key waiting for its backup has its silent payments address held.
    if !app.key_held(faraday_bip::keys::Fingerprint(fp)) {
        offers.push(tool(EN.catalog_silent_payments, KeyTool::Silent));
    }
    // The node key is made from BIP-39 words.
    if info.is_some_and(|k| k.has_words()) {
        offers.push(tool(EN.catalog_lightning_node, KeyTool::Lightning));
    }
    // One wallet of the key: it is picked. Several: they alone are offered.
    let sign = match app.message_offered_of(fp)[..] {
        [] => None,
        [wallet] => Some(Action::SignMessageWith(wallet)),
        _ => Some(Action::SignMessageKey(fp)),
    };
    if let Some(sign) = sign {
        offers.push(actions::offer(
            EN.catalog_sign_a_message,
            Some(Icon::Sign),
            sign,
        ));
    }
    offers.extend([
        actions::offer(
            super::parts::FROM_KEY,
            Some(Icon::Wallet),
            Action::KeyWallet(fp, 1),
        ),
        actions::offer(
            EN.loose_card_start_multisig,
            Some(Icon::Add),
            Action::KeyWallet(fp, 2),
        ),
        actions::offer(
            EN.loose_card_restore,
            offer_icon(Action::RestoreWallet),
            Action::RestoreWith(fp),
        ),
    ]);
    if let Some(save) = super::session_pane::key_save(app, fp) {
        offers.push(actions::offer(
            EN.add_to_vault_button,
            offer_icon(save),
            save,
        ));
    }
    if info.is_some_and(|k| k.has_words()) {
        let check = Action::CheckKeyCopy(fp);
        offers.push(actions::offer(EN.glance_sheet_scan_my, None, check));
    }
    if let Some((w, slot)) = crate::screens::session_pane::rows::account_key(app, fp) {
        let qr = Action::ShowCode(crate::Code::WalletKey(w, slot));
        offers.push(actions::offer(EN.glance_sheet_show_xpub, None, qr));
        let file = Action::WalletKeyOut(w, slot);
        offers.push(actions::offer(EN.glance_sheet_xpub_file, None, file));
    }
    offers
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
        Action::File(FilePress::CheckAddress(_)) | Action::AddressCheck(_) => Some(Icon::Verify),
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
    let enough = |k: usize| found.get_or_init(|| app.inbox_found()).shares_enough(k);
    crate::items::offer(app, k, &|seen| app.spends_from(seen), &loaded_at, &enough)
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
