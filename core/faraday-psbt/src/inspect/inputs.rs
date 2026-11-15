//! What inspection reads from each input: the amount it spends, the
//! weight it adds, whether it can be signed, whose it is, and the checks
//! on it, one function per job.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use bitcoin::bip32::DerivationPath;
use bitcoin::psbt::Input;
use bitcoin::secp256k1::{Secp256k1, VerifyOnly};
use bitcoin::{Amount, ScriptBuf, Txid, VarInt};
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;

use super::{
    Detail, InputInfo, KeyOrigin, Level, MultisigSummary, MusigInput, MusigRound, MusigSummary,
    OwnWallet, SpendRoute, Warning, WarningKind, account_of, cosigners, derives_to,
    has_musig_field, input_origins, lock_met, mismatch, musig_input, musig_round, own_path_open,
    push_unique, sighash, sighash_kind, signed_through_wallet, spend_route, tap_signable,
    wallet_input, warn,
};
use crate::Psbt;
use crate::classify::{Classified, ScriptKind, classify};
use crate::context::Context;
use crate::reason::Reason;

/// What every part of the inspection reads and none changes.
pub(super) struct Walk<'a> {
    pub c: &'a Compute,
    pub psbt: &'a Psbt,
    pub ctx: &'a Context<'a>,
    pub secp: &'a Secp256k1<VerifyOnly>,
    /// The id the open MuSig2 session, if any, is keyed by.
    pub txid: Txid,
}

impl<'a> Walk<'a> {
    fn inner(&self) -> &'a bitcoin::Psbt {
        self.psbt.inner()
    }
}

/// What the inputs add up to, and every warning given so far.
pub(super) struct Inputs {
    pub warnings: Vec<Warning>,
    pub inputs: Vec<InputInfo>,
    pub participating: Vec<Fingerprint>,
    pub multisig: Option<MultisigSummary>,
    pub musig: Option<MusigSummary>,
    pub kinds: Vec<ScriptKind>,
    pub input_scripts: Vec<ScriptBuf>,
    /// The inputs held for want of a registered wallet, and which keys of
    /// each multisig input a loaded key derives, for the register offer.
    pub unregistered: Vec<usize>,
    pub multisig_ours: BTreeMap<usize, Vec<bool>>,
    /// The person's wallets the inputs spend: an output is their change
    /// only when it is the one wallet all of these are.
    pub spent: Vec<OwnWallet>,
    pub total_in: Amount,
    pub fee_known: bool,
    pub weight: usize,
    has_witness: bool,
}

/// Whose an input is, and what it says about its key.
#[derive(Default)]
struct Ownership {
    is_ours: bool,
    /// The person's wallet this input spends, when it is verified as one.
    wallet: Option<OwnWallet>,
    origin: Option<KeyOrigin>,
    musig: Option<MusigInput>,
}

/// Costs what `read_input` costs for each input. Reads every input of
/// the PSBT.
pub(super) fn read_inputs(w: &Walk) -> Inputs {
    let inner = w.inner();
    let tx = &inner.unsigned_tx;
    let mut st = Inputs {
        warnings: Vec::new(),
        inputs: Vec::with_capacity(inner.inputs.len()),
        participating: Vec::new(),
        multisig: None,
        musig: None,
        kinds: Vec::new(),
        input_scripts: Vec::new(),
        unregistered: Vec::new(),
        multisig_ours: BTreeMap::new(),
        spent: Vec::new(),
        total_in: Amount::ZERO,
        fee_known: true,
        weight: (4 + 4) * 4
            + VarInt::from(tx.input.len()).size() * 4
            + VarInt::from(tx.output.len()).size() * 4,
        has_witness: false,
    };
    for i in 0..inner.inputs.len() {
        read_input(w, &mut st, i);
    }
    if st.has_witness {
        st.weight += 2;
    }
    st
}

