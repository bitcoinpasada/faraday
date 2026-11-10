//! Check a timestamp in Tools (`docs/decisions/F230.md`): the Proof card
//! with a file to choose, and the Result of a proof checked against its
//! block's header.

use crate::tour::Tour;
use faraday_core::ots_check::OtsAction as O;
use faraday_core::{Action, Screen, StorageEvent};

const V: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tools/vectors/opentimestamps"
);

fn vector(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(format!("{V}/{path}")).map_err(|e| format!("{path}: {e}"))
}

pub(crate) fn ots_check_tour(t: &mut Tour) -> Result<(), String> {
    t.app.storage(StorageEvent::Restored {
        inbox: vec![
            (
                "different-blockchains.txt.ots".to_string(),
                vector("examples/different-blockchains.txt.ots")?,
            ),
            (
                "different-blockchains.txt".to_string(),
                vector("examples/different-blockchains.txt")?,
            ),
            ("455605.hex".to_string(), vector("headers/455605.hex")?),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Check a timestamp")
        .ok_or("no Check a timestamp tile")?;
    t.press(Action::Catalog(tile as u8));
    t.shot("ots-proof")?;
    t.press(Action::OtsCheck(O::Proof(0)));
    t.press(Action::OtsCheck(O::File(1)));
    t.shot("ots-header")?;
    t.press(Action::OtsCheck(O::Header(2)));
    t.shot("ots-result")?;
    Ok(())
}
