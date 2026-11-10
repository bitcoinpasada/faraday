//! Faraday as a check on another signer (`docs/FLOWS.md`, Nonce check):
//! a PSBT signed elsewhere, or a finished transaction, brought in by
//! Files, the camera or Tools opens its nonce check rather than signing.
//! Every signature is checked against what the transaction holds, a
//! damaged one and two under one nonce are refused, a signature under a
//! key loaded here is recomputed and named by the nonce rule that made
//! it, and a final raw transaction says which keys it could not match. A
//! PSBT a key here is still to sign goes to signing as before.
//!
//! The signer elsewhere is rust-bitcoin's secp256k1 signing over
//! rust-bitcoin's sighash, not Faraday's signing code.

use crate::common::add_key;
use faraday_bip::bitcoin::secp256k1::{
    Message, PublicKey as Point, Scalar, Secp256k1, SecretKey, ecdsa::Signature,
};
use faraday_bip::bitcoin::sighash::{EcdsaSighashType, SighashCache};
use faraday_bip::bitcoin::{OutPoint, PublicKey, ecdsa, hashes::Hash};
use faraday_core::strings::EN;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::Unmatched;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_psbt::verify::{NonceMode, Unchecked, Verdict};
use faraday_shell_api::{App, Event};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_inbox(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

/// How the signer elsewhere picks its nonce.
#[derive(Clone, Copy)]
enum How {
    /// RFC 6979, ground to a low R (Bitcoin Core, Sparrow).
    LowR,
    /// RFC 6979's first nonce (Trezor, the RFC's own vectors).
    First,
    /// One nonce chosen for every input: the key given away.
    SameNonce,
    /// A nonce of the signer's own choosing, one input: a signature that
    /// verifies and that no nonce rule here makes.
    OwnNonce,
}

/// ECDSA with the nonce `k` chosen here rather than by RFC 6979, so that
/// one nonce can sign two inputs. `s = k⁻¹(z + r·d)`, the inverse by
/// Fermat over the curve order.
#[expect(
    clippy::disallowed_methods,
    reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
)]
fn ecdsa_with_k(d: &SecretKey, z: [u8; 32], k: &SecretKey) -> Signature {
    const N_MINUS_2: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36,
        0x41, 0x3f,
    ];
    let secp = Secp256k1::new();
    let point = Point::from_secret_key(&secp, k);
    let x: [u8; 32] = point.serialize()[1..33].try_into().unwrap();
    let r = SecretKey::from_slice(&x).unwrap();
    let z = SecretKey::from_slice(&z).unwrap();
    let zrd = d
        .mul_tweak(&Scalar::from(r))
        .unwrap()
        .add_tweak(&Scalar::from(z))
        .unwrap();
    let mut inverse: Option<SecretKey> = None;
    for byte in N_MINUS_2 {
        for bit in (0..8).rev() {
            if let Some(acc) = inverse {
                inverse = Some(acc.mul_tweak(&Scalar::from(acc)).unwrap());
            }
            if byte >> bit & 1 == 1 {
                inverse = Some(match inverse {
                    Some(acc) => acc.mul_tweak(&Scalar::from(*k)).unwrap(),
                    None => *k,
                });
            }
        }
    }
    let s = zrd.mul_tweak(&Scalar::from(inverse.unwrap())).unwrap();
    let mut compact = [0u8; 64];
    compact[..32].copy_from_slice(&r.secret_bytes());
    compact[32..].copy_from_slice(&s.secret_bytes());
    let mut sig = Signature::from_compact(&compact).unwrap();
    sig.normalize_s();
    sig
}

/// Savings' spend (2 of 3) signed elsewhere by test key 2 on every input,
/// as a SeedSigner or a hardware wallet returns it: a partial signature
/// on each, nothing finished. One nonce on every input needs two inputs
/// under one key: a second, spending the first one's output script, is
/// added where the spend has one.
fn signed_elsewhere(how: How) -> faraday_psbt::Psbt {
    let c = testkit::compute();
    let (mut psbt, _) = testkit::spend_of(&kit("savings")).unwrap();
    if matches!(how, How::SameNonce) && psbt.unsigned_tx().input.len() < 2 {
        let inner = psbt.inner_mut();
        let mut txin = inner.unsigned_tx.input[0].clone();
        txin.previous_output =
            OutPoint::new(txin.previous_output.txid, txin.previous_output.vout + 1);
        inner.unsigned_tx.input.push(txin);
        let mut second = inner.inputs[0].clone();
        second.non_witness_utxo = None;
        inner.inputs.push(second);
    }
    let words = testkit::test_words("zebra");
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    let zebra =
        faraday_bip::keys::MasterKey::from_seed(&c, &m.to_seed(&c, b"").unwrap(), testkit::net());
    let tx = psbt.unsigned_tx().clone();
    let secp = Secp256k1::new();
    let k = SecretKey::from_slice(&[7u8; 32]).unwrap();
    for i in 0..tx.input.len() {
        let input = psbt.inner().inputs[i].clone();
        let (pk, (_, path)) = input
            .bip32_derivation
            .iter()
            .find(|(_, (fp, _))| fp.to_bytes() == zebra.fingerprint().0)
            .expect("test key 2 is a key of Savings");
        let derived = zebra.derive(&c, path);
        let ws = input.witness_script.clone().expect("a P2WSH input");
        let value = input.witness_utxo.as_ref().expect("its output").value;
        let hash = SighashCache::new(&tx)
            .p2wsh_signature_hash(i, &ws, value, EcdsaSighashType::All)
            .unwrap();
        let msg = Message::from_digest(hash.to_byte_array());
        let signature = match how {
            How::LowR => secp.sign_ecdsa_low_r(&msg, derived.secret_key()),
            How::First => secp.sign_ecdsa(&msg, derived.secret_key()),
            How::SameNonce | How::OwnNonce => {
                ecdsa_with_k(derived.secret_key(), hash.to_byte_array(), &k)
            }
        };
        psbt.inner_mut().inputs[i].partial_sigs.insert(
            PublicKey::new(*pk),
            ecdsa::Signature {
                signature,
                sighash_type: EcdsaSighashType::All,
            },
        );
    }
    psbt
}

/// The same PSBT with input 0's signature changed by one bit of `s`.
fn damaged(mut psbt: faraday_psbt::Psbt) -> faraday_psbt::Psbt {
    let sig = psbt.inner_mut().inputs[0]
        .partial_sigs
        .values_mut()
        .next()
        .unwrap();
    let mut compact = sig.signature.serialize_compact();
    compact[63] ^= 1;
    sig.signature = Signature::from_compact(&compact).unwrap();
    psbt
}

/// The nonce check of Inbox file 0, opened from Files, though nothing in
/// the file is a signature that stands (Files then offers Sign).
fn opened(app: &mut Faraday) -> &faraday_core::decode::OutsideCheck {
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        0,
    )));
    assert_eq!(app.screen, Screen::NonceCheck);
    app.outside_check().expect("the check")
}

/// The nonce check of Inbox file 0, opened from Files.
fn checked(app: &mut Faraday) -> &faraday_core::decode::OutsideCheck {
    app.press(Action::Nav(Screen::Files));
    assert!(
        app.checked_not_signed(0),
        "Files offers Sign, not Nonce check"
    );
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        0,
    )));
    assert_eq!(app.screen, Screen::NonceCheck);
    app.outside_check().expect("the check")
}

#[test]
fn a_psbt_signed_elsewhere_is_checked_valid_and_its_nonces_need_that_device() {
    let mut app = with_inbox(vec![(
        "seedsigner.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    let o = checked(&mut app);
    assert!(!o.check.rows.is_empty(), "a signature per input");
    assert!(o.check.rows.iter().all(|r| r.verdict == Verdict::Valid));
    assert!(
        o.check
            .rows
            .iter()
            .all(|r| r.recomputed.is_none() && r.unmatched.is_none()),
        "no key here: signed elsewhere"
    );
    assert!(o.check.sound());
    assert!(o.finished.is_none(), "one of two signatures is no spend");
    assert_eq!(o.back, Screen::Files);
    // Carried on to the next signer as it came.
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    assert!(app.outbox.iter().any(|i| i.name == "seedsigner.psbt"));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
}

/// A checked transaction whose file went beside another check's of the
/// same name offers Remove on its own file, and a check not yet saved
/// offers Save and not Remove on a file of its name that is not its.
#[test]
fn a_second_checks_remove_takes_off_its_own_file() {
    let mut app = with_inbox(vec![(
        "seedsigner.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    checked(&mut app);
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    app.outbox[0].bytes = b"another file".to_vec();
    checked(&mut app);
    let _ = app.frame();
    assert!(
        !app.offers(Action::OutboxRemove(0)),
        "the file of the name is not this check's"
    );
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    let own = app
        .outbox
        .iter()
        .position(|i| i.name == "seedsigner-2.psbt")
        .expect("beside the other");
    let _ = app.frame();
    assert!(app.offers(Action::OutboxRemove(own)));
    assert!(!app.offers(Action::OutboxRemove(0)));
}

#[test]
fn with_the_other_signers_seed_loaded_here_each_signature_is_recomputed() {
    for how in [How::LowR, How::First] {
        let psbt = signed_elsewhere(how);
        // RFC 6979's first nonce that already gives a low R is the low-R
        // rule's too, and is named as that.
        let high_r = psbt.inner().inputs[0]
            .partial_sigs
            .values()
            .next()
            .is_some_and(|s| s.signature.serialize_compact()[0] >= 0x80);
        let mode = match how {
            How::First if high_r => NonceMode::First,
            _ => NonceMode::LowR,
        };
        let mut app = with_inbox(vec![("signed.psbt", psbt.to_bytes())]);
        add_key(&mut app, "zebra");
        let o = checked(&mut app);
        assert!(
            o.check
                .rows
                .iter()
                .all(|r| r.recomputed == Some(Some(mode))),
            "{:?}",
            o.check
                .rows
                .iter()
                .map(|r| r.recomputed)
                .collect::<Vec<_>>()
        );
        assert!(o.check.sound());
    }
}

#[test]
fn a_psbt_a_key_here_is_still_to_sign_goes_to_signing() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    add_key(&mut app, "bacon");
    assert!(!app.checked_not_signed(0), "test key 1 has not signed");
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Nonce check")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.sheet, Some(Sheet::Scan), "nothing to check: the camera");
}

#[test]
fn a_damaged_signature_refuses_the_transaction() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        damaged(signed_elsewhere(How::LowR)).to_bytes(),
    )]);
    let o = checked(&mut app);
    assert!(o.check.wrong());
    assert_eq!(o.check.rows[0].verdict, Verdict::Invalid);
    assert!(o.finished.is_none());
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    assert!(
        app.outbox.is_empty(),
        "a refused transaction is not carried on"
    );
}

#[test]
fn two_signatures_under_one_nonce_are_refused() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    let o = checked(&mut app);
    assert!(!o.check.reused.is_empty(), "the shared nonce is named");
    assert!(o.check.wrong());
    assert!(!o.check.sound());
}

/// Savings signed by test keys 1 (here) and 2 (elsewhere), finished:
/// the raw transaction, and the unsigned PSBT that states its amounts.
fn finished_savings() -> (String, Vec<u8>) {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut b = with_inbox(vec![
        ("spend.psbt", unsigned.to_bytes()),
        ("savings-wallet.txt", k.descriptor.into_bytes()),
        ("from-a.psbt", signed_elsewhere(How::LowR).to_bytes()),
    ]);
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
    b.press(Action::LoadWallet(at(&b, "savings-wallet.txt")));
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
    b.press(Action::SignHere);
    b.press(Action::Collect(at(&b, "from-a.psbt")));
    let hex = b.spend().unwrap().spend.finished_hex().expect("finished");
    (hex, unsigned.to_bytes())
}

#[test]
fn a_final_raw_transaction_says_which_keys_it_could_not_match() {
    let (hex, unsigned) = finished_savings();
    // Alone: no output it spends is stated, so nothing can be checked,
    // and the headline says so.
    let mut alone = with_inbox(vec![("broadcast-me.txn", hex.clone().into_bytes())]);
    let o = checked(&mut alone);
    assert!(o.check.unread > 0);
    assert!(
        o.check.headline().contains("not checked"),
        "{}",
        o.check.headline()
    );
    assert!(!o.check.sound() && !o.check.wrong());

    // Beside the PSBT that states the amounts: every signature valid,
    // and its key not matched, a raw transaction naming no origin.
    let mut beside = with_inbox(vec![
        ("broadcast-me.txn", hex.into_bytes()),
        ("spend.psbt", unsigned),
    ]);
    add_key(&mut beside, "bacon");
    let o = checked(&mut beside);
    assert!(o.check.rows.iter().all(|r| r.verdict == Verdict::Valid));
    assert!(
        o.check
            .rows
            .iter()
            .all(|r| r.unmatched == Some(Unmatched::NoOrigin))
    );
    assert!(o.finished.is_some());
    beside.press(Action::NonceCheck(faraday_core::decode::CheckPress::Decode));
    assert_eq!(beside.screen, Screen::Decode);
    assert!(beside.decode().unwrap().decoded.fee.is_some());
    beside.press(Action::NonceCheck(faraday_core::decode::CheckPress::Back));
    assert_eq!(beside.screen, Screen::NonceCheck);
    beside.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    assert!(beside.outbox.iter().any(|i| i.name == "broadcast-me.txn"));
}

#[test]
fn a_signed_psbt_scanned_opens_its_check_and_goes_back_where_the_scan_started() {
    let psbt = signed_elsewhere(How::LowR);
    let mut app = with_inbox(Vec::new());
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: psbt.to_base64().into_bytes(),
    });
    assert_eq!(app.screen, Screen::NonceCheck);
    assert_eq!(app.sheet, None);
    assert_eq!(app.outside_check().unwrap().back, Screen::Home);
    let _ = app.frame();
}

#[test]
fn tools_nonce_check_opens_a_signed_file_here() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Nonce check")
        .unwrap();
    assert!(app.tile_opens(faraday_core::catalog::TILES[tile].go));
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::NonceCheck);
    assert_eq!(app.outside_check().unwrap().back, Screen::Catalog);
}

#[test]
fn a_key_loaded_since_the_check_opened_is_counted_on_the_way_back() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        0,
    )));
    assert!(
        app.outside_check()
            .unwrap()
            .check
            .rows
            .iter()
            .all(|r| r.recomputed.is_none())
    );
    add_key(&mut app, "zebra");
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Back));
    assert_eq!(app.screen, Screen::NonceCheck);
    assert!(
        app.outside_check()
            .unwrap()
            .check
            .rows
            .iter()
            .all(|r| r.recomputed == Some(Some(NonceMode::LowR))),
        "the check follows the keys"
    );
}

/// A PSBT of the kit, signed by keys the session holds and so by the
/// device that is to read it as signed elsewhere.
fn signed_by_seeds(id: &str) -> (Kit, Vec<u8>) {
    let k = kit(id);
    let c = testkit::compute();
    let mut s = testkit::session();
    for (w, l) in testkit::TEST_SEEDS {
        s.keys
            .add_words(&c, &testkit::test_words(w), l, None)
            .unwrap();
    }
    s.add_wallet(
        &c,
        "W",
        &k.descriptor,
        "Test wallet",
        faraday_core::wallet::Origin::Named,
    )
    .unwrap();
    let mut made = testkit::unsigned(&k).unwrap();
    let accepted: Vec<_> = s
        .inspect(&c, &made)
        .warnings
        .iter()
        .map(|w| w.kind.clone())
        .collect();
    s.sign_with(
        &c,
        &mut made,
        &faraday_crypto::Zeroizing::new([7; 32]),
        &mut None,
        None,
        &accepted,
    )
    .unwrap();
    (k, made.to_bytes())
}

/// The Nonce check line of the Finish card as drawn.
fn finish_line(app: &mut Faraday) -> Option<String> {
    let _ = app.frame();
    app.drawn_texts()
        .into_iter()
        .find(|t| t.starts_with("Nonce check ·"))
}

#[test]
fn the_finish_cards_nonce_line_follows_a_key_loaded_while_it_is_drawn() {
    for id in ["spending", "taproot"] {
        let (k, bytes) = signed_by_seeds(id);
        let mut app = with_inbox(vec![
            ("spend.psbt", bytes),
            ("w.txt", k.descriptor.as_bytes().to_vec()),
        ]);
        let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
        app.press(Action::LoadWallet(at(&app, "w.txt")));
        app.press(Action::StartSpend(at(&app, "spend.psbt")));
        app.press(Action::Step(faraday_core::wallet::step::FINISH));
        assert_eq!(
            finish_line(&mut app).as_deref(),
            Some("Nonce check · 1 signature valid · 0 recomputed here"),
            "{id}: no key here"
        );
        add_key(&mut app, "bacon");
        assert_eq!(
            finish_line(&mut app).as_deref(),
            Some("Nonce check · 1 signature valid · 1 recomputed here"),
            "{id}: the key came in after the card was drawn"
        );
        app.press(Action::RemoveKeys);
        // Keys removed close the spend: there is no card left to read.
        assert!(app.spend().is_none(), "{id}");
        assert_eq!(finish_line(&mut app), None, "{id}");
    }
}

#[test]
fn a_collect_that_brings_a_changed_signature_shows_its_verdict_in_the_check() {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut app = with_inbox(vec![
        ("spend.psbt", unsigned.to_bytes()),
        ("savings-wallet.txt", k.descriptor.into_bytes()),
        ("from-a.psbt", signed_elsewhere(How::LowR).to_bytes()),
        ("from-b.psbt", signed_elsewhere(How::OwnNonce).to_bytes()),
    ]);
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
    app.press(Action::LoadWallet(at(&app, "savings-wallet.txt")));
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    app.press(Action::SignHere);
    app.press(Action::Collect(at(&app, "from-a.psbt")));
    add_key(&mut app, "zebra");
    assert_eq!(
        finish_line(&mut app).as_deref(),
        Some("Nonce check · 2 signatures valid · 2 recomputed here")
    );
    // The file brings zebra's signature again, made with a nonce of its
    // own: it verifies, and is not the one this device makes.
    app.press(Action::Collect(at(&app, "from-b.psbt")));
    assert_eq!(
        finish_line(&mut app).as_deref(),
        Some("Nonce check · a signature is wrong")
    );
}

/// One input of a finalized taproot transaction.
#[derive(Clone)]
enum Tap {
    /// A key-path spend, signed over the whole transaction.
    Key,
    /// A script-path spend of a hash-lock leaf: `[preimage, leaf,
    /// control]`, the control block with a byte flipped when `spoil`.
    Lock { preimage: Vec<u8>, spoil: bool },
    /// The real key-path signature hidden as `[sig, a made-up leaf,
    /// control]` under a key-path output: a signature that is right, in
    /// a witness that confirms nowhere.
    Hidden,
    /// A script-path spend of a leaf signed by key A, with a real signature
    /// or a forged one, final or as a partial signature in the fields.
    Leaf {
        kind: LeafKind,
        forged: bool,
        partial: bool,
    },
}

/// The leaf of a [`Tap::Leaf`].
#[derive(Clone, Copy)]
enum LeafKind {
    /// `<A> CHECKSIG` of leaf version 0xc0: a Tap miniscript, run.
    Run,
    /// `OP_CODESEPARATOR <A> CHECKSIG`: signed over the script after it.
    Separator,
    /// `OP_DROP <A> CHECKSIG` spent with a data element.
    Drop,
    /// `<A> CHECKSIG` of leaf version 0xc2.
    Future,
}

/// The leaf of one input made by [`taproot_psbt`], ready to sign.
type LeafMade = (
    Tap,
    faraday_bip::bitcoin::ScriptBuf,
    faraday_bip::bitcoin::taproot::LeafVersion,
    faraday_bip::bitcoin::taproot::ControlBlock,
);

/// Signs a leaf with key A and files it as the final witness, or as a
/// partial signature, on input `i`.
fn sign_leaf(
    inner: &mut faraday_bip::bitcoin::Psbt,
    i: usize,
    tx: &faraday_bip::bitcoin::Transaction,
    prevouts: &[faraday_bip::bitcoin::TxOut],
    pair: &faraday_bip::bitcoin::secp256k1::Keypair,
    made: LeafMade,
) {
    use faraday_bip::bitcoin::Witness;
    use faraday_bip::bitcoin::sighash::{Prevouts, TapSighashType};
    let (
        Tap::Leaf {
            kind,
            forged,
            partial,
        },
        script,
        version,
        control,
    ) = made
    else {
        return;
    };
    let leaf_hash = faraday_bip::bitcoin::taproot::TapLeafHash::from_script(&script, version);
    let separator = matches!(kind, LeafKind::Separator).then_some(0);
    let hash = SighashCache::new(tx)
        .taproot_signature_hash(
            i,
            &Prevouts::All(prevouts),
            None,
            Some((leaf_hash, separator.unwrap_or(0xffff_ffff))),
            TapSighashType::Default,
        )
        .unwrap();
    let mut sig = Secp256k1::new()
        .sign_schnorr_no_aux_rand(&Message::from_digest(hash.to_byte_array()), pair)
        .serialize()
        .to_vec();
    if forged {
        sig[5] ^= 1;
    }
    if partial {
        let (a, _) = pair.x_only_public_key();
        inner.inputs[i]
            .tap_scripts
            .insert(control, (script, version));
        inner.inputs[i].tap_script_sigs.insert(
            (a, leaf_hash),
            faraday_bip::bitcoin::taproot::Signature::from_slice(&sig).unwrap(),
        );
    } else {
        let mut elements = vec![sig];
        if matches!(kind, LeafKind::Drop) {
            elements.push(vec![0x42]);
        }
        elements.push(script.to_bytes());
        elements.push(control.serialize());
        inner.inputs[i].final_script_witness = Some(Witness::from_slice(&elements));
    }
}