/// Costs what the wallet, cosigner, MuSig2 and spend-route reads cost,
/// and what `register::several_keys` costs through `registration_hold`:
/// a miniscript parse and the tree check.
/// Reads input `i`: its amount and weight, whether it can be signed,
/// whose it is, the checks on it, and what it says about itself.
fn read_input(w: &Walk, st: &mut Inputs, i: usize) {
    let inner = w.inner();
    let tx = &inner.unsigned_tx;
    let input = &inner.inputs[i];
    let cl = classify(inner, i);
    classification_cards(&mut st.warnings, &cl, i);
    let amount = input_amount(st, &cl, i);
    let spk: Option<&ScriptBuf> = cl.utxo.as_ref().map(|u| &u.script_pubkey);
    let script = spk.map(ScriptBuf::as_script);
    let txin = &tx.input[i];
    let finalized = input.final_script_sig.is_some() || input.final_script_witness.is_some();

    add_weight(st, input, &cl, finalized);
    signing_support(w, st, input, &cl, finalized, i);

    // The registered wallet whose script this input is, for the kinds
    // a wallet of several keys can be.
    let registered = match cl.kind {
        ScriptKind::Multisig { .. }
        | ScriptKind::Miniscript { .. }
        | ScriptKind::P2trScript
        | ScriptKind::Unknown => wallet_input(w.c, w.ctx, input, script),
        _ => None,
    };
    let owner = match cl.kind {
        ScriptKind::P2pkh | ScriptKind::P2shP2wpkh | ScriptKind::P2wpkh => {
            own_key_hash(w, st, input, spk, i)
        }
        ScriptKind::Multisig { m, n, .. } => own_multisig(
            w,
            st,
            input,
            &cl,
            registered,
            (usize::from(m), usize::from(n)),
            i,
        ),
        ScriptKind::P2trKey if has_musig_field(input) => own_musig(w, st, input, script, i),
        ScriptKind::P2trKey => own_taproot_key(w, st, input, spk, i),
        ScriptKind::Miniscript { .. } | ScriptKind::P2trScript | ScriptKind::Unknown => {
            own_script_wallet(w, st, input, registered)
        }
    };

    record_spent(st, w.ctx, input, &cl, finalized, registered, owner.wallet);

    if !finalized && registered.is_none() {
        registration_hold(w, st, input, &cl, script, i);
    }

    let sighash = sighash_kind(input, cl.kind, script);
    sighash::sighash_card(&mut st.warnings, i, sighash, tx.output.len());

    let signatures = input.partial_sigs.len()
        + usize::from(input.tap_key_sig.is_some())
        + input.tap_script_sigs.len();
    let spend_route = read_route(w, st, input, &cl, i);

    st.inputs.push(InputInfo {
        index: i,
        txid: txin.previous_output.txid,
        vout: txin.previous_output.vout,
        amount,
        script_type: cl.kind,
        is_ours: owner.is_ours,
        origin: owner.origin,
        sighash,
        already_signed_by: signed_by(input),
        signatures,
        finalized,
        sequence: txin.sequence,
        spend_route,
        musig: owner.musig,
    });
}

/// The cards for what the BIP-174 signer checks found wrong with an
/// input, and for an amount nothing commits to.
fn classification_cards(warnings: &mut Vec<Warning>, cl: &Classified, i: usize) {
    for p in &cl.problems {
        warn(
            warnings,
            Level::Blocked,
            WarningKind::UtxoMismatch,
            Detail::InputFault {
                input: i,
                reason: *p,
            },
        );
    }
    if cl.amount_unverified {
        warn(
            warnings,
            Level::Blocked,
            WarningKind::AmountUnverified,
            Detail::AmountUnverified { input: i },
        );
    }
}

/// What input `i` spends, added to the total in; an input that does not
/// say makes the fee unknown and is a danger.
fn input_amount(st: &mut Inputs, cl: &Classified, i: usize) -> Option<Amount> {
    match &cl.utxo {
        Some(u) => {
            st.total_in = st
                .total_in
                .checked_add(u.value)
                .unwrap_or(Amount::MAX_MONEY);
            st.input_scripts.push(u.script_pubkey.clone());
            Some(u.value)
        }
        None => {
            st.fee_known = false;
            warn(
                &mut st.warnings,
                Level::Danger,
                WarningKind::MissingUtxo,
                Detail::MissingUtxo { input: i },
            );
            None
        }
    }
}

