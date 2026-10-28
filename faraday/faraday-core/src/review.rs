//! A spend's Review where a check stops it (F136): why it does not go
//! on, the fee hold that takes it past a holdable danger, and signing
//! with what was held.

use std::borrow::Cow;

use faraday_crypto::work::Compute;
use faraday_psbt::{Detail, Inspection, Level, Psbt, WarningKind as K};

use crate::strings::{EN, fill};
use crate::wallet::{InspectInputs, Session, Signed, SpendState};

/// Why Review does not go on, from the inspection as it is: a
/// transaction nothing signs, said with what to do instead, or a
/// holdable danger not yet held (F136). No key here taking part is no
/// stop: the spend goes on to collect the signatures made elsewhere.
pub(crate) fn review_stop(inspection: &Inspection, held: &[K]) -> Option<&'static str> {
    let stops = |w: &&faraday_psbt::Warning| match w.level {
        Level::Blocked => true,
        Level::Danger => !w.kind.is_holdable() && w.kind != K::NoParticipatingKey,
        _ => false,
    };
    let stopping: Vec<&K> = inspection
        .warnings
        .iter()
        .filter(stops)
        .map(|w| &w.kind)
        .collect();
    let all = |f: fn(&K) -> bool| !stopping.is_empty() && stopping.iter().all(|k| f(k));
    if all(|k| matches!(k, K::TimelockNotMet { .. })) {
        Some(EN.step_review_timelock)
    } else if all(|k| *k == K::WalletNotRegistered) {
        // Its way on is the wallet registered, offered above (F135).
        Some(EN.step_review_register)
    } else if !stopping.is_empty() {
        Some(EN.step_review_refused)
    } else if inspection.holdable().iter().any(|k| !held.contains(k)) {
        Some(EN.step_review_hold_first)
    } else {
        None
    }
}

/// The words Review gives warning `w` of inspection `i`: what it states,
/// worded from its kind and data, an input left unsigned for want of
/// its wallet (F141) in Review's own words.
pub(crate) fn warning_text<'a>(_i: &Inspection, w: &'a faraday_psbt::Warning) -> Cow<'a, str> {
    let d = &w.detail;
    input_words(d)
        .or_else(|| output_words(d))
        .unwrap_or_else(|| other_words(d))
}

/// What a warning about an input states, in the strings file's words.
fn input_words(d: &Detail) -> Option<Cow<'static, str>> {
    use Detail as D;
    let n = |v: usize| v.to_string();
    let t = |template: &str, args: &[&str]| Cow::Owned(fill(template, args));
    Some(match d {
        D::InputFault { input, reason } => {
            t(EN.psbt_error_input, &[&n(*input), reason_text(*reason)])
        }
        D::AmountUnverified { input } => t(EN.psbt_warn_amount_unverified, &[&n(*input)]),
        D::MissingUtxo { input } => t(EN.psbt_warn_missing_utxo, &[&n(*input)]),
        D::CannotSign { input, reason } => t(
            EN.psbt_warn_cannot_sign,
            &[&n(*input), reason_text(*reason)],
        ),
        D::KeyNotUnderAccount { input, key, path } => t(
            EN.psbt_warn_key_not_under_account,
            &[&n(*input), &key.to_string(), &path.to_string()],
        ),
        D::KeyInOtherScript { input, key, path } => t(
            EN.psbt_warn_key_other_script,
            &[&n(*input), &key.to_string(), &path.to_string()],
        ),
        D::KeyNotInput { input, key, path } => t(
            EN.psbt_warn_key_not_input,
            &[&n(*input), &key.to_string(), &path.to_string()],
        ),
        D::MultisigShapes {
            input,
            m,
            n: keys,
            first_m,
            first_n,
        } => t(
            EN.psbt_warn_multisig_shapes,
            &[&n(*input), &n(*m), &n(*keys), &n(*first_m), &n(*first_n)],
        ),
        D::MusigNonceReplaced { input } => t(EN.psbt_warn_musig_nonce_replaced, &[&n(*input)]),
        D::UnreadRecords { input } => t(EN.psbt_warn_unread_records, &[&n(*input)]),
        D::WalletNotRegistered { input } => t(EN.psbt_warn_wallet_not_registered, &[&n(*input)]),
        D::NoWalletForScript { input, taproot } => t(
            if *taproot {
                EN.step_input_no_wallet_tree
            } else {
                EN.step_input_no_wallet_script
            },
            &[&n(*input)],
        ),
        D::TimelockNotMet { input, lock } => {
            t(EN.psbt_warn_timelock, &[&n(*input), &wait_text(*lock)])
        }
        _ => return None,
    })
}

