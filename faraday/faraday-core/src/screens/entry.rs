//! `Screen::Entry`: typing a seed.

use super::parts::{
    Cells, dots_field, guide_text, keyed_cells, keyed_pills, never_below, typed_cells, word_keys,
};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{CoreUi, Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Screen};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{button_rows, section_label};

/// The one word the typed prefix can complete to, if there is one.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        crate::compact::screen::entry(app, ui, x0, cw, h);
        return;
    }
    let x = x0 + 56.0;
    let width = (cw - 112.0).min(900.0);
    // The word grid can run to 24 cells, taller than the window; the
    // page scrolls, as Tools' and Files' do (`app.list_offset`).
    let top = 36.0 - app.list_offset;
    let mut y = top;
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    let back = Action::Nav(app.entry_leave());
    ui.icon(x - 4.0, y, 16.0, Icon::ChevronLeft, 10.0, MUTED);
    ui.text(
        x + 14.0,
        y,
        12.0,
        W::R,
        MUTED,
        app.named_back((
            if app.entry_leave() == Screen::Family {
                EN.entry_spend
            } else {
                EN.entry_wallets
            },
            back,
        ))
        .0,
    );
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, back);
    ui.back(back);
    y += 22.0;
    let for_wallet = app
        .entry
        .wallet
        .and_then(|w| app.session.wallets.get(w))
        .map(|wl| wl.name.as_str());
    let heading = match (app.entry.wanted, for_wallet) {
        (Some(fp), _) => fill(
            EN.entry_add_key,
            &[&fp_text(faraday_bip::keys::Fingerprint(fp)).to_string()],
        ),
        (None, Some(name)) => fill(EN.entry_add_a_key_for, &[name]),
        (None, None) if app.entry.form != crate::forms::Form::Words => {
            fill(EN.entry_add_a_key_n, &[app.entry.form.name()])
        }
        (None, None) => EN.entry_add_a_key.to_string(),
    };
    let heading = ui.fit(26.0, W::S, &heading, width);
    ui.text(x, y, 26.0, W::S, TEXT, &heading);
    y += 48.0;
    // The words form is the one everyday way in; the other three are
    // behind Other forms, or already shown when a Tools tile opened one
    // of them (`docs/history/SIMPLIFY.md` §2.4).
    if app.entry.other_forms {
        let forms: Vec<(String, Style, Action)> = crate::forms::Form::ALL
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
        y += button_rows(ui, x, y, width, &forms) + 4.0;
    }
    if app.entry.form != crate::forms::Form::Words {
        let bottom = entry_form(app, ui, x, y, width, h, None);
        entry_scrolled(app, ui, clip, h, bottom);
        return;
    }
    // Scan leads, above the words: primary until a word is typed, when
    // Add key is (`docs/history/SCAN-TO-SIGN.md` §2.5).
    ui.button(
        x,
        y,
        Some(width),
        40.0,
        EN.entry_scan_a_seedqr,
        if entry_typed(app) {
            Style::Secondary
        } else {
            Style::Primary
        },
        entry_scan(app),
    );
    y += 52.0;
    y = entry_words(app, ui, x, y, width, h, true);
    let bottom = entry_finish(app, ui, x, y, width, h);
    entry_scrolled(app, ui, clip, h, bottom);
}

