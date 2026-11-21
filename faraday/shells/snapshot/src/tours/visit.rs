//! The tour of the stick's pane of Files.

use crate::tour::{Tour, scroll, scroll_to};
use faraday_core::{Action, Screen, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// Costs what `classify` costs. The stick's pane of Files with a wallet backup's 20
/// files in the Outbox and an unprotected secret: the Outbox list at
/// its top and scrolled to its end by the wheel over it, then the
/// stick's files at their end beside it. On a small panel the page
/// scrolls as one.
pub(crate) fn visit_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::wallet::classify;
    import_and_sign(t)?;
    let stem = "family-savings";
    let mut names: Vec<String> = [
        "descriptor.txt",
        "wallet.json",
        "sparrow.txt",
        "coldcard.txt",
        "nunchuk.bsms",
        "core-import.txt",
        "backup.pdf",
    ]
    .iter()
    .map(|e| format!("{stem}-{e}"))
    .collect();
    for k in 1..=3 {
        names.push(format!("{stem}-share-{k}-of-3.pdf"));
        names.push(format!("{stem}-share-{k}-of-3.txt"));
        names.push(format!("{stem}-key-{k}-sheet.pdf"));
    }
    names.push(format!("{stem}-vault.kdbx"));
    names.push(format!("{stem}-checklist.pdf"));
    names.push(format!("{stem}-recovery.txt"));
    for name in &names {
        let bytes = vec![b'#'; 1800];
        t.app.outbox.push(faraday_core::Item {
            name: faraday_text::Text::of(name),
            kind: classify(c, name, &bytes),
            bytes: bytes.into(),
            secret: false,
            picture: None,
            sha: None,
            facts: None,
            wallet: None,
            psbt: None,
            outpoints: Vec::new(),
            unsigned_txid: None,
        });
    }
    t.app.outbox.push(faraday_core::Item {
        name: faraday_text::Text::of("seed-words.txt"),
        kind: faraday_core::wallet::FileKind::Text,
        bytes: b"words".to_vec().into(),
        secret: true,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    t.sticks(true);
    t.press(Action::Nav(Screen::Files));
    t.shot("visit-outbox")?;
    if t.app.is_compact() {
        // Down past the stick's files to the Outbox, then through it.
        for _ in 0..400 {
            let _ = t.app.frame();
            if t.app.offers(Action::VisitOutAll) {
                break;
            }
            scroll(t, 200);
            t.app.settle();
        }
        t.shot("visit-page-outbox")?;
        for k in 1..=4 {
            scroll(t, 500);
            t.shot(&format!("visit-page-outbox-{k}"))?;
        }
        unprotected(t)?;
        not_clean(t)?;
        return same_files(t);
    }
    let _ = t.app.frame();
    let over = t
        .app
        .where_offered(Action::VisitOut(0))
        .ok_or("the Outbox's first file is not on screen")?;
    t.app.event(Event::Wheel {
        x: over.0,
        y: over.1,
        dy: 5000,
    });
    t.shot("visit-outbox-end")?;
    let over = t
        .app
        .where_offered(Action::VisitIn(0))
        .ok_or("the stick's first file is not on screen")?;
    t.app.event(Event::Wheel {
        x: over.0,
        y: over.1,
        dy: 5000,
    });
    t.shot("visit-both-ends")?;
    t.press(Action::VisitOutAll);
    t.press(Action::VisitOutAll);
    t.shot("visit-outbox-none")?;
    unprotected(t)?;
    not_clean(t)?;
    same_files(t)
}

/// The stick's pane of Files once the process is no longer clean: a words file in the
/// Inbox, a key loaded with the stick out, the stick back in with its Lock
/// sheet put off, and Write on the stick's pane asking to lock.
fn not_clean(t: &mut Tour) -> Result<(), String> {
    faraday_core::testkit::inbox_push(
        &mut t.app,
        faraday_core::Item {
            name: faraday_text::Text::of("my-words.txt"),
            kind: faraday_core::wallet::FileKind::Words,
            bytes: b"words".to_vec().into(),
            secret: false,
            picture: None,
            sha: None,
            facts: None,
            wallet: None,
            psbt: None,
            outpoints: Vec::new(),
            unsigned_txid: None,
        },
    );
    t.sticks(false);
    t.press(Action::Entry(None));
    t.type_key(0);
    t.sticks(true);
    if t.app.sheet.is_some() {
        t.press(Action::Cancel);
    }
    t.press(Action::Nav(Screen::Files));
    t.press(Action::VisitWrite);
    t.shot("visit-write-lock")
}

/// The stick's pane once a secret file written to the stick without
/// encryption is on it: the notice naming the stick, with its Delete button.
fn unprotected(t: &mut Tour) -> Result<(), String> {
    let stick = t.sticks.first().ok_or("no stick")?.id.clone();
    t.app.outbox.push(faraday_core::Item {
        name: faraday_text::Text::of("backup-words.txt"),
        kind: faraday_core::wallet::FileKind::Words,
        bytes: b"words".to_vec().into(),
        secret: true,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    t.app.storage(StorageEvent::Written {
        stick,
        name: faraday_text::Text::of("backup-words.txt"),
        wrote_as: faraday_text::Text::of("backup-words.txt"),
    });
    t.press(Action::Nav(Screen::Files));
    if t.app.is_compact() {
        // The page is still scrolled to the Outbox's end: back to its top,
        // where the notice is, then down to it.
        for _ in 0..40 {
            scroll(t, -500);
            t.app.settle();
        }
        scroll_to(t, Action::DeleteUnprotected);
        // The small panel's page runs to the notice, the buttons and the log.
        scroll(t, 5000);
        t.app.settle();
    }
    t.shot("visit-unprotected")
}

/// A stick holding one PSBT, in a clean process: the stick's pane offers
/// Import and sign with the file's name.
fn import_and_sign(t: &mut Tour) -> Result<(), String> {
    t.app
        .storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("one-psbt"),
            label: faraday_text::Text::of("TESTSTICK"),
            boot: false,
            files: vec![(faraday_text::Text::of("payment.psbt"), 1400)],
            vaults: Vec::new(),
        }]));
    t.press(Action::Nav(Screen::Files));
    t.shot("visit-import-and-sign")?;
    if t.app.is_compact() {
        // The small panel's page to its end: Delete files and both buttons.
        scroll(t, 5000);
        t.app.settle();
        t.shot("visit-import-and-sign-end")?;
    }
    t.sticks(false);
    Ok(())
}

