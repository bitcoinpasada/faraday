//! The transaction being signed with its screen state, the Wallets tab's area and the
//! spend's steps.

use super::*;

/// One other file's name, unsigned transaction id and place in Files, as the
/// review's other transactions were made from them.
pub(crate) type OtherFrom = (String, faraday_bip::bitcoin::Txid, (bool, usize));

/// A transaction being signed, with its screen's state.
pub struct SpendState {
    /// The spend.
    pub spend: Spend,
    /// The open step, if any.
    pub open: Option<u8>,
    /// Steps closed as done.
    pub done: [bool; 10],
    /// The holdable dangers held on Review (F136), passed to signing.
    pub held: Vec<faraday_psbt::WarningKind>,
    /// The full table is shown.
    pub table: bool,
    /// The last refusal or failure.
    pub error: Option<String>,
    /// The inspection, kept current: worked out again only when what it
    /// is of changed ([`SpendState::inspected`]).
    pub inspection: faraday_psbt::Inspection,
    /// What [`SpendState::inspection`] is of: the PSBT, the session's
    /// network, keys and wallets. `refresh_spend` inspects again only when
    /// one of them changed.
    pub(crate) inspected: Option<(faraday_psbt::Psbt, InspectInputs)>,
    /// Fingerprints with a valid signature.
    pub signers: Vec<[u8; 4]>,
    /// The masters still to sign: named on an input not yet final with no
    /// signature counted there ([`Spend::signing`]).
    pub to_sign: Vec<[u8; 4]>,
    /// The masters whose partial signature is in the file, verifies, and
    /// that this device cannot confirm ([`Signing::in_file`]).
    pub in_file: Vec<[u8; 4]>,
    /// The masters whose partial signature in the file does not verify
    /// ([`Signing::failed`]).
    pub failed: Vec<[u8; 4]>,
    /// Which results went to the Outbox.
    pub out_signed: bool,
    /// The finished transaction went to the Outbox.
    pub out_tx: bool,
    /// The names this spend's files went out under, beside any other
    /// transaction's file of the name: the signed PSBT, the finished
    /// transaction and the part-signed PSBT.
    pub out_names: OutNames,
    /// The finished transaction's hex is shown.
    pub show_hex: bool,
    /// The last nonce check, with the PSBT it was made over: it signs
    /// again to compare, so it runs once per set of signatures, not once
    /// per frame. [`Faraday::refresh_spend`] makes it where the file
    /// changes; a frame that finds the PSBT other than this one makes it.
    /// The keys found used before are carried into every check of the spend
    /// until it closes (F227): a new check never starts without them.
    pub(crate) nonce_check: std::cell::RefCell<Option<(Psbt, std::rc::Rc<NonceCheck>)>>,
    /// The step column's scroll, in design units.
    pub scroll: f32,
    /// The open step changed: scroll it into view on the next frame.
    pub follow: bool,
    /// The loaded wallet the transaction spends from.
    pub wallet: Option<usize>,
    /// The steps this spend shows, in order (`wallet::step`).
    pub steps: Vec<u8>,
    /// Signatures needed.
    pub needed: usize,
    /// Every input can be finalised now.
    pub complete: bool,
    /// A MuSig2 round this device opened: its secret nonce, until round
    /// two. Gone on lock.
    pub musig: Option<faraday_psbt::MusigSession>,
    /// The open step has been set once, at entry (`refresh_spend`): once
    /// true, closing every card (an interactive collapse) leaves `open`
    /// at `None` rather than reopening one.
    pub opened: bool,
    /// The register sheet, while it is open over this spend.
    pub register: Option<crate::register::RegisterAsk>,
    /// [`Spend::refused`] of the PSBT as it is, worked out with the
    /// inspection (`refresh_spend`), read when drawing.
    pub refused: Option<String>,
    /// The other transactions spending what this one spends, with their
    /// fee bump results in words (`docs/COMPARE.md` §4.3), made with the
    /// inspection and read when drawing.
    pub(crate) others: Vec<crate::compare::others::Other>,
    /// What [`SpendState::others`] were made from: each other file's name,
    /// unsigned transaction id and place in Files. `None` until made.
    pub(crate) others_from: Option<Vec<OtherFrom>>,
    /// The inspection was made again since [`SpendState::others`] were.
    pub(crate) others_stale: bool,
    /// The PSBT this device's Review (and Check, where the wallet is
    /// known) was gone through for: what the Outbox waits for. A PSBT that
    /// is any other is not vouched for (`SpendState::review_lapse`).
    reviewed: Option<Psbt>,
    /// [`SpendState::review_owed`] as it was last worked out, for drawing
    /// to read (`SpendState::review_owed_shown`). Worked out by
    /// [`SpendState::settle_owed`] wherever the PSBT, `reviewed`, `steps`,
    /// `wallet` or Check's `done` change; it starts as owed.
    owed: Option<(u8, usize)>,
}

