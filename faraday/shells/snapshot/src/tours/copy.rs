//! The copy tour.

use faraday_core::{Action, StorageEvent, bstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};
use crate::tour::{Tour, copy_digits, scroll_to, type_words};

/// Costs what `copy_digits` costs.
pub(crate) fn copy_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress, Thing, Zone};
    t.load_kit()?;
    t.press(Action::Backup(0));
    // The words and the SeedQR both copied by hand.
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.press(Action::Board(BoardPress::Form(
        Zone::Paper(0),
        Thing::Seed(0),
        1,
    )));
    t.press(Action::BChecklist);
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.app.storage(StorageEvent::Cameras(Vec::new()));
    scroll_to(t, Action::BCheck);
    t.shot("copy-no-camera")?;
    t.press(Action::BCheck);
    type_words(t, Some(3))?;
    t.shot("copy-typing")?;
    t.press(Action::BCheckClear);
    t.app.storage(StorageEvent::Cameras(vec![(
        "/dev/video0".into(),
        "Integrated Camera".into(),
    )]));
    type_words(t, None)?;
    scroll_to(t, Action::BScan);
    t.shot("copy-camera")?;
    t.press(Action::BScan);
    let wrong = copy_digits(c, t, Some(4))?;
    t.app.event(Event::Scanned { bytes: wrong });
    t.shot("copy-scan-differs")?;
    let right = copy_digits(c, t, None)?;
    t.app.event(Event::Scanned { bytes: right });
    scroll_to(t, Action::BScan);
    t.shot("copy-scan-matched")?;
    Ok(())
}