/// The output and the leaf of a [`Tap::Leaf`] of `kind`, under internal key `internal`.
fn leaf_output(
    secp: &Secp256k1<faraday_bip::bitcoin::secp256k1::All>,
    pair: &faraday_bip::bitcoin::secp256k1::Keypair,
    internal: faraday_bip::bitcoin::XOnlyPublicKey,
    tap: Tap,
    kind: LeafKind,
) -> (faraday_bip::bitcoin::ScriptBuf, LeafMade) {
    use faraday_bip::bitcoin::opcodes::all::{OP_CHECKSIG, OP_CODESEPARATOR, OP_DROP};
    use faraday_bip::bitcoin::taproot::{LeafVersion, TaprootBuilder};
    let (a, _) = pair.x_only_public_key();
    let mut b = faraday_bip::bitcoin::script::Builder::new();
    b = match kind {
        LeafKind::Separator => b.push_opcode(OP_CODESEPARATOR),
        LeafKind::Drop => b.push_opcode(OP_DROP),
        _ => b,
    };
    let script = b.push_x_only_key(&a).push_opcode(OP_CHECKSIG).into_script();
    let version = match kind {
        LeafKind::Future => LeafVersion::from_consensus(0xc2).unwrap(),
        _ => LeafVersion::TapScript,
    };
    let info = TaprootBuilder::new()
        .add_leaf_with_ver(0, script.clone(), version)
        .unwrap()
        .finalize(secp, internal)
        .unwrap();
    let control = info.control_block(&(script.clone(), version)).unwrap();
    let spk = faraday_bip::bitcoin::ScriptBuf::new_p2tr(secp, internal, info.merkle_root());
    (spk, (tap, script, version, control))
}

/// The output and the final witness of a hash-lock leaf of [`Tap::Lock`].
fn lock_output(
    secp: &Secp256k1<faraday_bip::bitcoin::secp256k1::All>,
    internal: faraday_bip::bitcoin::XOnlyPublicKey,
    preimage: &[u8],
    spoil: bool,
) -> (
    faraday_bip::bitcoin::ScriptBuf,
    faraday_bip::bitcoin::Witness,
) {
    use faraday_bip::bitcoin::taproot::{LeafVersion, TaprootBuilder};
    let hash = faraday_bip::bitcoin::hashes::sha256::Hash::hash(preimage);
    let script = faraday_bip::bitcoin::script::Builder::new()
        .push_opcode(faraday_bip::bitcoin::opcodes::all::OP_SHA256)
        .push_slice(hash.to_byte_array())
        .push_opcode(faraday_bip::bitcoin::opcodes::all::OP_EQUAL)
        .into_script();
    let info = TaprootBuilder::new()
        .add_leaf(0, script.clone())
        .unwrap()
        .finalize(secp, internal)
        .unwrap();
    let mut control = info
        .control_block(&(script.clone(), LeafVersion::TapScript))
        .unwrap()
        .serialize();
    if spoil {
        control[2] ^= 1;
    }
    let spk = faraday_bip::bitcoin::ScriptBuf::new_p2tr(secp, internal, info.merkle_root());
    let witness =
        faraday_bip::bitcoin::Witness::from_slice(&[preimage.to_vec(), script.to_bytes(), control]);
    (spk, witness)
}

/// A PSBT whose inputs are finalized as `inputs` says, each spending a
/// real taproot output.
fn taproot_psbt(inputs: &[Tap]) -> faraday_psbt::Psbt {
    taproot_psbt_under(inputs, &SecretKey::from_slice(&[7; 32]).unwrap())
}

/// [`taproot_psbt`], the internal and leaf key being that of `secret`.
fn taproot_psbt_under(inputs: &[Tap], secret: &SecretKey) -> faraday_psbt::Psbt {
    use faraday_bip::bitcoin::key::TapTweak;
    use faraday_bip::bitcoin::secp256k1::Keypair;
    use faraday_bip::bitcoin::sighash::{Prevouts, TapSighashType};
    use faraday_bip::bitcoin::{
        Amount, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness, absolute::LockTime,
        transaction::Version,
    };
    let secp = Secp256k1::new();
    let pair = Keypair::from_secret_key(&secp, secret);
    let (internal, _) = pair.x_only_public_key();
    let signer = pair.tap_tweak(&secp, None).to_keypair();
    let other = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[9; 32]).unwrap());
    let (other_key, _) = other.x_only_public_key();
    let mut prevouts = Vec::new();
    let mut leaves = Vec::new();
    let mut pending: Vec<Option<LeafMade>> = Vec::new();
    for tap in inputs {
        let (spk, leaf) = match tap {
            Tap::Leaf { kind, .. } => {
                let (spk, made) = leaf_output(&secp, &pair, other_key, tap.clone(), *kind);
                pending.push(Some(made));
                (spk, None)
            }
            Tap::Key | Tap::Hidden => {
                pending.push(None);
                (ScriptBuf::new_p2tr(&secp, internal, None), None)
            }
            Tap::Lock { preimage, spoil } => {
                pending.push(None);
                let (spk, witness) = lock_output(&secp, other_key, preimage, *spoil);
                (spk, Some(witness))
            }
        };
        prevouts.push(TxOut {
            value: Amount::from_sat(50_000),
            script_pubkey: spk,
        });
        leaves.push(leaf);
    }
    let hidden: Vec<bool> = inputs.iter().map(|t| matches!(t, Tap::Hidden)).collect();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: (0..inputs.len())
            .map(|i| TxIn {
                previous_output: OutPoint::new(
                    faraday_bip::bitcoin::Txid::from_byte_array([1; 32]),
                    i as u32,
                ),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: prevouts[0].script_pubkey.clone(),
        }],
    };
    let mut inner = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx.clone()).unwrap();
    for (i, leaf) in leaves.into_iter().enumerate() {
        inner.inputs[i].witness_utxo = Some(prevouts[i].clone());
        if let Some(done) = pending[i].clone() {
            sign_leaf(&mut inner, i, &tx, &prevouts, &pair, done);
            continue;
        }
        inner.inputs[i].final_script_witness = Some(leaf.unwrap_or_else(|| {
            let hash = SighashCache::new(&tx)
                .taproot_key_spend_signature_hash(
                    i,
                    &Prevouts::All(&prevouts),
                    TapSighashType::Default,
                )
                .unwrap();
            let sig = Secp256k1::new()
                .sign_schnorr_no_aux_rand(&Message::from_digest(hash.to_byte_array()), &signer);
            if hidden[i] {
                let mut control = vec![0xc0];
                control.extend_from_slice(&internal.serialize());
                Witness::from_slice(&[sig.serialize().to_vec(), vec![0x51], control])
            } else {
                Witness::from_slice(&[sig.serialize().to_vec()])
            }
        }));
    }
    faraday_psbt::Psbt::from(inner)
}

fn leaf(kind: LeafKind, forged: bool, partial: bool) -> Tap {
    Tap::Leaf {
        kind,
        forged,
        partial,
    }
}

/// A script-path signature on a leaf Faraday does not run, final or filed as
/// a partial signature under a leaf the input files with a control block
/// that commits, is not refused, true or forged (a forged one reads not
/// checked, a true one valid): Review and Collect take the file, and no key
/// counts as signed. A forged signature
/// on a Tap miniscript leaf (which is run) is refused.
#[test]
fn a_signature_on_a_leaf_that_is_not_run_is_not_refused() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    for (what, kind, run) in [
        ("a separator leaf", LeafKind::Separator, false),
        ("a leaf with OP_DROP and data", LeafKind::Drop, false),
        ("a future leaf version", LeafKind::Future, false),
        ("a Tap miniscript leaf", LeafKind::Run, true),
    ] {
        for forged in [false, true] {
            let what = format!("{what}, forged {forged}");
            // Final: Review and finish.
            let psbt = taproot_psbt(&[leaf(kind, forged, false)]);
            let app = testkit::started();
            let check = app.session.nonce_check(&c, &psbt);
            let refused = run && forged;
            assert_eq!(check.wrong(), refused, "{what}");
            assert_eq!(check.not_evaluated.is_empty(), run, "{what}");
            if !run {
                assert!(!check.sound(), "{what}");
            }
            // Collect: a cosigner's file onto an unsigned own copy.
            let mut own = psbt.clone();
            own.inner_mut().inputs[0].final_script_witness = None;
            let mut spend = Spend::new(own, "own.psbt");
            let merged = spend.collect(&c, &session, &psbt.to_bytes(), "cosigner.psbt");
            assert_eq!(merged.is_err(), refused, "{what}");
            if !run {
                assert!(merged.unwrap().is_empty(), "{what}: no key counted");
                assert!(
                    spend.signing(&c, &session, None, None).signers.is_empty(),
                    "{what}: no signer"
                );
            }
            let mut spend = Spend::new(psbt, "t.psbt");
            assert_eq!(spend.finish().is_err(), refused, "{what}");
            // Partial: the signature in the fields, read by Review.
            let psbt = taproot_psbt(&[leaf(kind, forged, true)]);
            let rows: Vec<_> = faraday_psbt::verify::verify_signatures(
                &psbt,
                &faraday_psbt::Context {
                    network: faraday_bip::keys::Network::Mainnet,
                    keys: &[],
                    wallets: &[],
                    musig_session: None,
                },
            )
            .into_iter()
            .flat_map(|i| i.signatures)
            .collect();
            assert_eq!(rows.len(), 1, "{what}");
            let invalid = rows[0].verdict.is_invalid();
            assert_eq!(invalid, refused, "{what}");
            if forged {
                assert_ne!(rows[0].verdict, Verdict::Valid, "{what}");
            }
        }
    }
}

#[test]
fn a_64_byte_preimage_row_is_marked_a_preimage_and_a_true_unsupported_row_is_not() {
    let app = with_inbox(vec![]);
    let check = |tap| {
        app.session
            .nonce_check(&testkit::compute(), &taproot_psbt(&[tap]))
    };
    let rows = check(Tap::Lock {
        preimage: vec![0xcd; 64],
        spoil: true,
    })
    .rows;
    assert!(rows.iter().any(|r| r.preimage && r.unchecked()));
    assert!(check(Tap::Key).rows.iter().all(|r| !r.preimage));
}

fn lock(spoil: bool) -> Tap {
    Tap::Lock {
        preimage: vec![0xcd; 32],
        spoil,
    }
}

/// Inbox file 0 is offered as checked rather than signed.
fn offered(psbt: &faraday_psbt::Psbt) -> bool {
    with_inbox(vec![("t.psbt", psbt.to_bytes())]).checked_not_signed(0)
}

#[test]
fn a_failing_witness_with_no_signature_row_is_not_sound() {
    // Input 0 a valid key-path spend, input 1 a hash-lock leaf whose
    // control block does not commit: no row for it, and still wrong.
    let psbt = taproot_psbt(&[Tap::Key, lock(true)]);
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    assert!(
        o.check
            .rows
            .iter()
            .all(|r| r.input == 0 && r.verdict == Verdict::Valid)
    );
    assert_eq!(o.check.mismatched, vec![1]);
    assert!(o.check.wrong());
    assert!(!o.check.sound());
    assert!(o.finished.is_none(), "no finished transaction is offered");
    let good = taproot_psbt(&[Tap::Key, lock(false)]);
    let mut app = with_inbox(vec![("t.psbt", good.to_bytes())]);
    let o = checked(&mut app);
    assert!(o.check.mismatched.is_empty());
    // The hash lock's leaf is no miniscript, so it is not run: nothing
    // refuses it, and it is not called sound.
    assert!(!o.check.sound());
    assert!(!o.check.wrong());
}

#[test]
fn a_single_failing_witness_is_not_checked_as_signed() {
    let failing = taproot_psbt(&[lock(true)]);
    let committing = taproot_psbt(&[lock(false)]);
    assert!(!offered(&failing));
    assert!(offered(&committing));
    assert!(!faraday_core::wallet::PsbtSeen::of(&failing).signed);
    assert!(faraday_core::wallet::PsbtSeen::of(&committing).signed);
    let mixed = taproot_psbt(&[Tap::Key, lock(true)]);
    assert!(faraday_core::wallet::PsbtSeen::of(&mixed).signed);
}

#[test]
fn a_cosigners_failing_witness_is_not_merged_and_a_true_one_is() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    let unfinalized = |p: &faraday_psbt::Psbt| {
        let mut own = p.clone();
        own.inner_mut().inputs[0].final_script_witness = None;
        Spend::new(own, "own.psbt")
    };
    for (what, tap) in [
        ("no row", lock(true)),
        (
            "a proven preimage only",
            Tap::Lock {
                preimage: vec![0xcd; 64],
                spoil: true,
            },
        ),
    ] {
        let theirs = taproot_psbt(&[tap]);
        let mut spend = unfinalized(&theirs);
        let before = spend.psbt.to_bytes();
        let why = spend
            .collect(&c, &session, &theirs.to_bytes(), "cosigner.psbt")
            .expect_err(what);
        assert_eq!(why, mismatch_text(0), "{what}");
        assert!(!why.contains("cosigner.psbt"), "{what}");
        assert_eq!(spend.psbt.to_bytes(), before, "{what}");
        // The spend keeps its own state: nothing to finish for lack of a signature.
        assert_eq!(
            spend.finish().unwrap_err(),
            EN.wallet_not_enough_signatures,
            "{what}"
        );
        assert!(spend.finished.is_none(), "{what}");
    }
    for tap in [Tap::Key, lock(false)] {
        let theirs = taproot_psbt(&[tap]);
        let mut spend = unfinalized(&theirs);
        spend
            .collect(&c, &session, &theirs.to_bytes(), "cosigner.psbt")
            .expect("a true spend merges");
        assert!(spend.psbt.inner().inputs[0].final_script_witness.is_some());
        assert!(spend.finish().is_ok());
    }
    // A spend opened on a failing witness finishes nothing either.
    let mut spend = Spend::new(taproot_psbt(&[lock(true)]), "own.psbt");
    assert_eq!(spend.finish().unwrap_err(), mismatch_text(0));
    assert!(spend.finished.is_none());
}

/// A finalized input whose signature is by another key than the one it
/// names finishes nothing, beside the red card the check shows.
#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
)]
fn a_final_signature_that_does_not_verify_finishes_nothing() {
    use faraday_bip::bitcoin::{ScriptBuf, WPubkeyHash};
    use faraday_core::wallet::Spend;
    let secp = Secp256k1::new();
    let signer = |seed: u8| {
        let sk = SecretKey::from_slice(&[seed; 32]).unwrap();
        let key = PublicKey::new(Point::from_secret_key(&secp, &sk));
        let mut sig = secp
            .sign_ecdsa(&Message::from_digest([9; 32]), &sk)
            .serialize_der()
            .to_vec();
        sig.push(1);
        (key, sig)
    };
    let (key, _) = signer(5);
    let (_, other_sig) = signer(6);
    let spk = ScriptBuf::new_p2wpkh(&WPubkeyHash::hash(&key.to_bytes()));
    let psbt = second_input(spk, ScriptBuf::new(), vec![other_sig, key.to_bytes()]);
    let app = testkit::started();
    let check = app.session.nonce_check(&testkit::compute(), &psbt);
    assert!(!check.sound());
    let mut spend = Spend::new(psbt, "t.psbt");
    assert_eq!(
        spend.finish().unwrap_err(),
        EN.psbt_final_signature_refused.replace("{}", "0")
    );
    assert!(spend.finished.is_none());
}

/// What `p2wsh_spend` signs with: a function of the key's seed.
type Signer<'a> = &'a dyn Fn(u8) -> Vec<u8>;

/// A final p2wsh input of `script` at `sequence`, its witness made from
/// the signatures `make` signs over the input's digest (seeds 5 and 6
/// are the keys A and B).
fn p2wsh_spend(
    script: &faraday_bip::bitcoin::ScriptBuf,
    sequence: u32,
    make: &dyn Fn(Signer) -> Vec<Vec<u8>>,
) -> faraday_psbt::Psbt {
    p2wsh_spend_v(script, sequence, 2, make)
}

/// [`p2wsh_spend`] in a transaction of `version`.
fn p2wsh_spend_v(
    script: &faraday_bip::bitcoin::ScriptBuf,
    sequence: u32,
    version: i32,
    make: &dyn Fn(Signer) -> Vec<Vec<u8>>,
) -> faraday_psbt::Psbt {
    use faraday_bip::bitcoin::{ScriptBuf, Sequence};
    let mut psbt = second_input(
        ScriptBuf::new_p2wsh(&script.wscript_hash()),
        ScriptBuf::new(),
        vec![],
    );
    psbt.inner_mut().unsigned_tx.input[0].sequence = Sequence(sequence);
    psbt.inner_mut().unsigned_tx.version = faraday_bip::bitcoin::transaction::Version(version);
    let value = psbt.inner().inputs[0].witness_utxo.as_ref().unwrap().value;
    let digest = SighashCache::new(&psbt.inner().unsigned_tx)
        .p2wsh_signature_hash(0, script, value, EcdsaSighashType::All)
        .unwrap()
        .to_byte_array();
    let secp = Secp256k1::new();
    let sign = |seed: u8| {
        let sk = SecretKey::from_slice(&[seed; 32]).unwrap();
        let mut sig = secp
            .sign_ecdsa(&Message::from_digest(digest), &sk)
            .serialize_der()
            .to_vec();
        sig.push(1);
        sig
    };
    let mut parts = make(&sign);
    parts.push(script.to_bytes());
    psbt.inner_mut().inputs[0].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&parts));
    psbt
}

#[expect(
    clippy::disallowed_methods,
    reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
)]
fn key_of(seed: u8) -> Vec<u8> {
    let secp = Secp256k1::new();
    let sk = SecretKey::from_slice(&[seed; 32]).unwrap();
    PublicKey::new(Point::from_secret_key(&secp, &sk)).to_bytes()
}

/// A final p2wpkh input of the key of `seed`, signed by it.
fn p2wpkh_spend(seed: u8) -> faraday_psbt::Psbt {
    use faraday_bip::bitcoin::{ScriptBuf, WPubkeyHash};
    let key = key_of(seed);
    let spk = ScriptBuf::new_p2wpkh(&WPubkeyHash::hash(&key));
    let mut psbt = second_input(spk.clone(), ScriptBuf::new(), vec![]);
    let value = psbt.inner().inputs[0].witness_utxo.as_ref().unwrap().value;
    let hash = SighashCache::new(&psbt.inner().unsigned_tx)
        .p2wpkh_signature_hash(0, &spk, value, EcdsaSighashType::All)
        .unwrap();
    let mut sig = Secp256k1::new()
        .sign_ecdsa(
            &Message::from_digest(hash.to_byte_array()),
            &SecretKey::from_slice(&[seed; 32]).unwrap(),
        )
        .serialize_der()
        .to_vec();
    sig.push(1);
    psbt.inner_mut().inputs[0].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&[sig, key]));
    psbt
}

/// `IF SHA256 <hash> EQUALVERIFY <a> CHECKSIG ELSE 144 CSV DROP <b> CHECKSIG
/// ENDIF`: not a miniscript.
fn hash_time_lock(a: &[u8], b: &[u8], hash: [u8; 32]) -> faraday_bip::bitcoin::ScriptBuf {
    use faraday_bip::bitcoin::opcodes::all::*;
    use faraday_bip::bitcoin::script::{Builder, PushBytesBuf};
    let push = |b: Builder, k: &[u8]| b.push_slice(PushBytesBuf::try_from(k.to_vec()).unwrap());
    let s = Builder::new()
        .push_opcode(OP_IF)
        .push_opcode(OP_SHA256)
        .push_slice(hash)
        .push_opcode(OP_EQUALVERIFY);
    let s = push(s, a).push_opcode(OP_CHECKSIG).push_opcode(OP_ELSE);
    let s = s.push_int(144).push_opcode(OP_CSV).push_opcode(OP_DROP);
    push(s, b)
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_ENDIF)
        .into_script()
}

/// A script Faraday does not run is not a refusal and not sound: a hash
/// time lock's true and false spends all finish and none reads sound or
/// wrong; a miniscript, a p2wpkh and a 2-of-2 still read sound.
#[test]
fn a_script_that_is_not_run_reads_not_sound_and_is_not_refused() {
    use faraday_bip::bitcoin::ScriptBuf;
    use faraday_bip::bitcoin::opcodes::all::*;
    use faraday_bip::bitcoin::script::{Builder, PushBytesBuf};
    use faraday_core::wallet::Spend;
    let push = |b: Builder, k: Vec<u8>| b.push_slice(PushBytesBuf::try_from(k).unwrap());
    let (a, b) = (key_of(5), key_of(6));
    let preimage = [7u8; 32];
    let hash = faraday_bip::bitcoin::hashes::sha256::Hash::hash(&preimage).to_byte_array();
    let lock = hash_time_lock(&a, &b, hash);
    let mut wrong_preimage = preimage;
    wrong_preimage[0] ^= 1;
    let app = testkit::started();
    let c = testkit::compute();
    let finishes = |what: &str, psbt: faraday_psbt::Psbt, sound: bool| {
        let check = app.session.nonce_check(&c, &psbt);
        assert!(!check.wrong(), "{what}");
        assert_eq!(check.sound(), sound, "{what}");
        assert_eq!(check.not_evaluated.is_empty(), sound, "{what}");
        let mut spend = Spend::new(psbt, "t.psbt");
        assert!(spend.finish().is_ok(), "{what}");
    };
    finishes(
        "hash branch",
        p2wsh_spend(&lock, 0xffff_ffff, &|s| {
            vec![s(5), preimage.to_vec(), vec![1]]
        }),
        false,
    );
    finishes(
        "time branch",
        p2wsh_spend(&lock, 144, &|s| vec![s(6), vec![]]),
        false,
    );
    finishes(
        "wrong preimage",
        p2wsh_spend(&lock, 0xffff_ffff, &|s| {
            vec![s(5), wrong_preimage.to_vec(), vec![1]]
        }),
        false,
    );
    finishes(
        "A's signature on the time branch",
        p2wsh_spend(&lock, 144, &|s| vec![s(5), vec![]]),
        false,
    );
    finishes(
        "a sequence below the lock",
        p2wsh_spend(&lock, 143, &|s| vec![s(6), vec![]]),
        false,
    );
    // Scripts Faraday reads stay sound.
    let pk = Builder::new();
    let pk = push(pk, a.clone()).push_opcode(OP_CHECKSIG).into_script();
    finishes(
        "miniscript",
        p2wsh_spend(&pk, 0xffff_ffff, &|s| vec![s(5)]),
        true,
    );
    let two = {
        let s = Builder::new().push_int(2);
        let s = push(s, a.clone());
        push(s, b.clone())
            .push_int(2)
            .push_opcode(OP_CHECKMULTISIG)
            .into_script()
    };
    finishes(
        "2-of-2",
        p2wsh_spend(&two, 0xffff_ffff, &|s| vec![vec![], s(5), s(6)]),
        true,
    );
    finishes("p2wpkh", p2wpkh_spend(5), true);
    // A p2sh puzzle and an anchor are not run either, and not refused.
    let puzzle = Builder::new()
        .push_opcode(OP_ADD)
        .push_opcode(OP_PUSHNUM_5)
        .push_opcode(OP_EQUAL)
        .into_script();
    let script_sig = Builder::new()
        .push_opcode(OP_PUSHNUM_2)
        .push_opcode(OP_PUSHNUM_3)
        .push_slice(PushBytesBuf::try_from(puzzle.to_bytes()).unwrap())
        .into_script();
    let psbt = second_input(
        ScriptBuf::new_p2sh(&puzzle.script_hash()),
        script_sig,
        vec![],
    );
    finishes("p2sh puzzle", psbt, false);
    let anchor = ScriptBuf::from(vec![0x51, 0x02, 0x4e, 0x73]);
    finishes(
        "anchor",
        second_input(anchor, ScriptBuf::new(), vec![vec![]]),
        false,
    );
}

