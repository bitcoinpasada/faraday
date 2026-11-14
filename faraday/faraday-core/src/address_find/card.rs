//! The address check's read card while it looks further (`docs/QR.md`
//! §2): what it says of the look, its ways over, and the glue from a press
//! and a tick to the [`Look`] it holds.

use super::{Look, LookWay};
use crate::Faraday;
use crate::scan_read::{ReadCard, Way};
use crate::strings::EN;
use crate::wallet::Session;
use faraday_text::Text;

/// What a miss says: how far it looked, in how many wallets.
pub(crate) fn missed(look: &Look) -> Text {
    let looked = Text::num(u64::from(look.looked));
    if look.wallets == 1 {
        Text::fill(EN.address_note_missed_one, &[&looked])
    } else {
        Text::fill(
            EN.address_note_missed,
            &[&looked, &Text::num(look.wallets as u64)],
        )
    }
}

/// The card's line and ways made again from the look it holds.
pub(crate) fn refresh(card: &mut ReadCard, session: &Session) {
    let Some(look) = card.look.as_ref() else {
        return;
    };
    let (line, ways) = if let Some((w, change, index)) = look.found {
        let name = session
            .wallets
            .get(w)
            .map_or_else(Text::default, |w| w.name.clone());
        let chain = if change {
            EN.address_note_change
        } else {
            EN.address_note_receive
        };
        (
            Text::fill(
                EN.address_note_found,
                &[&name, chain, &Text::num(u64::from(index))],
            ),
            Vec::new(),
        )
    } else if let Some((done, depth)) = look.progress() {
        (
            Text::fill(
                EN.read_look_progress,
                &[&Text::num(u64::from(done)), &Text::num(u64::from(depth))],
            ),
            vec![Way::Look(LookWay::Stop)],
        )
    } else {
        (
            missed(look),
            look.offered()
                .map(|(i, _)| Way::Look(LookWay::To(i)))
                .collect(),
        )
    };
    card.facts = vec![line];
    card.ways = ways;
}

impl Faraday {
    /// The read card of an address that is none of the first hundred of
    /// any loaded wallet: its line, and **Look further** to each depth.
    pub(crate) fn address_miss_card(&mut self, card: &mut ReadCard, look: Look) {
        card.look = Some(look);
        refresh(card, &self.session);
    }

    /// A press on **Look further** or **Stop**.
    pub(crate) fn look_go(&mut self, way: LookWay) {
        let now = self.now_ms;
        let Some(card) = self.camera.card_mut() else {
            return;
        };
        if let Some(look) = card.look.as_mut() {
            look.press(way, now);
        }
        refresh(card, &self.session);
    }

    /// Costs an address derivation for each index of the tick, on both
    /// chains of every loaded wallet. One tick of the look under way on
    /// the open read card. Returns whether to draw.
    pub(crate) fn look_tick(&mut self, now_ms: u64) -> bool {
        let running = self
            .camera
            .scan()
            .and_then(|s| s.card.as_ref())
            .and_then(|k| k.look.as_ref())
            .is_some_and(Look::running);
        if !running {
            return false;
        }
        let c = self.compute();
        let session = &self.session;
        let policies: Vec<_> = session.wallets.iter().map(|w| w.policy()).collect();
        let Some(card) = self.camera.card_mut() else {
            return false;
        };
        if let Some(look) = card.look.as_mut() {
            look.tick(&c, &policies, now_ms);
        }
        refresh(card, session);
        true
    }
}
