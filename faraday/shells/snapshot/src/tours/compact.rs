//! The compact tour.

use super::seeds::seeds_tour;
use crate::tour::{Tour, type_text};
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event, Key};

/// The small panel's Home, and Make a new wallet with a new key from dice,
/// a step to a page.
pub(crate) fn compact_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::cstep;
    t.shot("home")?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("wallets-empty")?;
    // Add a key with a stick in: the sheet, then Add a key once it is
    // pulled.
    t.sticks(true);
    t.press(Action::Nav(Screen::Home));
    t.shot("home-stick")?;
    t.press(Action::Entry(None));
    t.shot("home-pull")?;
    t.sticks(false);
    t.shot("entry-after-pull")?;
    // Its foot: Scan a SeedQR, Make a new key, and the way in from a stick.
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: 240,
        y: 200,
        dy: 1200,
    });
    t.shot("entry-foot")?;
    t.press(Action::Nav(Screen::Home));
    t.press(Action::CreateWallet);
    // Create opens on Kind, single key the default
    // (`docs/history/NEW-WALLET.md` §2.1).
    t.shot("create-kind")?;
    t.press(Action::CNext(cstep::KIND));
    t.shot("create-keys")?;
    t.press(Action::KeyGen(Some(0)));
    t.shot("keygen-length")?;
    t.press(Action::KWords(12));
    t.shot("keygen-source")?;
    t.press(Action::KNext);
    t.shot("keygen-rolls-none")?;
    let mut x: u32 = 0x2545_f491;
    for i in 0..128 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        t.press(Action::KRoll((x % 6) as u8 + 1));
        if i == 20 {
            t.shot("keygen-rolls")?;
        }
    }
    // The randomness check is on the entry card once the last roll is
    // in; its Continue opens Key (`docs/history/NEW-WALLET.md` §3.2).
    t.shot("keygen-rolls-done")?;
    t.press(Action::KNext);
    t.shot("keygen-key")?;
    t.press(Action::KPassField(0));
    // Focused and empty: the No passphrase pill, the caret after it
    // (`docs/history/NEW-WALLET.md` §13.4).
    t.shot("keygen-key-passphrase-empty")?;
    type_text(t, "Ride the 7 bus");
    t.shot("keygen-key-passphrase")?;
    // Five of the dots selected with Shift and Left (§13.2).
    t.app.event(Event::Shift { held: true });
    for _ in 0..5 {
        t.app.event(Event::Key(Key::Left));
    }
    t.app.event(Event::Shift { held: false });
    t.shot("keygen-key-passphrase-selected")?;
    t.press(Action::KPassField(1));
    type_text(t, "Ride the 7 bus");
    // Lock in adds the key and, a single key being the whole wallet,
    // makes it: the Key card shows its descriptor and first address.
    t.press(Action::KLock);
    t.shot("keygen-key-locked")?;
    t.shot("create-backup")?;
    t.press(Action::CStep(cstep::KEYS));
    t.shot("create-keys-filled")?;
    t.press(Action::CStep(cstep::CHECK));
    t.shot("create-check")?;
    t.press(Action::CStep(cstep::BACKUP));
    // Paper and vault: the backup on the new wallet, preset applied.
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.shot("create-backup-plan")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("home-loaded")?;
    // Add a key: two words typed, the third begun; then a SLIP-39 share.
    t.press(Action::Entry(None));
    type_text(t, "zebra zebra zeb");
    t.shot("entry-typing")?;
    t.press(Action::EntryForm(1));
    type_text(t, "shadow pistol academic ac");
    t.shot("entry-slip39")?;
    t.press(Action::Nav(Screen::Home));
    seeds_tour(t)?;
    Ok(())
}
