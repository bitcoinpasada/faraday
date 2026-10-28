//! An input spent from a script of several keys is signed only for a
//! registered wallet, and a PSBT that carries its cosigners' extended
//! keys (BIP 174's global xpubs) registers that wallet itself, once the
//! wallet built from them gives every input's and change output's
//! script.

use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use faraday_bip::keys::{Fingerprint, MasterKey, Network, ScriptType};
use faraday_crypto::work::Compute;
use faraday_psbt::{
    Aux, Context, Error, KeyRef, Level, Nonce, OutputKind, Psbt, RegisterRefusal, WarningKind,
    Wrapper, inspect, register, sign,
};

use crate::common::builder::{
    Build, bip48_path, multisig_policy, single_sig, sorted_multisig, sorted_multisig_change,
};
use crate::common::{ABANDON, LEGAL, ZOO, master};

const NET: Network = Network::Regtest;

/// What a device holding `masters` answers the register's question
/// with: the extended public key a loaded key gives at a path.
fn yours<'a>(
    masters: &'a [&'a MasterKey],
) -> impl Fn(Fingerprint, &DerivationPath) -> Option<Xpub> + 'a {
    move |fp, path| {
        let c = Compute::unchecked();
        masters
            .iter()
            .find(|m| m.fingerprint() == fp)
            .map(|m| m.derive(&c, path).to_xpub(&c))
    }
}

fn sign_now(
    c: &Compute,
    psbt: &mut Psbt,
    keys: &[&MasterKey],
    ctx: &Context,
    accepted: &[WarningKind],
) -> Result<faraday_psbt::SignResult, Error> {
    let selection: Vec<_> = keys.iter().map(|k| k.fingerprint()).collect();
    sign(
        c,
        psbt,
        keys,
        &selection,
        ctx,
        accepted,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
}

/// The three keys' BIP-48 p2wsh account xpubs, written into the PSBT's
/// global map with their origins, the way Sparrow and Bitcoin Core fill
/// a multisig PSBT in.
fn with_global_xpubs(mut psbt: Psbt, masters: &[&MasterKey]) -> Psbt {
    let c = Compute::unchecked();
    let path = bip48_path(NET);
    for m in masters {
        let xpub = m.derive(&c, &path).to_xpub(&c);
        psbt.inner_mut()
            .xpub
            .insert(xpub, (m.fingerprint().into(), path.clone()));
    }
    psbt
}

/// A 2-of-3 spend of the three test keys, `recipient` paid and change
/// back to the wallet's `…/1/0`.
fn two_of_three() -> (MasterKey, MasterKey, MasterKey, Psbt) {
    let c = Compute::unchecked();
    let (abandon, zoo, legal) = (master(ABANDON, NET), master(ZOO, NET), master(LEGAL, NET));
    let recipient = legal
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 3)
        .unwrap()
        .script_pubkey();
    let psbt = sorted_multisig_change(
        &[&abandon, &zoo, &legal],
        2,
        NET,
        100_000,
        &[(recipient, 60_000)],
        39_000,
    );
    (abandon, zoo, legal, psbt)
}

/// With no wallet registered, the 2-of-3 input is Blocked by the new
/// kind and nothing is signed, whatever is held.
#[test]
fn an_unregistered_two_of_three_input_is_blocked_held_or_not() {
    let c = Compute::unchecked();
    let (abandon, zoo, _, psbt) = two_of_three();
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    let blocked: Vec<_> = insp
        .warnings
        .iter()
        .filter(|w| w.level == Level::Blocked)
        .collect();
    assert_eq!(blocked.len(), 1, "{:?}", insp.warnings);
    assert_eq!(blocked[0].kind, WarningKind::WalletNotRegistered);
    assert_eq!(
        blocked[0].detail,
        faraday_psbt::Detail::WalletNotRegistered { input: 0 }
    );
    for accepted in [Vec::new(), vec![WarningKind::AbsurdFeeRate]] {
        let mut attempt = psbt.clone();
        let r = sign_now(&c, &mut attempt, &[&abandon], &ctx, &accepted);
        assert!(
            matches!(&r, Err(Error::Blocked(kinds)) if kinds == &[WarningKind::WalletNotRegistered]),
            "{r:?}"
        );
        assert_eq!(attempt, psbt, "nothing is written");
    }
    let _ = zoo;
}

