//! The New key screen: the step-card column over [`crate::keygen`]. The
//! options and their names are OpenSigner's; this file only lays them out.

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::bip39::Language;
use faraday_entropy::{CameraNoise, CardDraws, CoinFlips, DiceProcedure, RANKS, SUITS};

use crate::drawing::{Scrolled, View};
use crate::keygen::{
    Group, KeyGen, MIX_SOURCES, Source, Start, WAYS, Way, kstep, procedure_name, source_name,
};
use crate::screens::{
    buttons_and_next, descriptor_row, locked_tag, next_button, pass_field, section_label, stepper,
};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, flow, guide};
use faraday_draw::Color;
use faraday_draw::widgets::Icon;

const TITLES: [&str; kstep::COUNT] = [
    EN.keygen_start,
    EN.keygen_length,
    EN.keygen_randomness,
    EN.keygen_entries,
    EN.keygen_key,
    EN.keygen_check,
    EN.keygen_words,
    EN.keygen_quiz,
    EN.keygen_words,
];

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(k) = app.keygen.as_ref() else {
        return;
    };
    let steps = k.steps();
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&s| flow::Card {
            title: title_of(k, s),
            summary: summary(k, s),
            mono: (s == kstep::KEY && k.locked)
                || (s == kstep::QUIZ && k.fingerprint.is_some() && k.done[usize::from(s)]),
            done: k.done[usize::from(s)] || (s == kstep::KEY && k.locked),
            open: k.open == Some(s),
            // Once locked in, nothing before Key changes: no Change.
            default: !k.locked && matches!(s, kstep::START | kstep::LENGTH | kstep::SOURCE),
            toggle: Action::KStep(s),
            // The walk-through is New key's own: a key typed has none.
            guide: (s != kstep::START && s != kstep::TYPED && !(s == kstep::KEY && k.typed()))
                .then(|| guide::keygen(s, k)),
        })
        .collect();
    let back_label = match k.back {
        crate::Screen::Create => EN.keygen_make_a_new_wallet,
        crate::Screen::Entry => EN.keygen_add_a_key,
        crate::Screen::Catalog => EN.keygen_tools,
        _ => EN.keygen_back,
    };
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + 40.0,
        w: (cw - 80.0).min(900.0),
        h,
        back: Some((back_label, Action::Nav(k.back))),
        // Add a key wherever the flow opens on Start; New key for a Create
        // slot, and from Add a key's own Make a new key
        // (`docs/NEW-WALLET.md` §15.3).
        heading: if k.slot.is_some() || k.back == crate::Screen::Entry {
            EN.keygen_new_key
        } else {
            EN.keygen_add_a_key
        },
        guided: app.guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
        cancel: false,
    };
    let scroll = k.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.keygen.as_ref() {
                Some(k) => card_body(app_ref, k, ui, steps[i], x, y, w),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::KeyGen, next, again);
}

/// A card's title.
fn title_of(k: &KeyGen, s: u8) -> String {
    match s {
        kstep::ENTER => entries_title(k),
        // A form other than words is named for itself.
        kstep::TYPED => match k.start.form() {
            Some(f) if f != crate::forms::Form::Words => f.name().to_string(),
            _ => TITLES[usize::from(s)].to_string(),
        },
        _ => TITLES[usize::from(s)].to_string(),
    }
}

/// The Start card (`docs/NEW-WALLET.md` §15.1): Make a new key, Type the
/// words, Scan a SeedQR, and under Other forms the other four; a
/// threshold wallet's slot offers the first two only.
fn start(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let offered = k.starts();
    let one = |ui: &mut Ui, cy: &mut f32, s: Start| {
        row(
            ui,
            x,
            *cy,
            w,
            Row {
                on: k.start == s,
                name: start_name(s),
                line: "",
                action: Action::KStart(s.index()),
            },
        );
        *cy += 60.0;
    };
    for &s in offered.iter().filter(|s| !s.other()) {
        one(ui, &mut cy, s);
    }
    if offered.iter().any(|s| s.other()) {
        let open = k.others_open || k.start.other();
        let lw = ui.button(
            x,
            cy,
            None,
            36.0,
            EN.entry_other_forms,
            Style::Ghost,
            Action::KOthers,
        );
        ui.icon(
            x + lw + 2.0,
            cy + 8.0,
            20.0,
            if open {
                Icon::ChevronUp
            } else {
                Icon::ChevronRight
            },
            10.0,
            DIM,
        );
        cy += 44.0;
        if open {
            for &s in offered.iter().filter(|s| s.other()) {
                one(ui, &mut cy, s);
            }
        }
    }
    if let Some(n) = &k.note {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, n) + 6.0;
    }
    let drawn = next_button(ui, x, cy, w, EN.keygen_continue, Action::KNext);
    cy + (if drawn { 48.0 } else { 8.0 }) - y
}

/// The Words card for a key typed: Add a key's form, drawn by Add a
/// key's own code, then Continue to Key (`docs/NEW-WALLET.md` §15.2).
fn typed(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    // The room a list's own keyboard is sized to: the card grows to it.
    let room = y + 2000.0;
    let words = app.entry.form == crate::forms::Form::Words;
    let mut cy = match (words, ui.compact) {
        (true, false) => crate::screens::entry_words(app, ui, x, y, w, room, false),
        (true, true) => crate::compact_screens::entry_words(app, ui, x, y, w, false),
        (false, false) => {
            return crate::screens::entry_form(app, ui, x, y, w, room, Some(Action::KNext)) + 8.0
                - y;
        }
        (false, true) => {
            return crate::compact_screens::entry_form(app, ui, x, y, w, Some(Action::KNext)) + 8.0
                - y;
        }
    };
    cy += 4.0;
    let drawn = next_button(ui, x, cy, w, EN.keygen_continue, Action::KNext);
    cy + (if drawn { 48.0 } else { 8.0 }) - y
}

fn entries_title(k: &KeyGen) -> String {
    let what = match k.active() {
        Some(Source::Dice) => EN.keygen_rolls,
        Some(Source::Coins) => EN.keygen_flips,
        Some(Source::Cards) => EN.keygen_cards,
        Some(Source::Hex) => EN.keygen_digits,
        Some(Source::Camera) => EN.keygen_pictures,
        Some(Source::Device) => EN.keygen_generator,
        _ => EN.keygen_entries,
    };
    if k.source == Some(Source::Mix) {
        let n = k.mix_list().len();
        fill(
            EN.keygen_source_of,
            &[what, &((k.mix_at + 1).min(n)).to_string(), &n.to_string()],
        )
    } else {
        what.to_string()
    }
}

/// What a source asks for at this length, as its row's second line.
fn asks(k: &KeyGen, s: Source) -> String {
    let st = k.strength();
    match s {
        Source::Dice => fill(EN.keygen_rolls_of_a_six, &[&k.dice_needed().to_string()]),
        Source::Coins => fill(
            EN.keygen_flips_or_rolls,
            &[&CoinFlips::needed(st).to_string()],
        ),
        Source::Cards => fill(
            EN.keygen_cards_from_a,
            &[&CardDraws::needed(st).to_string()],
        ),
        Source::Hex => fill(EN.keygen_digits_made, &[&st.hex_digits().to_string()]),
        Source::Camera => fill(
            EN.keygen_pictures_n,
            &[&CameraNoise::needed(st).to_string()],
        ),
        Source::Mix => EN.keygen_two_or_more.to_string(),
        Source::Device => fill(EN.keygen_getrandom_2, &[EN.create_device_os]),
        Source::SeedXor => String::new(),
    }
}

/// A Start choice's name, on the card and in its summary.
fn start_name(s: Start) -> &'static str {
    match s {
        Start::Make => EN.keygen_make_a_new_key,
        Start::Words => EN.keygen_type_the_words,
        Start::Scan => EN.keygen_scan_a_seedqr,
        Start::MakeShares => EN.keygen_make_slip_39_shares,
        Start::Shares => EN.keygen_type_slip_39_shares,
        Start::Codex32 => EN.keygen_type_a_codex32,
        Start::Xor => EN.keygen_type_seed_xor,
    }
}

