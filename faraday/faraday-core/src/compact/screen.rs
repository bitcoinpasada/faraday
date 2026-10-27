//! Screens laid out for a small panel (`Faraday::is_compact`), where the
//! desktop's layout has columns side by side that a 268 dp panel has no
//! room for. Each is a single scrolled column under the page's bar.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use faraday_draw::widgets::Icon;

use crate::compact::M;
use crate::screens::{missing_keys_line, never_real_seed};
use crate::ui::pal::*;
use crate::ui::{CoreUi, Style, Ui, W};
use crate::wallet::{Session, fp_text};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, BUTTON_H, FIELD_H, FIELD_PAD, LABEL_L, LABEL_M, LABEL_S, TITLE_M,
};

pub(crate) use faraday_ui::parts::measured;

/// A row that goes somewhere: an icon, a name, a line under it, and a
/// chevron when it can be pressed. Returns the height it took.
#[allow(clippy::too_many_arguments)]
pub(crate) fn row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    icon: Option<Icon>,
    label: &str,
    sub: &str,
    sub_tone: crate::ui::Paint,
    action: Option<Action>,
) -> f32 {
    let enabled = action.is_some();
    let pressed = action.is_some_and(|a| ui.is_pressed(a));
    let mut tx = x + 14.0;
    if icon.is_some() {
        tx = x + 52.0;
    }
    let room = x + w - tx - if enabled { 26.0 } else { 10.0 };
    // A label that does not fit at its size breaks at a space onto
    // further lines, and the row grows (`docs/DESIGN.md` "Menu row"):
    // never cut off, the label this row is given is always shown whole.
    let lines = ui.wrap_lines_as(LABEL_L, label, room);
    let line_h = ui.line_as(LABEL_L);
    let h = 56.0 + (lines - 1) as f32 * line_h;
    crate::ui::fill_pressed(ui, (x, y, w, h, 10.0), Role::SurfaceContainerLow, pressed);
    ui.stroke(x, y, w, h, 10.0, Role::OutlineVariant.paint());
    if let Some(i) = icon {
        let (bg, fg) = if enabled {
            (
                Role::SecondaryContainer.paint(),
                Role::OnSecondaryContainer.paint(),
            )
        } else {
            (
                Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTAINER),
                crate::ui::off(),
            )
        };
        ui.fill(x + 10.0, y + 12.0, 32.0, 32.0, 8.0, bg);
        ui.icon(x + 10.0, y + 12.0, 32.0, i, 14.0, fg);
    }
    let label_color = if enabled {
        Role::OnSurface.paint()
    } else {
        crate::ui::off()
    };
    let sub_y = if lines > 1 {
        ui.wrap_as((tx, y + 9.0, room), LABEL_L, label_color, label);
        y + 9.0 + lines as f32 * line_h
    } else {
        let label = ui.fit_as(LABEL_L, label, room);
        ui.text_as(tx, y + 9.0, LABEL_L, label_color, &label);
        y + 30.0
    };
    if !sub.is_empty() {
        let sub = ui.fit_as(BODY_S, sub, room);
        ui.text_as(tx, sub_y, BODY_S, sub_tone, &sub);
    }
    if let Some(a) = action {
        ui.icon(
            x + w - 24.0,
            y + 18.0,
            20.0,
            Icon::ChevronRight,
            9.0,
            Role::OnSurfaceVariant,
        );
        ui.hit(x, y, w, h, a);
    }
    h + 8.0
}

/// Full-width buttons, one under another. Returns their height.
pub(crate) fn stack(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, Style, Action)]) -> f32 {
    let mut cy = y;
    for &(label, style, action) in items {
        ui.button(x, cy, Some(w), BUTTON_H, label, style, action);
        cy += 52.0;
    }
    cy - y
}

/// A section's heading. Returns its height.
pub(crate) fn heading(ui: &mut Ui, x: f32, y: f32, s: &str) -> f32 {
    ui.text_as(x, y, LABEL_M, Role::OnSurfaceVariant, s);
    24.0
}

