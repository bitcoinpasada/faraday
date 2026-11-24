//! The createslots tour: a multisig's Keys step with its cosigner slots as
//! a pick (`docs/WORKSPACE.md` §3): no key held, one key picked of three
//! (the others, with that key dimmed in the next slot's pick), all three
//! filled, and slot 2 opened again with Change.

use crate::tour::Tour;
use faraday_core::create::NewKind;
use faraday_core::{Action, Screen, cstep};

pub(crate) fn create_slots_tour(t: &mut Tour) -> Result<(), String> {
    let multi = NewKind::ALL
        .iter()
        .position(|x| *x == NewKind::Multi)
        .ok_or("no Multisig kind")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CNext(cstep::QUORUM));
    t.shot("slots-1-no-key")?;
    // The keys are loaded, then the wallet is begun again.
    t.press(Action::Nav(Screen::WalletsStart));
    for n in 0..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CNext(cstep::QUORUM));
    let fps: Vec<[u8; 4]> = t.app.session.keys.all().map(|k| k.fingerprint.0).collect();
    if fps.len() < 3 || t.app.create().is_none() {
        return Err("the three keys are not held, or Create closed".into());
    }
    t.shot("slots-2-three-keys")?;
    t.press(Action::CSlotHere(0, fps[0]));
    t.shot("slots-3-one-picked-key-in-slot-1-dimmed")?;
    t.press(Action::CSlotHere(1, fps[1]));
    t.press(Action::CSlotHere(2, fps[2]));
    t.shot("slots-4-all-filled")?;
    t.press(Action::CSlotPick(1));
    t.shot("slots-5-slot-2-reopened")?;
    Ok(())
}