fn mismatch_text(input: usize) -> String {
    EN.psbt_warn_witness_mismatch
        .replace("{}", &input.to_string())
}

/// One input spending `spk` by `script_sig` and `witness`.
fn second_input(
    spk: faraday_bip::bitcoin::ScriptBuf,
    script_sig: faraday_bip::bitcoin::ScriptBuf,
    witness: Vec<Vec<u8>>,
) -> faraday_psbt::Psbt {
    let mut psbt = taproot_psbt(&[Tap::Key]);
    let input = &mut psbt.inner_mut().inputs[0];
    input.witness_utxo.as_mut().unwrap().script_pubkey = spk;
    input.final_script_sig = Some(script_sig);
    input.final_script_witness = Some(faraday_bip::bitcoin::Witness::from_slice(&witness));
    psbt
}

#[test]
fn a_previous_transaction_of_another_txid_takes_nothing_out_of_the_check() {
    use faraday_bip::bitcoin::{Amount, Sequence, Transaction, TxIn, TxOut, Witness};
    use faraday_core::wallet::Session;
    use faraday_core::wallet::Spend;
    let mut psbt = taproot_psbt(&[Tap::Key, lock(true)]);
    let out = psbt.inner().inputs[1].witness_utxo.clone().unwrap();
    // Another transaction than the one input 1 spends, with that output
    // at that index.
    let other = Transaction {
        version: faraday_bip::bitcoin::transaction::Version::TWO,
        lock_time: faraday_bip::bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: Default::default(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut {
                value: Amount::from_sat(1),
                script_pubkey: Default::default(),
            },
            out,
        ],
    };
    psbt.inner_mut().inputs[1].witness_utxo = None;
    psbt.inner_mut().inputs[1].non_witness_utxo = Some(other);
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    assert_eq!(o.check.unread, 1);
    assert!(!o.check.sound());
    assert!(o.finished.is_none(), "no finished transaction is offered");
    let mut spend = Spend::new(psbt.clone(), "t.psbt");
    assert_eq!(
        spend.finish().unwrap_err(),
        EN.psbt_final_prev_unread.replace("{}", "1")
    );
    assert!(spend.finished.is_none());
    // A cosigner's copy carrying it is refused where the own copy states
    // no previous output of that input.
    let mut own = psbt.clone();
    own.inner_mut().inputs[1].non_witness_utxo = None;
    own.inner_mut().inputs[1].final_script_witness = None;
    let mut spend = Spend::new(own, "own.psbt");
    let before = spend.psbt.to_bytes();
    let why = spend
        .collect(
            &testkit::compute(),
            &Session::default(),
            &psbt.to_bytes(),
            "c.psbt",
        )
        .unwrap_err();
    assert_eq!(
        why,
        EN.wallet_file_states_false
            .replacen("{}", "c.psbt", 1)
            .replacen("{}", "1", 1)
    );
    assert_eq!(spend.psbt.to_bytes(), before);
}

#[test]
fn a_witness_of_one_element_under_an_output_that_cannot_confirm_is_refused() {
    use faraday_bip::bitcoin::{ScriptBuf, opcodes::all::OP_PUSHNUM_1, script::Builder};
    use faraday_core::wallet::Spend;
    let sig = vec![1u8; 64];
    // An output key off the curve, spent with [sig] and with [sig, annex].
    let off_curve = Builder::new()
        .push_opcode(OP_PUSHNUM_1)
        .push_slice([0xff; 32])
        .into_script();
    // A p2sh of all zeros, its scriptSig pushing a p2tr script.
    let p2tr = ScriptBuf::new_p2tr(
        &Secp256k1::new(),
        faraday_bip::bitcoin::key::Keypair::from_secret_key(
            &Secp256k1::new(),
            &SecretKey::from_slice(&[7; 32]).unwrap(),
        )
        .x_only_public_key()
        .0,
        None,
    );
    let wrapped = Builder::new()
        .push_slice(<&faraday_bip::bitcoin::script::PushBytes>::try_from(p2tr.as_bytes()).unwrap())
        .into_script();
    let zero_hash =
        ScriptBuf::new_p2sh(&faraday_bip::bitcoin::ScriptHash::from_byte_array([0; 20]));
    let psbts = [
        (
            "off the curve",
            second_input(off_curve.clone(), ScriptBuf::new(), vec![sig.clone()]),
        ),
        (
            "off the curve, annex",
            second_input(
                off_curve,
                ScriptBuf::new(),
                vec![sig.clone(), vec![0x50, 1]],
            ),
        ),
        ("p2sh of zeros", second_input(zero_hash, wrapped, vec![sig])),
    ];
    for (what, psbt) in psbts {
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        app.press(Action::Nav(Screen::Files));
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
            0,
        )));
        assert_eq!(app.screen, Screen::NonceCheck, "{what}");
        let o = app.outside_check().expect("the check");
        assert_eq!(o.check.mismatched, vec![0], "{what}");
        assert!(!o.check.sound(), "{what}");
        assert!(o.finished.is_none(), "{what}");
        assert_eq!(o.check.headline(), EN.nonce_check_witness_wrong, "{what}");
        let mut spend = Spend::new(psbt, "t.psbt");
        assert_eq!(spend.finish().unwrap_err(), mismatch_text(0), "{what}");
    }
}

#[test]
fn a_cosigners_clean_file_is_not_blamed_for_a_failing_witness_already_in_the_spend() {
    use faraday_core::wallet::{Session, Spend};
    let failing = taproot_psbt(&[Tap::Key, lock(true)]);
    let mut clean = taproot_psbt(&[Tap::Key, lock(false)]);
    clean.inner_mut().inputs[1].final_script_witness = None;
    let mut spend = Spend::new(failing, "own.psbt");
    let why = spend
        .collect(
            &testkit::compute(),
            &Session::default(),
            &clean.to_bytes(),
            "clean.psbt",
        )
        .unwrap_err();
    assert_eq!(why, mismatch_text(1));
    assert!(!why.contains("clean.psbt"));
}

#[test]
fn a_failing_witness_alone_is_not_called_a_wrong_signature() {
    let mut app = with_inbox(vec![(
        "t.psbt",
        taproot_psbt(&[Tap::Key, lock(true)]).to_bytes(),
    )]);
    let o = checked(&mut app);
    assert_eq!(o.check.headline(), EN.nonce_check_witness_wrong);
    let shown = app.drawn_texts().join(" ");
    assert!(shown.contains(EN.nonce_screen_refused_witness), "{shown}");
    assert!(!shown.contains(EN.nonce_screen_refused), "{shown}");
    assert!(shown.contains(&mismatch_text(1)), "{shown}");
    // A wrong signature as well: the signature's words.
    let mut bad = taproot_psbt(&[Tap::Key, lock(true)]);
    let w = bad.inner().inputs[0].final_script_witness.clone().unwrap();
    let mut sig = w.nth(0).unwrap().to_vec();
    sig[63] ^= 1;
    bad.inner_mut().inputs[0].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&[sig]));
    let mut app = with_inbox(vec![("t.psbt", bad.to_bytes())]);
    let o = checked(&mut app);
    assert_eq!(o.check.headline(), EN.nonce_check_nonce_check);
    let shown = app.drawn_texts().join(" ");
    assert!(shown.contains(EN.nonce_screen_refused), "{shown}");
}

#[test]
fn the_reason_a_spend_is_not_finished_is_the_refusal_and_holds_it() {
    use faraday_core::wallet::Spend;
    let spend = Spend::new(taproot_psbt(&[lock(true)]), "t.psbt");
    // The finalize of a copy alone accepts this witness.
    assert_eq!(spend.finish_reason(None), None);
    let refused = spend.refused();
    assert_eq!(refused.as_deref(), Some(mismatch_text(0).as_str()));
    assert_eq!(spend.finish_reason(refused.as_deref()), refused);
    let true_one = Spend::new(taproot_psbt(&[lock(false)]), "t.psbt");
    assert_eq!(true_one.refused(), None);
    assert_eq!(true_one.finish_reason(None), None);
}

/// The previous transaction of another txid than input `vout` spends, with
/// `out` at index `vout`.
fn other_previous(
    out: faraday_bip::bitcoin::TxOut,
    vout: usize,
) -> faraday_bip::bitcoin::Transaction {
    use faraday_bip::bitcoin::{Amount, Sequence, Transaction, TxIn, TxOut, Witness};
    let mut output = vec![
        TxOut {
            value: Amount::from_sat(1),
            script_pubkey: Default::default(),
        };
        vout
    ];
    output.push(out);
    Transaction {
        version: faraday_bip::bitcoin::transaction::Version::TWO,
        lock_time: faraday_bip::bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: Default::default(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output,
    }
}

/// A PSBT whose input 1 (a hash-lock spend) has its previous output
/// unreadable in one of three ways, input 0 a key-path spend (or, for the
/// right txid without the output, another hash-lock spend: the outpoint
/// it changes would spoil a signature).
fn unread_final(how: &str) -> faraday_psbt::Psbt {
    use faraday_bip::bitcoin::{Amount, Transaction, TxOut, transaction::Version};
    if how == "no output at the index" {
        // The txid is the one the input names, but that transaction has
        // no output at the index.
        let mut psbt = taproot_psbt(&[lock(false), lock(false)]);
        let previous = Transaction {
            version: Version::TWO,
            lock_time: faraday_bip::bitcoin::absolute::LockTime::ZERO,
            input: Vec::new(),
            output: vec![TxOut {
                value: Amount::from_sat(1),
                script_pubkey: Default::default(),
            }],
        };
        psbt.inner_mut().unsigned_tx.input[1].previous_output =
            OutPoint::new(previous.compute_txid(), 1);
        let i = &mut psbt.inner_mut().inputs[1];
        i.witness_utxo = None;
        i.non_witness_utxo = Some(previous);
        return psbt;
    }
    let mut psbt = taproot_psbt(&[Tap::Key, lock(false)]);
    let out = psbt.inner().inputs[1].witness_utxo.clone().unwrap();
    match how {
        "another txid" => {
            let i = &mut psbt.inner_mut().inputs[1];
            i.witness_utxo = None;
            i.non_witness_utxo = Some(other_previous(out, 1));
        }
        "none" => psbt.inner_mut().inputs[1].witness_utxo = None,
        _ => unreachable!(),
    }
    psbt
}

#[test]
fn a_final_input_whose_output_cannot_be_read_offers_nothing_to_carry_on() {
    use faraday_core::decode::CheckPress;
    for how in ["another txid", "none", "no output at the index"] {
        let psbt = unread_final(how);
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = checked(&mut app);
        assert_eq!(o.check.unread, 1, "{how}");
        assert!(!o.check.sound() && o.finished.is_none(), "{how}");
        assert!(o.check.refused(o.finished.is_some()), "{how}");
        let shown = app.drawn_texts().join(" | ");
        assert!(
            shown.contains(EN.nonce_screen_refused_unread),
            "{how}: {shown}"
        );
        for offer in [
            EN.nonce_screen_more_wanted,
            EN.nonce_screen_refused,
            EN.vault_save_to_a_stick,
            EN.step_show_as_qr,
        ] {
            assert!(!shown.contains(offer), "{how}: {offer}");
        }
        app.press(Action::NonceCheck(CheckPress::Out));
        assert!(app.outbox.is_empty(), "{how}");
        app.press(Action::NonceCheck(CheckPress::Qr));
        assert_eq!(app.screen, Screen::NonceCheck, "{how}");
        assert_eq!(app.sheet, None, "{how}: no QR sheet opens");
    }
    // A raw transaction with an input it cannot check keeps its offers.
    let (hex, _) = finished_savings();
    let mut raw = with_inbox(vec![("broadcast-me.txn", hex.into_bytes())]);
    let o = checked(&mut raw);
    assert!(o.check.unread > 0 && o.finished.is_some());
    assert!(!o.check.refused(o.finished.is_some()));
    let shown = raw.drawn_texts().join(" | ");
    assert!(shown.contains(EN.vault_save_to_a_stick), "{shown}");
    assert!(shown.contains(EN.step_show_as_qr), "{shown}");
    // A PSBT with no final input stays offered, and both offers work.
    let mut app = with_inbox(vec![("t.psbt", signed_elsewhere(How::LowR).to_bytes())]);
    let o = checked(&mut app);
    assert!(o.finished.is_none());
    assert!(!o.check.refused(o.finished.is_some()));
    let shown = app.drawn_texts().join(" | ");
    assert!(shown.contains(EN.vault_save_to_a_stick), "{shown}");
    assert!(shown.contains(EN.step_show_as_qr), "{shown}");
    app.press(Action::NonceCheck(CheckPress::Qr));
    assert_eq!(app.sheet, Some(Sheet::Qr), "the QR sheet opens");
}

#[test]
fn a_real_signature_in_a_failing_witness_is_a_witness_that_is_wrong() {
    for inputs in [vec![Tap::Hidden], vec![Tap::Key, Tap::Hidden]] {
        let at = inputs.len() - 1;
        let mut app = with_inbox(vec![("t.psbt", taproot_psbt(&inputs).to_bytes())]);
        let o = opened(&mut app);
        assert!(
            o.check
                .rows
                .iter()
                .any(|r| r.input == at && r.verdict == Verdict::Invalid)
        );
        assert_eq!(o.check.mismatched, vec![at]);
        assert!(o.check.only_witness_wrong());
        assert_eq!(o.check.headline(), EN.nonce_check_witness_wrong);
        let shown = app.drawn_texts().join(" ");
        assert!(shown.contains(EN.nonce_screen_refused_witness), "{shown}");
    }
    // A truly wrong signature on another input still reads as one.
    let mut bad = taproot_psbt(&[Tap::Key, Tap::Hidden]);
    let w = bad.inner().inputs[0].final_script_witness.clone().unwrap();
    let mut sig = w.nth(0).unwrap().to_vec();
    sig[63] ^= 1;
    bad.inner_mut().inputs[0].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&[sig]));
    let mut app = with_inbox(vec![("t.psbt", bad.to_bytes())]);
    let o = checked(&mut app);
    assert!(!o.check.only_witness_wrong());
    assert_eq!(o.check.headline(), EN.nonce_check_nonce_check);
}

#[test]
fn collect_blames_the_input_and_not_the_file_where_the_cosigner_states_no_output() {
    use faraday_core::wallet::{Session, Spend};
    let theirs = taproot_psbt(&[Tap::Hidden]);
    let mut own = theirs.clone();
    own.inner_mut().inputs[0].final_script_witness = None;
    let mut theirs = theirs;
    theirs.inner_mut().inputs[0].witness_utxo = None;
    let mut spend = Spend::new(own, "own.psbt");
    let why = spend
        .collect(
            &testkit::compute(),
            &Session::default(),
            &theirs.to_bytes(),
            "c.psbt",
        )
        .unwrap_err();
    assert_eq!(why, mismatch_text(0));
}

#[test]
fn collect_refuses_an_own_final_input_only_if_it_is_still_unread_after_the_merge() {
    use faraday_core::wallet::{Session, Spend};
    let clean = taproot_psbt(&[lock(false)]);
    let mut own = clean.clone();
    own.inner_mut().inputs[0].witness_utxo = None;
    let session = Session::default();
    let c = testkit::compute();
    // The file states the output the own copy lacks: merged, finishes.
    let mut spend = Spend::new(own.clone(), "own.psbt");
    spend
        .collect(&c, &session, &clean.to_bytes(), "c.psbt")
        .expect("the merged PSBT stands");
    assert!(spend.psbt.inner().inputs[0].witness_utxo.is_some());
    assert!(spend.finish().is_ok());
    // A file that states none leaves it unread: refused, nothing merged.
    let mut spend = Spend::new(own.clone(), "own.psbt");
    let before = spend.psbt.to_bytes();
    let why = spend
        .collect(&c, &session, &own.to_bytes(), "c.psbt")
        .unwrap_err();
    assert_eq!(why, EN.psbt_final_prev_unread.replace("{}", "0"));
    assert_eq!(spend.psbt.to_bytes(), before);
}

#[test]
fn a_previous_transaction_of_another_txid_beside_a_witness_utxo_is_refused() {
    use faraday_core::wallet::{Session, Spend};
    let mut psbt = taproot_psbt(&[Tap::Key, lock(false)]);
    let out = psbt.inner().inputs[1].witness_utxo.clone().unwrap();
    psbt.inner_mut().inputs[1].non_witness_utxo = Some(other_previous(out, 1));
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    assert_eq!(o.check.unread, 1);
    assert!(!o.check.sound());
    assert!(o.finished.is_none());
    assert!(!offered_after(&psbt));
    let mut spend = Spend::new(psbt.clone(), "t.psbt");
    assert_eq!(
        spend.finish().unwrap_err(),
        EN.psbt_final_prev_unread.replace("{}", "1")
    );
    assert!(spend.finished.is_none());
    // Over an own witness_utxo, the cosigner's file is not merged.
    let mut own = psbt.clone();
    own.inner_mut().inputs[1].non_witness_utxo = None;
    own.inner_mut().inputs[1].final_script_witness = None;
    let mut spend = Spend::new(own, "own.psbt");
    let why = spend
        .collect(
            &testkit::compute(),
            &Session::default(),
            &psbt.to_bytes(),
            "c.psbt",
        )
        .unwrap_err();
    assert_eq!(
        why,
        EN.wallet_file_states_false
            .replacen("{}", "c.psbt", 1)
            .replacen("{}", "1", 1)
    );
}

/// The nonce check of `psbt` offers to carry on.
fn offered_after(psbt: &faraday_psbt::Psbt) -> bool {
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    !o.check.refused(o.finished.is_some())
}

#[test]
fn a_raw_transaction_beside_a_psbt_whose_previous_transaction_is_another_one_reads_no_output() {
    use faraday_bip::bitcoin::{Amount, TxOut};
    use faraday_core::decode::CheckPress;
    let (hex, unsigned) = finished_savings();
    let mut psbt = faraday_psbt::Psbt::parse_bytes(&testkit::compute(), &unsigned).expect("psbt");
    for (i, input) in psbt.inner_mut().inputs.iter_mut().enumerate() {
        input.witness_utxo = None;
        // Rich outputs under another txid: amounts read from them would
        // show a fee.
        let rich = TxOut {
            value: Amount::from_sat(10_000_000_000),
            script_pubkey: Default::default(),
        };
        input.non_witness_utxo = Some(other_previous(rich, i));
    }
    let mut app = with_inbox(vec![
        ("broadcast-me.txn", hex.into_bytes()),
        ("spend.psbt", psbt.to_bytes()),
    ]);
    add_key(&mut app, "bacon");
    let o = checked(&mut app);
    assert!(o.check.unread > 0, "no output is read from another txid");
    app.press(Action::NonceCheck(CheckPress::Decode));
    assert_eq!(app.screen, Screen::Decode);
    assert!(app.decode().unwrap().decoded.fee.is_none());
}

#[test]
fn a_native_taproot_input_with_a_script_sig_and_no_witness_is_refused() {
    use faraday_bip::bitcoin::script::Builder;
    use faraday_core::wallet::Spend;
    let mut psbt = taproot_psbt(&[Tap::Key]);
    let input = &mut psbt.inner_mut().inputs[0];
    input.final_script_witness = None;
    input.final_script_sig = Some(Builder::new().push_int(1).into_script());
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = opened(&mut app);
    assert_eq!(o.check.mismatched, vec![0]);
    assert!(!o.check.sound() && o.finished.is_none());
    let mut spend = Spend::new(psbt, "t.psbt");
    assert_eq!(spend.finish().unwrap_err(), mismatch_text(0));
    assert!(spend.finished.is_none());
}

/// A witness no output of a p2tr key takes, on input 0.
fn junk_witness(psbt: &mut faraday_psbt::Psbt) {
    psbt.inner_mut().inputs[0].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&[
            vec![1u8; 10],
            vec![2u8; 33],
        ]));
}

