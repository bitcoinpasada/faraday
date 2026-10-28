//! The pictures of `docs/guide/walk/make-a-wallet.md`.

use super::{DICE_WORDS, NAME, fresh_session, kind_at};
use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::create::NewKind;
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, Screen, StorageEvent, cstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, Key};

/// Costs what loading a key from codex32 strings costs. §5.1: New key's
/// Type codex32 strings, the strings typed with a correction: its Key
/// card, then locked in with the key's correction line beside its
/// fingerprint; then the same on Add a key, and Wallets with the line.
pub(super) fn corrected_key(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    fresh_session(t);
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::Codex32.index()));
    corrected_entry(t);
    t.shot_marking("5-1-key-card", Action::KLock)?;
    t.press(Action::KLock);
    t.shot("make-a-wallet-newkey-codex32-key")?;
    fresh_session(t);
    let at = faraday_core::forms::Form::ALL
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Codex32)
        .ok_or("no codex32 form")?;
    t.press(Action::Entry(None));
    t.press(Action::EntryForm(at as u8));
    corrected_entry(t);
    t.app.event(Event::Key(Key::Enter));
    t.pump();
    t.press(Action::Nav(Screen::Wallets));
    t.shot("5-1-wallets-mark")
}

/// Suggest corrections ticked, vector 2's A typed with box 20 wrong, I
/// mistyped and the box typed again from the paper, then C typed.
fn corrected_entry(t: &mut Tour) {
    use faraday_core::codex32_correct::Press;
    t.press(Action::Codex32Correct(Press::Tick));
    let good = NAME[0].as_bytes()[19];
    let mut bad = NAME[0].as_bytes().to_vec();
    bad[19] = if good == b'q' { b'p' } else { b'q' };
    string_in(t, &String::from_utf8_lossy(&bad));
    t.press(Action::Codex32Correct(Press::Mistyped));
    t.app.event(Event::Key(Key::Char(char::from(good))));
    t.pump();
    string_in(t, NAME[1]);
}

/// One string typed into Add a key's codex32 form, then Enter, the
/// keyboard put away.
fn string_in(t: &mut Tour, s: &str) {
    type_text(t, s);
    t.app.event(Event::Key(Key::Enter));
    t.pump();
    t.press(Action::Osk(faraday_core::OskPress::Hide));
}

/// Costs what checking codex32 strings costs. §5.1: Add a key's codex32
/// form: Check a string on vector 2's A with a
/// box changed, its final row and its worksheet; then A added, with Make
/// the rest and Add a share; Make the rest from A and C.
pub(super) fn codex32_form(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::forms::{C32Press, Form};
    fresh_session(t);
    let at = Form::ALL
        .iter()
        .position(|f| *f == Form::Codex32)
        .ok_or("no codex32 form")?;
    t.press(Action::Entry(None));
    t.press(Action::EntryForm(at as u8));
    t.press(Action::Codex32Form(C32Press::Check));
    let mut bad = NAME[0].as_bytes().to_vec();
    bad[19] = if bad[19] == b'q' { b'p' } else { b'q' };
    string_in(t, &String::from_utf8_lossy(&bad));
    t.shot_marking("5-1-check-a-string", Action::Codex32Form(C32Press::Sheet))?;
    t.press(Action::Codex32Form(C32Press::Sheet));
    t.shot_marking("5-1-worksheet", Action::Codex32Form(C32Press::Sheet))?;
    t.press(Action::Codex32Form(C32Press::Sheet));
    t.press(Action::Codex32Form(C32Press::Check));
    string_in(t, NAME[0]);
    t.shot_marking("5-1-codex32", Action::Codex32Form(C32Press::Rest))?;
    t.press(Action::Codex32Form(C32Press::Rest));
    string_in(t, NAME[1]);
    t.shot("5-1-make-the-rest")
}

/// §5.1 to §5.4: a fresh session; the dice key typed back from its
/// paper and test keys 2 and 3 typed; Make a new wallet, a 2-of-3:
/// Kind, Quorum, Keys, Check; the wallet card; its wallet QR.
pub(super) fn multisig(t: &mut Tour) -> Result<(), String> {
    t.restart();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.pump();
    // Wallets' Add a key is New key on Start: Change, Type the words,
    // the words typed, Lock in and Done.
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::Words.index()));
    t.shot("5-1-type-words")?;
    type_text(t, DICE_WORDS);
    t.shot_marking("make-a-wallet-type-words-filled", Action::KNext)?;
    t.press(Action::KNext);
    t.shot_marking("5-1-words-key", Action::KLock)?;
    t.press(Action::KLock);
    if t.app.keygen().is_some() {
        t.press(Action::KNext);
    }
    t.shot("5-1-key-loaded")?;
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    // Multisig moves on to Quorum; Kind, opened again, shows it ticked,
    // and Multisig, pressed again, moves on (F103, F104).
    let multi = kind_at(NewKind::Multi)?;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CStep(cstep::KIND));
    t.shot_marking("5-2-kind", Action::CKind(multi))?;
    t.press(Action::CKind(multi));
    t.shot_marking("5-2-quorum", Action::CNext(cstep::QUORUM))?;
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    t.shot("make-a-wallet-multi-keys")?;
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.shot_marking("5-2-keys", Action::CNext(cstep::KEYS))?;
    t.press(Action::CNext(cstep::KEYS));
    t.shot("5-2-check")?;
    scroll_to(t, Action::CNext(cstep::CHECK));
    t.shot("make-a-wallet-check-addresses")?;
    let w = t
        .app
        .create()
        .and_then(|c| c.built)
        .ok_or("the wallet was not made")?;
    t.press(Action::OpenWallet(w));
    t.shot_marking("5-3-card", Action::QrWallet(w))?;
    t.press(Action::QrWallet(w));
    t.shot_marking("5-4-wallet-qr", Action::Cancel)?;
    t.press(Action::Cancel);
    Ok(())
}

/// §5.5: the 2-of-3's board, the standard way; a share at Place 3 that
/// lets it see the balance; a second seed at Place 1 that lets it spend.
pub(super) fn multisig_board(t: &mut Tour) -> Result<(), String> {
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Backup(w));
    t.shot_marking("5-5-board", Action::Board(BP::Customize))?;
    t.app.press(Action::Board(BP::Customize));
    t.pump();
    t.press(Action::Board(BP::Off(Zone::Paper(2), Thing::Whole)));
    t.app
        .press(Action::Board(BP::Chip(Chip::List(Thing::Share(0)))));
    t.app.press(Action::Board(BP::Zone(Zone::Paper(2))));
    t.pump();
    t.shot_marking("5-5-sees", Action::Board(BP::SeesUndo))?;
    t.shot("back-up-sees")?;
    t.app.press(Action::Board(BP::SeesUndo));
    t.app
        .press(Action::Board(BP::Chip(Chip::List(Thing::Seed(1)))));
    t.app.press(Action::Board(BP::Zone(Zone::Paper(0))));
    t.pump();
    t.shot_marking("5-5-spend", Action::Board(BP::SeesUndo))?;
    t.app.press(Action::Board(BP::SeesUndo));
    t.pump();
    Ok(())
}