/// One wallet's card, as its own page: what it is, how many signatures
/// can be made here, its keys and what each needs, its first address,
/// and what can be done with it.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn wallet_card(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let (x, iw) = (x0 + M, cw - 2.0 * M);
    let top = -app.list_offset;
    let mut cy = top + 12.0;
    let Some(wlt) = app.session.wallets.get(app.wallets.wallet()) else {
        if app.session.loose_keys().is_empty() {
            ui.text_as(
                x,
                cy,
                BODY_M,
                Role::OnSurfaceVariant,
                EN.compact_no_wallet_loaded,
            );
        } else {
            cy += crate::screens::loose_card(app, ui, x, cy, iw);
            finish(app, ui, x0, cw, h, cy - top + 8.0);
        }
        return;
    };
    // The chart, as its own page.
    if app.glance.open()
        && let Some(g) = app.glance_of(app.wallets.wallet())
    {
        cy += crate::glance::draw_column(ui, &g, crate::glance::Direction::WalletFirst, x, cy, iw);
        finish(app, ui, x0, cw, h, cy - top + 8.0);
        return;
    }

    // A PSBT waiting leads, unless it is this wallet's: the card's first
    // button is for that.
    let lead = match app.wallets.spend() {
        Some(s) if s.wallet == Some(app.wallets.wallet()) => None,
        Some(s) => Some((
            fill(EN.compact_continue_signing, &[&s.spend.source.to_string()]),
            Action::Nav(Screen::Spend),
        )),
        None if app.psbt_for(app.wallets.wallet()).is_some() => None,
        None => app.lead_psbt().map(|i| {
            (
                fill(EN.compact_review, &[&app.inbox[i].name.to_string()]),
                Action::StartSpend(i),
            )
        }),
    };
    if let Some((line, action)) = lead {
        let bg = if ui.is_pressed(action) {
            Role::Primary.with_alpha(40)
        } else {
            Role::Primary.with_alpha(18)
        };
        ui.fill(x, cy, iw, 44.0, 10.0, bg);
        ui.stroke(x, cy, iw, 44.0, 10.0, Role::Primary.with_alpha(76));
        ui.icon(x + 10.0, cy + 12.0, 20.0, Icon::File, 12.0, Role::Primary);
        let line = ui.fit_as(LABEL_L, &line, iw - 64.0);
        ui.text_mid_as((x + 36.0, cy, 44.0), LABEL_L, Role::OnSurface, &line);
        ui.icon(
            x + iw - 24.0,
            cy + 12.0,
            20.0,
            Icon::ChevronRight,
            9.0,
            Role::Primary,
        );
        ui.hit(x, cy, iw, 44.0, action);
        cy += 56.0;
    }

    // Its name, which can be changed.
    match app.wallets.renaming() {
        Some(name) => {
            ui.field_box((x, cy, iw), true, None);
            ui.typed(
                x + FIELD_PAD,
                cy,
                FIELD_H,
                (TITLE_M).size,
                W::from((TITLE_M).face),
                Role::OnSurface,
                name,
                iw - 2.0 * FIELD_PAD,
                true,
                Action::Rename,
            );
            ui.hit(x, cy, iw, FIELD_H, Action::Rename);
            cy += FIELD_H + 8.0;
        }
        None => {
            let rw = ui.measure_as(LABEL_L, EN.compact_rename) + 24.0;
            let name_h = ui.wrap_as(
                (x, cy + 4.0, iw - rw - 8.0),
                TITLE_M,
                Role::OnSurface,
                &wlt.name,
            );
            ui.button(
                x + iw - rw,
                cy,
                Some(rw),
                BUTTON_H,
                EN.compact_rename,
                Style::Ghost,
                Action::Rename,
            );
            cy += name_h.max(30.0) + 10.0;
        }
    }
    let line = format!(
        "{} · {}",
        Session::shape(wlt),
        crate::wallet::source_tag(&wlt.source, wlt.origin, false)
    );
    cy += ui.wrap_as((x, cy, iw), BODY_S, Role::OnSurfaceVariant, &line) + 8.0;
    // Where its backup is (§5.2): a press opens Backups on it.
    cy += crate::screens::backup_line(app, ui, x, cy, iw, app.wallets.wallet());

    if let Some(r) = wlt.policy().silent() {
        silent_card(app, ui, r, x, iw, cy, top, h);
        return;
    }

    // Signatures possible here.
    let (m, _) = Session::quorum(wlt);
    let slots = app.session.slots(wlt);
    let here = slots.iter().filter(|s| s.held_by.is_some()).count().min(m);
    // Short: Scan a key under the line; enough: Can spend
    // (`docs/history/SCAN-TO-SIGN.md` §2.2).
    let can = app.can_spend(app.wallets.wallet());
    let box_h = if can { 76.0 } else { 90.0 };
    ui.fill(x, cy, iw, box_h, 10.0, Role::SurfaceContainerHighest);
    ui.stroke(x, cy, iw, box_h, 10.0, Role::OutlineVariant);
    let pw = ui.pips(x + 14.0, cy + 16.0, m, here);
    let sigs = fill(
        EN.compact_of_signatures,
        &[&here.to_string(), &m.to_string()],
    );
    let sigs = ui.fit_as(LABEL_L, &sigs, iw - pw - 40.0);
    ui.text_mid_as(
        (x + 26.0 + pw, cy + 4.0, 32.0),
        LABEL_L,
        Role::OnSurface,
        &sigs,
    );
    if can {
        ui.chip(
            x + 14.0,
            cy + 38.0,
            EN.compact_can_spend,
            Role::OnOkContainer,
            Role::OkContainer,
        );
    } else {
        ui.button(
            x + 10.0,
            cy + 40.0,
            Some(iw - 20.0),
            BUTTON_H,
            EN.compact_scan_a_key,
            Style::Primary.when(app.may_load_keys()),
            Action::ScanKey(app.wallets.wallet()),
        );
    }
    cy += box_h + 16.0;

    // The first thing to do with it, on the first screen: from the vault
    // to a spend (`docs/history/NEW-WALLET.md` §11.2).
    let (spend_label, spend_action) = app.spend_button(app.wallets.wallet());
    cy += stack(
        ui,
        x,
        cy,
        iw,
        &[(spend_label.as_str(), Style::Primary, spend_action)],
    ) + 4.0;

    // The wallet at a glance, as its own page: its keys and its backup.
    let quorum_here =
        slots.iter().filter(|s| s.held_by.is_some()).count() >= crate::wallet::needed(wlt);
    if let Some(g) = app.glance_of(app.wallets.wallet()) {
        let sub = format!(
            "{} {} · {}",
            g.keys.len(),
            if g.keys.len() == 1 {
                EN.compact_key_inline
            } else {
                EN.compact_keys
            },
            match &g.backup {
                crate::glance::Backup::Plan { nodes, .. } => fill(
                    EN.compact_backup,
                    &[
                        &nodes.len().to_string(),
                        (if nodes.len() == 1 {
                            EN.compact_place
                        } else {
                            EN.compact_places
                        })
                    ]
                ),
                crate::glance::Backup::Locked { vault, .. } =>
                    fill(EN.compact_plan_in_locked, &[&vault.to_string()]),
                crate::glance::Backup::None => EN.compact_no_backup_plan.to_string(),
            }
        );
        cy += row(
            ui,
            x,
            cy,
            iw,
            None,
            EN.compact_at_a_glance,
            &sub,
            Role::OnSurfaceVariant.paint(),
            Some(Action::Glance(true)),
        );
    }
    cy += crate::screens::slot_marks(app, ui, (x, cy, iw), &slots);
    if !quorum_here {
        cy += 4.0;
        cy += missing_keys_line(app, ui, &slots, Screen::Wallets, x, cy, iw);
    }
    cy += 10.0;
    cy += heading(ui, x, cy, EN.compact_first_receive);
    let (addr, held) = app.receive_line(wlt, false, 0);
    cy += ui.wrap_as(
        (x, cy, iw),
        BODY_M.mono(),
        if held { Role::Warning } else { Role::OnSurface },
        &addr,
    ) + 18.0;

    // What else can be done with it.
    let mut buttons = vec![
        (
            EN.compact_show_wallet,
            Style::Secondary,
            Action::QrWallet(app.wallets.wallet()),
        ),
        (
            if app.wallet_backup_unfinished(app.wallets.wallet()) {
                EN.backup_flow_continue
            } else {
                EN.compact_back_up
            },
            Style::Secondary,
            Action::Backup(app.wallets.wallet()),
        ),
    ];
    // Create filled from this wallet (`docs/history/NEW-WALLET.md` §16.2).
    if crate::create::NewKind::of(wlt.policy()).is_some() {
        buttons.push((
            EN.wallets_new_wallet_button,
            Style::Secondary,
            Action::NewWalletFrom(app.wallets.wallet()),
        ));
    }
    if app
        .session
        .message_wallets()
        .contains(&app.wallets.wallet())
    {
        buttons.push((
            EN.compact_sign_a_message,
            Style::Secondary,
            Action::SignMessage,
        ));
    }
    buttons.push((
        EN.compact_remove_from_session,
        Style::Ghost,
        Action::RemoveWallet(app.wallets.wallet()),
    ));
    cy += stack(ui, x, cy, iw, &buttons);
    finish(app, ui, x0, cw, h, cy - top + 8.0);
}

