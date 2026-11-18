//! `Screen::Create`: creating a wallet.

use super::backup;
use super::parts::{
    OptionRow, buttons_and_next, descriptor_row, first_addresses, kind_list, locked_tag,
    never_real_seed, next_button, option_list, pass_field, stepper,
};
use super::pass_pair::{self, pass_pair};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::{fp_text, key_line};
use crate::{Action, Code, Screen, flow};
use faraday_draw::tokens;
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, LABEL_M, TITLE_S};
use faraday_ui::parts::button_rows;

pub(crate) const CSTEPS: [&str; crate::cstep::COUNT] = [
    EN.app_kind,
    EN.app_quorum,
    EN.app_keys,
    EN.app_check,
    // Numbers 4 to 8 are not cards of Make a new wallet.
    "",
    "",
    "",
    "",
    "",
    EN.create_backup,
];

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::cstep;
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(900.0);
    let Some(c) = app.create.state() else { return };
    let steps = app.create_cards();
    let filled = c
        .slots
        .iter()
        .filter(|s| **s != crate::create::Source::Empty)
        .count();
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&k| flow::Card {
            title: CSTEPS[k as usize].to_string(),
            summary: card_summary(app, k, filled),
            mono: false,
            done: app.create_card_done(k),
            open: c.open == Some(k),
            default: k == cstep::KIND || k == cstep::QUORUM || k == cstep::BACKUP_FORM,
            toggle: Action::CStep(k),
            lead: None,
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        // Back where it was opened from.
        back: Some(app.named_back(if c.from_home {
            (EN.create_home, Action::Nav(Screen::Home))
        } else {
            (EN.create_wallets, Action::Nav(Screen::WalletsStart))
        })),
        heading: EN.create_make_a_new_wallet,
        wrap_closed: false,
        total_fixed: !c.kind_unchosen,
        all_shown: false,
        note: None,
        chip: None,
        chip_tap: None,
        moves: crate::ui::Moves::Yes,
    };
    let scroll = c.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            create_body(app_ref, ui, steps[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Create, next, again);
}

/// The closed line of Make a new wallet's card `k`, with the account on
/// the card that carries the Account row (F98); `filled`, the slots with
/// a key.
fn card_summary(app: &View, k: u8, filled: usize) -> String {
    use crate::cstep;
    let Some(c) = app.create.state() else {
        return String::new();
    };
    let line = match k {
        cstep::KIND if c.kind_unchosen => EN.create_not_answered.to_string(),
        cstep::KIND => c.kind.name().to_string(),
        cstep::BACKUP_FORM => c
            .backup
            .map_or(EN.create_not_answered, crate::create::BackupForm::name)
            .to_string(),
        cstep::QUORUM => fill(EN.create_threshold, &[&c.m.to_string(), &c.n.to_string()]),
        cstep::KEYS => match app.create_waiting() {
            0 => fill(
                EN.create_of_chosen,
                &[&filled.to_string(), &c.slots.len().to_string()],
            ),
            k => fill(
                EN.create_slots_progress,
                &[
                    &(filled - k).to_string(),
                    &c.slots.len().to_string(),
                    &k.to_string(),
                ],
            ),
        },
        cstep::CHECK => match c.built.and_then(|i| app.session.wallets.get(i)) {
            Some(w) => fill(EN.create_first_addresses, &[&w.name]),
            None => EN.create_not_made_yet.to_string(),
        },
        _ => String::new(),
    };
    // A wallet made from another, or at an account other than 0, says
    // its account (`docs/WALLETS.md` §14).
    if c.account_card() == Some(k) && (c.from.is_some() || c.account != 0) {
        return fill(EN.create_at_account, &[&line, &c.account.to_string()]);
    }
    line
}

