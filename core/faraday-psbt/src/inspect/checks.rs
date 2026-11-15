//! The checks on the whole transaction once its inputs and outputs are
//! read: address reuse, the money range, the fee, and the signatures and
//! fields already on it.

use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::{Amount, ScriptBuf, Transaction};

use super::{
    ABSURD_FEE_RATE_SAT_VB, Detail, HIGH_FEE_CAUTION_PCT, HIGH_FEE_DANGER_ABS, HIGH_FEE_DANGER_PCT,
    InputInfo, Level, OutputInfo, Warning, WarningKind, looks_like_message_request, sighash, warn,
};
use crate::Psbt;
use crate::context::Context;

/// An output that pays the same script as an earlier output, or as an
/// input, is a caution.
pub(super) fn address_reuse(
    outputs: &[OutputInfo],
    input_scripts: &[ScriptBuf],
    warnings: &mut Vec<Warning>,
) {
    for (j, b) in outputs.iter().enumerate() {
        if let Some(a) = outputs[..j]
            .iter()
            .find(|a| a.script_pubkey == b.script_pubkey)
        {
            warn(
                warnings,
                Level::Caution,
                WarningKind::AddressReuse,
                Detail::SameAddress {
                    first: a.index,
                    second: j,
                },
            );
        } else if let Some(k) = input_scripts.iter().position(|s| *s == b.script_pubkey) {
            warn(
                warnings,
                Level::Caution,
                WarningKind::AddressReuse,
                Detail::PaysToInput {
                    output: j,
                    input: k,
                },
            );
        }
    }
}

/// BIP-322's `to_sign` (version 0, or an OP_RETURN of 0 as the only
/// output of a spend of output 0 on input 0) is no spend: signing it signs
/// a message whose text is not shown here.
pub(super) fn message_request(psbt: &Psbt, warnings: &mut Vec<Warning>) {
    if psbt.inner().unsigned_tx.version.0 == 0 || looks_like_message_request(psbt) {
        warn(
            warnings,
            Level::Blocked,
            WarningKind::MessageRequest,
            Detail::MessageRequest,
        );
    }
}

/// The money range, as Core checks it: the outputs as CheckTransaction
/// does, the amounts the inputs spend as CheckTxInputs does. A total out
/// of range makes the fee unknown, which is returned as `false`.
pub(super) fn money_range(
    inputs: &[InputInfo],
    tx: &Transaction,
    mut fee_known: bool,
    warnings: &mut Vec<Warning>,
) -> bool {
    if crate::transaction::money_total(inputs.iter().filter_map(|i| i.amount)).is_err() {
        fee_known = false;
        warn(
            warnings,
            Level::Blocked,
            WarningKind::AmountOutOfRange,
            Detail::InputsOverMaxMoney,
        );
    }
    if crate::transaction::money_total(tx.output.iter().map(|o| o.value)).is_err() {
        fee_known = false;
        warn(
            warnings,
            Level::Blocked,
            WarningKind::AmountOutOfRange,
            Detail::OutputsOverMaxMoney,
        );
    }
    fee_known
}

/// What the amounts and the weight give for the fee.
pub(super) struct Fee {
    pub fee: Amount,
    pub fee_rate_sat_vb: Option<f32>,
    pub vsize_estimate: usize,
}

/// The totals the fee is worked out from.
pub(super) struct Totals {
    pub total_in: Amount,
    pub total_out: Amount,
    pub amount_to_others: Amount,
    pub weight: usize,
}

/// The fee, its rate, and the cards for one too high; none of it where
/// an amount is unknown (`fee_known`) or the outputs exceed the inputs.
pub(super) fn fee_checks(t: &Totals, mut fee_known: bool, warnings: &mut Vec<Warning>) -> Fee {
    let vsize_estimate = t.weight.div_ceil(4);
    let mut fee = Amount::ZERO;
    let mut fee_rate_sat_vb = None;
    if fee_known {
        match t.total_in.checked_sub(t.total_out) {
            Some(f) => fee = f,
            None => {
                fee_known = false;
                warn(
                    warnings,
                    Level::Blocked,
                    WarningKind::UtxoMismatch,
                    Detail::OutputsExceedInputs,
                );
            }
        }
    }
    if fee_known {
        let rate = fee.to_sat() as f32 / vsize_estimate as f32;
        fee_rate_sat_vb = Some(rate);
        fee_share_card(fee, t, warnings);
        if fee.to_sat() >= ABSURD_FEE_RATE_SAT_VB.saturating_mul(vsize_estimate as u64) {
            warn(
                warnings,
                Level::Danger,
                WarningKind::AbsurdFeeRate,
                Detail::FeeRate {
                    sat_vb: rate as u64,
                },
            );
        }
    }
    Fee {
        fee,
        fee_rate_sat_vb,
        vsize_estimate,
    }
}

