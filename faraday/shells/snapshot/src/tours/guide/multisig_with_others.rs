//! The pictures of `docs/guide/walk/multisig-with-others.md`.

use super::{DICE_WORDS, files_in, key_session, kind_at, last_wallet, seedqr};
use crate::tour::{Tour, key_text_of, scroll_to};
use faraday_core::create::NewKind;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Screen, cstep};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// A cosigner's account key at `path`, a Mainnet path as the guide's
/// pictures are of Mainnet: test key 2 (`5d388376`) or 3 (`cf2e083d`),
/// public test keys standing for the cosigners' own.
fn cosigner(n: usize, path: &str) -> String {
    testkit::key(n, path)
}

/// Costs a key's account derived. The 2-of-3 over the guide's key and
/// the two cosigners', as the coordinator writes it.
fn shared_kit(c: &Compute, t: &Tour) -> Result<Kit, String> {
    let p = "m/48'/0'/0'/2'";
    let keys = [
        key_text_of(c, &t.app.session.keys, 0, NewKind::Multi)?,
        cosigner(1, p),
        cosigner(2, p),
    ];
    Ok(Kit {
        id: "guide",
        name: "New wallet 1",
        descriptor: NewKind::Multi.descriptor(2, &keys),
        cosigner: Some(1),
        fee: 2_140,
    })
}

/// Costs what making and spending from a 2-of-3 costs. The guide's key
/// with two cosigners' xpubs (`docs/guide/walk/multisig-with-others.md`):
/// Key 1's xpub as a code; Keys with Key 2 a cosigner's xpub file; the
/// coordinator's wallet file in Files; Who signs and The other signatures
/// with one key here; the PSBT whose wallet is not registered, and
/// Register this wallet; a Taproot wallet whose key path another key
/// spends, its sheet over Files.
pub(super) fn with_others(c: &Compute, t: &mut Tour) -> Result<(), String> {
    key_session(t);
    let p = "m/48'/0'/0'/2'";
    files_in(
        t,
        &[
            ("5d388376-xpub.txt", cosigner(1, p).into_bytes()),
            ("cf2e083d-xpub.txt", cosigner(2, p).into_bytes()),
        ],
        &[],
    );
    let at = |t: &Tour, name: &str| {
        t.app
            .inbox()
            .iter()
            .position(|i| i.name == name)
            .ok_or(format!("{name} is not in Files"))
    };
    let multi = kind_at(NewKind::Multi)?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    if t.app.create().and_then(|c| c.open) == Some(cstep::KIND) {
        t.press(Action::CKind(multi));
    }
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CKeyQr(0));
    t.shot("multisig-with-others-xpub-qr")?;
    t.press(Action::Cancel);
    t.press(Action::CSlotFile(1, at(t, "5d388376-xpub.txt")?));
    t.shot("multisig-with-others-keys")?;
    let kit = shared_kit(c, t)?;

    // The coordinator's wallet, loaded from Files.
    key_session(t);
    files_in(
        t,
        &[(
            "sparrow-new-wallet-1.txt",
            kit.descriptor.clone().into_bytes(),
        )],
        &[],
    );
    t.press(Action::Nav(Screen::Files));
    let file = at(t, "sparrow-new-wallet-1.txt")?;
    t.shot_marking("multisig-with-others-load", Action::LoadWallet(file))?;
    t.press(Action::LoadWallet(file));
    let w = last_wallet(t)?;
    let psbt = testkit::unsigned(&kit)?.to_bytes();
    files_in(t, &[("payment.psbt", psbt)], &[]);
    t.press(Action::Nav(Screen::Files));
    t.press(Action::StartSpend(at(t, "payment.psbt")?));
    t.press(Action::StepNext(step::TRANSACTION));
    if t.app.spend().and_then(|s| s.open) == Some(step::CHECK) {
        t.press(Action::StepNext(step::CHECK));
    }
    t.shot_marking(
        "multisig-with-others-who-signs",
        Action::StepNext(step::SIGNERS),
    )?;
    t.press(Action::StepNext(step::SIGNERS));
    t.press(Action::SignHere);
    if t.app.spend().and_then(|s| s.open) == Some(step::SIGN) {
        t.press(Action::StepNext(step::SIGN));
    }
    t.shot("multisig-with-others-other-signatures")?;
    t.press(Action::RemoveWallet(w));

    // A first spend before the wallet is here: register it.
    key_session(t);
    let with = testkit::unsigned_with_xpubs(&kit)?.to_bytes();
    files_in(t, &[("payment.psbt", with)], &[]);
    t.press(Action::Nav(Screen::Files));
    t.press(Action::StartSpend(at(t, "payment.psbt")?));
    if t.app
        .spend()
        .is_some_and(|s| s.open != Some(step::TRANSACTION))
    {
        t.press(Action::Step(step::TRANSACTION));
    }
    scroll_to(t, Action::RegisterOpen);
    t.press(Action::RegisterOpen);
    t.shot("multisig-with-others-register")?;
    t.shot("wallets-register")?;

    // A Taproot wallet whose key path is a key not of its own.
    key_session(t);
    let tp = "m/48'/0'/0'/3'";
    let tree = format!(
        "tr(79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798,multi_a(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        key_text_of(c, &t.app.session.keys, 0, NewKind::TapMulti)?,
        cosigner(1, tp),
        cosigner(2, tp)
    );
    files_in(t, &[("shared-wallet.txt", tree.into_bytes())], &[]);
    t.press(Action::Nav(Screen::Files));
    t.press(Action::LoadWallet(at(t, "shared-wallet.txt")?));
    t.shot("multisig-with-others-key-path")?;
    t.shot("wallets-key-path")
}

/// Costs two mnemonics' checksums. §5.6: Remove every key; the 2-of-3's
/// card short of keys; Scan a key; test key 2's SeedQR, then the dice
/// key's.
pub(super) fn scan_key(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Nav(Screen::Settings));
    t.shot_marking("5-6-settings", Action::RemoveKeys)?;
    t.press(Action::RemoveKeys);
    t.shot_marking(
        "multisig-with-others-keys-removed",
        Action::Nav(Screen::Wallets),
    )?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot_marking("5-6-card-short", Action::ScanKey(w))?;
    t.press(Action::ScanKey(w));
    crate::tours::site_tour::camera(t);
    t.shot("5-6-camera")?;
    let zebra = faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0);
    t.app.event(Event::Scanned {
        bytes: seedqr(c, &zebra)?,
    });
    t.pump();
    t.shot_after("5-6-one-more", 0)?;
    t.app.event(Event::Scanned {
        bytes: seedqr(c, DICE_WORDS)?,
    });
    t.pump();
    t.shot_after("5-6-can-spend", 0)?;
    t.shot("multisig-with-others-card-can-spend")
}