/// Under a Keys slot's session keys, each open vault's keys not loaded,
/// under the vault's name, and an Unlock button for each locked vault
/// (`docs/WALLETS.md` §14). A key in another slot is not
/// offered (§2.2). Returns the height used.
#[allow(
    clippy::too_many_arguments,
    reason = "the create-vault rows' view, vaults, slot, position and width"
)]
fn create_vault_rows(
    app: &View,
    ui: &mut Ui,
    vaults: &crate::create_from::CreateVaults,
    slot: usize,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    use crate::create::Source;
    let Some(c) = app.create.state() else {
        return 0.0;
    };
    let mut cy = y;
    for group in &vaults.open {
        let row: Vec<(String, Style, Action)> = group
            .keys
            .iter()
            .filter(|k| {
                !c.slots
                    .iter()
                    .enumerate()
                    .any(|(i, s)| i != slot && *s == Source::Here(k.fp))
            })
            .map(|k| {
                (
                    k.line.clone(),
                    Style::Secondary,
                    Action::CSlotVault(slot as u8, group.vault, k.record),
                )
            })
            .collect();
        if row.is_empty() {
            continue;
        }
        let name = ui.fit_as(LABEL_M, &group.name, w);
        ui.text_as(x, cy, LABEL_M, Role::OnSurfaceVariant, &name);
        cy += ui.line_as(LABEL_M) + faraday_draw::tokens::GAP_SMALL;
        cy += button_rows(ui, x, cy, w, &row);
    }
    if !vaults.locked.is_empty() {
        let row: Vec<(String, Style, Action)> = vaults
            .locked
            .iter()
            .map(|(i, name)| {
                (
                    fill(EN.vaults_unlock, &[name]),
                    Style::Secondary,
                    Action::Vault(crate::vault::VaultAction::OpenFrom(*i, Screen::Create)),
                )
            })
            .collect();
        cy += button_rows(ui, x, cy, w, &row);
    }
    cy - y
}

/// A key or wallet file of the Inbox with no key for the kind: its name
/// as a disabled item, no press, its reason under it. Returns the height.
fn refused_file_row(ui: &mut Ui, x: f32, y: f32, w: f32, f: &crate::create::KeyFile) -> f32 {
    let Err(why) = &f.key else {
        return 0.0;
    };
    let faint = Role::OnSurface.at(tokens::DISABLED_CONTENT);
    let name = fill(EN.create_slot_source, &[&f.name]);
    let name = ui.fit_as(TITLE_S, &name, w);
    ui.text_as(x, y + tokens::REFUSED_FILE_NAME_Y, TITLE_S, faint, &name);
    let h = ui.wrap_as(
        (x, y + tokens::REFUSED_FILE_REASON_Y, w),
        BODY_M,
        faint,
        why,
    );
    tokens::REFUSED_FILE_REASON_Y + h + tokens::REFUSED_FILE_GAP
}

fn create_body(app: &View, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::cstep;
    let c = app.create.state().expect("create");
    let at = (x, y, w);
    match n {
        cstep::KIND => create_kind(app, ui, c, at),
        cstep::BACKUP_FORM => create_backup_form(ui, c, at),
        cstep::QUORUM => create_quorum(ui, c, n, at),
        cstep::KEYS => create_keys(app, ui, c, n, at),
        cstep::CHECK => create_check(app, ui, c, n, at),
        _ => 0.0,
    }
}

/// The kind step: the kinds to choose from, and Start over once a wallet is
/// begun.
fn create_kind(app: &View, ui: &mut Ui, c: &crate::CreateState, (x, y, w): (f32, f32, f32)) -> f32 {
    let locked = c.built.is_some();
    let mut cy = y;
    // A row pressed moves on, the ticked one too: no Continue.
    let shown = crate::create::NewKind::create_rows(c.more_kinds);
    let lines: Vec<(usize, String)> = shown
        .into_iter()
        .map(|i| {
            (
                i,
                crate::create::NewKind::ALL[i]
                    .create_line((c.m, c.n))
                    .as_str()
                    .into(),
            )
        })
        .collect();
    let rows: Vec<(usize, &str)> = lines.iter().map(|(i, l)| (*i, l.as_str())).collect();
    let more = (!c.more_kinds).then_some(Action::CMoreKinds);
    // From Move money no kind is ticked: the person chooses
    // (owner, 2026-10-24).
    let chosen = (!c.kind_unchosen).then_some(c.kind);
    cy += kind_list(
        ui,
        (x, cy, w),
        &rows,
        chosen,
        more,
        !locked,
        (Action::CKind, crate::create::NewKind::create_name),
    );
    if app.create_unfinished() {
        cy += tokens::GAP_SMALL;
        let over = [(EN.create_start_over, Style::Ghost, Action::CreateOver)];
        cy += buttons_and_next(ui, x, cy, w, &over, None);
    }
    cy - y
}

