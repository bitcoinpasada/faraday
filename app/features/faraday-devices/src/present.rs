use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_organisms::content::{Block, Content};
use faraday_organisms::sheet::{DialogModel, SheetModel, dialog_room_least};
use faraday_organisms::{ButtonModel, ButtonsModel, HoldModel, HoldTone, Shown, Style};
use faraday_present::{Present, offer};
use faraday_strings::{EN, fill};
use faraday_text::Text;

use crate::inputs::{CODE_LEN, NewInput, TAP_TRIES, TARGETS_MAX, target_count};
use crate::{Devices, HOLD, Model, Msg, Press, Refusal, Said, State, scroll};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        Line::Fixed(match self {
            Refusal::VirtualMachine => EN.devices_real_computer,
            Refusal::NotAsked => EN.devices_continue_ask,
            Refusal::StillHeld => EN.devices_still,
        })
    }
}

impl Refused for Refusal {}

/// A device's name for a toast: what it gave, or "USB device" when it gave none.
fn named(name: &Text) -> &str {
    if name.is_empty() {
        EN.inputs_usb_device_unnamed
    } else {
        name.as_str()
    }
}

impl Worded for Said {
    fn words(&self, _show: &Show<'_>) -> Line {
        let one =
            |template: &str, name: &Text| Line::Filled(Text::fill(template, &[name.as_str()]));
        match self {
            Said::NewDeviceHeld(n) => one(EN.inputs_new_device_held, n),
            Said::KeyboardIn(n) => one(EN.inputs_keyboard_in_use, n),
            Said::PointerIn(n) => one(EN.inputs_pointer_in_use, n),
            Said::KeyboardPointerIn(n) => one(EN.inputs_keyboard_pointer_in_use, n),
            Said::TouchIn(n) => one(EN.inputs_touchscreen_in_use, n),
            Said::WrongCode(n) => one(EN.inputs_wrong_code, n),
            Said::WrongPresses(n) => one(EN.inputs_wrong_presses, n),
            Said::WrongTaps(n) => one(EN.inputs_wrong_taps, n),
            Said::Ignoring { alone: true } => Line::Fixed(EN.inputs_new_devices_ignored),
            Said::Ignoring { alone: false } => Line::Fixed(EN.inputs_new_keyboards_ignored),
            Said::Plugged(names) => {
                let named: Vec<&str> = names.iter().map(named).collect();
                Line::Filled(Text::fill(
                    EN.inputs_plugged_in_at_start,
                    &[Text::joined(&named, ", ").as_str()],
                ))
            }
            Said::Still => Line::Fixed(EN.devices_still),
            Said::KeyboardsHeld(names) => {
                let named: Vec<&str> = names.iter().map(named).collect();
                Line::Filled(Text::fill(
                    EN.inputs_keyboard_attached,
                    &[Text::joined(&named, ", ").as_str()],
                ))
            }
        }
    }
}

fn button<'s>(
    state: &State,
    view: &View<'_>,
    label: &'static str,
    style: Style,
    press: Press,
) -> ButtonModel<'s, Press> {
    ButtonModel {
        label: Shown::plain(label),
        style,
        offered: offer::<Press, Devices>(state, &Msg::Press(press.clone()), press, view),
        chosen: false,
        icon: None,
    }
}

fn dialog<'s>(
    blocks: Vec<Block<'s, Press>>,
    foot: Vec<ButtonModel<'s, Press>>,
    scroll: faraday_port::values::Anchor,
    beside: Option<Press>,
) -> DialogModel<'s, Press> {
    DialogModel {
        sheet: SheetModel {
            head: None,
            body: Content(blocks),
            foot: ButtonsModel::row(foot),
            beside,
            scroll,
        },
        want: 0.0,
    }
}

