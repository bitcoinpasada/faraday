//! `Screen::Entry`: typing a seed.

use super::parts::{
    Cells, dots_field, guide_text_learn, keyed_cells, keyed_pills, never_below, typed_cells,
    word_keys,
};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{CoreUi, Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Screen};
use faraday_draw::widgets::Icon;
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
    let top = 36.0 - app.list_offset;
    let mut y = top;
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    let back = Action::Nav(app.entry_leave());
    ui.icon(
        x - 4.0,
        y,
        16.0,
        Icon::ChevronLeft,
        10.0,
        Role::OnSurfaceVariant,
    );
    ui.text(
        x + 14.0,
        y,
        12.0,
        W::R,
        Role::OnSurfaceVariant,
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
        .state()
        .wallet
        .and_then(|w| app.session.wallets.get(w))
        .map(|wl| wl.name.as_str());
    let heading = match (app.entry.state().wanted, for_wallet) {
        (Some(fp), _) => fill(
            EN.entry_add_key,
            &[&fp_text(faraday_bip::keys::Fingerprint(fp)).to_string()],
        ),
        (None, Some(name)) => fill(EN.entry_add_key_title, &[name]),
        (None, None) if app.entry.state().form != crate::forms::Form::Words => {
            fill(EN.entry_add_a_key_n, &[app.entry.state().form.name()])
        }
        (None, None) => EN.entry_add_a_key.to_string(),
    };
    let heading = ui.fit(26.0, W::S, &heading, width);
    ui.text(x, y, 26.0, W::S, Role::OnSurface, &heading);
    y += 48.0;
    // The words form is the one everyday way in; the other three are
    // behind Other forms, or already shown when a Tools tile opened one
    // of them (`docs/history/SIMPLIFY.md` §2.4).
    if app.entry.state().other_forms {
        let forms: Vec<(String, Style, Action)> = crate::forms::Form::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.name().to_string(),
                    if *f == app.entry.state().form {
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
    if app.entry.state().form != crate::forms::Form::Words {
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
    let lang = app.entry.state().language();
    let english = lang == faraday_bip::bip39::Language::English;
    let mut langs: Vec<(String, Style, Action)> = if english && !app.entry.state().languages {
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
    if page && !app.entry.state().other_forms {
        langs.insert(
            0,
            (
                EN.entry_other_forms.to_string(),
                Style::Ghost,
                Action::EntryOtherForms,
            ),
        );
    }
    if let Some(w) = app.entry.state().keys.as_deref() {
        if app.entry_paste_offered() {
            ui.demo_offer(EN.paste_words, Action::PasteWords);
        }
        y += button_rows(ui, x, y, width, &langs);
        return keyed_words(app, w, ui, x, y, width, h);
    }
    y += guide_text_learn(
        app,
        ui,
        x,
        y,
        width,
        EN.entry_type_the_words,
        crate::learn::Topic::Words,
    );

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
        },
    );
    y += button_rows(ui, x, y, width, &langs);

    // Status.
    let n = words.len();
    let complete = [12, 15, 18, 21, 24].contains(&n)
        && words
            .iter()
            .all(|wd| crate::forms::typed_index(lang, wd).is_some());
    let m = complete.then(|| crate::forms::typed_mnemonic(lang, &app.entry.state().typed));
    entry_status(app, ui, x, y, n, m)
}

/// Closes the clip `entry` opened around its content and reports how far
/// it scrolls: `bottom`, where the content (including the footer) ends,
/// measured from the top of the screen as if unscrolled, so the last
/// scroll leaves the footer and its margin inside the window.
fn entry_scrolled(app: &View, ui: &mut Ui, clip: faraday_draw::Rect, h: f32, bottom: f32) {
    ui.c.pop_clip();
    app.scroll.content_h().set(bottom + app.list_offset + 24.0);
    ui.report_scroll(clip, app.scroll.content_h().get() - h);
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
        ui.text_mid(
            x,
            y,
            32.0,
            13.0,
            W::S,
            Role::OnSurfaceVariant,
            EN.word_count_row_words,
        );
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
    let lang = app.entry.state().language();
    let kind = w.keyboard();
    let mut y = y;
    y += guide_text_learn(
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
        crate::learn::Topic::Words,
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
            boxes: app.entry.state().word_boxes(taken.len() + 1),
            masked: false,
            caret: !app.entry.state().on_passphrase,
            never: app.seed_line,
        },
    );
    // The count and, when the words are a whole key, its checksum and
    // fingerprint, or the last refusal: on the line above the
    // candidates, to keep the keyboard on screen.
    let (status, tone) = match &app.entry.state().error {
        Some(e) => (e.clone(), Role::Error.paint()),
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
    word_keys(app.entry.state().shift, w, ui, x, y, width, kh)
}

/// The words typed on a list's own keyboard, as a status line: the
/// count, and once it is a whole key, the checksum and fingerprint.
pub(crate) fn keyed_status(
    app: &View,
    lang: faraday_bip::bip39::Language,
    taken: &[u16],
) -> (String, crate::ui::Paint) {
    let n = taken.len();
    let count = || {
        (
            fill(EN.keyed_status_word_count, &[&n.to_string()]),
            Role::OnSurfaceVariant.paint(),
        )
    };
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
            Role::Ok,
        ),
        Some(Err(e)) => (e, Role::Error),
        None => (
            fill(EN.entry_status_words, &[&n.to_string()]),
            Role::OnSurfaceVariant,
        ),
    };
    ui.text(x, y, 14.0, W::S, status.1, &status.0);
    y += 30.0;
    if let Some(e) = &app.entry.state().error {
        ui.text(x, y, 13.0, W::R, Role::Error, e);
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
    ui.text_mid(
        x,
        y,
        40.0,
        13.0,
        W::R,
        Role::OnSurfaceVariant,
        EN.entry_finish_bip39_passphrase,
    );
    let px = x + 150.0;
    let pw = 380.0f32.min(width - 150.0);
    let on = app.entry.state().on_passphrase;
    ui.fill(px, y, pw, 40.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(
        px,
        y,
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
        y,
        pw,
        &app.entry.state().passphrase,
        (on, app.pass_chip(crate::demo::PassField::Entry)),
        Action::EntryPassphrase,
    );
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
    use faraday_draw::tokens::{CAPTION, GAP};
    let (muted, row) = (Role::OnSurfaceVariant, 46.0);
    if hx + ui.measure(CAPTION, W::R, hint) <= x + width {
        ui.text_mid(hx, by, row, CAPTION, W::R, muted, hint);
        by + row
    } else {
        let hy = by + row + GAP;
        hy + ui.wrap(x, hy, width, CAPTION, W::R, muted, hint)
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
        ui.text_mid(
            x + 24.0,
            y,
            36.0,
            13.0,
            W::R,
            Role::OnSurfaceVariant,
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
    let form = app.entry.state().form;
    let mut y = y;
    let checking = form == Form::Codex32 && app.entry.state().codex32.checking;
    let guide = match form {
        Form::Slip39 => EN.entry_form_type_one_share,
        Form::Codex32 if checking => EN.codex32_form_check_guide,
        Form::Codex32 => EN.entry_form_type_one_codex32,
        _ => EN.entry_form_type_one,
    };
    y += guide_text_learn(
        app,
        ui,
        x,
        y,
        width,
        guide,
        crate::learn::Topic::OtherBackups,
    );
    let mut replaced = false;
    if form == Form::Codex32 {
        let (hh, r) = codex32_head(app, ui, (x, y, width), true);
        y += hh;
        replaced = r;
    }
    if !replaced {
        entry_field(app, ui, x, y, width, form);
        y += 54.0 + never_below(app, ui, x, y + 44.0, width);
    }
    if let Some(e) = &app.entry.state().error {
        y += ui.wrap(x, y, width, 13.0, W::R, Role::Error, e) + 8.0;
    }
    y += codex32_found(app, ui, x, y, width);
    // What is in.
    let lines = app.entry.state().parts.lines(form);
    section_label(
        ui,
        x,
        y,
        &match lines.len() {
            0 => EN.entry_form_nothing.to_string(),
            1 => EN.entry_form_count_one.to_string(),
            n => fill(EN.entry_form_count_many, &[&n.to_string()]),
        },
    );
    y += 26.0;
    for (n, l) in lines.iter().enumerate() {
        ui.icon(x, y - 2.0, 18.0, Icon::Done, 10.0, Role::Ok);
        let indent = faraday_draw::tokens::PART_INDENT;
        ui.text(x + indent, y, 13.0, W::R, Role::OnSurface, l);
        y += 24.0;
        y += codex32_mark(app, ui, x + indent, y, width - indent, n);
    }
    y += 8.0;
    if form == Form::Codex32 {
        y += codex32_check_button(app, ui, x, y, width);
    }
    // A SLIP-39 backup's passphrase; a Seed XOR key's BIP-39 one.
    if form != Form::Codex32 && flow.is_none() {
        ui.text_mid(
            x,
            y,
            40.0,
            13.0,
            W::R,
            Role::OnSurfaceVariant,
            EN.entry_form_passphrase,
        );
        let px = x + 150.0;
        let pw = 380.0f32.min(width - 150.0);
        let on = app.entry.state().on_passphrase;
        ui.fill(px, y, pw, 40.0, 8.0, Role::SurfaceContainerHighest);
        ui.stroke(
            px,
            y,
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
            y,
            pw,
            &app.entry.state().passphrase,
            (on, app.pass_chip(crate::demo::PassField::Entry)),
            Action::EntryPassphrase,
        );
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
    let ready = app.entry.state().parts.ready(form);
    let aw = ui.button(
        x,
        by,
        None,
        46.0,
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
        46.0,
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
        let hx = x + aw + mw + 28.0;
        return keys_hint(ui, (x, width), (hx, by), EN.entry_form_enter_adds);
    }
    by + 46.0
}

/// The field a share, string or part is typed into, 44 tall.
fn entry_field(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32, form: crate::forms::Form) {
    use crate::forms::Form;
    // What is being typed.
    ui.fill(x, y, width, 44.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(
        x,
        y,
        width,
        44.0,
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
    let typed = if empty && on {
        // The caret stands before the hint while nothing is typed.
        ui.caret(x + 12.0, y + 13.0, 18.0);
        match form {
            Form::Codex32 => crate::secret_text::SecretText::of("ms1…"),
            _ => crate::secret_text::SecretText::of(EN.entry_form_one_share_words),
        }
    } else {
        let mut t = crate::secret_text::SecretText::of(&app.entry.state().typed);
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
        if app.entry.state().typed.is_empty() {
            Role::OnSurfaceVariant
        } else {
            Role::OnSurface
        },
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
    use faraday_draw::tokens::{CORRECT_GAP, LABEL};
    let index = clash
        .text
        .as_str()
        .as_bytes()
        .get(8)
        .copied()
        .unwrap_or(b'?');
    let title = fill(
        EN.codex32_form_clash,
        &[&char::from(index.to_ascii_uppercase()).to_string()],
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
        h += ui.wrap(x, y + h, width, LABEL, W::S, Role::OnSurfaceVariant, label);
        h += string_boxes(ui, x, y + h, width, text, &clash.differ) + CORRECT_GAP;
    }
    let boxes: Vec<String> = clash.differ.iter().map(ToString::to_string).collect();
    h += ui.wrap(
        x,
        y + h,
        width,
        LABEL,
        W::R,
        Role::OnSurfaceVariant,
        &fill(EN.codex32_form_differ, &[&boxes.join(", ")]),
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
    use faraday_draw::tokens::{CORRECT_GAP, CORRECT_LINE_TEXT};
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
    h += ui.wrap(
        x,
        y + h,
        width,
        CORRECT_LINE_TEXT,
        W::R,
        Role::OnSurface,
        &checked.what,
    );
    if let Some(of) = checked.of_the_set {
        let (text, tone) = if of {
            (EN.codex32_form_of_the_set, Role::Ok)
        } else {
            (EN.codex32_form_not_of_the_set, Role::Error)
        };
        h += ui.wrap(x, y + h, width, CORRECT_LINE_TEXT, W::R, tone, text);
    }
    // Strings of its set the open vaults keep (§9.5).
    if checked.holds && e.codex32.check_in_vaults > 0 {
        let n = e.codex32.check_in_vaults.to_string();
        h += ui.wrap(
            x,
            y + h,
            width,
            CORRECT_LINE_TEXT,
            W::R,
            Role::OnSurface,
            &fill(EN.c32r_in_vaults, &[&n]),
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
        // The one pressed is filled: its label fits as measured, where a
        // selected button's check would not.
        let on = |pressed: bool| {
            if pressed {
                Style::Primary
            } else {
                Style::Secondary
            }
        };
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