fn summary(k: &KeyGen, s: u8) -> String {
    match s {
        kstep::START => start_name(k.start).to_string(),
        kstep::TYPED => match k.mnemonic.as_ref() {
            _ if !k.done[usize::from(kstep::TYPED)] => EN.keygen_not_yet.to_string(),
            Some(m) => fill(EN.keygen_words_n, &[&m.word_count().to_string()]),
            None => fill(EN.entry_form_in, &[&k.parts_in.to_string()]),
        },
        kstep::LENGTH if k.slip39 => fill(
            EN.keygen_slip_39_of_shares,
            &[
                &k.slip_m.to_string(),
                &k.slip_n.to_string(),
                &k.words.to_string(),
            ],
        ),
        kstep::LENGTH => fill(EN.keygen_words_n, &[&k.words.to_string()]),
        kstep::SOURCE => match k.source {
            Some(Source::Dice) => k.way().map(way_name).unwrap_or_default(),
            Some(Source::Coins) if k.by_die => {
                fill(EN.keygen_read_from_a, &[source_name(Source::Coins)])
            }
            Some(Source::Mix) => k
                .mix_list()
                .iter()
                .map(|s| source_name(*s))
                .collect::<Vec<_>>()
                .join(" + "),
            Some(s) => source_name(s).to_string(),
            None => EN.keygen_not_chosen.to_string(),
        },
        kstep::ENTER => {
            if k.done[usize::from(kstep::ENTER)] {
                EN.keygen_complete.to_string()
            } else {
                let (have, need) = k.progress();
                fill(EN.keygen_of, &[&have.to_string(), &need.to_string()])
            }
        }
        kstep::KEY => match k.fingerprint.filter(|_| k.locked) {
            None => EN.keygen_not_locked_in.to_string(),
            Some(fp) => {
                let fp = fp_text(faraday_bip::keys::Fingerprint(fp));
                match (k.asks_passphrase(), k.with_passphrase) {
                    (false, _) => fill(EN.keygen_locked_in, &[&fp.to_string()]),
                    (true, true) => fill(EN.keygen_passphrase_set, &[&fp.to_string()]),
                    (true, false) => fill(EN.keygen_passphrase_not, &[&fp.to_string()]),
                }
            }
        },
        kstep::CHECK => match k.source {
            Some(Source::Device) => EN.create_trusts_device.to_string(),
            Some(Source::Mix) => fill(EN.keygen_sources_combined, &[&k.mixed.len().to_string()]),
            _ if !k.done[usize::from(kstep::ENTER)] => EN.keygen_not_yet.to_string(),
            _ => match k.caution_lines().len() {
                0 => EN.keygen_no_cautions.to_string(),
                1 => EN.keygen_1_caution.to_string(),
                n => fill(EN.keygen_cautions, &[&n.to_string()]),
            },
        },
        kstep::WORDS if k.slip39 => match k.shares.len() {
            0 => EN.keygen_not_made_yet.to_string(),
            n => fill(EN.keygen_shares_n, &[&n.to_string()]),
        },
        kstep::WORDS => match k.mnemonic.as_ref() {
            Some(m) => fill(EN.keygen_words_n, &[&m.indices().len().to_string()]),
            None => EN.keygen_not_made_yet.to_string(),
        },
        _ => match (k.fingerprint, k.done[usize::from(kstep::QUIZ)]) {
            (Some(fp), true) => fp_text(faraday_bip::keys::Fingerprint(fp)),
            _ => match k.quiz.as_ref().map(|q| q.progress()) {
                Some((d, n)) => fill(EN.keygen_of, &[&d.to_string(), &n.to_string()]),
                None => EN.keygen_not_yet.to_string(),
            },
        },
    }
}

fn card_body(app: &View, k: &KeyGen, ui: &mut Ui, s: u8, x: f32, y: f32, w: f32) -> f32 {
    match s {
        kstep::START | kstep::LENGTH | kstep::SOURCE | kstep::ENTER | kstep::TYPED if k.locked => {
            read_only(k, ui, s, x, y, w)
        }
        kstep::START => start(k, ui, x, y, w),
        kstep::TYPED => typed(app, ui, x, y, w),
        kstep::LENGTH => length(k, ui, x, y, w),
        kstep::SOURCE => sources(k, ui, x, y, w),
        kstep::ENTER => entries(k, ui, x, y, w),
        kstep::KEY => key_card(app, k, ui, x, y, w),
        kstep::CHECK => check(k, ui, x, y, w),
        kstep::WORDS => words(k, ui, x, y, w),
        _ => quiz(k, ui, x, y, w),
    }
}

/// A card before Key once the key is locked in: what was chosen and
/// entered, to read, with nothing to press (`docs/NEW-WALLET.md` §3.3).
fn read_only(k: &KeyGen, ui: &mut Ui, s: u8, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match s {
        kstep::START => {
            cy = stat(ui, x, cy, EN.keygen_start, &summary(k, s), TEXT);
        }
        kstep::TYPED => {
            cy = stat(ui, x, cy, &title_of(k, s), &summary(k, s), TEXT);
        }
        kstep::LENGTH => {
            cy = stat(ui, x, cy, EN.keygen_length, &summary(k, s), TEXT);
        }
        kstep::SOURCE => {
            cy = stat(ui, x, cy, EN.keygen_randomness, &summary(k, s), TEXT);
        }
        _ => {
            if k.has_check() {
                cy = check_rows(k, k.active(), ui, x, cy, w);
                cy = verdict(k, ui, x, cy);
            } else {
                cy = check_rows(k, k.source, ui, x, cy, w);
            }
        }
    }
    cy + 6.0 - y
}

/// The randomness check's verdict: "Fair: no cautions", or each caution
/// in `WARN`. Returns where the next row starts.
fn verdict(k: &KeyGen, ui: &mut Ui, x: f32, y: f32) -> f32 {
    let mut cy = y;
    let cautions = k.caution_lines();
    if cautions.is_empty() {
        ui.text(x, cy, 13.0, W::S, OK, EN.keygen_fair_no_cautions);
        cy += 22.0;
    }
    for c in cautions {
        ui.text(x, cy, 13.0, W::R, WARN, c);
        cy += 22.0;
    }
    cy
}

/// A label over a long value, wrapped in the mono face: a key or an
/// address. Returns the height used.
fn long_row(ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, value: &str) -> f32 {
    ui.text(x, y, 12.0, W::R, MUTED, label);
    let h = ui.wrap(x, y + 20.0, w, 13.0, W::M, TEXT, value).max(18.0);
    20.0 + h + 10.0
}

/// The words in their numbered grid, shown or hidden. Returns the height
/// used.
fn word_grid(
    ui: &mut Ui,
    m: &faraday_bip::bip39::Mnemonic,
    shown: bool,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let (lang, words) = (m.language(), m.indices());
    let cols = if ui.compact { 2 } else { 4 };
    let cellw = (w - (cols - 1) as f32 * 10.0) / cols as f32;
    for (i, &idx) in words.iter().enumerate() {
        let cx = x + (i % cols) as f32 * (cellw + 10.0);
        let wy = y + (i / cols) as f32 * 46.0;
        ui.fill(cx, wy, cellw, 38.0, 8.0, BG);
        ui.stroke(cx, wy, cellw, 38.0, 8.0, INNER);
        ui.text_mid(cx + 10.0, wy, 38.0, 11.0, W::R, DIM, &(i + 1).to_string());
        let word = if shown {
            lang.word_display(idx)
        } else {
            "••••••"
        };
        ui.text_mid(cx + 36.0, wy, 38.0, 14.0, W::M, TEXT, word);
    }
    words.len().div_ceil(cols) as f32 * 46.0 + 8.0
}

