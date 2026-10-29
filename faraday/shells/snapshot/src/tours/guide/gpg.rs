//! The pictures of `docs/guide/walk/gpg.md`, and the GPG pictures of
//! `docs/guide/screens/vault.md` and `backup-flow.md`; with the backup
//! flow they share with `docs/guide/walk/secure-boot-keys.md`.

use crate::tour::{Files, Tour, type_text};
use faraday_core::backup_flow::Kind;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};
use faraday_shell_api::{App, Event, Key};

/// The vault's passphrase in these walkthroughs.
const PHRASE: &str = "guide vault phrase";

/// The text file signed, as the walkthrough brings it in from a stick.
const NOTES: &[u8] = b"Notes for the guide: a file to sign.\n";

/// Costs what the flow's key and signatures cost. Tools' GPG key with
/// nothing open: the plan, the sticks brought in, Vault 1 made, the key
/// made, the lock and the sticks written; Vault 1 unlocked on the key's
/// page; then a file from the stick signed, and two.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    let notes: Files = vec![
        ("notes.txt".into(), NOTES.to_vec()),
        ("minutes.txt".into(), b"Minutes of the meeting.\n".to_vec()),
    ];
    start(t, Kind::Gpg)?;
    t.shot_marking("gpg-where", Action::Flow(F::Continue))?;
    t.shot("backup-flow-where")?;
    t.press(Action::Flow(F::Continue));
    bring_in(t)?;
    t.shot_marking("gpg-vaults", Action::Flow(F::CreateVault(0)))?;
    create_vault(t)?;
    type_text(t, "Example Signer");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "signer@example.org");
    t.shot_marking("gpg-make", Action::Flow(F::Make))?;
    make(t)?;
    t.shot_marking("gpg-made", Action::Flow(F::MadeContinue))?;
    t.press(Action::Flow(F::MadeContinue));
    lock_and_write(t, notes, Some("gpg-write"))?;
    unlock_done(t)?;
    t.shot("gpg-key-page")?;
    t.shot("vault-gpg-key")?;
    sign(t)
}

/// A file ticked on Sign a file, its sheet; then two files on the sheet.
fn sign(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Vault(V::GpgSignPick));
    let at = |t: &Tour, name: &str| {
        t.app
            .inbox()
            .iter()
            .filter(|i| !i.name.ends_with(".asc"))
            .position(|i| i.name == name)
            .ok_or(format!("{name} is not in Files"))
    };
    // Of two files, none is ticked at first.
    t.press(Action::Vault(V::GpgTick(at(t, "notes.txt")?)));
    t.shot_marking("gpg-sign-a-file", Action::Vault(V::GpgSign))?;
    t.press(Action::Vault(V::GpgSign));
    t.shot_marking("gpg-sign-with", Action::Vault(V::GpgHoldSign))?;
    t.press(Action::Cancel);
    let minutes = at(t, "minutes.txt")?;
    t.press(Action::Vault(V::GpgTick(minutes)));
    t.press(Action::Vault(V::GpgSign));
    t.shot("vault-sign-with")
}

/// The clock as a phone shows it, checked: the key and its signatures
/// carry it. Memory and an unlock time measured, as a PC gives them.
pub(super) fn machine(t: &mut Tour) {
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_793_000_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.pump();
}

/// A fresh stick session with nothing loaded, the clock set, and the
/// Tools tile for `kind` pressed: the flow, on Where it will be kept.
pub(super) fn start(t: &mut Tour, kind: Kind) -> Result<(), String> {
    crate::tours::screens::stick_session(t);
    machine(t);
    let screen = t.app.screen;
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    t.press(Action::Nav(screen));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|x| x.go == faraday_core::catalog::Go::VaultCategory(kind.category()))
        .ok_or("no tile for the key")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    if t.app.screen != Screen::BackupFlow {
        return Err(format!("the tile opened {:?}", t.app.screen));
    }
    Ok(())
}

/// The stick a place keeps, the same each time it is plugged in.
fn stick_of(slot: usize) -> StickInfo {
    StickInfo {
        serial: Some(0x3100 + slot as u32),
        id: format!("place{slot}"),
        label: "KEYSTICK".into(),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }
}

/// The stick's press: a tap, or the hold a stick that reads as another
/// asks for.
fn stick_press(t: &Tour) -> Action {
    let hold = t
        .app
        .flow()
        .and_then(|f| f.live.reading.as_ref())
        .is_some_and(|r| r.hold);
    Action::Flow(if hold { F::Hold } else { F::Tap })
}

/// Bring in the sticks: each place's stick plugged in, taken, pulled.
pub(super) fn bring_in(t: &mut Tour) -> Result<(), String> {
    for _ in 0..6 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.bound.is_none()));
        let Some(slot) = slot else {
            // The last stick in opens Create Vault 1, and the reminders
            // before a secret is typed: put off, back on the Vaults card.
            if t.app.sheet == Some(faraday_core::Sheet::Devices) {
                t.press(Action::Cancel);
            }
            if t.app.screen != Screen::BackupFlow {
                t.press(Action::Vault(V::Back));
            }
            return Ok(());
        };
        t.app.storage(StorageEvent::Sticks(vec![stick_of(slot)]));
        t.pump();
        let press = stick_press(t);
        t.press(press);
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
    }
    Err("the sticks were not all taken".into())
}

/// The Vaults card's Create Vault 1: its passphrase typed twice, made.
pub(super) fn create_vault(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Flow(F::CreateVault(0)));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, PHRASE);
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::BackupFlow && !t.app.vaults().open.is_empty() {
            return Ok(());
        }
        t.tick();
    }
    Err(format!("Vault 1 was not made: {:?}", t.app.screen))
}

/// Make the key pressed, and the key made.
pub(super) fn make(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Flow(F::Make));
    for _ in 0..10 {
        if t.app.backup_flows().last().is_some_and(|f| f.made) {
            return Ok(());
        }
        if t.app.screen == Screen::SetTime {
            t.press(Action::SetTimeKeep);
            t.press(Action::Flow(F::Make));
        }
        t.tick();
    }
    Err("the flow made no key".into())
}

/// Lock and write: the lock, the next process given what it kept and
/// `files` copied in since, and each place's stick written; the shot
/// `first`, if any, once the first is written and pulled.
pub(super) fn lock_and_write(
    t: &mut Tour,
    files: Files,
    first: Option<&str>,
) -> Result<(), String> {
    t.press(Action::Flow(F::LockWrite));
    let (mut inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    inbox.extend(files);
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    machine(t);
    for n in 0..6 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.written.is_none()));
        let Some(slot) = slot else { break };
        t.app.storage(StorageEvent::Sticks(vec![stick_of(slot)]));
        t.pump();
        for _ in 0..2 {
            let press = stick_press(t);
            t.press(press);
        }
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
        if n == 0
            && let Some(name) = first
        {
            t.shot(name)?;
        }
    }
    for _ in 0..5 {
        t.tick();
    }
    if t.app
        .flow()
        .is_some_and(|f| f.slots.iter().any(|s| s.written.is_none()))
    {
        return Err("a stick was not written".into());
    }
    Ok(())
}

/// Done's Unlock Vault 1, its passphrase typed, unlocked: the key's
/// page.
pub(super) fn unlock_done(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Flow(F::UnlockDone(0)));
    type_text(t, PHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() {
            break;
        }
        t.tick();
    }
    crate::tour::not_now_load(t);
    if t.app.screen != Screen::VaultContents {
        return Err(format!("the unlock opened {:?}", t.app.screen));
    }
    Ok(())
}