/// The same spend with the wallet registered is signed, and its change
/// is the wallet's own.
#[test]
fn the_same_input_signs_once_its_wallet_is_registered() {
    let c = Compute::unchecked();
    let (abandon, zoo, legal, mut psbt) = two_of_three();
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [multisig_policy(&[&abandon, &zoo, &legal], 2, NET)];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    assert!(insp.register.is_none(), "nothing to register");
    let r = sign_now(&c, &mut psbt, &[&abandon], &ctx, &[]).expect("a registered wallet");
    assert_eq!(r.signed_inputs.len(), 1);
}

/// A PSBT carrying the three cosigners' extended keys offers its wallet:
/// 2 of 3, P2WSH, sorted, every cosigner by fingerprint and path, the
/// loaded one marked. Registering it gives the wallet whose addresses
/// are the descriptor's, and the spend then signs with its change
/// verified.
#[test]
fn a_psbt_with_global_xpubs_registers_its_wallet_and_then_signs() {
    let c = Compute::unchecked();
    let (abandon, zoo, legal, psbt) = two_of_three();
    let mut psbt = with_global_xpubs(psbt, &[&abandon, &zoo, &legal]);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert_eq!(insp.blocked(), vec![WarningKind::WalletNotRegistered]);
    let offer = insp.register.as_ref().expect("the PSBT defines its wallet");
    assert_eq!(offer.threshold, 2);
    assert_eq!(offer.cosigners.len(), 3);
    assert_eq!(offer.wrapper, Wrapper::P2wsh);
    assert!(offer.readings[0].sorted);
    let mut fps: Vec<_> = offer.cosigners.iter().map(|k| k.fingerprint).collect();
    fps.sort_by_key(|f| f.0);
    let mut expected = vec![
        abandon.fingerprint(),
        zoo.fingerprint(),
        legal.fingerprint(),
    ];
    expected.sort_by_key(|f| f.0);
    assert_eq!(fps, expected);
    assert!(offer.cosigners.iter().all(|k| k.path == bip48_path(NET)));
    let ours: Vec<_> = offer
        .cosigners
        .iter()
        .filter(|k| k.ours)
        .map(|k| k.fingerprint)
        .collect();
    assert_eq!(ours, vec![abandon.fingerprint()]);

    let policy =
        register(&c, &psbt, offer, 0, &yours(&[&abandon])).expect("the wallet gives every script");
    let reference = multisig_policy(&[&abandon, &zoo, &legal], 2, NET);
    for change in [false, true] {
        for index in 0..4 {
            assert_eq!(
                policy.script_at(&c, change, index).unwrap(),
                reference.script_at(&c, change, index).unwrap(),
                "the registered wallet's address {change}/{index} is the descriptor's"
            );
        }
    }

    let wallets = [policy];
    let ctx = Context {
        wallets: &wallets,
        ..ctx
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    assert_eq!(
        insp.outputs[1].kind,
        OutputKind::WalletChange {
            wallet: 0,
            change: true,
            index: 0
        }
    );
    let r = sign_now(&c, &mut psbt, &[&abandon], &ctx, &[]).expect("now registered");
    assert_eq!(r.signed_inputs.len(), 1);
}

/// Global xpubs whose origins name the input's keys but whose keys do
/// not derive them: the register refuses by naming the input. A change
/// output that claims the wallet and pays elsewhere is refused by name
/// too.
#[test]
fn global_xpubs_that_do_not_give_the_scripts_are_refused_by_name() {
    let c = Compute::unchecked();
    let (abandon, zoo, legal, psbt) = two_of_three();
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };

    // Legal's entry carries the xpub of another account under the
    // origin of the one the input uses.
    let mut wrong = with_global_xpubs(psbt.clone(), &[&abandon, &zoo]);
    let h = |i| ChildNumber::from_hardened_idx(i).unwrap();
    let other: DerivationPath = vec![h(48), h(NET.coin_type()), h(1), h(2)].into();
    wrong.inner_mut().xpub.insert(
        legal.derive(&c, &other).to_xpub(&c),
        (legal.fingerprint().into(), bip48_path(NET)),
    );
    let insp = inspect(&c, &wrong, &ctx);
    let offer = insp.register.as_ref().expect("the origins name every key");
    assert_eq!(
        register(&c, &wrong, offer, 0, &yours(&[&abandon])),
        Err(RegisterRefusal::Input(0))
    );

    // The right keys, and change that claims the wallet but pays to the
    // wallet's next address instead.
    let mut moved = with_global_xpubs(psbt, &[&abandon, &zoo, &legal]);
    let reference = multisig_policy(&[&abandon, &zoo, &legal], 2, NET);
    moved.inner_mut().unsigned_tx.output[1].script_pubkey =
        reference.script_at(&c, true, 1).unwrap();
    let insp = inspect(&c, &moved, &ctx);
    let offer = insp
        .register
        .as_ref()
        .expect("the inputs define the wallet");
    assert_eq!(
        register(&c, &moved, offer, 0, &yours(&[&abandon])),
        Err(RegisterRefusal::Output(1))
    );
}