#[test]
fn a_file_and_a_spend_are_each_blamed_for_the_witness_failing_under_their_own_output() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    let bare = |p: &mut faraday_psbt::Psbt| {
        for input in &mut p.inner_mut().inputs {
            input.final_script_witness = None;
        }
    };
    // The file's input 1 fails under its own output; its input 0 states
    // no output, and fails only under the one the spend states.
    let mut file = taproot_psbt(&[Tap::Key, lock(true)]);
    junk_witness(&mut file);
    file.inner_mut().inputs[0].witness_utxo = None;
    let mut own = taproot_psbt(&[Tap::Key, lock(false)]);
    bare(&mut own);
    let mut spend = Spend::new(own, "own.psbt");
    let why = spend
        .collect(&c, &session, &file.to_bytes(), "file.psbt")
        .unwrap_err();
    assert_eq!(why, mismatch_text(1));
    // The same the other way round: the spend's input 1 fails under its
    // own output, the file's input 0 only under the spend's.
    let mut own = taproot_psbt(&[Tap::Key, lock(true)]);
    own.inner_mut().inputs[0].final_script_witness = None;
    let mut file = taproot_psbt(&[Tap::Key, lock(false)]);
    bare(&mut file);
    junk_witness(&mut file);
    file.inner_mut().inputs[0].witness_utxo = None;
    let mut spend = Spend::new(own, "own.psbt");
    let why = spend
        .collect(&c, &session, &file.to_bytes(), "file.psbt")
        .unwrap_err();
    assert_eq!(why, mismatch_text(1));
}

#[test]
fn a_native_taproot_spend_with_a_script_sig_is_refused() {
    use faraday_bip::bitcoin::script::Builder;
    use faraday_core::wallet::Spend;
    for (what, tap) in [("key path", Tap::Key), ("script path", lock(false))] {
        let mut psbt = taproot_psbt(&[tap]);
        psbt.inner_mut().inputs[0].final_script_sig =
            Some(Builder::new().push_slice([1u8]).into_script());
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = opened(&mut app);
        assert_eq!(o.check.mismatched, vec![0], "{what}");
        assert!(!o.check.sound() && o.finished.is_none(), "{what}");
        let mut spend = Spend::new(psbt, "t.psbt");
        assert_eq!(spend.finish().unwrap_err(), mismatch_text(0), "{what}");
    }
}

#[test]
fn the_reason_shown_for_a_spend_follows_what_a_collect_changes() {
    use faraday_core::wallet::Spend;
    let clean = taproot_psbt(&[lock(false)]);
    let mut own = clean.clone();
    own.inner_mut().inputs[0].witness_utxo = None;
    let unread = EN.psbt_final_prev_unread.replace("{}", "0");
    let mut app = with_inbox(vec![
        ("own.psbt", own.to_bytes()),
        ("clean.psbt", clean.to_bytes()),
    ]);
    app.press(Action::StartSpend(0));
    let state = |app: &mut Faraday| {
        let _ = app.frame();
        let s = app.spend().expect("a spend");
        (s.refused.clone(), Spend::refused(&s.spend))
    };
    let (kept, fresh) = state(&mut app);
    assert_eq!(kept.as_deref(), Some(unread.as_str()));
    assert_eq!(kept, fresh);
    app.press(Action::Collect(1));
    let (kept, fresh) = state(&mut app);
    assert_eq!(fresh, None);
    assert_eq!(kept, fresh, "the kept reason is the fresh one");
}

/// A finalized taproot key-path spend of a previous transaction (of the
/// right txid) that pays 50,000 sat: the PSBT states `stated` as its
/// `witness_utxo`, and the signature is made over `signed`, the amount a
/// node reads being the 50,000. With `previous` false the previous
/// transaction is left out.
fn key_spend_with_previous(signed: u64, stated: u64, previous: bool) -> faraday_psbt::Psbt {
    use faraday_bip::bitcoin::key::TapTweak;
    use faraday_bip::bitcoin::secp256k1::Keypair;
    use faraday_bip::bitcoin::sighash::{Prevouts, TapSighashType};
    use faraday_bip::bitcoin::{
        Amount, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness, absolute::LockTime,
        transaction::Version,
    };
    let secp = Secp256k1::new();
    let pair = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[7; 32]).unwrap());
    let (internal, _) = pair.x_only_public_key();
    let signer = pair.tap_tweak(&secp, None).to_keypair();
    let spk = ScriptBuf::new_p2tr(&secp, internal, None);
    let out = |sat| TxOut {
        value: Amount::from_sat(sat),
        script_pubkey: spk.clone(),
    };
    let funding = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![out(50_000)],
    };
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(funding.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![out(10_000)],
    };
    let hash = SighashCache::new(&tx)
        .taproot_key_spend_signature_hash(
            0,
            &Prevouts::All(&[out(signed)]),
            TapSighashType::Default,
        )
        .unwrap();
    let sig = secp.sign_schnorr_no_aux_rand(&Message::from_digest(hash.to_byte_array()), &signer);
    let mut inner = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    inner.inputs[0].witness_utxo = Some(out(stated));
    inner.inputs[0].non_witness_utxo = previous.then_some(funding);
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[sig.serialize().to_vec()]));
    faraday_psbt::Psbt::from(inner)
}

fn disagree_text(input: usize) -> String {
    EN.psbt_final_prev_disagree
        .replace("{}", &input.to_string())
}

/// A final input whose `witness_utxo` states another amount than its
/// previous transaction is refused whatever the signature was made over:
/// the nonce check says the two disagree (and not that a signature is
/// wrong or that the output cannot be read), offers nothing, and `finish`
/// gives the same reason.
#[test]
fn a_witness_utxo_that_disagrees_with_the_previous_transaction_is_refused_with_its_own_words() {
    use faraday_core::wallet::Spend;
    for (signed, what) in [
        (99_999, "signed over the stated amount"),
        (50_000, "signed over the previous transaction's amount"),
    ] {
        let psbt = key_spend_with_previous(signed, 99_999, true);
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = checked(&mut app);
        assert_eq!((o.check.unread, o.check.disagree), (1, 1), "{what}");
        assert!(!o.check.sound(), "{what}");
        assert!(o.finished.is_none(), "{what}");
        assert!(o.check.refused(false), "{what}");
        // A signature made over the stated amount is wrong against the
        // previous transaction a node reads, and is said to be; one made
        // over the right amount is not, and the screen says only that the
        // two outputs differ.
        let (head, line) = if signed == 99_999 {
            (
                EN.nonce_check_nonce_check.to_string(),
                EN.nonce_screen_refused,
            )
        } else {
            (
                EN.nonce_disagree_head.replace("{}", "1"),
                EN.nonce_screen_refused_disagree,
            )
        };
        assert_eq!(o.check.headline(), head, "{what}");
        let shown = app.drawn_texts().join(" | ");
        assert!(shown.contains(line), "{what}: {shown}");
        for absent in [
            EN.nonce_screen_refused_unread,
            EN.nonce_screen_more_wanted,
            EN.vault_save_to_a_stick,
        ] {
            assert!(!shown.contains(absent), "{what}: {absent}");
        }
        if signed == 50_000 {
            assert!(!shown.contains(EN.nonce_screen_refused), "{what}: {shown}");
        }
        let mut spend = Spend::new(psbt.clone(), "t.psbt");
        assert_eq!(spend.finish().unwrap_err(), disagree_text(0), "{what}");
        assert!(spend.finished.is_none(), "{what}");
        assert_eq!(spend.refused(), Some(disagree_text(0)), "{what}");
    }
    // Agreeing, with the previous transaction or without it, it stands.
    for (stated, previous) in [(50_000, true), (50_000, false)] {
        let psbt = key_spend_with_previous(50_000, stated, previous);
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = checked(&mut app);
        assert_eq!((o.check.unread, o.check.disagree), (0, 0));
        assert!(o.check.sound() && o.finished.is_some());
        assert!(Spend::new(psbt, "t.psbt").finish().is_ok());
    }
}

/// Decode reads the output a PSBT spends as the PSBT's own checks do: the
/// previous transaction of the right txid first, none where the previous
/// transaction is another one's.
#[test]
fn decode_reads_the_previous_transaction_before_a_witness_utxo() {
    use faraday_bip::bitcoin::Amount;
    let amount = |p: &faraday_psbt::Psbt| {
        faraday_core::decode::prevouts_of(p)[0]
            .as_ref()
            .map(|o| o.value)
    };
    let lying = key_spend_with_previous(50_000, 99_999, true);
    assert_eq!(amount(&lying), Some(Amount::from_sat(50_000)));
    let only_witness = key_spend_with_previous(50_000, 99_999, false);
    assert_eq!(amount(&only_witness), Some(Amount::from_sat(99_999)));
    let mut wrong = key_spend_with_previous(50_000, 50_000, true);
    let funding = wrong.inner().inputs[0].non_witness_utxo.clone().unwrap();
    wrong.inner_mut().inputs[0].non_witness_utxo =
        Some(other_previous(funding.output[0].clone(), 1));
    assert_eq!(
        amount(&wrong),
        None,
        "a previous transaction of another txid"
    );
}

/// A cosigner's file never replaces an own right-txid previous
/// transaction by an amount, nor an own `witness_utxo` by a previous
/// transaction that disagrees, and is named whatever the own copy holds;
/// one that agrees is merged.
#[test]
fn collect_refuses_a_file_that_puts_a_witness_utxo_against_the_previous_transaction() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    let truth = key_spend_with_previous(50_000, 50_000, true);
    let lie = key_spend_with_previous(50_000, 99_999, false);
    let lie_with_previous = key_spend_with_previous(50_000, 99_999, true);
    let only_witness = key_spend_with_previous(50_000, 50_000, false);
    // Own copy with its previous transaction, the file only a lying
    // witness_utxo.
    let mut previous_only = truth.clone();
    previous_only.inner_mut().inputs[0].witness_utxo = None;
    let mut spend = Spend::new(previous_only, "own.psbt");
    let before = spend.psbt.to_bytes();
    let why = spend
        .collect(&c, &session, &lie.to_bytes(), "c.psbt")
        .unwrap_err();
    assert_eq!(
        why,
        EN.wallet_file_states_false
            .replacen("{}", "c.psbt", 1)
            .replacen("{}", "0", 1)
    );
    assert_eq!(spend.psbt.to_bytes(), before);
    assert!(spend.finish().is_ok(), "the own copy is unchanged");
    // Own copy with both agreeing: a file's lying witness_utxo is refused
    // and named too, as it is when the own copy holds only the previous
    // transaction.
    let mut spend = Spend::new(truth.clone(), "own.psbt");
    let before = spend.psbt.to_bytes();
    let why = spend
        .collect(&c, &session, &lie.to_bytes(), "c.psbt")
        .unwrap_err();
    assert_eq!(
        why,
        EN.wallet_file_states_false
            .replacen("{}", "c.psbt", 1)
            .replacen("{}", "0", 1)
    );
    assert_eq!(spend.psbt.to_bytes(), before);
    assert!(spend.finish().is_ok());
    // Own copy with only a witness_utxo that is false, the file with the
    // previous transaction.
    let mut spend = Spend::new(lie.clone(), "own.psbt");
    let before = spend.psbt.to_bytes();
    let why = spend
        .collect(&c, &session, &lie_with_previous.to_bytes(), "c.psbt")
        .unwrap_err();
    assert_eq!(
        why,
        EN.wallet_file_states_false
            .replacen("{}", "c.psbt", 1)
            .replacen("{}", "0", 1)
    );
    assert_eq!(spend.psbt.to_bytes(), before);
    // A file adding a previous transaction that agrees is merged.
    let mut spend = Spend::new(only_witness, "own.psbt");
    spend
        .collect(&c, &session, &truth.to_bytes(), "c.psbt")
        .expect("an agreeing previous transaction is merged");
    assert!(spend.psbt.inner().inputs[0].non_witness_utxo.is_some());
    assert!(spend.finish().is_ok());
}

/// The spend's own false `witness_utxo` is the spend's: an honest file
/// is not named, the merge is refused and the spend stays as it was.
#[test]
fn collect_does_not_name_an_honest_file_for_the_spends_own_false_witness_utxo() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    let truth = key_spend_with_previous(50_000, 50_000, true);
    let mut previous_only = truth.clone();
    previous_only.inner_mut().inputs[0].witness_utxo = None;
    for file in [truth, previous_only] {
        let mut spend = Spend::new(key_spend_with_previous(50_000, 99_999, false), "own.psbt");
        let before = spend.psbt.to_bytes();
        let why = spend
            .collect(&c, &session, &file.to_bytes(), "c.psbt")
            .unwrap_err();
        assert_eq!(why, EN.psbt_final_prev_disagree.replacen("{}", "0", 1));
        assert!(!why.contains("c.psbt"), "{why}");
        assert_eq!(spend.psbt.to_bytes(), before);
    }
}

/// A real signature of each kind of spend, the previous transaction and
/// `witness_utxo` stated as asked. Kinds: 0 p2wpkh, 1 p2sh-p2wpkh, 2
/// p2sh-p2wsh, 3 p2sh, 4 p2wsh, 5 p2pkh, 6 taproot key path, 7 taproot
/// script path. A taproot signature is made over `taproot_sees` for input
/// `.0` where given. Every output is 50,000 sat.
#[expect(
    clippy::too_many_lines,
    clippy::disallowed_methods,
    reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
)]
fn build(
    kinds: &[usize],
    witness_utxo: bool,
    non_witness: bool,
    taproot_sees: Option<(usize, u64)>,
) -> faraday_psbt::Psbt {
    use btc::key::{Keypair, TapTweak};
    use btc::script::{Builder, PushBytesBuf};
    use btc::sighash::{Prevouts, TapSighashType};
    use btc::taproot::{LeafVersion, TapLeafHash, TaprootBuilder};
    use btc::{Amount, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
    use faraday_bip::bitcoin as btc;
    let secp = Secp256k1::new();
    let sk = SecretKey::from_slice(&[5; 32]).unwrap();
    let pk = PublicKey::new(Point::from_secret_key(&secp, &sk));
    let pair = Keypair::from_secret_key(&secp, &sk);
    let (xonly, _) = pair.x_only_public_key();
    let signer = pair.tap_tweak(&secp, None).to_keypair();
    let other = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[6; 32]).unwrap())
        .x_only_public_key()
        .0;
    let push = |b: Vec<u8>| PushBytesBuf::try_from(b).unwrap();
    let wpkh = ScriptBuf::new_p2wpkh(&pk.wpubkey_hash().unwrap());
    let ms = Builder::new()
        .push_int(1)
        .push_key(&pk)
        .push_int(1)
        .push_opcode(btc::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let wsh = ScriptBuf::new_p2wsh(&ms.wscript_hash());
    let leaf = Builder::new()
        .push_x_only_key(&xonly)
        .push_opcode(btc::opcodes::all::OP_CHECKSIG)
        .into_script();
    let info = TaprootBuilder::new()
        .add_leaf(0, leaf.clone())
        .unwrap()
        .finalize(&secp, other)
        .unwrap();
    let control = info
        .control_block(&(leaf.clone(), LeafVersion::TapScript))
        .unwrap()
        .serialize();
    let spks = [
        wpkh.clone(),
        ScriptBuf::new_p2sh(&wpkh.script_hash()),
        ScriptBuf::new_p2sh(&wsh.script_hash()),
        ScriptBuf::new_p2sh(&ms.script_hash()),
        wsh.clone(),
        ScriptBuf::new_p2pkh(&pk.pubkey_hash()),
        ScriptBuf::new_p2tr(&secp, xonly, None),
        ScriptBuf::new_p2tr(&secp, other, info.merkle_root()),
    ];
    let amount = Amount::from_sat(50_000);
    let prev = Transaction {
        version: btc::transaction::Version::TWO,
        lock_time: btc::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(btc::Txid::from_byte_array([8; 32]), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: spks
            .iter()
            .map(|s| TxOut {
                value: amount,
                script_pubkey: s.clone(),
            })
            .collect(),
    };
    let txid = prev.compute_txid();
    let tx = Transaction {
        version: btc::transaction::Version::TWO,
        lock_time: btc::absolute::LockTime::ZERO,
        input: kinds
            .iter()
            .map(|k| TxIn {
                previous_output: OutPoint::new(txid, *k as u32),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: wpkh.clone(),
        }],
    };
    let prevouts: Vec<TxOut> = kinds.iter().map(|k| prev.output[*k].clone()).collect();
    let mut seen = prevouts.clone();
    if let Some((i, sat)) = taproot_sees {
        seen[i].value = Amount::from_sat(sat);
    }
    let mut inner = btc::Psbt::from_unsigned_tx(tx.clone()).unwrap();
    let all = EcdsaSighashType::All;
    let der = |z: [u8; 32]| {
        let sig = secp.sign_ecdsa(&Message::from_digest(z), &sk);
        let mut v = sig.serialize_der().to_vec();
        v.push(1);
        v
    };
    let script_sig = |b: &ScriptBuf| Builder::new().push_slice(push(b.to_bytes())).into_script();
    let mut cache = SighashCache::new(&tx);
    for (i, k) in kinds.iter().enumerate() {
        let spk = &spks[*k];
        let (sig, wit): (Option<ScriptBuf>, Option<Witness>) = match *k {
            0 | 1 => {
                let script = if *k == 0 { spk } else { &wpkh };
                let z = cache.p2wpkh_signature_hash(i, script, amount, all).unwrap();
                let w = Witness::from_slice(&[der(z.to_byte_array()), pk.to_bytes()]);
                ((*k == 1).then(|| script_sig(&wpkh)), Some(w))
            }
            2 | 4 => {
                let z = cache.p2wsh_signature_hash(i, &ms, amount, all).unwrap();
                let w = Witness::from_slice(&[vec![], der(z.to_byte_array()), ms.to_bytes()]);
                ((*k == 2).then(|| script_sig(&wsh)), Some(w))
            }
            3 => {
                let z = cache.legacy_signature_hash(i, &ms, 1).unwrap();
                let sig = Builder::new()
                    .push_int(0)
                    .push_slice(push(der(z.to_byte_array())))
                    .push_slice(push(ms.to_bytes()))
                    .into_script();
                (Some(sig), None)
            }
            5 => {
                let z = cache.legacy_signature_hash(i, spk, 1).unwrap();
                let sig = Builder::new()
                    .push_slice(push(der(z.to_byte_array())))
                    .push_slice(push(pk.to_bytes()))
                    .into_script();
                (Some(sig), None)
            }
            6 => {
                let z = cache
                    .taproot_key_spend_signature_hash(
                        i,
                        &Prevouts::All(&seen),
                        TapSighashType::Default,
                    )
                    .unwrap();
                let m = Message::from_digest(z.to_byte_array());
                let s = secp.sign_schnorr_no_aux_rand(&m, &signer);
                (None, Some(Witness::from_slice(&[s.serialize().to_vec()])))
            }
            _ => {
                let hash = TapLeafHash::from_script(&leaf, LeafVersion::TapScript);
                let z = cache
                    .taproot_script_spend_signature_hash(
                        i,
                        &Prevouts::All(&seen),
                        hash,
                        TapSighashType::Default,
                    )
                    .unwrap();
                let m = Message::from_digest(z.to_byte_array());
                let s = secp.sign_schnorr_no_aux_rand(&m, &pair);
                let w = Witness::from_slice(&[
                    s.serialize().to_vec(),
                    leaf.to_bytes(),
                    control.clone(),
                ]);
                (None, Some(w))
            }
        };
        let input = &mut inner.inputs[i];
        input.final_script_sig = sig;
        input.final_script_witness = wit;
        if witness_utxo {
            input.witness_utxo = Some(prevouts[i].clone());
        }
        if non_witness {
            input.non_witness_utxo = Some(prev.clone());
        }
    }
    faraday_psbt::Psbt::from(inner)
}

/// True spends of every kind, stated by the previous transaction only, by
/// a `witness_utxo` only and by both, alone and all in one PSBT: every
/// row reads Valid, the check is sound and offers the finished
/// transaction, `finish` works, and the amounts read give the fee.
#[test]
fn true_spends_of_every_kind_are_never_refused_in_any_form() {
    use faraday_core::wallet::Spend;
    let names = [
        "p2wpkh",
        "p2sh-p2wpkh",
        "p2sh-p2wsh",
        "p2sh",
        "p2wsh",
        "p2pkh",
        "tr-key",
        "tr-script",
    ];
    let all: Vec<usize> = (0..8).collect();
    let mut cases: Vec<(String, Vec<usize>)> =
        (0..8).map(|k| (names[k].to_string(), vec![k])).collect();
    cases.push(("all eight kinds".to_string(), all));
    for (what, kinds) in cases {
        for (witness, previous) in [(false, true), (true, false), (true, true)] {
            let how = format!("{what}, witness_utxo {witness}, previous {previous}");
            let psbt = build(&kinds, witness, previous, None);
            let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
            let o = checked(&mut app);
            assert_eq!(o.check.rows.len(), kinds.len(), "{how}: a row per input");
            for r in &o.check.rows {
                assert!(!r.wrong() && !r.unchecked(), "{how}: {:?}", r.verdict);
            }
            assert!(o.check.mismatched.is_empty(), "{how}");
            assert_eq!((o.check.unread, o.check.disagree), (0, 0), "{how}");
            assert!(o.check.sound(), "{how}");
            assert!(o.finished.is_some(), "{how}");
            let mut spend = Spend::new(psbt.clone(), "t.psbt");
            assert!(spend.finish().is_ok(), "{how}");
            // The fee the check reports is the inputs' amounts less the
            // one output.
            app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Decode));
            assert_eq!(app.screen, Screen::Decode, "{how}");
            assert_eq!(
                app.decode().unwrap().decoded.fee,
                Some(50_000 * kinds.len() as u64 - 10_000),
                "{how}"
            );
        }
    }
}