/// A silent payments wallet's card on a small panel.
#[allow(clippy::too_many_arguments)]
fn silent_card(
    app: &View,
    ui: &mut Ui,
    r: &faraday_bip::silent_wallet::SilentWallet,
    x: f32,
    iw: f32,
    y: f32,
    top: f32,
    h: f32,
) {
    let may = app.may_load_keys();
    let mut cy = y;
    cy += heading(ui, x, cy, EN.compact_key);
    let held = app.session.keys.key_label(r.fingerprint);
    ui.text_mid_as(
        (x, cy, 30.0),
        BODY_M.mono(),
        Role::OnSurface,
        &fp_text(r.fingerprint),
    );
    let (state, fg, bg) = if app.silent_differs(app.wallets.wallet()) {
        (
            EN.silent_wallet_card_differs,
            Role::OnErrorContainer,
            Role::ErrorContainer,
        )
    } else if held.is_some() {
        (EN.compact_here, Role::OnOkContainer, Role::OkContainer)
    } else {
        (
            EN.compact_not_here,
            Role::OnSurfaceVariant,
            Role::SurfaceContainerHighest,
        )
    };
    let chipw = ui.measure_as(BODY_S, state) + 34.0;
    ui.chip(x + iw - chipw, cy + 3.0, state, fg, bg);
    cy += 30.0;
    if let Some(label) = held {
        if !label.trim().eq_ignore_ascii_case(&fp_text(r.fingerprint)) {
            ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, label);
            cy += 20.0;
        }
    } else {
        let slot = crate::wallet::Slot::bare(r.fingerprint);
        let (label, style, action) = if app.vault_key_for(&slot).is_some() {
            (
                EN.compact_load_from_vault,
                Style::Primary,
                Action::Vault(crate::vault::VaultAction::LoadKeyOf(
                    r.fingerprint.0,
                    crate::vault::SlotOf::Bare,
                )),
            )
        } else {
            (
                EN.compact_add_its_key,
                Style::Secondary,
                Action::Entry(Some(r.fingerprint.0)),
            )
        };
        let style = style.when(may);
        ui.button(x, cy, Some(iw), BUTTON_H, label, style, action);
        cy += 44.0;
    }
    cy += 10.0;
    cy += heading(ui, x, cy, EN.compact_address);
    cy += crate::screens::silent_address_lines(app, ui, r, (x, cy, iw)) + 14.0;
    ui.text_as(
        x,
        cy,
        LABEL_M,
        Role::OnSurfaceVariant,
        EN.compact_labels_handed,
    );
    ui.text_right_as(
        (x + iw, cy - 6.0, 28.0),
        BODY_M.mono(),
        Role::OnSurface,
        &r.labels.to_string(),
    );
    cy += 32.0;
    cy += stack(
        ui,
        x,
        cy,
        iw,
        &[
            (
                EN.compact_address_labels,
                Style::Primary,
                Action::SWallet(app.wallets.wallet()),
            ),
            (
                EN.compact_show_wallet,
                Style::Secondary,
                Action::QrWallet(app.wallets.wallet()),
            ),
            (
                EN.compact_remove_from_session,
                Style::Ghost,
                Action::RemoveWallet(app.wallets.wallet()),
            ),
        ],
    );
    finish(app, ui, 0.0, x + iw + M, h, cy - top + 8.0);
}

