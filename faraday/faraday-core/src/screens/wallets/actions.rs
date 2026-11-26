//! A wallet's "What you can do" group (`docs/WORKSPACE.md` §7): one
//! function names what a wallet offers, as data, and the card draws it
//! as a heading and one list row for each. The session pane's details
//! read the same list.

use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Shape, Ui};
use faraday_draw::tokens::{
    self, BODY_L, ICON_SMALL, LIST_1, LIST_2, LIST_PAD, LIST_PAD_V, SHAPE_L, TITLE_S,
};
use faraday_draw::widgets::Icon;

/// One thing a wallet offers: its words, its icon where it has one and
/// what a press does.
pub(crate) struct Offer {
    pub label: String,
    pub icon: Option<Icon>,
    pub action: Action,
    /// The press opens a page or a flow: the row ends in a chevron. One
    /// that acts at once has none.
    pub opens: bool,
    /// The press begins a dangerous act: the row is in the error tone.
    pub danger: bool,
}

/// Whether a press of `action` acts where it stands, with no page or flow
/// opening (F332).
fn acts_at_once(action: Action) -> bool {
    matches!(action, Action::RemoveWallet(_))
}

pub(crate) fn offer(label: &str, icon: Option<Icon>, action: Action) -> Offer {
    Offer {
        label: label.to_string(),
        icon,
        action,
        opens: !acts_at_once(action),
        danger: matches!(action, Action::RemoveWallet(_)),
    }
}

/// Add a key to loaded wallet `w` while it needs one: the slot of its
/// only empty key, else any. None for a wallet with enough keys here and
/// for a silent payments one. The pane row's press and the card's. A
/// lookup.
pub(crate) fn add_a_key(app: &View, w: usize) -> Option<Action> {
    let wl = app.session.wallets.get(w)?;
    if wl.policy().silent().is_some() || app.can_spend(w) {
        return None;
    }
    let slots = app.session.slots(wl);
    let mut empty = slots.iter().filter(|s| s.held_by.is_none());
    let only = match (empty.next(), empty.next()) {
        (Some(slot), None) => slot.fingerprint,
        _ => None,
    };
    Some(Action::Entry(only.map(|fp| fp.0)))
}

/// What wallet `w` offers, likeliest first: Add a key while it needs
/// the table's others in its order, then the
/// rest in the order the card has always had, Remove from session last.
/// `new_from`: New wallet from this one is not already on the card's
/// head. A lookup.
pub(crate) fn offers(app: &View, w: usize, new_from: bool) -> Vec<Offer> {
    let Some(wlt) = app.session.wallets.get(w) else {
        return Vec::new();
    };
    if wlt.policy().silent().is_some() {
        let mut list = vec![offer(
            EN.silent_wallet_card_address,
            Some(Icon::Wallet),
            Action::SWallet(w),
        )];
        // No code of a held address is made: no button for it.
        if !app.silent_wallet_held(w) {
            list.push(offer(
                EN.silent_wallet_card_show,
                Some(Icon::Qr),
                Action::QrWallet(w),
            ));
        }
        list.push(offer(
            EN.silent_wallet_card_remove,
            Some(Icon::Trash),
            Action::RemoveWallet(w),
        ));
        return list;
    }
    let (label, action) = app.spend_button(w);
    let first_icon = if matches!(action, Action::BackupFirst(_)) {
        Icon::Shield
    } else {
        Icon::Sign
    };
    let mut list = Vec::new();
    if let Some(add) = add_a_key(app, w) {
        list.push(offer(EN.wallets_add_a_key, Some(Icon::Add), add));
    }
    list.push(offer(&label, Some(first_icon), action));
    // A backup left part way is offered as such.
    let back_up = if app.wallet_backup_unfinished(w) {
        EN.backup_flow_continue
    } else {
        EN.wallets_back_up
    };
    list.push(offer(back_up, Some(Icon::Shield), Action::Backup(w)));
    list.push(offer(
        EN.wallets_check_an_address,
        Some(Icon::Verify),
        Action::AddressCheck(crate::address_check::AddressPress::Another),
    ));
    if app.message_offered().contains(&w) {
        list.push(offer(
            EN.wallets_sign_a_message,
            Some(Icon::Sign),
            Action::SignMessageWith(w),
        ));
    }
    list.push(offer(
        EN.wallets_show_wallet,
        Some(Icon::Qr),
        Action::QrWallet(w),
    ));
    if new_from {
        list.push(offer(
            EN.wallets_new_wallet_button,
            Some(Icon::Add),
            Action::NewWalletFrom(w),
        ));
    }
    if let Some(fp) = app
        .session
        .slots(wlt)
        .iter()
        .find(|s| s.held_by.is_some())
        .and_then(|s| s.fingerprint)
    {
        list.push(offer(
            EN.wallets_explore_its_keys,
            Some(Icon::Explore),
            Action::ExploreKey(fp.0),
        ));
    }
    list.push(offer(
        EN.wallets_remove_from_session,
        Some(Icon::Trash),
        Action::RemoveWallet(w),
    ));
    list
}

/// The group at (x, y), `w` wide: the heading, then a row for each
/// offer, a touch target tall, the whole row its press. The first row's
/// icon is in the accent the card's first button had. Returns its height.
pub(crate) fn draw(ui: &mut Ui, (x, y, w): (f32, f32, f32), list: &[Offer]) -> f32 {
    faraday_ui::parts::section_label(ui, x, y, EN.wallets_what_you_can_do);
    let mut cy = y + ui.line_as(TITLE_S) + tokens::GRID;
    // A group none of whose offers has an icon leaves no gap for one.
    let lead = if list.iter().any(|o| o.icon.is_some()) {
        2.0 * LIST_PAD + ICON_SMALL
    } else {
        LIST_PAD
    };
    let room = w - lead - LIST_PAD - ICON_SMALL - tokens::GRID;
    for (i, o) in list.iter().enumerate() {
        let ink = if o.danger {
            Role::Error
        } else {
            Role::OnSurface
        };
        // A label that does not fit takes a second line, as a list item's
        // headline does.
        let line = ui.line_as(BODY_L);
        let two = ui.measure_as(BODY_L, &o.label) > room;
        let h = if two {
            LIST_2.max(2.0 * line + 2.0 * LIST_PAD_V)
        } else {
            LIST_1
        };
        if let Some(icon) = o.icon {
            let tone = if o.danger {
                Role::Error
            } else if i == 0 {
                Role::Primary
            } else {
                Role::OnSurfaceVariant
            };
            ui.icon(
                x + LIST_PAD,
                cy + (h - ICON_SMALL) / 2.0,
                ICON_SMALL,
                icon,
                ICON_SMALL,
                tone,
            );
        }
        if two {
            let at = ui.first_line_end(BODY_L, &o.label, room);
            let (one, rest) = o.label.split_at(at);
            let top = cy + (h - 2.0 * line) / 2.0;
            ui.text_mid_as((x + lead, top, line), BODY_L, ink, one.trim_end());
            let rest = ui.fit_as(BODY_L, rest.trim_start(), room);
            ui.text_mid_as((x + lead, top + line, line), BODY_L, ink, &rest);
        } else {
            ui.text_mid_as(
                (x + lead, cy + (h - line) / 2.0, line),
                BODY_L,
                ink,
                &o.label,
            );
        }
        if o.opens {
            ui.row_chevron(x + w - LIST_PAD, cy, h, Role::OnSurfaceVariant);
        }
        ui.press(Shape::new(x, cy, w, h, SHAPE_L), ink.paint(), o.action);
        cy += h;
    }
    cy - y
}