/// Input 0 a true taproot spend beside input 1 whose `witness_utxo` lies:
/// the taproot signature commits to the previous transaction's amounts
/// and reads Valid, the check says the disagreement; one made over the
/// false amount reads Invalid.
#[test]
fn a_true_taproot_signature_reads_valid_beside_another_inputs_false_amount() {
    for (kinds, what) in [
        (vec![6, 0], "key path beside p2wpkh"),
        (vec![7, 5], "script path beside p2pkh"),
    ] {
        let mut psbt = build(&kinds, false, true, None);
        let out = psbt.inner().inputs[1]
            .non_witness_utxo
            .clone()
            .unwrap()
            .output[kinds[1]]
            .clone();
        let mut lie = out;
        lie.value = faraday_bip::bitcoin::Amount::from_sat(99_999);
        psbt.inner_mut().inputs[1].witness_utxo = Some(lie);
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = checked(&mut app);
        let row0 = o
            .check
            .rows
            .iter()
            .find(|r| r.input == 0)
            .expect("input 0's row");
        assert!(
            !row0.wrong() && !row0.unchecked(),
            "{what}: {:?}",
            row0.verdict
        );
        assert_eq!((o.check.unread, o.check.disagree), (1, 1), "{what}");
        assert_eq!(
            o.check.headline(),
            EN.nonce_disagree_head.replace("{}", "1"),
            "{what}"
        );
        assert!(!o.check.sound() && o.finished.is_none(), "{what}");
        // The signature over the false amount is wrong.
        let mut over_lie = build(&kinds, false, true, Some((1, 99_999)));
        over_lie.inner_mut().inputs[1].witness_utxo = psbt.inner().inputs[1].witness_utxo.clone();
        let mut app = with_inbox(vec![("t.psbt", over_lie.to_bytes())]);
        let o = checked(&mut app);
        let row0 = o
            .check
            .rows
            .iter()
            .find(|r| r.input == 0)
            .expect("input 0's row");
        assert!(row0.wrong(), "{what}: {:?}", row0.verdict);
    }
}

/// A p2wpkh with its true partial signature (not final), its previous
/// transaction of 50,000 sat the right one.
fn partial_p2wpkh() -> faraday_psbt::Psbt {
    let mut psbt = build(&[0], false, true, None);
    let input = &mut psbt.inner_mut().inputs[0];
    let witness = input.final_script_witness.take().unwrap();
    let w: Vec<&[u8]> = witness.iter().collect();
    let sig = ecdsa::Signature::from_slice(w[0]).unwrap();
    let key = PublicKey::from_slice(w[1]).unwrap();
    input.partial_sigs.insert(key, sig);
    psbt
}

fn lying_witness(psbt: &mut faraday_psbt::Psbt, sat: u64) {
    let mut out = psbt.inner().inputs[0]
        .non_witness_utxo
        .clone()
        .unwrap()
        .output[0]
        .clone();
    out.value = faraday_bip::bitcoin::Amount::from_sat(sat);
    psbt.inner_mut().inputs[0].witness_utxo = Some(out);
}

fn wrong_txid(psbt: &mut faraday_psbt::Psbt) {
    let out = psbt.inner().inputs[0]
        .non_witness_utxo
        .clone()
        .unwrap()
        .output[0]
        .clone();
    psbt.inner_mut().inputs[0].non_witness_utxo = Some(other_previous(out, 0));
}

/// A false statement on an input that is not final is refused as on a
/// final one: nothing offered, though the signature on it is true. The
/// same PSBT with honest inputs is still offered.
#[test]
fn a_false_statement_on_an_input_not_yet_final_offers_nothing() {
    let honest = partial_p2wpkh();
    let mut app = with_inbox(vec![("t.psbt", honest.to_bytes())]);
    let o = checked(&mut app);
    assert!(o.check.sound() && !o.check.refused(false));
    let shown = app.drawn_texts().join(" | ");
    assert!(shown.contains(EN.vault_save_to_a_stick), "{shown}");
    let mut disagree = honest.clone();
    lying_witness(&mut disagree, 99_999);
    let mut wrong_prev = honest.clone();
    wrong_txid(&mut wrong_prev);
    for (what, psbt, line) in [
        (
            "witness_utxo against the previous transaction",
            disagree,
            EN.nonce_screen_refused_disagree,
        ),
        (
            "a previous transaction of another txid",
            wrong_prev,
            EN.nonce_screen_refused_unread,
        ),
    ] {
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = checked(&mut app);
        assert!(!o.check.sound(), "{what}");
        assert!(o.check.refused(false) && o.finished.is_none(), "{what}");
        let shown = app.drawn_texts().join(" | ");
        assert!(shown.contains(line), "{what}: {shown}");
        for absent in [EN.nonce_screen_more_wanted, EN.vault_save_to_a_stick] {
            assert!(!shown.contains(absent), "{what}: {shown}");
        }
    }
}

/// Beside another input that states no output, a disagreement's line is
/// what is said, and a witness that does not match beside it is not "only
/// the witness".
#[test]
fn a_disagreement_beside_an_unread_input_is_named_and_not_called_unread() {
    let mut psbt = build(&[6, 0], false, true, None);
    let out = psbt.inner().inputs[1]
        .non_witness_utxo
        .clone()
        .unwrap()
        .output[0]
        .clone();
    let mut lie = out;
    lie.value = faraday_bip::bitcoin::Amount::from_sat(99_999);
    psbt.inner_mut().inputs[1].witness_utxo = Some(lie);
    psbt.inner_mut().inputs[0].non_witness_utxo = None;
    psbt.inner_mut().inputs[0].witness_utxo = None;
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    assert_eq!((o.check.unread, o.check.disagree), (2, 1));
    assert_eq!(
        o.check.headline(),
        EN.nonce_disagree_head.replace("{}", "1")
    );
    let shown = app.drawn_texts().join(" | ");
    assert!(shown.contains(EN.nonce_screen_refused_disagree), "{shown}");
    assert!(!shown.contains(EN.nonce_screen_refused_unread), "{shown}");
    // A failing witness beside a disagreement does not read as the only
    // thing wrong.
    let mut o = faraday_core::wallet::NonceCheck {
        rows: vec![],
        reused: vec![],
        mismatched: vec![1],
        unread: 1,
        disagree: 1,
        not_evaluated: vec![],
        fields_refused: vec![],
        tx: [0; 32],
        partials: vec![],
        used_before: vec![],
        history: faraday_core::memory::nonces::History::Off,
        history_gen: 0,
    };
    assert!(!o.only_witness_wrong());
    assert_eq!(o.headline(), EN.nonce_disagree_head.replace("{}", "1"));
    o.disagree = 0;
    assert!(o.only_witness_wrong());
}

/// A cosigner's file never brings a false statement into a spend under
/// way, final or not: it is refused by name, the own copy is as it was,
/// and a clean file still collects. A right-txid previous transaction
/// repairs an own wrong-txid one, a bare witness_utxo does not, and an own
/// copy that is still false after a file says so without blaming it.
#[test]
fn collect_keeps_a_files_false_statement_out_of_a_spend_not_yet_final() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    let honest = partial_p2wpkh();
    let mut bare = honest.clone();
    lying_witness(&mut bare, 99_999);
    bare.inner_mut().inputs[0].non_witness_utxo = None;
    let mut truthful_witness = honest.clone();
    lying_witness(&mut truthful_witness, 50_000);
    truthful_witness.inner_mut().inputs[0].non_witness_utxo = None;
    let mut wrong_prev = honest.clone();
    wrong_txid(&mut wrong_prev);
    let mut wrong_only = wrong_prev.clone();
    wrong_only.inner_mut().inputs[0].partial_sigs.clear();
    let why_file = |spend: &mut Spend, file: &faraday_psbt::Psbt| {
        let before = spend.psbt.to_bytes();
        let why = spend
            .collect(&c, &session, &file.to_bytes(), "c.psbt")
            .unwrap_err();
        assert_eq!(spend.psbt.to_bytes(), before, "the own copy is as it was");
        why
    };
    // Own previous transaction, the file a lying witness_utxo.
    let mut spend = Spend::new(honest.clone(), "own.psbt");
    let why = why_file(&mut spend, &bare);
    assert_eq!(
        why,
        EN.wallet_file_states_false
            .replacen("{}", "c.psbt", 1)
            .replacen("{}", "0", 1)
    );
    assert!(
        spend
            .collect(&c, &session, &honest.to_bytes(), "d.psbt")
            .is_ok(),
        "a clean file collects"
    );
    // Own true witness_utxo, the file a previous transaction of another txid.
    let mut spend = Spend::new(truthful_witness.clone(), "own.psbt");
    let why = why_file(&mut spend, &wrong_prev);
    assert!(why.contains("c.psbt"), "{why}");
    assert!(
        spend
            .collect(&c, &session, &truthful_witness.to_bytes(), "d.psbt")
            .is_ok()
    );
    // Own wrong-txid previous transaction, the file the right one.
    let mut spend = Spend::new(wrong_only.clone(), "own.psbt");
    assert!(spend.refused().is_some());
    spend
        .collect(&c, &session, &honest.to_bytes(), "c.psbt")
        .expect("proven repair");
    assert_eq!(spend.refused(), None);
    assert!(spend.psbt.inner().inputs[0].non_witness_utxo.is_some());
    // The other way: own right, file wrong-txid is refused (above); a bare
    // witness_utxo repairs nothing, and nobody is blamed.
    let mut spend = Spend::new(wrong_only.clone(), "own.psbt");
    let why = why_file(&mut spend, &truthful_witness);
    assert!(!why.contains("c.psbt"), "{why}");
    assert_eq!(Some(why), spend.refused());
    // A file that is false on its own is named, whatever the own copy
    // holds, and so is one whose `witness_utxo` differs from the output of
    // the own right previous transaction.
    let mut lying_with_previous = honest.clone();
    lying_witness(&mut lying_with_previous, 99_999);
    let mut own_both = honest.clone();
    lying_witness(&mut own_both, 50_000);
    let named = EN
        .wallet_file_states_false
        .replacen("{}", "c.psbt", 1)
        .replacen("{}", "0", 1);
    for (what, own, file) in [
        ("own right, file wrong txid", &honest, &wrong_prev),
        (
            "own true witness_utxo, file right previous with a lie",
            &truthful_witness,
            &lying_with_previous,
        ),
        ("own both, file a bare lie", &own_both, &bare),
        (
            "own wrong txid, file right previous with a lie",
            &wrong_only,
            &lying_with_previous,
        ),
        ("own wrong txid, file wrong txid", &wrong_only, &wrong_only),
    ] {
        let mut spend = Spend::new(own.clone(), "own.psbt");
        assert_eq!(why_file(&mut spend, file), named, "{what}");
        spend
            .collect(&c, &session, &honest.to_bytes(), "d.psbt")
            .unwrap_or_else(|e| panic!("{what}: a clean file collects: {e}"));
    }
}

/// A file repairing one input while another stays false is refused, and
/// blames no file: a single file repairing both is the way out.
#[test]
fn collect_repairs_all_false_inputs_or_none() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    let honest = build(&[0, 1], false, true, None);
    let mut own = honest.clone();
    for i in 0..2 {
        let out = honest.inner().inputs[i]
            .non_witness_utxo
            .clone()
            .unwrap()
            .output[i]
            .clone();
        own.inner_mut().inputs[i].non_witness_utxo = Some(other_previous(out, 0));
    }
    let mut one_fixed = own.clone();
    one_fixed.inner_mut().inputs[0].non_witness_utxo =
        honest.inner().inputs[0].non_witness_utxo.clone();
    one_fixed.inner_mut().inputs[1].non_witness_utxo = None;
    let mut spend = Spend::new(own, "own.psbt");
    let before = spend.psbt.to_bytes();
    let why = spend
        .collect(&c, &session, &one_fixed.to_bytes(), "c.psbt")
        .unwrap_err();
    assert!(!why.contains("c.psbt"), "{why}");
    assert_eq!(spend.psbt.to_bytes(), before);
    spend
        .collect(&c, &session, &honest.to_bytes(), "d.psbt")
        .expect("a file repairing both");
    assert_eq!(spend.refused(), None);
}

/// A disagreement beside an input whose witness fails is what the check
/// says first, with real rows: the failing witness's own Invalid row does
/// not make it "a signature is wrong". A wrong signature on another input
/// does.
#[test]
fn a_disagreement_beside_a_failing_witness_is_named_not_called_a_wrong_signature() {
    let mut psbt = build(&[0, 7], false, true, None);
    lying_witness(&mut psbt, 99_999);
    let witness = psbt.inner().inputs[1].final_script_witness.clone().unwrap();
    let mut parts: Vec<Vec<u8>> = witness.iter().map(<[u8]>::to_vec).collect();
    parts[2][5] ^= 1;
    psbt.inner_mut().inputs[1].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&parts));
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    let verdict = |i: usize| {
        o.check
            .rows
            .iter()
            .find(|r| r.input == i)
            .map(|r| r.wrong())
    };
    assert_eq!(verdict(0), Some(false), "input 0 is true");
    assert_eq!(verdict(1), Some(true), "input 1's witness fails");
    assert_eq!(o.check.mismatched, vec![1]);
    assert_eq!(
        o.check.headline(),
        EN.nonce_disagree_head.replace("{}", "1")
    );
    let shown = app.drawn_texts().join(" | ");
    assert!(shown.contains(EN.nonce_screen_refused_disagree), "{shown}");
    assert!(!shown.contains(EN.nonce_screen_refused), "{shown}");
}

/// Honest partly signed PSBTs of more than p2wpkh, one signature of two
/// under a p2wsh multisig and under a taproot script path, are sound and
/// offered as they are: more signatures are wanted.
#[test]
#[expect(
    clippy::too_many_lines,
    clippy::disallowed_methods,
    reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
)]
fn an_honest_partly_signed_multisig_and_script_path_are_offered() {
    use btc::key::Keypair;
    use btc::script::Builder;
    use btc::sighash::{Prevouts, TapSighashType};
    use btc::taproot::{LeafVersion, TapLeafHash, TaprootBuilder};
    use faraday_bip::bitcoin::{self as btc, Amount, ScriptBuf, TxOut};
    let secp = Secp256k1::new();
    let sk = |n: u8| SecretKey::from_slice(&[n; 32]).unwrap();
    let pk = |n: u8| PublicKey::new(Point::from_secret_key(&secp, &sk(n)));
    let pair = |n: u8| Keypair::from_secret_key(&secp, &sk(n));
    // 1. p2wsh, two of two, one signature.
    let ms = Builder::new()
        .push_int(2)
        .push_key(&pk(5))
        .push_key(&pk(6))
        .push_int(2)
        .push_opcode(btc::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let wsh = ScriptBuf::new_p2wsh(&ms.wscript_hash());
    // 2. taproot script path, two of two in the leaf, one signature.
    let (x5, _) = pair(5).x_only_public_key();
    let (x6, _) = pair(6).x_only_public_key();
    let (internal, _) = pair(9).x_only_public_key();
    let leaf = Builder::new()
        .push_x_only_key(&x5)
        .push_opcode(btc::opcodes::all::OP_CHECKSIGVERIFY)
        .push_x_only_key(&x6)
        .push_opcode(btc::opcodes::all::OP_CHECKSIG)
        .into_script();
    let info = TaprootBuilder::new()
        .add_leaf(0, leaf.clone())
        .unwrap()
        .finalize(&secp, internal)
        .unwrap();
    let tr = ScriptBuf::new_p2tr(&secp, internal, info.merkle_root());
    let previous = btc::Transaction {
        version: btc::transaction::Version::TWO,
        lock_time: btc::absolute::LockTime::ZERO,
        input: vec![btc::TxIn {
            previous_output: OutPoint::new(btc::Txid::from_byte_array([8; 32]), 0),
            script_sig: ScriptBuf::new(),
            sequence: btc::Sequence::MAX,
            witness: btc::Witness::new(),
        }],
        output: [&wsh, &tr]
            .iter()
            .map(|s| TxOut {
                value: Amount::from_sat(50_000),
                script_pubkey: (*s).clone(),
            })
            .collect(),
    };
    let txid = previous.compute_txid();
    let tx = btc::Transaction {
        version: btc::transaction::Version::TWO,
        lock_time: btc::absolute::LockTime::ZERO,
        input: (0..2)
            .map(|v| btc::TxIn {
                previous_output: OutPoint::new(txid, v),
                script_sig: ScriptBuf::new(),
                sequence: btc::Sequence::MAX,
                witness: btc::Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: wsh.clone(),
        }],
    };
    let mut inner = btc::Psbt::from_unsigned_tx(tx.clone()).unwrap();
    for input in &mut inner.inputs {
        input.non_witness_utxo = Some(previous.clone());
    }
    let mut cache = SighashCache::new(&tx);
    let z = cache
        .p2wsh_signature_hash(0, &ms, Amount::from_sat(50_000), EcdsaSighashType::All)
        .unwrap();
    let sig = secp.sign_ecdsa(&Message::from_digest(z.to_byte_array()), &sk(5));
    inner.inputs[0].witness_script = Some(ms.clone());
    inner.inputs[0].partial_sigs.insert(
        pk(5),
        ecdsa::Signature {
            signature: sig,
            sighash_type: EcdsaSighashType::All,
        },
    );
    let hash = TapLeafHash::from_script(&leaf, LeafVersion::TapScript);
    let z = cache
        .taproot_script_spend_signature_hash(
            1,
            &Prevouts::All(&previous.output),
            hash,
            TapSighashType::Default,
        )
        .unwrap();
    let schnorr = secp.sign_schnorr_no_aux_rand(&Message::from_digest(z.to_byte_array()), &pair(5));
    inner.inputs[1].tap_internal_key = Some(internal);
    inner.inputs[1].tap_merkle_root = info.merkle_root();
    inner.inputs[1].tap_scripts.insert(
        info.control_block(&(leaf.clone(), LeafVersion::TapScript))
            .unwrap(),
        (leaf.clone(), LeafVersion::TapScript),
    );
    inner.inputs[1].tap_script_sigs.insert(
        (x5, hash),
        btc::taproot::Signature {
            signature: schnorr,
            sighash_type: TapSighashType::Default,
        },
    );
    let psbt = faraday_psbt::Psbt::from(inner);
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    assert_eq!(o.check.rows.len(), 2);
    for r in &o.check.rows {
        assert!(!r.wrong() && !r.unchecked(), "{:?}", r.verdict);
    }
    assert_eq!((o.check.unread, o.check.disagree), (0, 0));
    assert!(o.check.sound() && !o.check.refused(false));
    let shown = app.drawn_texts().join(" | ");
    assert!(shown.contains(EN.nonce_screen_more_wanted), "{shown}");
    assert!(shown.contains(EN.vault_save_to_a_stick), "{shown}");
}

/// A taproot input signed with ANYONECANPAY whose spent output the file
/// does not state truly (a previous transaction of another txid, or
/// nothing) has no sighash to check against: the check says so and the
/// app goes on.
#[test]
fn an_anyone_can_pay_taproot_signature_with_no_readable_output_is_not_a_crash() {
    use faraday_bip::bitcoin as btc;
    use faraday_bip::bitcoin::sighash::TapSighashType;
    let signature = btc::taproot::Signature {
        signature: btc::secp256k1::schnorr::Signature::from_slice(&[1u8; 64]).unwrap(),
        sighash_type: TapSighashType::AllPlusAnyoneCanPay,
    };
    for script_path in [false, true] {
        for form in [
            "wrong txid beside witness_utxo",
            "wrong txid alone",
            "nothing",
        ] {
            let mut p = testkit::unsigned(&kit("taproot")).unwrap();
            let input = &mut p.inner_mut().inputs[0];
            if script_path {
                let x = input.tap_internal_key.unwrap();
                let leaf = btc::taproot::TapLeafHash::from_byte_array([7u8; 32]);
                input.tap_script_sigs.insert((x, leaf), signature);
            } else {
                input.tap_key_sig = Some(signature);
            }
            if form != "nothing" {
                let mut prev = input.non_witness_utxo.clone().unwrap();
                prev.lock_time = btc::absolute::LockTime::from_consensus(1);
                input.non_witness_utxo = Some(prev);
            } else {
                input.non_witness_utxo = None;
            }
            if form != "wrong txid beside witness_utxo" {
                input.witness_utxo = None;
            }
            let mut app = testkit::started();
            add_key(&mut app, "bacon");
            let check = app.session.nonce_check(&testkit::compute(), &p);
            let unread = usize::from(form != "nothing");
            assert_eq!(
                (
                    check.rows.len(),
                    check.unread,
                    check.disagree,
                    check.sound()
                ),
                (1, unread, 0, false),
                "{script_path} {form}"
            );
        }
    }
}

type Spoil = fn(&mut faraday_bip::bitcoin::psbt::Input);

fn set_spk(input: &mut faraday_bip::bitcoin::psbt::Input, spk: faraday_bip::bitcoin::ScriptBuf) {
    input.witness_utxo.as_mut().unwrap().script_pubkey = spk;
}

/// The signatures of a true spend, put under an output they do not
/// commit to or into a shape no node relays (the finalized input is
/// never run as a script: hashes and shapes only).
#[test]
#[expect(clippy::too_many_lines, reason = "a table of cases")]
fn a_finalized_non_taproot_input_that_does_not_match_its_output_is_refused() {
    use faraday_bip::bitcoin::script::{Builder, PushBytesBuf};
    use faraday_bip::bitcoin::{
        PubkeyHash, ScriptBuf, ScriptHash, WPubkeyHash, WScriptHash, Witness,
    };
    use faraday_core::wallet::{Session, Spend};
    fn push(b: &[u8]) -> PushBytesBuf {
        PushBytesBuf::try_from(b.to_vec()).unwrap()
    }
    let cases: Vec<(&str, usize, Spoil, bool)> = vec![
        (
            "p2wpkh of another key",
            0,
            |i| {
                set_spk(
                    i,
                    ScriptBuf::new_p2wpkh(&WPubkeyHash::from_byte_array([9; 20])),
                );
            },
            true,
        ),
        (
            "p2wsh of all zeros",
            4,
            |i| {
                set_spk(
                    i,
                    ScriptBuf::new_p2wsh(&WScriptHash::from_byte_array([0; 32])),
                );
            },
            true,
        ),
        (
            "p2pkh of another key",
            5,
            |i| {
                set_spk(
                    i,
                    ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([9; 20])),
                );
            },
            true,
        ),
        (
            "plain p2sh of all zeros",
            3,
            |i| {
                set_spk(
                    i,
                    ScriptBuf::new_p2sh(&ScriptHash::from_byte_array([0; 20])),
                );
            },
            true,
        ),
        (
            "p2sh-p2wpkh under another hash",
            1,
            |i| {
                set_spk(
                    i,
                    ScriptBuf::new_p2sh(&ScriptHash::from_byte_array([0; 20])),
                );
            },
            true,
        ),
        (
            "p2sh-p2wpkh with a scriptSig of two pushes",
            1,
            |i| {
                let redeem = i.final_script_sig.clone().unwrap();
                let mut s = Builder::new().push_int(0).into_script().to_bytes();
                s.extend_from_slice(redeem.as_bytes());
                i.final_script_sig = Some(ScriptBuf::from(s));
            },
            true,
        ),
        (
            "p2sh-p2wpkh with no witness key",
            1,
            |i| {
                let w: Vec<Vec<u8>> = i
                    .final_script_witness
                    .as_ref()
                    .unwrap()
                    .iter()
                    .take(1)
                    .map(<[u8]>::to_vec)
                    .collect();
                i.final_script_witness = Some(Witness::from_slice(&w));
            },
            false,
        ),
        (
            "p2wpkh with a scriptSig",
            0,
            |i| {
                i.final_script_sig = Some(Builder::new().push_int(0).into_script());
            },
            true,
        ),
        (
            "p2wpkh with three elements",
            0,
            |i| {
                let mut w: Vec<Vec<u8>> = i
                    .final_script_witness
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(<[u8]>::to_vec)
                    .collect();
                w.push(vec![1]);
                i.final_script_witness = Some(Witness::from_slice(&w));
            },
            false,
        ),
        (
            "p2wsh with an empty witness",
            4,
            |i| {
                i.final_script_witness = Some(Witness::new());
            },
            false,
        ),
        (
            "p2wsh with another script last",
            4,
            |i| {
                let mut w: Vec<Vec<u8>> = i
                    .final_script_witness
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(<[u8]>::to_vec)
                    .collect();
                *w.last_mut().unwrap() = vec![0x51];
                i.final_script_witness = Some(Witness::from_slice(&w));
            },
            true,
        ),
        (
            "p2pkh with the signature alone",
            5,
            |i| {
                let sig = i
                    .final_script_sig
                    .as_ref()
                    .unwrap()
                    .instructions()
                    .next()
                    .unwrap()
                    .unwrap();
                let faraday_bip::bitcoin::script::Instruction::PushBytes(b) = sig else {
                    panic!()
                };
                i.final_script_sig =
                    Some(Builder::new().push_slice(push(b.as_bytes())).into_script());
            },
            false,
        ),
        (
            "p2pkh with a witness",
            5,
            |i| {
                i.final_script_witness = Some(Witness::from_slice(&[vec![1u8]]));
            },
            true,
        ),
        (
            "plain p2sh with a witness",
            3,
            |i| {
                i.final_script_witness = Some(Witness::from_slice(&[vec![1u8]]));
            },
            true,
        ),
        (
            "plain p2sh with a scriptSig of another script",
            3,
            |i| {
                i.final_script_sig = Some(
                    Builder::new()
                        .push_int(0)
                        .push_slice(push(&[1; 71]))
                        .push_slice(push(&[0x51]))
                        .into_script(),
                );
            },
            false,
        ),
    ];
    for (what, kind, spoil, rows) in cases {
        let true_psbt = build(&[0, kind], true, false, None);
        assert!(offered(&true_psbt), "{what}: the true spend is offered");
        let mut psbt = true_psbt.clone();
        spoil(&mut psbt.inner_mut().inputs[1]);
        assert!(
            faraday_psbt::verify::witness_mismatch(psbt.inner(), 1),
            "{what}"
        );
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = checked(&mut app);
        assert_eq!(o.check.mismatched, vec![1], "{what}");
        let own_rows: Vec<_> = o.check.rows.iter().filter(|r| r.input == 1).collect();
        assert_eq!(own_rows.is_empty(), !rows, "{what}: rows");
        assert!(
            own_rows.iter().all(|r| r.verdict == Verdict::Invalid),
            "{what}"
        );
        assert!(
            o.check
                .rows
                .iter()
                .filter(|r| r.input == 0)
                .all(|r| r.verdict == Verdict::Valid),
            "{what}: the true input beside it"
        );
        assert!(!o.check.sound(), "{what}");
        assert!(o.finished.is_none(), "{what}: nothing finished is offered");
        let mut spend = Spend::new(psbt.clone(), "t.psbt");
        assert_eq!(spend.finish().unwrap_err(), mismatch_text(1), "{what}");
        assert!(spend.finished.is_none(), "{what}");
        // A cosigner's file with it is not merged; the true one is.
        let unfinalized = |p: &faraday_psbt::Psbt| {
            let mut own = p.clone();
            own.inner_mut().inputs[1].final_script_witness = None;
            own.inner_mut().inputs[1].final_script_sig = None;
            Spend::new(own, "own.psbt")
        };
        let c = testkit::compute();
        let mut own = unfinalized(&true_psbt);
        let before = own.psbt.to_bytes();
        let why = own
            .collect(&c, &Session::default(), &psbt.to_bytes(), "cosigner.psbt")
            .expect_err(what);
        assert_eq!(why, mismatch_text(1), "{what}");
        assert_eq!(own.psbt.to_bytes(), before, "{what}");
        let mut own = unfinalized(&true_psbt);
        own.collect(
            &c,
            &Session::default(),
            &true_psbt.to_bytes(),
            "cosigner.psbt",
        )
        .expect("a true spend merges");
        assert!(own.finish().is_ok(), "{what}");
    }
}

