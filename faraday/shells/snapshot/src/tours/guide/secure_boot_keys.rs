//! The pictures of `docs/guide/walk/secure-boot-keys.md`.

use super::gpg::{bring_in, create_vault, lock_and_write, make, start, unlock_done};
use crate::tour::{Files, Tour, type_text};
use faraday_core::Action;
use faraday_core::backup_flow::Kind;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::vault::VaultAction as V;

/// The boot program, as the test release's sums name it.
const EFI: &str = "faraday-0.9.0-BOOTX64.EFI";

/// Costs what the flow's keys and the image's signature cost. Tools'
/// Secure Boot keys with nothing open: the plan, the sticks, Vault 1,
/// the keys made for Example Org, the sticks written; Vault 1 unlocked
/// on the keys' page; the release's boot program, copied in with its
/// signed sums, listed and looked at before signing.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    start(t, Kind::SecureBoot)?;
    t.shot_marking("secure-boot-keys-where", Action::Flow(F::Continue))?;
    t.press(Action::Flow(F::Continue));
    bring_in(t)?;
    create_vault(t)?;
    type_text(t, "Example Org");
    t.shot_marking("secure-boot-keys-make", Action::Flow(F::Make))?;
    make(t)?;
    t.shot_marking("secure-boot-keys-made", Action::Flow(F::MadeContinue))?;
    t.press(Action::Flow(F::MadeContinue));
    lock_and_write(t, release()?, None)?;
    t.shot_marking("secure-boot-keys-done", Action::Flow(F::UnlockDone(0)))?;
    unlock_done(t)?;
    t.press(Action::Vault(V::SbOwnOnly(false)));
    t.shot_marking("secure-boot-keys-page", Action::Vault(V::SbImages))?;
    t.press(Action::Vault(V::SbImages));
    let efi = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == EFI)
        .ok_or("the boot program is not in Files")?;
    t.shot_marking("secure-boot-keys-images", Action::Vault(V::SbPick(efi)))?;
    t.press(Action::Vault(V::SbPick(efi)));
    t.shot_marking("secure-boot-keys-sign", Action::Vault(V::SbHoldSign))
}

/// The test release's boot program and its signed sums, as a stick
/// visit copies them in.
fn release() -> Result<Files, String> {
    let kit = faraday_core::testkit::files()?;
    let of = |name: &str| {
        kit.iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.clone())
            .ok_or(format!("the kit has no {name}"))
    };
    Ok(vec![
        (EFI.into(), of("BOOTX64.EFI")?),
        ("SHA256SUMS".into(), of("SHA256SUMS")?),
        ("SHA256SUMS.asc".into(), of("SHA256SUMS.asc")?),
    ])
}
