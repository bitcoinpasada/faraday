//! `Screen::CheckMessage`: checking a signed message.

use crate::drawing::View;
use crate::flow::{Contained, Pick, Row};
use crate::strings::{EN, fill};
use crate::trail::{StepFlow, Steps};
use crate::ui::pal::*;
use crate::ui::{Ui, grouped};
use crate::wallet::FileKind;
use crate::{Action, Faraday, Screen};
use faraday_draw::tokens::{BODY_M, BODY_S, TITLE_M, TITLE_S};
use faraday_draw::widgets::Icon;

/// Costs what `act` costs. Tools' Check a signed message: the message
/// itself when exactly one waits in Files, the files to choose from when
/// several do, and the empty part when none. Practice's sample message
/// stands in when there is none.
pub(crate) fn open_tile(app: &mut Faraday) {
    let waiting = waiting(app);
    match waiting[..] {
        [(k, _)] => app.act(Action::CheckMessage(k)),
        [] if app.put_on().is_some() => app.check_use_sample(),
        _ => {
            let back = match app.screen {
                Screen::CheckMessage => app.message.check_back(),
                s => s,
            };
            app.message.open_landing(back);
            app.list_offset = 0.0;
            app.screen = Screen::CheckMessage;
        }
    }
}

/// The widest the check's column is.
const COLUMN_MAX: f32 = 900.0;

/// The step that has no message yet: the pick.
pub(crate) const PICK: u8 = 0;
/// The step that shows the check's result.
pub(crate) const RESULT: u8 = 1;

/// The card titles, by step.
const TITLES: [&str; 2] = [EN.check_message, EN.decode_card_result];

/// Whether Check a signed message is drawn as the flow's one card: the one
/// place every site asks. A small panel draws the page.
pub(crate) fn contained(app: &Faraday) -> bool {
    app.screen == Screen::CheckMessage && !app.compact
}

/// The step open: the result once a message is on show.
fn open_step(app: &Faraday) -> u8 {
    if app.message.has_check() {
        RESULT
    } else {
        PICK
    }
}

/// Check a signed message's steps, plugged into Back and Your choices.
/// Only the open step is in the order: Back leaves as the page's does.
pub(crate) struct CheckMessageSteps;

/// The one [`CheckMessageSteps`].
pub(crate) static CHECK_MESSAGE_STEPS: CheckMessageSteps = CheckMessageSteps;

impl StepFlow for CheckMessageSteps {
    fn steps(&self, app: &Faraday) -> Option<Steps> {
        let k = open_step(app);
        Some(Steps::new(vec![k], Some(k)))
    }

    fn open(&self, app: &Faraday, k: u8) -> Action {
        if k == PICK {
            Action::CheckMessageAnother
        } else {
            Action::Nav(app.message.check_back())
        }
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        TITLES[usize::from(k.min(RESULT))]
    }

    fn contained(&self) -> Option<&dyn Contained> {
        Some(self)
    }
}

impl Contained for CheckMessageSteps {
    fn rows(&self, app: &Faraday) -> Vec<Row> {
        let Some((file, _)) = app.message.check_text(&app.inbox) else {
            return Vec::new();
        };
        vec![Row {
            label: faraday_text::Text::of(TITLES[usize::from(PICK)]),
            value: Some(file.duplicate()),
            mono: false,
            opens: PICK,
            current: false,
            mark: None,
        }]
    }

    fn hide_shown(&self, _: &mut Faraday) {
        // A signed message is public: no step shows a secret.
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }

    /// The only step in the order stands without a count.
    fn heads(&self, _: &Faraday) -> usize {
        1
    }

    fn pick(&self, app: &Faraday, _: u8) -> Option<Pick> {
        // Both steps: the pick stays at the left with the one shown marked.
        Some(Pick {
            question: EN.decode_choose_message,
            groups: pick_groups(app),
        })
    }
}