/// The Key card (`docs/NEW-WALLET.md` §3.3): the words, a passphrase
/// for a Create slot, and Lock in; once locked in, the fingerprint and
/// what the key makes, and Continue.
fn key_card(app: &View, k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    // A key typed in a form other than words has no words to show: the
    // parts typed, until it is locked in (§15.2).
    let parts = k.typed() && k.mnemonic.is_none() && k.done[usize::from(kstep::TYPED)];
    if k.mnemonic.is_none() && !parts {
        ui.text(x, cy, 13.0, W::R, MUTED, EN.keygen_not_made_yet);
        return 26.0;
    }
    if k.locked {
        locked_tag(ui, x, cy);
        cy += 38.0;
    }
    if let Some(m) = k.mnemonic.as_ref() {
        cy += word_grid(ui, m, k.shown, x, cy, w);
    } else if !k.locked {
        for l in app.entry.parts.lines(app.entry.form) {
            ui.icon(x, cy - 2.0, 18.0, Icon::Done, 10.0, OK);
            cy += ui.wrap(x + 24.0, cy, w - 24.0, 13.0, W::R, TEXT, &l) + 10.0;
        }
        cy += 4.0;
    }
    let show = (
        if k.shown {
            EN.keygen_hide_words
        } else {
            EN.keygen_show_words
        },
        Style::Secondary,
        Action::KShow,
    );
    let shows: &[(&str, Style, Action)] = if k.mnemonic.is_some() {
        std::slice::from_ref(&show)
    } else {
        &[]
    };
    if !k.locked {
        if k.asks_passphrase() {
            cy += 4.0;
            cy += pass_field(
                ui,
                x,
                cy,
                w,
                EN.keygen_passphrase,
                &k.passphrase,
                k.focus == Some(0),
                Action::KPassField(0),
            );
            cy += pass_field(
                ui,
                x,
                cy,
                w,
                EN.keygen_passphrase_again,
                &k.passphrase2,
                k.focus == Some(1),
                Action::KPassField(1),
            );
        }
        // The demo adds a public seed in place of words made here (F20).
        if !k.typed()
            && let Some((word, fp)) = app.session.next_handout()
        {
            let line = fill(EN.keygen_lock_in_adds, &[word, &fp_text(fp)]);
            cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, &line) + 6.0;
        }
        if let Some(n) = &k.note {
            cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, n) + 6.0;
        }
        cy += 4.0;
        cy += buttons_and_next(
            ui,
            x,
            cy,
            w,
            shows,
            Some((EN.keygen_lock_in, Action::KLock)),
        );
        if !ui.compact {
            ui.text(x, cy, 12.0, W::R, DIM, EN.keygen_enter_locks);
            cy += 24.0;
        }
        return cy - y;
    }
    if k.asks_passphrase() {
        cy = stat_pill(
            ui,
            x,
            cy,
            EN.keygen_passphrase,
            if k.with_passphrase {
                EN.keygen_set
            } else {
                EN.keygen_not_set
            },
        );
    }
    if let Some(fp) = k.fingerprint {
        cy = stat(
            ui,
            x,
            cy,
            EN.keygen_fingerprint,
            &fp_text(faraday_bip::keys::Fingerprint(fp)),
            TEXT,
        );
    }
    cy += handed_line(k, ui, x, cy, w);
    cy += 6.0;
    let create = app.create.as_ref().filter(|_| k.slot.is_some());
    match create {
        // A single-key wallet is this key: its descriptor and its first
        // address.
        Some(c) if !c.kind.multi() => {
            if let Some(i) = c.built
                && let Some(wl) = app.session.wallets.get(i)
            {
                cy += descriptor_row(ui, x, cy, w, wl, Action::QrWallet(i));
                let (a, held) = app.receive_line(wl, false, 0);
                if held {
                    cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, &a) + 8.0;
                } else {
                    cy += long_row(ui, x, cy, w, EN.keygen_first_address, &a);
                }
            }
        }
        // A slot of a wallet of several keys, or the key on its own: the
        // key as the wallet or a cosigner takes it.
        _ => {
            if let Some(t) = &k.key_text {
                cy += long_row(ui, x, cy, w, EN.keygen_key, t);
            }
        }
    }
    if let Some(n) = &k.note {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, n) + 6.0;
    }
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        shows,
        Some((EN.keygen_continue, Action::KNext)),
    );
    cy - y
}

/// What SLIP-39 shares reveal and what they do not, stated beside the
/// choice so the person can weigh it.
const SLIP39_FACTS: [&str; 7] = [
    EN.keygen_fewer_shares,
    EN.keygen_each_share_states,
    EN.keygen_the_threshold,
    EN.keygen_the_split_s,
    EN.keygen_a_share_cannot,
    EN.keygen_shares_restore,
    EN.keygen_a_share_is_20,
];

fn length(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    // Words or shares.
    if !k.only_24 {
        let mut bx = x;
        for (label, on) in [
            (EN.keygen_bip_39_words, false),
            (EN.keygen_slip_39_shares, true),
        ] {
            let style = if k.slip39 == on {
                Style::Primary
            } else {
                Style::Secondary
            };
            bx += ui.button(bx, cy, None, 36.0, label, style, Action::KForm(on)) + 8.0;
        }
        cy += 50.0;
    }
    let used = length_counts(k, ui, x, cy, w);
    cy += used;
    if !k.slip39 {
        cy += 4.0;
        if next_button(
            ui,
            x,
            cy,
            w.min(760.0),
            EN.keygen_continue,
            Action::KWords(k.words as u8),
        ) {
            cy += 52.0;
        }
    }
    if k.slip39 {
        for (label, value, minus, plus) in [
            (
                EN.keygen_shares_needed,
                k.slip_m,
                Action::KSlipM(-1),
                Action::KSlipM(1),
            ),
            (
                EN.keygen_shares_dealt,
                k.slip_n,
                Action::KSlipN(-1),
                Action::KSlipN(1),
            ),
        ] {
            stepper(ui, x, cy, w, label, usize::from(value), minus, plus, true);
            cy += 50.0;
        }
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::S,
            TEXT,
            &fill(
                EN.keygen_any_of_the_shares,
                &[&k.slip_m.to_string(), &k.slip_n.to_string()],
            ),
        ) + 12.0;
        ui.text(x, cy, 13.0, W::S, MUTED, EN.keygen_what_shares);
        cy += 24.0;
        for f in SLIP39_FACTS {
            ui.fill(x + 2.0, cy + 7.0, 4.0, 4.0, 2.0, MUTED);
            cy += ui.wrap(x + 16.0, cy, w.min(760.0) - 16.0, 13.0, W::R, TEXT, f) + 8.0;
        }
        cy += 4.0;
        if next_button(
            ui,
            x,
            cy,
            w.min(760.0),
            EN.keygen_continue,
            Action::KWords(k.words as u8),
        ) {
            cy += 52.0;
        }
    }
    cy - y
}

fn length_counts(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut bx = x;
    let mut y = y;
    let top = y;
    for &n in k.counts() {
        // Wrapped to the width: a small panel takes two to a row.
        if bx > x && bx + 104.0 > x + w {
            bx = x;
            y += 48.0;
        }
        let allowed = !k.only_24 || n == 24;
        let style = if !allowed {
            Style::Disabled
        } else if usize::from(n) == k.words {
            // The default shows chosen from the start, as any choice
            // shows its value (`docs/NEW-WALLET.md` §13.3).
            Style::Primary
        } else {
            Style::Secondary
        };
        bx += ui.button(
            bx,
            y,
            Some(104.0),
            40.0,
            &fill(EN.keygen_words_n, &[&n.to_string()]),
            style,
            if k.slip39 {
                Action::KSlipWords(n)
            } else {
                Action::KWords(n)
            },
        ) + 8.0;
    }
    let mut used = y - top + 52.0;
    if k.only_24 {
        ui.text(x, y + used, 13.0, W::R, MUTED, EN.keygen_a_threshold);
        used += 26.0;
    }
    used
}

/// One choosable row: a radio mark, a name and a second line.
struct Row<'a> {
    on: bool,
    name: &'a str,
    line: &'a str,
    action: Action,
}

fn row(ui: &mut Ui, x: f32, y: f32, w: f32, r: Row) {
    let Row {
        on,
        name,
        line,
        action: a,
    } = r;
    ui.fill(x, y, w, 52.0, 10.0, if on { INNER } else { BG });
    ui.stroke(
        x,
        y,
        w,
        52.0,
        10.0,
        if on { ACCENT.with_alpha(110) } else { LINE },
    );
    ui.dot(x + 22.0, y + 26.0, 7.0, if on { ACCENT } else { BORDER });
    if !on {
        ui.dot(x + 22.0, y + 26.0, 5.0, BG);
    }
    // A row the frame dims (the demo's SLIP-39 sources) reads dimmed and
    // takes no press.
    let dim = ui.dimmed.contains(&a);
    let name = ui.fit(14.0, W::S, name, w - 54.0);
    // A row with no second line has its name in the middle.
    if line.is_empty() {
        ui.text_mid(
            x + 42.0,
            y,
            52.0,
            14.0,
            W::S,
            if dim { DIM } else { TEXT },
            &name,
        );
        ui.hit(x, y, w, 52.0, a);
        return;
    }
    ui.text(
        x + 42.0,
        y + 8.0,
        14.0,
        W::S,
        if dim { DIM } else { TEXT },
        &name,
    );
    let line = ui.fit(12.0, W::R, line, w - 54.0);
    ui.text(
        x + 42.0,
        y + 29.0,
        12.0,
        W::R,
        if dim { DIM } else { MUTED },
        &line,
    );
    ui.hit(x, y, w, 52.0, a);
}

/// An option's name on the Randomness card and in its summary.
fn way_name(w: Way) -> String {
    match w {
        Way::DiceFlips => fill(EN.keygen_flip_mode, &[source_name(Source::Dice)]),
        Way::DiceWords | Way::DiceHashed | Way::DiceSixAsZero => format!(
            "{} · {}",
            source_name(Source::Dice),
            procedure_name(match w {
                Way::DiceWords => DiceProcedure::Words,
                Way::DiceSixAsZero => DiceProcedure::SixAsZero,
                _ => DiceProcedure::Hashed,
            })
        ),
        w => source_name(w.source()).to_string(),
    }
}

/// What an option asks for at this length, as its row's second line.
fn way_line(k: &KeyGen, w: Way) -> String {
    let st = k.strength();
    match w {
        Way::DiceFlips => fill(
            EN.keygen_rolls_read_as,
            &[&CoinFlips::needed(st).to_string()],
        ),
        Way::DiceWords | Way::DiceHashed | Way::DiceSixAsZero => {
            let p = match w {
                Way::DiceWords => DiceProcedure::Words,
                Way::DiceSixAsZero => DiceProcedure::SixAsZero,
                _ => DiceProcedure::Hashed,
            };
            EN.dice_procedure_rolls
                .replacen("{}", &k.rolls_for(p).to_string(), 1)
        }
        w => asks(k, w.source()),
    }
}