/// What explains a page, after its controls and behind a tap, as a
/// step's is. Returns its height.
pub(crate) fn about(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    about_learn(ui, x, y, w, text, None)
}

/// [`about`] with a link to the Learn page on `learn` under the text.
pub(crate) fn about_learn(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    text: &str,
    learn: Option<crate::learn::Topic>,
) -> f32 {
    crate::flow::about(
        ui,
        x,
        y,
        w,
        EN.compact_about_page,
        crate::ABOUT_PAGE,
        (text, learn),
    )
}

/// Reports how tall the page is, so it scrolls.
pub(crate) fn finish(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32, content: f32) {
    app.scroll.content_h().set(content);
    let view = ui.rect(x0, 0.0, cw, h);
    ui.report_scroll(view, content - h);
}

/// A passphrase field: its label over it, dots for what is typed, a
/// press to type into it. Returns its height.
#[allow(clippy::too_many_arguments)]
fn pass_field(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    secret: &str,
    (on, chip): (bool, Option<Action>),
    action: Action,
) -> f32 {
    let lh = ui.field_label(x, y, label);
    let fy = y + lh;
    ui.field_box((x, fy, w), on, None);
    ui.hit(x, fy, w, FIELD_H, action);
    if secret.is_empty() {
        ui.no_passphrase(x, fy, FIELD_H, on);
        // Use faraday goes in the "Demo only" card (`docs/DEMO.md` §5.4).
        if let Some(a) = chip {
            ui.demo_offer(EN.demo_use_faraday, a);
        }
    } else if chip.is_some() && secret == crate::demo::PASSPHRASE {
        crate::screens::code_field(ui, x, fy, w, FIELD_H, secret);
    } else {
        let dots = "•".repeat(secret.chars().count());
        ui.typed(
            x + FIELD_PAD,
            fy,
            FIELD_H,
            (BODY_M.mono()).size,
            W::from((BODY_M.mono()).face),
            Role::OnSurface,
            &dots,
            w - 2.0 * FIELD_PAD,
            on,
            action,
        );
    }
    lh + FIELD_H + 12.0
}

