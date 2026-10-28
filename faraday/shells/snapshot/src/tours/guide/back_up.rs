//! The pictures of `docs/guide/walk/back-up.md`.

use super::{DICE_WORDS, kind_at, open_item, vault_made, write_sticks};
use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::create::NewKind;
use faraday_core::{Action, Screen, cstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, TouchPhase};

/// The pictures of the sticks written, by the names `write_sticks`
/// gives them.
const STICKS: &[(&str, &str)] = &[
    ("back-up-sticks-insert-0", "4-10-insert"),
    ("back-up-sticks-read-0", "4-10-this-is-the-stick"),
    ("back-up-sticks-written-0", "4-10-written"),
    ("back-up-sticks-done", "4-10-done"),
];

/// Costs what the parts it runs cost. The key just made made a
/// single-key wallet, its board, and its checklist done to the sticks.
pub(super) fn run(c: &Compute, t: &mut Tour) -> Result<(), String> {
    single_key(t)?;
    board(t)?;
    checklist(c, t)
}

/// §4.1: Make a new wallet, a single key from the dice key: Kind, the
/// vault question, Keys, Check, and its Back up step.
fn single_key(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    // Create opens on Kind (`docs/CODEX32.md` §6.9); a kind pressed
    // moves on (F103).
    t.shot_marking("4-1-kind", Action::CKind(kind_at(NewKind::NativeSegwit)?))?;
    t.press(Action::CKind(kind_at(NewKind::NativeSegwit)?));
    t.shot("4-1-use-vaults")?;
    // No moves on to Keys by itself.
    t.press(Action::CVaults(false));
    t.shot("back-up-single-keys")?;
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.shot_marking("4-1-keys", Action::CNext(cstep::KEYS))?;
    t.press(Action::CNext(cstep::KEYS));
    t.shot("4-1-check")?;
    // Lock while the wallet waits for its backup: the sheet asks.
    t.press(Action::LockAsk);
    t.shot_marking("2-3-lock", Action::Cancel)?;
    t.press(Action::Cancel);
    t.press(Action::CNext(cstep::CHECK));
    t.shot_marking("4-1-back-up", Action::CBackup)?;
    t.press(Action::CBackup);
    t.shot_marking("4-2-board-standard", Action::Board(BP::Customize))?;
    t.shot("back-up-board")?;
    Ok(())
}

/// A finger on the panel at `at`, as `phase`.
fn touch(t: &mut Tour, (x, y): (u16, u16), phase: TouchPhase) {
    t.app.event(Event::Touch { x, y, phase });
    t.pump();
}

/// `chip` dragged by a finger onto `zone`: a shot on the way, over the
/// zone, and once dropped.
fn drag(t: &mut Tour, chip: Chip, zone: Zone, over: &str, dropped: &str) -> Result<(), String> {
    let _ = t.app.frame();
    let from = t
        .app
        .where_offered(Action::Board(BP::Chip(chip)))
        .ok_or("the chip is not on screen")?;
    let to = t
        .app
        .where_offered(Action::Board(BP::Zone(zone)))
        .ok_or("the zone is not on screen")?;
    touch(t, from, TouchPhase::Down);
    let step =
        |a: u16, b: u16, k: i32| (i32::from(a) + (i32::from(b) - i32::from(a)) * k / 4) as u16;
    for k in 1..=4 {
        touch(
            t,
            (step(from.0, to.0, k), step(from.1, to.1, k)),
            TouchPhase::Move,
        );
    }
    t.shot(over)?;
    touch(t, to, TouchPhase::Up);
    t.shot(dropped)
}