/// A group's heading. Each group opens and closes (this device's closed
/// at the start); a closed one names the option chosen in it.
fn group_head(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32, g: Group) -> f32 {
    let title = match g {
        Group::ByHand => EN.keygen_your_own_entropy,
        Group::Computed => EN.keygen_your_entropy,
        Group::Device => EN.keygen_made_by_this,
    };
    if ui.compact {
        let mut cy = y;
        cy += ui.wrap(x, cy, w - 34.0, 14.0, W::S, MUTED, title);
        let i = group_index(g);
        let open = k.groups_open[usize::from(i)];
        let chosen = k.way().filter(|w| w.group(k.slip39) == g);
        if g == Group::ByHand || (!open && chosen.is_some()) {
            cy += 4.0;
            let mut bx = x;
            if g == Group::ByHand {
                bx += ui.chip(x, cy, EN.keygen_recommended, OK, OK.with_alpha(30)) + 8.0;
            }
            if let Some(way) = chosen.filter(|_| !open) {
                let name = ui.fit(13.0, W::S, &way_name(way), x + w - bx);
                ui.text(bx, cy + 2.0, 13.0, W::S, ACCENT, &name);
            }
            cy += 24.0;
        }
        ui.icon(
            x + w - 24.0,
            y - 3.0,
            20.0,
            if open {
                Icon::ChevronUp
            } else {
                Icon::ChevronRight
            },
            10.0,
            DIM,
        );
        ui.hit(x, y - 8.0, w, cy - y + 8.0, Action::KGroup(i));
        return cy - y + 8.0;
    }
    section_label(ui, x, y, title);
    let i = group_index(g);
    if g == Group::ByHand {
        let lw = ui.measure(14.0, W::S, title);
        ui.chip(
            x + lw + 12.0,
            y - 4.0,
            EN.keygen_recommended,
            OK,
            OK.with_alpha(30),
        );
    }
    let open = k.groups_open[usize::from(i)];
    ui.icon(
        x + w - 30.0,
        y - 3.0,
        20.0,
        if open {
            Icon::ChevronUp
        } else {
            Icon::ChevronRight
        },
        10.0,
        DIM,
    );
    if !open && let Some(way) = k.way().filter(|w| w.group(k.slip39) == g) {
        ui.text_right(
            x + w - 40.0,
            y - 6.0,
            30.0,
            13.0,
            W::S,
            ACCENT,
            &way_name(way),
        );
    }
    ui.hit(x, y - 8.0, w, 32.0, Action::KGroup(i));
    28.0
}

/// A group's place in [`KeyGen::groups_open`], which `KGroup` carries.
fn group_index(g: Group) -> u8 {
    match g {
        Group::ByHand => 0,
        Group::Computed => 1,
        Group::Device => 2,
    }
}

fn sources(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let chosen = k.way();
    for g in [Group::ByHand, Group::Computed, Group::Device] {
        let ways: Vec<Way> = WAYS
            .iter()
            .copied()
            .filter(|w| w.offered(k.slip39) && w.group(k.slip39) == g)
            .collect();
        if ways.is_empty() {
            continue;
        }
        if cy > y {
            cy += 8.0;
        }
        cy += group_head(k, ui, x, cy, w, g);
        let open = k.groups_open[usize::from(group_index(g))];
        if !open {
            cy += 4.0;
            continue;
        }
        for way in ways {
            let line = way_line(k, way);
            row(
                ui,
                x,
                cy,
                w,
                Row {
                    on: chosen == Some(way),
                    name: &way_name(way),
                    line: &line,
                    action: Action::KWay(way.index()),
                },
            );
            cy += 60.0;
            // The mix's sources open under its row.
            if chosen == Some(way) && way == Way::Mix {
                for (j, &m) in MIX_SOURCES.iter().enumerate() {
                    let on = k.mix[j];
                    let live = !ui.dimmed.contains(&Action::KMix(j as u8));
                    ui.checkbox(x + 44.0, cy + 7.0, on, live);
                    let tone = if live { TEXT } else { DIM };
                    ui.text(x + 74.0, cy + 6.0, 14.0, W::R, tone, source_name(m));
                    let nw = ui.measure(14.0, W::R, source_name(m));
                    ui.text(x + 86.0 + nw, cy + 7.0, 12.0, W::R, MUTED, &asks(k, m));
                    ui.hit(x + 40.0, cy, w - 40.0, 32.0, Action::KMix(j as u8));
                    cy += 34.0;
                }
                cy += 8.0;
            }
        }
    }
    if let Some(n) = &k.note {
        ui.text(x, cy + 10.0, 13.0, W::R, ERR, n);
    }
    let drawn = next_button(ui, x, cy, w, EN.keygen_continue, Action::KNext);
    cy + (if drawn { 48.0 } else { 8.0 }) - y
}