/// What Add a key on a small panel is for, over its form: the round's
/// codex32 string typed in place (`docs/CODEX32.md` §9.5), the key a slot
/// wants, or the wallet whose keys it may give.
fn entry_for(app: &View) -> Option<String> {
    let es = app.entry.state();
    let for_wallet = es
        .wallet
        .and_then(|w| app.session.wallets.get(w))
        .map(|wl| fill(EN.compact_entry_for_wallet, &[&wl.name.to_string()]));
    let wanted = es.wanted.map(|fp| {
        fill(
            EN.compact_for_key,
            &[&fp_text(faraday_bip::keys::Fingerprint(fp)).to_string()],
        )
    });
    let round = es.c32_round.as_ref();
    let round = round.map(|w| fill(EN.c32_round_heading, &[&w.share(), &w.id_text()]));
    round.or(wanted).or(for_wallet)
}

/// Add a key on a small panel. Words: what is being typed, the words it
/// can be, and the word keyboard on the first screenful; the
/// words taken, the passphrase and the buttons under them. The other
/// forms type on the docked keyboard.
pub(crate) fn entry(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let es = app.entry.state();
    use crate::forms::Form;
    let (x, iw) = (x0 + M, cw - 2.0 * M);
    let top = -app.list_offset;
    let mut y = top + 10.0;
    if let Some(line) = entry_for(app) {
        let line = ui.fit_as(LABEL_L, &line, iw);
        ui.text_as(x, y, LABEL_L, Role::Primary, &line);
        y += 22.0;
    }
    // The form's chips step aside while a codex32 correction or clash
    // stands in place of the field, so the panel stays above the keyboard
    // (`docs/CODEX32.md` §8.4).
    if es.other_forms && !(es.form == Form::Codex32 && crate::forms::codex32_replaced(es)) {
        let forms: Vec<(String, Style, Action)> = Form::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.name().to_string(),
                    if *f == es.form {
                        Style::Primary
                    } else {
                        Style::Secondary
                    },
                    Action::EntryForm(i as u8),
                )
            })
            .collect();
        y += crate::screens::button_rows(ui, x, y, iw, &forms) - 4.0;
    }
    if es.form != Form::Words {
        let y = entry_form(app, ui, x, y, iw, None);
        finish(app, ui, x0, cw, h, y - top + 8.0);
        return;
    }
    // Scan leads, above the word field: primary until a word is typed
    // (`docs/history/SCAN-TO-SIGN.md` §2.5).
    let typed = crate::screens::entry_typed(app);
    ui.button(
        x,
        y,
        Some(iw),
        BUTTON_H,
        EN.compact_scan_a_seedqr,
        if typed {
            Style::Secondary
        } else {
            Style::Primary
        },
        crate::screens::entry_scan(app),
    );
    y += 48.0;
    if es.keys.is_none() {
        entry_words(app, ui, x, y, iw, true);
        return;
    }
    y = entry_words(app, ui, x, y, iw, true);
    y += pass_field(
        ui,
        x,
        y,
        iw,
        EN.compact_bip_39_passphrase,
        &es.passphrase,
        (
            es.on_passphrase,
            app.pass_chip(crate::demo::PassField::Entry),
        ),
        Action::EntryPassphrase,
    );
    // Add key is pinned at the foot.
    let add = if typed {
        Style::Primary
    } else {
        Style::Secondary
    };
    ui.pin = Some((EN.compact_add_key.to_string(), add, Action::EntryAdd));
    y += stack(
        ui,
        x,
        y,
        iw,
        &[
            (
                EN.compact_make_a_new_key,
                Style::Secondary,
                Action::KeyGen(None),
            ),
            (EN.compact_clear, Style::Ghost, Action::EntryClear),
        ],
    );
    crate::screens::entry_stick(app, ui, x, y + 4.0);
    y += 44.0;
    finish(app, ui, x0, cw, h, y - top + 8.0);
}

