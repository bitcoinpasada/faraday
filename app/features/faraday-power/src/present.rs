use faraday_kernel::session::WaitingItem;
use faraday_kernel::{Line, Refused, SelfTest, Show, View, Worded};
use faraday_organisms::content::{Block, Content};
use faraday_organisms::sheet::{DialogModel, SheetModel};
use faraday_organisms::{ButtonModel, ButtonsModel, CautionModel, CautionSize, Shown, Style};
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

/// One line of what a lock wipes: its text and whether it is kept nowhere else.
struct Wiped {
    line: Text,
    nowhere: bool,
}

/// Whether a copy of the key with this fingerprint has been checked. The backup's check is not
/// built, so none is (`main`'s `seed_copy_checked`).
fn copy_checked(_view: &View<'_>, _fingerprint: [u8; 4]) -> bool {
    false
}

/// What a lock wipes, from what the session holds, as `main`'s `kept_wiped_now` lists it for a
/// mode change: Make a new wallet under way, the files for the stick, each key by its name and
/// where else it is, and each wallet; what is nowhere else first. No vault can be open yet, so
/// no key is in one and there is no Kept list.
fn wiped(s: &Strings, view: &View<'_>) -> Vec<Wiped> {
    let mut lines = Vec::new();
    // A key held until its backup is written is Make a new wallet under way.
    if view
        .session
        .waiting()
        .iter()
        .any(|w| matches!(w, WaitingItem::Held(_)))
    {
        lines.push(Wiped {
            line: Text::of(s.lock_held_ends),
            nowhere: true,
        });
    }
    let n = view.session.outbox().len();
    if n > 0 {
        let noun = if n == 1 {
            s.vaults_file
        } else {
            s.vaults_files
        };
        lines.push(Wiped {
            line: Text::of(&format!("{n} {noun} {}", s.medium_for_the_stick_inline)),
            nowhere: true,
        });
    }
    for key in view.session.keys().all() {
        let fp = key
            .fingerprint
            .0
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        // A key with no name of its own goes by "Seed {fingerprint}".
        let name = if key.label.as_str().is_empty() || key.label.as_str().eq_ignore_ascii_case(&fp)
        {
            Text::of(&fill(s.vaults_seed_n, &[&fp]))
        } else {
            key.label.clone()
        };
        let checked = copy_checked(view, key.fingerprint.0);
        let state = if checked {
            s.vaults_copy_checked
        } else {
            s.vaults_nowhere_else
        };
        lines.push(Wiped {
            line: Text::of(&format!("{} \u{b7} {state}", name.as_str())),
            nowhere: !checked,
        });
    }
    for wallet in view.session.wallets() {
        lines.push(Wiped {
            line: wallet.name.clone(),
            nowhere: false,
        });
    }
    // What is nowhere else first, where it is read.
    lines.sort_by_key(|l| !l.nowhere);
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
                // A mode change: titled by the mode asked, Not now and Lock and use it. No vault
                // can be open yet, so "Lock without sealing" is never offered: with the vault
                // tickets it takes the place of the sealing Lock while `seal_waits` (`main`,
                // sheets.rs `lock_ask_sheet`, vault/mod.rs), with its randomness line.
                Some(to) => {
                    let mut items = vec![on(s.mode_not_now, Style::Tonal, Press::Away)];
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
                for Wiped { line, nowhere } in lines {
                    // Nowhere else is in the warning tone, the kit's one toned line.
                    blocks.push(if nowhere {
                        Block::Caution(CautionModel {
                            text: Shown::made(line),
                            size: CautionSize::Body,
                        })
                    } else {
                        Block::Fact {
                            label: Shown::made(line),
                            value: Shown::plain(""),
                            mono: false,
                        }
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
    use faraday_app_test::TestView;
    use faraday_testkit::test_seeds;

    use super::*;

    /// The Lock sheet of a mode change as a person reads it: its body's lines, each tagged by
    /// kind, and its buttons with whether each is the Filled one.
    fn read(view: &TestView, mode: u8) -> (Vec<String>, Vec<(String, bool)>) {
        let mut state = State::new(Some(Which::LockAsk));
        state.set_mode_change(Some(mode));
        view.showing(|view, show| {
            let model = <Power as Present>::present(&state, view, show);
            let sheet = model.sheet.expect("the Lock sheet is up").sheet;
            let body = sheet
                .body
                .0
                .iter()
                .map(|b| match b {
                    Block::Headline(t) => format!("headline:{}", t.as_str()),
                    Block::Heading(t) => format!("heading:{}", t.as_str()),
                    Block::Fact { label, .. } => format!("fact:{}", label.as_str()),
                    Block::Caution(c) => format!("caution:{}", c.text.as_str()),
                    _ => "other".to_owned(),
                })
                .collect();
            let foot = sheet
                .foot
                .items
                .iter()
                .map(|b| (b.label.as_str().to_owned(), b.style == Style::Filled))
                .collect();
            (body, foot)
        })
    }

    /// Invariant: The Lock sheet of a mode change is headed "Change to X?" and its Filled button reads "Lock and use X" with the mode asked, and it offers Not now beside it and never "Lock without sealing" (no vault can be open).
    /// Regression: The sheet says "Lock" whatever was asked, names the wrong mode, or offers a lock without sealing that loses nothing.
    /// Owner: app
    #[test]
    fn the_mode_change_sheet_names_the_mode_asked() {
        let (body, foot) = read(&TestView::device(), 1);
        assert!(body.contains(&"headline:Change to Mainnet?".to_owned()));
        assert_eq!(
            foot,
            [
                ("Not now".to_owned(), false),
                ("Lock and use Mainnet".to_owned(), true)
            ]
        );
    }

    /// Invariant: A key with no checked copy is listed on the mode change's sheet under what is wiped as nowhere else, first, in the warning tone, by its name.
    /// Regression: A key kept nowhere else is left off the list, or listed after what is safe.
    /// Owner: app
    #[test]
    fn a_key_kept_nowhere_else_is_listed_first_among_what_is_wiped() {
        let seed = test_seeds()[0];
        let (body, _) = read(&TestView::device().with_key(seed), 2);
        let at = body
            .iter()
            .position(|l| l == "heading:Wiped from memory")
            .expect("the wiped list");
        assert_eq!(
            body[at + 1],
            format!("caution:{} \u{b7} nowhere else", seed.name)
        );
    }
}
