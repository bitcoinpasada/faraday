//! The seeds tour.

use crate::tour::{Tour, scroll, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event, Key};

/// A wallet from seed words alone: Restore on a 2-of-3 with no
/// description, two seeds typed into their slots, the third cosigner's
/// xpub in the last, and the Check card;
/// then the Spend tab's What do you have? (`docs/history/SCAN-TO-SIGN.md` §4):
/// empty, after one seed, after two, the intake scanner asking for the
/// description, the shape made by hand, and Check the money guessed,
/// after No money shows (§5).
pub(crate) fn seeds_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::{FamilyAction as F, page};
    use faraday_core::seeds::{Focus, SLIDE_N, SeedsAction as S};
    t.press(Action::Lock);
    t.sticks(false);
    t.tick();
    testkit::move_to(&mut t.app.session, testkit::net()).map_err(|e| e.text())?;
    t.press(Action::RestoreWallet);
    t.press(Action::RKind(4));
    t.press(Action::RNext(faraday_core::rstep::KIND));
    t.press(Action::RNext(faraday_core::rstep::QUORUM));
    t.shot("restore-type-seeds")?;
    t.press(Action::RSeeds);
    t.press(Action::RVaults(false));
    t.press(Action::RNext(faraday_core::rstep::WHERE));
    for n in [0, 1] {
        t.press(Action::RSlotWords);
        t.type_key(n);
    }
    if t.app.screen != Screen::Restore {
        return Err("Add a key did not return to Restore".into());
    }
    t.shot("restore-seeds-first")?;
    t.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(t, &testkit::key(2, "m/48'/1'/0'/2'"));
    t.app.event(Event::Key(Key::Enter));
    t.shot("restore-seeds-shape")?;
    t.press(Action::Seeds(S::Make));
    t.shot("restore-seeds-check")?;
    if t.app.restore().and_then(|r| r.wallet).is_none() {
        return Err("the seeds made no wallet".into());
    }
    // The Spend tab: words, then another seed, then how many sign.
    t.press(Action::Lock);
    // The process after the lock has its clock from the next tick.
    t.tick();
    testkit::move_to(&mut t.app.session, testkit::net()).map_err(|e| e.text())?;
    t.press(Action::Nav(Screen::Family));
    t.press(Action::Family(F::StartOver));
    t.press(Action::Family(F::Next(page::MAP)));
    t.press(Action::Family(F::Next(page::SAFE)));
    t.shot("spend-intake")?;
    t.press(Action::Family(F::TypeWords));
    t.type_key(0);
    t.shot("spend-words-one-seed")?;
    t.press(Action::Family(F::TypeWords));
    t.type_key(1);
    t.shot("spend-words-two-seeds")?;
    t.press(Action::Family(F::Scan));
    t.shot("spend-intake-scanner")?;
    t.press(Action::Cancel);
    t.press(Action::Family(F::NoDescription));
    t.press(Action::Slide(SLIDE_N, 3));
    t.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(t, &testkit::key(2, "m/48'/1'/0'/2'"));
    t.app.event(Event::Key(Key::Enter));
    // Down the shape to Make the wallet: further on a small panel.
    let steps = if t.size().0 < 700 { 5 } else { 2 };
    for k in 1..=steps {
        scroll(t, 300);
        t.shot(&format!("spend-words-shape-{k}"))?;
    }
    t.press(Action::Seeds(S::Make));
    t.shot("spend-words-made")?;
    if t.app.family_wallet().is_none() {
        return Err("the Spend tab's seeds made no wallet".into());
    }
    // Made without its description: No money shows tries the next kind.
    t.press(Action::Family(F::NoMoney));
    t.shot("spend-guess-multi")?;
    // One seed and That's everything: native SegWit, guessed.
    t.press(Action::Family(F::StartOver));
    t.press(Action::Family(F::TypeWords));
    t.type_key(0);
    t.press(Action::Family(F::Everything));
    t.shot("spend-guess-check")?;
    t.press(Action::Family(F::NoMoney));
    t.shot("spend-guess-passphrase")?;
    t.press(Action::Family(F::Passphrase(false)));
    t.shot("spend-guess-taproot")?;
    if t.app.family_guess().is_none_or(|g| g.at != 2) {
        return Err("No money shows did not go on to Taproot".into());
    }
    t.press(Action::Lock);
    Ok(())
}