fn entries(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let (have, need) = k.progress();
    let active = k.active();
    // How far along, as a count and a bar.
    if active != Some(Source::Device) {
        let unit = match active {
            Some(Source::Dice) => EN.keygen_rolls_inline,
            Some(Source::Coins) => EN.keygen_flips_inline,
            Some(Source::Cards) => EN.keygen_cards_inline,
            Some(Source::Hex) => EN.keygen_digits_inline,
            _ => EN.keygen_pictures_inline,
        };
        ui.text(
            x,
            cy,
            15.0,
            W::S,
            TEXT,
            &fill(
                EN.keygen_of_2,
                &[&have.to_string(), &need.to_string(), unit],
            ),
        );
        if let Some(last) = last_entry(k) {
            ui.text_right(
                x + w,
                cy - 6.0,
                30.0,
                13.0,
                W::M,
                MUTED,
                &fill(EN.keygen_last, &[&last.to_string()]),
            );
        }
        cy += 28.0;
        ui.fill(x, cy, w, 6.0, 3.0, INNER);
        let f = if need == 0 {
            0.0
        } else {
            (have as f32 / need as f32).min(1.0)
        };
        if f > 0.0 {
            ui.fill(
                x,
                cy,
                w * f,
                6.0,
                3.0,
                if have >= need { OK } else { ACCENT },
            );
        }
        cy += 22.0;
    }
    if matches!(active, Some(Source::Dice | Source::Coins)) {
        cy = entry_style(k, ui, x, cy, w);
    }
    match active {
        Some(Source::Dice | Source::Coins) if k.typing => {
            cy = typed_box(k, ui, x, cy, w);
            if k.reveals() {
                cy = reveal(k, ui, x, cy, w);
            }
        }
        Some(Source::Coins) if k.by_die => {
            let bw = ((w - 5.0 * 8.0) / 6.0).min(84.0);
            for f in 1..=6u8 {
                ui.button(
                    x + f32::from(f - 1) * (bw + 8.0),
                    cy,
                    Some(bw),
                    56.0,
                    &f.to_string(),
                    Style::Secondary,
                    Action::KDie(f),
                );
            }
            cy += 68.0;
            if k.reveals() {
                cy = reveal(k, ui, x, cy, w);
            }
        }
        Some(Source::Dice) => {
            let bw = ((w - 5.0 * 8.0) / 6.0).min(84.0);
            for f in 1..=6u8 {
                let style = if k.procedure().direct() && !k.procedure().accepts(k.dice.len(), f) {
                    Style::Disabled
                } else {
                    Style::Secondary
                };
                ui.button(
                    x + f32::from(f - 1) * (bw + 8.0),
                    cy,
                    Some(bw),
                    56.0,
                    &f.to_string(),
                    style,
                    Action::KRoll(f),
                );
            }
            cy += 68.0;
            if k.reveals() {
                cy = reveal(k, ui, x, cy, w);
            }
        }
        Some(Source::Coins) => {
            ui.button(
                x,
                cy,
                Some(160.0),
                56.0,
                EN.create_heads,
                Style::Secondary,
                Action::KFlip(true),
            );
            ui.button(
                x + 168.0,
                cy,
                Some(160.0),
                56.0,
                EN.create_tails,
                Style::Secondary,
                Action::KFlip(false),
            );
            cy += 68.0;
            if k.reveals() {
                cy = reveal(k, ui, x, cy, w);
            }
        }
        Some(Source::Cards) => {
            let bw = ((w - 12.0 * 6.0) / 13.0).min(52.0);
            for (r, ch) in RANKS.iter().enumerate() {
                let on = k.rank == Some(r as u8);
                ui.button(
                    x + r as f32 * (bw + 6.0),
                    cy,
                    Some(bw),
                    44.0,
                    &ch.to_string(),
                    if on { Style::Primary } else { Style::Secondary },
                    Action::KRank(r as u8),
                );
            }
            cy += 52.0;
            for (s, ch) in SUITS.iter().enumerate() {
                let taken = k
                    .rank
                    .and_then(|r| faraday_entropy::card_index(r, s as u8))
                    .is_some_and(|i| k.cards.drawn(i));
                let style = if k.rank.is_none() || taken {
                    Style::Disabled
                } else {
                    Style::Secondary
                };
                // Four to the row's width at most; on a small panel the
                // deck goes under them.
                let sw = ((w - 3.0 * 8.0) / 4.0).min(64.0);
                ui.button(
                    x + s as f32 * (sw + 8.0),
                    cy,
                    Some(sw),
                    44.0,
                    &ch.to_string(),
                    style,
                    Action::KSuit(s as u8),
                );
            }
            let deck = fill(
                EN.keygen_deck_drawn_from,
                &[&k.cards.deck().to_string(), &k.cards.in_deck().to_string()],
            );
            if ui.compact {
                cy += 52.0;
                ui.text(x, cy, 12.0, W::R, MUTED, &deck);
                cy += 28.0;
            } else {
                ui.text_mid(x + 4.0 * 72.0 + 8.0, cy, 44.0, 12.0, W::R, MUTED, &deck);
                cy += 56.0;
            }
        }
        Some(Source::Hex) => {
            let bw = ((w - 7.0 * 6.0) / 8.0).min(60.0);
            for v in 0..16u8 {
                let label = char::from_digit(u32::from(v), 16)
                    .unwrap_or('0')
                    .to_string();
                ui.button(
                    x + f32::from(v % 8) * (bw + 6.0),
                    cy + f32::from(v / 8) * 48.0,
                    Some(bw),
                    42.0,
                    &label,
                    Style::Secondary,
                    Action::KHex(v),
                );
            }
            cy += 104.0;
        }
        Some(Source::Camera) => {
            let bw = ui.button(
                x,
                cy,
                None,
                48.0,
                EN.create_camera_shutter,
                if k.frame.is_some() {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                Action::KFrame,
            );
            let state = match k.camera.last() {
                Some(st) => fill(EN.keygen_luma_values, &[&st.distinct.to_string()]),
                None if k.frame.is_some() => EN.keygen_camera_on.to_string(),
                None => EN.keygen_waiting_for.to_string(),
            };
            ui.text_mid(x + bw + 16.0, cy, 48.0, 13.0, W::R, MUTED, &state);
            cy += 60.0;
        }
        Some(Source::Device) => {
            cy = device_rows(ui, x, cy, w);
        }
        _ => {}
    }
    // Words for a Create slot are shown and checked again by Back up
    // (`docs/NEW-WALLET.md` §3.2).
    if !k.slip39 && k.reveals() && k.slot.is_some() {
        cy += ui.wrap(x, cy, w, 12.0, W::R, MUTED, EN.keygen_no_need_to_write) + 10.0;
    }
    // BIP-39 words: the randomness check is here, once the last entry is
    // in. Before that, and for shares, cautions as they appear.
    let ready = k.ready();
    let checked = !k.slip39 && ready && k.has_check();
    if checked {
        cy += 4.0;
        section_label(ui, x, cy, EN.keygen_randomness_check);
        cy += 26.0;
        cy = check_rows(k, active, ui, x, cy, w);
        cy = verdict(k, ui, x, cy);
    } else {
        for c in k.caution_lines() {
            ui.text(x, cy, 13.0, W::R, WARN, c);
            cy += 22.0;
        }
    }
    // What the last press did.
    if let Some(n) = &k.note {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, n) + 4.0;
    }
    cy += 6.0;
    let last_of_mix = k.source != Some(Source::Mix) || k.mix_at + 1 >= k.mix_list().len();
    let label = if !last_of_mix {
        EN.keygen_next_source
    } else if !k.slip39 && k.reveals() {
        EN.keygen_continue
    } else {
        EN.keygen_make_the_words
    };
    let mut edits: Vec<(&str, Style, Action)> = Vec::new();
    if !matches!(active, Some(Source::Device)) {
        edits.push((EN.keygen_undo, Style::Ghost, Action::KUndo));
        edits.push((EN.keygen_clear, Style::Ghost, Action::KClear));
    }
    // A caution is not a refusal: the foot button still works, and the
    // source's word for starting again is beside it.
    if checked && !k.caution_lines().is_empty() {
        edits.push((again_label(k), Style::Ghost, Action::KAgain));
    }
    // Words' foot button waits for the last entry.
    let next = (k.slip39 || ready).then_some((label, Action::KNext));
    cy += buttons_and_next(ui, x, cy, w, &edits, next) + 4.0;
    if ui.compact {
        return cy - y;
    }
    let hint = match active {
        Some(Source::Dice) => EN.keygen_type_1_to_6,
        Some(Source::Coins) if k.by_die => EN.keygen_type_1_to_6,
        Some(Source::Coins) => EN.keygen_type_h_or_t,
        Some(Source::Cards) => EN.keygen_type_the_rank,
        Some(Source::Hex) => EN.keygen_type_09_and,
        Some(Source::Camera) => EN.keygen_space_takes,
        _ => EN.keygen_enter_continues,
    };
    ui.text(x, cy, 12.0, W::R, DIM, hint);
    cy + 24.0 - y
}

/// How entries are made: for coins, a coin or a die read as one; for
/// both, pressed one at a time or typed as one string. Returns where the
/// next row starts.
fn entry_style(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let style = |on: bool| if on { Style::Primary } else { Style::Secondary };
    let mut bx = x;
    if k.active() == Some(Source::Coins) {
        for (label, on) in [(EN.keygen_coin, false), (EN.keygen_die, true)] {
            bx += ui.button(
                bx,
                cy,
                None,
                34.0,
                label,
                style(k.by_die == on),
                Action::KByDie(on),
            ) + 6.0;
        }
        bx += 18.0;
        // Typed and Buttons go on a row of their own when the four do
        // not fit.
        let need: f32 = [EN.keygen_typed, EN.keygen_buttons]
            .iter()
            .map(|l| ui.measure(13.0, W::S, l) + 38.0)
            .sum();
        if bx + need > x + w {
            bx = x;
            cy += 42.0;
        }
    }
    for (label, on) in [(EN.keygen_typed, true), (EN.keygen_buttons, false)] {
        bx += ui.button(
            bx,
            cy,
            None,
            34.0,
            label,
            style(k.typing == on),
            Action::KTyping(on),
        ) + 6.0;
    }
    cy += 46.0;
    if k.active() == Some(Source::Coins) {
        let line = if k.by_die {
            EN.keygen_each_roll_is
        } else {
            EN.keygen_heads_is_1_tails
        };
        cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, line) + 10.0;
    }
    cy
}

/// The box entries are typed into. Each is taken as it is typed, and
/// the box shows every one taken, by buttons too.
fn typed_box(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    ui.fill(x, y, w, 40.0, 8.0, BG);
    ui.stroke(x, y, w, 40.0, 8.0, ACCENT);
    // A press in the box places the caret, a drag selects rolls, and
    // what is deleted there leaves the key (`docs/NEW-WALLET.md` §13.2).
    ui.typed(
        x + 12.0,
        y,
        40.0,
        14.0,
        W::M,
        TEXT,
        &k.entered,
        w - 30.0,
        true,
        Action::KTyping(true),
    );
    ui.hit(x, y, w, 40.0, Action::KTyping(true));
    let what = match k.active() {
        Some(Source::Coins) if !k.by_die => EN.keygen_h_or_1_for_heads,
        _ => EN.keygen_faces_1_to_6,
    };
    ui.text(x, y + 48.0, 12.0, W::R, DIM, what);
    y + 76.0
}

