//! The restore tour.

use crate::tour::{Tour, type_text};
use faraday_core::Action;
use faraday_core::testkit;

/// Restore a wallet made elsewhere (`docs/history/NEW-WALLET.md` §12): Kind with
/// the description shortcut (Scan it, From Files), Quorum at 2 of 3,
/// Savings' description from Files, then Seeds with test key 1 matched
/// to its key and a seed of no test wallet refused, the other two keys
/// Not here, Check and Done.
pub(crate) fn restore_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::rstep;
    use faraday_core::seeds::SeedsAction as S;
    testkit::move_to(&mut t.app.session, testkit::NET).map_err(|e| e.text())?;
    t.copy_wallets();
    t.sticks(false);
    t.press(Action::RestoreWallet);
    t.shot("restore-kind")?;
    t.press(Action::RKind(4));
    t.press(Action::RNext(rstep::KIND));
    t.shot("restore-quorum")?;
    let i = t
        .app
        .inbox()
        .iter()
        .position(|it| it.name == "savings-wallet.txt")
        .ok_or("no savings-wallet.txt in Files")?;
    t.press(Action::RUse(i));
    // The question of vaults, in its turn before the seeds.
    t.press(Action::RVaults(false));
    t.press(Action::RNext(rstep::WHERE));
    t.press(Action::RSlotWords);
    t.type_key(0);
    t.shot("restore-seeds-matched")?;
    t.press(Action::RSlotWords);
    type_text(
        t,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    t.press(Action::EntryAdd);
    t.shot("restore-seeds-refused")?;
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
    t.shot("restore-seeds-filled")?;
    t.press(Action::Seeds(S::Make));
    t.shot("restore-check")?;
    t.press(Action::RNext(rstep::CHECK));
    t.shot("restore-done")?;
    Ok(())
}