/// A single key's backup: a row pressed moves on, the ticked one too
/// (F103); "codex32" in the Shares row's name carries the one tooltip on a
/// working screen (F112, F240).
fn create_backup_form(ui: &mut Ui, c: &crate::CreateState, (x, y, w): (f32, f32, f32)) -> f32 {
    let locked = c.built.is_some();
    let mut cy = y;
    // Every way is a row, none ticked until one is pressed.
    let rows: Vec<OptionRow> = crate::create::BackupForm::offered()
        .into_iter()
        .map(|f| OptionRow {
            name: f.name(),
            line: f.line(),
            on: c.backup == Some(f),
            action: Action::CBackupForm(f.index()),
            tip: f == crate::create::BackupForm::Codex32,
        })
        .collect();
    cy += option_list(ui, (x, cy, w), &rows, None, !locked);
    cy - y
}

/// The quorum step: signatures needed and keys, the sentence saying what
/// they mean, the path and account rows and Continue.
fn create_quorum(ui: &mut Ui, c: &crate::CreateState, n: u8, (x, y, w): (f32, f32, f32)) -> f32 {
    let locked = c.built.is_some();
    let mut cy = y;
    // MuSig2 is every key: only the count is chosen.
    let rows: &[(&str, usize, Action, Action)] = if c.kind.all_sign() {
        &[(EN.create_keys, c.n, Action::CN(-1), Action::CN(1))]
    } else {
        &[
            (
                EN.create_signatures_needed,
                c.m,
                Action::CM(-1),
                Action::CM(1),
            ),
            (EN.create_keys, c.n, Action::CN(-1), Action::CN(1)),
        ]
    };
    for &(label, value, minus, plus) in rows {
        stepper(ui, x, cy, w, label, value, minus, plus, !locked);
        cy += tokens::STEPPER_ROW;
    }
    let sentence = if c.kind.all_sign() {
        fill(EN.create_all_keys_sign, &[&c.n.to_string()])
    } else {
        fill(
            EN.create_any_of_the_keys,
            &[
                &c.m.to_string(),
                &c.n.to_string(),
                &(c.n - c.m).to_string(),
                (if c.n - c.m == 1 {
                    EN.create_key_inline
                } else {
                    EN.create_keys_inline
                }),
                &c.m.to_string(),
            ],
        )
    };
    cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, &sentence) + 14.0;
    // The path choice and the account, below the quorum (F98).
    if c.kind.has_bip87() {
        cy += path_choice(ui, x, cy, w, c);
    }
    if c.account_card() == Some(n) {
        cy += account_row(ui, x, cy, w, c);
    }
    if next_button(ui, x, cy, w, EN.create_continue, Action::CNext(n)) {
        cy += 48.0;
    }
    cy - y
}

/// What every slot of Keys draws from, read once for the step.
struct KeySlots<'a> {
    /// The Inbox's files with a key for this kind: position, name, key text.
    key_files: Vec<(usize, &'a str, &'a str)>,
    /// The first slot still empty, where New key leads.
    first_empty: Option<usize>,
    vaults: &'a crate::create_from::CreateVaults,
}

