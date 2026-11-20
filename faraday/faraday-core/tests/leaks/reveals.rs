//! Every reason a secret leaves the store (`faraday_keys::Reveal`, with
//! the `Part` it carries) names the leak flow that reaches it
//! (`docs/SECRET-LEAKS.md` §4). [`flows`] matches with no wildcard arm, so
//! a new reason does not compile here until it has a flow;
//! `tools/lint-leak-reveals.py` (in `just lint-repo`) reads the arms and
//! fails on a reason with none, and on a name that is no test of this
//! binary and no flow label of `areas.rs`. The flows of this file show
//! what no other flow of the binary does and end by Lock.

use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish};
use faraday_core::explore::{Eye, XAction};
use faraday_core::wallet_backup::quick::Quick;
use faraday_core::{Action, Faraday, testkit};
use faraday_keys::{Part, Reveal};
use faraday_shell_api::App as _;

/// The flows that reach a reason: names of `#[test]` functions of this
/// binary, or labels of `areas.rs`'s flows. An empty list is only for a
/// reason the app never asks, and its arm says so.
#[expect(
    dead_code,
    reason = "held by the compiler and tools/lint-leak-reveals.py"
)]
fn flows(why: Reveal) -> &'static [&'static str] {
    match why {
        Reveal::Words => &["the_words_shown_and_hidden_leave_no_copy_after_lock"],
        Reveal::Passphrase => {
            &["a_seed_checked_and_its_passphrase_taken_again_leaves_no_copy_after_lock"]
        }
        Reveal::SeedQr => &["a_seedqr_shown_on_the_quick_backup_leaves_no_copy_after_lock"],
        Reveal::CopyByHand => &["a Krux code's passphrase in a backup"],
        Reveal::PaperForm => &["Convert a seed"],
        Reveal::Bip85Child => &["BIP-85 child"],
        Reveal::LightningKey => {
            &["a_lightning_key_shown_and_put_in_a_vault_leaves_no_copy_after_lock"]
        }
        Reveal::GpgPaperkey => &[
            "a_vaults_gpg_key_shown_by_its_eye_and_its_revocation_let_out_leave_no_copy_after_lock",
        ],
        Reveal::VaultEntry(p) => vault_entry(p),
        Reveal::OutboxFile(p) => outbox_file(p),
        Reveal::BackupStick(p) => backup_stick(p),
        Reveal::SeedShown | Reveal::ExtendedPrivateKey | Reveal::AddressKey => {
            &["the_explore_eyes_opened_and_closed_leave_no_copy_after_lock"]
        }
        Reveal::KefCode => &["a_krux_code_sealed_leaves_no_copy"],
    }
}

/// The parts of a vault record put on screen, into a form or a record.
/// `Passphrase` and `Codex32` have no flow yet (the lint lists them).
fn vault_entry(p: Part) -> &'static [&'static str] {
    match p {
        Part::Words | Part::Password | Part::Totp | Part::EntryNotes => {
            &["a_vaults_entries_listed_and_opened_leave_no_copy_after_lock"]
        }
        Part::Note => &["a_vaults_note_listed_and_opened_leaves_no_copy_after_lock"],
        Part::NodeKey => &["a_lightning_key_shown_and_put_in_a_vault_leaves_no_copy_after_lock"],
        Part::ScanKey => &["a_scan_key_put_in_a_vault_and_let_out_leaves_no_copy_after_lock"],
        Part::Passphrase | Part::Codex32 | Part::SeedQr | Part::Revocation | Part::KeyFile => &[],
    }
}

/// The parts an unprotected file to the Outbox carries.
fn outbox_file(p: Part) -> &'static [&'static str] {
    match p {
        Part::Words | Part::Passphrase | Part::SeedQr => {
            &["a_seed_file_let_out_unprotected_leaves_no_copy_after_lock"]
        }
        Part::NodeKey => &["a secret on its sheet"],
        Part::ScanKey => &["a_scan_key_put_in_a_vault_and_let_out_leaves_no_copy_after_lock"],
        Part::Revocation => &[
            "a_vaults_gpg_key_shown_by_its_eye_and_its_revocation_let_out_leave_no_copy_after_lock",
        ],
        Part::KeyFile => &["a_key_file_made_for_a_vault_leaves_no_copy_after_lock"],
        Part::Password | Part::Totp | Part::Note | Part::EntryNotes | Part::Codex32 => &[],
    }
}

/// The parts the backup flow writes to a stick. `Codex32` has no flow yet
/// (the lint lists it).
fn backup_stick(p: Part) -> &'static [&'static str] {
    match p {
        Part::Words | Part::Passphrase | Part::SeedQr => {
            &["a_seed_file_let_out_unprotected_leaves_no_copy_after_lock"]
        }
        Part::Codex32
        | Part::Password
        | Part::Totp
        | Part::Note
        | Part::EntryNotes
        | Part::Revocation
        | Part::ScanKey
        | Part::NodeKey
        | Part::KeyFile => &[],
    }
}

/// A key loaded from the test seed.
fn key_loaded() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    crate::common::type_text(&mut app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    testkit::keep_published(&mut app);
    app
}

#[test]
fn the_explore_eyes_opened_and_closed_leave_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = key_loaded();
    app.press(Action::Explore);
    for e in [Eye::Secret, Eye::Xprv, Eye::Wif] {
        app.press(Action::X(XAction::Eye(e)));
        let _ = app.frame();
        app.press(Action::X(XAction::Eye(e)));
        let _ = app.frame();
    }
    crate::seed_load::lock(&mut app);
    finish("explore eyes", armed, true);
}

#[test]
fn a_seedqr_shown_on_the_quick_backup_leaves_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = key_loaded();
    let w = crate::areas::wallet_over_key(&mut app);
    app.press(Action::BQuick(Quick::Open(w)));
    app.press(Action::BQuick(Quick::SeedQr));
    let _ = app.frame();
    app.press(Action::BQuick(Quick::Wrote));
    let _ = app.frame();
    crate::seed_load::lock(&mut app);
    finish("quick backup SeedQR", armed, true);
}
