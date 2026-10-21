//! The visit tour.

use faraday_core::{Action, Screen};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};
use crate::tour::{Tour, scroll};

/// Costs what `classify` costs. Stick visit with a wallet backup's 20
/// files in the Outbox and an unprotected secret: the Outbox list at
/// its top and scrolled to its end by the wheel over it, then the
/// stick's files at their end beside it. On a small panel the page
/// scrolls as one.
pub(crate) fn visit_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::wallet::classify;
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
    names.push(format!("{stem}-vault.ofv"));
    names.push(format!("{stem}-checklist.pdf"));
    names.push(format!("{stem}-recovery.txt"));
    for name in &names {
        let bytes = vec![b'#'; 1800];
        t.app.outbox.push(faraday_core::Item {
            name: name.clone(),
            kind: classify(c, name, &bytes),
            bytes,
            secret: false,
            picture: None,
            sha: None,
            wallet: None,
            psbt: None,
        });
    }
    t.app.outbox.push(faraday_core::Item {
        name: "seed-words.txt".into(),
        kind: faraday_core::wallet::FileKind::Text,
        bytes: b"words".to_vec(),
        secret: true,
        picture: None,
        sha: None,
        wallet: None,
        psbt: None,
    });
    t.sticks(true);
    t.press(Action::Nav(Screen::Visit));
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
        return Ok(());
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
    Ok(())
}