impl Devices {
    /// The Devices sheet: what is held, with what each can do, and the ways on. While a press
    /// waits for the check it lists the reminders too (`main`'s `devices` sheet).
    #[must_use]
    pub fn present_devices<'s>(state: &'s State, view: &View<'s>, _show: &Show<'s>) -> Model<'s> {
        if !state.devices_up() {
            return Model { sheet: None };
        }
        let check = state.check();
        let on = |label, style, press| button(state, view, label, style, press);
        let waiting = state.waiting().is_some();
        let held = check.held();
        let title = if held.is_empty() && waiting {
            EN.devices_before
        } else {
            EN.devices_title
        };
        let mut blocks = vec![Block::Headline(Shown::plain(title))];
        for c in held {
            blocks.push(Block::Fact {
                label: Shown::plain(&c.name),
                value: Shown::plain(c.kind.what()),
                mono: false,
            });
        }
        if check.refuses() {
            blocks.push(Block::Heading(Shown::plain(EN.devices_real_computer)));
        }
        if waiting {
            // The reminders show only while a secret waits, never at start.
            for r in check.reminders_due() {
                blocks.push(Block::Heading(Shown::plain(r.line())));
            }
            blocks.push(Block::Heading(Shown::plain(EN.devices_look_around)));
        }
        let mut foot = Vec::new();
        if waiting {
            foot.push(on(EN.devices_cancel, Style::Tonal, Press::Cancel));
        }
        if !held.is_empty() {
            foot.push(on(EN.devices_check_again, Style::Tonal, Press::Check));
            if check.refuses() {
                foot.push(on(EN.devices_power_off, Style::Filled, Press::PowerOff));
            } else if check.asking() {
                blocks.push(Block::Heading(Shown::plain(EN.devices_continue_ask)));
                blocks.push(Block::Hold(HoldModel {
                    label: Shown::plain(EN.devices_continue_hold),
                    tone: HoldTone::Warning,
                    id: HOLD,
                    offered: offer::<Press, Devices>(
                        state,
                        &Msg::Press(Press::LetThrough),
                        Press::LetThrough,
                        view,
                    ),
                }));
            } else {
                foot.push(on(EN.devices_continue, Style::Outlined, Press::Continue));
            }
        } else if waiting {
            foot.push(on(EN.devices_understood, Style::Filled, Press::Understood));
        }
        let beside = waiting.then_some(Press::Cancel);
        Model {
            sheet: Some(dialog(blocks, foot, scroll(0), beside)),
        }
    }

    /// The New input sheet, for the first held device that acted: its name, the code it must
    /// type or the target it must press, and Ignore this device.
    #[must_use]
    pub fn present_input<'s>(state: &'s State, view: &View<'s>, _show: &Show<'s>) -> Model<'s> {
        let inputs = state.inputs();
        let Some(d) = inputs.shown() else {
            return Model { sheet: None };
        };
        let on = |label, style, press| button(state, view, label, style, press);
        let by_target = inputs.by_target(d);
        let title = match (d.touch, by_target || d.pointer) {
            (true, _) => EN.new_input_a_new_touchscreen,
            (false, true) if !d.keyboard => EN.new_input_a_new_pointer,
            _ => EN.new_input_a_new_keyboard,
        };
        let mut blocks = vec![
            Block::Headline(Shown::plain(title)),
            Block::Heading(Shown::plain(EN.new_input_the_name)),
            Block::Fact {
                label: Shown::plain(EN.new_input_title),
                value: Shown::plain(&d.name),
                mono: false,
            },
        ];
        let mut foot = vec![on(
            EN.new_input_ignore_button,
            Style::Tonal,
            Press::Ignore(d.id),
        )];
        if by_target {
            target_lines(state, d, view, &mut blocks);
            // A device let in by targets has Ignore and the target only.
        } else {
            // A pointer with a trusted pointer to press it is let in by its button; a keyboard
            // by its code (`main`'s `new_input_sheet`).
            if !d.keyboard {
                foot.push(on(
                    EN.new_input_use_pointer,
                    Style::Filled,
                    Press::Use(d.id),
                ));
            }
            if d.keyboard {
                blocks.push(Block::Heading(Shown::plain(match d.code() {
                    Some(_) => EN.new_input_type_code,
                    None => EN.new_input_no_code,
                })));
            }
            if let Some(code) = d.code().filter(|_| d.keyboard) {
                blocks.push(Block::Fact {
                    label: Shown::plain(EN.new_input_type_code),
                    value: Shown::secret(code),
                    mono: true,
                });
                blocks.push(Block::Heading(Shown::made(Text::of(&format!(
                    "{}/{CODE_LEN}",
                    d.marked()
                )))));
            }
        }
        Model {
            sheet: Some(dialog(blocks, foot, scroll(1), None)),
        }
    }
}