/// Without global xpubs nothing defines the wallet: no offer, and the
/// spend stays Blocked.
#[test]
fn without_global_xpubs_no_wallet_is_offered() {
    let c = Compute::unchecked();
    let (abandon, _, _, psbt) = two_of_three();
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(insp.has_blocked());
    assert!(insp.register.is_none());

    // Global xpubs for two of the three keys are not enough either.
    let partial = with_global_xpubs(psbt, &[&abandon, &master(ZOO, NET)]);
    let insp = inspect(&c, &partial, &ctx);
    assert!(insp.register.is_none());
}

/// A single-key spend is signed as before, with no wallet registered.
#[test]
fn a_single_key_spend_needs_no_registered_wallet() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let recipient = master(ZOO, NET)
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey();
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((recipient, 90_000));
    let mut psbt = single_sig(&abandon, &b);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    sign_now(&c, &mut psbt, &[&abandon], &ctx, &[]).expect("a single-key spend");
}

/// An input none of whose keys is loaded here is not this device's to
/// sign, so it does not hold the spend up; the device signs nothing in
/// it either way.
#[test]
fn a_several_key_input_naming_no_loaded_key_is_not_held_to_it() {
    let c = Compute::unchecked();
    let (abandon, zoo, legal) = (master(ABANDON, NET), master(ZOO, NET), master(LEGAL, NET));
    let psbt = sorted_multisig(&[&zoo, &legal, &abandon], 2, NET, 100_000, &[]);
    let other = master(
        "bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon",
        NET,
    );
    let keys = [KeyRef::from_master(&c, &other, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(
        !insp.blocked().contains(&WarningKind::WalletNotRegistered),
        "{:?}",
        insp.warnings
    );
}

/// A key not the person's, under the person's fingerprint: every entry
/// of the PSBT that names `fake`'s fingerprint names `as_fp` instead.
fn relabel(psbt: &mut Psbt, fake: Fingerprint, as_fp: Fingerprint) {
    let (from, to): (bitcoin::bip32::Fingerprint, bitcoin::bip32::Fingerprint) =
        (fake.into(), as_fp.into());
    let inner = psbt.inner_mut();
    for (_, (fp, _)) in inner
        .inputs
        .iter_mut()
        .flat_map(|i| i.bip32_derivation.iter_mut())
        .chain(
            inner
                .outputs
                .iter_mut()
                .flat_map(|o| o.bip32_derivation.iter_mut()),
        )
    {
        if *fp == from {
            *fp = to;
        }
    }
    for (fp, _) in inner.xpub.values_mut() {
        if *fp == from {
            *fp = to;
        }
    }
}

/// The 2-of-3 policy over `keys` at BIP-48 account 0, each key written
/// under the fingerprint given beside it.
fn policy_as(keys: &[(&MasterKey, Fingerprint)]) -> faraday_bip::policy::WalletPolicy {
    let c = Compute::unchecked();
    let path = bip48_path(NET);
    let mut text = String::from("wsh(sortedmulti(2,@0/**,@1/**,@2/**))");
    for (m, fp) in keys {
        let xpub = m.derive(&c, &path).to_xpub(&c);
        text.push_str(&format!("\n[{fp}/{path:#}]{xpub}"));
    }
    faraday_bip::policy::WalletPolicy::parse(&c, &text).unwrap()
}

const OTHER: &str =
    "letter advice cage absurd amount doctor acoustic avoid letter advice cage above";

/// A coordinator writes its own key under the person's fingerprint: the
/// wallet gives every script of the spend, but none of its keys is the
/// person's, so it is not registered; and the same wallet loaded some
/// other way vouches for no change and signs nothing.
#[test]
fn a_wallet_none_of_whose_keys_is_yours_is_not_registered_and_counts_no_change() {
    let c = Compute::unchecked();
    let (abandon, zoo, legal) = (master(ABANDON, NET), master(ZOO, NET), master(LEGAL, NET));
    let fake = master(OTHER, NET);
    let recipient = legal
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 3)
        .unwrap()
        .script_pubkey();
    let mut psbt = with_global_xpubs(
        sorted_multisig_change(
            &[&fake, &zoo, &legal],
            2,
            NET,
            100_000,
            &[(recipient, 60_000)],
            39_000,
        ),
        &[&fake, &zoo, &legal],
    );
    relabel(&mut psbt, fake.fingerprint(), abandon.fingerprint());
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    let offer = insp.register.as_ref().expect("the origins name every key");
    // The coordinator's key is under the person's fingerprint, so it is
    // refused as a key that claims to be theirs.
    assert!(matches!(
        register(&c, &psbt, offer, 0, &yours(&[&abandon])),
        Err(RegisterRefusal::NotYourKey(_))
    ));
    // And a person who holds none of its fingerprints is not its owner.
    assert_eq!(
        register(&c, &psbt, offer, 0, &yours(&[])),
        Err(RegisterRefusal::NotYours)
    );

    // Loaded as a wallet file instead: its change is not this device's.
    let wallets = [policy_as(&[
        (&fake, abandon.fingerprint()),
        (&zoo, zoo.fingerprint()),
        (&legal, legal.fingerprint()),
    ])];
    let ctx = Context {
        wallets: &wallets,
        ..ctx
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(
        !insp.outputs[1].is_ours(),
        "a wallet without the person's key vouches for no change"
    );
    assert!(insp.blocked().contains(&WarningKind::WalletNotRegistered));
}

/// Two wallets on one BIP-48 key, as two multisigs on one device are:
/// a spend from the second is signed with both registered.
#[test]
fn a_spend_from_the_second_of_two_wallets_sharing_a_key_signs() {
    let c = Compute::unchecked();
    let (abandon, zoo, legal) = (master(ABANDON, NET), master(ZOO, NET), master(LEGAL, NET));
    let recipient = legal
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 3)
        .unwrap()
        .script_pubkey();
    let mut psbt = sorted_multisig(&[&abandon, &zoo], 2, NET, 100_000, &[(recipient, 99_000)]);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [
        multisig_policy(&[&abandon, &zoo, &legal], 2, NET),
        multisig_policy(&[&abandon, &zoo], 2, NET),
    ];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    assert_eq!(insp.multisig.as_ref().unwrap().wallet, Some(1));
    let r = sign_now(&c, &mut psbt, &[&abandon], &ctx, &[]).expect("the second wallet");
    assert_eq!(r.signed_inputs.len(), 1);
}

/// The three keys in an order where their keys at `0/0` sort as they
/// stand and their keys at `1/index` do not: a `multi` wallet whose first
/// address reads as a `sortedmulti` one.
fn sorted_by_chance() -> (Vec<MasterKey>, u32) {
    let c = Compute::unchecked();
    let secp = bitcoin::secp256k1::Secp256k1::verification_only();
    let at = |m: &MasterKey, change: u32, index: u32| {
        m.derive(&c, &bip48_path(NET))
            .to_xpub(&c)
            .derive_pub(
                &secp,
                &[
                    ChildNumber::from_normal_idx(change).unwrap(),
                    ChildNumber::from_normal_idx(index).unwrap(),
                ],
            )
            .unwrap()
            .public_key
            .serialize()
    };
    let words = [ABANDON, ZOO, LEGAL];
    for p in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let ms: Vec<MasterKey> = p.iter().map(|&i| master(words[i], NET)).collect();
        let sorted = |change, index| {
            let k: Vec<_> = ms.iter().map(|m| at(m, change, index)).collect();
            k.windows(2).all(|w| w[0] <= w[1])
        };
        if !sorted(0, 0) {
            continue;
        }
        if let Some(index) = (0..40).find(|&i| !sorted(1, i)) {
            return (ms, index);
        }
    }
    panic!("no order of the three keys sorts at 0/0 alone");
}

