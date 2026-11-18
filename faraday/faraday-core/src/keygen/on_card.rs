//! The making on Make a new wallet's Keys card (F282): New key's state,
//! drawn on the card, so the wallet's first key is made where it is
//! chosen. It opens with BIP-39 and neither count chosen, and ends,
//! with nothing kept, whenever the card is left or its answers change.

use super::{KeyGen, Way};

impl KeyGen {
    /// On the card the dice are the source: after Length the entry stays
    /// open at the new count.
    pub(super) fn card_dice(&mut self) {
        if self.on_card {
            self.choose(Way::DiceWords);
            self.source_next();
        }
    }

    /// Whether the words show as the entry is finished: those that came in
    /// openly do, except on the card, where Show words reveals them.
    pub(super) fn reveals_as_made(&self) -> bool {
        self.reveals() && !self.on_card
    }

    /// Make again on the card: the words, the passphrase and what it was
    /// typed with go.
    pub(super) fn card_forget_words(&mut self) {
        if self.on_card {
            self.passphrase.clear();
            self.passphrase2.clear();
            self.pass_open = false;
            self.focus = None;
            self.shown = false;
        }
    }
}
