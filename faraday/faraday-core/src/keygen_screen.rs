//! The New key screen: the step-card column over [`crate::keygen`]. The
//! options and their names are OpenSigner's; this file only lays them out.

use opensigner_core::strings::EN;
use osk_bip::bip39::Language;
use osk_entropy::{CameraNoise, CardDraws, CoinFlips, DiceProcedure, RANKS, SUITS};

use crate::keygen::{KeyGen, MIX_SOURCES, SOURCE_ROWS, Source, kstep, procedure_name, source_name};
use crate::screens::{next_button, section_label};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday, flow, guide};

const TITLES: [&str; kstep::COUNT] = ["Length", "Randomness", "Entries", "Check", "Words", "Quiz"];

pub(crate) fn draw(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(k) = app.keygen.as_ref() else {
        return;
    };
    let cards: Vec<flow::Card> = (0..kstep::COUNT as u8)
        .map(|s| flow::Card {
            title: if s == kstep::ENTER {
                entries_title(k)
            } else {
                TITLES[usize::from(s)].to_string()
            },
            summary: summary(k, s),
            mono: s == kstep::QUIZ && k.fingerprint.is_some(),
            done: k.done[usize::from(s)],
            open: k.open == Some(s),
            toggle: Action::KStep(s),
            guide: Some(guide::keygen(s, k.active(), k.slip39, k.by_die)),
        })
        .collect();
    let back_label = match k.back {
        crate::Screen::Create => "Create a wallet",
        crate::Screen::Entry => "Add a key",
        crate::Screen::Catalog => "Tools",
        _ => "Back",
    };
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + 40.0,
        w: (cw - 80.0).min(900.0),
        h,
        back: Some((back_label, Action::Nav(k.back))),
        heading: "New key",
        guided: app.guided,
        switch: true,
        note: None,
    };
    let scroll = k.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.keygen.as_ref() {
                Some(k) => card_body(k, ui, i as u8, x, y, w),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(k) = app.keygen.as_mut() {
        k.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
}

fn entries_title(k: &KeyGen) -> String {
    let what = match k.active() {
        Some(Source::Dice) => "Rolls",
        Some(Source::Coins) => "Flips",
        Some(Source::Cards) => "Cards",
        Some(Source::Hex) => "Digits",
        Some(Source::Camera) => "Pictures",
        Some(Source::Device) => "Generator",
        _ => "Entries",
    };
    if k.source == Some(Source::Mix) {
        let n = k.mix_list().len();
        format!("{what} · source {} of {n}", (k.mix_at + 1).min(n))
    } else {
        what.to_string()
    }
}

/// What a source asks for at this length, as its row's second line.
fn asks(k: &KeyGen, s: Source) -> String {
    let st = k.strength();
    match s {
        Source::Dice => format!("{} rolls of a six-sided die", k.procedure().needed(st)),
        Source::Coins => format!(
            "{} flips, or rolls of a die read as flips",
            CoinFlips::needed(st)
        ),
        Source::Cards => format!("{} cards from a shuffled deck", CardDraws::needed(st)),
        Source::Hex => format!("{} digits made elsewhere", st.hex_digits()),
        Source::Camera => format!("{} pictures", CameraNoise::needed(st)),
        Source::Mix => "Two or more of these, combined".to_string(),
        Source::Device => format!(
            "{} · {}: {} · getrandom(2)",
            EN.create_device_os, EN.create_trust_row, EN.create_trust_device
        ),
        Source::SeedXor => String::new(),
    }
}

fn summary(k: &KeyGen, s: u8) -> String {
    match s {
        kstep::LENGTH if k.slip39 => format!(
            "SLIP-39 · {} of {} shares · {} words each",
            k.slip_m, k.slip_n, k.words
        ),
        kstep::LENGTH => format!("{} words", k.words),
        kstep::SOURCE => match k.source {
            Some(Source::Dice) => format!("Dice · {}", procedure_name(k.procedure)),
            Some(Source::Coins) if k.by_die => {
                format!("{} · read from a die", source_name(Source::Coins))
            }
            Some(Source::Mix) => k
                .mix_list()
                .iter()
                .map(|s| source_name(*s))
                .collect::<Vec<_>>()
                .join(" + "),
            Some(s) => source_name(s).to_string(),
            None => "Not chosen".to_string(),
        },
        kstep::ENTER => {
            if k.done[usize::from(kstep::ENTER)] {
                "Complete".to_string()
            } else {
                let (have, need) = k.progress();
                format!("{have} of {need}")
            }
        }
        kstep::CHECK => match k.source {
            Some(Source::Device) => EN.create_trusts_device.to_string(),
            Some(Source::Mix) => format!("{} sources combined", k.mixed.len()),
            _ if !k.done[usize::from(kstep::ENTER)] => "Not yet".to_string(),
            _ => match k.caution_lines().len() {
                0 => "No cautions".to_string(),
                1 => "1 caution".to_string(),
                n => format!("{n} cautions"),
            },
        },
        kstep::WORDS if k.slip39 => match k.shares.len() {
            0 => "Not made yet".to_string(),
            n => format!("{n} shares"),
        },
        kstep::WORDS => match k.mnemonic.as_ref() {
            Some(m) => format!("{} words", m.indices().len()),
            None => "Not made yet".to_string(),
        },
        _ => match (k.fingerprint, k.done[usize::from(kstep::QUIZ)]) {
            (Some(fp), true) => fp_text(osk_bip::keys::Fingerprint(fp)),
            _ => match k.quiz.as_ref().map(|q| q.progress()) {
                Some((d, n)) => format!("{d} of {n}"),
                None => "Not yet".to_string(),
            },
        },
    }
}

fn card_body(k: &KeyGen, ui: &mut Ui, s: u8, x: f32, y: f32, w: f32) -> f32 {
    match s {
        kstep::LENGTH => length(k, ui, x, y),
        kstep::SOURCE => sources(k, ui, x, y, w),
        kstep::ENTER => entries(k, ui, x, y, w),
        kstep::CHECK => check(k, ui, x, y, w),
        kstep::WORDS => words(k, ui, x, y, w),
        _ => quiz(k, ui, x, y, w),
    }
}

/// What SLIP-39 shares reveal and what they do not, stated beside the
/// choice so the person can weigh it.
const SLIP39_FACTS: [&str; 7] = [
    "Fewer shares than the threshold say nothing about the seed: with them, every seed stays as likely as every other. No entropy is lost to a holder of too few shares.",
    "Each share states in the clear its backup's identifier, the shares needed, how many were dealt and its own number: one share tells its holder how many others to look for.",
    "The threshold of shares rebuilds the whole seed on one device, and that device holds all of it while it signs.",
    "The split's own randomness comes from the device that deals it, here this device's system generator. A dealer with weak randomness gives the seed away in its shares.",
    "A share cannot be checked on its own against the seed; a wrong or mixed-up share shows only when the threshold is put together, by a 4-byte digest.",
    "Shares restore only in software that reads SLIP-39, such as Trezor's and OpenSigner's; a wallet that reads only BIP-39 words cannot.",
    "A share is 20 words for a 128-bit seed, the strength of 12 BIP-39 words, or 33 words for 256 bits.",
];

fn length(k: &KeyGen, ui: &mut Ui, x: f32, y: f32) -> f32 {
    let mut cy = y;
    // Words or shares.
    if !k.only_24 {
        let mut bx = x;
        for (label, on) in [("BIP-39 words", false), ("SLIP-39 shares", true)] {
            let style = if k.slip39 == on {
                Style::Primary
            } else {
                Style::Secondary
            };
            bx += ui.button(bx, cy, None, 36.0, label, style, Action::KForm(on)) + 8.0;
        }
        cy += 50.0;
    }
    let used = length_counts(k, ui, x, cy);
    cy += used;
    if k.slip39 {
        for (label, value, minus, plus) in [
            (
                "Shares needed",
                k.slip_m,
                Action::KSlipM(-1),
                Action::KSlipM(1),
            ),
            (
                "Shares dealt",
                k.slip_n,
                Action::KSlipN(-1),
                Action::KSlipN(1),
            ),
        ] {
            ui.text_mid(x, cy, 40.0, 13.0, W::R, MUTED, label);
            ui.button(
                x + 180.0,
                cy,
                Some(40.0),
                40.0,
                "-",
                Style::Secondary,
                minus,
            );
            ui.text_mid(x + 236.0, cy, 40.0, 18.0, W::S, TEXT, &value.to_string());
            ui.button(x + 270.0, cy, Some(40.0), 40.0, "+", Style::Secondary, plus);
            cy += 50.0;
        }
        ui.text(
            x,
            cy,
            13.0,
            W::S,
            TEXT,
            &format!(
                "Any {} of the {} shares restore the key",
                k.slip_m, k.slip_n
            ),
        );
        cy += 30.0;
        ui.text(x, cy, 13.0, W::S, MUTED, "What shares reveal");
        cy += 24.0;
        for f in SLIP39_FACTS {
            ui.fill(x + 2.0, cy + 7.0, 4.0, 4.0, 2.0, MUTED);
            cy += ui.wrap(x + 16.0, cy, 760.0, 13.0, W::R, TEXT, f) + 8.0;
        }
        cy += 4.0;
        next_button(ui, x, cy, 760.0, "Continue", Action::KWords(k.words as u8));
        cy += 52.0;
    }
    cy - y
}

fn length_counts(k: &KeyGen, ui: &mut Ui, x: f32, y: f32) -> f32 {
    let mut bx = x;
    for &n in k.counts() {
        let allowed = !k.only_24 || n == 24;
        let style = if !allowed {
            Style::Disabled
        } else if usize::from(n) == k.words && (k.slip39 || k.done[usize::from(kstep::LENGTH)]) {
            Style::Primary
        } else {
            Style::Secondary
        };
        bx += ui.button(
            bx,
            y,
            Some(104.0),
            40.0,
            &format!("{n} words"),
            style,
            if k.slip39 {
                Action::KSlipWords(n)
            } else {
                Action::KWords(n)
            },
        ) + 8.0;
    }
    let mut used = 52.0;
    if k.only_24 {
        ui.text(
            x,
            y + used,
            13.0,
            W::R,
            MUTED,
            "A threshold wallet's keys are 24 words",
        );
        used += 26.0;
    }
    used
}

/// One choosable row: a radio mark, a name and a second line.
struct Row<'a> {
    on: bool,
    name: &'a str,
    line: &'a str,
    tone: Color,
    action: Action,
}