/// The keys step: the account row where there is no Quorum card, every slot,
/// the files refused and the foot.
fn create_keys(
    app: &View,
    ui: &mut Ui,
    c: &crate::CreateState,
    n: u8,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    use crate::create::Source;
    let locked = c.built.is_some();
    let mut cy = y;
    // The account, at the head of Keys for a kind with no Quorum
    // card (F98).
    if c.account_card() == Some(n) {
        cy += account_row(ui, x, cy, w, c);
    }
    // A vault already on a stick comes in before any key exists: off
    // until pressed, and a stick is never read while a key is open.
    let on = c.vaults.answer == Some(true);
    if !locked && (on || c.slots.iter().all(|s| *s == Source::Empty)) {
        let row = (x, cy, w);
        cy += ui.switch_row(
            row,
            EN.create_use_existing_vault,
            on,
            Action::CExistingVault(!on),
        );
    }
    if let (true, false, Some(i)) = (
        app.create_setup_pending(),
        locked,
        app.wallet_flow_of_owner(c.owner),
    ) {
        cy += ui.wrap_as(
            (x, cy + tokens::GAP_SMALL, w),
            BODY_M,
            Role::OnSurfaceVariant,
            EN.create_vault_to_unlock,
        ) + tokens::GAP_SMALL;
        let show = Action::Flow(crate::backup_flow::steps::FlowAction::Show(i));
        if next_button(ui, x, cy, w, app.medium.bring_in(), show) {
            cy += tokens::TOUCH;
        }
        return cy - y;
    }
    // The Inbox's key and wallet files, from the drawing memo:
    // those with a key for this kind are offered, the others
    // listed after the slots with why.
    let all_files = app.create_key_files();
    let key_files: Vec<(usize, &str, &str)> = all_files
        .iter()
        .filter_map(|f| Some((f.at, f.name.as_str(), f.key.as_deref().ok()?)))
        .collect();
    // The way on leads: New key on the first slot still empty,
    // Continue once every slot is filled.
    let first_empty = c.slots.iter().position(|s| *s == Source::Empty);
    // The open vaults' keys and the locked vaults, from the
    // drawing memo (`docs/WALLETS.md` §14).
    let vaults = app.create_vaults();
    let slots = KeySlots {
        key_files,
        first_empty,
        vaults: &vaults,
    };
    for (slot, src) in c.slots.iter().enumerate() {
        cy += key_slot(app, ui, c, &slots, (slot, src), (x, cy, w));
    }
    if !locked {
        for f in all_files.iter().filter(|f| f.key.is_err()) {
            cy += refused_file_row(ui, x, cy, w, f);
        }
    }
    if let Some(e) = &c.error {
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, e) + 6.0;
    }
    // The foot shows once every slot is filled: Make the wallet,
    // which makes it; the slot's own New key leads until then.
    // Once made, Continue goes on to Check.
    let label = match first_empty {
        Some(_) if !locked => None,
        _ if locked => Some(EN.create_continue),
        _ => Some(EN.create_make_the_wallet_button),
    };
    if let Some(label) = label
        && next_button(ui, x, cy, w, label, Action::CNext(n))
    {
        cy += tokens::TOUCH;
    }
    cy - y
}

/// One slot of Keys: its label and state, then, until the wallet is made,
/// the choices for it, its xpub outputs and its passphrase fields.
fn key_slot(
    app: &View,
    ui: &mut Ui,
    c: &crate::CreateState,
    slots: &KeySlots,
    (slot, src): (usize, &crate::create::Source),
    (x, y, w): (f32, f32, f32),
) -> f32 {
    use crate::create::Source;
    let mut cy = y;
    let label = format!("{} {}", EN.create_key, slot + 1);
    ui.text_as(x, cy + 4.0, LABEL_L, Role::OnSurface, &label);
    // A key locked in with a passphrase here says so.
    if let Source::Here(fp) = src
        && c.pass_locked.contains(fp)
    {
        let tw = ui.measure_as(LABEL_M, EN.create_locked_line) + 40.0;
        locked_tag(ui, x + w - tw, cy);
    }
    let (state, color) = match src {
        Source::Empty => (EN.create_not_chosen.to_string(), Role::OnSurfaceVariant),
        Source::Here(fp) => {
            let fpt = fp_text(faraday_bip::keys::Fingerprint(*fp));
            let name = app
                .session
                .keys
                .key_label(faraday_bip::keys::Fingerprint(*fp))
                .unwrap_or("");
            (fill(EN.create_loaded_here, &[&fpt, name]), Role::Ok)
        }
        Source::Cosigner(t) => (
            fill(EN.create_a_cosigner_s, &[&t[1..9.min(t.len())]]),
            Role::OnSurfaceVariant,
        ),
        Source::Later => (
            app.create_slot_needs(slot)
                .unwrap_or_else(|| EN.create_a_cosigner_adds.to_string()),
            Role::Warning,
        ),
    };
    let indent = if ui.compact {
        cy += 22.0;
        let state = ui.fit_as(BODY_M, &state, w);
        ui.text_as(x, cy + 4.0, BODY_M, color, &state);
        0.0
    } else {
        ui.text_as(x + 70.0, cy + 4.0, BODY_M, color, &state);
        70.0
    };
    cy += 28.0;
    if c.built.is_none() {
        let pass_offered = app.create_pass_offered(slot);
        // The slot's making on the card stands in place of New key.
        let making = app
            .keygen()
            .filter(|k| k.on_card && k.slot == Some(slot as u8));
        if let Some(k) = making {
            let at = (x + indent, cy, w - indent);
            cy += super::create_making::slot_making(app, ui, k, c.kind.multi(), at);
        }
        let at = (x + indent, cy, w - indent);
        cy += slot_choices(
            app,
            ui,
            c,
            slots,
            (slot, src),
            (pass_offered, making.is_some()),
            at,
        );
        let at = (x + indent, cy, w - indent);
        cy += slot_xpub_outputs(app, ui, c, (slot, src), pass_offered, at);
        let at = (x + indent, cy, w - indent);
        cy += slot_passphrase(app, ui, c, slot, pass_offered, at);
    }
    ui.rule(x, cy, w, Role::OutlineVariant);
    cy += 10.0;
    cy - y
}