/// The weight an input adds to the transaction: what its final fields
/// weigh, or what satisfying it is estimated to.
fn add_weight(st: &mut Inputs, input: &Input, cl: &Classified, finalized: bool) {
    if finalized {
        let ss = input.final_script_sig.as_ref().map_or(0, |s| s.len());
        let wit = input.final_script_witness.as_ref().map_or(0, |w| w.size());
        st.weight += (32 + 4 + 4 + VarInt::from(ss).size() + ss) * 4 + wit;
        st.has_witness |= wit > 0;
    } else {
        st.weight += cl
            .satisfaction_weight
            .unwrap_or_else(|| cl.kind.estimated_weight());
        st.has_witness |= !matches!(cl.kind, ScriptKind::P2pkh);
    }
}

/// Whether this device can sign an input of this kind: the kinds it signs
/// are listed, and one it cannot is a caution, unless it is spent or its
/// output is unknown.
fn signing_support(
    w: &Walk,
    st: &mut Inputs,
    input: &Input,
    cl: &Classified,
    finalized: bool,
    i: usize,
) {
    let spk = cl.utxo.as_ref().map(|u| u.script_pubkey.as_script());
    let signable = cl.kind.is_supported()
        && (cl.kind != ScriptKind::P2trScript || tap_signable(w.secp, input, spk));
    if signable {
        if !st.kinds.contains(&cl.kind) {
            st.kinds.push(cl.kind);
        }
    } else {
        let reason = match cl.kind {
            ScriptKind::P2trScript => Reason::NoTaprootLeafOrKey,
            _ => Reason::UnknownScript,
        };
        if cl.utxo.is_some() && !finalized {
            warn(
                &mut st.warnings,
                Level::Caution,
                WarningKind::UnsupportedInput,
                Detail::CannotSign { input: i, reason },
            );
        }
    }
}

/// A single-key input of a legacy or segwit v0 script: ours when a loaded
/// key, derived at the path stated, is the key and the script the input
/// spends.
fn own_key_hash(
    w: &Walk,
    st: &mut Inputs,
    input: &Input,
    spk: Option<&ScriptBuf>,
    i: usize,
) -> Ownership {
    let mut own = Ownership::default();
    for (pk, (fp, path)) in &input.bip32_derivation {
        let fp = Fingerprint::from(*fp);
        let claim = KeyOrigin {
            fingerprint: fp,
            path: path.clone(),
        };
        let Some(key) = w.ctx.key(fp) else {
            own.origin.get_or_insert(claim);
            continue;
        };
        own.origin = Some(claim);
        match key.derive(w.secp, path) {
            Some(d) if d.key == *pk && Some(&d.script_pubkey) == spk => {
                own.is_ours = true;
                own.wallet = Some(OwnWallet::Account {
                    key: fp,
                    path: account_of(path),
                });
                push_unique(&mut st.participating, fp);
            }
            Some(d) => mismatch(&mut st.warnings, i, fp, path, d.key == *pk),
            None => key_not_under_account(st, i, fp, path),
        }
    }
    own
}

/// A loaded key at a path that is under none of its accounts: it takes
/// part, and the input says so as a caution.
fn key_not_under_account(st: &mut Inputs, i: usize, fp: Fingerprint, path: &DerivationPath) {
    push_unique(&mut st.participating, fp);
    warn(
        &mut st.warnings,
        Level::Caution,
        WarningKind::UnknownDerivation,
        Detail::KeyNotUnderAccount {
            input: i,
            key: fp,
            path: path.clone(),
        },
    );
}

