//! Review stops where a check keeps the person's money safe and has no
//! way past it, and offers a hold only where a decision names a case for
//! one (`docs/decisions/F136.md`): a fee this high is paid only after
//! "Hold to pay this fee"; a key origin that does not give the input's
//! key is refused at Review, as signing refuses it; an output that claims
//! to be change and cannot be verified is drawn as a payment; an input
//! no registered wallet gives is named and left unsigned (F141).

use crate::common::{add_key, hold, offered, texts};
use faraday_core::strings::{EN, fill};
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_psbt::Psbt;

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// The spending wallet and its key loaded, and `psbt` open on Review.
fn reviewing(psbt: &Psbt) -> Faraday {
    reviewing_with(psbt, true)
}

/// The spending wallet loaded, its key too when `key`, and `psbt` open
/// on Review.
fn reviewing_with(psbt: &Psbt, key: bool) -> Faraday {
    let k = kit("spending");
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("spending-wallet.txt"),
                k.descriptor.into_bytes(),
            ),
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    if key {
        add_key(&mut app, "bacon");
    }
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    assert_eq!(app.spend().unwrap().open, Some(step::TRANSACTION));
    app
}

/// Everything the frame draws, as one line: a wrapped line reads whole.
fn screen(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

fn count(app: &mut Faraday, want: &str) -> usize {
    texts(app).iter().filter(|t| *t == want).count()
}

#[test]
fn a_high_fee_is_paid_only_after_its_hold() {
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    // Three million sat less change: the fee is twice what it sends.
    psbt.inner_mut().unsigned_tx.output[1].value -=
        faraday_psbt::bitcoin::Amount::from_sat(3_000_000);
    let mut app = reviewing(&psbt);
    let shown = screen(&mut app);
    assert!(shown.contains(EN.step_fee_hold), "{shown}");
    assert!(shown.contains(EN.step_fee_hold_cases), "{shown}");
    // The warning says the fee and its share of what is sent.
    let i = &app.spend().unwrap().inspection;
    let share = fill(
        EN.step_review_fee_share,
        &[
            &i.fee.to_sat().to_string(),
            &i.fee_share_pct().unwrap().to_string(),
        ],
    );
    assert!(shown.contains(&share), "{shown}");
    assert!(!offered(&mut app, Action::StepNext(step::TRANSACTION)));
    // Continue and Sign before the hold: nothing goes on, nothing signs.
    app.press(Action::StepNext(step::TRANSACTION));
    let s = app.spend().unwrap();
    assert!(!s.done[step::TRANSACTION as usize]);
    assert_eq!(s.error.as_deref(), Some(EN.step_review_hold_first));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
    // Held: Review is done and the key signs.
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    hold(&mut app, Action::ReviewHold);
    let s = app.spend().unwrap();
    assert!(s.done[step::TRANSACTION as usize]);
    assert!(!s.held.is_empty());
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
}

/// While Review stops it, the side panel offers no signing: its line
/// points to Transaction and its button is off; once held, it signs.
#[test]
fn the_side_panel_offers_no_signing_while_review_stops_it() {
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    psbt.inner_mut().unsigned_tx.output[1].value -=
        faraday_psbt::bitcoin::Amount::from_sat(3_000_000);
    let mut app = reviewing(&psbt);
    let stopped = faraday_core::strings::fill(EN.spend_status_stopped, &[EN.app_transaction]);
    assert_eq!(count(&mut app, &stopped), 1);
    assert!(!offered(&mut app, Action::Primary));
    app.press(Action::Primary);
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
    hold(&mut app, Action::ReviewHold);
    assert_eq!(count(&mut app, &stopped), 0);
    assert!(offered(&mut app, Action::Primary));
}

#[test]
fn a_key_origin_that_does_not_give_the_inputs_key_is_refused_at_review() {
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    // The same key named at another index of the account.
    for (_, path) in psbt.inner_mut().inputs[0].bip32_derivation.values_mut() {
        let at: Vec<_> = path.into_iter().copied().collect();
        let (last, rest) = at.split_last().unwrap();
        let other =
            faraday_psbt::bitcoin::bip32::ChildNumber::from_normal_idx(u32::from(*last) + 5)
                .unwrap();
        *path = rest.iter().copied().chain([other]).collect();
    }
    let mut app = reviewing(&psbt);
    assert!(app.spend().unwrap().inspection.refuses());
    let shown = screen(&mut app);
    assert!(shown.contains(EN.step_review_refused), "{shown}");
    assert!(!offered(&mut app, Action::StepNext(step::TRANSACTION)));
    assert!(!offered(&mut app, Action::ReviewHold));
    app.press(Action::StepNext(step::TRANSACTION));
    let s = app.spend().unwrap();
    assert!(!s.done[step::TRANSACTION as usize]);
    assert_eq!(s.error.as_deref(), Some(EN.step_review_refused));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
}

#[test]
fn change_that_cannot_be_verified_is_drawn_as_a_payment() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut app = reviewing(&psbt);
    let (sends, change) = (
        count(&mut app, EN.step_sends),
        count(&mut app, EN.step_change),
    );
    // The change output's origin names a key that is not loaded.
    let mut claimed = psbt.clone();
    for (fp, _) in claimed.inner_mut().outputs[1].bip32_derivation.values_mut() {
        *fp = faraday_psbt::bitcoin::bip32::Fingerprint::from([9, 9, 9, 9]);
    }
    let mut app = reviewing(&claimed);
    assert!(matches!(
        app.spend().unwrap().inspection.outputs[1].kind,
        faraday_psbt::OutputKind::UnverifiedChange { .. }
    ));
    assert_eq!(count(&mut app, EN.step_sends), sends + 1);
    assert_eq!(count(&mut app, EN.step_change), change - 1);
    assert!(!texts(&mut app).iter().any(|t| t.contains("Unverified")));
}