/// The choices for one slot, wrapped to the card: the keys held here, New
/// key, the Inbox's files, scan, someone else, a passphrase and Clear, then
/// the open vaults' keys.
fn slot_choices(
    app: &View,
    ui: &mut Ui,
    c: &crate::CreateState,
    slots: &KeySlots,
    (slot, src): (usize, &crate::create::Source),
    (pass_offered, making): (bool, bool),
    (x, y, w): (f32, f32, f32),
) -> f32 {
    use crate::create::Source;
    let KeySlots {
        key_files,
        first_empty,
        vaults,
    } = slots;
    let mut cy = y;
    // A key chosen for one slot is offered in no other:
    // clearing the slot
    // offers it again.
    let used_elsewhere_here = |fp: [u8; 4]| {
        c.slots
            .iter()
            .enumerate()
            .any(|(i, s)| i != slot && *s == Source::Here(fp))
    };
    let used_elsewhere_cosigner = |text: &str| {
        c.slots
            .iter()
            .enumerate()
            .any(|(i, s)| i != slot && matches!(s, Source::Cosigner(t) if t == text))
    };
    // The choices for the slot, wrapped to the card.
    let mut row: Vec<(String, Style, Action)> = Vec::new();
    for k in &app.session.keys {
        let fp = k.fingerprint;
        if used_elsewhere_here(fp.0) {
            continue;
        }
        let on = *src == Source::Here(fp.0);
        row.push((
            String::from(&*key_line(fp, &k.label)),
            Style::chosen(on),
            Action::CSlotHere(slot as u8, fp.0),
        ));
    }
    // Its making on the card has its own Other ways in place of New key.
    // A multisig's empty slot makes its key in place beside Scan its xpub
    // (F282): New key is one press away under the making.
    let words_here = c.kind.multi() && *src == Source::Empty;
    if words_here && !making {
        row.push((
            EN.create_scan_its_xpub.to_string(),
            if *first_empty == Some(slot) {
                Style::Primary
            } else {
                Style::Secondary
            },
            Action::ScanFor(crate::Ask::Xpub(slot as u8 + 1)),
        ));
        row.push((
            EN.create_new_words.to_string(),
            Style::Secondary,
            Action::KeyOnCard(slot as u8),
        ));
    } else if !making {
        row.push((
            EN.create_new_key.to_string(),
            if *first_empty == Some(slot) {
                Style::Primary
            } else {
                Style::Secondary
            },
            Action::KeyGen(Some(slot as u8)),
        ));
    }
    for (i, name, text) in key_files {
        if used_elsewhere_cosigner(text) {
            continue;
        }
        row.push((
            fill(EN.create_slot_source, &[name]),
            Style::Secondary,
            Action::CSlotFile(slot as u8, *i),
        ));
    }
    if c.kind.multi() && (*src == Source::Later || (*src == Source::Empty && making)) {
        row.push((
            EN.create_scan_its_xpub.to_string(),
            Style::Secondary,
            Action::ScanFor(crate::Ask::Xpub(slot as u8 + 1)),
        ));
    }
    if c.kind.multi() && *src == Source::Empty {
        row.push((
            EN.create_someone_else.to_string(),
            Style::Secondary,
            Action::CSlotLater(slot as u8),
        ));
    }
    // A key from BIP-39 words with no passphrase can take
    // one here: on the slot's own row on a single-key
    // wallet, beside Show xpub QR on a multisig.
    if pass_offered && !c.kind.multi() {
        row.push((
            EN.create_add_a_passphrase.to_string(),
            Style::chosen(c.pass_slot == Some(slot as u8)),
            Action::CPassOpen(slot as u8),
        ));
    }
    if *src != Source::Empty {
        row.push((
            EN.create_clear.to_string(),
            Style::Ghost,
            Action::CSlotClear(slot as u8),
        ));
    }
    cy += button_rows(ui, x, cy, w, &row) - 44.0;
    cy += 44.0;
    cy += create_vault_rows(app, ui, vaults, slot, x, cy, w);
    cy - y
}