/// A taproot key-path input without MuSig2 fields: ours when a loaded
/// key, derived at the path stated, is the internal key and the script
/// the input spends.
fn own_taproot_key(
    w: &Walk,
    st: &mut Inputs,
    input: &Input,
    spk: Option<&ScriptBuf>,
    i: usize,
) -> Ownership {
    let mut own = Ownership::default();
    for (xonly, (_, (fp, path))) in &input.tap_key_origins {
        let fp = Fingerprint::from(*fp);
        let claim = KeyOrigin {
            fingerprint: fp,
            path: path.clone(),
        };
        let Some(key) = w.ctx.key(fp) else {
            own.origin.get_or_insert(claim);
            continue;
        };
        own.origin = Some(claim);
        match key.derive(w.secp, path) {
            Some(d)
                if d.key.x_only_public_key().0 == *xonly
                    && Some(&d.script_pubkey) == spk
                    && input.tap_internal_key == Some(*xonly) =>
            {
                own.is_ours = true;
                own.wallet = Some(OwnWallet::Account {
                    key: fp,
                    path: account_of(path),
                });
                push_unique(&mut st.participating, fp);
            }
            Some(d) => mismatch(
                &mut st.warnings,
                i,
                fp,
                path,
                d.key.x_only_public_key().0 == *xonly,
            ),
            None => key_not_under_account(st, i, fp, path),
        }
    }
    own
}

/// Costs what `cosigners` and `derives_to` cost. A multisig input: whose
/// each key is, the claims against loaded keys, and the summary of the
/// wallet (shape `m` of `n`) the first such input gives.
fn own_multisig(
    w: &Walk,
    st: &mut Inputs,
    input: &Input,
    cl: &Classified,
    registered: Option<usize>,
    shape: (usize, usize),
    i: usize,
) -> Ownership {
    let (m, n) = shape;
    let mut own = Ownership::default();
    let ms = cl
        .multisig
        .as_ref()
        .expect("multisig kind carries the script");
    let cosigners = cosigners(w.c, w.secp, ms, input, w.ctx);
    let ours = cosigners.iter().any(|c| c.ours);
    st.multisig_ours
        .insert(i, cosigners.iter().map(|c| c.ours).collect());
    for (key, cosigner) in ms.keys.iter().zip(&cosigners) {
        if let Some(fp) = cosigner.fingerprint {
            let path = input.bip32_derivation[&key.inner].1.clone();
            let claim = KeyOrigin {
                fingerprint: fp,
                path,
            };
            if cosigner.ours {
                own.origin = Some(claim);
                push_unique(&mut st.participating, fp);
                continue;
            }
            let path = &claim.path;
            if w.ctx.is_ours(fp) {
                match derives_to(w.c, w.secp, w.ctx, fp, path, &key.inner) {
                    Some(_) => mismatch(&mut st.warnings, i, fp, path, false),
                    None => key_not_under_account(st, i, fp, path),
                }
            }
            own.origin.get_or_insert(claim);
        }
    }
    own.is_ours = ours;
    match &st.multisig {
        None => {
            st.multisig = Some(MultisigSummary {
                m,
                n,
                cosigners,
                ours,
                wallet: registered,
            })
        }
        Some(first) if first.m != m || first.n != n => warn(
            &mut st.warnings,
            Level::Info,
            WarningKind::MixedScriptTypes,
            Detail::MultisigShapes {
                input: i,
                m,
                n,
                first_m: first.m,
                first_n: first.n,
            },
        ),
        Some(_) => {}
    }
    own
}