/// The signed messages held in Files, then the ways in: Scan and Open
/// Files. The message on show, when it came from a held file, is marked.
fn pick_groups(app: &Faraday) -> Vec<crate::flow::PickGroup> {
    let shown = app.message.checking();
    let held: Vec<crate::pick::PickRow> = waiting(app)
        .into_iter()
        .map(|(k, name)| {
            crate::pick::PickRow::pick(
                Icon::File,
                name.to_string(),
                String::new(),
                shown == Some(k),
                Action::CheckMessage(k),
            )
        })
        .collect();
    let loaded = !held.is_empty();
    let mut groups = Vec::new();
    if loaded {
        groups.push(crate::flow::PickGroup {
            head: EN.session_group_files,
            rows: held,
        });
    }
    groups.push(crate::flow::PickGroup {
        head: crate::pick::ways_in_head(loaded),
        rows: vec![
            crate::pick::PickRow::way_in(Icon::Scan, EN.compact_scan, Action::Scan),
            crate::pick::PickRow::way_in(
                Icon::File,
                EN.parts_open_files,
                Action::Nav(Screen::Files),
            ),
        ],
    });
    groups
}

/// The signed messages in Files, by index and name.
fn waiting(app: &Faraday) -> Vec<(usize, &str)> {
    app.inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == FileKind::Message)
        .map(|(k, i)| (k, &*i.name))
        .collect()
}

/// No message chosen: the files to choose from, or the empty part.
fn landing(app: &View, ui: &mut Ui, at: (f32, f32, f32)) {
    let waiting = waiting(app);
    if waiting.is_empty() {
        super::parts::nothing_to_work_on(
            app,
            ui,
            at,
            EN.check_no_signed_message,
            (Some(Action::Scan), Screen::CheckMessage),
        );
    } else {
        let rows: Vec<(&str, Action)> = waiting
            .into_iter()
            .map(|(k, name)| (name, Action::CheckMessage(k)))
            .collect();
        super::parts::file_choices(ui, at, EN.decode_choose_message, &rows);
    }
}

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if contained(app) {
        return card(app, ui, x0, cw, h);
    }
    // A small panel has the way back and the name in its bar, and one
    // column that scrolls: each value under its name.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, (cw - 96.0).min(COLUMN_MAX))
    };
    let top = if compact {
        ui.page_begin((x0, cw, h), app.list_offset);
        14.0
    } else {
        let to = app.message.check_back();
        let name = if to == Screen::WalletsStart {
            EN.check_wallets
        } else {
            super::decode_back(to)
        };
        let back = app.named_back((name, Action::Nav(to)));
        super::bar_head(ui, Some(back), EN.check_check_a_signed)
    };
    let mut y = top;
    let Some((file, bytes)) = app.message.check_text(&app.inbox) else {
        landing(app, ui, (x, y, width));
        return;
    };
    let Some(end) = result(app, ui, file, bytes, (x, y, width)) else {
        return;
    };
    y = end;
    if compact {
        let guide = EN.check_a_valid_signature;
        y += 16.0 + crate::compact::screen::about(ui, x, y + 16.0, width, guide);
        crate::compact::screen::finish(app, ui, h, y - top + 24.0);
    }
}