/// A file in Faraday and on the stick, the same file: in a fresh process
/// with the stick in, one wallet file is copied in through the Read path and
/// its row is green in both cards; then Files is open again with the same
/// stick, and the other files stay as they were.
fn same_files(t: &mut Tour) -> Result<(), String> {
    t.restart();
    t.sticks(true);
    t.press(Action::Nav(Screen::Files));
    t.shot("visit-same-before")?;
    let names: Vec<faraday_text::Text> = t.stick().files.iter().map(|(n, _)| n.clone()).collect();
    let at = names
        .iter()
        .position(|n| n.ends_with("-wallet.txt"))
        .ok_or("no wallet file on the stick")?;
    t.press(Action::VisitInAll);
    t.press(Action::VisitIn(at));
    t.press(Action::VisitCopy);
    show_same(t, "visit-same-import", at)?;
    // A file written to the stick and read back: its row in the Outbox's
    // card, the receipt and the stick's card.
    let bytes = b"Tour notes\n".to_vec();
    let bytes_on_disk = bytes.clone();
    t.app.outbox.push(faraday_core::Item {
        name: faraday_text::Text::of("tour-notes.txt"),
        kind: faraday_core::wallet::FileKind::Text,
        bytes: bytes.into(),
        secret: false,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    t.press(Action::Nav(Screen::Files));
    // The stick takes the file as a real one does, before it is listed.
    std::fs::write(t.kit.join("tour-notes.txt"), &bytes_on_disk).map_err(|e| e.to_string())?;
    t.press(Action::VisitWrite);
    t.shot("visit-same-write")?;
    t.sticks(true);
    let names: Vec<faraday_text::Text> = t.stick().files.iter().map(|(n, _)| n.clone()).collect();
    let at = names
        .iter()
        .position(|n| &**n == "tour-notes.txt")
        .ok_or("the written file is not on the stick")?;
    show_same(t, "visit-same-written", at)
}

/// The frame `name` of the cards with the stick's file `at` in view.
fn show_same(t: &mut Tour, name: &str, at: usize) -> Result<(), String> {
    if t.app.is_compact() {
        t.press(Action::VisitTab(faraday_core::Column::Stick));
        scroll_to(t, Action::VisitIn(at));
        t.shot(&format!("{name}-stick"))?;
        t.press(Action::VisitTab(faraday_core::Column::Outbox));
    }
    t.shot(name)
}
