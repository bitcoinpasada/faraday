//! What the app keeps worked out stays within a fixed size however long
//! a session goes on (`docs/SPEED.md` §3.3): each list kept by what it
//! was worked out from has a bound, and the oldest entry, or the whole
//! list, goes when it is reached. A long session loads and drops every
//! test wallet, brings files in and takes them out, changes a backup's
//! plan, locks and unlocks the test vault, and walks far through
//! addresses, account keys, typed descriptions and QR codes; each list
//! is within its bound at the end of every round.
//!
//! The values kept one at a time (Explore's reading, the calculator's,
//! the Silent page's, the signed message checked, a spend's nonce check,
//! what the Inbox gives, Load a key's probe) are an `Option` each, bound
//! by their type.

use faraday_shell_api::App;

use super::{unlock_test_vault, walk};
use crate::create::NewKind;
use crate::plan::{Answers, Preset};
use crate::seeds::SeedsState;
use crate::wallet::{KeyTexts, Origin, WALLET_ADDRESSES_KEPT, WALLET_SCRIPTS_KEPT};
use crate::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_keys::KEY_TEXTS_KEPT;

/// Each kept list with its length and its bound.
fn sizes(app: &Faraday) -> Vec<(String, usize, usize)> {
    let mut out: Vec<(String, usize, usize)> = app
        .kept
        .sizes()
        .into_iter()
        .map(|(name, len, bound)| (name.to_string(), len, bound))
        .collect();
    out.extend([
        (
            "vault keys' fingerprints (vaults.fingerprints)".into(),
            app.vaults.fingerprints.borrow().len(),
            crate::vault::FINGERPRINTS_KEPT,
        ),
        (
            "vault keys' account keys (vaults.accounts)".into(),
            app.vaults.accounts.borrow().len(),
            crate::vault::ACCOUNTS_KEPT,
        ),
        (
            "APKs read (apk)".into(),
            app.vaults.apk.kept(),
            crate::apk::APK_READS_KEPT,
        ),
    ]);
    for w in &app.session.wallets {
        let (addresses, scripts) = w.kept();
        out.push((
            format!("{}'s addresses", w.name),
            addresses,
            WALLET_ADDRESSES_KEPT,
        ));
        out.push((
            format!("{}'s scripts", w.name),
            scripts,
            WALLET_SCRIPTS_KEPT,
        ));
    }
    for (i, k) in app.session.keys.all().enumerate() {
        out.push((
            format!("{}'s key texts", k.label),
            app.session.keys.texts_kept(i),
            KEY_TEXTS_KEPT,
        ));
    }
    out
}

/// One round of the long session.
fn round(app: &mut Faraday, n: usize) {
    let c = testkit::compute();
    let net = app.session.network();
    // Every test wallet dropped and loaded again.
    while app.session.wallets.len() > 1 {
        let last = app.session.wallets.len() - 1;
        app.press(Action::RemoveWallet(last));
    }
    for kit in testkit::kits() {
        let _ = app
            .session
            .add_wallet(&c, kit.name, &kit.descriptor, "test", Origin::Named);
    }
    // Files in under new names, and out again.
    let mut inbox = testkit::files().expect("the test stick's files");
    // The vault keeps its name: it is unlocked below by it.
    for (name, _) in inbox.iter_mut().filter(|(name, _)| name != "vault.ofv") {
        *name = format!("{n}-{name}");
    }
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    for screen in [
        Screen::Files,
        Screen::Home,
        Screen::Wallets,
        Screen::Backups,
    ] {
        app.press(Action::Nav(screen));
        let _ = app.frame();
    }
    // The backup's plan through every preset, drawn.
    if let Some(b) = app.backup.as_ref() {
        let shape = app.plan_shape_now(b.wallet);
        for preset in Preset::ALL {
            if let Some(b) = app.backup.as_mut() {
                b.answers = Answers::preset(&shape, preset);
            }
            let _ = app
                .kept
                .plan_mapped(&c, &shape, &Answers::preset(&shape, preset));
            let _ = app.frame();
        }
    }
    // The test vault locked and unlocked, its contents drawn.
    let _ = app.vault_lock_one(0);
    unlock_test_vault(app);
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    // A long walk through addresses and places, account keys, typed
    // descriptions, QR codes and the Seeds tool's wallets.
    for w in &app.session.wallets {
        for i in 0..300 {
            let _ = w.address_on(&c, net, false, i + 300 * n as u32);
            let _ = w.script_at(&c, true, i + 300 * n as u32);
        }
    }
    for k in 0..app.session.keys.len() {
        for a in 0..80 {
            let _ = app
                .session
                .keys
                .account_text(&c, k, NewKind::NativeSegwit, a + 80 * n as u32);
        }
    }
    for i in 0..40 {
        let text = format!("{n} {i} not a wallet");
        let _ = app.kept.read_kept(&c, &text);
        let _ = app.kept.parsed_kept(&c, &text);
        let _ = app.kept.config_name_kept(&c, &text);
        let _ = app.kept.public_qr(&c, &text);
    }
    for a in 0..20 {
        let s = SeedsState {
            keys: vec![app.session.keys[0].fingerprint.0],
            m: 1,
            n: 1,
            kind: NewKind::NativeSegwit,
            account: a + 20 * n as u32,
            ..SeedsState::default()
        };
        let _ = app.seeds_address(&c, &s);
    }
}

#[test]
fn every_kept_list_stays_within_its_bound_over_a_long_session() {
    let c = testkit::compute();
    let (mut app, w) = walk(&c, &mut |_, _| {});
    app.press(Action::Backup(w));
    let mut failures = Vec::new();
    for n in 0..3 {
        round(&mut app, n);
        for (name, len, bound) in sizes(&app) {
            if len > bound {
                failures.push(format!(
                    "round {n}: {name} keeps {len}, more than its bound {bound}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