/// What a warning about the outputs or the fee states, in the strings file's words.
fn output_words(d: &Detail) -> Option<Cow<'static, str>> {
    use Detail as D;
    let n = |v: usize| v.to_string();
    let t = |template: &str, args: &[&str]| Cow::Owned(fill(template, args));
    Some(match d {
        D::DataOutput { output } => t(EN.psbt_warn_data_output, &[&n(*output)]),
        D::NonStandardOutput { output } => t(EN.psbt_warn_nonstandard_output, &[&n(*output)]),
        D::Dust { output, sat } => t(EN.psbt_warn_dust, &[&n(*output), &sat.to_string()]),
        D::SameAddress { first, second } => {
            t(EN.psbt_warn_same_address, &[&n(*first), &n(*second)])
        }
        D::PaysToInput { output, input } => {
            t(EN.psbt_warn_pays_to_input, &[&n(*output), &n(*input)])
        }
        D::OutputsExceedInputs => Cow::Borrowed(EN.psbt_warn_outputs_exceed),
        D::FeeShare { fee, pct } => t(
            EN.step_review_fee_share,
            &[&fee.to_string(), &pct.to_string()],
        ),
        D::FeeRate { sat_vb } => t(EN.step_review_fee_rate, &[&sat_vb.to_string()]),
        D::MixedScripts { kinds } => {
            let kinds: Vec<String> = kinds.iter().map(|k| script_kind_text(*k)).collect();
            t(EN.psbt_warn_mixed_scripts, &[&kinds.join(", ")])
        }
        D::Locktime(lt) => t(EN.psbt_warn_locktime, &[&locktime_text(*lt)]),
        D::NoParticipatingKey => Cow::Borrowed(EN.psbt_warn_no_key),
        D::NonceReuse { key } => t(EN.psbt_warn_nonce_reuse, &[key]),
        D::Inscription { input } => t(EN.psbt_warn_inscription, &[&n(*input)]),
        D::MusigChangeSpoof { output } => t(EN.psbt_warn_musig_change_spoof, &[&n(*output)]),
        D::WalletChangeSpoof { output } => t(EN.psbt_warn_wallet_change_spoof, &[&n(*output)]),
        D::ChangeKeyNotLoaded { output, key } => t(
            EN.psbt_warn_change_not_loaded,
            &[&n(*output), &key.to_string()],
        ),
        D::ChangeSpoof { output, key, path } => t(
            EN.psbt_warn_change_spoof,
            &[&n(*output), &key.to_string(), &path.to_string()],
        ),
        D::ChangeNotRegistered { output, key, path } => t(
            EN.psbt_warn_change_not_registered,
            &[&n(*output), &key.to_string(), &path.to_string()],
        ),
        D::ChangeNotUnderAccount { output, key, path } => t(
            EN.psbt_warn_change_not_under_account,
            &[&n(*output), &key.to_string(), &path.to_string()],
        ),
        _ => return None,
    })
}

