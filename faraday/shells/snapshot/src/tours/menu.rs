//! The menu tour.

use faraday_core::Action;
use faraday_shell_api::{App, Event, Key};
use crate::tour::{Tour, bput, scroll_to};

/// The ⋮ menu on the backup board (`docs/BACKUP-FLOW.md` §17.20) for
/// Savings, its three keys typed in, on Paper and vault, customised: on
/// `wide` a list chip's Add to…, a placed chip's menu and its Move to…,
/// and the menu opened by the keys; in the narrow column the Add sheet
/// with each chip's ⋮, a chip's Add to… over it, and a placed chip's menu.
pub(crate) fn menu_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress as B, Chip, Thing, Verb, Zone};
    t.load_kit()?;
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    let savings = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Savings")
        .ok_or("no Savings")?;
    t.press(Action::Backup(savings));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::Board(B::Customize));
    let seed = Chip::List(Thing::Seed(0));
    // A chip placed on Place 1's paper, as the preset placed it or put
    // there now.
    let placed = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    let _ = t.app.frame();
    if !t.app.offers(Action::Board(B::Menu(placed))) {
        bput(t, Thing::Seed(0), Zone::Paper(0));
    }
    if t.app.is_compact() {
        t.press(Action::Board(B::OpenList));
        t.shot("menu-narrow-add-sheet")?;
        t.press(Action::Board(B::Menu(seed)));
        t.shot("menu-narrow-add-to")?;
        t.press(Action::Board(B::MenuClose));
        t.press(Action::Cancel);
        scroll_to(t, Action::Board(B::Menu(placed)));
        t.press(Action::Board(B::Menu(placed)));
        t.shot("menu-narrow-placed")?;
        t.press(Action::Board(B::MenuClose));
        return Ok(());
    }
    let _ = t.app.frame();
    t.press(Action::Board(B::Menu(seed)));
    t.shot("menu-add-to")?;
    t.press(Action::Board(B::MenuClose));
    scroll_to(t, Action::Board(B::Menu(placed)));
    t.press(Action::Board(B::Menu(placed)));
    t.shot("menu-placed")?;
    t.press(Action::Board(B::MenuVerb(Verb::Move)));
    t.shot("menu-move-to")?;
    t.press(Action::Board(B::MenuClose));
    // By the keys: Tab to the first ⋮, Enter, and Down twice.
    let _ = t.app.frame();
    t.app.event(Event::Key(Key::Tab));
    t.shot("menu-dot-focused")?;
    t.app.event(Event::Key(Key::Enter));
    t.app.event(Event::Key(Key::Down));
    t.app.event(Event::Key(Key::Down));
    t.shot("menu-keys")?;
    t.app.event(Event::Key(Key::Escape));
    Ok(())
}
