//! The flow tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};
use faraday_shell_api::{App, Event, Key};

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn flow_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::Kind;
    use faraday_core::backup_flow::steps::FlowAction as F;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|x| x.go == faraday_core::catalog::Go::VaultCategory(Kind::Gpg.category()))
        .ok_or("no GPG tile")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    // Vault 1 is the test vault, on the first stick; Vault 2 is a new one
    // at the second and third places, on two empty sticks.
    t.press(Action::Flow(F::AddPlace));
    t.press(Action::Flow(F::PlaceVault(1, 1)));
    t.press(Action::Flow(F::PlaceVault(2, 1)));
    t.press(Action::Flow(F::VaultExisting(0)));
    t.shot("flow-1-where")?;
    t.press(Action::Flow(F::Continue));
    t.shot("flow-2-insert")?;
    let first = t.stick();
    t.app.storage(StorageEvent::Sticks(vec![first.clone()]));
    t.pump();
    t.shot("flow-3-which-vault")?;
    let k = t
        .app
        .flow()
        .and_then(|f| f.live.reading.as_ref())
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "vault.ofv"))
        .ok_or("the stick shows no vault.ofv")?;
    t.press(Action::Flow(F::PickFile(0, k)));
    t.shot("flow-4-confirm")?;
    t.press(Action::Flow(F::Tap));
    // Bound: its pill is lit and the card says to mark it (§18).
    t.shot("flow-4b-mark")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    // Two empty sticks that read exactly alike: only the person can tell
    // them apart.
    let empty = |id: &str| StickInfo {
        serial: Some(0x1357_9bdf),
        id: id.into(),
        label: "SPARE".into(),
        boot: false,
        files: Vec::new(),
    };
    t.app.storage(StorageEvent::Sticks(vec![empty("empty-a")]));
    t.pump();
    t.press(Action::Flow(F::Tap));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.app.storage(StorageEvent::Sticks(vec![empty("empty-b")]));
    t.pump();
    t.shot("flow-5-hold")?;
    t.press(Action::Flow(F::Hold));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.shot("flow-6-vaults")?;
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    t.tick();
    t.shot("flow-7-create-vault")?;
    t.press(Action::Flow(F::CreateVault(1)));
    // A finger on the first passphrase: a small panel opens the page
    // with nothing focused.
    t.press(Action::Vault(V::CFocus(0, false)));
    t.shot("flow-8-create-vault-form")?;
    type_text(t, "a second vault phrase");
    t.press(Action::Vault(V::CFocus(0, true)));
    type_text(t, "a second vault phrase");
    t.press(Action::Vault(V::CGo));
    t.tick();
    t.tick();
    t.tick();
    t.shot("flow-9-make")?;
    type_text(t, "Test Person");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "test@example.com");
    t.press(Action::Flow(F::Make));
    t.shot("flow-10-made")?;
    t.press(Action::Flow(F::MadeContinue));
    t.shot("flow-11-lock")?;
    t.press(Action::Flow(F::LockWrite));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.pump();
    t.shot("flow-12-write")?;
    t.app.storage(StorageEvent::Sticks(vec![first]));
    t.pump();
    t.shot("flow-13-written")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    // A stick that is not the one read for the place: Use another stick.
    let mut other = empty("other");
    other.label = "ELSEWHERE".into();
    other.serial = Some(0x2468_ace0);
    t.app.storage(StorageEvent::Sticks(vec![other]));
    t.pump();
    t.shot("flow-14-use-another")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.app.storage(StorageEvent::Sticks(vec![empty("empty-a")]));
    t.pump();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.app.storage(StorageEvent::Sticks(vec![empty("empty-b")]));
    t.pump();
    t.shot("flow-15-second-stick")?;
    t.press(Action::Flow(F::Hold));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.shot("flow-16-done")?;
    println!("the backup flow of a GPG key in two vaults at three places, from Where to Done");
    Ok(())
}