/// What a warning about networks or signatures states, in the strings file's words.
fn other_words(d: &Detail) -> Cow<'static, str> {
    use Detail as D;
    let n = |v: usize| v.to_string();
    let t = |template: &str, args: &[&str]| Cow::Owned(fill(template, args));
    match d {
        D::KeyNetwork {
            key,
            loaded,
            checked,
        } => t(
            EN.psbt_warn_key_network,
            &[&key.to_string(), loaded.name(), checked.name()],
        ),
        D::XpubNetwork { checked } => t(EN.psbt_warn_xpub_network, &[checked.name()]),
        D::CoinType {
            output,
            index,
            path,
            coin,
            checked,
            expected,
        } => t(
            if *output {
                EN.psbt_warn_coin_output
            } else {
                EN.psbt_warn_coin_input
            },
            &[
                &n(*index),
                &path.to_string(),
                &coin.to_string(),
                checked.name(),
                &expected.to_string(),
            ],
        ),
        D::SighashNone { input, sighash } => t(
            EN.psbt_warn_sighash_none,
            &[&n(*input), &sighash_text(*sighash)],
        ),
        D::SighashNonStandard { input, sighash } => t(
            EN.psbt_warn_sighash_nonstandard,
            &[&n(*input), &sighash_text(*sighash)],
        ),
        D::SingleNoOutput { input } => t(EN.psbt_warn_single_no_output, &[&n(*input), &n(*input)]),
        D::Single { input } => t(EN.psbt_warn_single, &[&n(*input), &n(*input)]),
        D::SighashPartial { input, sighash } => t(
            EN.psbt_warn_sighash_partial,
            &[&n(*input), &sighash_text(*sighash)],
        ),
        D::SignatureInvalid { input, key } => t(EN.psbt_warn_signature_invalid, &[&n(*input), key]),
        D::SignatureUnchecked { input, reason } => t(
            EN.psbt_warn_signature_unchecked,
            &[&n(*input), unchecked_text(*reason)],
        ),
        D::SignedNone {
            input,
            key,
            signed,
            asked,
        } => t(
            EN.psbt_warn_signed_none,
            &[
                &n(*input),
                key,
                &sighash_text(*signed),
                &sighash_text(*asked),
            ],
        ),
        D::SignedStricter {
            input,
            key,
            signed,
            asked,
        } => t(
            EN.psbt_warn_signed_stricter,
            &[
                &n(*input),
                key,
                &sighash_text(*signed),
                &sighash_text(*asked),
            ],
        ),
        D::SignedOther {
            input,
            key,
            signed,
            asked,
        } => t(
            EN.psbt_warn_signed_other,
            &[
                &n(*input),
                key,
                &sighash_text(*signed),
                &sighash_text(*asked),
            ],
        ),
        _ => unreachable!("input_words and output_words take the rest"),
    }
}

/// Why faraday-psbt refused to sign or finish, in the strings file's
/// words: its errors are kinds.
pub(crate) fn sign_error_text(e: &faraday_psbt::Error) -> String {
    use faraday_psbt::Error as E;
    let n = |v: usize| v.to_string();
    match e {
        E::Base64 => EN.psbt_error_base64.to_string(),
        E::NotPsbt => EN.psbt_error_not_psbt.to_string(),
        E::Parse(_) => EN.psbt_error_parse.to_string(),
        E::UnsupportedVersion(v) => fill(EN.psbt_error_version, &[&v.to_string()]),
        E::MissingUtxo { input } => fill(EN.psbt_error_no_utxo, &[&n(*input)]),
        E::Unsupported { input, reason } | E::Finalize { input, reason } => {
            fill(EN.psbt_error_input, &[&n(*input), reason_text(*reason)])
        }
        E::KeyMismatch { input } => fill(EN.psbt_error_key_mismatch, &[&n(*input)]),
        E::NoSuchKey(fp) => fill(EN.psbt_error_no_such_key, &[&fp.to_string()]),
        E::Danger(kinds) => fill(EN.psbt_error_danger, &[&n(kinds.len())]),
        E::NotHoldable(_) => EN.psbt_error_not_holdable.to_string(),
        E::Blocked(kinds) => fill(EN.psbt_error_blocked, &[&n(kinds.len())]),
        E::Sighash { input } => fill(EN.psbt_error_sighash, &[&n(*input)]),
        E::VerifyFailed { input } => fill(EN.psbt_error_verify, &[&n(*input)]),
        E::Nondeterministic { input } => fill(EN.psbt_error_nondeterministic, &[&n(*input)]),
        E::MusigPartialSig { input, participant } => {
            fill(EN.psbt_error_musig_partial, &[&n(*input), participant])
        }
        E::Extract(why) => fill(EN.psbt_error_extract, &[why]),
    }
}

