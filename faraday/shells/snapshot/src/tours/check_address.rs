//! The check-address tour (`docs/DESIGN.md` §5, Check an address): the
//! page with nothing loaded, an address typed and found, and one refused.

use crate::tour::{Tour, type_text};
use faraday_bip::policy::WalletPolicy;
use faraday_core::address_check::AddressPress;
use faraday_core::testkit;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_shell_api::{App, Event, Key};

/// The small panel's keyboard put away, so the page under it is seen;
/// nothing where the window has none up.
fn hide_keyboard(t: &mut Tour) {
    t.press(Action::Osk(faraday_core::OskPress::Hide));
}

pub(crate) fn check_address_tour(t: &mut Tour) -> Result<(), String> {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no spending wallet")?;
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Check an address")
        .ok_or("no Check an address tile")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile as u8));
    hide_keyboard(t);
    t.shot("check-address-empty")?;
    t.press(Action::Nav(Screen::Catalog));
    t.load_kit()?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile as u8));
    let c = t.app.compute();
    let policy = WalletPolicy::parse_any(&c, &kit.descriptor).map_err(|e| format!("{e:?}"))?;
    let address = policy
        .address_at(&c, testkit::net(), false, 2)
        .map_err(|e| format!("{e:?}"))?
        .to_string();
    type_text(t, &address);
    t.app.event(Event::Key(Key::Enter));
    t.pump();
    hide_keyboard(t);
    t.shot("check-address-found")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile as u8));
    type_text(t, "hello");
    t.app.event(Event::Key(Key::Enter));
    t.pump();
    hide_keyboard(t);
    t.shot("check-address-refused")?;
    let list = format!(
        "pay to {}\nand {}\n{}\n",
        address_at(t, &policy, 1)?,
        address_at(t, &policy, 4)?,
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
    );
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("addresses.txt".to_string(), list.into_bytes())]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile as u8));
    t.press(Action::AddressCheck(AddressPress::File(0)));
    t.pump();
    hide_keyboard(t);
    t.shot("check-address-list")
}

/// The wallet's receive address at `index`.
fn address_at(t: &mut Tour, policy: &WalletPolicy, index: u32) -> Result<String, String> {
    let c = t.app.compute();
    policy
        .address_at(&c, testkit::net(), false, index)
        .map(|a| a.to_string())
        .map_err(|e| format!("{e:?}"))
}
