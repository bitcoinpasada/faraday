//! The seed's words shown and hidden on the backup's copy step, and a
//! backup written of the test seed (its copy checked, the seed saved into
//! a vault, a seed file let out), each ended by Lock.

use crate::common::{device, type_text};
use crate::forms::{PHRASE, VAULT_PASSPHRASE, seed_forms, vault_passphrase_forms};
use crate::{arm, finish};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::{Action, Faraday, Screen, Sheet, bstep, plan, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};

/// The test seed typed into Add a key, a one-key wallet made from it and
/// its backup planned as `preset`, open on the seed's copy.
fn at_the_copy(app: &mut Faraday, preset: plan::Preset) {
    let c = testkit::compute();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    app.press(Action::Entry(None));
    type_text(app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    assert_eq!(app.session.wallets.len(), 1, "no wallet was made");
    app.press(Action::Backup(0));
    testkit::backup_as(app, preset);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    if app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
}

#[test]
fn the_words_shown_and_hidden_leave_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = testkit::started();
    at_the_copy(&mut app, plan::Preset::Paper);
    app.press(Action::BReveal);
    let _ = app.frame();
    app.press(Action::BReveal);
    let _ = app.frame();
    crate::seed_load::lock(&mut app);
    finish("words shown and hidden", armed, true);
}

/// The copy checked by typing the words back, the seed saved into the
/// open vault, then Lock.
#[test]
fn a_backup_saved_into_a_vault_leaves_no_copy_after_lock() {
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x5f, Vec::new());
    let mut forms = seed_forms("");
    forms.extend(vault_passphrase_forms());
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    at_the_copy(&mut app, plan::Preset::PaperVault);
    app.press(Action::BReveal);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, PHRASE);
    crate::seed_load::unlock(&mut app);
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BVaultSave(0));
    // The shell takes what the app asks of storage after every press.
    while app.poll_storage().is_some() {}
    crate::seed_load::lock(&mut app);
    while app.poll_storage().is_some() {}
    finish("backup saved into a vault", armed, true);
}

/// A seed file let out unprotected, which a person confirms and which
/// then stays in the Outbox across a lock.
#[test]
#[ignore = "by design: a seed file let out unprotected stays in the Outbox after Lock until it is sent or removed, so its words stay live (three blocks of 153 bytes)"]
fn a_seed_file_let_out_unprotected_leaves_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = testkit::started();
    at_the_copy(&mut app, plan::Preset::Paper);
    app.press(Action::BReveal);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, PHRASE);
    app.press(Action::BFile);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    app.press(Action::SecretForm(0));
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    assert_eq!(app.sheet, None);
    while app.poll_storage().is_some() {}
    crate::seed_load::lock(&mut app);
    while app.poll_storage().is_some() {}
    finish("seed file let out", armed, true);
}
