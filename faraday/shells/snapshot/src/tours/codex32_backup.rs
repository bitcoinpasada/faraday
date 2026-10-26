//! The codex32 backup tour (`docs/CODEX32.md` §9,
//! `docs/BACKUP-FLOW.md` §17.23): a key with a set opening on one string
//! per place; the checklist's item checking each string typed back, its
//! Codex32QR drawn by hand; the set's actions.

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
    t.press(Action::CKind(0));
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

/// String A kept at the board's first place as its Codex32QR drawn by
/// hand, not written (`docs/CODEX32.md` §9.5). Returns its index.
fn drawn_by_hand(t: &mut Tour) -> Result<usize, String> {
    use faraday_core::board::{BoardPress as B, Thing, Zone};
    let a = faraday_core::plan::codex32::index_of(b'a').ok_or("index")?;
    t.press(Action::Board(B::Customize));
    for k in [1, 0] {
        t.press(Action::Board(B::Form(
            Zone::Paper(0),
            Thing::String(0, a),
            k,
        )));
    }
    Ok(a)
}

fn c32(t: &mut Tour, p: Press) {
    t.press(Action::BC32(p));
}

pub(crate) fn codex32_backup_tour(t: &mut Tour) -> Result<(), String> {
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
    let a = drawn_by_hand(t)?;
    t.press(Action::BChecklist);
    t.shot("c32b-1b-templates-codex32qr")?;
    t.press(Action::BNext(bstep::BLANK));
    let n = bstep::of(plan::Item::Codex32(0));
    if t.app.backup().and_then(|b| b.open) != Some(n) {
        t.press(Action::BStep(n));
    }
    t.shot("c32b-2-item-list")?;
    let a = a as u8;
    c32(t, Press::Open(a));
    for ch in A[3..].chars() {
        c32(t, Press::Key(faraday_core::codex32::OskPress::Char(ch)));
    }
    // Typed from its paper: its Codex32QR to draw, block by block.
    t.shot("c32b-3-typed-back")?;
    c32(t, Press::BlockAt(8));
    crate::tour::scroll_to(t, Action::BC32(Press::ScanCopy));
    t.shot("c32b-3b-codex32qr-block")?;
    c32(t, Press::Rows(true));
    t.shot("c32b-3c-codex32qr-rows")?;
    c32(t, Press::Rows(false));
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
    Ok(())
}