/// Costs what `musig_input` and `musig_round` cost. A taproot key-path
/// input with MuSig2 fields: the session it establishes, what this pass
/// will do with each participant this device holds, and the summary the
/// first such input gives.
fn own_musig(
    w: &Walk,
    st: &mut Inputs,
    input: &Input,
    script: Option<&bitcoin::Script>,
    i: usize,
) -> Ownership {
    let mut own = Ownership::default();
    match musig_input(w.c, w.secp, w.ctx, input, script) {
        Err(block) => warn(
            &mut st.warnings,
            Level::Blocked,
            block.kind,
            Detail::InputFault {
                input: i,
                reason: block.reason,
            },
        ),
        Ok(None) => {}
        Ok(Some(mut established)) => {
            own.is_ours = !established.ours.is_empty();
            if own.is_ours {
                own.wallet = Some(OwnWallet::Registered(established.wallet));
            }
            if let Some(first) = established.ours.first() {
                own.origin = Some(KeyOrigin {
                    fingerprint: first.fingerprint,
                    path: first.path.clone(),
                });
                for participation in &established.ours {
                    push_unique(&mut st.participating, participation.fingerprint);
                }
            }
            // What this pass will do with each participant
            // this device holds, which is what the hold's
            // label and the caution below are about.
            let rounds: Vec<MusigRound> = established
                .ours
                .iter()
                .map(|p| musig_round(w.c, w.psbt, &established, p, w.ctx.musig_session, w.txid, i))
                .collect();
            established.will_share_nonce =
                !rounds.is_empty() && rounds.iter().all(|r| matches!(r, MusigRound::Share { .. }));
            established.nonce_stale = rounds
                .iter()
                .any(|r| matches!(r, MusigRound::Share { stale: true }));
            if established.nonce_stale {
                warn(
                    &mut st.warnings,
                    Level::Caution,
                    WarningKind::MusigNonceReplaced,
                    Detail::MusigNonceReplaced { input: i },
                );
            }
            if st.musig.is_none() {
                st.musig = Some(MusigSummary {
                    wallet: established.wallet,
                    keys: established.participants.len(),
                    partial_signatures: crate::musig::partial_sigs(input)
                        .map_or(0, |sigs| sigs.len()),
                    ours: established.ours.iter().map(|p| p.fingerprint).collect(),
                });
            }
            own.musig = Some(established);
        }
    }
    own
}

/// A miniscript, a taproot tree or a script not taken apart: a wallet in
/// use answers for its own inputs, and the origin shown is the one of a
/// loaded key where there is one.
fn own_script_wallet(
    w: &Walk,
    st: &mut Inputs,
    input: &Input,
    registered: Option<usize>,
) -> Ownership {
    let origin = input
        .bip32_derivation
        .values()
        .map(|(fp, path)| ((*fp).into(), path))
        .chain(
            input
                .tap_key_origins
                .values()
                .map(|(_, (fp, path))| ((*fp).into(), path)),
        )
        .max_by_key(|(fp, _)| w.ctx.is_ours(*fp))
        .map(|(fingerprint, path)| KeyOrigin {
            fingerprint,
            path: path.clone(),
        });
    let mut own = Ownership {
        origin,
        ..Ownership::default()
    };
    // A miniscript or taproot-tree wallet in use answers for
    // its own inputs: the script in front of us is that
    // wallet's own at the index its key origins name. This
    // is the rule a multisig input is read by, and it is the
    // only thing that says whose these coins are, since the
    // script itself is not one this crate takes apart.
    if registered.is_some() {
        own.is_ours = true;
        for (fp, _) in input_origins(input) {
            if w.ctx.is_ours(fp) {
                push_unique(&mut st.participating, fp);
            }
        }
    }
    own
}

/// Records the person's wallet input `input` spends in `st.spent`: the
/// registered wallet whose script it is, or the account it is verified
/// under, each named one way. An input verified as no wallet of theirs
/// that still names a loaded key and is a single-key script (p2pkh,
/// p2sh-p2wpkh, p2wpkh, p2tr key path) or an unregistered 1-of-1 multi
/// (signed from a loaded key; one of several keys is blocked anyway) is
/// one the signer signs from the loaded key (a key at a path outside the
/// accounts, another script type): it is a wallet of its own, so nothing
/// is change of it. An input the signer leaves unsigned (a tap-script or
/// miniscript no wallet is registered for) is not recorded.
fn record_spent(
    st: &mut Inputs,
    ctx: &Context,
    input: &Input,
    cl: &Classified,
    finalized: bool,
    registered: Option<usize>,
    own: Option<OwnWallet>,
) {
    if let Some(w) = registered
        .map(OwnWallet::Registered)
        .or(own)
        .map(|w| w.named(ctx))
    {
        push_wallet(&mut st.spent, w);
    } else if !finalized
        && matches!(
            cl.kind,
            ScriptKind::P2pkh
                | ScriptKind::P2shP2wpkh
                | ScriptKind::P2wpkh
                | ScriptKind::P2trKey
                | ScriptKind::Multisig { .. }
        )
    {
        let ecdsa = input
            .bip32_derivation
            .values()
            .map(|(fp, path)| (Fingerprint::from(*fp), path));
        let taproot = input
            .tap_key_origins
            .values()
            .map(|(_, (fp, path))| (Fingerprint::from(*fp), path));
        for (fp, path) in ecdsa.chain(taproot) {
            if ctx.is_ours(fp) {
                push_wallet(
                    &mut st.spent,
                    OwnWallet::Account {
                        key: fp,
                        path: account_of(path),
                    },
                );
            }
        }
    }
}

