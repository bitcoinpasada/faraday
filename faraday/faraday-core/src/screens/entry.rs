//! `Screen::Entry`: typing a seed.

use super::parts::{
    Cells, dots_field, guide_text, keyed_cells, keyed_pills, never_below, typed_cells, word_keys,
};
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Region;
use crate::ui::pal::*;
use crate::ui::{CoreUi, Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, BUTTON_H, GAP_SMALL, LABEL_L, LABEL_M, TITLE_S,
};
use faraday_draw::widgets::Icon;
use faraday_text::Text;
use faraday_ui::parts::{button_rows, section_label, wrap_buttons};

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
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let back = Action::Nav(app.entry_leave());
    let back_name = if app.entry_leave() == Screen::SpendBackup {
        EN.entry_spend
    } else {
        EN.entry_wallets
    };
    let for_wallet = app
        .entry
        .state()
        .wallet
        .and_then(|w| app.session.wallets.get(w))
        .map(|wl| wl.name.as_str());
    // The round's codex32 string, typed in place (`docs/CODEX32.md` §9.5).
    let round = app.entry.state().c32_round.as_ref();
    let round = round.map(|w| Text::fill(EN.c32_round_heading, &[&w.share(), &w.id_text()]));
    let heading = match (app.entry.state().wanted, for_wallet) {
        _ if round.is_some() => round.unwrap_or_default(),
        (Some(fp), _) => Text::fill(
            EN.entry_add_key,
            &[&fp_text(faraday_bip::keys::Fingerprint(fp))],
        ),
        (None, Some(name)) => Text::fill(EN.entry_add_key_title, &[name]),
        (None, None) if app.entry.state().form != crate::forms::Form::Words => {
            Text::fill(EN.entry_add_a_key_n, &[app.entry.state().form.name()])
        }
        (None, None) => Text::of(EN.entry_add_a_key),
    };
    // The top app bar names the page and has its way back.
    let back = Some(app.named_back((back_name, back)));
    let top = super::bar_head(ui, back, &heading);
    let mut y = top;
    // The words form is the one everyday way in; the other three are
    // behind Other forms, or already shown when a Tools tile opened one
    // of them (`docs/history/SIMPLIFY.md` §2.4).
    if app.entry.state().other_forms {
        let forms: Vec<(&str, Style, Action)> = crate::forms::Form::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.name(),
                    Style::chosen(*f == app.entry.state().form),
                    Action::EntryForm(i as u8),
                )
            })
            .collect();
        y += button_rows(ui, x, y, width, &forms) + 4.0;
    }
    if app.entry.state().form != crate::forms::Form::Words {
        let bottom = entry_form(app, ui, x, y, width, h, None);
        entry_scrolled(app, ui, open, h, bottom);
        return;
    }
    // Scan leads, above the words: primary until a word is typed, when
    // Add key is (`docs/history/SCAN-TO-SIGN.md` §2.5).
    ui.button(
        x,
        y,
        Some(width),
        BUTTON_H,
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
    entry_scrolled(app, ui, open, h, bottom);
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
    let lang = app.entry.state().language();
    let english = lang == faraday_bip::bip39::Language::English;
    let mut langs: Vec<(&str, Style, Action)> = if english && !app.entry.state().languages {
        vec![(
            EN.entry_other_languages,
            Style::Ghost,
            Action::EntryLanguages,
        )]
    } else {
        crate::forms::LANGUAGES
            .iter()
            .enumerate()
            .map(|(i, l)| {
                (
                    crate::forms::language_name(*l),
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
            (EN.entry_other_forms, Style::Ghost, Action::EntryOtherForms),
        );
    }
    if let Some(w) = app.entry.state().keys.as_deref() {
        if app.entry_paste_offered() {
            ui.demo_offer(EN.paste_words, Action::PasteWords);
        }
        y += button_rows(ui, x, y, width, &langs);
        return keyed_words(app, w, ui, x, y, width, h);
    }
    y += guide_text(app, ui, x, y, width, EN.entry_type_the_words);

    let words: Vec<&str> = app.entry.state().typed.split_whitespace().collect();
    y += word_count_row(app, ui, x, y, width, true);
    // Use a sample goes in the host's strip while the grid is empty, and
    // Paste words while it is on screen (`docs/DESKTOP-HOST.md` §3.1).
    if app.entry_sample_shown() {
        crate::samples::put::offer(ui, Action::UseSample);
    }
    if app.entry_paste_offered() {
        ui.demo_offer(EN.paste_words, Action::PasteWords);
    }
    let grid = (y, app.entry.state().word_boxes(words.len()));
    y = typed_cells(
        ui,
        (x, y, width),
        lang,
        &app.entry.state().typed,
        Cells {
            boxes: grid.1,
            masked: false,
            caret: !app.entry.state().on_passphrase,
            never: app.seed_line,
            press: Some(Action::TypeHere(crate::edit::Here::Words)),
        },
    );
    y += button_rows(ui, x, y, width, &langs);

    // Status.
    let n = words.len();
    let complete = [12, 15, 18, 21, 24].contains(&n)
        && words
            .iter()
            .all(|wd| crate::forms::typed_index(lang, wd).is_some());
    let m = complete.then(|| {
        crate::forms::typed_mnemonic(lang, &app.entry.state().typed).map_err(|e| Text::of(&e))
    });
    entry_status(app, ui, x, y, n, m)
}

/// Closes the clip `entry` opened around its content and reports how far
/// it scrolls: `bottom`, where the content (including the footer) ends,
/// measured from the top of the screen as if unscrolled, so the last
/// scroll leaves the footer and its margin inside the window.
fn entry_scrolled(app: &View, ui: &mut Ui, open: faraday_ui::Open, h: f32, bottom: f32) {
    app.scroll.content_h().set(bottom + 24.0);
    ui.region_close(open, app.scroll.content_h().get() - h);
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
    let chosen = app.entry.state().words_chosen();
    let mut bx = x;
    if labelled {
        ui.text_mid_as(
            (x, y, 32.0),
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.word_count_row_words,
        );
        bx += ui.measure_as(LABEL_L, EN.word_count_row_words) + 14.0;
    }
    let counts = crate::EntryState::COUNTS;
    let (gap, max) = (6.0, 64.0);
    let row = BUTTON_H + GAP_SMALL;
    let names: Vec<faraday_text::Num> = counts.iter().map(|n| Text::num(u64::from(*n))).collect();
    let labels: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
    let room = x + w - bx;
    let (cols, bw) = if ui.compact {
        faraday_ui::parts::chip_grid(ui, room, gap, max, &labels)
    } else {
        (
            counts.len(),
            ((room - gap * (counts.len() - 1) as f32) / counts.len() as f32).min(max),
        )
    };
    for (i, n) in counts.into_iter().enumerate() {
        let style = Style::chosen(usize::from(n) == chosen);
        let (col, r) = ((i % cols) as f32, (i / cols) as f32);
        ui.button(
            bx + col * (bw + gap),
            y + r * row,
            Some(bw),
            BUTTON_H,
            labels[i],
            style,
            Action::EntryWordCount(n),
        );
    }
    row * counts.len().div_ceil(cols) as f32
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
    let lang = app.entry.state().language();
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
    let open = w.held_indices().count() + usize::from(!w.held_prefix().is_empty());
    y += keyed_cells(
        ui,
        (x, y, width),
        lang,
        w,
        Cells {
            boxes: app.entry.state().word_boxes(taken.len() + 1 + open),
            masked: false,
            caret: !app.entry.state().on_passphrase,
            never: app.seed_line,
            press: Some(Action::TypeHere(crate::edit::Here::Keyed)),
        },
    );
    // The count and, when the words are a whole key, its checksum and
    // fingerprint, or the last refusal: on the line above the
    // candidates, to keep the keyboard on screen.
    let (status, tone) = match &app.entry.state().error {
        Some(e) => (Text::of(e), Role::Error.paint()),
        None => keyed_status(app, lang, &taken),
    };
    ui.text_right_as((x + width, y - 4.0, 24.0), LABEL_L, tone, &status);
    // What the keys typed can be: a pill each, pressed to take it.
    y += keyed_pills(ui, x, y, width, w, lang, true);
    // The keyboard, laid out in design units, its keys as tall as the
    // room above the status, the passphrase and the buttons allows.
    let rows = faraday_draw::widgets::keyboard::rows(kind) as f32;
    let room = h - 82.0 - 8.0 - 40.0 - 12.0 - y;
    let kh = (room / rows).clamp(22.0, 36.0);
    word_keys(app.entry.state().shift, w, ui, x, y, width, kh)
}

/// The words typed on a list's own keyboard, as a status line: the
/// count, and once it is a whole key, the checksum and fingerprint.
pub(crate) fn keyed_status(
    app: &View,
    lang: faraday_bip::bip39::Language,
    taken: &[u16],
) -> (Text, crate::ui::Paint) {
    let n = taken.len();
    let count = || {
        (
            Text::fill(EN.keyed_status_word_count, &[&Text::num(n as u64)]),
            Role::OnSurfaceVariant.paint(),
        )
    };
    if ![12, 15, 18, 21, 24].contains(&n) {
        return count();
    }
    // While typing goes on the key is not worked out: the count alone.
    let added = match faraday_keys::words::from_indices(lang, taken) {
        Err(e) => Err(crate::wallet::Refusal::Words(faraday_strings::bip::bip39_error(&e)).text()),
        Ok(m) => match app.entry_probed(&m) {
            Some(r) => r.map_err(|e| Text::of(&e)),
            None => return count(),
        },
    };
    match added {
        Ok(fp) => (
            Text::fill(EN.keyed_status_words, &[&Text::num(n as u64), &fp_text(fp)]),
            Role::Ok.paint(),
        ),
        Err(e) => (e, Role::Error.paint()),
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
    m: Option<Result<faraday_bip::bip39::Mnemonic, Text>>,
) -> f32 {
    let mut y = y;
    // While typing goes on the key is not worked out: the count alone.
    let added = m.and_then(|made| match made {
        Err(e) => Some(Err(crate::wallet::Refusal::Words(e).text())),
        Ok(m) => app.entry_probed(&m).map(|r| r.map_err(|e| Text::of(&e))),
    });
    let status = match added {
        Some(Ok(fp)) => (
            Text::fill(
                EN.entry_status_words_checksum,
                &[&Text::num(n as u64), &fp_text(fp)],
            ),
            Role::Ok,
        ),
        Some(Err(e)) => (e, Role::Error),
        None => (
            Text::fill(EN.entry_status_words, &[&Text::num(n as u64)]),
            Role::OnSurfaceVariant,
        ),
    };
    ui.text_as(x, y, TITLE_S, status.1, &status.0);
    y += ui.advance_as(TITLE_S, 30.0);
    if let Some(e) = &app.entry.state().error {
        ui.text_as(x, y, BODY_M, Role::Error, e);
    }
    y + 34.0
}

/// Add a key's passphrase on a wide page: the field with its label at its
/// left, and a second one under it, with the line while they differ, where
/// the passphrase is asked twice (`EntryState::pair`). Returns the height
/// used.
fn entry_pass_fields(app: &View, ui: &mut Ui, (x, y, width): (f32, f32, f32), label: &str) -> f32 {
    let e = app.entry.state();
    let px = x + 150.0;
    let pw = 380.0f32.min(width - 150.0);
    let field = |ui: &mut Ui, n: u8, fy: f32| {
        let second = n == 1;
        let on = e.on_passphrase && e.on_again == second;
        ui.text_mid_as(
            (x, fy, 40.0),
            BODY_S,
            Role::OnSurfaceVariant,
            if second {
                EN.keygen_passphrase_again
            } else {
                label
            },
        );
        ui.fill(px, fy, pw, 40.0, 8.0, Role::SurfaceContainerHighest);
        ui.stroke(
            px,
            fy,
            pw,
            40.0,
            8.0,
            if on {
                Role::Primary
            } else {
                Role::OutlineVariant
            },
        );
        dots_field(
            ui,
            px,
            fy,
            pw,
            if second { &e.again } else { &e.passphrase },
            (on, entry_chip(app, second)),
            if second {
                Action::EntryPassphraseAgain
            } else {
                Action::EntryPassphrase
            },
        );
        48.0
    };
    if e.pair() {
        super::pass_pair::pass_pair(
            ui,
            (px, y, pw),
            (e.passphrase.as_str(), e.again.as_str()),
            field,
        )
    } else {
        field(ui, 0, y);
        40.0
    }
}

/// Add a key's Use faraday for its first field or, `second`, the one
/// again. It fills both, so an empty field offers it only while the other
/// is empty too, never over what the person typed. A lookup.
pub(crate) fn entry_chip(app: &View, second: bool) -> Option<Action> {
    let e = app.entry.state();
    let (this, other) = if second {
        (&e.again, &e.passphrase)
    } else {
        (&e.passphrase, &e.again)
    };
    let other_typed = !other.is_empty() && (second || e.pair());
    app.pass_chip(if second {
        crate::demo::PassField::EntryAgain
    } else {
        crate::demo::PassField::Entry
    })
    .filter(|_| !(this.is_empty() && other_typed))
}

/// The BIP-39 passphrase and the buttons under the words.
/// The passphrase field, the way in from a stick, and the Add key ·
/// Clear · Make a new key row. Returns the bottom of the row, so the
/// page that scrolls to fit it (`entry`) knows how tall its content is.
fn entry_finish(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32, h: f32) -> f32 {
    // An optional BIP-39 passphrase: the same words with another
    // passphrase are another key.
    let ph = entry_pass_fields(app, ui, (x, y, width), EN.entry_finish_bip39_passphrase);
    // The stick row and the buttons sit at the bottom of the screen, but
    // move down below the passphrase field when the words above it (up
    // to 24 of them) would otherwise run under it.
    let by = (h - 36.0 - 46.0).max(y + ph + 24.0 + 48.0);
    entry_stick(app, ui, x, by - 48.0);
    ui.button(
        x,
        by,
        Some(140.0),
        BUTTON_H,
        EN.entry_finish_add_key,
        if app.entry.state().differs() {
            Style::Disabled
        } else if entry_typed(app) {
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
        BUTTON_H,
        EN.entry_finish_clear,
        Style::Ghost,
        Action::EntryClear,
    );
    let new_x = x + 152.0 + clear_w + 8.0;
    let new_w = ui.button(
        new_x,
        by,
        None,
        BUTTON_H,
        EN.entry_finish_new_key,
        Style::Secondary,
        Action::KeyGen(None),
    );
    keys_hint(
        ui,
        (x, width),
        (new_x + new_w + 20.0, by),
        EN.entry_finish_keys_hint,
    )
}

/// The keys' hint of a foot row of 46 high at `by`: beside its buttons,
/// from `hx`, where it fits in the page (`x`, `width`); on a page narrowed
/// (beside the "Demo only" card, a medium window) on its own line under
/// them, wrapped, never cut. Returns the row's foot.
fn keys_hint(ui: &mut Ui, (x, width): (f32, f32), (hx, by): (f32, f32), hint: &str) -> f32 {
    use faraday_draw::tokens::GAP;
    let (muted, row) = (Role::OnSurfaceVariant, 46.0);
    if hx + ui.measure_as(BODY_S, hint) <= x + width {
        ui.text_mid_as((hx, by, row), BODY_S, muted, hint);
        by + row
    } else {
        let hy = by + row + GAP;
        hy + ui.wrap_as((x, hy, width), BODY_S, muted, hint)
    }
}

/// Whether any of a key's words is typed on Add a key.
pub(crate) fn entry_typed(app: &View) -> bool {
    match app.entry.state().keys.as_deref() {
        Some(w) => w.committed_indices().next().is_some() || !w.prefix().is_empty(),
        None => !app.entry.state().typed.trim().is_empty(),
    }
}

/// Add a key's Scan a SeedQR: for a wallet's missing key, that wallet's
/// key scanner; else the SeedQR scanner.
pub(crate) fn entry_scan(app: &View) -> Action {
    match app.entry.state().wallet {
        Some(w) if w < app.session.wallets.len() => Action::ScanKey(w),
        _ => Action::ScanSeed,
    }
}

/// Add a key's way in from a stick: a key file on it is copied in on the
/// stick visit and loads when the stick is pulled.
pub(crate) fn entry_stick(app: &View, ui: &mut Ui, x: f32, y: f32) {
    if app.stick.is_empty() {
        ui.icon(
            x - 2.0,
            y + 8.0,
            20.0,
            app.medium.icon(),
            12.0,
            Role::OnSurfaceVariant,
        );
        ui.text_mid_as(
            (x + 24.0, y, 36.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &Text::fill(EN.entry_stick_from_plug, &[app.medium.a()]),
        );
    } else {
        ui.button(
            x,
            y,
            None,
            BUTTON_H,
            EN.entry_stick_copy_files,
            Style::Secondary,
            Action::Nav(Screen::Files),
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
    let form = app.entry.state().form;
    let mut y = y;
    let checking = form == Form::Codex32 && app.entry.state().codex32.checking;
    let guide = match form {
        Form::Slip39 => EN.entry_form_type_one_share,
        Form::Codex32 if checking => EN.codex32_form_check_guide,
        Form::Codex32 => EN.entry_form_type_one_codex32,
        _ => EN.entry_form_type_one,
    };
    y += guide_text(app, ui, x, y, width, guide);
    let mut replaced = false;
    if form == Form::Codex32 {
        let (hh, r) = codex32_head(app, ui, (x, y, width), true);
        y += hh;
        replaced = r;
    }
    if !replaced {
        entry_field(app, ui, x, y, width, form);
        y += faraday_draw::tokens::TOUCH
            + 10.0
            + never_below(app, ui, x, y + faraday_draw::tokens::TOUCH, width);
    }
    if let Some(e) = &app.entry.state().error {
        y += ui.wrap_as((x, y, width), BODY_M, Role::Error, e) + 8.0;
    }
    y += codex32_found(app, ui, x, y, width);
    // What is in.
    let lines = app.entry.state().parts.lines(form);
    section_label(
        ui,
        x,
        y,
        &match lines.len() {
            0 => Text::of(EN.entry_form_nothing),
            1 => Text::of(EN.entry_form_count_one),
            n => Text::fill(EN.entry_form_count_many, &[&Text::num(n as u64)]),
        },
    );
    y += 26.0;
    for (n, l) in lines.iter().enumerate() {
        ui.icon(x, y - 2.0, 18.0, Icon::Done, 10.0, Role::Ok);
        let indent = faraday_draw::tokens::PART_INDENT;
        ui.text_as(x + indent, y, BODY_M, Role::OnSurface, l);
        y += ui.line_as(BODY_M) + faraday_draw::tokens::GAP_SMALL;
        y += codex32_mark(app, ui, x + indent, y, width - indent, n);
    }
    y += 8.0;
    if form == Form::Codex32 {
        y += codex32_check_button(app, ui, x, y, width);
    }
    // A SLIP-39 backup's passphrase; a Seed XOR key's BIP-39 one.
    let mut ph = 40.0;
    if form != Form::Codex32 && flow.is_none() {
        ph = entry_pass_fields(app, ui, (x, y, width), EN.entry_form_passphrase);
    }
    // As `entry_finish`: the stick row and the buttons sit at the bottom
    // of the screen, but move down below what is above them (the
    // passphrase field, or the list of collected shares) when that would
    // otherwise run under it.
    let by = match flow {
        Some(_) => y,
        None => (h - 36.0 - 46.0).max(y + ph + 24.0 + 48.0),
    };
    if flow.is_none() {
        entry_stick(app, ui, x, by - 48.0);
    }
    let ready = app.entry.state().parts.ready(form);
    let aw = ui.button(
        x,
        by,
        None,
        BUTTON_H,
        if checking {
            EN.codex32_form_check_it
        } else {
            EN.entry_form_add_share
        },
        if app.entry.state().typed.trim().is_empty() || replaced {
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
        BUTTON_H,
        EN.entry_form_scan_a_qr,
        Style::Secondary,
        Action::ScanPart,
    );
    let aw = aw + sw + 8.0;
    let (label, action) = match flow {
        Some(next) => (EN.keygen_continue, next),
        None => (EN.entry_form_make_key, Action::EntryRecover),
    };
    let mw = ui.button(
        x + aw + 8.0,
        by,
        None,
        BUTTON_H,
        label,
        Style::Primary.when(ready && !app.entry.state().differs()),
        action,
    );
    // Its Enter on an empty line makes the key; the flow's opens Key.
    if flow.is_none() {
        let hx = x + aw + mw + 28.0;
        return keys_hint(ui, (x, width), (hx, by), EN.entry_form_enter_adds);
    }
    by + 46.0
}

/// The field a share, string or part is typed into, 44 tall.
fn entry_field(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32, form: crate::forms::Form) {
    use crate::forms::Form;
    // What is being typed, in a field a touch target tall: a press
    // places the caret, or moves typing here from the passphrase.
    let fh = faraday_draw::tokens::TOUCH;
    ui.fill(x, y, width, fh, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(
        x,
        y,
        width,
        fh,
        8.0,
        if app.entry.state().on_passphrase {
            Role::OutlineVariant
        } else {
            Role::Primary
        },
    );
    let on = !app.entry.state().on_passphrase;
    let empty = app.entry.state().typed.is_empty();
    // Use a sample goes in the "Demo only" card while the field is empty.
    if app.entry_sample_shown() {
        crate::samples::put::offer(ui, Action::UseSample);
    }
    let here = Action::TypeHere(crate::edit::Here::Entry);
    ui.press_box(x, y, width, fh, here);
    let typed = if empty && on {
        // The caret stands before the hint while nothing is typed.
        let c = faraday_draw::tokens::CARET_MIN;
        ui.caret(x + 12.0, y + (fh - c) / 2.0, c);
        match form {
            Form::Codex32 => crate::secret_text::SecretText::of("ms1…"),
            _ => crate::secret_text::SecretText::of(EN.entry_form_one_share_words),
        }
    } else {
        // What is typed, read where it is held (`docs/TEXT-CURSOR.md`).
        ui.typed(
            x + faraday_draw::tokens::TYPED_INSET,
            y,
            fh,
            BODY_M.size,
            W::M,
            Role::OnSurface,
            &app.entry.state().typed,
            width - 2.0 * faraday_draw::tokens::TYPED_INSET,
            on,
            here,
        );
        return;
    };
    let shown = ui.fit_secret(14.0, W::M, &typed, width - 24.0);
    ui.text_mid_as(
        (x + if empty && on { 18.0 } else { 12.0 }, y, fh),
        BODY_M.mono(),
        Role::OnSurfaceVariant,
        &shown,
    );
}

/// The codex32 form's head (`docs/CODEX32.md` §8.4, §8.5): the tick's
/// row where `tick`, then, in place of the field, the correction's panel
/// while it is up or a string at an index already in beside the one
/// held. Returns its height and whether it stands in place of the field.
pub(crate) fn codex32_head(
    app: &View,
    ui: &mut Ui,
    (x, y, width): (f32, f32, f32),
    tick: bool,
) -> (f32, bool) {
    use crate::codex32_correct::screen::panel;
    use faraday_draw::tokens::CORRECT_GAP;
    let e = app.entry.state();
    let mut h = if tick {
        codex32_tick(app, ui, x, y, width)
    } else {
        0.0
    };
    if e.correction.is_open() {
        h += panel(ui, x, y + h, width, &e.correction, Action::Codex32Correct) + CORRECT_GAP;
        return (h, true);
    }
    if let Some(clash) = &e.codex32.clash {
        h += codex32_clash(ui, x, y + h, width, clash) + CORRECT_GAP;
        return (h, true);
    }
    (h, false)
}

/// The codex32 form's tick alone, **Suggest corrections**, and the gap
/// under it: where the small panel draws it, under the field. Returns its
/// height.
pub(crate) fn codex32_tick(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32) -> f32 {
    use crate::codex32_correct::screen::tick_row;
    let c = &app.entry.state().correction;
    tick_row(ui, x, y, width, c, Action::Codex32Correct) + faraday_draw::tokens::CORRECT_GAP
}

/// A string at an index already in, beside the one held, their differing
/// boxes marked, and **Keep the one held** · **Replace it** (§8.5).
fn codex32_clash(ui: &mut Ui, x: f32, y: f32, width: f32, clash: &crate::forms::Clash) -> f32 {
    use crate::codex32_correct::screen::{line, string_boxes};
    use crate::forms::C32Press;
    use faraday_draw::tokens::CORRECT_GAP;
    let index = clash
        .text
        .as_str()
        .as_bytes()
        .get(8)
        .copied()
        .unwrap_or(b'?');
    let mut upper = [0u8; 4];
    let title = Text::fill(
        EN.codex32_form_clash,
        &[char::from(index.to_ascii_uppercase()).encode_utf8(&mut upper)],
    );
    let mut h = line(
        ui,
        x,
        y,
        width,
        Icon::Warning,
        Role::Warning.paint(),
        &title,
    ) + CORRECT_GAP;
    for (label, text) in [
        (EN.codex32_form_held, clash.held.as_str()),
        (EN.codex32_form_new, clash.text.as_str()),
    ] {
        h += ui.wrap_as((x, y + h, width), TITLE_S, Role::OnSurfaceVariant, label);
        h += string_boxes(ui, x, y + h, width, text, &clash.differ) + CORRECT_GAP;
    }
    let mut boxes = Text::new();
    for (i, d) in clash.differ.iter().enumerate() {
        if i > 0 {
            boxes.push_str(", ");
        }
        boxes.push_str(&Text::num(*d as u64));
    }
    h += ui.wrap_as(
        (x, y + h, width),
        BODY_M,
        Role::OnSurfaceVariant,
        &Text::fill(EN.codex32_form_differ, &[&boxes]),
    ) + CORRECT_GAP;
    h + faraday_ui::sheet::buttons(
        ui,
        x,
        y + h,
        width,
        &[
            (
                EN.codex32_form_keep,
                Style::Primary,
                Action::Codex32Form(C32Press::Keep),
            ),
            (
                EN.codex32_form_replace,
                Style::Secondary,
                Action::Codex32Form(C32Press::Replace),
            ),
        ],
    )
}

/// Under the codex32 form's field: "Already in", or what Check it found
/// (§8.3): the checksum, what the string is, whether it is of the set
/// typed in, and its mark. Returns its height.
pub(crate) fn codex32_found(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32) -> f32 {
    use crate::codex32_correct::screen::line;
    use faraday_draw::tokens::CORRECT_GAP;
    let e = app.entry.state();
    if e.codex32.already {
        return line(
            ui,
            x,
            y,
            width,
            Icon::Info,
            Role::OnSurfaceVariant.paint(),
            EN.codex32_form_already_in,
        ) + CORRECT_GAP;
    }
    let Some(checked) = &e.codex32.checked else {
        return 0.0;
    };
    let (icon, tone, text) = if checked.holds {
        (Icon::Done, Role::Ok, EN.codex32_form_holds)
    } else {
        (Icon::Error, Role::Error, EN.codex32_form_fails)
    };
    let mut h = line(ui, x, y, width, icon, tone.paint(), text);
    h += ui.wrap_as((x, y + h, width), BODY_M, Role::OnSurface, &checked.what);
    if let Some(of) = checked.of_the_set {
        let (text, tone) = if of {
            (EN.codex32_form_of_the_set, Role::Ok)
        } else {
            (EN.codex32_form_not_of_the_set, Role::Error)
        };
        h += ui.wrap_as((x, y + h, width), BODY_M, tone, text);
    }
    // The codex32 strings of its set the open vaults keep (§9.5).
    if checked.holds && e.codex32.check_in_vaults > 0 {
        let n = Text::num(e.codex32.check_in_vaults as u64);
        h += ui.wrap_as(
            (x, y + h, width),
            BODY_M,
            Role::OnSurface,
            &Text::fill(EN.c32r_in_vaults, &[&n]),
        );
    }
    if let Some(text) = checked.mark.and_then(|m| m.string_line()) {
        h += line(
            ui,
            x,
            y + h,
            width,
            Icon::Warning,
            Role::Warning.paint(),
            text,
        );
    }
    h += CORRECT_GAP;
    // The final row of its worksheet, or the whole sheet (§8.3).
    if let Some(sheet) = &e.codex32.sheet {
        let open = e.codex32.sheet_open;
        let pan = (open, e.codex32.pan);
        h += crate::codex32::worksheet_screen::check_sheet(app, sheet, pan, ui, (x, y + h, width));
        let label = if open {
            EN.c32r_final_row_only
        } else {
            EN.c32r_show_worksheet
        };
        h += wrap_buttons(
            ui,
            x,
            y + h,
            width,
            faraday_draw::tokens::TOUCH,
            &[(
                label,
                Style::Ghost,
                Action::Codex32Form(crate::forms::C32Press::Sheet),
            )],
        );
    }
    h
}

/// The line under a codex32 string in the list of strings in, where it
/// went in by a correction that left it unchecked: "Repaired, not
/// checked". Returns its height.
pub(crate) fn codex32_mark(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32, n: usize) -> f32 {
    let e = app.entry.state();
    if e.form != crate::forms::Form::Codex32 {
        return 0.0;
    }
    let Some(text) = e
        .parts
        .codex32
        .get(n)
        .and_then(|s| e.correction.mark_of(s))
        .and_then(|m| m.string_line())
    else {
        return 0.0;
    };
    crate::codex32_correct::screen::line(
        ui,
        x,
        y,
        width,
        Icon::Warning,
        Role::Warning.paint(),
        text,
    )
}

/// The codex32 form's ways on from the strings in: **Make the rest** and
/// **Add a share** once a string of a split set is in (§8.1, §8.2), and
/// Check a string, or back to adding strings (§8.3), each a touch target
/// tall. Returns their height.
pub(crate) fn codex32_check_button(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32) -> f32 {
    use crate::forms::C32Press;
    use faraday_draw::tokens::{CORRECT_GAP, TOUCH};
    let e = app.entry.state();
    let checking = e.codex32.checking;
    let making = e.codex32.making;
    let label = if checking {
        EN.codex32_form_back_to_adding
    } else {
        EN.codex32_form_check_a_string
    };
    let mut items = Vec::new();
    if !checking && crate::forms::offers_more(&e.parts) {
        use crate::codex32::Given;
        // The one pressed is the chosen one of the two: selected, with
        // its tick (F108), its row making room for it.
        let on = Style::chosen;
        items.push((
            EN.c32r_make_the_rest,
            on(making == Some(Given::Rest)),
            Action::Codex32Form(C32Press::Rest),
        ));
        items.push((
            EN.c32r_add_a_share,
            on(making == Some(Given::Add)),
            Action::Codex32Form(C32Press::AddShare),
        ));
    }
    items.push((label, Style::Ghost, Action::Codex32Form(C32Press::Check)));
    wrap_buttons(ui, x, y, width, TOUCH, &items) + CORRECT_GAP
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
    let prefix: Text = w.prefix().iter().copied().collect();
    // The word being typed.
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label);
    let off = (ui.measure_as(BODY_S, label) + 12.0).max(64.0);
    // A word of Check my copy's opened for an edit stays hidden, the
    // caret at its end; another's caret stands at its place in the keys
    // (`docs/TEXT-CURSOR.md`).
    use crate::edit::Here;
    let hide = w.is_open() && ui.edit.field == Some(Action::TypeHere(Here::CheckKeyed));
    let shown: &str = if prefix.is_empty() && !caret {
        "…"
    } else if hide {
        "••••••"
    } else {
        &prefix
    };
    let pw = ui.text_as(x + off, y - 4.0, BODY_L.mono(), Role::Primary, shown);
    if let Some(label) = chip {
        ui.demo_offer(label, Action::UseSample);
    }
    let at = ui.edit.caret_in(w.prefix().len());
    let cw = if hide || at >= w.prefix().len() {
        pw
    } else {
        let b = prefix
            .char_indices()
            .nth(at)
            .map_or(prefix.len(), |(b, _)| b);
        ui.measure_as(BODY_L.mono(), &prefix[..b])
    };
    if caret {
        ui.caret(x + off + 2.0 + cw, y - 2.0, 20.0);
    }
    y += 26.0;
    if let Some((status, tone)) = status {
        let status = ui.fit_as(LABEL_M, status, iw);
        ui.text_as(x, y, LABEL_M, tone, &status);
    }
    y += 22.0;
    // What the keys typed can be: a pill each.
    y += keyed_pills(ui, x, y, iw, w, lang, false);
    // The keyboard, unless the passphrase has the docked one up.
    if caret {
        y = word_keys(shift, w, ui, x - 4.0, y, iw + 8.0, 34.0);
    }
    y
}
