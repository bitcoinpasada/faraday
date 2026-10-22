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
    sub_tone: faraday_draw::Color,
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
    let lines = ui.wrap_lines(14.0, W::S, label, room);
    let line_h = ui.line(14.0, W::S);
    let h = 56.0 + (lines - 1) as f32 * line_h;
    ui.fill(x, y, w, h, 10.0, if pressed { INNER } else { SURFACE });
    ui.stroke(x, y, w, h, 10.0, BORDER);
    if let Some(i) = icon {
        let (bg, fg) = if enabled {
            (ACCENT.with_alpha(26), ACCENT)
        } else {
            (INNER, DIM)
        };
        ui.fill(x + 10.0, y + 12.0, 32.0, 32.0, 8.0, bg);
        ui.icon(x + 10.0, y + 12.0, 32.0, i, 14.0, fg);
    }
    let label_color = if enabled { TEXT } else { DIM };
    let sub_y = if lines > 1 {
        ui.wrap(tx, y + 9.0, room, 14.0, W::S, label_color, label);
        y + 9.0 + lines as f32 * line_h
    } else {
        let label = ui.fit(14.0, W::S, label, room);
        ui.text(tx, y + 9.0, 14.0, W::S, label_color, &label);
        y + 30.0
    };
    if !sub.is_empty() {
        let sub = ui.fit(12.0, W::R, sub, room);
        ui.text(tx, sub_y, 12.0, W::R, sub_tone, &sub);
    }
    if let Some(a) = action {
        ui.icon(x + w - 24.0, y + 18.0, 20.0, Icon::ChevronRight, 9.0, DIM);
        ui.hit(x, y, w, h, a);
    }
    h + 8.0
}

/// Full-width buttons, one under another. Returns their height.
pub(crate) fn stack(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, Style, Action)]) -> f32 {
    let mut cy = y;
    for &(label, style, action) in items {
        ui.button(x, cy, Some(w), 44.0, label, style, action);
        cy += 52.0;
    }
    cy - y
}

