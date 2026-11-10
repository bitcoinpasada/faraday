//! What a spend's PSBT and session make of it: the one cut of
//! `refresh_spend` between the work that reads them ([`worked_out`]) and the
//! moves that keep the answer on the screen's state ([`SpendState::take`]).

use crate::wallet::{InspectInputs, NonceCheck, Session, Signing, Spend, SpendState};
use faraday_psbt::{Inspection, MusigSessionView, Psbt};

/// A spend read anew: everything that is a function of its PSBT and the
/// session's [`InspectInputs`]. Public data and verdicts, no key.
pub(crate) struct Fresh {
    /// What the session was when the spend was read.
    pub(crate) inputs: InspectInputs,
    /// The PSBT as read, after any aggregation.
    pub(crate) psbt: Psbt,
    pub(crate) inspection: Inspection,
    pub(crate) refused: Option<String>,
    /// Left out when the spend is opened: the Finish card works it out when
    /// it is drawn.
    pub(crate) check: Option<NonceCheck>,
    pub(crate) signing: Signing,
    /// The loaded wallet the transaction spends from.
    pub(crate) wallet: Option<usize>,
    /// Every input can be finalised now.
    pub(crate) complete: bool,
}

/// Costs an inspection, the aggregation of a MuSig2 round, a second
/// inspection when it gained a signature, the nonce check and `Session::nonce_partials`, `Spend::signing`,
/// `wallet_for` and a finalize of a copy; the nonce check only where `nonce`
/// says (a spend opened leaves it to the card that shows it). Reads `spend` and `session` as they
/// are, and finishes the transaction in `spend` when it has the signatures.
pub(crate) fn worked_out(
    c: &faraday_crypto::work::Compute,
    session: &Session,
    spend: &mut Spend,
    view: Option<&MusigSessionView>,
    nonce: bool,
) -> Fresh {
    let inputs = session.inspect_inputs();
    let mut inspection = session.inspect(c, &spend.psbt);
    // Every participant's partial signature is in the file: aggregated
    // whenever the spend is read anew, whichever way it came to be (a file
    // opened before its wallet was registered, a Collect), and read again if
    // it gained one.
    if spend.aggregate_musig(c, &inspection) {
        inspection = session.inspect(c, &spend.psbt);
    }
    let refused = spend.refused();
    // Worked out here, where the file changed, so the first frame after
    // reads it.
    let check = nonce.then(|| {
        let mut check = session.nonce_check(c, &spend.psbt);
        check.partials = session.nonce_partials(c, &spend.psbt, &inspection);
        check
    });
    let signing = spend.signing(c, session, Some(&inspection), view);
    let wallet = session.wallet_for(c, &spend.psbt);
    let complete = spend.finished.is_some() || spend.finish().is_ok();
    Fresh {
        inputs,
        psbt: spend.psbt.clone(),
        inspection,
        refused,
        check,
        signing,
        wallet,
        complete,
    }
}

impl SpendState {
    /// Moves only. Keeps what [`worked_out`] made as the spend's inspection,
    /// the PSBT it is of, the nonce check where it was worked out (not at
    /// opening), the signers, the wallet and whether it is complete.
    pub(crate) fn take(&mut self, f: Fresh) {
        let Fresh {
            inputs,
            psbt,
            inspection,
            refused,
            check,
            signing,
            wallet,
            complete,
        } = f;
        self.inspection = inspection;
        self.keep_fresh(
            (psbt, inputs),
            (refused, check),
            &signing,
            (wallet, complete),
        );
    }

    /// Moves only. A spend opened with what [`worked_out`] made of it: the
    /// state of a spend read anew, its first read done.
    pub(crate) fn opened_with(spend: Spend, f: Fresh) -> Self {
        let Fresh {
            inputs,
            psbt,
            inspection,
            refused,
            check,
            signing,
            wallet,
            complete,
        } = f;
        let mut s = Self::new(spend, inspection, inputs.clone());
        s.keep_fresh(
            (psbt, inputs),
            (refused, check),
            &signing,
            (wallet, complete),
        );
        s
    }

    fn keep_fresh(
        &mut self,
        (psbt, inputs): (Psbt, InspectInputs),
        (refused, check): (Option<String>, Option<NonceCheck>),
        signing: &Signing,
        (wallet, complete): (Option<usize>, bool),
    ) {
        self.refused = refused;
        // `Rc` is not `Send`: it is made here, on the thread that draws.
        // Never compared yet: `Faraday::nonce_spend_sync` compares it.
        *self.nonce_check.borrow_mut() = check.map(|mut check| {
            check.history_gen = u64::MAX;
            (psbt.clone(), std::rc::Rc::new(check))
        });
        self.inspected = Some((psbt, inputs));
        self.keep_signing(signing);
        self.wallet = wallet;
        self.complete = complete;
    }

    /// Moves only. The signers' lists of a [`Signing`].
    pub(crate) fn keep_signing(&mut self, now: &Signing) {
        let raw = |v: &[faraday_bip::keys::Fingerprint]| v.iter().map(|f| f.0).collect::<Vec<_>>();
        self.signers = raw(&now.signers);
        self.to_sign = raw(&now.to_sign);
        self.in_file = raw(&now.in_file);
        self.failed = raw(&now.failed);
    }
}
