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
        let m =
            faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
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
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
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
            previous_output: OutPoint::new(faraday_bip::bitcoin::Txid::from_byte_array([8; 32]), 0),
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
    let policy = include_str!("../../../../../tools/vectors/psbt/wallet-musig-regtest.policy");
    let text =
        include_str!("../../../../../tools/vectors/psbt/wallet-musig-core-first-signed.psbt");
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
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Hex));
    let text = shown(&mut app);
    assert!(!text.contains(EN.vault_save_to_a_stick), "{text}");
    assert!(text.contains(EN.nonce_screen_refused_used), "{text}");
    // A refusal found stays while the page holds the file, history off or not.
    nonce_history_on(&mut app, false);
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Hex));
    let text = shown(&mut app);
    assert!(!text.contains(EN.vault_save_to_a_stick), "{text}");
    assert!(text.contains(EN.nonce_screen_refused_used), "{text}");
}

#[test]
fn the_nonce_check_page_keeps_its_refusal_after_the_vault_is_dropped() {
    let (d, k) = (key(3), key(9));
    let (a, b) = (signed(0, &d, &k), signed(1, &d, &k));
    let mut app = with_named_vault(&[&a, &b]);
    remembered(&mut app);
    let _ = check_file(&mut app, 2);
    assert!(shown(&mut app).contains(EN.nonce_screen_refused_used));
    let id = app.vaults().open[0].id;
    faraday_core::testkit::nonce_history_drop(&mut app, id);
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Hex));
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Qr));
    assert_ne!(app.sheet, Some(faraday_core::Sheet::Qr));
    let text = shown(&mut app);
    assert!(!text.contains(EN.vault_save_to_a_stick), "{text}");
    assert!(text.contains(EN.nonce_screen_refused_used), "{text}");
}

/// A page whose history was off when it opened, and then turns on and
/// holds the nonce.
fn page_then_history_holds() -> Faraday {
    let (d, k) = (key(3), key(9));
    let (a, b) = (signed(0, &d, &k), signed(1, &d, &k));
    let mut app = with_named_vault(&[&a, &b]);
    remembered(&mut app);
    nonce_history_on(&mut app, false);
    let check = check_file(&mut app, 2);
    assert_eq!(check.history, History::Off);
    nonce_history_on(&mut app, true);
    app
}

#[test]
fn save_to_a_stick_as_the_first_press_after_the_history_changes_puts_nothing_out() {
    let mut app = page_then_history_holds();
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    assert!(app.outbox.is_empty(), "nothing is put in the outbox");
    assert!(shown(&mut app).contains(EN.nonce_screen_refused_used));
}

#[test]
fn decode_as_the_first_press_after_the_history_changes_opens_nothing() {
    let mut app = page_then_history_holds();
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Decode));
    assert_eq!(app.screen, Screen::NonceCheck);
    assert!(app.decode().is_none());
}