impl SpendState {
    /// Review is gone through for the PSBT as it is, and Check too where
    /// the wallet is known: a spend's files go to the Outbox only then.
    pub(crate) fn reviewed_now(&self) -> bool {
        self.reviewed.as_ref() == Some(&self.spend.psbt)
            && (self.wallet.is_none() || self.done[step::CHECK as usize])
    }

    /// Notes that Review was gone through for the PSBT as it is.
    pub(crate) fn mark_reviewed(&mut self) {
        self.reviewed = Some(self.spend.psbt.clone());
        self.settle_owed();
    }

    /// Review was gone through for `before`, the PSBT before this device
    /// signed it: it vouches for the signed one.
    pub(crate) fn carry_review(&mut self, before: &Psbt) {
        if self.reviewed.as_ref() == Some(before) {
            self.mark_reviewed();
        }
        self.settle_owed();
    }

    /// Review is needed again where the PSBT changed since it was gone
    /// through (a Collect answer); signing here is not held by it, the
    /// Outbox is.
    pub(crate) fn review_lapse(&mut self) {
        if self
            .reviewed
            .as_ref()
            .is_some_and(|p| *p != self.spend.psbt)
        {
            self.reviewed = None;
        }
        self.settle_owed();
    }

    /// Works [`SpendState::review_owed`] out again and keeps it: called
    /// wherever the PSBT, what was reviewed, the steps, the wallet or
    /// Check's `done` change.
    pub(crate) fn settle_owed(&mut self) {
        self.owed = self.review_owed();
    }

    /// [`SpendState::review_owed`] as last settled: a read, for drawing.
    pub(crate) fn review_owed_shown(&self) -> Option<(u8, usize)> {
        self.owed
    }

    /// The first step of Review and Check not done yet, with its place in
    /// the steps shown (counted from 1): `None` once the spend is reviewed.
    pub(crate) fn review_owed(&self) -> Option<(u8, usize)> {
        if self.reviewed_now() {
            return None;
        }
        let owed = |n: u8| {
            (n == step::TRANSACTION && self.reviewed.as_ref() != Some(&self.spend.psbt))
                || (n == step::CHECK && self.wallet.is_some() && !self.done[n as usize])
        };
        let i = self
            .steps
            .iter()
            .position(|&n| owed(n))
            .or_else(|| self.steps.iter().position(|&n| n == step::TRANSACTION))
            .unwrap_or(0);
        Some((
            self.steps.get(i).copied().unwrap_or(step::TRANSACTION),
            i + 1,
        ))
    }

    /// Collect is done and the spend complete: Finish opens, or Review
    /// first where it is owed, also for a PSBT signed elsewhere.
    pub(crate) fn open_finish(&mut self) {
        self.done[step::COLLECT as usize] = true;
        self.follow = true;
        match self.review_owed() {
            None => self.open = Some(step::FINISH),
            Some((owed, place)) => {
                self.open = Some(owed);
                self.error = Some(fill(EN.primary_finish_step, &[&place.to_string()]));
            }
        }
    }

    /// The keys this device has signed with that are not still to sign: the
    /// ones the Sign card names as signed.
    pub(crate) fn signed_keys(&self) -> Vec<Fingerprint> {
        self.spend
            .signed_here
            .iter()
            .filter(|f| !self.to_sign.contains(&f.0))
            .copied()
            .collect()
    }

    /// A spend opened on `spend`, its `inspection` made with the
    /// session's `inputs`. Its open step
    /// is computed in `refresh_spend`, once Check's done state is known:
    /// open at Check, or past it when this wallet is already checked
    /// this power-on.
    pub(crate) fn new(
        spend: Spend,
        inspection: faraday_psbt::Inspection,
        inputs: InspectInputs,
    ) -> Self {
        let inspected = Some((spend.psbt.clone(), inputs));
        let refused = spend.refused();
        SpendState {
            refused,
            spend,
            open: None,
            done: [false; 10],
            held: Vec::new(),
            table: false,
            error: None,
            inspection,
            inspected,
            signers: Vec::new(),
            to_sign: Vec::new(),
            in_file: Vec::new(),
            failed: Vec::new(),
            out_signed: false,
            out_tx: false,
            out_names: OutNames::default(),
            show_hex: false,
            nonce_check: Default::default(),
            scroll: 0.0,
            follow: false,
            wallet: None,
            steps: Vec::new(),
            needed: 1,
            complete: false,
            musig: None,
            opened: false,
            register: None,
            others: Vec::new(),
            others_from: None,
            others_stale: true,
            reviewed: None,
            owed: Some((step::TRANSACTION, 1)),
        }
    }