/// Add a key's words form, from the list the words are from to the
/// status under them: the word count, the word fields or a list's own
/// keyboard, the words the letters can still become, and the count and
/// checksum. Add a key draws it, and so does New key's Words card for a
/// key typed (`docs/history/NEW-WALLET.md` §15.2). `page`: on Add a key's own
/// page, whose row of lists offers Other forms too. Returns where the
/// next row starts.
pub(crate) fn entry_words(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    h: f32,
    page: bool,
) -> f32 {
    let mut y = y;
    // Which list the words are from: English, the others one press away;
    // and, unless the chips are already shown, the way to them. Under the
    // word fields, or above the keyboard of a list typed on its own keys.
    let lang = app.entry.language();
    let english = lang == faraday_bip::bip39::Language::English;
    let mut langs: Vec<(String, Style, Action)> = if english && !app.entry.languages {
        vec![(
            EN.entry_other_languages.to_string(),
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
                EN.entry_other_forms.to_string(),
                Style::Ghost,
                Action::EntryOtherForms,
            ),
        );
    }
    if let Some(w) = app.entry.keys.as_deref() {
        y += button_rows(ui, x, y, width, &langs);
        return keyed_words(app, w, ui, x, y, width, h);
    }
    y += guide_text(app, ui, x, y, width, EN.entry_type_the_words);

    let words: Vec<&str> = app.entry.typed.split_whitespace().collect();
    y += word_count_row(app, ui, x, y, width, true);
    // Use a sample stands in the empty grid, in Demo (`docs/MODES.md` §6.1).
    let chip = app.entry_sample_shown();
    let grid = (y, app.entry.word_boxes(words.len()));
    y = typed_cells(
        ui,
        (x, y, width),
        lang,
        &app.entry.typed,
        Cells {
            boxes: grid.1,
            masked: false,
            caret: !app.entry.on_passphrase && !chip,
            never: app.online,
        },
    );
    if chip {
        crate::samples::put::grid_chip(ui, (x, grid.0, width), grid.1);
    }
    y += button_rows(ui, x, y, width, &langs);

    // Status.
    let n = words.len();
    let complete = [12, 15, 18, 21, 24].contains(&n)
        && words
            .iter()
            .all(|wd| crate::forms::typed_index(lang, wd).is_some());
    let m = complete.then(|| crate::forms::typed_mnemonic(lang, &app.entry.typed));
    entry_status(app, ui, x, y, n, m)
}

/// Closes the clip `entry` opened around its content and reports how far
/// it scrolls: `bottom`, where the content (including the footer) ends,
/// measured from the top of the screen as if unscrolled, so the last
/// scroll leaves the footer and its margin inside the window.
fn entry_scrolled(app: &View, ui: &mut Ui, clip: faraday_draw::Rect, h: f32, bottom: f32) {
    ui.c.pop_clip();
    app.content_h.set(bottom + app.list_offset + 24.0);
    ui.report_scroll(clip, app.content_h.get() - h);
}

/// Add a key's word count: 12 · 15 · 18 · 21 · 24 as a row of equal
/// chips, the chosen one primary, with "Words" before them when
/// `labelled`. Returns its height and the gap under it.
pub(crate) fn word_count_row(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    labelled: bool,
) -> f32 {
    let chosen = app.entry.words_chosen();
    let mut bx = x;
    if labelled {
        ui.text_mid(x, y, 32.0, 13.0, W::S, MUTED, EN.word_count_row_words);
        bx += ui.measure(13.0, W::S, EN.word_count_row_words) + 14.0;
    }
    let counts = crate::EntryState::COUNTS;
    let gap = 6.0;
    let bw = ((x + w - bx - gap * (counts.len() - 1) as f32) / counts.len() as f32).min(64.0);
    for n in counts {
        let style = if usize::from(n) == chosen {
            Style::Primary
        } else {
            Style::Secondary
        };
        ui.button(
            bx,
            y,
            Some(bw),
            32.0,
            &n.to_string(),
            style,
            Action::EntryWordCount(n),
        );
        bx += bw + gap;
    }
    44.0
}