#[test]
fn the_decode_screen_offers_no_hex_and_no_code_once_the_check_refuses() {
    use faraday_core::wallet::Origin;
    let k = kit("spending");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut app = with_vault(&[&unsigned]);
    app.session
        .add_wallet(
            &testkit::compute(),
            "Spending",
            &k.descriptor,
            "Test wallet",
            Origin::Named,
        )
        .unwrap();
    add_key(&mut app, "bacon");
    let id = app.vaults().open[0].id;
    nonce_history_held(&mut app, id, &[], 0);
    app.press(Action::StartSpend(1));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.finished.is_some());
    let name = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with("-signed.psbt"))
        .map(|i| i.name.clone())
        .expect("the signed PSBT");
    testkit::outbox_to_files(&mut app, &[name]);
    let (_, own) = nonce_history_pending(&app).remove(0);
    nonce_history_held(
        &mut app,
        id,
        &[NonceEntry {
            msg: [9; 32],
            ..own[0]
        }],
        0,
    );
    nonce_history_on(&mut app, false);
    let last = app.inbox().len() - 1;
    let _ = check_file(&mut app, last);
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Decode));
    assert_eq!(app.screen, Screen::Decode);
    let text = shown(&mut app);
    assert!(text.contains(EN.decode_show_the_hex), "{text}");
    assert!(text.contains(EN.decode_show_as_qr), "{text}");
    let hex = app.decode().unwrap().decoded.hex.clone();
    nonce_history_on(&mut app, true);
    // The hex was asked for before the history turned on.
    app.press(Action::DecodeHex);
    let text = shown(&mut app);
    assert!(!text.contains(EN.decode_show_the_hex), "{text}");
    assert!(!text.contains(EN.decode_hide_the_hex), "{text}");
    assert!(!text.contains(EN.decode_show_as_qr), "{text}");
    assert!(!text.contains(&hex[..16]), "the hex is not shown");
}

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
)]
fn an_ecdsa_key_filed_under_a_held_fingerprint_but_not_derived_from_it_is_not_recorded() {
    let (d, k) = (key(4), key(9));
    let mut a = signed(0, &d, &k);
    let pk = PublicKey::new(Point::from_secret_key(&Secp256k1::new(), &d));
    let (fp, path) = a.inner().inputs[0]
        .bip32_derivation
        .values()
        .find(|(fp, _)| *fp == zebra_fingerprint())
        .cloned()
        .expect("zebra is a key of Savings");
    a.inner_mut().inputs[0]
        .bip32_derivation
        .insert(pk.inner, (fp, path));
    let mut app = with_named_vault(&[&a]);
    load_zebra(&mut app);
    let check = check_file(&mut app, 1);
    assert_eq!(check.rows.len(), 1);
    assert_eq!(check.rows[0].verdict, Verdict::Valid);
    assert!(check.entries().is_empty(), "not recorded");
    assert!(nonce_history_pending(&app).is_empty());
}

