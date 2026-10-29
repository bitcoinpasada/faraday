//! The screens tour: the pictures of the guide's screen reference
//! (`docs/guide/screens/`), each screen in a representative state on a
//! Mainnet stick with the guide's key (3dca6805) and its single-key
//! wallet, each shot named as its picture's file (`just guide-pictures`).

use crate::tour::{Tour, laptop, type_text};
use crate::tours::guide::{DICE_WORDS, kind_at};
use faraday_core::create::NewKind;
use faraday_core::fresh::OwnSource;
use faraday_core::fresh::page::Act;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Medium, Screen, StickInfo, StorageEvent, cstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::App;

/// Costs what the screens it opens cost. A fresh stick session: Home and
/// Wallets with nothing loaded, the Pull sheet, New key's Start, Add a
/// key with the guide's words typed; then, with that key, each tool
/// screen and Settings; the wallet made from it, its card, Backups and
/// the Spend tab from Home; the Lock and power sheets; last, Faraday Demo's Mode
/// and Load samples sheets.
pub(crate) fn screens_tour(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    stick_session(t);
    t.press(Action::Nav(Screen::Home));
    t.shot("home-new-stick")?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("wallets-start-empty")?;
    t.press(Action::Nav(Screen::Vaults));
    t.shot("vaults-list")?;
    t.press(Action::Vault(V::Create));
    t.press(Action::Vault(V::CStep(faraday_core::vault::vstep::PRESET)));
    t.shot("create-vault-size")?;
    // With a stick in, Add a key asks for it out first.
    t.app.storage(StorageEvent::Sticks(vec![stick()]));
    t.pump();
    t.go_on = false;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.shot("prompts-pull")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.go_on = true;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.shot("new-key-start")?;
    t.press(Action::Learn);
    t.shot("learn")?;
    t.press(Action::Cancel);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Entry(None));
    let _ = t.app.frame();
    type_text(t, DICE_WORDS);
    t.shot_marking("add-a-key-words", Action::EntryAdd)?;
    t.press(Action::EntryAdd);
    tools(t)?;
    wallet(t)?;
    demo(t)
}

/// A fresh stick session on Mainnet, as the guide's pictures show it: a
/// laptop named for no maker, Secure Boot off at start, no stick in.
pub(crate) fn stick_session(t: &mut Tour) {
    let size = t.size();
    t.app = Faraday::new();
    t.app.medium = Medium::Stick;
    t.app.devices = laptop();
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    let _ = faraday_core::testkit::move_to(&mut t.app.session, faraday_core::testkit::net());
    t.app.event(crate::tour::display(size, t.dpi));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
}

/// A stick with nothing on it.
fn stick() -> StickInfo {
    StickInfo {
        serial: Some(0x4000),
        id: "screens".into(),
        label: "KEYSTICK".into(),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }
}

/// Each tool screen with the guide's key loaded, opened from Tools: the
/// calculators on Hashes, each tool that works from a key, Vanity
/// address's search run; Settings and its two pages of its own.
fn tools(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Catalog));
    t.shot("tools")?;
    crate::tours::guide::hashes(t, "calculators")?;
    // Each tool on the key opened from Tools, its way back.
    t.press(Action::Nav(Screen::Catalog));
    crate::tours::guide::on_the_key(t, "")?;
    for (screen, name) in [
        (Screen::Settings, "settings"),
        (Screen::SetTime, "settings-set-time"),
    ] {
        t.press(Action::Nav(screen));
        t.shot(name)?;
    }
    t.press(Action::Nav(Screen::Settings));
    t.press(Action::Own(Act::Open));
    t.press(Action::Own(Act::Source(OwnSource::Dice)));
    for f in [3, 1, 4, 1, 5, 2, 6, 5, 3, 5] {
        t.press(Action::Own(Act::Roll(f)));
    }
    t.shot("settings-add-randomness")
}

/// Make a new wallet's Keys with the guide's key in its slot, the Lock
/// sheet while it is made, the wallet's card, Backups, and the Spend tab
/// as Home opens it, then on How to spend bitcoin;
/// the power sheet on a stick started with Secure Boot off.
fn wallet(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(kind_at(NewKind::NativeSegwit)?));
    t.press(Action::CVaults(false));
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.shot_marking("make-a-wallet-keys", Action::CNext(cstep::KEYS))?;
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::LockAsk);
    t.shot("lock-and-power-lock")?;
    t.press(Action::Cancel);
    t.press(Action::CancelFlow);
    t.shot("prompts-cancel")?;
    t.press(Action::Cancel);
    t.press(Action::CNext(cstep::CHECK));
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot("wallets-card")?;
    t.press(Action::SignMessage);
    t.shot("messages-sign")?;
    t.press(Action::Nav(Screen::Backups));
    t.shot("backups-list")?;
    // Spend as Home's Spend from a backup, step by step opens it: what
    // is loaded, then Walk through from the start on How to spend
    // bitcoin.
    t.press(Action::Nav(Screen::Home));
    t.press(Action::Nav(Screen::Family));
    t.shot("spend-tab-loaded")?;
    t.press(Action::Family(
        faraday_core::family::FamilyAction::Walkthrough,
    ));
    t.shot("spend-tab-map")?;
    t.press(Action::PowerAsk);
    t.shot("lock-and-power-power")?;
    t.press(Action::Cancel);
    Ok(())
}

/// Faraday Demo's Mode sheet with Practice ticked, and Load samples with
/// every sample ticked.
fn demo(t: &mut Tour) -> Result<(), String> {
    crate::tours::guide::demo_session(t);
    t.press(Action::ModeSheet);
    t.shot("mode-mode")?;
    t.press(Action::Cancel);
    t.press(Action::Samples);
    t.press(Action::SamplesAll);
    t.shot("mode-samples")
}