/// Words typed on a list's own on-screen keyboard, through the
/// word entry: the words taken so far in a grid, the keys typed for the
/// next one, the words they can be as pills to take one, the keyboard
/// with every key that leads nowhere disabled, and the status. Returns
/// where the next row starts.
fn keyed_words(
    app: &View,
    w: &crate::word_entry::WordEntry,
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    h: f32,
) -> f32 {
    use faraday_draw::widgets::keyboard;
    let lang = app.entry.language();
    let kind = w.keyboard();
    let mut y = y;
    y += guide_text(
        app,
        ui,
        x,
        y,
        width,
        match kind {
            keyboard::KeyboardKind::Pinyin => EN.keyed_words_type_each_word_s,
            keyboard::KeyboardKind::Zhuyin => EN.keyed_words_type_each_word,
            _ => EN.keyed_words_type_each,
        },
    );
    // The words taken, in a grid, the next one's keys in its cell.
    let taken: Vec<u16> = w.committed_indices().collect();
    y += word_count_row(app, ui, x, y, width, true);
    let prefix: String = w.prefix().iter().collect();
    y += keyed_cells(
        ui,
        (x, y, width),
        lang,
        &taken,
        &prefix,
        Cells {
            boxes: app.entry.word_boxes(taken.len() + 1),
            masked: false,
            caret: !app.entry.on_passphrase,
            never: app.online,
        },
    );
    // The count and, when the words are a whole key, its checksum and
    // fingerprint, or the last refusal: on the line above the
    // candidates, to keep the keyboard on screen.
    let (status, tone) = match &app.entry.error {
        Some(e) => (e.clone(), ERR),
        None => keyed_status(app, lang, &taken),
    };
    ui.text_right(x + width, y - 4.0, 24.0, 13.0, W::S, tone, &status);
    // What the keys typed can be: a pill each, pressed to take it.
    y += keyed_pills(ui, x, y, width, w, lang, true);
    // The keyboard, laid out in design units, its keys as tall as the
    // room above the status, the passphrase and the buttons allows.
    let rows = faraday_draw::widgets::keyboard::rows(kind) as f32;
    let room = h - 82.0 - 8.0 - 40.0 - 12.0 - y;
    let kh = (room / rows).clamp(22.0, 36.0);
    word_keys(app.entry.shift, w, ui, x, y, width, kh)
}

/// The words typed on a list's own keyboard, as a status line: the
/// count, and once it is a whole key, the checksum and fingerprint.
pub(crate) fn keyed_status(
    app: &View,
    lang: faraday_bip::bip39::Language,
    taken: &[u16],
) -> (String, faraday_draw::Color) {
    let n = taken.len();
    let count = || (fill(EN.keyed_status_words_2, &[&n.to_string()]), MUTED);
    if ![12, 15, 18, 21, 24].contains(&n) {
        return count();
    }
    // While typing goes on the key is not worked out: the count alone.
    let added = match faraday_keys::words::from_indices(lang, taken) {
        Err(e) => Err(crate::wallet::Refusal::Words(e.to_string()).text()),
        Ok(m) => match app.entry_probed(&m) {
            Some(r) => r,
            None => return count(),
        },
    };
    match added {
        Ok(fp) => (
            fill(
                EN.keyed_status_words,
                &[&n.to_string(), &fp_text(fp).to_string()],
            ),
            OK,
        ),
        Err(e) => (e, ERR),
    }
}

/// The count of words and, when they are a whole key, its checksum and
/// fingerprint; then the last refusal. Returns where the next row starts.
fn entry_status(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    n: usize,
    m: Option<Result<faraday_bip::bip39::Mnemonic, String>>,
) -> f32 {
    let mut y = y;
    // While typing goes on the key is not worked out: the count alone.
    let added = m.and_then(|made| match made {
        Err(e) => Some(Err(crate::wallet::Refusal::Words(e).text())),
        Ok(m) => app.entry_probed(&m),
    });
    let status = match added {
        Some(Ok(fp)) => (
            fill(
                EN.entry_status_words_checksum,
                &[&n.to_string(), &fp_text(fp).to_string()],
            ),
            OK,
        ),
        Some(Err(e)) => (e, ERR),
        None => (fill(EN.entry_status_words, &[&n.to_string()]), MUTED),
    };
    ui.text(x, y, 14.0, W::S, status.1, &status.0);
    y += 30.0;
    if let Some(e) = &app.entry.error {
        ui.text(x, y, 13.0, W::R, ERR, e);
    }
    y + 34.0
}

