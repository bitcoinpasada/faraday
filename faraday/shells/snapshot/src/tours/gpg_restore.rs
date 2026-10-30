//! The gpg-restore tour (`docs/GPG-PAPERKEY.md` §8, Snapshots): a GPG key
//! restored from its paperkey. The sheet of the public keys in Files, one
//! that reads and one refused by its algorithm, and its scanner naming a
//! code that is not a public key; the flow's head card with
//! line 1 of the paperkey; step 4 before a line, with Scan the paperkey,
//! then with a line checked and one being typed;
//! a line in error; every line checked; and a key refused at **Restore the
//! key**. The keys are GnuPG's, from `tools/vectors/paperkey/`: one of
//! Faraday's shape with no passphrase, and an RSA key.

use crate::tour::{Tour, type_text};
use faraday_core::backup_flow::steps::{FlowAction as F, RestorePress as R};
use faraday_core::backup_flow::{BackupFlow, Plan, RestoreFrom, Step};
use faraday_core::{Action, StorageEvent};
use faraday_shell_api::{App, Event, Key as KeyIn};

/// A key of Faraday's shape GnuPG made, its public key and its paperkey.
const CERT: &[u8] = include_bytes!("../../../../../tools/vectors/paperkey/gpg-unprotected.asc");
const PAPER: &str = include_str!("../../../../../tools/vectors/paperkey/gpg-unprotected.txt");
/// An RSA public key GnuPG made: refused by its algorithm.
const RSA: &[u8] = include_bytes!("../../../../../tools/vectors/paperkey/gpg-rsa3072.asc");

pub(crate) fn gpg_restore_tour(t: &mut Tour) -> Result<(), String> {
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    t.app.storage(StorageEvent::Restored {
        inbox: vec![
            ("unprotected.asc".to_string(), CERT.to_vec()),
            ("rsa-key.asc".to_string(), RSA.to_vec()),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.pump();
    t.press(Action::Flow(F::Restore(R::Sheet)));
    t.shot("gpg-restore-1-sheet")?;
    // Scan the public key: the scanner, a frame up, a code not a key.
    t.press(Action::Flow(F::Restore(R::Scan)));
    frame(t);
    t.app.event(Event::Scanned {
        bytes: b"Signed after a restore.".to_vec(),
    });
    t.shot("gpg-restore-1-scan")?;
    t.press(Action::Cancel);
    // The flow on the key, at step 4 with its plan set: here a plan of no
    // vaults, so that Restore the key reaches the key's own check.
    let c = faraday_core::testkit::compute();
    let from = RestoreFrom::read(CERT).map_err(|e| e.clone())?;
    let mut f = BackupFlow::restoring(&c, from);
    f.plan = Plan::default();
    f.step = Step::Make;
    let i = faraday_core::testkit::flow_push(&mut t.app, f);
    t.press(Action::Flow(F::Show(i)));
    t.press(Action::Flow(F::Restore(R::Head)));
    t.shot("gpg-restore-2-head")?;
    t.press(Action::Flow(F::Restore(R::Head)));
    // Step 4 with no line typed: Next line and Scan the paperkey.
    t.shot("gpg-restore-2-step-four")?;
    let lines: Vec<&str> = PAPER
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    type_text(t, lines[0]);
    type_text(t, &lines[1][..20]);
    down(t, 3);
    t.shot("gpg-restore-3-lines")?;
    for _ in 0..20 {
        t.app.event(Event::Key(KeyIn::Backspace));
    }
    type_text(t, &lines[1].replacen(": 00 ", ": 01 ", 1));
    t.app.event(Event::Key(KeyIn::Enter));
    down(t, 6);
    t.shot("gpg-restore-4-error")?;
    for _ in 0..lines[1].len() {
        t.app.event(Event::Key(KeyIn::Backspace));
    }
    for l in &lines[1..] {
        type_text(t, l);
        t.app.event(Event::Key(KeyIn::Enter));
    }
    t.pump();
    t.shot("gpg-restore-5-checked")?;
    t.press(Action::Flow(F::Restore(R::Restore)));
    // Down to the button and the refusal under it.
    down(t, 10);
    t.shot("gpg-restore-6-refused")
}

/// A camera frame: a soft gradient, nothing to read in it.
fn frame(t: &mut Tour) {
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: (0..640u32 * 480).map(|i| ((i % 640) / 3) as u8).collect(),
        chroma: None,
    });
}

/// The page scrolled down `n` notches.
fn down(t: &mut Tour, n: usize) {
    for _ in 0..n {
        t.app.event(Event::Scroll {
            x: 40,
            y: 300,
            dy: 120,
        });
    }
    t.pump();
}
