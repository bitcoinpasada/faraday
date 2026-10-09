//! The New key screen: the step-card column over [`crate::keygen`]. The
//! options and their names are OpenSigner's; this file only lays them out.

use opensigner_core::strings::EN;
use osk_bip::bip39::Language;
use osk_entropy::{CameraNoise, CardDraws, CoinFlips, DiceProcedure, RANKS, SUITS};

use crate::keygen::{
    Group, KeyGen, MIX_SOURCES, Source, WAYS, Way, kstep, procedure_name, source_name,
};
use crate::screens::{buttons_and_next, next_button, section_label, stepper};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday, flow, guide};
use osk_ui::Color;
use osk_ui::widgets::Icon;

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
            guide: Some(guide::keygen(s, k)),
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
        chip: None,
        chip_tap: None,
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
        Source::Dice => format!("{} rolls of a six-sided die", k.dice_needed()),
        Source::Coins => format!(
            "{} flips, or rolls of a die read as flips",
            CoinFlips::needed(st)
        ),
        Source::Cards => format!("{} cards from a shuffled deck", CardDraws::needed(st)),
        Source::Hex => format!("{} digits made elsewhere", st.hex_digits()),
        Source::Camera => format!("{} pictures", CameraNoise::needed(st)),
        Source::Mix => "Two or more of these, combined".to_string(),
        Source::Device => format!("{} · getrandom(2)", EN.create_device_os),
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
            Some(Source::Dice) => k.way().map(way_name).unwrap_or_default(),
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
        kstep::LENGTH => length(k, ui, x, y, w),
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

fn length(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
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
    let used = length_counts(k, ui, x, cy, w);
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
            &format!(
                "Any {} of the {} shares restore the key",
                k.slip_m, k.slip_n
            ),
        ) + 12.0;
        ui.text(x, cy, 13.0, W::S, MUTED, "What shares reveal");
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
            "Continue",
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
    let mut used = y - top + 52.0;
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
    let name = ui.fit(14.0, W::S, name, w - 54.0);
    ui.text(x + 42.0, y + 8.0, 14.0, W::S, TEXT, &name);
    let line = ui.fit(12.0, W::R, line, w - 54.0);
    ui.text(x + 42.0, y + 29.0, 12.0, W::R, MUTED, &line);
    ui.hit(x, y, w, 52.0, a);
}

