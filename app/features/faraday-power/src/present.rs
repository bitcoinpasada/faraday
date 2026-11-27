use faraday_kernel::{Line, Refused, SelfTest, Show, View, Worded};
use faraday_organisms::content::{Block, Content};
use faraday_organisms::sheet::{DialogModel, SheetModel};
use faraday_organisms::{ButtonModel, ButtonsModel, Shown, Style};
use faraday_present::{Present, offer};

use crate::{FailedModel, Model, Msg, Power, Press, Refusal, State, Which, scroll};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        match *self {}
    }
}

impl Refused for Refusal {}

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
        offered: offer::<Press, Power>(state, &Msg::Press(press.clone()), press, view),
        chosen: false,
        icon: None,
    }
}

impl Present for Power {
    type Model<'s> = Model<'s>;

    fn present<'s>(state: &'s State, view: &View<'s>, show: &Show<'s>) -> Model<'s> {
        let s = show.strings();
        let online = view.mode.online;
        let on = |label, style, press| button(state, view, label, style, press);
        let Some(which) = state.sheet() else {
            return Model { sheet: None };
        };
        // The title, what happens next and the buttons, Cancel first and the main one last.
        let (title, then, items) = match which {
            Which::Power => {
                let leave = if online {
                    s.power_quit
                } else {
                    s.power_power_off
                };
                let mut items = vec![on(s.power_cancel, Style::Tonal, Press::Away)];
                if !online {
                    items.push(on(s.power_restart, Style::Tonal, Press::Restart));
                }
                items.push(on(leave, Style::Filled, Press::PowerOff));
                let then = if online {
                    s.power_app_closes
                } else {
                    s.power_the_computer_restarts
                };
                (if online { leave } else { s.power_title }, then, items)
            }
            Which::LockAsk => (
                s.vault_lock,
                s.lock_ask_then,
                vec![
                    on(s.lock_ask_cancel, Style::Tonal, Press::Away),
                    on(
                        s.vault_lock_without_saving,
                        Style::Tonal,
                        Press::LockWithoutSealing,
                    ),
                    on(s.vault_lock, Style::Filled, Press::Lock),
                ],
            ),
            Which::Locked => (
                s.locked_locked,
                "",
                vec![on(s.locked_continue, Style::Filled, Press::Away)],
            ),
            Which::IdleWarn => (
                s.idle_warn_locking_soon,
                "",
                vec![on(s.idle_warn_keep_working, Style::Filled, Press::Away)],
            ),
        };
        let mut blocks = vec![Block::Headline(Shown::plain(title))];
        if !then.is_empty() {
            blocks.push(Block::Heading(Shown::plain(then)));
        }
        Model {
            sheet: Some(DialogModel {
                sheet: SheetModel {
                    head: None,
                    body: Content(blocks),
                    foot: ButtonsModel::row(items),
                    beside: Some(Press::Away),
                    scroll: scroll(which),
                },
                want: 0.0,
            }),
        }
    }
}

impl Power {
    /// The failed self-test's page: the check that did not reproduce and the one way out. None
    /// while the self-test has not failed.
    #[must_use]
    pub fn failed<'s>(state: &State, view: &View<'s>, show: &Show<'s>) -> Option<FailedModel<'s>> {
        let SelfTest::Failed(check) = view.facts.selftest else {
            return None;
        };
        let s = show.strings();
        Some(FailedModel {
            title: Shown::plain(s.selftest_failed_title),
            row: Shown::plain(s.selftest_check_row),
            check: Shown::plain(check),
            exit: button(state, view, s.selftest_exit, Style::Filled, Press::PowerOff),
        })
    }
}