/// The words the entries name, as they come in: each word's place, the
/// flips or rolls that name it, its number in the BIP-39 list and the
/// word, with a link to it in the list. The last place shows no word
/// until the words are made, because its last bits are the checksum.
fn reveal(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if k.source == Some(Source::Dice) && !k.by_die {
        return rolled_words(k, ui, x, y, w);
    }
    let mut cy = y + 4.0;
    section_label(ui, x, cy, EN.keygen_words_so_far);
    cy += 24.0;
    ui.text(x, cy, 12.0, W::R, MUTED, crate::wordlist::PRESS_A_WORD);
    cy += 24.0;
    let live = k.live_words();
    // Flips always land in `k.coins`, whether Coins or Dice's Flip mode
    // took them.
    let coins = k.source == Some(Source::Coins) || k.by_die;
    let flips: zeroize::Zeroizing<Vec<u8>> =
        zeroize::Zeroizing::new(k.coins.flips().map(u8::from).collect());
    let rolls = k.dice.rolls();
    let per = if coins {
        11
    } else {
        faraday_entropy::DICE_WORD_ROLLS
    };
    let checksum = k.words / 3;
    let code_of = |slot: usize, len: usize| -> String {
        let from = slot * per;
        if coins {
            flips
                .iter()
                .skip(from)
                .take(len)
                .map(|b| if *b == 1 { '1' } else { '0' })
                .collect()
        } else {
            rolls
                .iter()
                .skip(from)
                .take(len)
                .map(|r| char::from(b'0' + r))
                .collect()
        }
    };
    let entered = if coins { flips.len() } else { rolls.len() };
    let num_w = ui.measure(13.0, W::M, "00");
    let code_w = ui.measure(13.0, W::M, &"0".repeat(per.max(11)));
    let index_w = ui.measure(13.0, W::M, "0000");
    // On a small panel the number in the list and the word go on a
    // second line, under the bits.
    let narrow = ui.compact;
    let (cx, ix, wx, down, step) = if narrow {
        let cx = x + num_w + 14.0;
        (cx, cx, cx + index_w + 14.0, 26.0, 58.0)
    } else {
        (
            x + num_w + 14.0,
            x + num_w + code_w + 34.0,
            x + num_w + code_w + index_w + 54.0,
            0.0,
            30.0,
        )
    };
    let row = |ui: &mut Ui, cy: f32, slot: usize, code: &str, index: Option<u16>, note: &str| {
        let n = (slot + 1).to_string();
        let nw = ui.measure(13.0, W::M, &n);
        ui.text_mid(x + num_w - nw, cy, 30.0, 13.0, W::M, DIM, &n);
        ui.text_mid(cx, cy, 30.0, 13.0, W::M, MUTED, code);
        let cy = cy + down;
        match index {
            Some(i) => {
                let num = (i + 1).to_string();
                let tw = ui.measure(13.0, W::M, &num);
                ui.text_mid(ix + index_w - tw, cy, 30.0, 13.0, W::M, MUTED, &num);
                ui.word_pill(wx, cy + 2.0, Language::English.word(i), open_word(i));
            }
            None => {
                let note = ui.fit(12.0, W::R, note, x + w - ix);
                ui.text_mid(ix, cy, 30.0, 12.0, W::R, DIM, &note);
            }
        }
    };
    let last = k.words - 1;
    for (slot, &i) in live.iter().enumerate() {
        row(ui, cy, slot, &code_of(slot, per), Some(i), "");
        cy += step;
    }
    // The word being entered, when it is not the last.
    let at = live.len();
    let part = entered.saturating_sub(at * per);
    if at < last && part > 0 {
        let note = if coins {
            fill(EN.keygen_of_11_bits, &[&part.to_string()])
        } else {
            fill(EN.keygen_of_rolls, &[&part.to_string(), &per.to_string()])
        };
        row(ui, cy, at, &code_of(at, part.min(per)), None, &note);
        cy += step;
    }
    // The last word: its own bits so far, then the checksum's.
    let made = k.checksum_word();
    let code = match made {
        Some(i) if coins => format!("{i:011b}"),
        _ if coins => {
            let have = entered.saturating_sub(last * 11).min(11 - checksum);
            format!("{}{}", code_of(last, have), "·".repeat(11 - have))
        }
        _ => code_of(last, per),
    };
    row(ui, cy, last, &code, made, EN.keygen_from_the_checksum);
    cy += step;
    if coins && made.is_some() {
        cy += ui.wrap(
            x,
            cy,
            w,
            12.0,
            W::R,
            DIM,
            &fill(
                EN.keygen_the_last_bits,
                &[&checksum.to_string(), &(last + 1).to_string()],
            ),
        ) + 8.0;
    }
    cy + 8.0
}

/// Rolls that name words (BitBox) as they come in, word by word: each
/// roll over the bits it stands for, the eleven bits read as the word's
/// number in the list, and the word, with a link to it in the list. The
/// last word's first bits are rolled the same way, and only those; its
/// last bits are the checksum, shown as soon as the last of them is in.
fn rolled_words(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y + 4.0;
    section_label(ui, x, cy, EN.keygen_words_so_far);
    cy += 24.0;
    ui.text(x, cy, 12.0, W::R, MUTED, crate::wordlist::PRESS_A_WORD);
    cy += 20.0;
    cy += ui.wrap(x, cy, w, 12.0, W::R, MUTED, EN.keygen_rolls_1_to_5) + 12.0;
    let per = faraday_entropy::DICE_WORD_ROLLS;
    let faces = faraday_entropy::DICE_WORD_FACES;
    let rolls = k.dice.rolls();
    let live = k.live_words();
    let last = k.words - 1;
    let checksum = k.words / 3;
    let num_w = ui.measure(13.0, W::M, "00");
    let cell = ui.measure(13.0, W::M, "00") + 12.0;
    let cells_x = x + num_w + 14.0;
    let info_x = cells_x + cell * per as f32 + 10.0;
    // On a small panel what a word's rolls name goes on a line under
    // them, from where the rolls start.
    let compact = ui.compact;
    let info_y = if compact { 46.0 } else { 0.0 };
    let info_w = x + w - cells_x;
    // A word's row: its rolls over their bits, then what they name.
    let row = |ui: &mut Ui,
               cy: f32,
               slot: usize,
               cells: &[(String, String)],
               sum: Option<&str>,
               index: Option<u16>,
               note: &str| {
        let n = (slot + 1).to_string();
        let nw = ui.measure(13.0, W::M, &n);
        ui.text_mid(x + num_w - nw, cy, 22.0, 13.0, W::M, DIM, &n);
        for (c, (face, bits)) in cells.iter().enumerate() {
            let cx = cells_x + c as f32 * cell;
            for (line, text, size, tone) in [(0.0, face, 13.0, TEXT), (22.0, bits, 13.0, MUTED)] {
                let tw = ui.measure(size, W::M, text);
                ui.text_mid(
                    cx + (cell - tw) / 2.0,
                    cy + line,
                    22.0,
                    size,
                    W::M,
                    tone,
                    text,
                );
            }
        }
        // The checksum's bits, after the last word's rolled ones.
        let mut info_x = if compact { cells_x } else { info_x };
        if let Some(sum) = sum {
            let sx = cells_x + cells.len() as f32 * cell + 6.0;
            ui.text_mid(sx, cy, 22.0, 11.0, W::R, DIM, EN.keygen_checksum);
            ui.text_mid(sx, cy + 22.0, 22.0, 13.0, W::M, MUTED, sum);
            let sw = ui
                .measure(11.0, W::R, EN.keygen_checksum)
                .max(ui.measure(13.0, W::M, sum));
            if !compact {
                info_x = info_x.max(sx + sw + 14.0);
            }
        }
        let cy = cy + info_y;
        match index {
            Some(i) if compact => {
                let pw = ui.word_pill(info_x, cy - 3.0, Language::English.word(i), open_word(i));
                let line = fill(EN.keygen_word, &[&i.to_string(), &(i + 1).to_string()]);
                ui.text_mid(info_x + pw + 8.0, cy - 3.0, 26.0, 12.0, W::R, MUTED, &line);
            }
            Some(i) => {
                ui.word_pill(info_x, cy - 3.0, Language::English.word(i), open_word(i));
                ui.text_mid(
                    info_x,
                    cy + 22.0,
                    22.0,
                    12.0,
                    W::R,
                    MUTED,
                    &fill(
                        EN.keygen_word_of_the,
                        &[&i.to_string(), &(i + 1).to_string()],
                    ),
                );
            }
            None if compact => {
                let note = ui.fit(12.0, W::R, note, info_w);
                ui.text_mid(info_x, cy - 3.0, 26.0, 12.0, W::R, DIM, &note);
            }
            None => {
                ui.text_mid(info_x, cy + 22.0, 22.0, 12.0, W::R, DIM, note);
            }
        }
    };
    let bits = |pos: usize, face: u8| -> String {
        if pos < faces {
            format!("{:02b}", face - 1)
        } else {
            (if face >= 4 { "1" } else { "0" }).to_string()
        }
    };
    let at = rolls.len() / per;
    for slot in 0..last.min(at + 1) {
        let got = &rolls[(slot * per).min(rolls.len())..rolls.len().min((slot + 1) * per)];
        if got.is_empty() {
            break;
        }
        let cells: Vec<(String, String)> = (0..per)
            .map(|p| match got.get(p) {
                Some(&f) => (f.to_string(), bits(p, f)),
                None => (
                    "·".to_string(),
                    if p < faces { "··" } else { "·" }.to_string(),
                ),
            })
            .collect();
        let note = fill(
            EN.keygen_of_rolls,
            &[&got.len().to_string(), &per.to_string()],
        );
        row(ui, cy, slot, &cells, None, live.get(slot).copied(), &note);
        cy += 50.0 + info_y;
    }
    // The last word: rolled only as far as its high bits, which the key
    // keeps; the rest, shown as dots, are the checksum's.
    let from = last * per;
    let got = rolls.get(from..).unwrap_or(&[]);
    let kept = k.last_bits();
    let cells: Vec<(String, String)> = (0..kept.div_ceil(2))
        .map(|p| {
            let (at, width) = if p < faces {
                (2 * p, 2)
            } else {
                (2 * faces, 1)
            };
            let keep = kept.saturating_sub(at).min(width);
            match got.get(p) {
                Some(&f) => {
                    let all = bits(p, f);
                    (
                        f.to_string(),
                        format!("{}{}", &all[..keep], "·".repeat(width - keep)),
                    )
                }
                None => ("·".to_string(), "·".repeat(width)),
            }
        })
        .collect();
    let made = k.checksum_word();
    let sum = match made {
        Some(i) => format!("{:0width$b}", i & ((1 << checksum) - 1), width = checksum),
        None => "·".repeat(checksum),
    };
    row(
        ui,
        cy,
        last,
        &cells,
        Some(&sum),
        made,
        EN.keygen_the_checksum,
    );
    cy + 58.0 + info_y
}

