//! The boot_place tour.

use crate::tour::Tour;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};

pub(crate) fn boot_place_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::Kind;
    use faraday_core::backup_flow::steps::FlowAction as F;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    t.app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        serial: Some(0x1234_5678),
        files: Vec::new(),
    }]));
    t.pump();
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|x| x.go == faraday_core::catalog::Go::VaultCategory(Kind::Gpg.category()))
        .ok_or("no GPG tile")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    t.press(Action::Flow(F::Continue));
    t.shot("bootplace-1-insert")?;
    let k = t
        .app
        .sticks()
        .iter()
        .position(|s| s.boot)
        .ok_or("no boot stick")?;
    t.press(Action::Flow(F::UseBoot(k)));
    t.pump();
    t.press(Action::Flow(F::Tap));
    t.pump();
    t.shot("bootplace-2-bound")?;
    println!("the boot stick taken for Place 1: Stick 1, the next Stick 2");
    Ok(())
}
