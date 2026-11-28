use faraday_kernel::{Line, Refused, SelfTest, Show, View, Worded};
use faraday_organisms::content::{Block, Content};
use faraday_organisms::sheet::{DialogModel, SheetModel};
use faraday_organisms::{ButtonModel, ButtonsModel, Shown, Style};
use faraday_present::{Present, offer};
use faraday_strings::{Strings, fill};
use faraday_text::Text;

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
    label: Shown<'s>,
    style: Style,
    press: Press,
) -> ButtonModel<'s, Press> {
    ButtonModel {
        label,
        style,
        offered: offer::<Press, Power>(state, &Msg::Press(press.clone()), press, view),
        chosen: false,
        icon: None,
    }
}

/// The name of a mode asked for, by its number (`Top::ModeLock`); `None` for a number no mode has.
fn mode_name(s: &Strings, mode: u8) -> Option<&'static str> {
    match mode {
        0 => Some(s.mode_demo),
        1 => Some(s.wallet_mainnet),
        2 => Some(s.wallet_testnet),
        3 => Some(s.wallet_signet),
        4 => Some(s.wallet_regtest),
        _ => None,
    }
}

/// What a mode change wipes, from what the session holds: each key by its name or its
/// fingerprint, each wallet by its name, and the files held (`main`'s `kept_wiped` for a mode
/// change).
fn wiped(s: &Strings, view: &View<'_>) -> Vec<Text> {
    let mut lines = Vec::new();
    for key in view.session.keys().all() {
        let name = if key.label.as_str().is_empty() {
            let fp = key
                .fingerprint
                .0
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            Text::of(&fill(s.vaults_seed_n, &[&fp]))
        } else {
            key.label.clone()
        };
        lines.push(name);
    }
    for wallet in view.session.wallets() {
        lines.push(wallet.name.clone());
    }
    let n = view.session.inbox().len() + view.session.outbox().len();
    if n > 0 {
        let noun = if n == 1 {
            s.vaults_file
        } else {
            s.vaults_files
        };
        lines.push(Text::of(&format!("{n} {noun}")));
    }
    lines
}

impl Present for Power {
    type Model<'s> = Model<'s>;

    fn present<'s>(state: &'s State, view: &View<'s>, show: &Show<'s>) -> Model<'s> {
        let s = show.strings();
        let online = view.mode.online;
        let on = |label: &'static str, style, press| {
            button(state, view, Shown::plain(label), style, press)
        };
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
                (
                    Shown::plain(if online { leave } else { s.power_title }),
                    then,
                    items,
                )
            }
            Which::LockAsk => match state.mode_change().and_then(|m| mode_name(s, m)) {
                // A mode change: titled by the mode asked, Not now and Lock and use it. Lock
                // without sealing only where nothing can seal (`main`'s `seal_waits`).
                Some(to) => {
                    let mut items = vec![on(s.mode_not_now, Style::Tonal, Press::Away)];
                    if !view.session.seeded() {
                        items.push(on(
                            s.vault_lock_without_sealing,
                            Style::Tonal,
                            Press::LockWithoutSealing,
                        ));
                    }
                    let use_it = Shown::made(Text::of(&fill(s.mode_lock_and_use, &[to])));
                    items.push(button(state, view, use_it, Style::Filled, Press::Lock));
                    let head = Shown::made(Text::of(&fill(s.mode_change_title, &[to])));
                    (head, s.lock_mode_keeps_nothing, items)
                }
                None => (
                    Shown::plain(s.vault_lock),
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
            },
            Which::Locked => (
                Shown::plain(s.locked_locked),
                "",
                vec![on(s.locked_continue, Style::Filled, Press::Away)],
            ),
            Which::IdleWarn => (
                Shown::plain(s.idle_warn_locking_soon),
                "",
                vec![on(s.idle_warn_keep_working, Style::Filled, Press::Away)],
            ),
        };
        let mut blocks = vec![Block::Headline(title)];
        // A mode change says what it wipes, as the Lock sheet of any lock does.
        if state.mode_change().is_some() && which == Which::LockAsk {
            let lines = wiped(s, view);
            if !lines.is_empty() {
                blocks.push(Block::Heading(Shown::plain(s.kept_sections_wiped)));
                for line in lines {
                    blocks.push(Block::Fact {
                        label: Shown::made(line),
                        value: Shown::plain(""),
                        mono: false,
                    });
                }
            }
        }
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
            exit: button(
                state,
                view,
                Shown::plain(s.selftest_exit),
                Style::Filled,
                Press::PowerOff,
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use faraday_strings::EN;

    use super::*;

    /// Invariant: The Lock sheet of a mode change is titled "Change to X?" and its Lock reads "Lock and use X" with the name of the mode asked, for each mode a person can ask, and a number that is no mode names none.
    /// Regression: The sheet says "Change to Mainnet?" whatever was asked, or names a mode that is not one.
    /// Owner: app
    #[test]
    fn the_mode_change_sheet_names_the_mode_asked() {
        for (mode, name) in [
            (0, "Practice"),
            (1, "Mainnet"),
            (2, "Testnet"),
            (3, "Signet"),
            (4, "Regtest"),
        ] {
            let to = mode_name(&EN, mode);
            assert_eq!(to, Some(name));
            assert_eq!(
                fill(EN.mode_change_title, &[name]),
                format!("Change to {name}?")
            );
            assert_eq!(
                fill(EN.mode_lock_and_use, &[name]),
                format!("Lock and use {name}")
            );
        }
        assert_eq!(mode_name(&EN, 5), None);
    }
}