/// A signer holding key A signs properly for an output of key B, naming
/// A's own key in the witness or `scriptSig`: the output hashes B's key,
/// so the input is a mismatch in each kind, where B's own key under the
/// same output is not.
#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
)]
fn a_key_signing_for_another_keys_output_is_a_mismatch_in_each_kind() {
    use faraday_bip::bitcoin::script::{Builder, PushBytesBuf};
    use faraday_bip::bitcoin::{ScriptBuf, WPubkeyHash};
    use faraday_core::wallet::Spend;
    let secp = Secp256k1::new();
    let signer = |seed: u8| {
        let sk = SecretKey::from_slice(&[seed; 32]).unwrap();
        let key = PublicKey::new(Point::from_secret_key(&secp, &sk));
        let mut sig = secp
            .sign_ecdsa(&Message::from_digest([9; 32]), &sk)
            .serialize_der()
            .to_vec();
        sig.push(1);
        (key, sig)
    };
    let (key_a, sig_a) = signer(5);
    let (key_b, sig_b) = signer(6);
    let push_all = |parts: &[&[u8]]| {
        let mut b = Builder::new();
        for p in parts {
            b = b.push_slice(PushBytesBuf::try_from(p.to_vec()).unwrap());
        }
        b.into_script()
    };
    // (what, output of key B, the input A makes, the input B makes)
    type Spent = (ScriptBuf, ScriptBuf, Vec<Vec<u8>>);
    let kinds: Vec<(&str, ScriptBuf, Spent, Spent)> = {
        let wpkh = |k: &PublicKey| ScriptBuf::new_p2wpkh(&WPubkeyHash::hash(&k.to_bytes()));
        let inner_b = wpkh(&key_b);
        vec![
            (
                "p2pkh",
                ScriptBuf::new_p2pkh(&key_b.pubkey_hash()),
                (
                    push_all(&[&sig_a, &key_a.to_bytes()]),
                    ScriptBuf::new(),
                    vec![],
                ),
                (
                    push_all(&[&sig_b, &key_b.to_bytes()]),
                    ScriptBuf::new(),
                    vec![],
                ),
            ),
            (
                "p2wpkh",
                inner_b.clone(),
                (
                    ScriptBuf::new(),
                    ScriptBuf::new(),
                    vec![sig_a.clone(), key_a.to_bytes()],
                ),
                (
                    ScriptBuf::new(),
                    ScriptBuf::new(),
                    vec![sig_b.clone(), key_b.to_bytes()],
                ),
            ),
            (
                "p2sh-p2wpkh",
                ScriptBuf::new_p2sh(&inner_b.script_hash()),
                (
                    push_all(&[inner_b.as_bytes()]),
                    ScriptBuf::new(),
                    vec![sig_a.clone(), key_a.to_bytes()],
                ),
                (
                    push_all(&[inner_b.as_bytes()]),
                    ScriptBuf::new(),
                    vec![sig_b.clone(), key_b.to_bytes()],
                ),
            ),
        ]
    };
    for (what, spk, attack, own) in kinds {
        let own = second_input(spk.clone(), own.0, own.2);
        assert!(
            !faraday_psbt::verify::witness_mismatch(own.inner(), 0),
            "{what}: B's own input"
        );
        let psbt = second_input(spk, attack.0, attack.2);
        assert!(
            faraday_psbt::verify::witness_mismatch(psbt.inner(), 0),
            "{what}"
        );
        let app = testkit::started();
        let check = app.session.nonce_check(&testkit::compute(), &psbt);
        assert_eq!(check.mismatched, vec![0], "{what}");
        assert!(!check.sound(), "{what}");
        let mut spend = Spend::new(psbt, "t.psbt");
        assert_eq!(spend.finish().unwrap_err(), mismatch_text(0), "{what}");
    }
}

/// The inheritance kit's recovery path, signed by the recovery key and
/// finalized: the key is in the witness, the script holds its hash. It reads
/// Valid, and finishes, from the final PSBT and through `collect`; a bad
/// signature in it is refused as a signature, and `collect` names the file.
#[test]
fn a_finalized_recovery_spend_is_a_true_spend_and_a_forged_one_is_not() {
    use faraday_core::wallet::{Origin, Spend};
    let c = testkit::compute();
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "inheritance")
        .unwrap();
    let mut session = testkit::session();
    for n in 1..3 {
        let (words, label) = testkit::TEST_SEEDS[n];
        session
            .keys
            .add_words(&c, &testkit::test_words(words), label, None)
            .map_err(|e| e.text())
            .unwrap();
    }
    session
        .add_wallet(&c, kit.name, &kit.descriptor, "Test wallet", Origin::Named)
        .map_err(|e| e.text())
        .unwrap();
    let mut signed = testkit::recovery(&kit).unwrap();
    session
        .sign(
            &c,
            &mut signed,
            &faraday_crypto::Zeroizing::new([0; 32]),
            &mut None,
        )
        .unwrap();
    let mut done = signed.clone();
    assert!(faraday_psbt::finalize(&mut done).unwrap().is_some());
    let rows: Vec<Verdict> = session
        .inspect(&c, &done)
        .present_signatures()
        .map(|s| s.verdict)
        .collect();
    assert!(!rows.is_empty() && rows.iter().all(|v| *v == Verdict::Valid));
    // Read back from the final PSBT.
    let read = faraday_psbt::Psbt::parse_bytes(&c, &done.to_bytes()).unwrap();
    assert!(Spend::new(read, "r.psbt").finish().is_ok());
    // Through collect, from the partly signed copy.
    let mut spend = Spend::new(signed.clone(), "r.psbt");
    spend
        .collect(&c, &session, &done.to_bytes(), "f.psbt")
        .expect("a true spend is collected");
    assert!(spend.finish().is_ok());
    // A signature that is another's.
    let mut forged = done.clone();
    let witness = forged.inner().inputs[0]
        .final_script_witness
        .clone()
        .unwrap();
    let mut items: Vec<Vec<u8>> = witness.iter().map(<[u8]>::to_vec).collect();
    let at = items[0].len() - 2;
    items[0][at] ^= 1;
    forged.inner_mut().inputs[0].final_script_witness =
        Some(faraday_psbt::bitcoin::Witness::from_slice(&items));
    assert_eq!(
        Spend::new(forged.clone(), "r.psbt").finish().unwrap_err(),
        EN.psbt_final_signature_refused.replace("{}", "0")
    );
    let mut spend = Spend::new(signed, "r.psbt");
    assert_eq!(
        spend
            .collect(&c, &session, &forged.to_bytes(), "f.psbt")
            .unwrap_err(),
        EN.wallet_a_signature.replace("{}", "f.psbt")
    );
}

/// A script that the interpreter alone refuses (an `older` of a version 1
/// transaction), with a true signature, is refused in the words of the
/// final fields, on the review, the nonce check and the finish, and its
/// true twin is not.
#[test]
fn a_time_lock_the_transaction_does_not_meet_is_refused_in_the_words_of_the_final_fields() {
    use faraday_bip::bitcoin::opcodes::all::{OP_CHECKSIGVERIFY, OP_CSV};
    use faraday_bip::bitcoin::script::{Builder, PushBytesBuf};
    use faraday_core::wallet::Spend;
    let a = key_of(5);
    let script = Builder::new()
        .push_slice(PushBytesBuf::try_from(a).unwrap())
        .push_opcode(OP_CHECKSIGVERIFY)
        .push_int(10)
        .push_opcode(OP_CSV)
        .into_script();
    let words = EN.psbt_final_signature_refused.replace("{}", "0");
    let c = testkit::compute();
    let app = with_inbox(vec![]);
    let make = |sign: Signer| vec![sign(5)];
    let bad = p2wsh_spend_v(&script, 10, 1, &make);
    assert_eq!(Spend::new(bad.clone(), "t.psbt").refused(), Some(words));
    assert!(Spend::new(bad.clone(), "t.psbt").finish().is_err());
    let cards = |p: &faraday_psbt::Psbt| {
        app.session
            .inspect(&c, p)
            .warnings
            .iter()
            .filter(|w| matches!(w.kind, faraday_psbt::WarningKind::FinalFieldsRefused { .. }))
            .count()
    };
    assert_eq!(cards(&bad), 1);
    let check = app.session.nonce_check(&c, &bad);
    assert_eq!(check.headline(), EN.nonce_check_fields_refused);
    let good = p2wsh_spend_v(&script, 10, 2, &make);
    assert_eq!(cards(&good), 0);
    assert_eq!(Spend::new(good.clone(), "t.psbt").refused(), None);
    assert!(Spend::new(good, "t.psbt").finish().is_ok());
}

/// Rows of a taproot leaf the output holds beside others, signed by key A
/// whose origin the input states.
mod leaf_rows {
    use super::{Unchecked, Verdict};
    use b::bip32::{DerivationPath, Fingerprint};
    use b::hashes::Hash;
    use b::opcodes::all::{OP_CHECKSIG, OP_CODESEPARATOR, OP_DROP};
    use b::secp256k1::{Keypair, Message, Secp256k1, SecretKey};
    use b::sighash::{Prevouts, SighashCache, TapSighashType};
    use b::taproot::{ControlBlock, LeafVersion, TapLeafHash, TaprootBuilder};
    use b::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
    use faraday_bip::bitcoin as b;
    use faraday_core::testkit;
    use faraday_core::wallet::{Session, Spend};
    use std::str::FromStr;

    const NO: u32 = 0xffff_ffff;

    #[derive(Clone, Copy, PartialEq)]
    enum Leaf {
        /// `<A> CHECKSIG`, a Tap miniscript: run.
        Run,
        /// `OP_DROP <A> CHECKSIG`: not run.
        Drop,
        /// `OP_CODESEPARATOR <A> CHECKSIG`: not run.
        Separator,
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Filing {
        Proper,
        NotFiled,
        /// The leaf's script with another leaf's control block.
        OtherLeafBlock,
        /// The leaf's script with a block committing to another output.
        OtherOutputBlock,
        /// The right block for the script, the entry stating version 0xc2.
        FalseVersion,
    }

    fn pair(n: u8) -> Keypair {
        Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[n; 32]).unwrap())
    }

    fn script(leaf: Leaf) -> ScriptBuf {
        let a = pair(7).x_only_public_key().0;
        let mut s = b::script::Builder::new();
        s = match leaf {
            Leaf::Run => s,
            Leaf::Drop => s.push_opcode(OP_DROP),
            Leaf::Separator => s.push_opcode(OP_CODESEPARATOR),
        };
        s.push_x_only_key(&a).push_opcode(OP_CHECKSIG).into_script()
    }

    /// The output holding the three leaves, and the control block of each.
    fn output(internal: u8) -> (ScriptBuf, [ControlBlock; 3]) {
        let secp = Secp256k1::new();
        let leaves = [Leaf::Run, Leaf::Drop, Leaf::Separator].map(script);
        let info = TaprootBuilder::new()
            .add_leaf(1, leaves[0].clone())
            .unwrap()
            .add_leaf(2, leaves[1].clone())
            .unwrap()
            .add_leaf(2, leaves[2].clone())
            .unwrap()
            .finalize(&secp, pair(internal).x_only_public_key().0)
            .unwrap();
        let block = |i: usize| {
            info.control_block(&(leaves[i].clone(), LeafVersion::TapScript))
                .unwrap()
        };
        (
            ScriptBuf::new_p2tr_tweaked(info.output_key()),
            [block(0), block(1), block(2)],
        )
    }

    fn index(leaf: Leaf) -> usize {
        match leaf {
            Leaf::Run => 0,
            Leaf::Drop => 1,
            Leaf::Separator => 2,
        }
    }

    fn base() -> (b::Psbt, Transaction, Vec<TxOut>) {
        let prev = TxOut {
            value: Amount::from_sat(20_000),
            script_pubkey: output(9).0,
        };
        let tx = Transaction {
            version: b::transaction::Version::TWO,
            lock_time: b::absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::new(b::Txid::from_byte_array([1; 32]), 0),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(10_000),
                script_pubkey: prev.script_pubkey.clone(),
            }],
        };
        let mut p = b::Psbt::from_unsigned_tx(tx.clone()).unwrap();
        p.inputs[0].witness_utxo = Some(prev.clone());
        (p, tx, vec![prev])
    }

    fn signature(
        tx: &Transaction,
        prev: &[TxOut],
        hash: TapLeafHash,
        sep: u32,
        forged: bool,
    ) -> Vec<u8> {
        let h = SighashCache::new(tx)
            .taproot_signature_hash(
                0,
                &Prevouts::All(prev),
                None,
                Some((hash, sep)),
                TapSighashType::Default,
            )
            .unwrap();
        let mut s = Secp256k1::new()
            .sign_schnorr_no_aux_rand(&Message::from_digest(h.to_byte_array()), &pair(7))
            .serialize()
            .to_vec();
        if forged {
            s[5] ^= 1;
        }
        s
    }

    fn c0() -> LeafVersion {
        LeafVersion::TapScript
    }

    fn c2() -> LeafVersion {
        LeafVersion::from_consensus(0xc2).unwrap()
    }

    /// A partial signature of key A on `leaf`, filed as `filing` says, with
    /// key A's origin stated. `sep` is the code separator position the
    /// message is signed at.
    fn partial(leaf: Leaf, filing: Filing, sep: u32, forged: bool) -> faraday_psbt::Psbt {
        let (_, blocks) = output(9);
        let (_, other_output) = output(8);
        let s = script(leaf);
        let version = if filing == Filing::FalseVersion {
            c2()
        } else {
            c0()
        };
        let hash = TapLeafHash::from_script(&s, version);
        let (mut p, tx, prev) = base();
        let block = match filing {
            Filing::NotFiled => None,
            Filing::OtherLeafBlock => Some(blocks[index(Leaf::Run)].clone()),
            Filing::OtherOutputBlock => Some(other_output[index(leaf)].clone()),
            _ => Some(blocks[index(leaf)].clone()),
        };
        if let Some(block) = block {
            p.inputs[0].tap_scripts.insert(block, (s, version));
        }
        let a = pair(7).x_only_public_key().0;
        p.inputs[0].tap_key_origins.insert(
            a,
            (
                vec![hash],
                (
                    Fingerprint::from_str("aabbccdd").unwrap(),
                    DerivationPath::from_str("m/86'/0'/0'/0/0").unwrap(),
                ),
            ),
        );
        let sig = signature(&tx, &prev, hash, sep, forged);
        p.inputs[0]
            .tap_script_sigs
            .insert((a, hash), b::taproot::Signature::from_slice(&sig).unwrap());
        faraday_psbt::Psbt::from(p)
    }

    /// A final witness of `leaf`: the signature at `sep`, then `after` (the
    /// signature made into the element it reads as), the script, the block.
    fn finished(
        leaf: Leaf,
        sep: u32,
        forged: bool,
        make: impl Fn(Vec<u8>) -> Vec<Vec<u8>>,
    ) -> faraday_psbt::Psbt {
        let (_, blocks) = output(9);
        let s = script(leaf);
        let hash = TapLeafHash::from_script(&s, c0());
        let (mut p, tx, prev) = base();
        let mut w = make(signature(&tx, &prev, hash, sep, forged));
        w.push(s.to_bytes());
        w.push(blocks[index(leaf)].serialize());
        p.inputs[0].final_script_witness = Some(Witness::from_slice(&w));
        faraday_psbt::Psbt::from(p)
    }

    /// What a person sees of a file: its rows, whether Collect takes it
    /// onto an unsigned copy and the keys it counts, the masters Signing
    /// counts and the ones it still offers Sign, and whether Finish takes it.
    struct Seen {
        rows: Vec<Verdict>,
        taken: bool,
        counted: usize,
        signers: usize,
        to_sign: usize,
        finish: bool,
    }

    fn see(psbt: &faraday_psbt::Psbt) -> Seen {
        let c = testkit::compute();
        let session = Session::default();
        let rows = faraday_psbt::verify::verify_signatures(
            psbt,
            &faraday_psbt::Context {
                network: faraday_bip::keys::Network::Mainnet,
                keys: &[],
                wallets: &[],
                musig_session: None,
            },
        )
        .into_iter()
        .flat_map(|i| i.signatures)
        .map(|r| r.verdict)
        .collect();
        let mut own = psbt.clone();
        own.inner_mut().inputs[0].final_script_witness = None;
        own.inner_mut().inputs[0].tap_script_sigs.clear();
        let mut spend = Spend::new(own, "own.psbt");
        let merged = spend.collect(&c, &session, &psbt.to_bytes(), "cosigner.psbt");
        let got = spend.signing(&c, &session, None, None);
        let (signers, to_sign) = (got.signers, got.to_sign);
        Seen {
            rows,
            taken: merged.is_ok(),
            counted: merged.map(|m| m.len()).unwrap_or(0),
            signers: signers.len(),
            to_sign: to_sign.len(),
            finish: Spend::new(psbt.clone(), "t.psbt").finish().is_ok(),
        }
    }

    const NOT_RUN: Verdict = Verdict::Unchecked(Unchecked::UnsupportedScript);

    /// A leaf is not run only as the output holds it: one not filed, filed
    /// with another leaf's control block, or with a block committing to
    /// another output is no leaf of this output, so a forged signature on it
    /// is a wrong one and the file is refused.
    #[test]
    fn a_leaf_the_output_does_not_hold_is_not_a_leaf_not_run() {
        for filing in [
            Filing::NotFiled,
            Filing::OtherLeafBlock,
            Filing::OtherOutputBlock,
        ] {
            let seen = see(&partial(Leaf::Drop, filing, NO, true));
            assert_eq!(seen.rows, vec![Verdict::Invalid]);
            assert!(!seen.taken);
        }
        // Filed as the output holds it, the same row is not run and taken.
        let seen = see(&partial(Leaf::Drop, Filing::Proper, NO, true));
        assert_eq!(seen.rows, vec![NOT_RUN]);
        assert!(seen.taken);
    }

    /// A partial signature on a leaf that is run: forged is refused, true
    /// counts its key.
    #[test]
    fn a_partial_signature_on_a_run_leaf_counts_when_true_and_is_refused_when_forged() {
        let seen = see(&partial(Leaf::Run, Filing::Proper, NO, true));
        assert_eq!(seen.rows, vec![Verdict::Invalid]);
        assert!(!seen.taken);
        let seen = see(&partial(Leaf::Run, Filing::Proper, NO, false));
        assert_eq!(seen.rows, vec![Verdict::Valid]);
        assert!(seen.taken);
        assert_eq!((seen.counted, seen.signers, seen.to_sign), (1, 1, 0));
    }

    /// A partial signature on a leaf not run is taken, true or forged, and
    /// counts no key: the master is still offered Sign.
    #[test]
    fn a_partial_signature_on_a_leaf_not_run_is_taken_and_counts_no_key() {
        let forged = see(&partial(Leaf::Drop, Filing::Proper, NO, true));
        assert_eq!(forged.rows, vec![NOT_RUN]);
        assert!(forged.taken);
        assert_eq!((forged.counted, forged.signers, forged.to_sign), (0, 0, 1));

        let seen = see(&partial(Leaf::Drop, Filing::Proper, NO, false));
        assert_eq!(seen.rows, vec![Verdict::Valid]);
        assert!(seen.taken);
        assert_eq!((seen.counted, seen.signers, seen.to_sign), (0, 0, 1));
        assert!(!seen.finish);
        // A row read as not run is counted as not checked in the headline.
        let app = testkit::started();
        let check = app.session.nonce_check(
            &testkit::compute(),
            &partial(Leaf::Drop, Filing::Proper, NO, true),
        );
        assert!(check.headline().contains("1 of 1 signature not checked"));
    }

    /// Under OP_CODESEPARATOR the message worked out here is not the real
    /// one, so a signature that verifies against it is no true signature:
    /// final or partial it reads not run and counts no key.
    #[test]
    fn a_signature_that_verifies_on_a_separator_leaf_is_not_called_true() {
        let seen = see(&partial(Leaf::Separator, Filing::Proper, NO, false));
        assert_eq!(seen.rows, vec![NOT_RUN]);
        assert!(seen.taken);
        assert_eq!((seen.counted, seen.signers, seen.to_sign), (0, 0, 1));
        let seen = see(&finished(Leaf::Separator, NO, false, |g| vec![g]));
        assert_eq!(seen.rows, vec![NOT_RUN]);
        assert!(seen.taken && seen.finish);
    }

    /// An entry that states another leaf version than its control block is
    /// no leaf the input files: a forged signature under its hash is wrong,
    /// and a true one counts no key.
    #[test]
    fn a_leaf_filed_with_another_version_than_its_control_block_is_not_filed() {
        let seen = see(&partial(Leaf::Run, Filing::FalseVersion, NO, true));
        assert_eq!(seen.rows, vec![Verdict::Invalid]);
        assert!(!seen.taken);
        let seen = see(&partial(Leaf::Run, Filing::FalseVersion, NO, false));
        assert!(seen.taken);
        assert_eq!((seen.counted, seen.signers), (0, 0));
    }

    /// An element of 65 bytes ending in a sighash byte other than 00 and
    /// 01, and a true signature with 00 appended, on a leaf not run are
    /// data to Finish and read as not run, but Collect refuses a file with
    /// one: it cannot be told from a signature. A 64-byte element is taken.
    #[test]
    fn a_65_byte_element_on_a_leaf_not_run_is_refused_by_collect_though_finish_takes_it() {
        for last in [0x02, 0x81, 0x83] {
            let seen = see(&finished(Leaf::Drop, NO, false, |g| {
                let mut data = vec![0xab; 64];
                data.push(last);
                vec![g, data]
            }));
            assert!(seen.finish, "{last:#x}");
            assert!(!seen.taken, "{last:#x}");
            assert!(!seen.rows.contains(&Verdict::Invalid), "{last:#x}");
        }
        let seen = see(&finished(Leaf::Drop, NO, false, |mut g| {
            g.push(0);
            vec![g, vec![0x42]]
        }));
        assert!(seen.finish && !seen.taken);
        assert_eq!(seen.rows, vec![NOT_RUN]);
        let seen = see(&finished(Leaf::Drop, NO, false, |g| {
            vec![g, vec![0xab; 64]]
        }));
        assert!(seen.finish && seen.taken);
    }
}