#[test]
fn a_script_path_key_filed_under_a_held_fingerprint_but_not_derived_from_it_is_not_recorded() {
    let mut app = with_vault(&[]);
    load_zebra(&mut app);
    let mut psbt = taproot_psbt_under(
        &[Tap::Leaf {
            kind: LeafKind::Run,
            forged: false,
            partial: true,
        }],
        &key(20),
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
    let check = app.session.nonce_check(&testkit::compute(), &psbt);
    assert_eq!(check.rows[0].verdict, Verdict::Valid);
    assert!(check.entries().is_empty(), "not recorded");
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
    assert_eq!(first.name, "a");
    let second = check.used_before.iter().find(|u| u.key == f.key).unwrap();
    assert_eq!(second.name, "b");
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
            t, n, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
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

/// Presses Spend on file `k` of the Inbox.
fn start(app: &mut Faraday, k: usize) {
    app.press(Action::StartSpend(k));
    if app.spend().is_none() {
        panic!("the spend did not open: {:?}", app.toast_text());
    }
}

/// Opens the Finish step of the spend.
fn read_all_steps(app: &mut Faraday) {
    let finish = faraday_core::wallet::step::FINISH;
    if app.spend().is_some_and(|s| s.open != Some(finish)) {
        app.press(Action::Step(finish));
    }
}

/// Whether Finish, Save to a stick and the QR code give anything.
fn gives_out(app: &mut Faraday) -> bool {
    let before = app.outbox.len();
    app.press(Action::SignedToOutbox);
    app.press(Action::TxToOutbox);
    let saved = app.outbox.len() != before;
    app.press(Action::QrSigned);
    let shown = app.sheet == Some(Sheet::Qr);
    saved || shown || app.spend().is_some_and(|s| s.spend.finished.is_some())
}

#[test]
fn a_clean_spend_finishes_and_is_saved_as_before() {
    let k = key(9);
    let a = key_path(0, &k);
    let mut app = with_named_vault(&[&a]);
    load_zebra(&mut app);
    let id = app.vaults().open[0].id;
    nonce_history_held(&mut app, id, &[], 0);
    start(&mut app, 1);
    let s = app.spend().unwrap();
    assert!(s.complete && s.spend.finished.is_some());
    assert!(
        !app.outbox.is_empty(),
        "both results are made for the stick"
    );
    assert_eq!(nonce_history_pending(&app).len(), 1, "and it is recorded");
}

#[test]
fn a_spend_whose_signature_reuses_a_nonce_held_in_the_vault_cannot_finish_save_or_show() {
    let k = key(9);
    let (a, b) = (key_path(0, &k), key_path(1, &k));
    let mut app = with_named_vault(&[&a, &b]);
    load_zebra(&mut app);
    remembered(&mut app);
    let outbox = app.outbox.len();
    start(&mut app, 2);
    let s = app.spend().unwrap();
    assert!(!s.complete && s.spend.finished.is_none());
    assert_eq!(app.outbox.len(), outbox, "nothing is made for the stick");
    assert!(!gives_out(&mut app));
    read_all_steps(&mut app);
    let text = shown(&mut app);
    assert!(text.contains(EN.spend_nonce_held), "{text}");
    assert!(text.contains("nonce used before"), "{text}");
}

/// A spend held by a nonce the vault holds, open on Savings.
fn held_spend() -> (Faraday, faraday_keys::vault::VaultId) {
    let k = key(9);
    let (a, b) = (key_path(0, &k), key_path(1, &k));
    let mut app = with_named_vault(&[&a, &b]);
    load_zebra(&mut app);
    remembered(&mut app);
    let id = app.vaults().open[0].id;
    start(&mut app, 2);
    assert!(!app.spend().unwrap().complete);
    (app, id)
}

/// The spend is still held, names its key and gives nothing out.
fn assert_still_held(app: &mut Faraday) {
    let s = app.spend().unwrap();
    assert!(!s.complete && s.spend.finished.is_none() && s.spend.nonce_hold);
    assert!(!gives_out(app));
    assert!(!app.outbox.iter().any(|i| i.name.ends_with("-part.psbt")));
    read_all_steps(app);
    let text = shown(app);
    assert!(text.contains(EN.spend_nonce_held), "{text}");
    assert!(text.contains("nonce used before"), "{text}");
}

#[test]
fn a_spend_held_by_a_nonce_used_before_stays_held_after_its_vault_is_dropped() {
    let (mut app, id) = held_spend();
    faraday_core::testkit::nonce_history_drop(&mut app, id);
    app.press(Action::ToggleTable);
    assert_still_held(&mut app);
}

#[test]
fn a_spend_held_by_a_nonce_used_before_stays_held_after_the_history_is_set_off() {
    let (mut app, _) = held_spend();
    nonce_history_on(&mut app, false);
    app.press(Action::ToggleTable);
    assert_still_held(&mut app);
    nonce_history_on(&mut app, true);
    assert_still_held(&mut app);
}

#[test]
fn a_spend_stays_held_when_keys_or_wallets_change_collect_or_sign_here_runs() {
    for drop_vault in [false, true] {
        let (mut app, id) = held_spend();
        if drop_vault {
            faraday_core::testkit::nonce_history_drop(&mut app, id);
        } else {
            nonce_history_on(&mut app, false);
        }
        assert_still_held(&mut app);
        add_key(&mut app, "bacon");
        let _ = app.frame();
        assert_still_held(&mut app);
        app.session
            .add_wallet(
                &testkit::compute(),
                "Taproot",
                &kit("taproot").descriptor,
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
        let _ = app.frame();
        assert_still_held(&mut app);
        app.press(Action::Collect(1));
        faraday_core::testkit::settle(&mut app);
        let _ = app.frame();
        assert_still_held(&mut app);
        app.press(Action::SignHere);
        faraday_core::testkit::settle(&mut app);
        let _ = app.frame();
        assert_still_held(&mut app);
    }
}

#[test]
fn a_collect_answering_after_the_hold_came_on_leaves_the_spend_held() {
    let k = key(9);
    let (a, b) = (key_path(0, &k), key_path(1, &k));
    let mut more = b.clone();
    more.inner_mut().inputs[0].unknown.insert(
        faraday_bip::bitcoin::psbt::raw::Key {
            type_value: 0xfc,
            key: vec![1],
        },
        vec![1],
    );
    let mut app = with_named_vault(&[&a, &b, &more]);
    load_zebra(&mut app);
    let first = check_file(&mut app, 1);
    let id = app.vaults().open[0].id;
    start(&mut app, 2);
    assert!(app.spend().unwrap().complete, "not held yet");
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Collect(3));
    nonce_history_held(&mut app, id, &first.entries(), 0);
    app.press(Action::ToggleTable);
    testkit::release_worker();
    testkit::settle(&mut app);
    let _ = app.frame();
    assert_still_held(&mut app);
}

#[test]
fn the_nonce_check_page_stays_refused_when_keys_or_wallets_change_with_the_vault_dropped() {
    let (d, k) = (key(3), key(9));
    let (a, b) = (signed(0, &d, &k), signed(1, &d, &k));
    let mut app = with_named_vault(&[&a, &b]);
    remembered(&mut app);
    let _ = check_file(&mut app, 2);
    let id = app.vaults().open[0].id;
    faraday_core::testkit::nonce_history_drop(&mut app, id);
    load_zebra(&mut app);
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Decode));
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Back));
    let text = shown(&mut app);
    assert!(!text.contains(EN.vault_save_to_a_stick), "{text}");
    assert!(text.contains(EN.nonce_screen_refused_used), "{text}");
}