/// Why an input is not signed or finished, in the strings file's words.
pub(crate) fn reason_text(r: faraday_psbt::Reason) -> &'static str {
    use faraday_psbt::Reason as R;
    use faraday_psbt::musig::Error as M;
    match r {
        R::PrevTxNotSpent => EN.psbt_reason_prev_tx_not_spent,
        R::PrevTxNoOutput => EN.psbt_reason_prev_tx_no_output,
        R::UtxosDisagree => EN.psbt_reason_utxos_disagree,
        R::WitnessUtxoForLegacy => EN.psbt_reason_witness_utxo_legacy,
        R::NoRedeemScript => EN.psbt_reason_no_redeem_script,
        R::RedeemScriptMismatch => EN.psbt_reason_redeem_mismatch,
        R::NoWitnessScript => EN.psbt_reason_no_witness_script,
        R::WitnessScriptMismatch => EN.psbt_reason_witness_mismatch,
        R::NoPrevTx => EN.psbt_reason_no_prev_tx,
        R::NoTaprootPathOurs => EN.psbt_reason_no_taproot_path_ours,
        R::NoTaprootLeafOrKey => EN.psbt_reason_no_taproot_leaf_or_key,
        R::UnknownScript => EN.psbt_reason_unknown_script,
        R::NoInternalKey => EN.psbt_reason_no_internal_key,
        R::RedeemScriptTooLong => EN.psbt_reason_redeem_too_long,
        R::Musig(M::Participants) => EN.psbt_reason_musig_participants,
        R::Musig(M::EntryKey) => EN.psbt_reason_musig_entry_key,
        R::Musig(M::Nonce) => EN.psbt_reason_musig_nonce,
        R::Musig(M::Signature) => EN.psbt_reason_musig_signature,
        R::Musig(M::ScriptPath) => EN.psbt_reason_musig_script_path,
        R::MusigMalformed => EN.psbt_reason_musig_malformed,
        R::MusigSighashDefault => EN.psbt_reason_musig_sighash,
        R::MusigNonceNotDrawn => EN.psbt_reason_musig_nonce_not_drawn,
        R::MusigNonceMissing => EN.psbt_reason_musig_nonce_missing,
        R::MusigOtherNonces => EN.psbt_reason_musig_other_nonces,
        R::MusigNonces => EN.psbt_reason_musig_nonces,
        R::MusigNoWallet => EN.psbt_reason_musig_no_wallet,
        R::MusigNoAggregate => EN.psbt_reason_musig_no_aggregate,
        R::MusigNotFieldKey => EN.psbt_reason_musig_not_field_key,
        R::MusigNoInternalKey => EN.psbt_reason_musig_no_internal_key,
        R::MusigNoChainIndex => EN.psbt_reason_musig_no_chain_index,
        R::MusigNotDerived => EN.psbt_reason_musig_not_derived,
        R::MusigInternalNotAggregate => EN.psbt_reason_musig_internal,
        R::MusigNoTweak => EN.psbt_reason_musig_no_tweak,
        R::MusigNotWalletAddress => EN.psbt_reason_musig_not_wallet,
    }
}

/// An inspected output's address, or for a script with no address form
/// its script named in the strings file's words, written into `i` once
/// as it is inspected; `i` back.
pub(crate) fn name_scripts(mut i: Inspection) -> Inspection {
    for o in &mut i.outputs {
        if o.address.is_empty() {
            let hex: String = o
                .script_pubkey
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            o.address = fill(EN.psbt_nonstandard_script, &[&hex]);
        }
    }
    i
}

/// A wait a spend path takes, in the strings file's words.
fn wait_text(lock: faraday_bip::spend::Lock) -> String {
    use faraday_bip::spend::Lock as L;
    match lock {
        L::Blocks(n) => fill(EN.psbt_wait_blocks, &[&n.to_string()]),
        L::Intervals(n) => fill(EN.psbt_wait_intervals, &[&n.to_string()]),
        L::Height(n) => fill(EN.psbt_wait_height, &[&n.to_string()]),
        L::Time(n) => fill(EN.psbt_wait_time, &[&n.to_string()]),
        L::Preimage => EN.psbt_wait_preimage.to_string(),
    }
}

