//! The cards for the signatures already on a transaction, apart from
//! the inspection that gathers them: invalid, unchecked, and made under a
//! sighash other than the one the input asks for.

use alloc::format;
use alloc::vec::Vec;

use super::{InputInfo, Level, SighashKind, Warning, WarningKind, warn};

/// The cards for the signatures already on the transaction: one that is
/// not this transaction's (Danger), one that could not be checked
/// (Caution), and one made under a sighash other than the one its input
/// asks for, or other than `ALL`/`DEFAULT` where the input asks for none
/// (Danger; Blocked when it is a `NONE` sighash, which commits to no
/// output; Caution when it is stricter than asked, committing to all the
/// asked one does and more). A signature is checked under the sighash it
/// states, so a signer that answered with another one passes that check:
/// this is the card that catches it. A lookup per signature.
pub(super) fn signature_cards(
    warnings: &mut Vec<Warning>,
    signatures: &[crate::verify::InputSignatures],
    inputs: &[InputInfo],
) {
    for sig in signatures.iter().flat_map(|i| i.signatures.iter()) {
        match sig.verdict {
            crate::verify::Verdict::Valid => {}
            crate::verify::Verdict::Invalid => warn(
                warnings,
                Level::Danger,
                WarningKind::InvalidSignature {
                    input: sig.input,
                    key: sig.name(),
                },
                format!(
                    "input {}: the signature under {} is not a signature of this transaction",
                    sig.input,
                    sig.name()
                ),
            ),
            crate::verify::Verdict::Unchecked(reason) => warn(
                warnings,
                Level::Caution,
                WarningKind::SignatureUnchecked {
                    input: sig.input,
                    reason,
                },
                format!("input {}: a signature was not checked: {reason}", sig.input),
            ),
        }
        let Some(asked) = inputs
            .iter()
            .find(|i| i.index == sig.input)
            .map(|i| i.sighash)
        else {
            continue;
        };
        if sighash_answers(asked, sig.sighash) {
            continue;
        }
        let (level, text) = if sig.sighash.is_none() {
            (
                Level::Blocked,
                format!(
                    "input {}: the signature under {} signs with {}, not the {asked} asked for: outputs are not committed to, anyone can redirect the funds",
                    sig.input,
                    sig.name(),
                    sig.sighash
                ),
            )
        } else if sighash_stricter(asked, sig.sighash) {
            (
                Level::Caution,
                format!(
                    "input {}: the signature under {} signs with {}, stricter than the {asked} asked for: the transaction cannot be changed as the request allows",
                    sig.input,
                    sig.name(),
                    sig.sighash
                ),
            )
        } else {
            (
                Level::Danger,
                format!(
                    "input {}: the signature under {} signs with {}, not the {asked} asked for",
                    sig.input,
                    sig.name(),
                    sig.sighash
                ),
            )
        };
        warn(
            warnings,
            level,
            WarningKind::SighashMismatch {
                input: sig.input,
                key: sig.name(),
            },
            text,
        );
    }
}

/// Whether a signature made under `signed` answers an input that asks
/// for `asked`: the same sighash, or, for the default, `ALL` or taproot's
/// `DEFAULT`, which commit to the same. A lookup.
fn sighash_answers(asked: SighashKind, signed: SighashKind) -> bool {
    asked == signed || (asked.is_default() && signed.is_default())
}

/// Whether a signature made under `signed` commits to everything one
/// under `asked` would, and more: as many outputs or more (`ALL` over
/// `SINGLE` over `NONE`), and the other inputs where `asked` leaves them
/// out (`ANYONECANPAY`). A lookup.
fn sighash_stricter(asked: SighashKind, signed: SighashKind) -> bool {
    let (Some((asked_outputs, asked_acp)), Some((signed_outputs, signed_acp))) =
        (commitment(asked), commitment(signed))
    else {
        return false;
    };
    signed_outputs >= asked_outputs && (asked_acp || !signed_acp)
}

/// What a sighash commits to: its outputs (2 for every output, 1 for its
/// own, 0 for none) and whether it leaves the other inputs out
/// (`ANYONECANPAY`); `None` for a non-standard value. A lookup.
fn commitment(kind: SighashKind) -> Option<(u8, bool)> {
    let byte = match kind {
        SighashKind::Ecdsa(t) => t.to_u32(),
        SighashKind::Taproot(t) => u32::from(t as u8),
        SighashKind::NonStandard(_) => return None,
    };
    let outputs = match byte & 0x1f {
        0 | 1 => 2,
        3 => 1,
        2 => 0,
        _ => return None,
    };
    Some((outputs, byte & 0x80 != 0))
}
