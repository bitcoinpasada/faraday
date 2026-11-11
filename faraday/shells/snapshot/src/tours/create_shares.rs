//! The createshares tour: Make a new wallet opened on Kind, nothing
//! ticked (who signs: Single key, Multisig, More kinds); Single key's
//! Backup card, nothing ticked, and the tooltip on "codex32"; Shares
//! (codex32) chosen, Use vaults, then New key's Start with Make a new
//! key fixed to Codex32 and every way to add a key you have, and Make
//! codex32 shares' Scheme with no "None, the secret alone"; and a Taproot
//! single key's Backup card, reached by More kinds (F240,
//! `docs/CODEX32.md` §6.9).

use crate::tour::Tour;
use faraday_core::create::{BackupForm, NewKind};
use faraday_core::keygen::Start;
use faraday_core::{Action, Screen, cstep};
use faraday_shell_api::{App, Event};

pub(crate) fn create_shares_tour(t: &mut Tour) -> Result<(), String> {
    let kind = |k: NewKind| {
        NewKind::ALL
            .iter()
            .position(|x| *x == k)
            .ok_or("no such kind")
            .map(|i| i as u8)
    };
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.shot("shares-1-opened")?;
    t.press(Action::CKind(kind(NewKind::NativeSegwit)?));
    t.shot("shares-2-backup")?;
    // The pointer resting on "codex32" brings its tooltip up.
    let (line, (x, y, w, h)) = t
        .app
        .drawn_text_boxes()
        .into_iter()
        .find(|(s, _)| s.ends_with("(codex32)"))
        .ok_or("no Shares (codex32) row")?;
    // The word's middle, by its characters' place in the line drawn.
    let (len, mid) = (line.chars().count() as i32, line.len() as i32 - 5);
    let (x, y) = ((x + w * mid / len) as u16, (y + h / 2) as u16);
    t.app.event(Event::Hover { x, y });
    t.tick();
    t.shot("shares-3-tooltip")?;
    t.app.event(Event::HoverEnd);
    t.press(Action::CBackupForm(BackupForm::Codex32.index()));
    t.shot("shares-4-use-vaults")?;
    // With no key loaded, Keys is passed over: New key opens on Start.
    t.press(Action::CVaults(false));
    if t.app.screen != Screen::KeyGen {
        return Err("Use vaults' No opened no New key".into());
    }
    t.shot("shares-5-start")?;
    t.press(Action::KStart(Start::MakeCodex32.index()));
    t.shot("shares-6-scheme")?;
    // Cancel, nothing entered, back on Make a new wallet: Kind changed to
    // a Taproot single key keeps its Backup card. The kind moves on to
    // New key again, its one way on; Cancel once more.
    t.press(Action::CancelFlow);
    t.press(Action::CStep(cstep::KIND));
    t.press(Action::CMoreKinds);
    t.shot("shares-7-more-kinds")?;
    t.press(Action::CKind(kind(NewKind::Taproot)?));
    if t.app.screen == Screen::KeyGen {
        t.press(Action::CancelFlow);
    }
    t.press(Action::CStep(cstep::BACKUP_FORM));
    t.shot("shares-8-taproot-backup")?;
    Ok(())
}