#[test]
fn a_spend_closed_and_opened_again_with_no_vault_open_is_not_held() {
    let (mut app, id) = held_spend();
    faraday_core::testkit::nonce_history_drop(&mut app, id);
    app.press(Action::RemoveKeys);
    assert!(app.spend().is_none(), "the spend is closed");
    load_zebra(&mut app);
    start(&mut app, 2);
    let s = app.spend().unwrap();
    assert!(!s.spend.nonce_hold);
    assert!(s.complete && s.spend.finished.is_some());
}

/// A taproot key-path spend of two inputs, both signed under nonce `k`.
fn two_inputs_one_nonce(k: &SecretKey) -> faraday_psbt::Psbt {
    let one = key_path(0, k);
    let secp = Secp256k1::new();
    let internal = Keypair::from_secret_key(&secp, &zebra_secret());
    let tweaked = internal.tap_tweak(&secp, None).to_keypair();
    let mut p = one.inner().clone();
    p.unsigned_tx.input.push(faraday_bip::bitcoin::TxIn {
        previous_output: OutPoint::new(faraday_bip::bitcoin::Txid::from_byte_array([8; 32]), 1),
        ..p.unsigned_tx.input[0].clone()
    });
    p.inputs.push(p.inputs[0].clone());
    let prevs: Vec<TxOut> = p
        .inputs
        .iter()
        .map(|i| i.witness_utxo.clone().unwrap())
        .collect();
    let tx = p.unsigned_tx.clone();
    for n in 0..2 {
        let h = SighashCache::new(&tx)
            .taproot_key_spend_signature_hash(n, &Prevouts::All(&prevs), TapSighashType::Default)
            .unwrap();
        let sig = schnorr_with_k(&tweaked, h.to_byte_array(), k);
        p.inputs[n].tap_key_sig =
            Some(faraday_bip::bitcoin::taproot::Signature::from_slice(&sig).unwrap());
    }
    faraday_psbt::Psbt::from(p)
}

