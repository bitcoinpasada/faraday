//! The vanity address grinder's screens, from a key page's "Vanity
//! address" row (`docs/PLANNING.md` §16.117).
//!
//! Four screens and a fifth behind the eye: the script type, the prefix
//! on the address keyboard, the run with its count and its rate, the
//! find, and — for the passphrase dial — the counter it named, which is
//! part of a passphrase and is masked like one.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::ScriptType;
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{
    self, Above, Action, Chrome, Entry, Item, Record, Result as ResultScreen,
    Secret as SecretScreen, Value,
};
use osk_ui::widgets::keyboard::{self, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::strings::Strings;
use crate::vanity::{Dial, Grind, Step};
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    /// The grinder's screens, by the step the flow is on.
    pub(crate) fn view_vanity(&self, key: usize) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let Some(g) = self.vanity.as_ref() else {
                return screens::menu(c, s.key_vanity_row, None, Vec::new(), Vec::new());
            };
            if self.keys.get(key).is_none() {
                return screens::menu(c, s.key_vanity_row, None, Vec::new(), Vec::new());
            }
            match g.step {
                Step::Script => self.vanity_script(c, g, s),
                Step::Prefix => self.vanity_prefix(c, g, s),
                Step::Running => self.vanity_running(c, g, s),
                Step::Found => self.vanity_found(c, g, s),
                Step::Secret => self.vanity_secret(c, g, s),
            }
        })
    }

    /// §5 Choice, "Which script type?": the four types, the one the
    /// grind is at checked, and Continue.
    fn vanity_script(&self, c: &Chrome<'_>, g: &Grind, s: &Strings) -> Node {
        let items: Vec<Item> = ScriptType::ALL
            .iter()
            .enumerate()
            .map(|(i, script)| {
                Item::key(
                    ids::at(ids::VANITY_SCRIPT_BASE, i),
                    text::script_short(*script, s),
                    String::from(osk_bip::vanity::fixed_prefix(*script, self.network)),
                    *script == g.script,
                )
            })
            .collect();
        screens::choice(
            c,
            s.script_type_title,
            items,
            Action::new(ids::VANITY_SCRIPT_CONTINUE, s.action_continue),
        )
    }

    /// §5 Entry, "Address prefix": the prefix whole above the field, the
    /// address keyboard under it with every character the encoding
    /// cannot carry dimmed, and ✓ dead until something is asked for
    /// past the fixed prefix.
    fn vanity_prefix(&self, c: &Chrome<'_>, g: &Grind, s: &Strings) -> Node {
        let typed = String::from(g.prefix.as_str());
        let enabled = self.vanity_keys(g)
            | if g.prefix.has_free(g.script, self.network) {
                0
            } else {
                DONE_DISABLED
            };
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: s.vanity_prefix_title,
                value: typed.clone(),
                mono: true,
                above: Above::Whole {
                    id: ids::VANITY_ADDRESS,
                    text: typed,
                },
                words: None,
                eye: None,
                keyboard: (ids::VANITY_KEYBOARD, KeyboardKind::Address),
                enabled,
                error: None,
            },
        )
    }

    /// The keys the address keyboard still offers: those that can
    /// continue a prefix of this script type on this network.
    fn vanity_keys(&self, g: &Grind) -> keyboard::KeyMask {
        let mut mask = 0;
        let mut next = String::from(g.prefix.as_str());
        for ch in ('a'..='z').chain('A'..='Z').chain('0'..='9') {
            next.truncate(g.prefix.as_str().len());
            next.push(ch);
            if osk_bip::vanity::can_begin(&next, g.script, self.network) {
                mask |= keyboard::key_bit(KeyboardKind::Address, ch);
            }
        }
        mask
    }

    /// §5 Record, "Grinding": how many candidates have been tried, how
    /// fast they are going, how long the prefix is expected to take,
    /// and Stop.
    ///
    /// §4.14's progress is "a bar with a count '2 of 5' above it", and
    /// a grind has no total to count towards: the expected figure is a
    /// mean, and a bar drawn against it would state a progress that
    /// does not exist. The three facts stand as rows instead.
    fn vanity_running(&self, c: &Chrome<'_>, g: &Grind, s: &Strings) -> Node {
        let tried = components::Record::mono(s.vanity_tried_row, count(g.tested));
        let rate = components::Record::text(
            s.vanity_rate_row,
            match g.per_second() {
                Some(rate) => strings::fill1(s.vanity_rate_value, &count(rate as u64)),
                None => String::from(s.value_none),
            },
            Tone::Text,
        );
        let expected = components::Record::text(
            s.vanity_expected_row,
            match g.expected_seconds(self.network) {
                Some(seconds) => duration(seconds, s),
                None => String::from(s.value_none),
            },
            Tone::Text,
        );
        screens::record(
            c,
            Record {
                title: s.vanity_running_title,
                key: None,
                network: text::network(self.network),
                rows: vec![tried, rate, expected],
                warnings: Vec::new(),
                pager: None,
                action: Some(Action::new(ids::VANITY_STOP, s.vanity_stop)),
            },
        )
    }

    /// §5 Result: the address that was found, the account or the
    /// counter that reaches it, "Use it", and — on the passphrase dial
    /// — the action that shows the counter.
    fn vanity_found(&self, c: &Chrome<'_>, g: &Grind, s: &Strings) -> Node {
        let Some(find) = g.find.as_ref() else {
            return screens::result(
                c,
                ResultScreen {
                    caption: None,
                    title: s.key_vanity_row,
                    icon: Icon::Warning,
                    tone: Tone::Caution,
                    result: s.vanity_exhausted_title,
                    rows: Vec::new(),
                    actions: vec![Action::new(ids::VANITY_STOP, s.vanity_stop)],
                },
            );
        };
        let mut rows = vec![components::Record::reference(
            ids::VANITY_ADDRESS,
            String::from(s.row_address),
            String::from(find.address.as_str()),
        )];
        let mut actions = vec![];
        match g.dial {
            Dial::Account => rows.push(components::Record::mono(
                s.vanity_account_row,
                count(u64::from(find.account)),
            )),
            Dial::Passphrase => {
                rows.push(components::Record::mono(
                    s.vanity_tried_row,
                    count(g.tested),
                ));
                actions.push(Action::new(ids::VANITY_SHOW, s.vanity_show));
            }
        }
        actions.push(Action::new(ids::VANITY_USE, s.vanity_use));
        screens::result(
            c,
            ResultScreen {
                caption: None,
                title: s.key_vanity_row,
                icon: Icon::Success,
                tone: Tone::Success,
                result: s.vanity_found_title,
                rows,
                actions,
            },
        )
    }

    /// §5 Secret: the counter the find named, masked until a finger is
    /// on the panel or the eye is running. It is what is appended to
    /// the key's passphrase, so it is as secret as the key.
    fn vanity_secret(&self, c: &Chrome<'_>, g: &Grind, s: &Strings) -> Node {
        let value = g
            .find
            .as_ref()
            .map(|f| String::from(f.suffix.as_str()))
            .unwrap_or_default();
        screens::secret(
            c,
            SecretScreen {
                title: s.vanity_secret_title,
                value: Value::Text(&value),
                revealed: self.revealed(ids::VANITY_REVEAL),
                panel: ids::VANITY_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                rows: Vec::new(),
                action: Some(Action::new(ids::VANITY_DONE, s.action_done)),
                secondary: None,
                pager: None,
            },
        )
    }
}