/// The change row names the wallet the change goes to (F254).
#[test]
fn the_change_row_names_its_wallet() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut app = reviewing(&psbt);
    let name = app.session.wallets[0].name.to_string();
    let shown = texts(&mut app);
    assert!(
        shown
            .iter()
            .any(|t| t.starts_with(&name) && t.contains(" · to 1/")),
        "{shown:?}"
    );
}

/// A wallet file of the single-key script `script` over the spending
/// wallet's key at its BIP-84 account, under the name `name`: with a script
/// other than `wpkh`, a wallet of its own beside the account.
fn odd_wallet(id: &'static str, name: &'static str, script: &str) -> testkit::Kit {
    let mut kit = kit("spending");
    kit.id = id;
    kit.name = name;
    kit.descriptor = format!(
        "{script}({}/<0;1>/*)",
        faraday_core::testkit::key(0, "m/84'/1'/0'")
    );
    if script == "sh(wpkh" {
        kit.descriptor.push(')');
    }
    kit
}

/// `psbt` open on Review with the wallets `first` then `second` (when
/// there is one) loaded in that order, and the spending key loaded.
fn reviewing_wallets(psbt: &Psbt, wallets: &[&testkit::Kit]) -> Faraday {
    reviewing_wallets_loaded(psbt, wallets, wallets.len())
}

/// As [`reviewing_wallets`], with only the first `load` of the wallets
/// loaded; the others wait in the Inbox.
fn reviewing_wallets_loaded(psbt: &Psbt, wallets: &[&testkit::Kit], load: usize) -> Faraday {
    let mut files: Vec<(faraday_text::Text, Vec<u8>)> = wallets
        .iter()
        .map(|k| {
            let (n, b) = testkit::wallet_file(k);
            (faraday_text::Text::of(&n), b)
        })
        .collect();
    files.push((faraday_text::Text::of("spend.psbt"), psbt.to_bytes()));
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    for i in 0..load {
        app.press(Action::LoadWallet(i));
        testkit::confirm_wallets(&mut app);
    }
    add_key(&mut app, "bacon");
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    assert_eq!(app.spend().unwrap().open, Some(step::TRANSACTION));
    app
}

/// Of two single-key wallets at one fingerprint and path, the change row
/// names the one that is that account's own script, not the one
/// registered first with another script.
#[test]
fn the_change_row_names_the_wallet_of_the_matching_script() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let odd = odd_wallet("odd", "Odd", "sh(wpkh");
    let main = odd_wallet("main", "Main", "wpkh");
    let mut app = reviewing_wallets(&psbt, &[&odd, &main]);
    let shown = texts(&mut app);
    assert!(shown.iter().any(|t| t == "Main · to 1/0"), "{shown:?}");
    assert!(!shown.iter().any(|t| t == "Odd · to 1/0"), "{shown:?}");
}

/// With only a wallet of another script at the account, the change row
/// names the key and the account's kind.
#[test]
fn the_change_row_falls_back_to_the_key_and_its_account() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let odd = odd_wallet("odd", "Odd", "sh(wpkh");
    let mut app = reviewing_wallets(&psbt, &[&odd]);
    let shown = texts(&mut app);
    assert!(
        shown
            .iter()
            .any(|t| t.contains(EN.explore_native_segwit) && t.ends_with(" · to 1/0")),
        "{shown:?}"
    );
    assert!(!shown.iter().any(|t| t.starts_with("Odd · ")), "{shown:?}");
}

