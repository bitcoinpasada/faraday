//! The pictures of `docs/guide/walk/restore.md`.

use super::DICE_WORDS;
use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::{Action, Screen};
use faraday_crypto::work::Compute;

/// Costs what restoring costs. §7.1: Restore an existing wallet, from
/// Files.
pub(super) fn restore_wallet(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::rstep;
    use faraday_core::seeds::SeedsAction as S;
    t.press(Action::Nav(Screen::Home));
    t.press(Action::RestoreWallet);
    t.shot_marking("7-1-kind", Action::RFromFiles)?;
    t.press(Action::RFromFiles);
    t.shot("7-1-vaults")?;
    // No moves on to Keys by itself (F103).
    t.press(Action::RVaults(false));
    t.shot("restore-vaults")?;
    if t.app.restore().and_then(|r| r.open) != Some(rstep::SEEDS) {
        return Err("No did not move on to Keys".into());
    }
    t.shot_marking("7-2-keys", Action::RSlotWords)?;
    t.press(Action::RSlotWords);
    t.shot("restore-type")?;
    type_text(t, DICE_WORDS);
    t.press(Action::EntryAdd);
    t.shot_marking("7-2-key-1", Action::RSlotWords)?;
    t.press(Action::RSlotWords);
    type_text(
        t,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    t.press(Action::EntryAdd);
    t.shot("7-2-refused")?;
    let w = t
        .app
        .restore()
        .and_then(|r| r.wallet)
        .ok_or("the description made no wallet")?;
    let empty: Vec<u8> = t
        .app
        .session
        .slots(&t.app.session.wallets[w])
        .iter()
        .enumerate()
        .filter(|(_, s)| s.held_by.is_none())
        .map(|(k, _)| k as u8)
        .collect();
    for k in empty {
        t.press(Action::RAbsent(k));
    }
    t.shot_marking("7-2-not-here", Action::Seeds(S::Make))?;
    t.shot("restore-keys")?;
    t.press(Action::Seeds(S::Make));
    t.shot("7-3-check")?;
    scroll_to(t, Action::RNext(rstep::CHECK));
    t.shot_marking("restore-check-addresses", Action::RNext(rstep::CHECK))?;
    t.press(Action::RNext(rstep::CHECK));
    t.shot("7-3-done")
}
