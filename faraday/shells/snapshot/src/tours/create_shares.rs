//! The createshares tour: Make a new wallet opened on Kind, Single key,
//! backup in shares (codex32) first and ticked, the tooltip on "codex32",
//! More kinds, that kind's Quorum (the codex32 scheme with the Account
//! row) at 2 of 3 and at 3 of 5, Use vaults, and its Keys slot
//! (`docs/CODEX32.md` §6.9).

use crate::tour::Tour;
use faraday_core::create::NewKind;
use faraday_core::{Action, Screen, cstep};
use faraday_shell_api::{App, Event};

pub(crate) fn create_shares_tour(t: &mut Tour) -> Result<(), String> {
    let shares = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Shares)
        .ok_or("no backup in shares")? as u8;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.shot("shares-1-opened")?;
    // The pointer resting on "codex32" brings its tooltip up.
    let (line, (x, y, w, h)) = t
        .app
        .drawn_text_boxes()
        .into_iter()
        .find(|(s, _)| s.ends_with("(codex32)"))
        .ok_or("no backup in shares row")?;
    // The word's middle, by its characters' place in the line drawn.
    let (len, mid) = (line.chars().count() as i32, line.len() as i32 - 5);
    let (x, y) = ((x + w * mid / len) as u16, (y + h / 2) as u16);
    t.app.event(Event::Hover { x, y });
    t.tick();
    t.shot("shares-2-tooltip")?;
    t.app.event(Event::HoverEnd);
    t.press(Action::CMoreKinds);
    t.shot("shares-3-more-kinds")?;
    t.press(Action::CKind(shares));
    t.shot("shares-4-quorum")?;
    t.press(Action::CM(1));
    t.press(Action::CN(1));
    t.press(Action::CN(1));
    t.press(Action::CAccount(0));
    t.shot("shares-5-quorum-3-of-5")?;
    t.press(Action::CNext(cstep::QUORUM));
    t.shot("shares-6-use-vaults")?;
    t.press(Action::CVaults(false));
    t.shot("shares-7-keys")?;
    t.press(Action::CStep(cstep::KIND));
    t.shot("shares-8-kind-line")?;
    Ok(())
}
