//! A spend's Review where a check stops it (F136): why it does not go
//! on, the fee hold that takes it past a holdable danger, and signing
//! with what was held.

use faraday_crypto::work::Compute;
use faraday_psbt::{Inspection, Level, Psbt, WarningKind as K};

use crate::strings::EN;
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
