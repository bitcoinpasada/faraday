//! The codex32 backup tour (`docs/CODEX32.md` §9,
//! `docs/BACKUP-FLOW.md` §17.23): a key with a set opening on one string
//! per place; a key of words turned to codex32 strings on the board;
//! the checklist's item making them and a string typed back; the set's
//! actions; the copies by hand's one codex32 choice.

use crate::tour::{Tour, type_text};
use faraday_core::Action;
use faraday_core::codex32_backup::{Press, SetPress};
use faraday_shell_api::{App, Event, Key};

/// Vector 2's shares A and C (BIP 93).
const A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";

/// A single-key wallet made by Create over session key `k`, its Back up
/// opened on the board.
fn wallet_of(t: &mut Tour, k: usize) -> Result<(), String> {
    use faraday_core::cstep;
    t.press(Action::CreateWallet);
    t.press(Action::CNext(cstep::KIND));
    let fp = t.app.session.keys[k].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Backup(w));
    Ok(())
}

fn c32(t: &mut Tour, p: Press) {
    t.press(Action::BC32(p));
}

pub(crate) fn codex32_backup_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::codex32::C32Board;
    use faraday_core::board::{BoardPress as B, Thing, Zone};
    use faraday_core::{bstep, plan};
    use faraday_vault::codex32::{SetFacts, State};
    // A key loaded from two strings of vector 2, its set told.
    t.press(Action::Entry(None));
    t.press(Action::EntryForm(2));
    for s in [A, C] {
        type_text(t, s);
        t.app.event(Event::Key(Key::Enter));
    }
    t.app.event(Event::Key(Key::Enter));
    t.pump();
    t.app.session.keys.set_codex32(
        0,
        Some(SetFacts {
            threshold: 2,
            identifier: *b"name",
            length: 48,
            unknown: 0,
            strings: vec![
                (b'a', State::Made),
                (b'c', State::Made),
                (b'd', State::Made),
            ],
        }),
    );
    wallet_of(t, 0)?;
    t.shot("c32b-1-standard-way")?;
    t.press(Action::BChecklist);
    t.press(Action::BNext(bstep::BLANK));
    let n = bstep::of(plan::Item::Codex32(0));
    if t.app.backup().and_then(|b| b.open) != Some(n) {
        t.press(Action::BStep(n));
    }
    t.shot("c32b-2-item-list")?;
    let a = plan::codex32::index_of(b'a').ok_or("index")? as u8;
    c32(t, Press::Open(a));
    for ch in A[3..].chars() {
        c32(t, Press::Key(faraday_core::codex32::OskPress::Char(ch)));
    }
    t.shot("c32b-3-typed-back")?;
    c32(t, Press::List);
    c32(t, Press::Set(SetPress::Rotate));
    c32(
        t,
        Press::Set(SetPress::Mark(
            plan::codex32::index_of(b'c').ok_or("index")? as u8,
            1,
        )),
    );
    t.shot("c32b-4-rotate-marks")?;
    c32(t, Press::Set(SetPress::Counted));
    t.shot("c32b-5-rotate-destroy")?;
    c32(t, Press::Set(SetPress::Leave));
    // A key of words: its seed turned to strings the plan makes.
    t.press(Action::Entry(None));
    type_text(
        t,
        &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0),
    );
    t.press(Action::EntryAdd);
    wallet_of(t, 1)?;
    t.press(Action::Board(B::Customize));
    t.press(Action::Board(B::C32(0, C32Board::On)));
    t.shot("c32b-6-board-group")?;
    t.press(Action::Board(B::AddPlace));
    t.press(Action::Board(B::AddPlace));
    t.press(Action::Board(B::Off(Zone::Paper(0), Thing::Seed(0))));
    let (_, ans) = t.app.board_plan().ok_or("a board")?;
    let idx = ans.c32(0).ok_or("a form")?.indices.clone();
    for (p, j) in idx.into_iter().enumerate() {
        t.press(Action::Board(B::Chip(faraday_core::board::Chip::List(
            Thing::String(0, j),
        ))));
        t.press(Action::Board(B::Zone(Zone::Paper(p))));
    }
    t.shot("c32b-7-board-placed")?;
    t.press(Action::BChecklist);
    t.shot("c32b-8-templates")?;
    t.press(Action::BNext(bstep::BLANK));
    if t.app.backup().and_then(|b| b.open) != Some(n) {
        t.press(Action::BStep(n));
    }
    c32(t, Press::Make);
    t.shot("c32b-9-string-made")
}
