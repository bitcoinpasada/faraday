//! The pictures of `docs/guide/walk/spend-from-a-backup.md`, and those of
//! `docs/guide/screens/vault.md` and `vaults.md` that show the vault a
//! backup leaves.

use super::{key_session, kind_at, last_wallet};
use crate::tour::{Files, Tour, type_text};
use faraday_core::create::NewKind;
use faraday_core::family::{FamilyAction as F, page};
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::step;
use faraday_core::{Action, Screen, StickInfo, StorageEvent, cstep};

/// The vault's passphrase in Place 1's envelope.
const PHRASE: &str = "guide vault phrase";

/// Costs what spending costs. The vault Place 1's envelope keeps, made
/// from the guide's key and its wallet; then, in a fresh session, the
/// Spend tab's map, What do you have? with the vault copied in, unlocked
/// and its wallet loaded, the money checked, the PSBT brought, signed and
/// sent, and everything put away; last, the vault unlocked from Vaults.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    let vault = envelope(t)?;
    crate::tours::screens::stick_session(t);
    super::gpg::machine(t);
    t.press(Action::Nav(Screen::Family));
    t.shot("spend-from-a-backup-map")?;
    t.press(Action::Family(F::Next(page::MAP)));
    t.shot("spend-from-a-backup-what-do-you-have")?;
    copy_in(t, &vault)?;
    t.press(Action::Nav(Screen::Family));
    t.press(Action::Family(F::Pick(0)));
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, PHRASE);
    t.shot_marking("spend-from-a-backup-unlock", Action::Family(F::Unlock))?;
    t.press(Action::Family(F::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() {
            break;
        }
        t.tick();
    }
    t.shot_marking(
        "spend-from-a-backup-vault-open",
        Action::Vault(V::LoadWithKeys(0)),
    )?;
    t.press(Action::Vault(V::LoadWithKeys(0)));
    spend(t)?;
    load_from(t, vault)
}

/// Costs a key's wallet made. Place 1's vault: the guide's key and New
/// wallet 1 over it, saved in a vault made on Vaults; its key's page and
/// Save to a stick; then the lock. Returns the vault's file.
fn envelope(t: &mut Tour) -> Result<(String, Vec<u8>), String> {
    key_session(t);
    super::gpg::machine(t);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(kind_at(NewKind::NativeSegwit)?));
    t.press(Action::CVaults(false));
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    let w = last_wallet(t)?;
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, PHRASE);
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, PHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() {
            break;
        }
        t.tick();
    }
    crate::tour::not_now_load(t);
    t.press(Action::Vault(V::AddKind(0)));
    t.press(Action::Vault(V::SaveKey(0)));
    t.press(Action::Vault(V::AddKind(1)));
    t.press(Action::Vault(V::SaveWallet(w)));
    t.press(Action::Vault(V::Category(0)));
    t.press(Action::Vault(V::Item(0)));
    t.shot("vault-key")?;
    t.press(Action::Vault(V::SaveToStick));
    t.shot("vault-save-to-a-stick")?;
    t.press(Action::Cancel);
    t.press(Action::LockAsk);
    t.press(Action::Lock);
    let (_, outbox, _) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    outbox
        .into_iter()
        .find(|(n, _)| n.ends_with(".ofv"))
        .ok_or_else(|| "the lock sealed no vault".into())
}

/// Where the envelope's stick keeps its files.
fn stick_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("faraday-guide-envelope")
}

/// The envelope's stick plugged in, its vault copied in on the visit,
/// and pulled.
fn copy_in(t: &mut Tour, (name, bytes): &(String, Vec<u8>)) -> Result<(), String> {
    let dir = stick_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    let stick = StickInfo {
        id: dir.display().to_string(),
        label: "PLACE1".into(),
        boot: false,
        serial: Some(0x3300),
        files: vec![(name.clone(), bytes.len() as u64)],
    };
    t.app.storage(StorageEvent::Sticks(vec![stick]));
    t.pump();
    t.press(Action::VisitCopy);
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    if !t.app.inbox().iter().any(|i| &i.name == name) {
        return Err(format!("{name} was not copied in"));
    }
    Ok(())
}

/// Costs a PSBT made and signed. Check the money is really there; the
/// payment written; the PSBT brought by QR, read, signed and sent; put
/// away.
fn spend(t: &mut Tour) -> Result<(), String> {
    let w = last_wallet(t)?;
    t.shot_marking(
        "spend-from-a-backup-check",
        Action::Family(F::Next(page::CHECK)),
    )?;
    t.press(Action::Family(F::Next(page::CHECK)));
    // Sparrow's payment written: I have the PSBT.
    t.press(Action::Family(F::Next(page::WRITE)));
    t.shot_marking("spend-from-a-backup-bring", Action::Scan)?;
    let psbt = faraday_core::testkit::unsigned(&super::kit_of(t, w, 1_850))?;
    t.press(Action::Scan);
    super::spend::show_psbt(t, &psbt.to_bytes(), None)?;
    // Read it.
    t.press(Action::Family(F::Next(page::BRING)));
    t.press(Action::StepNext(step::TRANSACTION));
    for at in [step::CHECK, step::SIGNERS] {
        if t.app.spend().and_then(|s| s.open) == Some(at) {
            t.press(Action::StepNext(at));
        }
    }
    t.press(Action::SignHere);
    for at in [step::SIGN, step::COLLECT] {
        if t.app.spend().and_then(|s| s.open) == Some(at) {
            t.press(Action::StepNext(at));
        }
    }
    t.shot_marking("spend-from-a-backup-send", Action::QrSigned)?;
    t.press(Action::StepNext(step::FINISH));
    t.shot_marking("spend-from-a-backup-away", Action::LockAsk)
}

/// A fresh session with the envelope's vault in Files: Unlock from
/// Vaults, and the sheet of what it loads.
fn load_from(t: &mut Tour, vault: (String, Vec<u8>)) -> Result<(), String> {
    let inbox: Files = vec![vault];
    crate::tours::screens::stick_session(t);
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    super::gpg::machine(t);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    t.shot_marking("vaults-unlock", Action::Vault(V::Unlock))?;
    type_text(t, PHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if t.app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
            return t.shot_marking("vaults-load-from", Action::Vault(V::LoadTicked));
        }
        t.tick();
    }
    Err(format!("the unlock asked nothing: {:?}", t.app.screen))
}