/// The check's result from (x, y) in a column `width` wide: the verdict,
/// the facts, the message and the buttons. Returns where it ends, none
/// where the file is not a signed message and only the refusal is drawn.
fn result(
    app: &View,
    ui: &mut Ui,
    file: &faraday_text::Text,
    bytes: &[u8],
    (x, y, width): (f32, f32, f32),
) -> Option<f32> {
    let compact = ui.compact;
    let mut y = y;
    let Some(c) = app.message_check(&String::from_utf8_lossy(bytes)) else {
        ui.wrap_as(
            (x, y, width),
            BODY_M,
            Role::Error,
            &fill(EN.check_not_a_signed, &[app.session.network().name()]),
        );
        return None;
    };
    let (line, color) = match &c.verdict {
        Ok(f) => (
            fill(
                EN.check_signature_valid,
                &[(if *f == faraday_psbt::message::Format::Bip322 {
                    "BIP-322"
                } else {
                    "BIP-137"
                })],
            ),
            Role::Ok,
        ),
        Err(e) => (
            crate::state::message_error_text(*e).to_string(),
            Role::Error,
        ),
    };
    // The verdict's card grows to its wrapped line.
    let (size, lx) = if compact {
        (TITLE_S, 46.0)
    } else {
        (TITLE_M, 54.0)
    };
    let line_h = if compact {
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let lh = ui.wrap_as((x + lx, y, width - lx - 14.0), size, color, &line);
        ui.c.pop_clip();
        lh
    } else {
        0.0
    };
    let card_h = if compact { line_h + 32.0 } else { 64.0 };
    ui.card(x, y, width, card_h, Role::OutlineVariant);
    ui.icon(
        x + lx - 36.0,
        y + card_h / 2.0 - 12.0,
        24.0,
        if c.verdict.is_ok() {
            Icon::Success
        } else {
            Icon::Error
        },
        16.0,
        color,
    );
    if compact {
        ui.wrap_as((x + lx, y + 16.0, width - lx - 14.0), size, color, &line);
    } else {
        ui.text_mid_as((x + 54.0, y, 64.0), TITLE_M, color, &line);
    }
    y += card_h + 20.0;
    let rows = [
        (EN.check_file, file.duplicate(), BODY_M.mono()),
        (EN.check_address, grouped(&c.signed.address), BODY_M.mono()),
        (
            EN.check_wallet,
            match c.wallet {
                Some((i, change, index)) => faraday_text::text!(
                    "{} · {} {}/{index}",
                    &*app.session.wallets[i].name,
                    if change {
                        EN.check_change
                    } else {
                        EN.check_receive
                    },
                    u8::from(change)
                ),
                None => faraday_text::Text::of(EN.check_not_an_address),
            },
            BODY_M,
        ),
    ];
    let vx = if compact { 0.0 } else { 100.0 };
    for (k, v, face) in rows.iter() {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, k);
        if compact {
            y += 18.0;
        }
        let used = ui.wrap_as((x + vx, y, width - vx), *face, Role::OnSurface, v);
        y += used.max(20.0) + 12.0;
    }
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.check_message);
    if compact {
        y += 18.0;
    } else {
        ui.fill(
            x + 100.0,
            y - 4.0,
            width - 100.0,
            1.0,
            0.0,
            Role::OutlineVariant,
        );
        y += 12.0;
    }
    let shown = app.message.shown().text(&c.signed.message);
    y += ui.wrap_as((x + vx, y, width - vx), BODY_M, Role::OnSurface, &shown);
    y += super::parts::ending(
        ui,
        (x, y, width),
        c.wallet
            .map(|(i, ..)| (EN.explore_open_wallet, Action::OpenWallet(i))),
        (
            Action::CheckMessageAnother,
            Action::Nav(app.message.check_back()),
        ),
    );
    Some(y)
}

/// Check a signed message as the flow's one card: the pick at the left,
/// the result as the card (`docs/FLOW-CONTAINER.md` §2).
fn card(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let to = app.message.check_back();
    let name = if to == Screen::WalletsStart {
        EN.check_wallets
    } else {
        super::decode_back(to)
    };
    let page = crate::flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        back: Some(app.named_back((name, Action::Nav(to)))),
        heading: EN.check_check_a_signed,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::No,
    };
    let Some(contained) = StepFlow::contained(&CHECK_MESSAGE_STEPS) else {
        return;
    };
    let scroll = app.message.check_scroll();
    let (next, again, _) = {
        let app_ref: &View = app;
        let mut body_of = |ui: &mut Ui, step: u8, x: f32, y: f32, w: f32| -> f32 {
            if step == PICK {
                let mut cy = y;
                if waiting(app_ref).is_empty() {
                    cy += ui.wrap_as(
                        (x, cy, w),
                        BODY_M,
                        Role::OnSurfaceVariant,
                        EN.check_no_signed_message,
                    );
                }
                // Where the pair stands the pane holds the rows.
                if crate::flow::pair_left(app_ref, &CHECK_MESSAGE_STEPS, ui.compact, cw) {
                    return cy - y;
                }
                for g in pick_groups(app_ref) {
                    cy += crate::pick::head(ui, x, cy, w, g.head);
                    cy += crate::pick::rows(ui, x, cy, w, &g.rows);
                }
                return cy - y;
            }
            let Some((file, bytes)) = app_ref.message.check_text(&app_ref.inbox) else {
                return 0.0;
            };
            let w = ui.flow_work.map_or(w, |end| end - x).min(COLUMN_MAX);
            result(app_ref, ui, file, bytes, (x, y, w)).map_or(0.0, |end| end - y)
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        crate::flow::contained(
            ui,
            app_ref,
            contained,
            &page,
            scroll,
            &mut body_of,
            &mut under,
        )
    };
    app.scroll(crate::drawing::Scrolled::CheckMessage, next, again);
}