/// §4.2 to §4.5: the board after Customize backup; a new vault dragged
/// onto Place 1; the seed's ⋮ menu putting it in the vault;
/// the description put there too; the SeedQR at Place 1 turned on.
fn board(t: &mut Tour) -> Result<(), String> {
    t.app.press(Action::Board(BP::Customize));
    t.pump();
    t.shot_marking("4-3-customize", Action::Board(BP::AddVault))?;
    t.press(Action::Board(BP::AddVault));
    t.shot("back-up-board-vault-added")?;
    drag(
        t,
        Chip::List(Thing::Vault(0)),
        Zone::Card(0),
        "4-4-drag",
        "4-4-dropped",
    )?;
    t.press(Action::Board(BP::Menu(Chip::List(Thing::Seed(0)))));
    t.shot_marking("4-5-menu", Action::Board(BP::MenuTo(Zone::Vault(0, 0, 0))))?;
    t.shot("back-up-menu")?;
    t.press(Action::Board(BP::MenuTo(Zone::Vault(0, 0, 0))));
    t.shot_marking(
        "back-up-board-seed-in-vault",
        Action::Board(BP::Chip(Chip::List(Thing::Whole))),
    )?;
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Whole))));
    t.press(Action::Board(BP::Zone(Zone::Vault(0, 0, 0))));
    let seed = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    t.press(Action::Board(BP::Chip(seed)));
    t.press(Action::Board(BP::Form(Zone::Paper(0), Thing::Seed(0), 1)));
    t.press(Action::Board(BP::LetGo));
    t.shot("4-5-board-done")?;
    scroll_to(t, Action::BChecklist);
    t.shot("back-up-board-plan")
}

/// Costs a mnemonic's checksum. §4.6 to §4.9: the checklist; the blank template; the copy by hand
/// and Check my copy; the vault made and saved into; the sticks written.
fn checklist(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::bstep;
    use faraday_core::vault::VaultAction as V;
    t.press(Action::BChecklist);
    t.shot_marking("4-6-checklist", Action::BNext(bstep::BLANK))?;
    t.press(Action::BNext(bstep::BLANK));
    t.shot_marking("4-7-copy", Action::BReveal)?;
    t.press(Action::BReveal);
    t.shot_marking("4-7-copy-shown", Action::BReveal)?;
    t.press(Action::BReveal);
    t.press(Action::BCheck);
    t.shot("back-up-check-copy")?;
    crate::tour::type_words(t, Some(5))?;
    t.shot("4-7-check-copy")?;
    for w in DICE_WORDS.split_whitespace().skip(5) {
        type_text(t, &format!("{w} "));
    }
    // The words checked are the item done here, with no camera yet, and
    // the checklist moves on (C8); the item, opened again, takes the scan.
    if open_item(t) != Some(bstep::COPY) {
        t.press(Action::BStep(bstep::COPY));
    }
    t.shot("back-up-check-copy-words")?;
    // The SeedQR copy, scanned.
    t.press(Action::BScan);
    crate::tours::site_tour::camera(t);
    t.shot("back-up-check-copy-scan")?;
    t.app.event(Event::Scanned {
        bytes: crate::tour::copy_digits(c, t, None)?,
    });
    t.pump();
    t.shot("4-7-matched")?;
    // An item done moves on by itself (C8): to the vault's, here.
    if open_item(t) != Some(bstep::VAULT) {
        t.press(Action::BStep(bstep::VAULT));
    }
    t.shot_marking(
        "4-8-vault-item",
        Action::Vault(V::CreateFrom(Screen::Backup)),
    )?;
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    t.shot("4-8-create-vault")?;
    vault_made(t);
    t.shot_marking("4-8-save", Action::BVaultSave(0))?;
    t.press(Action::BVaultSave(0));
    t.shot("back-up-checklist-vault-saved")?;
    if open_item(t) == Some(bstep::VAULT) {
        t.press(Action::BNext(bstep::VAULT));
    }
    t.shot("back-up-checklist-next")?;
    if open_item(t) != Some(bstep::ENVELOPE) {
        t.press(Action::BStep(bstep::ENVELOPE));
    }
    t.shot_marking("4-9-envelope", Action::BNext(bstep::ENVELOPE))?;
    t.press(Action::BNext(bstep::ENVELOPE));
    t.shot_marking(
        "4-10-lock-and-write",
        Action::Flow(faraday_core::backup_flow::steps::FlowAction::WalletLock),
    )?;
    write_sticks(t, "back-up-sticks", STICKS)
}