#[test]
fn a_spend_that_uses_one_nonce_twice_inside_the_transaction_cannot_finish_save_or_show() {
    let a = two_inputs_one_nonce(&key(9));
    let mut app = with_named_vault(&[&a]);
    load_zebra(&mut app);
    let id = app.vaults().open[0].id;
    nonce_history_held(&mut app, id, &[], 0);
    start(&mut app, 1);
    let s = app.spend().unwrap();
    assert!(!s.complete && s.spend.finished.is_none());
    assert!(app.outbox.is_empty());
    assert!(!gives_out(&mut app));
    read_all_steps(&mut app);
    let text = shown(&mut app);
    assert!(text.contains(EN.spend_nonce_held), "{text}");
    // With no vault open, the same transaction is refused all the same.
    let mut none = with_inbox(vec![("a.psbt", a.to_bytes())]);
    register_savings(&mut none);
    load_zebra(&mut none);
    start(&mut none, 0);
    assert!(!none.spend().unwrap().complete);
    assert!(!gives_out(&mut none));
}

#[test]
fn a_musig_partial_signature_in_a_spend_is_recorded_and_compared() {
    use faraday_bip::policy::WalletPolicy;
    let (psbt, got) = musig_round();
    let mut app = with_named_vault(&[&psbt]);
    let policy = include_str!("../../../../../tools/vectors/psbt/wallet-musig-regtest.policy");
    app.session
        .add_policy(
            &testkit::compute(),
            "Core",
            WalletPolicy::parse(&testkit::compute(), policy).unwrap(),
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let id = app.vaults().open[0].id;
    let entry = NonceEntry::of_partial(&got[0]);
    // Another round of the same participant's nonce is held.
    nonce_history_held(
        &mut app,
        id,
        &[NonceEntry {
            msg: [9; 32],
            ..entry
        }],
        0,
    );
    start(&mut app, 1);
    let pending = nonce_history_pending(&app);
    assert!(
        pending.iter().any(|(_, e)| e.contains(&entry)),
        "the partial signature is recorded"
    );
    assert!(!gives_out(&mut app));
    read_all_steps(&mut app);
    let text = shown(&mut app);
    assert!(text.contains("nonce used before"), "{text}");
}

/// A 2-of-3 spend signed here with the first key, the vault holding
/// `held` for it: the files the Outbox then has.
fn part_after_signing(held: Option<NonceEntry>) -> (Vec<String>, Vec<NonceEntry>) {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut app = with_vault(&[&unsigned]);
    register_savings(&mut app);
    add_key(&mut app, "bacon");
    let id = app.vaults().open[0].id;
    nonce_history_held(&mut app, id, held.as_slice(), 0);
    start(&mut app, 1);
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    let names = app.outbox.iter().map(|i| i.name.clone()).collect();
    let own = nonce_history_pending(&app)
        .into_iter()
        .flat_map(|(_, e)| e)
        .collect();
    (names, own)
}

#[test]
fn the_part_file_for_the_next_signer_is_not_made_while_the_history_holds_the_spend() {
    let (names, own) = part_after_signing(None);
    assert!(names.iter().any(|n| n.ends_with("-part.psbt")), "{names:?}");
    let reused = NonceEntry {
        msg: [9; 32],
        ..own[0]
    };
    let (names, _) = part_after_signing(Some(reused));
    assert!(
        !names.iter().any(|n| n.ends_with("-part.psbt")),
        "{names:?}"
    );
}

/// A spend that finished while the history was off, its files in the
/// Outbox, and the history then on, holding a nonce its signature used.
fn finished_then_history_holds() -> Faraday {
    let k = key(9);
    let (a, b) = (key_path(0, &k), key_path(1, &k));
    let mut app = with_named_vault(&[&a, &b]);
    load_zebra(&mut app);
    remembered(&mut app);
    nonce_history_on(&mut app, false);
    start(&mut app, 2);
    let s = app.spend().unwrap();
    assert!(s.complete && s.spend.finished.is_some());
    assert!(
        app.outbox.iter().any(|i| i.name.ends_with("-final.txn"))
            && app.outbox.iter().any(|i| i.name.ends_with("-signed.psbt"))
    );
    nonce_history_on(&mut app, true);
    app
}

fn outbox_has_results(app: &Faraday) -> bool {
    app.outbox
        .iter()
        .any(|i| i.name.ends_with("-final.txn") || i.name.ends_with("-signed.psbt"))
}

#[test]
fn files_a_spend_made_before_the_history_held_it_are_taken_back_and_stay_so_when_it_lets_go() {
    let mut app = finished_then_history_holds();
    app.press(Action::ToggleTable);
    assert!(!outbox_has_results(&app), "{:?}", app.outbox.len());
    assert!(!app.spend().unwrap().complete);
    nonce_history_on(&mut app, false);
    app.press(Action::ToggleTable);
    assert!(!app.spend().unwrap().complete);
    assert!(!outbox_has_results(&app), "the refusal stays");
}

#[test]
fn a_spend_press_right_after_the_history_changes_gives_nothing_out() {
    for action in [
        Action::QrSigned,
        Action::QrPart,
        Action::SignedToOutbox,
        Action::TxToOutbox,
        Action::DecodeFinished,
        Action::SpendHex,
    ] {
        let mut app = finished_then_history_holds();
        app.press(action);
        assert!(!outbox_has_results(&app), "{action:?}");
        assert_ne!(app.sheet, Some(Sheet::Qr), "{action:?}");
        assert_ne!(app.screen, Screen::Decode, "{action:?}");
    }
}

#[test]
fn a_decode_opened_from_the_spend_offers_no_hex_and_no_code_once_the_spend_is_held() {
    let k = key(9);
    let a = key_path(0, &k);
    let mut app = with_named_vault(&[&a]);
    load_zebra(&mut app);
    let id = app.vaults().open[0].id;
    nonce_history_held(&mut app, id, &[], 0);
    start(&mut app, 1);
    app.press(Action::DecodeFinished);
    assert_eq!(app.screen, Screen::Decode);
    let text = shown(&mut app);
    assert!(text.contains(EN.decode_show_as_qr), "{text}");
    // The history now holds the nonce.
    let own = nonce_history_pending(&app)[0].1[0];
    let held = NonceEntry {
        msg: [9; 32],
        ..own
    };
    nonce_history_held(&mut app, id, &[held], 0);
    app.press(Action::QrDecoded);
    assert_ne!(app.sheet, Some(Sheet::Qr));
    let text = shown(&mut app);
    assert!(!text.contains(EN.decode_show_as_qr), "{text}");
    assert!(!text.contains(EN.decode_show_the_hex), "{text}");
}

#[test]
fn a_spend_only_opened_that_carries_one_nonce_twice_is_held_and_gives_no_part_code() {
    let mut p = two_inputs_one_nonce(&key(9)).inner().clone();
    // A third input still to sign keeps the transaction from finishing.
    p.unsigned_tx.input.push(faraday_bip::bitcoin::TxIn {
        previous_output: OutPoint::new(faraday_bip::bitcoin::Txid::from_byte_array([7; 32]), 2),
        ..p.unsigned_tx.input[0].clone()
    });
    let mut third = p.inputs[0].clone();
    third.tap_key_sig = None;
    p.inputs.push(third);
    let a = faraday_psbt::Psbt::from(p);
    let mut app = with_inbox(vec![("a.psbt", a.to_bytes())]);
    register_savings(&mut app);
    start(&mut app, 0);
    let s = app.spend().unwrap();
    assert!(!s.complete);
    assert!(s.spend.nonce_hold, "held at once");
    app.press(Action::QrPart);
    assert_ne!(app.sheet, Some(Sheet::Qr));
}