/// A key held here goes out to the cosigners, who need it to make the same
/// wallet: its xpub as a code, a PNG and a file. Nothing for another source.
fn slot_xpub_outputs(
    app: &View,
    ui: &mut Ui,
    c: &crate::CreateState,
    (slot, src): (usize, &crate::create::Source),
    pass_offered: bool,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    use crate::create::Source;
    let mut cy = y;
    // A key held here goes out to the cosigners, who need
    // it to make the same wallet.
    if c.kind.multi() && matches!(src, Source::Here(_)) {
        let k = slot as u8;
        let mut row = vec![
            (
                EN.create_show_xpub_qr.to_string(),
                Style::Secondary,
                Action::CKeyQr(k),
            ),
            (
                fill(EN.create_xpub_png, &[app.medium.for_the()]),
                Style::Secondary,
                Action::CodePng(Code::Key(k)),
            ),
            (
                fill(EN.create_xpub_file, &[app.medium.for_the()]),
                Style::Secondary,
                Action::CKeyOut(k),
            ),
        ];
        if pass_offered {
            row.insert(
                1,
                (
                    EN.create_add_a_passphrase.to_string(),
                    Style::chosen(c.pass_slot == Some(k)),
                    Action::CPassOpen(k),
                ),
            );
        }
        cy += button_rows(ui, x, cy, w, &row);
    }
    cy - y
}

/// The passphrase fields, under the slot they are for. Nothing for another
/// slot.
fn slot_passphrase(
    app: &View,
    ui: &mut Ui,
    c: &crate::CreateState,
    slot: usize,
    pass_offered: bool,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let mut cy = y;
    // The passphrase fields, under the slot they are for.
    if c.pass_slot == Some(slot as u8) && pass_offered {
        let (px, pw) = (x, w);
        cy += pass_pair(
            ui,
            (px, cy, pw),
            (c.pass.as_str(), c.pass2.as_str()),
            |ui, n, fy| {
                let (label, text) = if n == 0 {
                    (EN.create_passphrase, &c.pass)
                } else {
                    (EN.create_passphrase_again, &c.pass2)
                };
                pass_field(
                    ui,
                    px,
                    fy,
                    pw,
                    label,
                    text,
                    (
                        c.pass_focus == Some(n),
                        app.pass_chip(crate::demo::PassField::Slot(n)),
                        app.typing.pass_eye.shown,
                    ),
                    Action::CPassField(n),
                )
            },
        );
        cy += ui.wrap_as(
            (px, cy, pw),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.create_pass_kept,
        ) + tokens::GAP_SMALL;
        cy += super::keys_read_as_us_line(app, ui, (px, cy, pw));
        cy += never_real_seed(app, ui, px, cy, pw);
        if let Some(n) = &c.pass_note {
            cy += ui.wrap_as((px, cy, pw), BODY_M, Role::Error, n) + 6.0;
        }
        let bw = ui.measure_as(TITLE_S, EN.create_lock_button) + 36.0;
        ui.button(
            px + pw - bw,
            cy,
            Some(bw),
            BUTTON_H,
            EN.create_lock_button,
            Style::Primary.when(
                !pass_pair::differ(c.pass.as_str(), c.pass2.as_str())
                    && !app.typing.pass_eye.waits(),
            ),
            Action::CPassLock,
        );
        cy += 52.0;
    }
    cy - y
}

