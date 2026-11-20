//! New key's steps as Back and Forward walk them (F104): its cards in
//! the order [`KeyGen::steps`] shows them, and inside Make codex32
//! shares that flow's own cards in its place (`docs/CODEX32.md` §6),
//! numbered from [`CODEX32_FROM`]. Its single choices move on inside the
//! flow itself (F103), so `moves_on` is left as the trait has it.

use super::{KeyGen, kstep};
use crate::codex32::{C32, Card};
use crate::flow::{Row, Title};
use crate::strings::EN;
use crate::trail::{StepFlow, Steps};
use crate::{Action, Faraday};
use faraday_text::Text;

/// The step number of Make codex32 shares' first card: card `c` is
/// `CODEX32_FROM + c.index()`.
pub(super) const CODEX32_FROM: u8 = 100;

/// The step number of the made card, the end of a key made from Tools
/// (`docs/FLOW-CONTAINER.md` §7).
pub(super) const MADE: u8 = 99;

/// New key's steps, plugged into Back and Forward.
pub(crate) struct KeyGenSteps;

/// The one [`KeyGenSteps`].
pub(crate) static KEYGEN_STEPS: KeyGenSteps = KeyGenSteps;

/// The step open: a card of New key, or of Make codex32 shares inside it.
fn open_step(k: &KeyGen) -> Option<u8> {
    match (k.open, k.codex32.as_ref()) {
        (Some(kstep::CODEX32), Some(m)) => m.open().map(|c| CODEX32_FROM + c.index()),
        (o, _) => o,
    }
}

/// Once the key is locked in, the first step Back may open: the one after
/// Key (F104), or Strings made for Make codex32 shares.
fn after_lock(k: &KeyGen, order: &[u8]) -> Option<u8> {
    if !k.locked {
        return None;
    }
    if k.codex32.is_some() {
        return Some(CODEX32_FROM + Card::Made.index());
    }
    let at = order.iter().position(|&s| s == kstep::KEY)?;
    Some(*order.get(at + 1).unwrap_or(&kstep::KEY))
}

/// New key's steps in order, Make codex32 shares' cards in place of its
/// one step (a set typed in has no identifier, entropy or entry card).
fn order_of(k: &KeyGen) -> Vec<u8> {
    let mut order = Vec::new();
    for s in k.steps() {
        match k.codex32.as_ref() {
            Some(m) if s == kstep::CODEX32 => order.extend(
                crate::codex32::screen::cards(k, m)
                    .iter()
                    .map(|(c, _)| CODEX32_FROM + c.index()),
            ),
            _ => order.push(s),
        }
    }
    order
}

/// The steps of `order` whose answers stand: a card done, or Key once the
/// key is locked in. Make codex32 shares' cards say so themselves.
fn answers(k: &KeyGen, order: &[u8]) -> Vec<u8> {
    let c32 = k
        .codex32
        .as_ref()
        .map(|m| crate::codex32::screen::cards(k, m))
        .unwrap_or_default();
    order
        .iter()
        .copied()
        .filter(|&s| {
            if s >= CODEX32_FROM {
                return c32
                    .iter()
                    .any(|(c, card)| CODEX32_FROM + c.index() == s && card.done);
            }
            k.done.get(usize::from(s)).copied().unwrap_or(false) || (s == kstep::KEY && k.locked)
        })
        .collect()
}

/// The step open as the state stands: the one open, or with none open the
/// first not done.
fn current(k: &KeyGen, order: &[u8], answered: &[u8]) -> Option<u8> {
    open_step(k).or_else(|| order.iter().copied().find(|s| !answered.contains(s)))
}

/// The rows of Your choices for the steps of `k` as it stands: one per
/// step, its answer where there is one. Never a secret: a card's summary
/// names counts, sources and fingerprints.
pub(super) fn rows_of(k: &KeyGen) -> Vec<Row> {
    let order = order_of(k);
    let answered = answers(k, &order);
    let open = current(k, &order, &answered);
    let c32 = k
        .codex32
        .as_ref()
        .map(|m| crate::codex32::screen::cards(k, m))
        .unwrap_or_default();
    order
        .into_iter()
        .map(|s| {
            let is_open = open == Some(s);
            if s >= CODEX32_FROM
                && let Some((_, card)) = c32.iter().find(|(c, _)| CODEX32_FROM + c.index() == s)
            {
                return Row {
                    label: Text::of(card.title.as_str()),
                    value: card.done.then(|| Text::of(card.summary.as_str())),
                    mono: card.mono,
                    opens: s,
                    current: is_open,
                };
            }
            // The entries and the quiz show how far they are.
            let live =
                answered.contains(&s) || (is_open && matches!(s, kstep::ENTER | kstep::QUIZ));
            Row {
                label: super::screen::title_of(k, s),
                value: live.then(|| super::screen::summary(k, s)),
                mono: super::screen::mono(k, s),
                opens: s,
                current: is_open,
            }
        })
        .collect()
}