/// A section's heading. Returns its height.
pub(crate) fn heading(ui: &mut Ui, x: f32, y: f32, s: &str) -> f32 {
    ui.text(x, y, 13.0, W::S, MUTED, s);
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
    let Some(wlt) = app.session.wallets.get(app.wallet) else {
        if app.session.loose_keys().is_empty() {
            ui.text(x, cy, 14.0, W::R, MUTED, EN.compact_no_wallet_loaded);
        } else {
            cy += crate::screens::loose_card(app, ui, x, cy, iw);
            finish(app, ui, x0, cw, h, cy - top + 8.0);
        }
        return;
    };
    // The chart, as its own page.
    if app.glance
        && let Some(g) = app.glance_of(app.wallet)
    {
        cy += crate::glance::draw_column(ui, &g, crate::glance::Direction::WalletFirst, x, cy, iw);
        finish(app, ui, x0, cw, h, cy - top + 8.0);
        return;
    }

    // A PSBT waiting leads, unless it is this wallet's: the card's first
    // button is for that.
    let lead = match app.spend.as_ref() {
        Some(s) if s.wallet == Some(app.wallet) => None,
        Some(s) => Some((
            fill(EN.compact_continue_signing, &[&s.spend.source.to_string()]),
            Action::Nav(Screen::Spend),
        )),
        None if app.psbt_for(app.wallet).is_some() => None,
        None => app.lead_psbt().map(|i| {
            (
                fill(EN.compact_review, &[&app.inbox[i].name.to_string()]),
                Action::StartSpend(i),
            )
        }),
    };
    if let Some((line, action)) = lead {
        let bg = if ui.is_pressed(action) {
            ACCENT.with_alpha(40)
        } else {
            ACCENT.with_alpha(18)
        };
        ui.fill(x, cy, iw, 44.0, 10.0, bg);
        ui.stroke(x, cy, iw, 44.0, 10.0, ACCENT.with_alpha(76));
        ui.icon(x + 10.0, cy + 12.0, 20.0, Icon::File, 12.0, ACCENT);
        let line = ui.fit(13.0, W::S, &line, iw - 64.0);
        ui.text_mid(x + 36.0, cy, 44.0, 13.0, W::S, TEXT, &line);
        ui.icon(
            x + iw - 24.0,
            cy + 12.0,
            20.0,
            Icon::ChevronRight,
            9.0,
            ACCENT,
        );
        ui.hit(x, cy, iw, 44.0, action);
        cy += 56.0;
    }

    // Its name, which can be changed.
    match app.renaming.as_ref() {
        Some(name) => {
            ui.fill(x, cy, iw, 44.0, 8.0, BG);
            ui.stroke(x, cy, iw, 44.0, 8.0, ACCENT.with_alpha(110));
            ui.typed(
                x + 10.0,
                cy,
                44.0,
                16.0,
                W::S,
                TEXT,
                name,
                iw - 24.0,
                true,
                Action::Rename,
            );
            ui.hit(x, cy, iw, 44.0, Action::Rename);
            cy += 52.0;
        }
        None => {
            let rw = ui.measure(13.0, W::S, EN.compact_rename) + 24.0;
            let name_h = ui.wrap(x, cy + 4.0, iw - rw - 8.0, 18.0, W::S, TEXT, &wlt.name);
            ui.button(
                x + iw - rw,
                cy,
                Some(rw),
                34.0,
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
    cy += ui.wrap(x, cy, iw, 12.0, W::R, MUTED, &line) + 8.0;
    // Where its backup is (§5.2): a press opens Backups on it.
    cy += crate::screens::backup_line(app, ui, x, cy, iw, app.wallet);

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
    let can = app.can_spend(app.wallet);
    let box_h = if can { 76.0 } else { 90.0 };
    ui.fill(x, cy, iw, box_h, 10.0, BG);
    ui.stroke(x, cy, iw, box_h, 10.0, INNER);
    let pw = ui.pips(x + 14.0, cy + 16.0, m, here);
    let sigs = fill(
        EN.compact_of_signatures,
        &[&here.to_string(), &m.to_string()],
    );
    let sigs = ui.fit(14.0, W::S, &sigs, iw - pw - 40.0);
    ui.text_mid(x + 26.0 + pw, cy + 4.0, 32.0, 14.0, W::S, TEXT, &sigs);
    if can {
        ui.chip(
            x + 14.0,
            cy + 38.0,
            EN.compact_can_spend,
            OK,
            OK.with_alpha(30),
        );
    } else {
        ui.button(
            x + 10.0,
            cy + 40.0,
            Some(iw - 20.0),
            40.0,
            EN.compact_scan_a_key,
            if app.may_load_keys() {
                Style::Primary
            } else {
                Style::Disabled
            },
            Action::ScanKey(app.wallet),
        );
    }
    cy += box_h + 16.0;

    // The first thing to do with it, on the first screen: from the vault
    // to a spend (`docs/history/NEW-WALLET.md` §11.2).
    let (spend_label, spend_action) = app.spend_button(app.wallet);
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
    if let Some(g) = app.glance_of(app.wallet) {
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
            MUTED,
            Some(Action::Glance(true)),
        );
    }
    if !quorum_here {
        cy += 4.0;
        cy += missing_keys_line(app, ui, &slots, Screen::Wallets, x, cy, iw);
    }
    cy += 10.0;
    cy += heading(ui, x, cy, EN.compact_first_receive);
    let (addr, held) = app.receive_line(wlt, false, 0);
    cy += ui.wrap(x, cy, iw, 14.0, W::M, if held { WARN } else { TEXT }, &addr) + 18.0;

    // What else can be done with it.
    let mut buttons = vec![
        (
            EN.compact_show_wallet,
            Style::Secondary,
            Action::QrWallet(app.wallet),
        ),
        (
            if app.wallet_backup_unfinished(app.wallet) {
                EN.backup_flow_continue
            } else {
                EN.compact_back_up
            },
            Style::Secondary,
            Action::Backup(app.wallet),
        ),
    ];
    // Create filled from this wallet (`docs/history/NEW-WALLET.md` §16.2).
    if crate::create::NewKind::of(wlt.policy()).is_some_and(|k| !k.threshold()) {
        buttons.push((
            EN.wallets_new_wallet_from,
            Style::Secondary,
            Action::NewWalletFrom(app.wallet),
        ));
    }
    if app.session.message_wallets().contains(&app.wallet) {
        buttons.push((
            EN.compact_sign_a_message,
            Style::Secondary,
            Action::SignMessage,
        ));
    }
    buttons.push((
        EN.compact_remove_from,
        Style::Ghost,
        Action::RemoveWallet(app.wallet),
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
    ui.text_mid(x, cy, 30.0, 14.0, W::M, TEXT, &fp_text(r.fingerprint));
    let (state, fg, bg) = if held.is_some() {
        (EN.compact_here, OK, OK.with_alpha(30))
    } else {
        (EN.compact_not_here, MUTED, INNER)
    };
    let chipw = ui.measure(12.0, W::R, state) + 34.0;
    ui.chip(x + iw - chipw, cy + 3.0, state, fg, bg);
    cy += 30.0;
    if let Some(label) = held {
        if !label.trim().eq_ignore_ascii_case(&fp_text(r.fingerprint)) {
            ui.text(x, cy, 12.0, W::R, MUTED, label);
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
        let style = if may { style } else { Style::Disabled };
        ui.button(x, cy, Some(iw), 36.0, label, style, action);
        cy += 44.0;
    }
    cy += 10.0;
    cy += heading(ui, x, cy, EN.compact_address);
    cy += ui.wrap(x, cy, iw, 14.0, W::M, TEXT, &r.address()) + 14.0;
    ui.text(x, cy, 13.0, W::S, MUTED, EN.compact_labels_handed);
    ui.text_right(
        x + iw,
        cy - 6.0,
        28.0,
        13.0,
        W::M,
        TEXT,
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
                Action::SWallet(app.wallet),
            ),
            (
                EN.compact_show_wallet,
                Style::Secondary,
                Action::QrWallet(app.wallet),
            ),
            (
                EN.compact_remove_from,
                Style::Ghost,
                Action::RemoveWallet(app.wallet),
            ),
        ],
    );
    finish(app, ui, 0.0, x + iw + M, h, cy - top + 8.0);
}

/// What explains a page, after its controls and behind a tap, as a
/// step's is. Returns its height.
pub(crate) fn about(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    crate::flow::about(ui, x, y, w, EN.compact_about_this, crate::ABOUT_PAGE, text)
}

/// Reports how tall the page is, so it scrolls.
pub(crate) fn finish(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32, content: f32) {
    app.content_h.set(content);
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
    on: bool,
    action: Action,
) -> f32 {
    ui.text(x, y, 12.0, W::R, MUTED, label);
    let fy = y + 18.0;
    ui.fill(x, fy, w, 42.0, 8.0, BG);
    ui.stroke(x, fy, w, 42.0, 8.0, if on { ACCENT } else { BORDER });
    if secret.is_empty() {
        ui.no_passphrase(x, fy, 42.0, on);
    } else {
        let dots = "•".repeat(secret.chars().count());
        ui.typed(
            x + 12.0,
            fy,
            42.0,
            14.0,
            W::M,
            TEXT,
            &dots,
            w - 30.0,
            on,
            action,
        );
    }
    ui.hit(x, fy, w, 42.0, action);
    18.0 + 42.0 + 12.0
}

/// Add a key on a small panel. Words: what is being typed, the words it
/// can be, and the word keyboard on the first screenful; the
/// words taken, the passphrase and the buttons under them. The other
/// forms type on the docked keyboard.
pub(crate) fn entry(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::forms::Form;
    let (x, iw) = (x0 + M, cw - 2.0 * M);
    let top = -app.list_offset;
    let mut y = top + 10.0;
    let for_wallet = app
        .entry
        .wallet
        .and_then(|w| app.session.wallets.get(w))
        .map(|wl| fill(EN.compact_for, &[&wl.name.to_string()]));
    let wanted = app.entry.wanted.map(|fp| {
        fill(
            EN.compact_for_key,
            &[&fp_text(faraday_bip::keys::Fingerprint(fp)).to_string()],
        )
    });
    if let Some(line) = wanted.or(for_wallet) {
        let line = ui.fit(13.0, W::S, &line, iw);
        ui.text(x, y, 13.0, W::S, ACCENT, &line);
        y += 22.0;
    }
    if app.entry.other_forms {
        let forms: Vec<(String, Style, Action)> = Form::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.name().to_string(),
                    if *f == app.entry.form {
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
    if app.entry.form != Form::Words {
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
        40.0,
        EN.compact_scan_a_seedqr,
        if typed {
            Style::Secondary
        } else {
            Style::Primary
        },
        crate::screens::entry_scan(app),
    );
    y += 48.0;
    if app.entry.keys.is_none() {
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
        &app.entry.passphrase,
        app.entry.on_passphrase,
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
    let lang = app.entry.language();
    let english = lang == faraday_bip::bip39::Language::English;
    // One button for the list in use; the lists open from it; unless
    // the chips are already shown, Other forms beside it.
    let mut langs: Vec<(String, Style, Action)> = if !app.entry.languages {
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
    if page && !app.entry.other_forms {
        langs.insert(
            0,
            (
                EN.compact_other_forms.to_string(),
                Style::Ghost,
                Action::EntryOtherForms,
            ),
        );
    }
    let Some(w) = app.entry.keys.as_deref() else {
        y += never_real_seed(app, ui, x, y, iw);
        return y + crate::screens::button_rows(ui, x, y, iw, &langs);
    };
    let taken: Vec<u16> = w.committed_indices().collect();
    let (status, tone) = match &app.entry.error {
        Some(e) => (e.clone(), ERR),
        None => crate::screens::keyed_status(app, lang, &taken),
    };
    y = keyed_head(
        ui,
        (x, y, iw),
        w,
        lang,
        &fill(EN.compact_word, &[&(taken.len() + 1).to_string()]),
        Some((&status, tone)),
        (!app.entry.on_passphrase, app.entry.shift),
    );
    // The words taken, as many boxes as the count chosen.
    y += 6.0;
    y += heading(ui, x, y, EN.compact_words);
    y += crate::screens::word_count_row(app, ui, x, y, iw, false);
    y += keyed_taken(
        ui,
        (x, y, iw),
        lang,
        &taken,
        (app.entry.word_boxes(taken.len() + 1), false, app.online),
    );
    // Under the word fields.
    y + crate::screens::button_rows(ui, x, y, iw, &langs) - 6.0
}

/// A list typed on its own keyboard, on a small panel: the word being
/// typed under `label`, `status` under it, the words it can be as pills
/// and the keyboard, with the on-screen keyboard's shift as `shift`. Add a
/// key's word entry and Check my copy's both draw it. Returns where the
/// next row starts.
pub(crate) fn keyed_head(
    ui: &mut Ui,
    (x, y, iw): (f32, f32, f32),
    w: &crate::word_entry::WordEntry,
    lang: faraday_bip::bip39::Language,
    label: &str,
    status: Option<(&str, faraday_draw::Color)>,
    (caret, shift): (bool, bool),
) -> f32 {
    let mut y = y;
    let prefix: String = w.prefix().iter().collect();
    // The word being typed.
    ui.text(x, y, 12.0, W::R, MUTED, label);
    let off = (ui.measure(12.0, W::R, label) + 12.0).max(64.0);
    let shown = if prefix.is_empty() && !caret {
        "…".to_string()
    } else {
        prefix.clone()
    };
    let pw = ui.text(x + off, y - 4.0, 18.0, W::M, ACCENT, &shown);
    if caret {
        ui.caret(x + off + 2.0 + pw, y - 2.0, 20.0);
    }
    y += 26.0;
    if let Some((status, tone)) = status {
        let status = ui.fit(12.0, W::S, status, iw);
        ui.text(x, y, 12.0, W::S, tone, &status);
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
        ui.fill(cx, cy, cellw, 28.0, 6.0, BG);
        ui.stroke(
            cx,
            cy,
            cellw,
            28.0,
            6.0,
            if k == taken.len() {
                ACCENT.with_alpha(110)
            } else {
                INNER
            },
        );
        ui.text_mid(cx + 6.0, cy, 28.0, 10.0, W::R, DIM, &(k + 1).to_string());
        if let Some(&i) = taken.get(k) {
            let word = if masked {
                "••••••"
            } else {
                lang.word_display(i)
            };
            let t = ui.fit(12.0, W::M, word, cellw - 26.0);
            ui.text_mid(cx + 22.0, cy, 28.0, 12.0, W::M, TEXT, &t);
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
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn entry_form(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    iw: f32,
    flow: Option<Action>,
) -> f32 {
    use crate::forms::Form;
    let form = app.entry.form;
    let mut y = y;
    let on = !app.entry.on_passphrase;
    let empty = app.entry.typed.is_empty();
    let typed = if empty {
        match form {
            Form::Codex32 => "ms1…".to_string(),
            _ => EN.compact_words_of_one.to_string(),
        }
    } else {
        let mut t = app.entry.typed.to_string();
        if on {
            t.push_str(ui.caret_char());
        }
        t
    };
    // The caret stands before the hint while nothing is typed.
    if empty && on {
        ui.caret(x + 10.0, y + 9.0, 18.0);
    }
    let th = ui.wrap(
        x + if empty && on { 16.0 } else { 10.0 },
        y + 10.0,
        iw - 20.0,
        13.0,
        W::M,
        if empty { DIM } else { TEXT },
        &typed,
    );
    let bh = th.max(20.0) + 20.0;
    ui.stroke(x, y, iw, bh, 8.0, if on { ACCENT } else { BORDER });
    if !on {
        ui.hit(x, y, iw, bh, Action::EntryPassphrase);
    }
    y += bh + 10.0 + crate::screens::never_below(app, ui, x, y + bh, iw);
    if let Some(e) = &app.entry.error {
        y += ui.wrap(x, y, iw, 13.0, W::R, ERR, e) + 8.0;
    }
    let lines = app.entry.parts.lines(form);
    y += heading(
        ui,
        x,
        y,
        &match lines.len() {
            0 => EN.compact_nothing_in.to_string(),
            1 => EN.compact_1_in.to_string(),
            n => fill(EN.compact_in, &[&n.to_string()]),
        },
    );
    for l in &lines {
        ui.icon(x - 2.0, y - 2.0, 18.0, Icon::Done, 10.0, OK);
        y += ui.wrap(x + 20.0, y, iw - 20.0, 12.0, W::R, TEXT, l) + 6.0;
    }
    y += 6.0;
    if form != Form::Codex32 && flow.is_none() {
        y += pass_field(
            ui,
            x,
            y,
            iw,
            EN.compact_passphrase,
            &app.entry.passphrase,
            app.entry.on_passphrase,
            Action::EntryPassphrase,
        );
    }
    let ready = app.entry.parts.ready(form);
    y += stack(
        ui,
        x,
        y,
        iw,
        &[
            (
                EN.compact_add_this_one,
                if app.entry.typed.trim().is_empty() {
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
                if ready {
                    Style::Primary
                } else {
                    Style::Disabled
                },
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
