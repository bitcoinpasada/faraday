//! The board tour.

use crate::tour::{Tour, bput, scroll, scroll_to, type_text, vault_place, vault_stick};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent, qstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event, Key};

/// Costs a map of the plan (through `vault_place` and `vault_stick`).
/// The backup board (`docs/BACKUP-FLOW.md` §17) for Savings, its three
/// keys typed in: the board on Paper and vault, Seed 1 selected with
/// the zones that take it edged, its words on Place 1's paper with
/// their forms open, a share on paper and in Vault 1, and the software
/// place.
pub(crate) fn board_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress as B, Chip, Thing, Zone};
    let was = t.app.theme();
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
    // The preset opens the board.
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.shot("board-paper-and-vault")?;
    t.press(Action::Board(B::Chip(Chip::List(Thing::Seed(0)))));
    t.shot("board-seed-selected")?;
    t.press(Action::Board(B::Zone(Zone::Paper(1))));
    t.press(Action::Board(B::Chip(Chip::Placed(
        Zone::Paper(1),
        Thing::Seed(0),
    ))));
    t.shot("board-seed-forms")?;
    t.press(Action::Board(B::Form(Zone::Paper(1), Thing::Seed(0), 1)));
    t.press(Action::Board(B::LetGo));
    t.press(Action::Board(B::Chip(Chip::List(Thing::Share(0)))));
    t.press(Action::Board(B::Zone(Zone::Paper(2))));
    t.press(Action::Board(B::Chip(Chip::List(Thing::Share(1)))));
    t.press(Action::Board(B::Zone(Zone::Vault(0, 0, 0))));
    t.press(Action::Board(B::Chip(Chip::List(Thing::Software))));
    t.shot("board-shares-and-software")?;
    // One stick with Vault 1 on it, a seed sealed inside, and beside the
    // vault on the same stick another seed's words file and SeedQR
    // picture; Paper under the stick box (§19.14): in the dark theme and
    // the light.
    let p = vault_place(c, t, 0).ok_or("Vault 1's stick has no place")?;
    let s = vault_stick(c, t, p, 0).ok_or("no stick for Vault 1")?;
    // The stick with Vault 1 and nothing beside it: its files box empty
    // (§19.17).
    scroll_to(t, Action::Board(B::Zone(Zone::Files(p, s))));
    t.shot("board-vault-no-files")?;
    bput(t, Thing::Seed(1), Zone::Files(p, s));
    t.press(Action::Board(B::Chip(Chip::Placed(
        Zone::Files(p, s),
        Thing::Seed(1),
    ))));
    t.press(Action::Board(B::Form(Zone::Files(p, s), Thing::Seed(1), 1)));
    t.press(Action::Board(B::LetGo));
    let dark = faraday_core::ui::Mode::Dark;
    t.press(Action::Theme(faraday_core::ui::Theme { mode: dark, ..was }));
    // In the narrow column the stick box is under Add and Place 1's head.
    if t.app.is_compact() {
        scroll(t, 260);
    }
    t.shot("board-vault-and-files-dark")?;
    let light = faraday_core::ui::Mode::Light;
    t.press(Action::Theme(faraday_core::ui::Theme {
        mode: light,
        ..was
    }));
    t.shot("board-vault-and-files-light")?;
    // Back to the theme the tour is in.
    t.press(Action::Theme(was));
    // + stick pressed with nothing held: an empty stick with its files
    // box; **+ vault**: a vault at no place, dim on the list (§19.16).
    t.press(Action::Board(B::Zone(Zone::NewStick(p))));
    scroll_to(t, Action::Board(B::RemoveEmpty(p)));
    t.shot("board-empty-stick")?;
    t.press(Action::Board(B::RemoveEmpty(p)));
    t.press(Action::Board(B::AddVault));
    t.shot("board-vault-at-no-place")?;
    // The narrow column: Add opens the list as a sheet; a chip pressed
    // there opens its menu, Add to… (§19.12 item 3, §19.20 item 4).
    if t.app.is_compact() {
        t.press(Action::Board(B::OpenList));
        t.shot("board-narrow-add-sheet")?;
        t.press(Action::Board(B::Chip(Chip::List(Thing::Seed(2)))));
        t.shot("board-narrow-seed-menu")?;
        t.press(Action::Board(B::MenuClose));
        t.press(Action::Cancel);
    }
    t.press(Action::BQ(qstep::PLAN));
    t.shot("board-the-plan")?;
    // A place's name, typed with a vault open: just pressed, the whole of
    // it selected; after one key, that key alone (§19.18).
    open_a_vault(t, "board phrase")?;
    t.press(Action::Backup(savings));
    if t.app.backup().and_then(|b| b.q) != Some(qstep::BOARD) {
        t.press(Action::BQ(qstep::BOARD));
    }
    scroll_to(t, Action::BName(0));
    t.press(Action::BName(0));
    t.shot("board-name-pressed")?;
    type_text(t, "K");
    t.shot("board-name-one-key")?;
    t.app.event(Event::Key(Key::Enter));
    Ok(())
}

/// A vault made with `phrase` and unlocked, as a person makes one from
/// Vaults.
pub(crate) fn open_a_vault(t: &mut Tour, phrase: &str) -> Result<(), String> {
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, phrase);
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, phrase);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.vaults().open.is_empty() {
        return Err("the vault made for the tour did not open".to_string());
    }
    Ok(())
}