/// A spend of the `multi` wallet over `ms` in that order, from its `0/0`,
/// with change to its `1/index` when given.
fn multi_spend(ms: &[MasterKey], change: Option<u32>) -> (faraday_bip::policy::WalletPolicy, Psbt) {
    use faraday_bip::miniscript::descriptor::{Descriptor, DescriptorPublicKey};
    use faraday_bip::miniscript::psbt::PsbtExt;
    let c = Compute::unchecked();
    let path = bip48_path(NET);
    let keys: Vec<String> = ms
        .iter()
        .map(|m| {
            format!(
                "[{}/{path:#}]{}/<0;1>/*",
                m.fingerprint(),
                m.derive(&c, &path).to_xpub(&c)
            )
        })
        .collect();
    let policy = faraday_bip::policy::WalletPolicy::parse_any(
        &c,
        &format!("wsh(multi(2,{}))", keys.join(",")),
    )
    .unwrap();
    let d: Descriptor<DescriptorPublicKey> = policy.to_descriptor().parse().unwrap();
    let chains = d.into_single_descriptors().unwrap();
    let input = chains[0].clone().at_derivation_index(0).unwrap();
    let funding = crate::common::builder::funding_tx(input.script_pubkey(), 100_000);
    let mut output = vec![bitcoin::TxOut {
        value: bitcoin::Amount::from_sat(60_000),
        script_pubkey: bitcoin::ScriptBuf::new_op_return(b"x"),
    }];
    let back = change.map(|i| chains[1].clone().at_derivation_index(i).unwrap());
    if let Some(b) = &back {
        output.push(bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(39_000),
            script_pubkey: b.script_pubkey(),
        });
    }
    let tx = bitcoin::Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint::new(funding.compute_txid(), 0),
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: bitcoin::Witness::new(),
        }],
        output,
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    psbt.inputs[0].witness_utxo = Some(funding.output[0].clone());
    psbt.inputs[0].non_witness_utxo = Some(funding);
    psbt.update_input_with_descriptor(0, &input).unwrap();
    if let Some(b) = &back {
        psbt.update_output_with_descriptor(1, b).unwrap();
    }
    let refs: Vec<&MasterKey> = ms.iter().collect();
    (policy, with_global_xpubs(Psbt::from(psbt), &refs))
}