/// An own address on the receive chain reads Receive, not Change.
#[test]
fn an_own_receive_address_reads_receive() {
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    // The output pays the input's own address, `0/0`.
    let own = psbt.inner_mut().inputs[0].witness_utxo.clone().unwrap();
    let derivation = psbt.inner_mut().inputs[0].bip32_derivation.clone();
    psbt.inner_mut().unsigned_tx.output[1].script_pubkey = own.script_pubkey;
    psbt.inner_mut().outputs[1].bip32_derivation = derivation;
    let mut app = reviewing(&psbt);
    let shown = texts(&mut app);
    assert!(shown.iter().any(|t| t == EN.step_receive), "{shown:?}");
}

/// A payment into another of the person's wallets sits under Sends, says
/// "To" that wallet, and carries its whole address.
#[test]
fn a_transfer_names_the_wallet_it_pays() {
    let (spending, savings) = (kit("spending"), kit("savings"));
    let psbt = testkit::unsigned_into(&spending, &savings).unwrap();
    let mut app = reviewing_wallets(&psbt, &[&spending, &savings]);
    let shown = texts(&mut app);
    let to = fill(EN.step_to_wallet, &[savings.name]);
    assert!(
        shown
            .iter()
            .any(|t| *t == format!("{} · {to}", EN.step_sends)),
        "{shown:?}"
    );
    let address = app.spend().unwrap().inspection.outputs[1].address.clone();
    let flat: String = shown
        .join("")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert!(flat.contains(&address), "{shown:?}");
}

/// With no key of the wallet here, Review goes on to collect the
/// signatures made elsewhere: no key here taking part is no refusal.
#[test]
fn with_no_key_here_review_goes_on_to_collect() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut app = reviewing_with(&psbt, false);
    assert!(
        app.spend()
            .unwrap()
            .inspection
            .dangers()
            .contains(&faraday_psbt::WarningKind::NoParticipatingKey)
    );
    let shown = screen(&mut app);
    assert!(!shown.contains(EN.step_review_refused), "{shown}");
    assert!(offered(&mut app, Action::StepNext(step::TRANSACTION)));
    app.press(Action::StepNext(step::TRANSACTION));
    let s = app.spend().unwrap();
    assert!(s.done[step::TRANSACTION as usize]);
    assert_eq!(s.error, None);
}

/// The hold accepts a fee only on Review as it stands: on a transaction
/// nothing signs, or one with no fee to hold for, it does nothing.
#[test]
fn the_fee_hold_accepts_nothing_where_review_has_no_fee_to_hold() {
    let plain = testkit::unsigned(&kit("spending")).unwrap();
    let mut refused = plain.clone();
    for (_, path) in refused.inner_mut().inputs[0].bip32_derivation.values_mut() {
        let at: Vec<_> = path.into_iter().copied().collect();
        let (last, rest) = at.split_last().unwrap();
        let other =
            faraday_psbt::bitcoin::bip32::ChildNumber::from_normal_idx(u32::from(*last) + 5)
                .unwrap();
        *path = rest.iter().copied().chain([other]).collect();
    }
    for psbt in [plain, refused] {
        let mut app = reviewing(&psbt);
        app.press(Action::ReviewHold);
        let s = app.spend().unwrap();
        assert!(s.held.is_empty());
        assert!(!s.done[step::TRANSACTION as usize]);
    }
}