/// Add a key's words form on a small panel: the word being typed, the
/// words it can be, the word keyboard, the words taken and the
/// lists. Add a key draws it, and so does New key's Words card for a key
/// typed (`docs/history/NEW-WALLET.md` §15.2). `page`: on Add a key's own page,
/// whose row of lists offers Other forms too. Returns where the next row
/// starts.
pub(crate) fn entry_words(app: &View, ui: &mut Ui, x: f32, y: f32, iw: f32, page: bool) -> f32 {
    let mut y = y;
    let lang = app.entry.state().language();
    let english = lang == faraday_bip::bip39::Language::English;
    // One button for the list in use; the lists open from it; unless
    // the chips are already shown, Other forms beside it.
    let mut langs: Vec<(String, Style, Action)> = if !app.entry.state().languages {
        vec![(
            if english {
                EN.compact_other_languages.to_string()
            } else {
                fill(EN.compact_other_lists, &[crate::forms::language_name(lang)])
            },
            Style::Ghost,
            Action::EntryLanguages,
        )]
    } else {
        crate::forms::LANGUAGES
            .iter()
            .enumerate()
            .map(|(i, l)| {
                (
                    crate::forms::language_name(*l).to_string(),
                    if *l == lang {
                        Style::Primary
                    } else {
                        Style::Ghost
                    },
                    Action::EntryLanguage(i as u8),
                )
            })
            .collect()
    };
    if page && !app.entry.state().other_forms {
        langs.insert(
            0,
            (
                EN.compact_other_forms.to_string(),
                Style::Ghost,
                Action::EntryOtherForms,
            ),
        );
    }
    let Some(w) = app.entry.state().keys.as_deref() else {
        y += never_real_seed(app, ui, x, y, iw);
        return y + crate::screens::button_rows(ui, x, y, iw, &langs);
    };
    let taken: Vec<u16> = w.committed_indices().collect();
    let (status, tone) = match &app.entry.state().error {
        Some(e) => (e.clone(), Role::Error.paint()),
        None => crate::screens::keyed_status(app, lang, &taken),
    };
    y = keyed_head(
        ui,
        (x, y, iw),
        w,
        lang,
        &fill(EN.compact_word, &[&(taken.len() + 1).to_string()]),
        Some((&status, tone)),
        (
            !app.entry.state().on_passphrase,
            app.entry.state().shift,
            app.entry_sample_shown().then_some(EN.use_a_sample),
        ),
    );
    if app.entry_paste_offered() {
        ui.demo_offer(EN.paste_words, Action::PasteWords);
    }
    // The words taken, as many boxes as the count chosen.
    y += 6.0;
    y += heading(ui, x, y, EN.compact_words);
    y += crate::screens::word_count_row(app, ui, x, y, iw, false);
    y += keyed_taken(
        ui,
        (x, y, iw),
        lang,
        &taken,
        (
            app.entry.state().word_boxes(taken.len() + 1),
            false,
            app.seed_line,
        ),
    );
    // Under the word fields.
    y + crate::screens::button_rows(ui, x, y, iw, &langs) - 6.0
}

/// A list typed on its own keyboard, on a small panel: the word being
/// typed under `label`, `status` under it, the words it can be as pills
/// and the keyboard, with the on-screen keyboard's shift as `shift`, and
/// with `chip`, Use a sample (or `chip`'s own label) handed to the "Practice
/// only" card (`docs/DEMO.md` §14). Add a key's word entry and Check my
/// copy's both draw it. Returns where the next row starts.
pub(crate) fn keyed_head(
    ui: &mut Ui,
    (x, y, iw): (f32, f32, f32),
    w: &crate::word_entry::WordEntry,
    lang: faraday_bip::bip39::Language,
    label: &str,
    status: Option<(&str, crate::ui::Paint)>,
    (caret, shift, chip): (bool, bool, Option<&'static str>),
) -> f32 {
    let mut y = y;
    let prefix: String = w.prefix().iter().collect();
    // The word being typed.
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label);
    let off = (ui.measure_as(BODY_S, label) + 12.0).max(64.0);
    let shown = if prefix.is_empty() && !caret {
        "…".to_string()
    } else {
        prefix.clone()
    };
    let pw = ui.text_as(x + off, y - 4.0, BODY_L.mono(), Role::Primary, &shown);
    if let Some(label) = chip {
        ui.demo_offer(label, Action::UseSample);
    }
    if caret {
        ui.caret(x + off + 2.0 + pw, y - 2.0, 20.0);
    }
    y += 26.0;
    if let Some((status, tone)) = status {
        let status = ui.fit_as(LABEL_M, status, iw);
        ui.text_as(x, y, LABEL_M, tone, &status);
    }
    y += 22.0;
    // What the keys typed can be: a pill each.
    y += crate::screens::keyed_pills(ui, x, y, iw, w, lang, false);
    // The keyboard, unless the passphrase has the docked one up.
    if caret {
        y = crate::screens::word_keys(shift, w, ui, x - 4.0, y, iw + 8.0, 34.0);
    }
    y
}