/// An option's name on the Randomness card and in its summary.
fn way_name(w: Way) -> String {
    match w {
        Way::DiceFlips => format!("{} · Flip mode", source_name(Source::Dice)),
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
        Way::DiceFlips => format!("{} rolls, read as flips", CoinFlips::needed(st)),
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
        Group::ByHand => "Your own entropy, words verifiable by hand",
        Group::Computed => "Your entropy, computer generates words",
        Group::Device => "Made by this device",
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
                bx += ui.chip(x, cy, "Recommended", OK, OK.with_alpha(30)) + 8.0;
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
        ui.chip(x + lw + 12.0, y - 4.0, "Recommended", OK, OK.with_alpha(30));
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
                    ui.checkbox(x + 44.0, cy + 7.0, on, true);
                    ui.text(x + 74.0, cy + 6.0, 14.0, W::R, TEXT, source_name(m));
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
    let drawn = next_button(ui, x, cy, w, "Continue", Action::KNext);
    cy + (if drawn { 48.0 } else { 8.0 }) - y
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
                    .and_then(|r| osk_entropy::card_index(r, s as u8))
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
            let deck = format!(
                "Deck {} · {} drawn from it",
                k.cards.deck(),
                k.cards.in_deck()
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
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, n) + 4.0;
    }
    cy += 6.0;
    let last_of_mix = k.source != Some(Source::Mix) || k.mix_at + 1 >= k.mix_list().len();
    let label = if last_of_mix {
        "Make the words"
    } else {
        "Next source"
    };
    let edits: &[(&str, Style, Action)] = if matches!(active, Some(Source::Device)) {
        &[]
    } else {
        &[
            ("Undo", Style::Ghost, Action::KUndo),
            ("Clear", Style::Ghost, Action::KClear),
        ]
    };
    cy += buttons_and_next(ui, x, cy, w, edits, Some((label, Action::KNext))) + 4.0;
    if ui.compact {
        return cy - y;
    }
    let hint = match active {
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
        // Typed and Buttons go on a row of their own when the four do
        // not fit.
        let need: f32 = ["Typed", "Buttons"]
            .iter()
            .map(|l| ui.measure(13.0, W::S, l) + 38.0)
            .sum();
        if bx + need > x + w {
            bx = x;
            cy += 42.0;
        }
    }
    for (label, on) in [("Typed", true), ("Buttons", false)] {
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

/// The box entries are typed into. Each is taken as it is typed, and
/// the box shows every one taken, by buttons too.
fn typed_box(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    ui.fill(x, y, w, 40.0, 8.0, BG);
    ui.stroke(x, y, w, 40.0, 8.0, ACCENT);
    let shown = tail_fit(ui, &k.entered, w - 30.0);
    ui.text_mid(x + 12.0, y, 40.0, 14.0, W::M, TEXT, &shown);
    let cx = x + 13.0 + ui.measure(14.0, W::M, &shown);
    ui.caret(cx, y + 11.0, 18.0);
    let what = match k.active() {
        Some(Source::Coins) if !k.by_die => "H or 1 for heads, T or 0 for tails",
        _ => "Faces 1 to 6, as rolled",
    };
    ui.text(x, y + 48.0, 12.0, W::R, DIM, what);
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
    if k.source == Some(Source::Dice) && !k.by_die {
        return rolled_words(k, ui, x, y, w);
    }
    let mut cy = y + 4.0;
    section_label(ui, x, cy, "Words so far");
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
            format!("{part} of 11 bits")
        } else {
            format!("{part} of {per} rolls")
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
    row(
        ui,
        cy,
        last,
        &code,
        made,
        "From the checksum, once the rest is in",
    );
    cy += step;
    if coins && made.is_some() {
        cy += ui.wrap(
            x,
            cy,
            w,
            12.0,
            W::R,
            DIM,
            &format!(
                "The last {checksum} bits of word {} are the checksum",
                last + 1
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
    section_label(ui, x, cy, "Words so far");
    cy += 24.0;
    ui.text(x, cy, 12.0, W::R, MUTED, crate::wordlist::PRESS_A_WORD);
    cy += 20.0;
    cy += ui.wrap(
        x,
        cy,
        w,
        12.0,
        W::R,
        MUTED,
        "Rolls 1 to 5 of a word: 1 = 00, 2 = 01, 3 = 10, 4 = 11 · Roll 6: 1 to 3 = 0, 4 to 6 = 1 · \
         The 11 bits are the word's number in the list, from 0",
    ) + 12.0;
    let per = osk_entropy::DICE_WORD_ROLLS;
    let faces = osk_entropy::DICE_WORD_FACES;
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
            ui.text_mid(sx, cy, 22.0, 11.0, W::R, DIM, "checksum");
            ui.text_mid(sx, cy + 22.0, 22.0, 13.0, W::M, MUTED, sum);
            let sw = ui
                .measure(11.0, W::R, "checksum")
                .max(ui.measure(13.0, W::M, sum));
            if !compact {
                info_x = info_x.max(sx + sw + 14.0);
            }
        }
        let cy = cy + info_y;
        match index {
            Some(i) if compact => {
                let pw = ui.word_pill(info_x, cy - 3.0, Language::English.word(i), open_word(i));
                let line = format!("= {i} · word {}", i + 1);
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
                    &format!("= {i} · word {} of the list", i + 1),
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
        let note = format!("{} of {per} rolls", got.len());
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
        "The checksum, from all the other bits, once every roll is in",
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
            let (r, s) = osk_entropy::card_parts(i);
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
    if ui.compact && ui.measure(13.0, W::M, value) > 130.0 {
        ui.text(x, y + 20.0, 13.0, W::M, tone, value);
        return y + 46.0;
    }
    let vx = if ui.compact { 110.0 } else { 140.0 };
    ui.text(x + vx, y, 13.0, W::M, tone, value);
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
        &opensigner_core::strings::fill(
            EN.create_run_face,
            &[&st.longest_run.to_string(), &side.to_lowercase()],
        ),
        TEXT,
    )
}

fn check(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match k.source {
        Some(Source::Dice) if k.by_die => {
            cy = coin_check(&k.coins, ui, x, cy);
        }
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
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        &[(again, Style::Ghost, Action::KAgain)],
        Some(("Continue", Action::KNext)),
    );
    cy - y
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
    let cols = if ui.compact { 2 } else { 4 };
    let cellw = (w - (cols - 1) as f32 * 10.0) / cols as f32;
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
    // The next share, or past the last one, the next step: pinned on a
    // small panel.
    if k.share_at + 1 < k.shares.len() {
        if ui.pinning {
            ui.pin = Some(("Next share".to_string(), Style::Primary, Action::KShare(1)));
        } else {
            ui.button(
                bx,
                cy,
                None,
                40.0,
                "Next share",
                Style::Primary,
                Action::KShare(1),
            );
        }
    } else {
        let _ = next_button(ui, x, cy, w, "I wrote them all down", Action::KNext);
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
    let cols = if ui.compact { 2 } else { 4 };
    let cellw = (w - (cols - 1) as f32 * 10.0) / cols as f32;
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
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        &[(
            if k.shown { "Hide words" } else { "Show words" },
            Style::Secondary,
            Action::KShow,
        )],
        Some(("I wrote them down", Action::KNext)),
    );
    cy - y
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
        let drawn = next_button(ui, x, cy, w, label, Action::KAdd);
        return cy + (if drawn { 48.0 } else { 8.0 }) - y;
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
                osk_bip::slip39::word(idx)
            } else {
                Language::English.word(idx)
            },
            style,
            Action::KQuiz(i as u8),
        );
    }
    cy += q.choices().len().div_ceil(per) as f32 * 56.0 + 4.0;
    if q.state() == QuizState::Wrong {
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::R,
            ERR,
            "Not that one. Check your copy against the words",
        ) + 10.0;
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
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::R,
            WARN,
            "Without the quiz, a word copied wrongly is found only when the backup is needed",
        ) + 10.0;
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
    if ui.compact {
        return cy - y;
    }
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