/// The check step: the made wallet's descriptor, paths and first addresses,
/// or what is still to do.
fn create_check(
    app: &View,
    ui: &mut Ui,
    c: &crate::CreateState,
    n: u8,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let mut cy = y;
    if let Some(wl) = c.built.and_then(|i| app.session.wallets.get(i)) {
        // The descriptor as a summary row: tapping it opens the
        // wallet's code with the descriptor's full text under it.
        cy += descriptor_row(ui, x, cy, w, wl, Action::QrWallet(c.built.unwrap_or(0)));
        cy += path_row(ui, x, cy, w, &c.path, app.create_can_take_back());
        cy += super::parts::key_path_line(app, ui, (x, cy, w), wl.policy());
        // A single key made here as codex32 shares, whatever its
        // address type: the set it was made as (F240,
        // `docs/CODEX32.md` §6.9).
        if let Some(set) = c.built.and_then(|i| app.create_shares_set(i)) {
            cy += change_row(ui, x, cy, w, EN.create_backup, &set.line(), None);
        }
        cy += first_addresses(app, ui, c.built.unwrap_or(0), x, cy, w);
        cy += 6.0;
        cy += check_foot(app, ui, x, cy, w, c.done[n as usize]);
    } else {
        ui.text_as(
            x,
            cy,
            BODY_S,
            Role::OnSurfaceVariant,
            EN.create_make_the_wallet,
        );
        cy += ui.advance_as(BODY_S, 30.0);
    }
    cy - y
}

/// Check's foot: **It matches** until the wallet is checked, then
/// the wallet's end card ([`backup::wallet_done`]), whose filled **Back up now** opens Back up on the wallet just made, the
/// board open on The standard way (`docs/BACKUP-FLOW.md` §17.21 item 2),
/// and beside it, for a wallet made to move another's money to
/// (`docs/WALLETS.md` §12), the spend of every coin from the
/// old one. Returns its height.
fn check_foot(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, checked: bool) -> f32 {
    if !checked {
        let next = (EN.create_it_matches, Action::CNext(crate::cstep::CHECK));
        return buttons_and_next(ui, x, y, w, &[], Some(next));
    }
    let moving = app.moving_from().map(|old| {
        let label = fill(EN.create_move_the_money, &[&app.session.wallets[old].name]);
        let label = ui.fit_as(TITLE_S, &label, w - 32.0);
        (
            label,
            Action::SpendBackup(crate::spend_backup::SpendBackupAction::SpendFrom(old)),
        )
    });
    let Some(wallet) = app.create().and_then(|c| c.built) else {
        return 0.0;
    };
    let moving = moving.as_ref().map(|(l, a)| (l.as_str(), *a));
    backup::wallet_done(app, ui, wallet, backup::From::Create, moving, (x, y, w))
}

/// Check's Path row: the made wallet's derivation paths, worked out when
/// it was made, with **Change** back to the Account row while the wallet
/// can be taken back (F98, `docs/WALLETS.md` §14). Nothing when
/// there is no path. Returns its height.
fn path_row(ui: &mut Ui, x: f32, y: f32, w: f32, path: &str, change: bool) -> f32 {
    if path.is_empty() {
        return 0.0;
    }
    let change = change.then_some((EN.create_path_change, Action::CPathChange));
    change_row(ui, x, y, w, EN.create_path, path, change)
}