/// The words taken on a small panel, as `count` boxes; `masked` draws a
/// word once taken as bullets. Returns its height.
pub(crate) fn keyed_taken(
    ui: &mut Ui,
    (x, y, iw): (f32, f32, f32),
    lang: faraday_bip::bip39::Language,
    taken: &[u16],
    (count, masked, never): (usize, bool, bool),
) -> f32 {
    let cols = 3;
    let cellw = (iw - (cols - 1) as f32 * 6.0) / cols as f32;
    for k in 0..count {
        let cx = x + (k % cols) as f32 * (cellw + 6.0);
        let cy = y + (k / cols) as f32 * 32.0;
        ui.fill(cx, cy, cellw, 28.0, 6.0, Role::SurfaceContainerHighest);
        ui.stroke(
            cx,
            cy,
            cellw,
            28.0,
            6.0,
            if k == taken.len() {
                Role::Primary.with_alpha(110)
            } else {
                Role::OutlineVariant.paint()
            },
        );
        ui.text_mid_as(
            (cx + 6.0, cy, 28.0),
            LABEL_S,
            Role::OnSurfaceVariant,
            &(k + 1).to_string(),
        );
        if let Some(&i) = taken.get(k) {
            let word = if masked {
                "••••••"
            } else {
                lang.word_display(i)
            };
            let t = ui.fit_as(BODY_S.mono(), word, cellw - 26.0);
            ui.text_mid_as((cx + 22.0, cy, 28.0), BODY_S.mono(), Role::OnSurface, &t);
        }
    }
    let rows_h = count.div_ceil(cols) as f32 * 32.0;
    if never {
        rows_h + 10.0 + crate::screens::grid_never(ui, x, y + rows_h - 4.0, iw)
    } else {
        rows_h + 10.0
    }
}

/// Add a key in another form: one share, string or part typed at a time
/// on the docked keyboard. New key's Words card for a key typed in that
/// form draws it too, with `flow` its Continue (`docs/history/NEW-WALLET.md`
/// §15.2): no passphrase (Key asks it), no way in from a stick, Continue
/// in place of Make the key. Returns where the next row starts.
pub(crate) fn entry_form(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    iw: f32,
    flow: Option<Action>,
) -> f32 {
    use crate::forms::Form;
    let form = app.entry.state().form;
    let mut y = y;
    // The codex32 form's tick, and the correction or a clash in place of
    // the field while one is up (`docs/CODEX32.md` §8.4, §8.5).
    let checking = form == Form::Codex32 && app.entry.state().codex32.checking;
    // On a small panel the field comes first, above the keyboard, and
    // the tick under it.
    let replaced = form == Form::Codex32 && crate::forms::codex32_replaced(app.entry.state());
    if replaced {
        y += crate::screens::codex32_head(app, ui, (x, y, iw), false).0;
    } else {
        y = entry_field(app, ui, x, y, iw, form);
        if form == Form::Codex32 {
            y += crate::screens::codex32_tick(app, ui, x, y, iw);
        }
    }
    if let Some(e) = &app.entry.state().error {
        y += ui.wrap_as((x, y, iw), BODY_M, Role::Error, e) + 8.0;
    }
    y += crate::screens::codex32_found(app, ui, x, y, iw);
    let lines = app.entry.state().parts.lines(form);
    y += heading(
        ui,
        x,
        y,
        &match lines.len() {
            0 => EN.compact_nothing_in_yet.to_string(),
            1 => EN.compact_count_one.to_string(),
            n => fill(EN.compact_count_many, &[&n.to_string()]),
        },
    );
    for (n, l) in lines.iter().enumerate() {
        ui.icon(x - 2.0, y - 2.0, 18.0, Icon::Done, 10.0, Role::Ok);
        let indent = faraday_draw::tokens::PART_INDENT_COMPACT;
        y += ui.wrap_as((x + indent, y, iw - indent), BODY_S, Role::OnSurface, l) + 6.0;
        y += crate::screens::codex32_mark(app, ui, x + indent, y, iw - indent, n);
    }
    y += 6.0;
    if form == Form::Codex32 {
        y += crate::screens::codex32_check_button(app, ui, x, y, iw);
    }
    entry_form_foot(app, ui, (x, y, iw), flow, (checking, replaced))
}

