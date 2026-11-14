//! The address check's read card while it looks further (`docs/QR.md`
//! §2): what it says of the look, its ways over, and its refresh from the [`Look`] it holds.

use super::{Account, Look, LookWay};
use crate::scan_read::{ReadCard, Way};
use crate::strings::EN;
use crate::wallet::Session;
use faraday_text::Text;

/// What a miss says: how far it looked, in how many wallets.
pub(crate) fn missed(look: &Look) -> Text {
    let looked = Text::num(u64::from(look.looked));
    if look.account_count() > 0 {
        Text::fill(
            EN.address_note_missed_accounts,
            &[
                &looked,
                &Text::num((look.wallets + look.account_count()) as u64),
            ],
        )
    } else if look.wallets == 1 {
        Text::fill(EN.address_note_missed_one, &[&looked])
    } else {
        Text::fill(
            EN.address_note_missed,
            &[&looked, &Text::num(look.wallets as u64)],
        )
    }
}

/// What a find in a standard account of a loaded key says: the key, its
/// account's path and script, the chain and the index.
pub(crate) fn found_in_account(a: &Account, chain: &str, index: u32) -> Text {
    Text::fill(
        EN.address_note_found_account,
        &[
            &a.label,
            &crate::wallet::fp_text(a.fingerprint),
            &a.path,
            a.script,
            chain,
            &Text::num(u64::from(index)),
        ],
    )
}

/// The card's line and ways made again from the look it holds.
pub(crate) fn refresh(card: &mut ReadCard, session: &Session) {
    let Some(look) = card.look.as_ref() else {
        return;
    };
    let (line, ways) = if let Some((w, change, index)) = look.found {
        let chain = if change {
            EN.address_note_change
        } else {
            EN.address_note_receive
        };
        let line = if let Some(a) = look.account(w) {
            found_in_account(a, chain, index)
        } else {
            let name = session
                .wallets
                .get(w)
                .map_or_else(Text::default, |w| w.name.clone());
            Text::fill(
                EN.address_note_found,
                &[&name, chain, &Text::num(u64::from(index))],
            )
        };
        (line, Vec::new())
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