fn push_wallet(spent: &mut Vec<OwnWallet>, w: OwnWallet) {
    if !spent.contains(&w) {
        spent.push(w);
    }
}

/// Costs what `several_keys` costs. A script of several keys is signed
/// only for a registered wallet whose script it is: the coordinator's
/// word for who the cosigners are is not taken. Only an input that names
/// a loaded key is held to it, since no other input is signed here. A
/// miniscript or a taproot tree of one key is not held, and not signed
/// either: it is left unsigned, said so, while the rest of the
/// transaction is signed (F141).
fn registration_hold(
    w: &Walk,
    st: &mut Inputs,
    input: &Input,
    cl: &Classified,
    script: Option<&bitcoin::Script>,
    i: usize,
) {
    let several = crate::register::several_keys(w.c, w.secp, input, cl.kind, script);
    let names_ours = input_origins(input).any(|(fp, _)| w.ctx.is_ours(fp));
    if several && names_ours {
        warn(
            &mut st.warnings,
            Level::Blocked,
            WarningKind::WalletNotRegistered,
            Detail::WalletNotRegistered { input: i },
        );
        st.unregistered.push(i);
    } else if names_ours && signed_through_wallet(cl.kind) {
        let taproot = cl.kind == ScriptKind::P2trScript;
        warn(
            &mut st.warnings,
            Level::Caution,
            WarningKind::NoWalletForScript { input: i, taproot },
            Detail::NoWalletForScript { input: i, taproot },
        );
    }
}

/// The fingerprints of the keys that already signed an input.
fn signed_by(input: &Input) -> Vec<Fingerprint> {
    let mut already_signed_by = Vec::new();
    for pk in input.partial_sigs.keys() {
        if let Some((fp, _)) = input.bip32_derivation.get(&pk.inner) {
            push_unique(&mut already_signed_by, (*fp).into());
        }
    }
    if input.tap_key_sig.is_some()
        && let Some(internal) = input.tap_internal_key
        && let Some((_, (fp, _))) = input.tap_key_origins.get(&internal)
    {
        push_unique(&mut already_signed_by, (*fp).into());
    }
    already_signed_by
}

/// Costs what `spend_route` and `own_path_open` cost. The route a
/// script input is spent by, and a block for each lock on it that is not
/// met. Refused only when no way the person's keys take part in is open:
/// the path read from the PSBT may be the coordinator's guess, its other
/// keys' origins left off (F136).
fn read_route(
    w: &Walk,
    st: &mut Inputs,
    input: &Input,
    cl: &Classified,
    i: usize,
) -> Option<SpendRoute> {
    let tx = &w.inner().unsigned_tx;
    let spend_route = spend_route(w.c, w.secp, w.ctx, input, cl, tx, i);
    if let Some(SpendRoute::Script(path)) = &spend_route
        && path.locks.iter().any(|l| !lock_met(*l, tx, i))
        && !own_path_open(w.c, w.ctx, input, cl, tx, i)
    {
        for lock in &path.locks {
            if !lock_met(*lock, tx, i) {
                warn(
                    &mut st.warnings,
                    Level::Blocked,
                    WarningKind::TimelockNotMet {
                        path: path.clone(),
                        lock: *lock,
                    },
                    Detail::TimelockNotMet {
                        input: i,
                        lock: *lock,
                    },
                );
            }
        }
    }
    spend_route
}
