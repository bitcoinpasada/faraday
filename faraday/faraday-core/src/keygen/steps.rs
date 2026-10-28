//! New key's steps as Back and Forward walk them (F104): its cards in
//! the order [`KeyGen::steps`] shows them, and inside Make codex32
//! shares that flow's own cards in its place (`docs/CODEX32.md` §6),
//! numbered from [`CODEX32_FROM`]. Its single choices move on inside the
//! flow itself (F103), so `moves_on` is left as the trait has it.

use super::{KeyGen, kstep};
use crate::codex32::{C32, CARDS, Card};
use crate::strings::EN;
use crate::trail::{StepFlow, Steps};
use crate::{Action, Faraday};

/// The step number of Make codex32 shares' first card: card `c` is
/// `CODEX32_FROM + c.index()`.
const CODEX32_FROM: u8 = 100;

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

impl StepFlow for KeyGenSteps {
    fn steps(&self, app: &Faraday) -> Option<Steps> {
        let k = app.keygen()?;
        let mut order = Vec::new();
        for s in k.steps() {
            if s == kstep::CODEX32 && k.codex32.is_some() {
                order.extend(CARDS.iter().map(|c| CODEX32_FROM + c.index()));
            } else {
                order.push(s);
            }
        }
        let mut steps = Steps::new(order, open_step(k));
        steps.first = after_lock(k, &steps.order);
        // Entries being made take no Back: a roller's click under way.
        steps.running =
            k.roller.rolling() || k.codex32.as_ref().is_some_and(|m| m.roller().rolling());
        Some(steps)
    }

    fn open(&self, _: &Faraday, s: u8) -> Action {
        if s >= CODEX32_FROM {
            Action::Codex32(C32::Card(s - CODEX32_FROM))
        } else {
            Action::KStep(s)
        }
    }

    fn name(&self, app: &Faraday, s: u8) -> &'static str {
        if s >= CODEX32_FROM {
            return match CARDS.get(usize::from(s - CODEX32_FROM)) {
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