    /// Whether [`SpendState::inspection`] is of the PSBT as it is, with
    /// the session's `inputs` as they are.
    pub(crate) fn inspected_for(&self, inputs: &InspectInputs) -> bool {
        self.inspected
            .as_ref()
            .is_some_and(|(p, i)| i == inputs && *p == self.spend.psbt)
    }
}

/// The Wallets tab's area: the wallet chosen, the key in no wallet
/// chosen, the new name being typed, the wallet card's scroll and the
/// transaction being signed.
#[derive(Default)]
pub(crate) struct WalletsArea {
    /// The wallet chosen on the Wallets tab.
    wallet: usize,
    /// The key in no wallet chosen on the Wallets tab, by fingerprint,
    /// when several are loaded and none of them in a wallet.
    loose_pick: Option<[u8; 4]>,
    /// The chosen wallet's new name, while it is typed.
    renaming: Option<String>,
    /// Design units scrolled past in the wallet card's body, on a desktop.
    card_offset: f32,
    /// The transaction being signed.
    spend: Option<SpendState>,
}

impl WalletsArea {
    /// The wallet chosen on the Wallets tab.
    pub(crate) fn wallet(&self) -> usize {
        self.wallet
    }

    /// Chooses wallet `i`.
    pub(crate) fn set_wallet(&mut self, i: usize) {
        self.wallet = i;
    }

    /// The key in no wallet chosen, by fingerprint.
    pub(crate) fn loose_pick(&self) -> Option<[u8; 4]> {
        self.loose_pick
    }

    /// Chooses the key in no wallet `pick`.
    pub(crate) fn set_loose_pick(&mut self, pick: Option<[u8; 4]>) {
        self.loose_pick = pick;
    }

    /// The new name being typed, if one is.
    pub(crate) fn renaming(&self) -> Option<&String> {
        self.renaming.as_ref()
    }

    /// A character typed into the new name, up to 40.
    pub(crate) fn rename_type(&mut self, c: char) {
        if let Some(name) = self.renaming.as_mut()
            && name.chars().count() < 40
        {
            name.push(c);
        }
    }

    /// The last character of the new name erased.
    pub(crate) fn rename_backspace(&mut self) {
        if let Some(name) = self.renaming.as_mut() {
            name.pop();
        }
    }

    /// Starts, changes or ends the typing of a new name.
    pub(crate) fn set_renaming(&mut self, name: Option<String>) {
        self.renaming = name;
    }

    /// Design units scrolled past in the wallet card's body.
    pub(crate) fn card_offset(&self) -> f32 {
        self.card_offset
    }

    /// The wallet card's scroll, to move it.
    pub(crate) fn card_offset_mut(&mut self) -> &mut f32 {
        &mut self.card_offset
    }

    /// Scrolls the wallet card's body to `to`.
    pub(crate) fn set_card_offset(&mut self, to: f32) {
        self.card_offset = to;
    }

    /// The transaction being signed, if one is.
    pub(crate) fn spend(&self) -> Option<&SpendState> {
        self.spend.as_ref()
    }

    /// The transaction being signed, to change it.
    pub(crate) fn spend_mut(&mut self) -> Option<&mut SpendState> {
        self.spend.as_mut()
    }

    /// Starts signing `state`.
    pub(crate) fn set_spend(&mut self, state: SpendState) {
        self.spend = Some(state);
    }

    /// Ends the signing.
    pub(crate) fn clear_spend(&mut self) {
        self.spend = None;
    }
}

/// Sign a transaction's steps, plugged into Back and Forward (F104).
pub(crate) struct SpendSteps;

/// The one [`SpendSteps`].
pub(crate) static SPEND_STEPS: SpendSteps = SpendSteps;

impl SpendSteps {
    /// The first step Back may open in `s`: the one after Sign once a key
    /// here has signed, or after Nonces once a round is open here (its
    /// nonces are never made again). Only lookups.
    pub(crate) fn first(s: &SpendState) -> Option<u8> {
        let made = if !s.spend.signed_here.is_empty() {
            step::SIGN
        } else if s.musig.is_some() {
            step::NONCES
        } else {
            return None;
        };
        let at = s.steps.iter().position(|&k| k == made)?;
        s.steps.get(at + 1).copied()
    }
}

impl crate::trail::StepFlow for SpendSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = app.wallets.spend()?;
        let mut steps = crate::trail::Steps::new(s.steps.clone(), s.open);
        steps.first = SpendSteps::first(s);
        Some(steps)
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::Step(k)
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        crate::screens::SPEND_TITLES[usize::from(k)]
    }
}