/// An input of the person's key in a taproot tree that no registered
/// wallet gives is left unsigned, and Review names it (F141): it stops
/// nothing, and signing signs the wallet's own input beside it.
#[test]
fn a_taproot_leaf_without_its_wallet_is_named_and_left_unsigned() {
    use faraday_psbt::bitcoin::bip32::{DerivationPath, Fingerprint, Xpub};
    use faraday_psbt::bitcoin::hashes::Hash;
    use faraday_psbt::bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
    use faraday_psbt::bitcoin::taproot::{LeafVersion, TapLeafHash, TaprootBuilder};
    use faraday_psbt::bitcoin::{Amount, OutPoint, ScriptBuf, TxIn, TxOut, Txid, opcodes};
    use std::str::FromStr;

    // Test key 1's key at m/86'/1'/0'/0/1, alone in a leaf `<K> CHECKSIG`
    // under BIP 341's unspendable internal key.
    let key = testkit::key(0, "m/86'/1'/0'/0/1");
    let (origin, xpub) = key[1..].split_once(']').unwrap();
    let fp = Fingerprint::from_str(&origin[..8]).unwrap();
    let secp = Secp256k1::new();
    let x = Xpub::from_str(xpub)
        .unwrap()
        .public_key
        .x_only_public_key()
        .0;
    let leaf = faraday_psbt::bitcoin::script::Builder::new()
        .push_x_only_key(&x)
        .push_opcode(opcodes::all::OP_CHECKSIG)
        .into_script();
    let nums = XOnlyPublicKey::from_str(
        "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0",
    )
    .unwrap();
    let info = TaprootBuilder::new()
        .add_leaf(0, leaf.clone())
        .unwrap()
        .finalize(&secp, nums)
        .unwrap();
    let control = info
        .control_block(&(leaf.clone(), LeafVersion::TapScript))
        .unwrap();
    let leaf_hash = TapLeafHash::from_script(&leaf, LeafVersion::TapScript);
    let path = DerivationPath::from_str(&testkit::on_net("m/86'/1'/0'/0/1")).unwrap();

    // The spending wallet's spend, with the leaf's coins as its second
    // input and their value added to the change.
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    let inner = psbt.inner_mut();
    inner.unsigned_tx.input.push(TxIn {
        previous_output: OutPoint::new(Txid::from_byte_array([7; 32]), 0),
        ..TxIn::default()
    });
    inner.unsigned_tx.output[1].value += Amount::from_sat(100_000);
    let mut input = faraday_psbt::bitcoin::psbt::Input {
        witness_utxo: Some(TxOut {
            value: Amount::from_sat(100_000),
            script_pubkey: ScriptBuf::new_p2tr_tweaked(info.output_key()),
        }),
        tap_internal_key: Some(nums),
        tap_merkle_root: info.merkle_root(),
        ..Default::default()
    };
    input
        .tap_scripts
        .insert(control, (leaf, LeafVersion::TapScript));
    input
        .tap_key_origins
        .insert(x, (vec![leaf_hash], (fp, path)));
    inner.inputs.push(input);

    let mut app = reviewing(&psbt);
    let line = faraday_core::strings::fill(EN.step_input_no_wallet_tree, &["1"]);
    let shown = screen(&mut app);
    assert!(shown.contains(&line), "{shown}");
    assert!(!shown.contains(EN.step_review_refused), "{shown}");
    assert!(offered(&mut app, Action::StepNext(step::TRANSACTION)));
    app.press(Action::StepNext(step::TRANSACTION));
    assert!(app.spend().unwrap().done[step::TRANSACTION as usize]);

    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    let spend = &app.spend().unwrap().spend;
    assert!(!spend.signed_here.is_empty());
    let inputs = &spend.psbt.inner().inputs;
    assert!(!inputs[0].partial_sigs.is_empty());
    assert!(inputs[1].tap_script_sigs.is_empty());
}

/// Loading another wallet that adds no danger leaves Review done and the
/// hold given on it: the person is not asked to review the same dangers
/// again.
#[test]
fn a_wallet_loaded_that_adds_no_danger_leaves_review_done() {
    let psbt = high_fee_psbt();
    let odd = odd_wallet("odd", "Odd", "sh(wpkh");
    let mut app = reviewing_wallets_loaded(&psbt, &[&kit("spending"), &odd], 1);
    hold(&mut app, Action::ReviewHold);
    let before = app.spend().unwrap().held.clone();
    assert!(!before.is_empty());
    app.press(Action::LoadWallet(1));
    let s = app.spend().unwrap();
    assert!(s.done[step::TRANSACTION as usize]);
    assert_eq!(s.held, before);
}

fn high_fee_psbt() -> Psbt {
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    psbt.inner_mut().unsigned_tx.output[1].value -=
        faraday_psbt::bitcoin::Amount::from_sat(3_000_000);
    psbt
}

/// Review was gone through, then another of the person's wallets was loaded
/// and a payment on it turns out to go into that wallet, a warning Review
/// never showed: Review opens again and Sign Here signs nothing.
#[test]
fn a_wallet_loaded_that_adds_a_warning_opens_review_again() {
    let (spending, savings) = (kit("spending"), kit("savings"));
    let psbt = testkit::unsigned_into(&spending, &savings).unwrap();
    let mut app = reviewing_wallets_loaded(&psbt, &[&spending, &savings], 1);
    app.press(Action::StepNext(step::TRANSACTION));
    assert!(app.spend().unwrap().done[step::TRANSACTION as usize]);
    app.press(Action::LoadWallet(1));
    testkit::confirm_wallets(&mut app);
    let s = app.spend().unwrap();
    assert!(
        s.inspection
            .warnings
            .iter()
            .any(|w| w.kind == faraday_psbt::WarningKind::TransferToOwnWallet),
        "{:?}",
        s.inspection.warnings
    );
    assert!(!s.done[step::TRANSACTION as usize], "Review reads done");
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
}

/// Review was gone through with no key of the wallet here, then a key was
/// loaded and takes part, a key Review never showed: Review opens again and
/// Sign Here signs nothing.
#[test]
fn a_key_loaded_that_takes_part_opens_review_again() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut app = reviewing_with(&psbt, false);
    app.press(Action::StepNext(step::TRANSACTION));
    assert!(app.spend().unwrap().done[step::TRANSACTION as usize]);
    add_key(&mut app, "bacon");
    let s = app.spend().unwrap();
    assert!(
        !s.inspection.participating_keys.is_empty(),
        "no key takes part"
    );
    assert!(!s.done[step::TRANSACTION as usize], "Review reads done");
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
}