/// The target lines of a device let in by pressing: which target of how many, tries left, or
/// that none fits. The target itself is the root's to draw at its place (`spots`).
fn target_lines<'s>(
    state: &State,
    d: &NewInput,
    view: &View<'_>,
    blocks: &mut Vec<Block<'s, Press>>,
) {
    let of = target_room(view);
    let Some(of) = of.filter(|_| d.drawn() && state.room_fits()) else {
        blocks.push(Block::Heading(Shown::plain(if d.touch {
            EN.new_input_no_target
        } else {
            EN.new_input_no_target_pointer
        })));
        return;
    };
    blocks.push(Block::Heading(Shown::plain(if d.touch {
        EN.new_input_tap_target
    } else {
        EN.new_input_press_target
    })));
    blocks.push(Block::Heading(Shown::made(Text::of(&fill(
        EN.new_input_target_step,
        &[&(d.reached + 1).to_string(), &of.to_string()],
    )))));
    if d.missed > 0 {
        blocks.push(Block::Heading(Shown::made(Text::of(&fill(
            EN.new_input_tries_left,
            &[&TAP_TRIES.saturating_sub(d.missed).to_string()],
        )))));
    }
}

/// The room, `(width, height)` in units, the count of targets assumes for the target's place: the
/// least a dialog leaves, less the target's own size. Whatever draws the target compares the room
/// it has with this and, when it is smaller, draws none and sends `Msg::Room(false)`.
#[must_use]
pub fn assumed_room(view: &View<'_>) -> (f32, f32) {
    let display = &view.facts.display;
    let (iw, ih) = dialog_room_least(f32::from(display.width), f32::from(display.height));
    ((iw - TOUCH).max(0.0), (ih - TOUCH).max(0.0))
}

/// How many targets the sheet draws, from [`assumed_room`]: a blind device's chance is never
/// better than a keyboard's where the room drawn in holds it (F202, `main`'s `target_count`).
fn target_room(view: &View<'_>) -> Option<usize> {
    let (w, h) = assumed_room(view);
    target_count(w, h, TOUCH).filter(|n| *n <= TARGETS_MAX)
}

/// The size of a touch, in logical pixels (the kit's `TOUCH`).
const TOUCH: f32 = 48.0;

impl Devices {
    /// The words of the notice that stays: the keyboards held back that have not acted, until a
    /// key is typed on a keyboard or the × is pressed (F202). The frame shows it; `None` when it
    /// names none. Not while a held device's sheet is up: that sheet is the whole screen.
    #[must_use]
    pub fn stays(state: &State, show: &Show<'_>) -> Option<Text> {
        if state.input_up() {
            return None;
        }
        let names = state.inputs().noticed();
        if names.is_empty() {
            return None;
        }
        match Said::KeyboardsHeld(names).words(show) {
            Line::Filled(text) => Some(text),
            Line::Fixed(fixed) => Some(Text::of(fixed)),
        }
    }
}

impl Present for Devices {
    type Model<'s> = Model<'s>;

    /// The New input sheet takes the screen before the Devices sheet.
    fn present<'s>(state: &'s State, view: &View<'s>, show: &Show<'s>) -> Model<'s> {
        if state.input_up() {
            Devices::present_input(state, view, show)
        } else {
            Devices::present_devices(state, view, show)
        }
    }
}