/// The registration a spend of `ms` offers, with the key of `ms[0]`
/// loaded.
fn offered(ms: &[MasterKey], psbt: &Psbt) -> faraday_psbt::Registration {
    let c = Compute::unchecked();
    let keys = [KeyRef::from_master(&c, &ms[0], 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    inspect(&c, psbt, &ctx)
        .register
        .expect("the PSBT defines its wallet")
}

/// A `multi` wallet whose keys sort by chance at the address spent, with
/// change at an address where they do not: only the `multi` reading
/// gives the change, so it is the one offered and registered.
#[test]
fn keys_sorted_by_chance_read_as_the_reading_that_gives_the_change() {
    let c = Compute::unchecked();
    let (ms, index) = sorted_by_chance();
    let (policy, psbt) = multi_spend(&ms, Some(index));
    let offer = offered(&ms, &psbt);
    assert_eq!(offer.readings.len(), 1, "{:?}", offer.readings);
    assert!(!offer.readings[0].sorted);
    let refs: Vec<&MasterKey> = ms.iter().collect();
    let registered = register(&c, &psbt, &offer, 0, &yours(&refs)).expect("the multi wallet");
    for i in 0..4 {
        assert_eq!(
            registered.script_at(&c, true, i).unwrap(),
            policy.script_at(&c, true, i).unwrap()
        );
    }
}

/// The same wallet spent with no change: both readings give the spend,
/// so both are offered, `sortedmulti` first, and each registers as
/// itself.
#[test]
fn keys_sorted_by_chance_with_nothing_to_tell_offer_both_readings() {
    let c = Compute::unchecked();
    let (ms, index) = sorted_by_chance();
    let (policy, psbt) = multi_spend(&ms, None);
    let offer = offered(&ms, &psbt);
    assert_eq!(offer.readings.len(), 2);
    assert!(offer.readings[0].sorted && !offer.readings[1].sorted);
    let refs: Vec<&MasterKey> = ms.iter().collect();
    let chosen = register(&c, &psbt, &offer, 1, &yours(&refs)).expect("the multi reading");
    assert_eq!(
        chosen.script_at(&c, true, index).unwrap(),
        policy.script_at(&c, true, index).unwrap()
    );
    let other = register(&c, &psbt, &offer, 0, &yours(&refs)).expect("the sorted reading");
    assert_ne!(
        other.script_at(&c, true, index).unwrap(),
        policy.script_at(&c, true, index).unwrap()
    );
}

/// A key whose origin is the master itself names no account: nothing is
/// offered.
#[test]
fn a_key_at_the_master_is_not_offered() {
    let c = Compute::unchecked();
    let (abandon, zoo, legal, psbt) = two_of_three();
    let mut psbt = with_global_xpubs(psbt, &[&zoo, &legal]);
    let root = DerivationPath::master();
    psbt.inner_mut().xpub.insert(
        abandon.derive(&c, &root).to_xpub(&c),
        (abandon.fingerprint().into(), root),
    );
    let fp: bitcoin::bip32::Fingerprint = abandon.fingerprint().into();
    let leaf: DerivationPath = vec![
        ChildNumber::from_normal_idx(0).unwrap(),
        ChildNumber::from_normal_idx(0).unwrap(),
    ]
    .into();
    for (f, path) in psbt.inner_mut().inputs[0].bip32_derivation.values_mut() {
        if *f == fp {
            *path = leaf.clone();
        }
    }
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(insp.blocked().contains(&WarningKind::WalletNotRegistered));
    assert!(insp.register.is_none());
}

/// BIP-48 p2wsh account `account` on the test network.
fn bip48_account(account: u32) -> DerivationPath {
    let h = |i| ChildNumber::from_hardened_idx(i).unwrap();
    DerivationPath::from(vec![h(48), h(NET.coin_type()), h(account), h(2)])
}

/// A 2-of-3 spend whose PSBT lists the person's real key at account 0
/// and, beside it, a key of someone else's written under the person's
/// fingerprint at account 1: what the person would take for two keys of
/// their own.
fn with_an_impostor() -> (MasterKey, Psbt, bitcoin::secp256k1::PublicKey) {
    let c = Compute::unchecked();
    let (abandon, legal) = (master(ABANDON, NET), master(LEGAL, NET));
    let fake = master(OTHER, NET);
    let mut psbt = with_global_xpubs(
        sorted_multisig(&[&abandon, &fake, &legal], 2, NET, 100_000, &[]),
        &[&abandon, &fake, &legal],
    );
    let leaf = [
        ChildNumber::from_normal_idx(0).unwrap(),
        ChildNumber::from_normal_idx(0).unwrap(),
    ];
    let fake_xpub = fake.derive(&c, &bip48_path(NET)).to_xpub(&c);
    let fake_key = fake_xpub
        .derive_pub(&bitcoin::secp256k1::Secp256k1::verification_only(), &leaf)
        .unwrap()
        .public_key;
    let inner = psbt.inner_mut();
    let as_fp: bitcoin::bip32::Fingerprint = abandon.fingerprint().into();
    inner.inputs[0]
        .bip32_derivation
        .insert(fake_key, (as_fp, bip48_account(1).extend(leaf)));
    inner.xpub.insert(fake_xpub, (as_fp, bip48_account(1)));
    (abandon, psbt, fake_key)
}

/// The keys of the first input's script, in script order.
fn sorted_keys(psbt: &Psbt) -> Vec<bitcoin::secp256k1::PublicKey> {
    let ws = psbt.inner().inputs[0].witness_script.clone().unwrap();
    faraday_psbt::Multisig::parse(&ws)
        .unwrap()
        .keys
        .iter()
        .map(|k| k.inner)
        .collect()
}

/// A key under the person's fingerprint that the person's key does not
/// give is refused by name, though the person's real key is in the
/// wallet too; and loaded some other way, it is never the person's.
#[test]
fn a_key_under_your_fingerprint_that_is_not_yours_is_refused_and_never_yours() {
    let c = Compute::unchecked();
    let (abandon, psbt, fake_key) = with_an_impostor();
    let script_keys = sorted_keys(&psbt);
    let account1 = abandon.derive(&c, &bip48_account(1)).to_xpub(&c);
    let keys = [KeyRef::from_master(&c, &abandon, 0)
        .unwrap()
        .with_extra(vec![(bip48_account(1), account1)])];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    let offer = insp.register.as_ref().expect("the origins name every key");
    let at = offer
        .cosigners
        .iter()
        .position(|k| k.path == bip48_account(1))
        .unwrap();
    assert_eq!(
        register(&c, &psbt, offer, 0, &yours(&[&abandon])),
        Err(RegisterRefusal::NotYourKey(at))
    );
    let ms = insp.multisig.as_ref().unwrap();
    assert_eq!(
        ms.cosigners.iter().filter(|k| k.ours).count(),
        1,
        "only the real key is the person's"
    );

    // The same wallet loaded from a file: it holds nothing, and the
    // impostor is not the person's.
    let text = format!(
        "wsh(sortedmulti(2,@0/**,@1/**,@2/**))\n[{fp}/{p0:#}]{x0}\n[{fp}/{p1:#}]{x1}\n[{lfp}/{p0:#}]{x2}",
        fp = abandon.fingerprint(),
        p0 = bip48_path(NET),
        p1 = bip48_account(1),
        x0 = abandon.derive(&c, &bip48_path(NET)).to_xpub(&c),
        x1 = master(OTHER, NET).derive(&c, &bip48_path(NET)).to_xpub(&c),
        lfp = master(LEGAL, NET).fingerprint(),
        x2 = master(LEGAL, NET).derive(&c, &bip48_path(NET)).to_xpub(&c),
    );
    let wallets = [faraday_bip::policy::WalletPolicy::parse(&c, &text).unwrap()];
    let ctx = Context {
        wallets: &wallets,
        ..ctx
    };
    assert!(!ctx.holds(&wallets[0]));
    let insp = inspect(&c, &psbt, &ctx);
    assert!(insp.blocked().contains(&WarningKind::WalletNotRegistered));
    let ms = insp.multisig.as_ref().unwrap();
    for (key, cosigner) in script_keys.iter().zip(&ms.cosigners) {
        if *key == fake_key {
            assert!(!cosigner.ours, "the impostor is never the person's");
        }
    }
}

/// A key origin of more than sixteen steps is refused, in a PSBT and in
/// a wallet, rather than derived along.
#[test]
fn an_origin_of_more_than_sixteen_steps_is_refused() {
    let c = Compute::unchecked();
    let (abandon, _, _, psbt) = two_of_three();
    let long: DerivationPath = (0..17)
        .map(|i| ChildNumber::from_normal_idx(i).unwrap())
        .collect::<Vec<_>>()
        .into();
    let mut inner = psbt.inner().clone();
    let first = *inner.inputs[0].bip32_derivation.keys().next().unwrap();
    inner.inputs[0]
        .bip32_derivation
        .insert(first, (abandon.fingerprint().into(), long.clone()));
    assert!(matches!(
        Psbt::parse_bytes(&c, &inner.serialize()),
        Err(Error::Parse(_))
    ));
    let sixteen: DerivationPath = (0..16)
        .map(|i| ChildNumber::from_normal_idx(i).unwrap())
        .collect::<Vec<_>>()
        .into();
    inner.inputs[0]
        .bip32_derivation
        .insert(first, (abandon.fingerprint().into(), sixteen));
    assert!(Psbt::parse_bytes(&c, &inner.serialize()).is_ok());

    let xpub = abandon.derive(&c, &long).to_xpub(&c);
    let text = format!("wpkh([{}/{long:#}]{xpub}/<0;1>/*)", abandon.fingerprint());
    assert!(faraday_bip::policy::WalletPolicy::parse_any(&c, &text).is_err());
}

/// A taproot leaf of the person's key under an internal key of someone
/// else's is a wallet of two keys; under BIP 341's unspendable point it
/// is the person's key alone.
#[test]
fn a_taproot_internal_key_counts_unless_it_is_unspendable() {
    use faraday_bip::miniscript::descriptor::{DefiniteDescriptorKey, Descriptor};
    use faraday_bip::miniscript::psbt::PsbtExt;
    let c = Compute::unchecked();
    let (abandon, zoo) = (master(ABANDON, NET), master(ZOO, NET));
    let h = |i| ChildNumber::from_hardened_idx(i).unwrap();
    let n = |i| ChildNumber::from_normal_idx(i).unwrap();
    let path = DerivationPath::from(vec![h(86), h(NET.coin_type()), h(0), n(0), n(0)]);
    let key = |m: &MasterKey| {
        format!(
            "[{}/{path:#}]{}",
            m.fingerprint(),
            crate::common::hex(
                &m.derive(&c, &path)
                    .to_xpub(&c)
                    .public_key
                    .x_only_public_key()
                    .0
                    .serialize()
            )
        )
    };
    let spend = |internal: &str| {
        let d: Descriptor<DefiniteDescriptorKey> = format!("tr({internal},pk({}))", key(&abandon))
            .parse()
            .unwrap();
        let funding = crate::common::builder::funding_tx(d.script_pubkey(), 100_000);
        let tx = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![bitcoin::TxIn {
                previous_output: bitcoin::OutPoint::new(funding.compute_txid(), 0),
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: bitcoin::Witness::new(),
            }],
            output: vec![bitcoin::TxOut {
                value: bitcoin::Amount::from_sat(90_000),
                script_pubkey: bitcoin::ScriptBuf::new_op_return(b"x"),
            }],
        };
        let mut p = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
        p.inputs[0].witness_utxo = Some(funding.output[0].clone());
        p.update_input_with_descriptor(0, &d).unwrap();
        Psbt::from(p)
    };
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let held = |psbt: &Psbt| {
        inspect(&c, psbt, &ctx)
            .blocked()
            .contains(&WarningKind::WalletNotRegistered)
    };
    assert!(
        held(&spend(&key(&zoo))),
        "someone else's internal key is a second key"
    );
    assert!(
        !held(&spend(
            "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0"
        )),
        "the unspendable point is no key"
    );
}