/// The BIP-39 passphrase and the buttons under the words.
/// The passphrase field, the way in from a stick, and the Add key ·
/// Clear · Make a new key row. Returns the bottom of the row, so the
/// page that scrolls to fit it (`entry`) knows how tall its content is.
fn entry_finish(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32, h: f32) -> f32 {
    // An optional BIP-39 passphrase: the same words with another
    // passphrase are another key.
    ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, EN.entry_finish_bip_39);
    let px = x + 150.0;
    let pw = 380.0f32.min(width - 150.0);
    let on = app.entry.on_passphrase;
    ui.fill(px, y, pw, 40.0, 8.0, BG);
    ui.stroke(px, y, pw, 40.0, 8.0, if on { ACCENT } else { BORDER });
    dots_field(
        ui,
        px,
        y,
        pw,
        &app.entry.passphrase,
        (on, app.demo_passphrase()),
        Action::EntryPassphrase,
    );
    ui.hit(px, y, pw, 40.0, Action::EntryPassphrase);
    // The stick row and the buttons sit at the bottom of the screen, but
    // move down below the passphrase field when the words above it (up
    // to 24 of them) would otherwise run under it.
    let by = (h - 36.0 - 46.0).max(y + 40.0 + 24.0 + 48.0);
    entry_stick(app, ui, x, by - 48.0);
    ui.button(
        x,
        by,
        Some(140.0),
        46.0,
        EN.entry_finish_add_key,
        if entry_typed(app) {
            Style::Primary
        } else {
            Style::Secondary
        },
        Action::EntryAdd,
    );
    let clear_w = ui.button(
        x + 152.0,
        by,
        None,
        46.0,
        EN.entry_finish_clear,
        Style::Ghost,
        Action::EntryClear,
    );
    let new_x = x + 152.0 + clear_w + 8.0;
    let new_w = ui.button(
        new_x,
        by,
        None,
        46.0,
        EN.entry_finish_make_a,
        Style::Secondary,
        Action::KeyGen(None),
    );
    ui.text_mid(
        new_x + new_w + 20.0,
        by,
        46.0,
        12.0,
        W::R,
        DIM,
        EN.entry_finish_type_on,
    );
    by + 46.0
}

/// Whether any of a key's words is typed on Add a key.
pub(crate) fn entry_typed(app: &View) -> bool {
    match app.entry.keys.as_deref() {
        Some(w) => w.committed_indices().next().is_some() || !w.prefix().is_empty(),
        None => !app.entry.typed.trim().is_empty(),
    }
}

/// Add a key's Scan a SeedQR: for a wallet's missing key, that wallet's
/// key scanner; else the SeedQR scanner.
pub(crate) fn entry_scan(app: &View) -> Action {
    match app.entry.wallet {
        Some(w) if w < app.session.wallets.len() => Action::ScanKey(w),
        _ => Action::ScanSeed,
    }
}

/// Add a key's way in from a stick: a key file on it is copied in on the
/// stick visit and loads when the stick is pulled.
pub(crate) fn entry_stick(app: &View, ui: &mut Ui, x: f32, y: f32) {
    if app.sticks.is_empty() {
        ui.icon(x - 2.0, y + 8.0, 20.0, app.medium.icon(), 12.0, DIM);
        ui.text_mid(
            x + 24.0,
            y,
            36.0,
            13.0,
            W::R,
            DIM,
            &fill(EN.entry_stick_from_plug, &[app.medium.a()]),
        );
    } else {
        ui.button(
            x,
            y,
            None,
            36.0,
            EN.entry_stick_copy_files,
            Style::Secondary,
            Action::Nav(Screen::Visit),
        );
    }
}