/// Taproot spends with a leaf run and a leaf not run, signed by a key
/// outside the app, and a seed loaded here for the rows that name a key
/// held.
mod leaf_not_run {
    use super::*;
    use b::bip32::{DerivationPath, Fingerprint as BFp};
    use b::key::TapTweak;
    use b::opcodes::all::*;
    use b::secp256k1::{Keypair, Secp256k1, SecretKey};
    use b::sighash::{Prevouts, SighashCache, TapSighashType};
    use b::taproot::{ControlBlock, LeafVersion, TapLeafHash, TaprootBuilder};
    use b::{Amount, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
    use faraday_bip::bitcoin as b;
    use faraday_core::wallet::Spend;
    use std::str::FromStr;

    const NO: u32 = 0xffff_ffff;
    const C0: LeafVersion = LeafVersion::TapScript;
    const PATH: &str = "m/86'/0'/0'/0/0";

    fn pair(n: u8) -> Keypair {
        Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[n; 32]).unwrap())
    }
    fn x(k: &Keypair) -> b::XOnlyPublicKey {
        k.x_only_public_key().0
    }
    fn key_script(k: &Keypair, first: Option<b::opcodes::Opcode>) -> ScriptBuf {
        let mut s = b::script::Builder::new();
        if let Some(op) = first {
            s = s.push_opcode(op);
        }
        s.push_x_only_key(&x(k))
            .push_opcode(OP_CHECKSIG)
            .into_script()
    }
    /// The key at `PATH` of the seed "zebra", which a session holds after
    /// `add_key`, and its master fingerprint.
    fn held() -> (Keypair, BFp) {
        let c = testkit::compute();
        let m = faraday_bip::bip39::Mnemonic::parse(
            &c,
            faraday_bip::bip39::Language::English,
            &testkit::test_words("zebra"),
        )
        .unwrap();
        let z = faraday_bip::keys::MasterKey::from_seed(
            &c,
            &m.to_seed(&c, b"").unwrap(),
            testkit::net(),
        );
        let d = z.derive(&c, &DerivationPath::from_str(PATH).unwrap());
        (
            Keypair::from_secret_key(&Secp256k1::new(), d.secret_key()),
            BFp::from(z.fingerprint().0),
        )
    }
    fn foreign() -> BFp {
        BFp::from_str("aabbccdd").unwrap()
    }
    fn app(with_held: bool) -> Faraday {
        let mut a = testkit::started();
        if with_held {
            add_key(&mut a, "zebra");
        }
        a
    }

    struct Tree {
        psbt: b::Psbt,
        tx: Transaction,
        prev: Vec<TxOut>,
        controls: Vec<ControlBlock>,
    }

    /// One input spending a tree of `leaves`, each filed.
    fn tree(leaves: &[ScriptBuf]) -> Tree {
        let secp = Secp256k1::new();
        let mut t = TaprootBuilder::new();
        let depths: &[u8] = if leaves.len() == 1 { &[0] } else { &[1, 1] };
        for (s, d) in leaves.iter().zip(depths) {
            t = t.add_leaf_with_ver(*d, s.clone(), C0).unwrap();
        }
        let info = t.finalize(&secp, x(&pair(9))).unwrap();
        let spk = ScriptBuf::new_p2tr_tweaked(info.output_key());
        let prev = TxOut {
            value: Amount::from_sat(20_000),
            script_pubkey: spk,
        };
        let tx = Transaction {
            version: b::transaction::Version::TWO,
            lock_time: b::absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::new(b::Txid::from_byte_array([1; 32]), 0),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(19_900),
                script_pubkey: ScriptBuf::new_p2tr_tweaked(x(&pair(40)).dangerous_assume_tweaked()),
            }],
        };
        let mut psbt = b::Psbt::from_unsigned_tx(tx.clone()).unwrap();
        psbt.inputs[0].witness_utxo = Some(prev.clone());
        let controls: Vec<_> = leaves
            .iter()
            .map(|s| info.control_block(&(s.clone(), C0)).unwrap())
            .collect();
        for (s, cb) in leaves.iter().zip(&controls) {
            psbt.inputs[0]
                .tap_scripts
                .insert(cb.clone(), (s.clone(), C0));
        }
        Tree {
            psbt,
            tx,
            prev: vec![prev],
            controls,
        }
    }

    impl Tree {
        fn sig(&self, leaf: TapLeafHash, sep: u32, who: &Keypair, forged: bool) -> Vec<u8> {
            let h = SighashCache::new(&self.tx)
                .taproot_signature_hash(
                    0,
                    &Prevouts::All(&self.prev),
                    None,
                    Some((leaf, sep)),
                    TapSighashType::Default,
                )
                .unwrap();
            let mut s = Secp256k1::new()
                .sign_schnorr_with_aux_rand(
                    &b::secp256k1::Message::from_digest(h.to_byte_array()),
                    who,
                    &[0u8; 32],
                )
                .serialize()
                .to_vec();
            if forged {
                s[5] ^= 1;
            }
            s
        }
        fn row(&mut self, who: &Keypair, leaf: TapLeafHash, sig: &[u8]) {
            self.psbt.inputs[0].tap_script_sigs.insert(
                (x(who), leaf),
                b::taproot::Signature::from_slice(sig).unwrap(),
            );
        }
        fn origin(&mut self, who: &Keypair, leaves: Vec<TapLeafHash>, fp: BFp) {
            self.psbt.inputs[0].tap_key_origins.insert(
                x(who),
                (leaves, (fp, DerivationPath::from_str(PATH).unwrap())),
            );
        }
    }

    fn lh(s: &ScriptBuf) -> TapLeafHash {
        TapLeafHash::from_script(s, C0)
    }

    /// A spend of a run leaf `<A> CHECKSIG` and a leaf not run
    /// `OP_DROP <A> CHECKSIG`, both filed, and the leaves' hashes.
    fn two_leaves(a: &Keypair) -> (Tree, ScriptBuf, ScriptBuf) {
        let m = key_script(a, None);
        let n = key_script(a, Some(OP_DROP));
        let mut t = tree(&[m.clone(), n.clone()]);
        t.origin(a, vec![lh(&m), lh(&n)], foreign());
        (t, m, n)
    }

    /// A cosigner's file whose only input is the final witness `elements`.
    fn final_file(t: &Tree, elements: &[Vec<u8>]) -> faraday_psbt::Psbt {
        let mut other = b::Psbt::from_unsigned_tx(t.tx.clone()).unwrap();
        other.inputs[0].witness_utxo = Some(t.prev[0].clone());
        other.inputs[0].final_script_witness = Some(Witness::from_slice(elements));
        faraday_psbt::Psbt::from(other)
    }

    /// Own spend: a true signature on the run leaf finishes it alone. The
    /// file carries a forged final witness on the leaf not run. Collect
    /// takes it, and the spend keeps its signature, its signer and its
    /// finish.
    #[test]
    fn a_forged_final_witness_on_a_leaf_not_run_does_not_replace_a_spend_that_finishes() {
        let c = testkit::compute();
        let a = pair(7);
        let (mut t, m, n) = two_leaves(&a);
        let good = t.sig(lh(&m), NO, &a, false);
        t.row(&a, lh(&m), &good);
        let bad = t.sig(lh(&n), NO, &a, true);
        let file = final_file(
            &t,
            &[
                bad.clone(),
                vec![0x42],
                n.to_bytes(),
                t.controls[1].serialize(),
            ],
        );
        let session = app(false);
        let mut spend = Spend::new(faraday_psbt::Psbt::from(t.psbt.clone()), "own.psbt");
        let before = spend.signing(&c, &session.session, None, None);
        assert!(
            spend
                .collect(&c, &session.session, &file.to_bytes(), "cosigner.psbt")
                .is_ok()
        );
        assert_eq!(spend.signing(&c, &session.session, None, None), before);
        spend.finish().unwrap();
        let tx = spend.finished.as_ref().unwrap();
        let w: Vec<Vec<u8>> = tx.input[0].witness.iter().map(|e| e.to_vec()).collect();
        assert_eq!(w[0], good);
        assert_eq!(w[1], m.to_bytes());
        assert!(!w.contains(&bad));
    }

    /// Own spend: a true row on a leaf not run. The file's forged row for
    /// the same key and leaf does not replace it. On a run leaf the
    /// forged row is still refused.
    #[test]
    fn a_forged_row_on_a_leaf_not_run_does_not_replace_a_true_one() {
        let c = testkit::compute();
        let a = pair(7);
        let (t, m, n) = two_leaves(&a);
        let session = app(false);
        for (leaf, run) in [(lh(&n), false), (lh(&m), true)] {
            let good = t.sig(leaf, NO, &a, false);
            let bad = t.sig(leaf, NO, &a, true);
            let mut own = Tree {
                psbt: t.psbt.clone(),
                tx: t.tx.clone(),
                prev: t.prev.clone(),
                controls: vec![],
            };
            own.row(&a, leaf, &good);
            let mut other = Tree {
                psbt: t.psbt.clone(),
                tx: t.tx.clone(),
                prev: t.prev.clone(),
                controls: vec![],
            };
            other.row(&a, leaf, &bad);
            let mut spend = Spend::new(faraday_psbt::Psbt::from(own.psbt), "own.psbt");
            let r = spend.collect(
                &c,
                &session.session,
                &faraday_psbt::Psbt::from(other.psbt).to_bytes(),
                "cosigner.psbt",
            );
            assert_eq!(r.is_err(), run);
            let kept = spend.psbt.inner().inputs[0]
                .tap_script_sigs
                .get(&(x(&a), leaf))
                .map(|s| s.to_vec());
            assert_eq!(kept.as_deref(), Some(&good[..]));
        }
    }

    /// Reads one partial row of `s` under `a` as the nonce check shows it.
    fn read(
        s: ScriptBuf,
        sep: u32,
        who: &Keypair,
        fp: BFp,
        forged: bool,
        with_held: bool,
    ) -> faraday_core::wallet::NonceCheck {
        let c = testkit::compute();
        let mut t = tree(std::slice::from_ref(&s));
        t.origin(who, vec![lh(&s)], fp);
        let g = t.sig(lh(&s), sep, who, forged);
        t.row(who, lh(&s), &g);
        app(with_held)
            .session
            .nonce_check(&c, &faraday_psbt::Psbt::from(t.psbt))
    }

    /// A signature on a leaf not run never reads wrong because the key is
    /// loaded here: the headline is the count not checked, key loaded or
    /// not. A true one on a leaf that drops reads valid.
    #[test]
    fn a_key_loaded_here_does_not_make_a_signature_on_a_leaf_not_run_wrong() {
        let (h, hfp) = held();
        let drop = key_script(&h, Some(OP_DROP));
        let sep = key_script(&h, Some(OP_CODESEPARATOR));
        let a1 = read(drop.clone(), NO, &h, hfp, false, true);
        assert!(!a1.wrong() && a1.unchecked() == 0);
        assert_eq!(a1.rows[0].recomputed.map(|m| m.is_some()), Some(true));
        let a2 = read(drop.clone(), NO, &h, hfp, true, true);
        let a3b = read(drop, NO, &h, foreign(), true, false);
        assert!(!a2.wrong());
        assert_eq!(a2.headline(), a3b.headline());
        let a4 = read(sep.clone(), 0, &h, hfp, false, true);
        let a4b = read(sep, 0, &h, foreign(), false, false);
        assert!(!a4.wrong());
        assert_eq!(a4.headline(), a4b.headline());
        assert_eq!(a4.unchecked(), 1);
    }

    /// A row on a leaf not run, with a key origin in the file, is not said
    /// to have no origin: its key is not counted because the script was
    /// not run. A key with no origin in the file still has none.
    #[test]
    fn a_row_on_a_leaf_not_run_does_not_say_its_key_has_no_origin() {
        let (h, _) = held();
        let drop = key_script(&h, Some(OP_DROP));
        let stated = read(drop.clone(), NO, &h, foreign(), false, false);
        assert_eq!(stated.rows[0].unmatched, Some(Unmatched::NotRun));
        let c = testkit::compute();
        let mut t = tree(std::slice::from_ref(&drop));
        let g = t.sig(lh(&drop), NO, &h, false);
        t.row(&h, lh(&drop), &g);
        let none = app(false)
            .session
            .nonce_check(&c, &faraday_psbt::Psbt::from(t.psbt));
        assert_eq!(none.rows[0].unmatched, Some(Unmatched::NoOrigin));
    }
}

/// The nonce history: a key's nonce an open vault holds over another
/// message refuses the signature (F227).
mod history {
    use super::*;
    use faraday_bip::bitcoin::hashes::{HashEngine, sha256};
    use faraday_bip::bitcoin::key::{Keypair, TapTweak};
    use faraday_bip::bitcoin::secp256k1::XOnlyPublicKey;
    use faraday_bip::bitcoin::sighash::{Prevouts, TapSighashType};
    use faraday_bip::bitcoin::{Amount, ScriptBuf, Transaction, TxOut, absolute, transaction};
    use faraday_core::memory::nonces::{History, HistorySnapshot, NonceEntry};
    use faraday_core::testkit::{nonce_history_held, nonce_history_on, nonce_history_pending};
    use faraday_core::vault::VaultAction as V;
    use faraday_shell_api::EntropyBytes;

    fn key(n: u8) -> SecretKey {
        SecretKey::from_slice(&[n; 32]).unwrap()
    }

