//! The again tour.

use crate::tour::{Tour, scroll, scroll_to, to_checklist, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent, bstep, qrow};

/// A loaded wallet backed up again: Savings loaded from its descriptor
/// file with no seed here opens on its public files, every one on offer;
/// a one-key wallet restored from test key 2 saves itself into the test
/// vault from its checklist; then the wallet chip's list of the two.
pub(crate) fn again_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::seeds::SeedsAction as S;
    t.copy_wallets();
    let savings = t
        .app
        .inbox()
        .iter()
        .position(|it| it.name == "savings-wallet.txt")
        .ok_or("savings-wallet.txt is not in Files")?;
    t.press(Action::LoadWallet(savings));
    t.press(Action::Backup(0));
    t.shot("again-watch-only")?;
    let every = [
        (qrow::SOFTWARE, 0),
        (qrow::SOFTWARE, 1),
        (qrow::SOFTWARE, 2),
        (qrow::SOFTWARE, 3),
        (qrow::FORM, 1),
    ];
    to_checklist(t, faraday_core::plan::Preset::PaperVaultSoftware, &every);
    t.press(Action::BStep(bstep::PUBLIC));
    scroll_to(t, Action::PublicOut(0, 7));
    t.shot("again-watch-only-files")?;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    let inbox: Vec<(String, Vec<u8>)> = t
        .app
        .inbox()
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .chain(std::iter::once(vault))
        .collect();
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.press(Action::Entry(None));
    t.type_key(1);
    let fp = t
        .app
        .session
        .keys
        .last()
        .ok_or("test key 2 did not load")?
        .fingerprint;
    t.press(Action::KeyWallet(fp.0, 1));
    t.press(Action::Seeds(S::Make));
    let w = t.app.session.wallets.len() - 1;
    t.press(Action::Backup(w));
    to_checklist(t, faraday_core::plan::Preset::PaperVault, &[]);
    t.press(Action::BStep(bstep::VAULT));
    scroll_to(t, Action::Vault(V::OpenFrom(0, Screen::Backup)));
    t.shot("again-restored-locked")?;
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Vault(V::OpenFrom(0, Screen::Backup)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.vaults().open.len() != 1 || t.app.screen != Screen::Backup {
        return Err("the test vault did not open back to the backup".into());
    }
    scroll_to(t, Action::Vault(V::SaveWallet(w)));
    t.shot("again-restored-vault")?;
    t.press(Action::Vault(V::SaveWallet(w)));
    t.shot("again-restored-saved")?;
    // Back to the head, where the chip is.
    for _ in 0..10 {
        scroll(t, -120);
        t.app.settle();
    }
    t.press(Action::BWallets);
    t.shot("again-wallets")?;
    Ok(())
}
