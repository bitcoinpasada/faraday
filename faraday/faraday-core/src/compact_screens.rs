//! Screens laid out for a small panel (`Faraday::is_compact`), where the
//! desktop's layout has columns side by side that a 268 dp panel has no
//! room for. Each is a single scrolled column under the page's bar.

use osk_ui::widgets::Icon;

use crate::compact::M;
use crate::screens::missing_keys_line;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, grouped};
use crate::wallet::{Session, fp_text};
use crate::{Action, Faraday, Screen};

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
    sub_tone: osk_ui::Color,
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
    ui.stroke(x, y, w, h, 10.0, LINE);
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
pub(crate) fn wallet_card(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let (x, iw) = (x0 + M, cw - 2.0 * M);
    let top = -app.list_offset;
    let mut cy = top + 12.0;
    let Some(wlt) = app.session.wallets.get(app.wallet) else {
        if app.session.loose_keys().is_empty() {
            ui.text(x, cy, 14.0, W::R, MUTED, "No wallet loaded");
        } else {
            cy += crate::screens::loose_card(app, ui, x, cy, iw);
            finish(app, ui, x0, cw, h, cy - top + 8.0);
        }
        return;
    };
    // The chart, as its own page.
    if app.glance
        && let Some(g) = crate::glance::of(app, app.wallet)
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
            format!("Continue signing {}", s.spend.source),
            Action::Nav(Screen::Spend),
        )),
        None if app.psbt_for(app.wallet).is_some() => None,
        None => app.lead_psbt().map(|i| {
            (
                format!("Review {}", app.inbox[i].name),
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
            let rw = ui.measure(13.0, W::S, "Rename") + 24.0;
            let name_h = ui.wrap(x, cy + 4.0, iw - rw - 8.0, 18.0, W::S, TEXT, &wlt.name);
            ui.button(
                x + iw - rw,
                cy,
                Some(rw),
                34.0,
                "Rename",
                Style::Ghost,
                Action::Rename,
            );
            cy += name_h.max(30.0) + 10.0;
        }
    }
    let line = format!(
        "{} · {}",
        Session::shape(wlt),
        crate::wallet::source_tag(&wlt.source, false)
    );
    cy += ui.wrap(x, cy, iw, 12.0, W::R, MUTED, &line) + 8.0;
    // Where its backup is (§5.2): a press opens Backups on it.
    cy += crate::screens::backup_line(app, ui, x, cy, iw, app.wallet);

    if let Some(r) = wlt.policy.silent() {
        silent_card(app, ui, r, x, iw, cy, top, h);
        return;
    }

    // Signatures possible here.
    let (m, _) = Session::quorum(wlt);
    let slots = app.session.slots(wlt);
    let here = slots.iter().filter(|s| s.held_by.is_some()).count().min(m);
    let short_by = m - here;
    let need = if short_by == 0 {
        "Nothing missing".to_string()
    } else {
        let more = if here > 0 { " more" } else { "" };
        format!(
            "Add {short_by}{more} {} or collect {short_by}{more} {}",
            if short_by == 1 { "key" } else { "keys" },
            if short_by == 1 {
                "signature"
            } else {
                "signatures"
            }
        )
    };
    let need_h = {
        let mark = ui.hits.len();
        ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
        let nh = ui.wrap(x + 14.0, 0.0, iw - 28.0, 12.0, W::R, TEXT, &need);
        ui.c.pop_clip();
        ui.hits.truncate(mark);
        nh
    };
    let box_h = 40.0 + need_h + 10.0;
    ui.fill(x, cy, iw, box_h, 10.0, BG);
    ui.stroke(x, cy, iw, box_h, 10.0, INNER);
    let pw = ui.pips(x + 14.0, cy + 16.0, m, here);
    let sigs = format!("{here} of {m} signatures here");
    let sigs = ui.fit(14.0, W::S, &sigs, iw - pw - 40.0);
    ui.text_mid(x + 26.0 + pw, cy + 4.0, 32.0, 14.0, W::S, TEXT, &sigs);
    ui.wrap(
        x + 14.0,
        cy + 38.0,
        iw - 28.0,
        12.0,
        W::R,
        if short_by == 0 { OK } else { WARN },
        &need,
    );
    cy += box_h + 16.0;

    // The first thing to do with it, on the first screen: from the vault
    // to a spend (`docs/NEW-WALLET.md` §11.2).
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
    if let Some(g) = crate::glance::of(app, app.wallet) {
        let sub = format!(
            "{} {} · {}",
            g.keys.len(),
            if g.keys.len() == 1 { "key" } else { "keys" },
            match &g.backup {
                crate::glance::Backup::Plan { nodes, .. } => format!(
                    "{} backup {}",
                    nodes.len(),
                    if nodes.len() == 1 { "place" } else { "places" }
                ),
                crate::glance::Backup::Locked { vault, .. } => format!("plan in {vault}, locked"),
                crate::glance::Backup::None => "no backup plan".to_string(),
            }
        );
        cy += row(
            ui,
            x,
            cy,
            iw,
            None,
            "At a glance",
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
    cy += heading(ui, x, cy, "First receive address · 0/0");
    let addr = grouped(&app.session.address_shown(wlt, false, 0));
    cy += ui.wrap(x, cy, iw, 14.0, W::M, TEXT, &addr) + 18.0;
    if !app.wallet_held(app.wallet).is_empty() {
        cy += ui.wrap(x, cy, iw, 13.0, W::S, WARN, crate::glance::RECEIVE_WARNING) + 12.0;
    }

    // What else can be done with it.
    let mut buttons = vec![
        (
            "Show wallet QR",
            Style::Secondary,
            Action::QrWallet(app.wallet),
        ),
        ("Back up", Style::Secondary, Action::Backup(app.wallet)),
    ];
    if app.session.message_wallets().contains(&app.wallet) {
        buttons.push(("Sign a message", Style::Secondary, Action::SignMessage));
    }
    buttons.push((
        "Remove from session",
        Style::Ghost,
        Action::RemoveWallet(app.wallet),
    ));
    cy += stack(ui, x, cy, iw, &buttons);
    finish(app, ui, x0, cw, h, cy - top + 8.0);
}

/// A silent payments wallet's card on a small panel.
#[allow(clippy::too_many_arguments)]
fn silent_card(
    app: &Faraday,
    ui: &mut Ui,
    r: &osk_bip::silent_wallet::SilentWallet,
    x: f32,
    iw: f32,
    y: f32,
    top: f32,
    h: f32,
) {
    let may = app.may_load_keys();
    let mut cy = y;
    cy += heading(ui, x, cy, "Key");
    let held = app.session.key_label(r.fingerprint);
    ui.text_mid(x, cy, 30.0, 14.0, W::M, TEXT, &fp_text(r.fingerprint));
    let (state, fg, bg) = if held.is_some() {
        ("Here", OK, OK.with_alpha(30))
    } else {
        ("Not here", MUTED, INNER)
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
        let (label, style, action) = if app.vault_key_for(r.fingerprint).is_some() {
            (
                "Load from vault",
                Style::Primary,
                Action::Vault(crate::vaults::VaultAction::LoadKeyOf(r.fingerprint.0)),
            )
        } else {
            (
                "Add its key",
                Style::Secondary,
                Action::Entry(Some(r.fingerprint.0)),
            )
        };
        let style = if may { style } else { Style::Disabled };
        ui.button(x, cy, Some(iw), 36.0, label, style, action);
        cy += 44.0;
    }
    cy += 10.0;
    cy += heading(ui, x, cy, "Address");
    cy += ui.wrap(
        x,
        cy,
        iw,
        14.0,
        W::M,
        TEXT,
        &app.session.shown(&r.address()),
    ) + 14.0;
    ui.text(x, cy, 13.0, W::S, MUTED, "Labels handed out");
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
                "Address, labels and payments",
                Style::Primary,
                Action::SWallet(app.wallet),
            ),
            (
                "Show wallet QR",
                Style::Secondary,
                Action::QrWallet(app.wallet),
            ),
            (
                "Remove from session",
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
    crate::flow::about(ui, x, y, w, "About this page", crate::ABOUT_PAGE, text)
}

/// The height `draw` takes, drawing nothing and keeping no hits: what a
/// card is measured with before its surface goes down.
pub(crate) fn measured(ui: &mut Ui, draw: impl FnOnce(&mut Ui) -> f32) -> f32 {
    let mark = ui.hits.len();
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let h = draw(ui);
    ui.c.pop_clip();
    ui.hits.truncate(mark);
    h
}

/// Reports how tall the page is, so it scrolls.
pub(crate) fn finish(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32, content: f32) {
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
/// can be, and OpenSigner's word keyboard on the first screenful; the
/// words taken, the passphrase and the buttons under them. The other
/// forms type on the docked keyboard.
pub(crate) fn entry(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::forms::Form;
    let (x, iw) = (x0 + M, cw - 2.0 * M);
    let top = -app.list_offset;
    let mut y = top + 10.0;
    if let Some(fp) = app.entry.wanted {
        let line = format!("For key {}", fp_text(osk_bip::keys::Fingerprint(fp)));
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
        entry_form(app, ui, x0, cw, h, x, y, iw, top);
        return;
    }
    let lang = app.entry.language();
    let english = lang == osk_bip::bip39::Language::English;
    // One button for the list in use; the lists open from it; unless
    // the chips are already shown, Other forms beside it.
    let mut langs: Vec<(String, Style, Action)> = if !app.entry.languages {
        vec![(
            if english {
                "Other languages".to_string()
            } else {
                format!("{} · other lists", crate::forms::language_name(lang))
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
    if !app.entry.other_forms {
        langs.insert(
            0,
            (
                "Other forms".to_string(),
                Style::Ghost,
                Action::EntryOtherForms,
            ),
        );
    }
    let Some(w) = app.entry.keys.as_deref() else {
        crate::screens::button_rows(ui, x, y, iw, &langs);
        return;
    };
    let taken: Vec<u16> = w.committed_indices().collect();
    let prefix: String = w.prefix().iter().collect();
    // The word being typed, and the last ones taken before it.
    ui.text(
        x,
        y,
        12.0,
        W::R,
        MUTED,
        &format!("Word {}", taken.len() + 1),
    );
    let on = !app.entry.on_passphrase;
    let shown = if prefix.is_empty() && !on {
        "…".to_string()
    } else {
        prefix.clone()
    };
    let pw = ui.text(x + 64.0, y - 4.0, 18.0, W::M, ACCENT, &shown);
    if on {
        ui.caret(x + 66.0 + pw, y - 2.0, 20.0);
    }
    let (status, tone) = match &app.entry.error {
        Some(e) => (e.clone(), ERR),
        None => crate::screens::keyed_status(app, lang, &taken),
    };
    y += 26.0;
    let status = ui.fit(12.0, W::S, &status, iw);
    ui.text(x, y, 12.0, W::S, tone, &status);
    y += 22.0;
    // What the keys typed can be: a pill each.
    let cands: Vec<u16> = w.candidates().collect();
    if cands.is_empty() {
        let line = if prefix.is_empty() {
            "Type the next word"
        } else if w.awaiting_tone() {
            "Now its tone"
        } else {
            "No word starts with that"
        };
        ui.text_mid(x, y, 30.0, 13.0, W::R, MUTED, line);
        y += 34.0;
    } else {
        let mut bx = x;
        for (n, &i) in cands.iter().enumerate() {
            let word = lang.word_display(i);
            let bw = ui.measure(14.0, W::M, word) + 24.0;
            if bx > x && bx + bw > x + iw {
                bx = x;
                y += 32.0;
            }
            bx += ui.word_pill(bx, y, word, Action::EntryCandidate(n as u8)) + 6.0;
        }
        y += 36.0;
    }
    // The keyboard, unless the passphrase has the docked one up.
    if !app.entry.on_passphrase {
        y = crate::screens::word_keys(app, w, ui, x - 4.0, y, iw + 8.0, 34.0);
    }
    // The words taken.
    let count = if taken.len() >= 12 { 24 } else { 12 };
    y += 6.0;
    y += heading(ui, x, y, "Words");
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
            let t = ui.fit(12.0, W::M, lang.word_display(i), cellw - 26.0);
            ui.text_mid(cx + 22.0, cy, 28.0, 12.0, W::M, TEXT, &t);
        }
    }
    y += (count / cols) as f32 * 32.0 + 10.0;
    // Under the word fields.
    y += crate::screens::button_rows(ui, x, y, iw, &langs) - 6.0;
    y += pass_field(
        ui,
        x,
        y,
        iw,
        "BIP-39 passphrase",
        &app.entry.passphrase,
        app.entry.on_passphrase,
        Action::EntryPassphrase,
    );
    // Add key is pinned at the foot.
    ui.pin = Some(("Add key".to_string(), Style::Primary, Action::EntryAdd));
    y += stack(
        ui,
        x,
        y,
        iw,
        &[
            ("Scan a SeedQR", Style::Secondary, Action::ScanSeed),
            ("Make a new key", Style::Secondary, Action::KeyGen(None)),
            ("Clear", Style::Ghost, Action::EntryClear),
        ],
    );
    crate::screens::entry_stick(app, ui, x, y + 4.0);
    y += 44.0;
    finish(app, ui, x0, cw, h, y - top + 8.0);
}

/// Add a key in another form: one share, string or part typed at a time
/// on the docked keyboard.
#[allow(clippy::too_many_arguments)]
fn entry_form(
    app: &Faraday,
    ui: &mut Ui,
    x0: f32,
    cw: f32,
    h: f32,
    x: f32,
    y: f32,
    iw: f32,
    top: f32,
) {
    use crate::forms::Form;
    let form = app.entry.form;
    let mut y = y;
    let on = !app.entry.on_passphrase;
    let empty = app.entry.typed.is_empty();
    let typed = if empty {
        match form {
            Form::Codex32 => "ms1…".to_string(),
            _ => "Words of one share".to_string(),
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
    y += bh + 10.0;
    if let Some(e) = &app.entry.error {
        y += ui.wrap(x, y, iw, 13.0, W::R, ERR, e) + 8.0;
    }
    let lines = app.entry.parts.lines(form);
    y += heading(
        ui,
        x,
        y,
        &match lines.len() {
            0 => "Nothing in yet".to_string(),
            1 => "1 in".to_string(),
            n => format!("{n} in"),
        },
    );
    for l in &lines {
        ui.icon(x - 2.0, y - 2.0, 18.0, Icon::Done, 10.0, OK);
        y += ui.wrap(x + 20.0, y, iw - 20.0, 12.0, W::R, TEXT, l) + 6.0;
    }
    y += 6.0;
    if form != Form::Codex32 {
        y += pass_field(
            ui,
            x,
            y,
            iw,
            "Passphrase",
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
                "Add this one",
                if app.entry.typed.trim().is_empty() {
                    Style::Disabled
                } else {
                    Style::Secondary
                },
                Action::EntryPart,
            ),
            ("Scan a QR code", Style::Secondary, Action::ScanPart),
            (
                "Make the key",
                if ready {
                    Style::Primary
                } else {
                    Style::Disabled
                },
                Action::EntryRecover,
            ),
        ],
    );
    crate::screens::entry_stick(app, ui, x, y + 4.0);
    y += 44.0;
    finish(app, ui, x0, cw, h, y - top + 8.0);
}