fn row(ui: &mut Ui, x: f32, y: f32, w: f32, r: Row) {
    let Row {
        on,
        name,
        line,
        tone,
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
    ui.text(x + 42.0, y + 8.0, 14.0, W::S, TEXT, name);
    ui.text(x + 42.0, y + 29.0, 12.0, W::R, tone, line);
    ui.hit(x, y, w, 52.0, a);
}

use osk_ui::Color;

fn sources(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    section_label(ui, x, cy, "Made by you");
    let lw = ui.measure(14.0, W::S, "Made by you");
    ui.chip(
        x + lw + 12.0,
        cy - 4.0,
        "Recommended",
        OK,
        OK.with_alpha(30),
    );
    cy += 28.0;
    for (i, &s) in SOURCE_ROWS.iter().enumerate() {
        if s == Source::Device {
            cy += 8.0;
            section_label(ui, x, cy, "Made by this device");
            cy += 28.0;
        }
        let on = k.source == Some(s);
        let tone = if s == Source::Device { WARN } else { MUTED };
        let line = asks(k, s);
        row(
            ui,
            x,
            cy,
            w,
            Row {
                on,
                name: source_name(s),
                line: &line,
                tone,
                action: Action::KSource(i as u8),
            },
        );
        cy += 60.0;
        // The dice's procedures and the mix's sources open under their row.
        if on && s == Source::Dice {
            for (j, &p) in DiceProcedure::ALL.iter().enumerate() {
                // Choosing words directly makes BIP-39 words, not a
                // SLIP-39 secret.
                if k.slip39 && p.direct() {
                    continue;
                }
                let n = p.needed(k.strength());
                let line = if p.direct() {
                    EN.dice_procedure_rolls_words
                        .replacen("{}", &n.to_string(), 1)
                } else {
                    EN.dice_procedure_rolls.replacen("{}", &n.to_string(), 1)
                };
                row(
                    ui,
                    x + 40.0,
                    cy,
                    w - 40.0,
                    Row {
                        on: k.procedure == p,
                        name: procedure_name(p),
                        line: &line,
                        tone: MUTED,
                        action: Action::KProc(j as u8),
                    },
                );
                cy += 60.0;
            }
        }
        if on && s == Source::Mix {
            for (j, &m) in MIX_SOURCES.iter().enumerate() {
                let chosen = k.mix[j];
                ui.checkbox(x + 44.0, cy + 7.0, chosen, true);
                ui.text(x + 74.0, cy + 6.0, 14.0, W::R, TEXT, source_name(m));
                let nw = ui.measure(14.0, W::R, source_name(m));
                ui.text(
                    x + 86.0 + nw,
                    cy + 7.0,
                    12.0,
                    W::R,
                    if m == Source::Device { WARN } else { MUTED },
                    &asks(k, m),
                );
                ui.hit(x + 40.0, cy, w - 40.0, 32.0, Action::KMix(j as u8));
                cy += 34.0;
            }
            cy += 8.0;
        }
    }
    if let Some(n) = &k.note {
        ui.text(x, cy + 10.0, 13.0, W::R, ERR, n);
    }
    next_button(ui, x, cy, w, "Continue", Action::KNext);
    cy + 48.0 - y
}

fn entries(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let (have, need) = k.progress();
    let active = k.active();
    // How far along, as a count and a bar.
    if active != Some(Source::Device) {
        let unit = match active {
            Some(Source::Dice) => "rolls",
            Some(Source::Coins) => "flips",
            Some(Source::Cards) => "cards",
            Some(Source::Hex) => "digits",
            _ => "pictures",
        };
        ui.text(x, cy, 15.0, W::S, TEXT, &format!("{have} of {need} {unit}"));
        if let Some(last) = last_entry(k) {
            ui.text_right(
                x + w,
                cy - 6.0,
                30.0,
                13.0,
                W::M,
                MUTED,
                &format!("Last: {last}"),
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
            if k.procedure().direct() && have >= need {
                cy = last_words(k, ui, x, cy, w);
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
            if k.procedure().direct() && have >= need {
                cy = last_words(k, ui, x, cy, w);
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
                    .and_then(|r| osk_entropy::card_index(r, s as u8))
                    .is_some_and(|i| k.cards.drawn(i));
                let style = if k.rank.is_none() || taken {
                    Style::Disabled
                } else {
                    Style::Secondary
                };
                ui.button(
                    x + s as f32 * 72.0,
                    cy,
                    Some(64.0),
                    44.0,
                    &ch.to_string(),
                    style,
                    Action::KSuit(s as u8),
                );
            }
            ui.text_mid(
                x + 4.0 * 72.0 + 8.0,
                cy,
                44.0,
                12.0,
                W::R,
                MUTED,
                &format!(
                    "Deck {} · {} drawn from it",
                    k.cards.deck(),
                    k.cards.in_deck()
                ),
            );
            cy += 56.0;
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
                Some(st) => format!("{} luma values in the last picture", st.distinct),
                None if k.frame.is_some() => "Camera on".to_string(),
                None => "Waiting for the camera".to_string(),
            };
            ui.text_mid(x + bw + 16.0, cy, 48.0, 13.0, W::R, MUTED, &state);
            cy += 60.0;
        }
        Some(Source::Device) => {
            cy = device_rows(ui, x, cy, w);
        }
        _ => {}
    }
    // Cautions as they appear, and what the last press did.
    for c in k.caution_lines() {
        ui.text(x, cy, 13.0, W::R, WARN, c);
        cy += 22.0;
    }
    if let Some(n) = &k.note {
        ui.text(x, cy, 13.0, W::R, ERR, n);
        cy += 22.0;
    }
    cy += 6.0;
    if !matches!(active, Some(Source::Device)) {
        let uw = ui.button(x, cy, None, 40.0, "Undo", Style::Ghost, Action::KUndo);
        ui.button(
            x + uw + 6.0,
            cy,
            None,
            40.0,
            "Clear",
            Style::Ghost,
            Action::KClear,
        );
    }
    let last_of_mix = k.source != Some(Source::Mix) || k.mix_at + 1 >= k.mix_list().len();
    let label = if last_of_mix {
        "Make the words"
    } else {
        "Next source"
    };
    next_button(ui, x, cy, w, label, Action::KNext);
    cy += 52.0;
    let hint = match active {
        Some(Source::Dice | Source::Coins) if k.typing => {
            "Typing goes into the box · Enter takes it · Enter again continues"
        }
        Some(Source::Dice) => "Type 1 to 6 · Backspace takes one back · Enter continues",
        Some(Source::Coins) if k.by_die => {
            "Type 1 to 6 · Backspace takes one back · Enter continues"
        }
        Some(Source::Coins) => "Type H or T · Backspace takes one back · Enter continues",
        Some(Source::Cards) => "Type the rank (A 2–9 T J Q K), then the suit (S H D C)",
        Some(Source::Hex) => "Type 0–9 and a–f · Backspace takes one back · Enter continues",
        Some(Source::Camera) => "Space takes a picture · Enter continues",
        _ => "Enter continues",
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
        for (label, on) in [("Coin", false), ("Die", true)] {
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
    }
    for (label, on) in [("Buttons", false), ("Typed", true)] {
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
            "Each roll is one flip: 1, 2, 3 are 0 (tails); 4, 5, 6 are 1 (heads). The key is the one these flips make"
        } else {
            "Heads is 1, tails is 0"
        };
        cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, line) + 10.0;
    }
    cy
}

/// The box a string of entries is typed into, and the button that takes
/// it.
fn typed_box(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let take = "Take these";
    let tw = ui.measure(14.0, W::S, take) + 32.0;
    let bw = w - tw - 8.0;
    ui.fill(x, y, bw, 40.0, 8.0, BG);
    ui.stroke(x, y, bw, 40.0, 8.0, ACCENT);
    let shown = tail_fit(ui, &k.typed, bw - 30.0);
    ui.text_mid(x + 12.0, y, 40.0, 14.0, W::M, TEXT, &shown);
    let cx = x + 13.0 + ui.measure(14.0, W::M, &shown);
    ui.fill(cx, y + 11.0, 2.0, 18.0, 1.0, ACCENT);
    ui.button(
        x + bw + 8.0,
        y,
        Some(tw),
        40.0,
        take,
        if k.typed.is_empty() {
            Style::Disabled
        } else {
            Style::Primary
        },
        Action::KTake,
    );
    let what = match k.active() {
        Some(Source::Coins) if !k.by_die => "H or 1 for heads, T or 0 for tails",
        _ => "Faces 1 to 6, as rolled",
    };
    let count = k.typed.chars().count();
    ui.text(
        x,
        y + 48.0,
        12.0,
        W::R,
        DIM,
        &format!("{what} · {count} typed"),
    );
    y + 76.0
}

/// The end of `s` that fits `max`, with an ellipsis before it when cut.
fn tail_fit(ui: &Ui, s: &str, max: f32) -> String {
    if ui.measure(14.0, W::M, s) <= max {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut from = 0;
    while from < chars.len() {
        let t: String = std::iter::once('…')
            .chain(chars[from..].iter().copied())
            .collect();
        if ui.measure(14.0, W::M, &t) <= max {
            return t;
        }
        from += 1;
    }
    String::new()
}

/// The words the entries name, as they come in: each word's place, the
/// flips or rolls that name it, its number in the BIP-39 list and the
/// word, with a link to it in the list. The last place shows no word
/// until the words are made, because its last bits are the checksum.
fn reveal(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y + 4.0;
    section_label(ui, x, cy, "Words so far");
    cy += 28.0;
    let live = k.live_words();
    let coins = k.source == Some(Source::Coins);
    let flips: zeroize::Zeroizing<Vec<u8>> =
        zeroize::Zeroizing::new(k.coins.flips().map(u8::from).collect());
    let rolls = k.dice.rolls();
    let per = if coins {
        11
    } else {
        osk_entropy::DICE_WORD_ROLLS
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
    let (cx, ix, wx) = (
        x + num_w + 14.0,
        x + num_w + code_w + 34.0,
        x + num_w + code_w + index_w + 54.0,
    );
    let link = "In the list";
    let lw = ui.measure(13.0, W::S, link) + 32.0;
    let row = |ui: &mut Ui, cy: f32, slot: usize, code: &str, index: Option<u16>, note: &str| {
        let n = (slot + 1).to_string();
        let nw = ui.measure(13.0, W::M, &n);
        ui.text_mid(x + num_w - nw, cy, 30.0, 13.0, W::M, DIM, &n);
        ui.text_mid(cx, cy, 30.0, 13.0, W::M, MUTED, code);
        match index {
            Some(i) => {
                let num = (i + 1).to_string();
                let tw = ui.measure(13.0, W::M, &num);
                ui.text_mid(ix + index_w - tw, cy, 30.0, 13.0, W::M, MUTED, &num);
                ui.text_mid(wx, cy, 30.0, 14.0, W::M, TEXT, Language::English.word(i));
                ui.button(
                    x + w - lw,
                    cy + 2.0,
                    Some(lw),
                    26.0,
                    link,
                    Style::Ghost,
                    Action::WordList(crate::wordlist::WordListAction::Open(0, Some(i))),
                );
            }
            None => {
                ui.text_mid(ix, cy, 30.0, 12.0, W::R, DIM, note);
            }
        }
    };
    let last = k.words - 1;
    for (slot, &i) in live.iter().enumerate() {
        row(ui, cy, slot, &code_of(slot, per), Some(i), "");
        cy += 30.0;
    }
    // The word being entered, when it is not the last.
    let at = live.len();
    let part = entered.saturating_sub(at * per);
    if at < last && part > 0 {
        let note = if coins {
            format!("{part} of 11 bits")
        } else {
            format!("{part} of {per} rolls")
        };
        row(ui, cy, at, &code_of(at, part.min(per)), None, &note);
        cy += 30.0;
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
    row(
        ui,
        cy,
        last,
        &code,
        made,
        "From the checksum, once the rest is in",
    );
    cy += 30.0;
    if coins && made.is_some() {
        ui.text(
            x,
            cy,
            12.0,
            W::R,
            DIM,
            &format!(
                "The last {checksum} bits of word {} are the checksum",
                last + 1
            ),
        );
        cy += 22.0;
    }
    cy + 8.0
}

/// The entry just made, shown once so a mistyped one can be taken back.
fn last_entry(k: &KeyGen) -> Option<String> {
    match k.active()? {
        Source::Dice => k.dice.last().map(|r| r.to_string()),
        Source::Coins => k
            .coins
            .last()
            .map(|h| if h { EN.create_heads } else { EN.create_tails }.to_string()),
        Source::Cards => k.cards.last().map(|i| {
            let (r, s) = osk_entropy::card_parts(i);
            format!("{}{}", RANKS[usize::from(r)], SUITS[usize::from(s)])
        }),
        Source::Hex => k.hex.last().map(|c| c.to_string()),
        _ => None,
    }
}

fn last_words(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    section_label(ui, x, cy, EN.create_last_word_title);
    cy += 28.0;
    let cands = k.last_word_candidates();
    let cell = 104.0;
    let cols = ((w + 6.0) / (cell + 6.0)).floor().max(1.0) as usize;
    for (i, &idx) in cands.iter().enumerate() {
        let on = k.last_word == Some(idx);
        ui.button(
            x + (i % cols) as f32 * (cell + 6.0),
            cy + (i / cols) as f32 * 40.0,
            Some(cell),
            34.0,
            Language::English.word(idx),
            if on { Style::Primary } else { Style::Secondary },
            Action::KLast(idx),
        );
    }
    cy + cands.len().div_ceil(cols) as f32 * 40.0 + 8.0
}

fn device_rows(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let rows = [
        (EN.create_device_row, EN.create_device_os, TEXT),
        (EN.create_trust_row, EN.create_trust_device, WARN),
        ("Result", EN.create_device_result, WARN),
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
    ui.text(x + 140.0, y, 13.0, W::M, tone, value);
    y + 26.0
}

fn check(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match k.source {
        Some(Source::Dice) => {
            let st = k.dice.stats();
            let n = k.dice.len();
            cy = stat(ui, x, cy, "Rolls", &n.to_string(), TEXT);
            let counts = st
                .counts
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{}:{c}", i + 1))
                .collect::<Vec<_>>()
                .join("  ");
            cy = stat(ui, x, cy, "Faces", &counts, TEXT);
            if !k.procedure().direct() {
                let high = st.chi_square > osk_entropy::DICE_CHI_SQUARE_LIMIT
                    && n >= osk_entropy::DICE_SKEW_MIN;
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
                &opensigner_core::strings::fill(
                    EN.create_run_face,
                    &[&st.longest_run.to_string(), face_name(st.run_face)],
                ),
                TEXT,
            );
        }
        Some(Source::Coins) => {
            let st = k.coins.stats();
            cy = stat(ui, x, cy, EN.create_heads, &st.heads.to_string(), TEXT);
            cy = stat(ui, x, cy, EN.create_tails, &st.tails.to_string(), TEXT);
            let side = if st.run_heads {
                EN.create_heads
            } else {
                EN.create_tails
            };
            cy = stat(
                ui,
                x,
                cy,
                EN.create_longest_run,
                &opensigner_core::strings::fill(
                    EN.create_run_face,
                    &[&st.longest_run.to_string(), &side.to_lowercase()],
                ),
                TEXT,
            );
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
            cy = stat(ui, x, cy, "Suits", &suits, TEXT);
        }
        Some(Source::Hex) => {
            cy = stat(ui, x, cy, "Digits", &k.hex.len().to_string(), TEXT);
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
                    &format!("SHA-256 {hex}…"),
                    TEXT,
                );
            }
        }
        Some(Source::Device) => cy = device_rows(ui, x, cy, w),
        _ => {}
    }
    for c in k.caution_lines() {
        ui.text(x, cy, 13.0, W::R, WARN, c);
        cy += 22.0;
    }
    cy += 6.0;
    let again = match k.source {
        Some(Source::Cards) => EN.create_sanity_again_draw,
        Some(Source::Camera) => EN.create_sanity_again_take,
        Some(Source::Device) => "Start again",
        _ => EN.create_sanity_again,
    };
    ui.button(x, cy, None, 40.0, again, Style::Ghost, Action::KAgain);
    next_button(ui, x, cy, w, "Continue", Action::KNext);
    cy + 48.0 - y
}

/// One SLIP-39 share at a time, its words numbered, with the way to the
/// next.
fn share_words(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let Some(ind) = k.share_indices(k.share_at) else {
        ui.text(x, cy, 13.0, W::R, MUTED, "Not made yet");
        return 26.0;
    };
    if let Some(fp) = k.fingerprint {
        cy = stat(
            ui,
            x,
            cy,
            "Fingerprint",
            &fp_text(osk_bip::keys::Fingerprint(fp)),
            TEXT,
        );
    }
    cy = stat(
        ui,
        x,
        cy,
        "Share",
        &format!(
            "{} of {} · {} needed",
            k.share_at + 1,
            k.shares.len(),
            k.slip_m
        ),
        TEXT,
    );
    cy += 6.0;
    let cols = 4;
    let cellw = (w - 3.0 * 10.0) / cols as f32;
    for (i, &idx) in ind.iter().enumerate() {
        let cx = x + (i % cols) as f32 * (cellw + 10.0);
        let wy = cy + (i / cols) as f32 * 46.0;
        ui.fill(cx, wy, cellw, 38.0, 8.0, BG);
        ui.stroke(cx, wy, cellw, 38.0, 8.0, INNER);
        ui.text_mid(cx + 10.0, wy, 38.0, 11.0, W::R, DIM, &(i + 1).to_string());
        let shown = if k.shown {
            osk_bip::slip39::word(idx)
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
        if k.shown { "Hide words" } else { "Show words" },
        Style::Secondary,
        Action::KShow,
    ) + 8.0;
    if k.share_at > 0 {
        bx += ui.button(
            bx,
            cy,
            None,
            40.0,
            "Previous share",
            Style::Secondary,
            Action::KShare(-1),
        ) + 8.0;
    }
    if k.share_at + 1 < k.shares.len() {
        ui.button(
            bx,
            cy,
            None,
            40.0,
            "Next share",
            Style::Primary,
            Action::KShare(1),
        );
    } else {
        next_button(ui, x, cy, w, "I wrote them all down", Action::KNext);
    }
    cy + 48.0 - y
}

fn words(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if k.slip39 {
        return share_words(k, ui, x, y, w);
    }
    let mut cy = y;
    let Some(m) = k.mnemonic.as_ref() else {
        ui.text(x, cy, 13.0, W::R, MUTED, "Not made yet");
        return 26.0;
    };
    if let Some(fp) = k.fingerprint {
        cy = stat(
            ui,
            x,
            cy,
            "Fingerprint",
            &fp_text(osk_bip::keys::Fingerprint(fp)),
            TEXT,
        );
    }
    cy = stat(ui, x, cy, "Source", &summary(k, kstep::SOURCE), TEXT);
    cy += 6.0;
    let cols = 4;
    let cellw = (w - 3.0 * 10.0) / cols as f32;
    for (i, &idx) in m.indices().iter().enumerate() {
        let cx = x + (i % cols) as f32 * (cellw + 10.0);
        let wy = cy + (i / cols) as f32 * 46.0;
        ui.fill(cx, wy, cellw, 38.0, 8.0, BG);
        ui.stroke(cx, wy, cellw, 38.0, 8.0, INNER);
        ui.text_mid(cx + 10.0, wy, 38.0, 11.0, W::R, DIM, &(i + 1).to_string());
        let shown = if k.shown {
            Language::English.word(idx)
        } else {
            "••••••"
        };
        ui.text_mid(cx + 36.0, wy, 38.0, 14.0, W::M, TEXT, shown);
    }
    cy += m.indices().len().div_ceil(cols) as f32 * 46.0 + 8.0;
    if let Some(n) = &k.note {
        ui.text(x, cy, 13.0, W::R, ERR, n);
        cy += 24.0;
    }
    ui.button(
        x,
        cy,
        None,
        40.0,
        if k.shown { "Hide words" } else { "Show words" },
        Style::Secondary,
        Action::KShow,
    );
    next_button(ui, x, cy, w, "I wrote them down", Action::KNext);
    cy + 48.0 - y
}

/// OpenSigner's backup quiz: each word asked in a random order, picked
/// from four.
fn quiz(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use opensigner_core::quiz::QuizState;
    let mut cy = y;
    let Some(q) = k.quiz.as_ref() else {
        ui.text(x, cy, 13.0, W::R, DIM, "Show the words first");
        return 26.0;
    };
    let (done, n) = q.progress();
    if q.state() == QuizState::Passed || k.skipped {
        if let Some(fp) = k.fingerprint {
            cy = stat(
                ui,
                x,
                cy,
                "Fingerprint",
                &fp_text(osk_bip::keys::Fingerprint(fp)),
                TEXT,
            );
        }
        let (said, tone) = if q.state() == QuizState::Passed && k.slip39 {
            (format!("All {} shares right", k.shares.len()), OK)
        } else if q.state() == QuizState::Passed {
            (format!("All {n} words right"), OK)
        } else {
            ("Skipped: the copy is not checked".to_string(), WARN)
        };
        cy = stat(ui, x, cy, "Quiz", &said, tone);
        if let Some(e) = &k.note {
            ui.text(x, cy, 13.0, W::R, ERR, e);
            cy += 24.0;
        }
        cy += 6.0;
        let label = if k.slot.is_some() {
            "Use this key"
        } else {
            "Add key"
        };
        next_button(ui, x, cy, w, label, Action::KAdd);
        return cy + 48.0 - y;
    }
    let progress = if k.slip39 {
        format!(
            "Share {} of {} · {done} of {n} right",
            k.quiz_share + 1,
            k.shares.len()
        )
    } else {
        format!("{done} of {n} right")
    };
    ui.text(x, cy, 13.0, W::R, MUTED, &progress);
    cy += 26.0;
    ui.text(
        x,
        cy,
        18.0,
        W::S,
        TEXT,
        &format!("Which is word {}?", q.position() + 1),
    );
    cy += 40.0;
    let wrong = q.wrong_slot();
    let bw = ((w - 3.0 * 8.0) / 4.0).min(170.0);
    for (i, &idx) in q.choices().iter().enumerate() {
        let style = if wrong == Some(i) {
            Style::Disabled
        } else {
            Style::Secondary
        };
        ui.button(
            x + i as f32 * (bw + 8.0),
            cy,
            Some(bw),
            48.0,
            if k.slip39 {
                osk_bip::slip39::word(idx)
            } else {
                Language::English.word(idx)
            },
            style,
            Action::KQuiz(i as u8),
        );
    }
    cy += 60.0;
    if q.state() == QuizState::Wrong {
        ui.text(
            x,
            cy,
            13.0,
            W::R,
            ERR,
            "Not that one. Check your copy against the words",
        );
        cy += 26.0;
        let aw = ui.button(
            x,
            cy,
            None,
            40.0,
            "Ask it again",
            Style::Secondary,
            Action::KQuizRetry,
        );
        ui.button(
            x + aw + 8.0,
            cy,
            None,
            40.0,
            "Show the words",
            Style::Ghost,
            Action::KStep(kstep::WORDS),
        );
        cy += 52.0;
    }
    if k.skip_ask {
        ui.text(
            x,
            cy,
            13.0,
            W::R,
            WARN,
            "Without the quiz, a word copied wrongly is found only when the backup is needed",
        );
        cy += 26.0;
        ui.button(
            x,
            cy,
            None,
            40.0,
            "Skip and add the key",
            Style::Secondary,
            Action::KSkip,
        );
    } else {
        ui.button(
            x,
            cy,
            None,
            40.0,
            "Skip the quiz",
            Style::Ghost,
            Action::KSkip,
        );
    }
    cy += 48.0;
    ui.text(
        x,
        cy,
        12.0,
        W::R,
        DIM,
        "Type 1 to 4 to pick · Enter adds the key once it passes",
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