/// Opens the BIP-39 list at word `i`.
fn open_word(i: u16) -> Action {
    Action::WordList(crate::wordlist::WordListAction::Open(0, Some(i)))
}

/// The entry just made, shown once so a mistyped one can be taken back.
fn last_entry(k: &KeyGen) -> Option<String> {
    match k.active()? {
        Source::Dice if k.by_die => k
            .coins
            .last()
            .map(|h| if h { EN.create_heads } else { EN.create_tails }.to_string()),
        Source::Dice => k.dice.last().map(|r| r.to_string()),
        Source::Coins => k
            .coins
            .last()
            .map(|h| if h { EN.create_heads } else { EN.create_tails }.to_string()),
        Source::Cards => k.cards.last().map(|i| {
            let (r, s) = faraday_entropy::card_parts(i);
            format!("{}{}", RANKS[usize::from(r)], SUITS[usize::from(s)])
        }),
        Source::Hex => k.hex.last().map(|c| c.to_string()),
        _ => None,
    }
}

fn device_rows(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let rows = [
        (EN.create_device_row, EN.create_device_os, TEXT),
        (EN.create_trust_row, EN.create_trust_device, WARN),
        (EN.keygen_result, EN.create_device_result, WARN),
    ];
    let mut cy = y;
    for (label, value, tone) in rows {
        ui.text(x, cy, 13.0, W::R, MUTED, label);
        ui.text(x + 140.0, cy, 13.0, W::S, tone, value);
        cy += 26.0;
        ui.rule(x, cy - 6.0, w, INNER);
    }
    cy + 6.0
}

fn stat(ui: &mut Ui, x: f32, y: f32, label: &str, value: &str, tone: Color) -> f32 {
    ui.text(x, y, 13.0, W::R, MUTED, label);
    if ui.compact && ui.measure(13.0, W::M, value) > 130.0 {
        ui.text(x, y + 20.0, 13.0, W::M, tone, value);
        return y + 46.0;
    }
    let vx = if ui.compact { 110.0 } else { 140.0 };
    ui.text(x + vx, y, 13.0, W::M, tone, value);
    y + 26.0
}

/// A stat row whose value is a pill (`docs/NEW-WALLET.md` §3.3), such as
/// the locked Key card's Passphrase: "Set" or "Not set", never bare
/// text, since bare text reads as the passphrase itself.
fn stat_pill(ui: &mut Ui, x: f32, y: f32, label: &str, value: &str) -> f32 {
    ui.text(x, y, 13.0, W::R, MUTED, label);
    let vx = if ui.compact { 110.0 } else { 140.0 };
    ui.pill(x + vx, y - 3.0, 22.0, value);
    y + 26.0
}

/// The heads, tails and longest-run rows a flip accumulator's stats make.
fn coin_check(coins: &CoinFlips, ui: &mut Ui, x: f32, y: f32) -> f32 {
    let mut cy = y;
    let st = coins.stats();
    cy = stat(ui, x, cy, EN.create_heads, &st.heads.to_string(), TEXT);
    cy = stat(ui, x, cy, EN.create_tails, &st.tails.to_string(), TEXT);
    let side = if st.run_heads {
        EN.create_heads
    } else {
        EN.create_tails
    };
    stat(
        ui,
        x,
        cy,
        EN.create_longest_run,
        &crate::strings::fill(
            EN.create_run_face,
            &[&st.longest_run.to_string(), &side.to_lowercase()],
        ),
        TEXT,
    )
}

/// SLIP-39's Check card: the counts, the cautions, and Roll again.
fn check(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = check_rows(k, k.source, ui, x, y, w);
    for c in k.caution_lines() {
        ui.text(x, cy, 13.0, W::R, WARN, c);
        cy += 22.0;
    }
    cy += 6.0;
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        &[(again_label(k), Style::Ghost, Action::KAgain)],
        Some((EN.keygen_continue, Action::KNext)),
    );
    cy - y
}

/// The source's word for starting the entries again.
fn again_label(k: &KeyGen) -> &'static str {
    match k.active().or(k.source) {
        Some(Source::Cards) => EN.create_sanity_again_draw,
        Some(Source::Camera) => EN.create_sanity_again_take,
        Some(Source::Device) => EN.keygen_start_again,
        _ => EN.create_sanity_again,
    }
}

/// The randomness check's counts for source `src`: rolls, faces,
/// chi-square and longest run, or the source's own. Returns where the
/// next row starts.
fn check_rows(k: &KeyGen, src: Option<Source>, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match src {
        Some(Source::Dice) if k.by_die => {
            cy = coin_check(&k.coins, ui, x, cy);
        }
        Some(Source::Dice) => {
            let st = k.dice.stats();
            let n = k.dice.len();
            cy = stat(ui, x, cy, EN.keygen_rolls, &n.to_string(), TEXT);
            let counts = st
                .counts
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{}:{c}", i + 1))
                .collect::<Vec<_>>()
                .join("  ");
            cy = stat(ui, x, cy, EN.keygen_faces, &counts, TEXT);
            if !k.procedure().direct() {
                let high = st.chi_square > faraday_entropy::DICE_CHI_SQUARE_LIMIT
                    && n >= faraday_entropy::DICE_SKEW_MIN;
                let judged = if high {
                    EN.create_stat_high
                } else {
                    EN.create_stat_normal
                };
                cy = stat(
                    ui,
                    x,
                    cy,
                    EN.create_chi_square,
                    &format!(
                        "{:.1} · {}",
                        st.chi_square,
                        judged.replacen("{}", &n.to_string(), 1)
                    ),
                    if high { WARN } else { TEXT },
                );
            }
            cy = stat(
                ui,
                x,
                cy,
                EN.create_longest_run,
                &crate::strings::fill(
                    EN.create_run_face,
                    &[&st.longest_run.to_string(), face_name(st.run_face)],
                ),
                TEXT,
            );
        }
        Some(Source::Coins) => {
            cy = coin_check(&k.coins, ui, x, cy);
        }
        Some(Source::Cards) => {
            let st = k.cards.stats();
            cy = stat(
                ui,
                x,
                cy,
                EN.create_cards_row,
                &k.cards.len().to_string(),
                TEXT,
            );
            let suits = SUITS
                .iter()
                .zip(st.suits)
                .map(|(s, c)| format!("{s} {c}"))
                .collect::<Vec<_>>()
                .join("   ");
            cy = stat(ui, x, cy, EN.keygen_suits, &suits, TEXT);
        }
        Some(Source::Hex) => {
            cy = stat(ui, x, cy, EN.keygen_digits, &k.hex.len().to_string(), TEXT);
        }
        Some(Source::Camera) => {
            cy = stat(
                ui,
                x,
                cy,
                EN.create_frames_row,
                &k.camera.len().to_string(),
                TEXT,
            );
            if let Some(st) = k.camera.last() {
                cy = stat(
                    ui,
                    x,
                    cy,
                    EN.create_distinct_row,
                    &st.distinct.to_string(),
                    TEXT,
                );
            }
        }
        Some(Source::Mix) => {
            for (i, (s, c)) in k.mix_list().iter().zip(k.mixed.commitments()).enumerate() {
                let hex: String = c[..8].iter().map(|b| format!("{b:02x}")).collect();
                cy = stat(
                    ui,
                    x,
                    cy,
                    &format!("{}. {}", i + 1, source_name(*s)),
                    &fill(EN.keygen_sha_256, &[&hex.to_string()]),
                    TEXT,
                );
            }
        }
        Some(Source::Device) => cy = device_rows(ui, x, cy, w),
        _ => {}
    }
    cy
}

/// One SLIP-39 share at a time, its words numbered, with the way to the
/// next.
/// The public seed the demo added in place of the key made, when it did
/// (F20): a line under the fingerprint, which is the public seed's.
fn handed_line(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    match (k.handed, k.fingerprint) {
        (Some(word), Some(fp)) => {
            let fp = fp_text(faraday_bip::keys::Fingerprint(fp));
            let line = fill(EN.keygen_added_public_seed, &[word, &fp]);
            ui.wrap(x, y, w, 13.0, W::S, WARN, &line) + 6.0
        }
        _ => 0.0,
    }
}