/// A count, grouped in threes where it is long enough to need it.
fn count(n: u64) -> String {
    text::thousands(u32::try_from(n).unwrap_or(u32::MAX))
}

/// A span of seconds in the largest unit that states it plainly.
fn duration(seconds: u64, s: &Strings) -> String {
    let (template, value) = if seconds < 120 {
        (s.vanity_seconds, seconds)
    } else if seconds < 7200 {
        (s.vanity_minutes, seconds / 60)
    } else if seconds < 172_800 {
        (s.vanity_hours, seconds / 3600)
    } else if seconds < 31_536_000 {
        (s.vanity_days, seconds / 86400)
    } else {
        return String::from(s.vanity_over_a_year);
    };
    strings::fill1(template, &count(value))
}

/// The two dials, as the Choice names them.
pub(crate) fn dial_name(dial: Dial, s: &Strings) -> &'static str {
    match dial {
        Dial::Passphrase => s.vanity_how_passphrase,
        Dial::Account => s.vanity_how_account,
    }
}

/// What one candidate costs on each dial, which is the second line of
/// its row.
pub(crate) fn dial_cost(dial: Dial, s: &Strings) -> &'static str {
    match dial {
        Dial::Passphrase => s.vanity_how_passphrase_under,
        Dial::Account => s.vanity_how_account_under,
    }
}