/// The path row of a multisig kind: its own path (BIP-48's, or `m/45'`)
/// or BIP-87's, the first chosen at entry, as a value with **Change**;
/// once Change is pressed, the two as a segmented button. The made
/// wallet's row has no Change. Returns its height.
fn path_choice(ui: &mut Ui, x: f32, y: f32, w: f32, c: &crate::CreateState) -> f32 {
    let own = if c.kind == crate::create::NewKind::MultiLegacy {
        EN.create_path_bip45
    } else {
        EN.create_path_bip48
    };
    let options = [
        (own, Action::CMultiPath(false)),
        (EN.create_path_bip87, Action::CMultiPath(true)),
    ];
    let locked = c.built.is_some();
    // The info button (F287) stands only where BIP-48 is on show, while
    // the wallet is being made: after the closed value, or after the
    // segmented button.
    let tip = !locked && c.kind != crate::create::NewKind::MultiLegacy;
    let side = ui.touch.max(tokens::ICON_BUTTON);
    if c.path_open && !locked {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.create_path);
        let top = y + tokens::CREATE_FIELD_LABEL + 2.0;
        let labels = [options[0].0, options[1].0];
        let sw = ui.segment_width(LABEL_L.size, &labels) * 2.0;
        // After the segments when they and the button fit one line, else
        // at the label's line, at the right.
        let beside = tip && sw + side <= w;
        let room = if beside { w - side } else { w };
        let h = ui.choices(
            (x, top, room, tokens::TOUCH),
            LABEL_L.size,
            &options,
            Some(usize::from(c.at_bip87())),
        );
        if tip {
            let at = if beside {
                (x + sw, top + h / 2.0)
            } else {
                (x + w - side, y + tokens::GRID)
            };
            super::parts::info_button(ui, at, Action::CPathAbout, EN.create_path_tip);
        }
        return tokens::CREATE_FIELD_LABEL + 2.0 + h + tokens::GAP;
    }
    let change = (!locked).then_some((EN.create_account_change, Action::CMultiPathChange));
    let value = options[usize::from(c.at_bip87())].0;
    let h = change_row(ui, x, y, w, EN.create_path, value, change);
    if tip && !c.at_bip87() {
        let end = x + ui.measure_as(BODY_M.mono(), value);
        super::parts::info_button(
            ui,
            (end, y + tokens::CREATE_FIELD_LABEL + tokens::GRID),
            Action::CPathAbout,
            EN.create_path_tip,
        );
    }
    h
}

/// The Account row (F98), for a kind whose path has an account: its
/// value with **Change**, or, once Change is pressed, its − n + stepper.
/// The made wallet's row has no Change. Returns its height.
fn account_row(ui: &mut Ui, x: f32, y: f32, w: f32, c: &crate::CreateState) -> f32 {
    let locked = c.built.is_some();
    if c.account_open && !locked {
        stepper(
            ui,
            x,
            y,
            w,
            EN.create_account,
            c.account as usize,
            Action::CAccount(-1),
            Action::CAccount(1),
            true,
        );
        return tokens::STEPPER_ROW;
    }
    let change = (!locked).then_some((EN.create_account_change, Action::CAccount(0)));
    change_row(
        ui,
        x,
        y,
        w,
        EN.create_account,
        &c.account.to_string(),
        change,
    )
}

/// A value row: `label` above, `value` mono and wrapped below, and a
/// **Change** button at the right when `change` is given. Returns its
/// height.
fn change_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    change: Option<(&str, Action)>,
) -> f32 {
    let bw = change.map_or(0.0, |(l, _)| ui.measure_as(LABEL_L, l) + 32.0);
    let vw = if change.is_some() { w - bw - 10.0 } else { w };
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label);
    let vh = ui
        .wrap_as((x, y + 20.0, vw), BODY_M.mono(), Role::OnSurface, value)
        .max(18.0);
    let h = 20.0 + vh + 8.0;
    if let Some((l, action)) = change {
        ui.button(
            x + w - bw,
            y + (h - BUTTON_H) / 2.0,
            Some(bw),
            BUTTON_H,
            l,
            Style::Ghost,
            action,
        );
    }
    ui.rule(x, y + h, w, Role::OutlineVariant);
    h + 12.0
}

/// **A BIP-85 child seed** (`docs/decisions/F166.md`): a wallet just made
/// with a key recorded as one. A Material basic dialog with one action,
/// **OK**. Reads the state.
pub(crate) fn child_notice(app: &View, ui: &mut Ui, w: f32, h: f32) {
    use faraday_draw::tokens::{DIALOG_MAX, GAP};
    use faraday_draw::widgets::Icon;
    let Some(fp) = app
        .create()
        .and_then(|c| c.child)
        .or_else(|| app.explore.child())
    else {
        return;
    };
    let line = fill(
        EN.create_child_body,
        &[&fp_text(faraday_bip::keys::Fingerprint(fp))],
    );
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        (
            Icon::Keys.into(),
            Role::Primary.paint(),
            EN.create_child_head,
        ),
        &mut |ui, x, y, iw| ui.wrap_as((x, y, iw), BODY_M, Role::OnSurface, &line) + GAP,
        &[(EN.create_child_ok, Style::Primary, Action::Cancel)],
    );
}