/// A transaction's locktime, in the strings file's words.
fn locktime_text(lt: faraday_psbt::Locktime) -> String {
    use faraday_psbt::LocktimeKind as K;
    match lt.kind {
        K::Time(t) => fill(EN.psbt_locktime_time, &[&t.to_string()]),
        K::Height(h) => fill(EN.psbt_locktime_block, &[&h.to_string()]),
        K::None => fill(EN.psbt_locktime_block, &["0"]),
    }
}

/// A sighash by its name (`SIGHASH_ALL`), or one no algorithm accepts in
/// the strings file's words.
fn sighash_text(s: faraday_psbt::SighashKind) -> String {
    use faraday_psbt::SighashKind as S;
    match s {
        S::Ecdsa(t) => t.to_string(),
        S::Taproot(t) => t.to_string(),
        S::NonStandard(v) => fill(EN.psbt_sighash_nonstandard, &[&format!("0x{v:02x}")]),
    }
}

/// An input's kind of script, by its code name (`p2wpkh`) or in the
/// strings file's words.
fn script_kind_text(k: faraday_psbt::ScriptKind) -> String {
    use faraday_psbt::ScriptKind as K;
    match k {
        K::P2trScript => EN.psbt_kind_tr_script.to_string(),
        K::Miniscript { wrapper } => fill(EN.psbt_kind_miniscript, &[&wrapper.to_string()]),
        K::Multisig { wrapper, m, n } => fill(
            EN.psbt_kind_multisig,
            &[&wrapper.to_string(), &m.to_string(), &n.to_string()],
        ),
        K::Unknown => EN.psbt_kind_unknown.to_string(),
        K::P2pkh | K::P2shP2wpkh | K::P2wpkh | K::P2trKey => k.to_string(),
    }
}

/// Why a signature was not checked, in the strings file's words.
fn unchecked_text(u: faraday_psbt::verify::Unchecked) -> &'static str {
    use faraday_psbt::verify::Unchecked as U;
    match u {
        U::NoPrevout => EN.nonce_unchecked_no_prevout,
        U::UnsupportedScript => EN.nonce_unchecked_script,
        U::UnknownKey => EN.nonce_unchecked_no_key,
    }
}

/// Review's Continue: whether it stops `s`, the reason then said as its
/// error.
pub(crate) fn stopped(s: &mut SpendState) -> bool {
    let Some(why) = review_stop(&s.inspection, &s.held) else {
        return false;
    };
    s.error = Some(why.to_string());
    s.follow = true;
    true
}

/// The fee hold on the open spend `s`'s Review: its holdable dangers are
/// held, only on Review as it is now, of this PSBT with the session's
/// `inputs`, and only where a fee is what stops it. Whether it held.
pub(crate) fn hold(s: Option<&mut SpendState>, inputs: &InspectInputs) -> bool {
    let Some(s) = s else {
        return false;
    };
    if !s.inspected_for(inputs)
        || review_stop(&s.inspection, &[]) != Some(EN.step_review_hold_first)
    {
        return false;
    }
    s.held = s.inspection.holdable();
    true
}

impl Session {
    /// Costs what `sign_with` costs. Signs `psbt`, the open spend `s`'s,
    /// past the holds held on its Review (F136), in the MuSig2 round it
    /// keeps. Its inspection chooses the keys when it is of this PSBT and
    /// session as they are; else signing inspects.
    pub(crate) fn sign_spend(
        &self,
        c: &Compute,
        psbt: &mut Psbt,
        seed: &zeroize::Zeroizing<[u8; 32]>,
        s: &mut SpendState,
    ) -> Result<Signed, String> {
        let inputs = self.inspect_inputs();
        let known = (s.inspected_for(&inputs) && s.spend.psbt == *psbt).then_some(&s.inspection);
        self.sign_with(c, psbt, seed, &mut s.musig, known, &s.held)
    }
}
