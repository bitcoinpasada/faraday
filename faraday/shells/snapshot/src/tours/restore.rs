//! The restore tour.

use crate::tour::{Tour, scroll, type_text};
use faraday_core::Action;
use faraday_core::testkit;
use faraday_shell_api::{App, Event, Key};

/// Restore a wallet made elsewhere: Kind with
/// the description shortcut (Scan it, From Files), Quorum at 2 of 3,
/// Savings' description from Files, then Seeds with test key 1 matched
/// to its key and a seed of no test wallet refused, the other two keys
/// Not here, Check and Done; then one key from its seed alone, at account 1.
pub(crate) fn restore_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::rstep;
    use faraday_core::seeds::SeedsAction as S;
    testkit::move_to(&mut t.app.session, testkit::net()).map_err(|e| String::from(&*e.text()))?;
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
    faraday_core::testkit::confirm_wallets(&mut t.app);
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
    // The refusal stays on Add a key with the words kept: back out.
    t.app.event(faraday_shell_api::Event::Key(
        faraday_shell_api::Key::Escape,
    ));
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
    faraday_core::testkit::confirm_wallets(&mut t.app);
    t.shot("restore-check")?;
    t.press(Action::RNext(rstep::CHECK));
    t.shot("restore-done")?;
    // One key from its seed alone: the account stepper, then the Check
    // card saying the account and script the seed alone gives.
    t.press(Action::RestoreWallet);
    t.press(Action::RNext(rstep::KIND));
    t.press(Action::RSeeds);
    t.press(Action::RVaults(false));
    t.press(Action::RNext(rstep::WHERE));
    t.press(Action::RSlotWords);
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    t.press(Action::EntryAdd);
    t.press(Action::Seeds(S::Account(true)));
    scroll(t, 300);
    t.shot("restore-seed-alone-keys")?;
    // A custom path with nothing typed gives no wallet and says why.
    t.press(Action::Seeds(S::Custom));
    for _ in 0..20 {
        t.app.event(Event::Key(Key::Backspace));
    }
    t.pump();
    scroll(t, 300);
    t.shot("restore-seed-alone-bad-path")?;
    t.press(Action::Seeds(S::Standard));
    t.press(Action::Seeds(S::Make));
    scroll(t, 600);
    t.shot("restore-seed-alone-check")?;
    Ok(())
}