/// Add a key in another form than BIP-39 words: one share, string or part
/// typed at a time, the ones collected listed, then put together. Returns
/// the bottom of its footer row, so the page that scrolls to fit it
/// (`entry`) knows how tall its content is.
///
/// New key's Words card for a key typed in another form draws it too,
/// with `flow` its Continue (`docs/history/NEW-WALLET.md` §15.2): no passphrase
/// (Key asks it), no way in from a stick, and Continue in place of Make
/// the key, under what is above it.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn entry_form(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    h: f32,
    flow: Option<Action>,
) -> f32 {
    use crate::forms::Form;
    let form = app.entry.form;
    let mut y = y;
    let guide = match form {
        Form::Slip39 => EN.entry_form_type_one_share,
        Form::Codex32 => EN.entry_form_type_one_codex32,
        _ => EN.entry_form_type_one,
    };
    y += guide_text(app, ui, x, y, width, guide);
    // What is being typed.
    ui.fill(x, y, width, 44.0, 8.0, BG);
    ui.stroke(
        x,
        y,
        width,
        44.0,
        8.0,
        if app.entry.on_passphrase {
            BORDER
        } else {
            ACCENT
        },
    );
    let on = !app.entry.on_passphrase;
    let empty = app.entry.typed.is_empty();
    // Use a sample stands where the hint would, in Demo.
    let chip = app.entry_sample_shown();
    if chip {
        let h = faraday_draw::tokens::ENTRY_FIELD_HEIGHT;
        crate::samples::put::field_chip(ui, (x, y, width, h));
    }
    let typed = if chip {
        crate::secret_text::SecretText::new()
    } else if empty && on {
        // The caret stands before the hint while nothing is typed.
        ui.caret(x + 12.0, y + 13.0, 18.0);
        match form {
            Form::Codex32 => crate::secret_text::SecretText::of("ms1…"),
            _ => crate::secret_text::SecretText::of(EN.entry_form_words_of),
        }
    } else {
        let mut t = crate::secret_text::SecretText::of(&app.entry.typed);
        if on {
            t.push_str(ui.caret_char());
        }
        t
    };
    let shown = ui.fit_secret(14.0, W::M, &typed, width - 24.0);
    ui.text_mid(
        x + if empty && on { 18.0 } else { 12.0 },
        y,
        44.0,
        14.0,
        W::M,
        if app.entry.typed.is_empty() {
            DIM
        } else {
            TEXT
        },
        &shown,
    );
    y += 54.0 + never_below(app, ui, x, y + 44.0, width);
    if let Some(e) = &app.entry.error {
        y += ui.wrap(x, y, width, 13.0, W::R, ERR, e) + 8.0;
    }
    // What is in.
    let lines = app.entry.parts.lines(form);
    section_label(
        ui,
        x,
        y,
        &match lines.len() {
            0 => EN.entry_form_nothing.to_string(),
            1 => EN.entry_form_1_in.to_string(),
            n => fill(EN.entry_form_in, &[&n.to_string()]),
        },
    );
    y += 26.0;
    for l in &lines {
        ui.icon(x, y - 2.0, 18.0, Icon::Done, 10.0, OK);
        ui.text(x + 24.0, y, 13.0, W::R, TEXT, l);
        y += 24.0;
    }
    y += 8.0;
    // A SLIP-39 backup's passphrase; a Seed XOR key's BIP-39 one.
    if form != Form::Codex32 && flow.is_none() {
        ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, EN.entry_form_passphrase);
        let px = x + 150.0;
        let pw = 380.0f32.min(width - 150.0);
        let on = app.entry.on_passphrase;
        ui.fill(px, y, pw, 40.0, 8.0, BG);
        ui.stroke(px, y, pw, 40.0, 8.0, if on { ACCENT } else { BORDER });
        dots_field(
            ui,
            px,
            y,
            pw,
            &app.entry.passphrase,
            (on, app.demo_passphrase()),
            Action::EntryPassphrase,
        );
        ui.hit(px, y, pw, 40.0, Action::EntryPassphrase);
    }
    // As `entry_finish`: the stick row and the buttons sit at the bottom
    // of the screen, but move down below what is above them (the
    // passphrase field, or the list of collected shares) when that would
    // otherwise run under it.
    let by = match flow {
        Some(_) => y,
        None => (h - 36.0 - 46.0).max(y + 40.0 + 24.0 + 48.0),
    };
    if flow.is_none() {
        entry_stick(app, ui, x, by - 48.0);
    }
    let ready = app.entry.parts.ready(form);
    let aw = ui.button(
        x,
        by,
        None,
        46.0,
        EN.entry_form_add_this,
        if app.entry.typed.trim().is_empty() {
            Style::Disabled
        } else {
            Style::Secondary
        },
        Action::EntryPart,
    );
    let sw = ui.button(
        x + aw + 8.0,
        by,
        None,
        46.0,
        EN.entry_form_scan_a_qr,
        Style::Secondary,
        Action::ScanPart,
    );
    let aw = aw + sw + 8.0;
    let (label, action) = match flow {
        Some(next) => (EN.keygen_continue, next),
        None => (EN.entry_form_make_the, Action::EntryRecover),
    };
    let mw = ui.button(
        x + aw + 8.0,
        by,
        None,
        46.0,
        label,
        if ready {
            Style::Primary
        } else {
            Style::Disabled
        },
        action,
    );
    // Its Enter on an empty line makes the key; the flow's opens Key.
    if flow.is_none() {
        ui.text_mid(
            x + aw + mw + 28.0,
            by,
            46.0,
            12.0,
            W::R,
            DIM,
            EN.entry_form_enter_adds,
        );
    }
    by + 46.0
}