/// The field a share, string or part is typed into on a small panel,
/// wrapped to its text. Returns where the next row starts.
fn entry_field(app: &View, ui: &mut Ui, x: f32, y: f32, iw: f32, form: crate::forms::Form) -> f32 {
    use crate::forms::Form;
    let on = !app.entry.state().on_passphrase;
    let empty = app.entry.state().typed.is_empty();
    // Use a sample goes in the "Demo only" card while the field is empty.
    if app.entry_sample_shown() {
        crate::samples::put::offer(ui, Action::UseSample);
    }
    // What is typed may be a share or a seed's words: the copy drawn
    // here is wiped when the frame is done with it.
    let typed = if empty {
        zeroize::Zeroizing::new(match form {
            Form::Codex32 => "ms1…".to_string(),
            _ => EN.compact_words_of_one.to_string(),
        })
    } else {
        let held = app.entry.state().typed.as_str();
        let caret = if on { ui.caret_char() } else { "" };
        let mut t = zeroize::Zeroizing::new(String::with_capacity(held.len() + caret.len()));
        t.push_str(held);
        t.push_str(caret);
        t
    };
    // The caret stands before the hint while nothing is typed.
    if empty && on {
        ui.caret(x + 10.0, y + 9.0, 18.0);
    }
    let th = ui.wrap_as(
        (
            x + if empty && on { 16.0 } else { 10.0 },
            y + 10.0,
            iw - 20.0,
        ),
        BODY_M.mono(),
        if empty {
            Role::OnSurfaceVariant
        } else {
            Role::OnSurface
        },
        &typed,
    );
    let bh = th.max(20.0) + 20.0;
    ui.stroke(
        x,
        y,
        iw,
        bh,
        8.0,
        if on {
            Role::Primary
        } else {
            Role::OutlineVariant
        },
    );
    if !on {
        ui.hit(x, y, iw, bh, Action::EntryPassphrase);
    }
    y + bh + 10.0 + crate::screens::never_below(app, ui, x, y + bh, iw)
}

/// The codex32 form's or another form's foot on a small panel: the
/// passphrase where the form has one, Add this one (Check it while
/// checking a string), Scan a QR code and Make the key. Returns where the
/// next row starts.
fn entry_form_foot(
    app: &View,
    ui: &mut Ui,
    (x, y, iw): (f32, f32, f32),
    flow: Option<Action>,
    (checking, replaced): (bool, bool),
) -> f32 {
    use crate::forms::Form;
    let form = app.entry.state().form;
    let mut y = y;
    if form != Form::Codex32 && flow.is_none() {
        y += pass_field(
            ui,
            x,
            y,
            iw,
            EN.compact_passphrase,
            &app.entry.state().passphrase,
            (
                app.entry.state().on_passphrase,
                app.pass_chip(crate::demo::PassField::Entry),
            ),
            Action::EntryPassphrase,
        );
    }
    let ready = app.entry.state().parts.ready(form);
    y += stack(
        ui,
        x,
        y,
        iw,
        &[
            (
                if checking {
                    EN.codex32_form_check_it
                } else {
                    EN.compact_add_this_one
                },
                if app.entry.state().typed.trim().is_empty() || replaced {
                    Style::Disabled
                } else {
                    Style::Secondary
                },
                Action::EntryPart,
            ),
            (
                EN.compact_scan_a_qr_code,
                Style::Secondary,
                Action::ScanPart,
            ),
            (
                match flow {
                    Some(_) => EN.keygen_continue,
                    None => EN.compact_make_the_key,
                },
                Style::Primary.when(ready),
                flow.unwrap_or(Action::EntryRecover),
            ),
        ],
    );
    if flow.is_none() {
        crate::screens::entry_stick(app, ui, x, y + 4.0);
        y += 44.0;
    }
    y
}