    /// Savings' spend, input 0 alone signed by `d` with nonce `k` over a
    /// transaction of lock time `lock`.
    #[expect(
        clippy::disallowed_methods,
        reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
    )]
    fn signed(lock: u32, d: &SecretKey, k: &SecretKey) -> faraday_psbt::Psbt {
        let mut psbt = signed_elsewhere(How::OwnNonce);
        psbt.inner_mut().unsigned_tx.lock_time = absolute::LockTime::from_consensus(lock);
        let tx = psbt.unsigned_tx().clone();
        for input in &mut psbt.inner_mut().inputs {
            input.partial_sigs.clear();
        }
        let input = psbt.inner().inputs[0].clone();
        let ws = input.witness_script.clone().unwrap();
        let value = input.witness_utxo.as_ref().unwrap().value;
        let hash = SighashCache::new(&tx)
            .p2wsh_signature_hash(0, &ws, value, EcdsaSighashType::All)
            .unwrap();
        // `key(3)` stands for the Savings key the zebra seed holds, whose
        // signature the wallet names; any other key is one no wallet names.
        let d = if *d == key(3) {
            let c = testkit::compute();
            let words = testkit::test_words("zebra");
            let m = faraday_bip::bip39::Mnemonic::parse(
                &c,
                faraday_bip::bip39::Language::English,
                &words,
            )
            .unwrap();
            let zebra = faraday_bip::keys::MasterKey::from_seed(
                &c,
                &m.to_seed(&c, b"").unwrap(),
                testkit::net(),
            );
            let (_, (_, path)) = input
                .bip32_derivation
                .iter()
                .find(|(_, (fp, _))| fp.to_bytes() == zebra.fingerprint().0)
                .expect("zebra is a key of Savings");
            *zebra.derive(&c, path).secret_key()
        } else {
            *d
        };
        let signature = ecdsa_with_k(&d, hash.to_byte_array(), k);
        let pk = PublicKey::new(Point::from_secret_key(&Secp256k1::new(), &d));
        psbt.inner_mut().inputs[0].partial_sigs.insert(
            pk,
            ecdsa::Signature {
                signature,
                sighash_type: EcdsaSighashType::All,
            },
        );
        psbt
    }

    /// An app with a vault open and the PSBTs `files` in its Inbox after it.
    fn with_vault(files: &[&faraday_psbt::Psbt]) -> Faraday {
        let mut app = testkit::started();
        app.storage(StorageEvent::Memory {
            available_mib: 15_000,
        });
        testkit::vault_speed(&mut app, Some(100));
        let mut inbox = vec![(
            "vault.kdbx".to_string(),
            crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7),
        )];
        for (n, f) in files.iter().enumerate() {
            inbox.push((format!("f{n}.psbt"), f.to_bytes()));
        }
        app.storage(StorageEvent::Restored {
            inbox,
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.event(Event::Entropy(EntropyBytes::new([7; 32])));
        app.press(Action::Vault(V::Open(0)));
        crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
        app.press(Action::Vault(V::Unlock));
        for t in 1..60u64 {
            if !app.vaults().open.is_empty() {
                break;
            }
            let _ = app.frame();
            app.event(Event::Tick { now_ms: t * 1000 });
        }
        assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
        app
    }

    /// Registers the Savings wallet, so the history can name the key
    /// `signed` signs under (no key of it is held here).
    fn register_savings(app: &mut Faraday) {
        app.session
            .add_wallet(
                &testkit::compute(),
                "Savings",
                &kit("savings").descriptor,
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
    }

    /// [`with_vault`], with the Savings wallet registered.
    fn with_named_vault(files: &[&faraday_psbt::Psbt]) -> Faraday {
        let mut app = with_vault(files);
        register_savings(&mut app);
        app
    }

    /// The master of the key `signed` names (test key 2).
    fn zebra_master() -> faraday_bip::keys::MasterKey {
        let c = testkit::compute();
        let words = testkit::test_words("zebra");
        let m =
            faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
                .unwrap();
        faraday_bip::keys::MasterKey::from_seed(&c, &m.to_seed(&c, b"").unwrap(), testkit::net())
    }

    /// The master fingerprint of the key `signed` names.
    fn zebra_fingerprint() -> faraday_bip::bitcoin::bip32::Fingerprint {
        zebra_master().fingerprint().0.into()
    }

    const ZEBRA_PATH: &str = "m/86'/1'/0'/0/0";

    /// The secret key test key 2 derives at [`ZEBRA_PATH`].
    fn zebra_secret() -> SecretKey {
        let path = ZEBRA_PATH.parse().unwrap();
        *zebra_master()
            .derive(&testkit::compute(), &path)
            .secret_key()
    }

    /// Test key 2 loaded in the app, so a key derived from it is proved.
    fn load_zebra(app: &mut Faraday) {
        app.session
            .keys
            .add_words(
                &testkit::compute(),
                &testkit::test_words("zebra"),
                "Test key 2",
                None,
            )
            .unwrap();
    }

    fn check_file(app: &mut Faraday, k: usize) -> faraday_core::wallet::NonceCheck {
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
            k,
        )));
        assert_eq!(app.screen, Screen::NonceCheck);
        app.outside_check().expect("the check").check.clone()
    }

    fn shown(app: &mut Faraday) -> String {
        app.drawn_texts().join("\n")
    }

    /// Checks file 1 as the first signature the vault remembers.
    fn remembered(app: &mut Faraday) {
        let first = check_file(app, 1);
        assert_eq!(first.entries().len(), 1);
        let id = app.vaults().open[0].id;
        nonce_history_held(app, id, &first.entries(), 0);
    }

    #[test]
    fn a_key_reusing_a_nonce_on_another_transaction_is_refused_and_nothing_is_offered() {
        let (d, k) = (key(3), key(9));
        let (a, b) = (signed(0, &d, &k), signed(1, &d, &k));
        let mut app = with_named_vault(&[&a, &b]);
        remembered(&mut app);
        let check = check_file(&mut app, 2);
        assert_eq!(check.used_before.len(), 1);
        assert!(check.wrong() && !check.sound());
        let o = app.outside_check().unwrap();
        assert!(o.finished.is_none() && o.hex.is_none());
        let text = shown(&mut app);
        assert!(text.contains(EN.nonce_screen_refused_used), "{text}");
        assert!(text.contains(&check.used_before[0].name), "{text}");
        assert!(text.contains("nonce used before, on another"), "{text}");
        assert!(!text.contains(EN.vault_save_to_a_stick), "{text}");
        assert!(!text.contains(EN.step_show_as_qr), "{text}");
        assert!(!text.contains(EN.nonce_history_clean), "{text}");
    }

    #[test]
    fn the_same_signature_again_is_not_refused_and_the_vault_says_so() {
        let (d, k) = (key(3), key(9));
        let a = signed(0, &d, &k);
        let mut app = with_named_vault(&[&a, &a]);
        remembered(&mut app);
        let check = check_file(&mut app, 2);
        assert!(check.used_before.is_empty());
        assert_eq!(check.history, History::Clean { vaults: 1 });
        assert!(shown(&mut app).contains(EN.nonce_history_clean));
    }

    #[test]
    fn another_key_with_the_same_r_is_not_refused() {
        let k = key(9);
        let (a, b) = (signed(0, &key(3), &k), signed(1, &key(4), &k));
        let mut app = with_named_vault(&[&a, &b]);
        remembered(&mut app);
        let check = check_file(&mut app, 2);
        assert!(check.used_before.is_empty());
        assert!(!check.wrong());
        // The same nonce under another named key is not compared either.
        let first = check_file(&mut app, 1);
        let e = first.entries()[0];
        let mut other = first.clone();
        other.against(&HistorySnapshot::of_entries(&[NonceEntry {
            key: [1; 32],
            msg: [2; 32],
            ..e
        }]));
        assert!(other.used_before.is_empty());
    }

    #[test]
    fn records_that_do_not_read_say_so_and_refuse_nothing() {
        let (d, k) = (key(3), key(9));
        let a = signed(0, &d, &k);
        let mut app = with_named_vault(&[&a]);
        let id = app.vaults().open[0].id;
        nonce_history_held(&mut app, id, &[], 2);
        let check = check_file(&mut app, 1);
        assert_eq!(
            check.history,
            History::Unread {
                vaults: 1,
                records: 2
            }
        );
        assert!(!check.wrong());
        let line = EN.nonce_history_unread.replace("{}", "2");
        assert!(shown(&mut app).contains(&line));
    }

    #[test]
    fn with_no_vault_or_the_setting_off_nothing_is_compared_or_drawn() {
        let (d, k) = (key(3), key(9));
        let (a, b) = (signed(0, &d, &k), signed(1, &d, &k));
        let mut none = with_inbox(vec![("a.psbt", a.to_bytes()), ("b.psbt", b.to_bytes())]);
        let first = check_file(&mut none, 0);
        assert_eq!(first.history, History::Off);
        assert!(nonce_history_pending(&none).is_empty());
        assert!(!shown(&mut none).contains(EN.nonce_history_clean));

        let mut app = with_named_vault(&[&a, &b]);
        remembered(&mut app);
        nonce_history_on(&mut app, false);
        let check = check_file(&mut app, 2);
        assert_eq!(check.history, History::Off);
        assert!(check.used_before.is_empty() && !check.wrong());
    }

    /// BIP-340 with the nonce `k` chosen here, over `m`.
    #[expect(
        clippy::disallowed_methods,
        reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
    )]
    fn schnorr_with_k(pair: &Keypair, m: [u8; 32], k: &SecretKey) -> Vec<u8> {
        let secp = Secp256k1::new();
        let tagged = |parts: &[&[u8]]| {
            let tag = sha256::Hash::hash(b"BIP0340/challenge");
            let mut e = sha256::Hash::engine();
            e.input(tag.as_byte_array());
            e.input(tag.as_byte_array());
            for p in parts {
                e.input(p);
            }
            sha256::Hash::from_engine(e).to_byte_array()
        };
        let (px, parity) = pair.x_only_public_key();
        let mut d = pair.secret_key();
        if parity == faraday_bip::bitcoin::secp256k1::Parity::Odd {
            d = d.negate();
        }
        let r_point = Point::from_secret_key(&secp, k);
        let (rx, rpar) = r_point.x_only_public_key();
        let k = if rpar == faraday_bip::bitcoin::secp256k1::Parity::Odd {
            k.negate()
        } else {
            *k
        };
        let e = Scalar::from_be_bytes(tagged(&[&rx.serialize(), &px.serialize(), &m])).unwrap();
        let s = d
            .mul_tweak(&e)
            .unwrap()
            .add_tweak(&Scalar::from(k))
            .unwrap();
        let mut out = rx.serialize().to_vec();
        out.extend_from_slice(&s.secret_bytes());
        out
    }

    /// A taproot key-path spend of one input, locked until `lock`, signed
    /// under nonce `k`.
    fn key_path(lock: u32, k: &SecretKey) -> faraday_psbt::Psbt {
        key_path_under(lock, k, &zebra_secret())
    }

    /// [`key_path`] under the internal key of `secret`, filed under test
    /// key 2's fingerprint whether or not that key derives it.
    fn key_path_under(lock: u32, k: &SecretKey, secret: &SecretKey) -> faraday_psbt::Psbt {
        let secp = Secp256k1::new();
        let internal = Keypair::from_secret_key(&secp, secret);
        let tweaked = internal.tap_tweak(&secp, None).to_keypair();
        let (out_key, _) = XOnlyPublicKey::from_keypair(&tweaked);
        let spk = ScriptBuf::new_p2tr_tweaked(
            faraday_bip::bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(out_key),
        );
        let prev = TxOut {
            value: Amount::from_sat(100_000),
            script_pubkey: spk,
        };
        let tx = Transaction {
            version: transaction::Version::TWO,
            lock_time: absolute::LockTime::from_consensus(lock),
            input: vec![faraday_bip::bitcoin::TxIn {
                previous_output: OutPoint::new(
                    faraday_bip::bitcoin::Txid::from_byte_array([8; 32]),
                    0,
                ),
                sequence: faraday_bip::bitcoin::Sequence::ENABLE_LOCKTIME_NO_RBF,
                ..Default::default()
            }],
            output: vec![TxOut {
                value: Amount::from_sat(90_000),
                script_pubkey: prev.script_pubkey.clone(),
            }],
        };
        let h = SighashCache::new(&tx)
            .taproot_key_spend_signature_hash(
                0,
                &Prevouts::All(std::slice::from_ref(&prev)),
                TapSighashType::Default,
            )
            .unwrap();
        let sig = schnorr_with_k(&tweaked, h.to_byte_array(), k);
        let mut p = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
        // The internal key's origin names a master, as a wallet's PSBT does.
        let (internal_x, _) = internal.x_only_public_key();
        p.inputs[0].tap_internal_key = Some(internal_x);
        p.inputs[0].tap_key_origins.insert(
            internal_x,
            (
                Vec::new(),
                (zebra_fingerprint(), ZEBRA_PATH.parse().unwrap()),
            ),
        );
        p.inputs[0].witness_utxo = Some(prev);
        p.inputs[0].tap_key_sig =
            Some(faraday_bip::bitcoin::taproot::Signature::from_slice(&sig).unwrap());
        faraday_psbt::Psbt::from(p)
    }

    #[test]
    fn a_taproot_key_path_nonce_used_before_is_refused() {
        let k = key(9);
        let (a, b) = (key_path(0, &k), key_path(1, &k));
        let mut app = with_vault(&[&a, &b]);
        load_zebra(&mut app);
        remembered(&mut app);
        let check = check_file(&mut app, 2);
        assert_eq!(check.used_before.len(), 1, "{:?}", check.history);
        assert!(check.wrong());
    }

    /// The Core wallet's MuSig2 file, one cosigner's partial signature in it.
    fn musig_round() -> (
        faraday_psbt::Psbt,
        Vec<faraday_psbt::musig::VerifiedPartial>,
    ) {
        use faraday_bip::policy::WalletPolicy;
        let c = testkit::compute();
        let policy = include_str!("../../../../tools/vectors/psbt/wallet-musig-regtest.policy");
        let text =
            include_str!("../../../../tools/vectors/psbt/wallet-musig-core-first-signed.psbt");
        let psbt = faraday_psbt::Psbt::parse_base64(&c, text.trim()).unwrap();
        let mut session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Regtest);
        session
            .add_policy(
                &c,
                "Core",
                WalletPolicy::parse(&c, policy).unwrap(),
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
        let seen = session.inspect(&c, &psbt);
        let round = seen.inputs[0].musig.as_ref().unwrap();
        let got = faraday_psbt::musig::verified_rounds(&c, &psbt, 0, round);
        (psbt, got)
    }

    #[test]
    fn a_musig_partial_in_a_file_gives_its_first_nonce_and_the_same_round_twice_is_one_entry() {
        let (psbt, got) = musig_round();
        assert_eq!(got.len(), 1, "the one partial signature that verifies");
        let nonces = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
        let theirs = nonces
            .iter()
            .find(|n| n.participant == got[0].participant)
            .expect("the participant's public nonce");
        assert_eq!(
            got[0].nonce_x[..],
            theirs.value.serialize()[1..33],
            "the x of the first point"
        );
        let (_, again) = musig_round();
        assert_eq!(got, again, "the same round is the same entry");
        let e = NonceEntry::of_partial(&got[0]);
        let held = HistorySnapshot::of_entries(&[e, NonceEntry::of_partial(&again[0])]);
        let app = with_inbox(vec![]);
        let mut check = app.session.nonce_check(&testkit::compute(), &psbt);
        check.partials = vec![(0, "k".into(), e)];
        check.against(&held);
        assert!(check.used_before.is_empty(), "the round collected twice");
        let other = NonceEntry { msg: [9; 32], ..e };
        check.partials = vec![(0, "k".into(), other)];
        check.against(&held);
        assert_eq!(check.used_before.len(), 1, "that nonce in another round");
    }

    #[test]
    fn a_page_check_made_with_nothing_held_is_refused_once_a_vault_holds_the_nonce() {
        let (d, k) = (key(3), key(9));
        let (a, b) = (signed(0, &d, &k), signed(1, &d, &k));
        let mut app = with_named_vault(&[&a, &b]);
        remembered(&mut app);
        nonce_history_on(&mut app, false);
        let check = check_file(&mut app, 2);
        assert_eq!(check.history, History::Off);
        assert!(shown(&mut app).contains(EN.vault_save_to_a_stick));
        nonce_history_on(&mut app, true);
        // The presses that carry the check on compare it again first, with
        // no other press before them, and do nothing.
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Qr));
        assert_ne!(
            app.sheet,
            Some(faraday_core::Sheet::Qr),
            "no QR sheet opens"
        );
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
        assert!(app.outbox.is_empty(), "nothing is put in the outbox");
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Decode));
        assert_eq!(app.screen, Screen::NonceCheck);
        let text = shown(&mut app);
        assert!(!text.contains(EN.vault_save_to_a_stick), "{text}");
        assert!(text.contains(EN.nonce_screen_refused_used), "{text}");
        // And back once the history is off again.
        nonce_history_on(&mut app, false);
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Hex));
        let text = shown(&mut app);
        assert!(text.contains(EN.vault_save_to_a_stick), "{text}");
        assert!(!text.contains(EN.nonce_screen_refused_used), "{text}");
    }

    #[test]
    fn two_files_opened_in_turn_are_compared_and_one_opened_again_is_not() {
        let (d, k) = (key(3), key(9));
        let (a, b) = (signed(0, &d, &k), signed(1, &d, &k));
        let mut app = with_named_vault(&[&a, &b]);
        let id = app.vaults().open[0].id;
        nonce_history_held(&mut app, id, &[], 0);
        let first = check_file(&mut app, 1);
        assert!(first.used_before.is_empty());
        let again = check_file(&mut app, 1);
        assert!(again.used_before.is_empty(), "the same file again");
        let second = check_file(&mut app, 2);
        assert_eq!(second.used_before.len(), 1, "another file, the same nonce");
        assert!(second.wrong());
    }

    #[test]
    fn a_forged_signature_under_a_held_key_and_nonce_is_neither_recorded_nor_refused_as_reuse() {
        let (d, k) = (key(3), key(9));
        let mut forged = signed(1, &d, &k);
        forged.inner_mut().unsigned_tx.lock_time = absolute::LockTime::from_consensus(2);
        let a = signed(0, &d, &k);
        let mut app = with_named_vault(&[&a, &forged]);
        remembered(&mut app);
        let before = nonce_history_pending(&app).len();
        let check = check_file(&mut app, 2);
        assert_eq!(check.rows[0].verdict, Verdict::Invalid);
        assert!(check.entries().is_empty());
        assert!(check.used_before.is_empty());
        assert_eq!(nonce_history_pending(&app).len(), before);
    }

    #[test]
    fn a_key_no_wallet_here_names_is_neither_recorded_nor_compared() {
        let (d, k) = (key(3), key(9));
        let (a, b) = (signed(0, &d, &k), signed(1, &d, &k));
        let mut app = with_vault(&[&a, &b]);
        let first = check_file(&mut app, 1);
        assert!(first.entries().is_empty());
        assert!(nonce_history_pending(&app).is_empty());
        let second = check_file(&mut app, 2);
        assert!(second.used_before.is_empty() && !second.wrong());
    }

    #[test]
    fn a_taproot_script_path_nonce_is_recorded_and_refused_when_used_before() {
        let mut app = with_vault(&[]);
        load_zebra(&mut app);
        let mut psbt = taproot_psbt_under(
            &[Tap::Leaf {
                kind: LeafKind::Run,
                forged: false,
                partial: true,
            }],
            &zebra_secret(),
        );
        let ((a, leaf), _) = psbt.inner().inputs[0]
            .tap_script_sigs
            .iter()
            .next()
            .map(|(k, v)| (*k, *v))
            .unwrap();
        psbt.inner_mut().inputs[0].tap_key_origins.insert(
            a,
            (
                vec![leaf],
                (zebra_fingerprint(), ZEBRA_PATH.parse().unwrap()),
            ),
        );
        let mut check = app.session.nonce_check(&testkit::compute(), &psbt);
        let entries = check.entries();
        assert_eq!(entries.len(), 1, "{:?}", check.rows.len());
        let other = NonceEntry {
            msg: [1; 32],
            ..entries[0]
        };
        check.against(&HistorySnapshot::of_entries(&[other]));
        assert_eq!(check.used_before.len(), 1);
        assert!(check.wrong());
    }

    #[test]
    fn a_key_filed_under_a_held_fingerprint_but_not_derived_from_it_is_not_recorded() {
        let k = key(9);
        let real = (key_path(0, &k), key_path(1, &k));
        // Throwaway keys, each filed under test key 2's fingerprint.
        let throwaway: Vec<_> = (20..26u8)
            .map(|n| key_path_under(u32::from(n), &key(n), &key(n)))
            .collect();
        let mut files: Vec<&faraday_psbt::Psbt> = vec![&real.0, &real.1];
        files.extend(throwaway.iter());
        let mut app = with_vault(&files);
        load_zebra(&mut app);
        remembered(&mut app);
        for n in 0..throwaway.len() {
            let check = check_file(&mut app, 3 + n);
            assert!(check.entries().is_empty(), "file {n} is not recorded");
        }
        assert!(nonce_history_pending(&app).is_empty(), "nothing was noted");
        let again = check_file(&mut app, 2);
        assert_eq!(again.used_before.len(), 1, "the real key is still caught");
    }

    #[test]
    fn a_key_in_the_history_is_one_line_however_many_inputs_it_signed() {
        let (psbt, got) = musig_round();
        let app = with_inbox(vec![]);
        let mut check = app.session.nonce_check(&testkit::compute(), &psbt);
        let e = NonceEntry::of_partial(&got[0]);
        let f = NonceEntry { key: [4; 32], ..e };
        check.partials = vec![(0, "a".into(), e), (1, "a".into(), e), (0, "b".into(), f)];
        let held = HistorySnapshot::of_entries(&[
            NonceEntry { msg: [1; 32], ..e },
            NonceEntry { msg: [1; 32], ..f },
        ]);
        check.against(&held);
        assert_eq!(check.used_before.len(), 2, "one line per key");
        let first = check.used_before.iter().find(|u| u.key == e.key).unwrap();
        assert_eq!(first.inputs, vec![0, 1]);
    }

    #[test]
    fn what_is_checked_and_not_yet_sealed_goes_with_the_last_vault_closed() {
        let (d, k) = (key(3), key(9));
        let a = signed(0, &d, &k);
        let mut app = with_named_vault(&[&a]);
        let id = app.vaults().open[0].id;
        nonce_history_held(&mut app, id, &[], 0);
        let _ = check_file(&mut app, 1);
        assert_eq!(nonce_history_pending(&app).len(), 1);
        // Reading the vault again keeps what is not yet sealed.
        nonce_history_held(&mut app, id, &[], 0);
        assert_eq!(nonce_history_pending(&app).len(), 1, "kept on a re-read");
        faraday_core::testkit::nonce_history_drop(&mut app, id);
        assert!(nonce_history_pending(&app).is_empty());
    }

    #[test]
    fn what_is_waiting_for_a_vault_stops_at_ten_thousand_signatures() {
        let mut app = with_vault(&[]);
        let id = app.vaults().open[0].id;
        nonce_history_held(&mut app, id, &[], 0);
        let entry = |t: u8, n: u8| NonceEntry {
            key: [1; 32],
            r: [
                t, n, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ],
            msg: [t; 32],
        };
        for t in 1..=101u8 {
            let entries: Vec<_> = (0..101u8).map(|n| entry(t, n)).collect();
            faraday_core::testkit::nonce_history_note(&mut app, [t; 32], &entries);
        }
        let pending = nonce_history_pending(&app);
        let total: usize = pending.iter().map(|(_, e)| e.len()).sum();
        let cap = faraday_core::memory::nonces::CAP;
        assert!(total <= cap && total + 101 > cap, "{total}");
        assert!(
            pending.iter().any(|(t, _)| *t == [101; 32]),
            "the newest stays"
        );
        assert!(
            pending.iter().all(|(t, _)| *t != [1; 32]),
            "the oldest goes"
        );
    }

    #[test]
    fn a_nonce_reused_inside_one_file_is_not_read_as_used_on_another_transaction_after_back() {
        let a = signed_elsewhere(How::SameNonce);
        let mut app = with_named_vault(&[&a]);
        let id = app.vaults().open[0].id;
        nonce_history_held(&mut app, id, &[], 0);
        let first = check_file(&mut app, 1);
        assert!(!first.reused.is_empty());
        // The keys change, so Back makes the check again.
        add_key(&mut app, "zebra");
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Back));
        assert_eq!(app.screen, Screen::NonceCheck);
        let check = app.outside_check().unwrap().check.clone();
        assert!(!check.reused.is_empty() && check.wrong());
        assert!(check.used_before.is_empty());
        let text = shown(&mut app);
        assert!(!text.contains("nonce used before, on another"), "{text}");
    }
}