/// The made card's title: a key added or a key made.
pub(super) fn made_title(m: &super::Made) -> &'static str {
    if m.typed {
        EN.keygen_result_added
    } else {
        EN.keygen_result_made
    }
}

impl StepFlow for KeyGenSteps {
    fn steps(&self, app: &Faraday) -> Option<Steps> {
        let k = app.keygen()?;
        // A key made from Tools: the steps it went through, then the made
        // card, past the point the flow cannot take back.
        if let Some(m) = k.made.as_ref() {
            let mut order: Vec<u8> = m.rows.iter().map(|r| r.opens).collect();
            order.push(MADE);
            let mut steps = Steps::new(order, Some(MADE));
            steps.first = Some(MADE);
            steps.answered = steps.order.clone();
            return Some(steps);
        }
        let order = order_of(k);
        let answered = answers(k, &order);
        // A state with no step open opens the first not done.
        let open = current(k, &order, &answered);
        let mut steps = Steps::new(order, open);
        steps.answered = answered;
        steps.first = after_lock(k, &steps.order);
        // Entries being made take no Back: a roller's click under way.
        steps.running = k.roller.rolling();
        Some(steps)
    }

    fn open(&self, _: &Faraday, s: u8) -> Action {
        if s >= CODEX32_FROM {
            Action::Codex32(C32::Card(s - CODEX32_FROM))
        } else {
            Action::KStep(s)
        }
    }

    fn contained(&self) -> Option<&dyn crate::flow::Contained> {
        Some(self)
    }

    fn name(&self, app: &Faraday, s: u8) -> &'static str {
        if s == MADE
            && let Some(m) = app.keygen().and_then(|k| k.made.as_ref())
        {
            return made_title(m);
        }
        if s >= CODEX32_FROM {
            return match crate::codex32::CARDS.get(usize::from(s - CODEX32_FROM)) {
                Some(Card::Scheme) => EN.codex32_scheme,
                Some(Card::Identifier) => EN.codex32_identifier,
                Some(Card::Entropy) => EN.codex32_entropy,
                Some(Card::Enter) => EN.codex32_enter,
                Some(Card::Sheets) => EN.codex32_worksheets,
                Some(Card::Strings) => EN.codex32_strings,
                _ => EN.codex32_strings_made,
            };
        }
        app.keygen()
            .map_or(EN.keygen_back, |k| super::screen::card_name(k, s))
    }
}

impl crate::flow::Contained for KeyGenSteps {
    fn hide_shown(&self, app: &mut Faraday) {
        // The words, the codex32 string's QR and the passphrase field's
        // eye; what was typed stays.
        app.keygen.hide_left_step();
        app.typing.pass_eye.close();
    }

    fn rows(&self, app: &Faraday) -> Vec<Row> {
        let Some(k) = app.keygen() else {
            return Vec::new();
        };
        let Some(m) = k.made.as_ref() else {
            return rows_of(k);
        };
        // What the flow went through, then what it made.
        let mut rows: Vec<Row> = m.rows.clone();
        rows.push(Row {
            label: Text::of(made_title(m)),
            value: Some(crate::wallet::fp_text(faraday_bip::keys::Fingerprint(
                m.fingerprint,
            ))),
            mono: true,
            opens: MADE,
            current: true,
        });
        rows
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }

    /// What the flow's own act code keeps when an earlier answer is given
    /// again: the source stays when only the length changes; the entries
    /// and what follows them are cleared.
    fn still_fits(&self, _: &Faraday, changed: u8, later: u8) -> bool {
        changed == kstep::LENGTH && later == kstep::SOURCE
    }

    /// Make codex32 shares' entry: its dice rolls, tracks and tree take
    /// the whole room (`docs/FLOW-CONTAINER.md` §5).
    fn full_width(&self, app: &Faraday) -> bool {
        app.keygen().is_some_and(|k| {
            k.made.is_none()
                && k.codex32
                    .as_ref()
                    .is_some_and(|m| m.open() == Some(Card::Enter))
        })
    }

    /// Every step stands where the codex32 sheets do, so the card's place
    /// does not move.
    fn wide_content(&self, _: &Faraday) -> bool {
        true
    }

    fn title(&self, app: &Faraday, s: u8) -> Title {
        let Some(k) = app.keygen() else {
            return Title::of(self.name(app, s));
        };
        if s == MADE {
            return Title::of(self.name(app, s));
        }
        if s >= CODEX32_FROM {
            let card = crate::codex32::CARDS.get(usize::from(s - CODEX32_FROM));
            if let (Some(m), Some(&card)) = (k.codex32.as_ref(), card) {
                // Where a character is still to come, the entry stands in
                // the top app bar (F146), and on the card only where the
                // bar has no room (`Page::step`).
                return crate::codex32::screen::title(m, card);
            }
            return Title::of(self.name(app, s));
        }
        super::screen::title_of(k, s)
    }
}
