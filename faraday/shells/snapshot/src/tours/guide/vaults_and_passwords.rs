//! The pictures of `docs/guide/walk/vaults-and-passwords.md`, and those
//! of `docs/guide/screens/create-vault.md` and `codes.md`.

use super::key_session;
use crate::tour::{Tour, type_text};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};

/// Rolls of five dice, as they land: six words of the long list.
const ROLLS: &str = "164532521346316254425163231564655213";

/// A fresh stick session with the guide's key typed in: Create a vault,
/// and Set the time; the Passwords vault made with a passphrase from
/// dice, unlocked, the key saved in it, a login with its TOTP code, the
/// codes; locked, written to a stick, and unlocked again.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    super::gpg::machine(t);
    t.press(Action::Nav(Screen::CreateVault));
    t.shot("vaults-and-passwords-create")?;
    t.press(Action::Nav(Screen::SetTime));
    t.shot("vaults-and-passwords-set-time")?;
    t.press(Action::SetTimeKeep);
    where_card(t)?;
    let phrase = create(t)?;
    unlock(t, &phrase)?;
    contents(t)?;
    entries(t)?;
    write_and_load(t, &phrase)
}

/// Create a vault's Customise: Where will you open it? with this
/// computer and a Raspberry Pi 3 ticked, as the size has them.
fn where_card(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    t.press(Action::Vault(V::CStep(faraday_core::vault::vstep::PRESET)));
    t.press(Action::Vault(V::CCustomise));
    // The size's machines, a Raspberry Pi 3 among them, ticked.
    let pi_3 = faraday_core::vault::MACHINES
        .iter()
        .position(|m| m.3 == 1024)
        .ok_or("no Raspberry Pi 3 to choose")?;
    if !t.app.vaults().create.as_ref().is_some_and(|c| c.on[pi_3]) {
        return Err("Customise left the Raspberry Pi 3 unticked".into());
    }
    t.shot("create-vault-where")?;
    Ok(())
}

/// A fresh Create a vault: the name Passwords, the dice with three words
/// in, then six, used; made. Returns the passphrase.
fn create(t: &mut Tour) -> Result<String, String> {
    t.press(Action::Vault(V::Create));
    t.press(Action::Vault(V::CName));
    type_text(t, "Passwords");
    t.press(Action::Vault(V::Dice(0)));
    type_text(t, &ROLLS[..15]);
    crate::tour::scroll_to(t, Action::Vault(V::DiceUse));
    t.shot_marking("vaults-and-passwords-dice", Action::Vault(V::DiceUse))?;
    type_text(t, &ROLLS[15..30]);
    t.press(Action::Vault(V::DiceUse));
    let phrase = t
        .app
        .vaults()
        .create
        .as_ref()
        .map(|c| c.phrases[0].0.text.to_string())
        .unwrap_or_default();
    if phrase.is_empty() {
        return Err("the dice gave no passphrase".into());
    }
    t.shot_marking("create-vault-passphrases", Action::Vault(V::CGo))?;
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            return Ok(phrase);
        }
        t.tick();
    }
    Err(format!("the vault was not made: {:?}", t.app.screen))
}

/// Unlock with the vault just made picked: its passphrase typed, and
/// unlocked on its empty contents.
fn unlock(t: &mut Tour, phrase: &str) -> Result<(), String> {
    type_text(t, phrase);
    t.shot_marking("vaults-and-passwords-unlock", Action::Vault(V::Unlock))?;
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() {
            return Ok(());
        }
        t.tick();
    }
    Err(format!(
        "the vault did not unlock: {:?}",
        t.app.vaults().unlock_error
    ))
}

/// Save a bitcoin key: the guide's key saved, and shown under Bitcoin
/// keys.
fn contents(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Vault(V::AddKind(0)));
    t.press(Action::Vault(V::SaveKey(0)));
    t.press(Action::Vault(V::Category(0)));
    t.press(Action::Vault(V::Item(0)));
    t.shot("vaults-and-passwords-contents")
}

/// An entry, Mail, with its fields and TOTP secret typed in, saved; the
/// codes; a second entry, Bank, and the codes again.
fn entries(t: &mut Tour) -> Result<(), String> {
    entry(
        t,
        &[
            "Mail",
            "me@example.com",
            "correct-horse-7",
            "https://mail.example.com",
            "Recovery codes are in the note.",
            "JBSWY3DPEHPK3PXP",
        ],
    );
    t.shot("vaults-and-passwords-entry")?;
    open_codes(t);
    if t.app.screen != Screen::Codes {
        return Err(format!("TOTP codes opened {:?}", t.app.screen));
    }
    t.shot("vaults-and-passwords-codes")?;
    t.press(Action::Nav(Screen::VaultContents));
    entry(
        t,
        &[
            "Bank",
            "me@example.com",
            "staple-battery-9",
            "https://bank.example.com",
            "",
            "GEZDGNBVGY3TQOJQ",
        ],
    );
    open_codes(t);
    t.shot("codes-rows")
}

/// TOTP codes, past Set the time where it opens first: the clock kept.
fn open_codes(t: &mut Tour) {
    t.press(Action::OpenCodes);
    if t.app.screen == Screen::SetTime {
        t.press(Action::SetTimeKeep);
    }
    if t.app.screen != Screen::Codes {
        t.press(Action::OpenCodes);
    }
}

/// A new entry with `fields` typed in order, saved.
fn entry(t: &mut Tour, fields: &[&str]) {
    t.press(Action::Vault(V::AddKind(2)));
    for (n, text) in fields.iter().enumerate() {
        t.press(Action::Vault(V::FocusField(n)));
        type_text(t, text);
    }
    t.press(Action::Vault(V::FormSave));
}

/// The stick the vault is written to.
fn stick() -> StickInfo {
    StickInfo {
        serial: Some(0x3200),
        id: "passwords".into(),
        label: "KEYSTICK".into(),
        boot: false,
        files: Vec::new(),
    }
}

/// The lock, the stick's visit writing the vault, Vaults; then the vault
/// unlocked from its row and the sheet of what to load.
fn write_and_load(t: &mut Tour, phrase: &str) -> Result<(), String> {
    t.press(Action::LockAsk);
    t.press(Action::Lock);
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    super::gpg::machine(t);
    t.app.storage(StorageEvent::Sticks(vec![stick()]));
    t.pump();
    t.press(Action::VisitWrite);
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.press(Action::Nav(Screen::Vaults));
    t.shot("vaults-and-passwords-written")?;
    let row = t
        .app
        .vault_files()
        .iter()
        .position(|f| f.name == "Passwords.fvlt")
        .ok_or("Passwords.fvlt is not listed")?;
    t.press(Action::Vault(V::Open(row)));
    type_text(t, phrase);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if t.app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
            return t.shot_marking("vaults-and-passwords-load", Action::Vault(V::LoadTicked));
        }
        t.tick();
    }
    Err(format!("the unlock asked nothing: {:?}", t.app.screen))
}