/// The card for a fee that is a large share of what is sent, or large
/// whatever the share.
fn fee_share_card(fee: Amount, t: &Totals, warnings: &mut Vec<Warning>) {
    let base = if t.amount_to_others > Amount::ZERO {
        t.amount_to_others
    } else {
        t.total_out
    };
    if base > Amount::ZERO {
        let pct = fee.to_sat().saturating_mul(100) / base.to_sat();
        let kind = WarningKind::HighFee {
            pct_of_amount: pct.min(u64::from(u32::MAX)) as u32,
        };
        if fee > HIGH_FEE_DANGER_ABS || pct >= HIGH_FEE_DANGER_PCT {
            warn(
                warnings,
                Level::Danger,
                kind,
                Detail::FeeShare {
                    fee: fee.to_sat(),
                    pct,
                },
            );
        } else if pct >= HIGH_FEE_CAUTION_PCT {
            warn(
                warnings,
                Level::Caution,
                kind,
                Detail::FeeShare {
                    fee: fee.to_sat(),
                    pct,
                },
            );
        }
    }
}

/// What is already signed on the transaction.
pub(super) struct Signed {
    pub signatures: Vec<crate::verify::InputSignatures>,
    pub nonce_reuse: Vec<crate::verify::NonceReuse>,
    pub inscriptions: Vec<crate::verify::Inscription>,
}

/// Costs a copy of the PSBT and one signature verification for each
/// signature already on it, and `final_invalid` for each final input.
/// What is already signed here is checked
/// here, so a signature that is not this transaction's, a nonce used
/// twice and an inscription envelope all reach the review as cards.
pub(super) fn signature_checks(
    psbt: &Psbt,
    ctx: &Context,
    inputs: &[InputInfo],
    warnings: &mut Vec<Warning>,
) -> Signed {
    let inner = psbt.inner();
    let signatures = crate::verify::verify_signatures(psbt, ctx);
    let mismatched: Vec<usize> = (0..inner.inputs.len())
        .filter(|i| crate::verify::witness_shape_mismatch(inner, *i))
        .collect();
    sighash::signature_cards(warnings, &signatures, inputs, &mismatched);
    final_field_cards(inner, warnings);
    let nonce_reuse = crate::verify::repeated_nonces(psbt, &signatures);
    for reuse in &nonce_reuse {
        warn(
            warnings,
            Level::Danger,
            WarningKind::NonceReuse {
                key: String::from(&*reuse.name()),
            },
            Detail::NonceReuse {
                key: String::from(&*reuse.name()),
            },
        );
    }
    let inscriptions = crate::verify::inscriptions(psbt);
    for inscription in &inscriptions {
        warn(
            warnings,
            Level::Info,
            WarningKind::Inscription {
                input: inscription.input,
                content_type: inscription
                    .content_type
                    .as_deref()
                    .map(String::from)
                    .unwrap_or_default(),
            },
            Detail::Inscription {
                input: inscription.input,
            },
        );
    }
    Signed {
        signatures,
        nonce_reuse,
        inscriptions,
    }
}

/// Costs a copy of the PSBT (`Pass::new`, made once, at the first input
/// that needs it) and one signature verification for each final input.
/// A danger for each input whose final fields are not the shape of its
/// script, or whose final signature is wrong.
fn final_field_cards(inner: &bitcoin::Psbt, warnings: &mut Vec<Warning>) {
    // Made at the first input that needs it: one copy of the file for the
    // whole loop.
    let mut pass: Option<crate::verify::Pass> = None;
    for input in 0..inner.inputs.len() {
        if crate::verify::witness_shape_mismatch(inner, input) {
            warn(
                warnings,
                Level::Danger,
                WarningKind::WitnessMismatch { input },
                Detail::WitnessMismatch { input },
            );
        } else if crate::verify::witness_mismatch(inner, input)
            && !pass
                .get_or_insert_with(|| crate::verify::Pass::new(inner))
                .final_signature_wrong(inner, input)
        {
            warn(
                warnings,
                Level::Danger,
                WarningKind::FinalFieldsRefused { input },
                Detail::FinalFieldsRefused { input },
            );
        }
    }
}