fn share_words(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let Some(ind) = k.share_indices(k.share_at) else {
        ui.text(x, cy, 13.0, W::R, MUTED, EN.keygen_not_made_yet);
        return 26.0;
    };
    if let Some(fp) = k.fingerprint {
        cy = stat(
            ui,
            x,
            cy,
            EN.keygen_fingerprint,
            &fp_text(faraday_bip::keys::Fingerprint(fp)),
            TEXT,
        );
    }
    cy += handed_line(k, ui, x, cy, w);
    cy = stat(
        ui,
        x,
        cy,
        EN.keygen_share,
        &fill(
            EN.keygen_of_needed,
            &[
                &(k.share_at + 1).to_string(),
                &k.shares.len().to_string(),
                &k.slip_m.to_string(),
            ],
        ),
        TEXT,
    );
    cy += 6.0;
    let cols = if ui.compact { 2 } else { 4 };
    let cellw = (w - (cols - 1) as f32 * 10.0) / cols as f32;
    for (i, &idx) in ind.iter().enumerate() {
        let cx = x + (i % cols) as f32 * (cellw + 10.0);
        let wy = cy + (i / cols) as f32 * 46.0;
        ui.fill(cx, wy, cellw, 38.0, 8.0, BG);
        ui.stroke(cx, wy, cellw, 38.0, 8.0, INNER);
        ui.text_mid(cx + 10.0, wy, 38.0, 11.0, W::R, DIM, &(i + 1).to_string());
        let shown = if k.shown {
            faraday_bip::slip39::word(idx)
        } else {
            "••••••"
        };
        ui.text_mid(cx + 36.0, wy, 38.0, 14.0, W::M, TEXT, shown);
    }
    cy += ind.len().div_ceil(cols) as f32 * 46.0 + 8.0;
    let mut bx = x;
    bx += ui.button(
        bx,
        cy,
        None,
        40.0,
        if k.shown {
            EN.keygen_hide_words
        } else {
            EN.keygen_show_words
        },
        Style::Secondary,
        Action::KShow,
    ) + 8.0;
    if k.share_at > 0 {
        bx += ui.button(
            bx,
            cy,
            None,
            40.0,
            EN.keygen_previous_share,
            Style::Secondary,
            Action::KShare(-1),
        ) + 8.0;
    }
    // The next share, or past the last one, the next step: pinned on a
    // small panel.
    if k.share_at + 1 < k.shares.len() {
        if ui.pinning {
            ui.pin = Some((
                EN.keygen_next_share.to_string(),
                Style::Primary,
                Action::KShare(1),
            ));
        } else {
            ui.button(
                bx,
                cy,
                None,
                40.0,
                EN.keygen_next_share,
                Style::Primary,
                Action::KShare(1),
            );
        }
    } else {
        let _ = next_button(ui, x, cy, w, EN.keygen_i_wrote_them_all, Action::KNext);
    }
    cy + 48.0 - y
}

fn words(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if k.slip39 {
        return share_words(k, ui, x, y, w);
    }
    let mut cy = y;
    let Some(m) = k.mnemonic.as_ref() else {
        ui.text(x, cy, 13.0, W::R, MUTED, EN.keygen_not_made_yet);
        return 26.0;
    };
    if let Some(fp) = k.fingerprint {
        cy = stat(
            ui,
            x,
            cy,
            EN.keygen_fingerprint,
            &fp_text(faraday_bip::keys::Fingerprint(fp)),
            TEXT,
        );
    }
    cy += handed_line(k, ui, x, cy, w);
    cy = stat(
        ui,
        x,
        cy,
        EN.keygen_source,
        &summary(k, kstep::SOURCE),
        TEXT,
    );
    cy += 6.0;
    cy += word_grid(ui, m, k.shown, x, cy, w);
    if let Some(n) = &k.note {
        ui.text(x, cy, 13.0, W::R, ERR, n);
        cy += 24.0;
    }
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        &[(
            if k.shown {
                EN.keygen_hide_words
            } else {
                EN.keygen_show_words
            },
            Style::Secondary,
            Action::KShow,
        )],
        Some((EN.keygen_i_wrote_them, Action::KNext)),
    );
    cy - y
}

/// OpenSigner's backup quiz: each word asked in a random order, picked
/// from four.
fn quiz(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::quiz::QuizState;
    let mut cy = y;
    let Some(q) = k.quiz.as_ref() else {
        ui.text(x, cy, 13.0, W::R, DIM, EN.keygen_show_the_words_first);
        return 26.0;
    };
    let (done, n) = q.progress();
    if q.state() == QuizState::Passed || k.skipped {
        if let Some(fp) = k.fingerprint {
            cy = stat(
                ui,
                x,
                cy,
                EN.keygen_fingerprint,
                &fp_text(faraday_bip::keys::Fingerprint(fp)),
                TEXT,
            );
        }
        cy += handed_line(k, ui, x, cy, w);
        let (said, tone) = if q.state() == QuizState::Passed && k.slip39 {
            (
                fill(EN.keygen_all_shares_right, &[&k.shares.len().to_string()]),
                OK,
            )
        } else if q.state() == QuizState::Passed {
            (fill(EN.keygen_all_words_right, &[&n.to_string()]), OK)
        } else {
            (EN.keygen_skipped_the.to_string(), WARN)
        };
        cy = stat(ui, x, cy, EN.keygen_quiz, &said, tone);
        if let Some(e) = &k.note {
            ui.text(x, cy, 13.0, W::R, ERR, e);
            cy += 24.0;
        }
        cy += 6.0;
        // A BIP-39 key was added at Lock in: the quiz's last button only
        // ends the flow.
        let label = if !k.slip39 {
            EN.keygen_done
        } else if k.slot.is_some() {
            EN.keygen_use_this_key
        } else {
            EN.keygen_add_key
        };
        let drawn = next_button(ui, x, cy, w, label, Action::KAdd);
        return cy + (if drawn { 48.0 } else { 8.0 }) - y;
    }
    let progress = if k.slip39 {
        fill(
            EN.keygen_share_of_of,
            &[
                &(k.quiz_share + 1).to_string(),
                &k.shares.len().to_string(),
                &done.to_string(),
                &n.to_string(),
            ],
        )
    } else {
        fill(EN.keygen_of_right, &[&done.to_string(), &n.to_string()])
    };
    ui.text(x, cy, 13.0, W::R, MUTED, &progress);
    cy += 26.0;
    ui.text(
        x,
        cy,
        18.0,
        W::S,
        TEXT,
        &fill(EN.keygen_which_is_word, &[&(q.position() + 1).to_string()]),
    );
    cy += 40.0;
    let wrong = q.wrong_slot();
    let per = if ui.compact { 2 } else { 4 };
    let bw = ((w - (per - 1) as f32 * 8.0) / per as f32).min(170.0);
    for (i, &idx) in q.choices().iter().enumerate() {
        let style = if wrong == Some(i) {
            Style::Disabled
        } else {
            Style::Secondary
        };
        ui.button(
            x + (i % per) as f32 * (bw + 8.0),
            cy + (i / per) as f32 * 56.0,
            Some(bw),
            48.0,
            if k.slip39 {
                faraday_bip::slip39::word(idx)
            } else {
                Language::English.word(idx)
            },
            style,
            Action::KQuiz(i as u8),
        );
    }
    cy += q.choices().len().div_ceil(per) as f32 * 56.0 + 4.0;
    if q.state() == QuizState::Wrong {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, EN.keygen_not_that_one) + 10.0;
        let aw = ui.button(
            x,
            cy,
            None,
            40.0,
            EN.keygen_ask_it_again,
            Style::Secondary,
            Action::KQuizRetry,
        );
        ui.button(
            x + aw + 8.0,
            cy,
            None,
            40.0,
            EN.keygen_show_the_words,
            Style::Ghost,
            Action::KStep(kstep::WORDS),
        );
        cy += 52.0;
    }
    if k.skip_ask {
        cy += ui.wrap(x, cy, w, 13.0, W::R, WARN, EN.keygen_without_the) + 10.0;
        ui.button(
            x,
            cy,
            None,
            40.0,
            if k.slip39 {
                EN.keygen_skip_and_add
            } else {
                EN.keygen_skip_the_quiz
            },
            Style::Secondary,
            Action::KSkip,
        );
    } else {
        ui.button(
            x,
            cy,
            None,
            40.0,
            EN.keygen_skip_the_quiz,
            Style::Ghost,
            Action::KSkip,
        );
    }
    cy += 48.0;
    if ui.compact {
        return cy - y;
    }
    ui.text(
        x,
        cy,
        12.0,
        W::R,
        DIM,
        if k.slip39 {
            EN.keygen_type_1_to_4_to
        } else {
            EN.keygen_type_1_to_4
        },
    );
    cy + 24.0 - y
}

/// A face's name in a run, as OpenSigner writes it: "ones" to "sixes".
fn face_name(face: u8) -> &'static str {
    match face {
        1 => EN.create_face_1,
        2 => EN.create_face_2,
        3 => EN.create_face_3,
        4 => EN.create_face_4,
        5 => EN.create_face_5,
        _ => EN.create_face_6,
    }
}
