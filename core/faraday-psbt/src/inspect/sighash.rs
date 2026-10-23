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
/// output). A signature is checked under the sighash it states, so a
/// signer that answered with another one passes that check: this is the
/// card that catches it. A lookup per signature.
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
