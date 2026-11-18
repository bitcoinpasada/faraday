//! The quick tour.

use crate::tour::{Tour, scroll_to, type_text, type_words};
use faraday_core::kef_form::KefAct;
use faraday_core::kef_tool::KefToolAction;
use faraday_core::testkit;
use faraday_core::wallet_backup::quick::Quick;
use faraday_core::{Action, Code, cstep};
use faraday_shell_api::{App, Event, Key};

/// The quick paper backup of a single key made here: Make a copy
/// before and after Show the seed, the Blank template sheet, Check
/// my copy, the Krux code's card, and the end card with the first receive address.
pub(crate) fn quick_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Entry(None));
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    t.press(Action::EntryAdd);
    t.press(Action::CreateWallet);
    t.press(Action::CKind(0));
    t.press(Action::CBackupForm(
        faraday_core::create::BackupForm::Words.index(),
    ));
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.shot("quick-1-write")?;
    t.press(Action::BQuick(Quick::Show));
    t.shot("quick-2-write-shown")?;
    t.press(Action::BQuick(Quick::Template));
    t.shot("quick-3-blank-template")?;
    t.press(Action::Cancel);
    t.press(Action::BQuick(Quick::SeedQr));
    t.shot("quick-3b-seedqr")?;
    t.press(Action::BQuick(Quick::Code));
    t.shot("quick-3c-code-added")?;
    t.press(Action::BQuick(Quick::Wrote));
    t.shot("quick-4-check")?;
    type_words(t, None)?;
    t.shot("quick-5-matched")?;
    t.press(Action::BQuick(Quick::Next));
    t.shot("quick-5b-code-card")?;
    let form = |a: KefAct| Action::KefTool(KefToolAction::Backup(a));
    t.press(form(KefAct::Options));
    t.press(form(KefAct::Rounds));
    for _ in 0..8 {
        t.app.event(Event::Key(Key::Backspace));
    }
    type_text(t, "10000");
    t.press(form(KefAct::Back));
    t.press(form(KefAct::Field));
    type_text(t, "a passphrase");
    t.press(form(KefAct::FieldAgain));
    type_text(t, "a passphrase");
    t.press(Action::Osk(faraday_core::OskPress::Hide));
    t.press(form(KefAct::Make));
    testkit::settle(&mut t.app);
    t.tick();
    t.shot("quick-5c-code-shown")?;
    t.press(form(KefAct::Check));
    type_text(t, "a passphrase");
    t.press(form(KefAct::Compare));
    testkit::settle(&mut t.app);
    t.tick();
    t.shot("quick-5d-code-checked")?;
    t.press(Action::BQuick(Quick::Done));
    t.shot("quick-6-checklist-done")?;
    scroll_to(t, Action::ShowCode(Code::Receive(0, 0)));
    t.shot("quick-7-end-card")?;
    Ok(())
}
