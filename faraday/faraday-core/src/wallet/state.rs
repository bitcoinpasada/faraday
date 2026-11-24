//! The transaction being signed with its screen state, the Wallets tab's area and the
//! spend's steps.

use super::*;
use faraday_text::{Text, text};

/// One other file's name, unsigned transaction id and place in Files, as the
/// review's other transactions were made from them.
pub(crate) type OtherFrom = (Text, faraday_bip::bitcoin::Txid, (bool, usize));

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
    pub error: Option<Text>,
    /// The inspection, kept current: worked out again only when what it
    /// is of changed ([`SpendState::inspected`]).
    pub inspection: faraday_psbt::Inspection,
    /// The wallet each output of the inspection goes to, by name, one per
    /// output in order (`None` where it is no wallet of the person's):
    /// worked out where the spend is refreshed, read when drawing.
    pub own_names: Vec<Option<Text>>,
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
    pub refused: Option<Text>,
    /// The other transactions spending what this one spends, with their
    /// fee bump results in words (`docs/COMPARE.md` §4.3), made with the
    /// inspection and read when drawing.
    pub(crate) others: Vec<crate::decode::check_others::Other>,
    /// What [`SpendState::others`] were made from: each other file's name,
    /// unsigned transaction id and place in Files. `None` until made.
    pub(crate) others_from: Option<Vec<OtherFrom>>,
    /// The inspection was made again since [`SpendState::others`] were.
    pub(crate) others_stale: bool,
    /// The PSBT this device's Review (and Check, where the wallet is
    /// known) was gone through for: what the Outbox waits for. A PSBT that
    /// is any other is not vouched for (`SpendState::review_lapse`), nor is
    /// one whose inspection came out with other warnings or keys
    /// (`SpendState::review_lapse_inspection`).
    reviewed: Option<Psbt>,
    /// What Review showed of the inspection when it was marked done: what
    /// every later inspection is compared with, until Review is done again.
    shown: Option<ReviewShown>,
    /// [`SpendState::review_owed`] as it was last worked out, for drawing
    /// to read (`SpendState::review_owed_shown`). Worked out by
    /// [`SpendState::settle_owed`] wherever the PSBT, `reviewed`, `steps`,
    /// `wallet` or Check's `done` change; it starts as owed.
    owed: Option<(u8, usize)>,
}

/// What Review showed of an inspection (`SpendState::review_shown`).
pub(crate) struct ReviewShown {
    warnings: Vec<(
        faraday_psbt::Level,
        faraday_psbt::WarningKind,
        faraday_psbt::Detail,
    )>,
    keys: Vec<faraday_bip::keys::Fingerprint>,
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
        self.shown = Some(self.review_shown());
        self.settle_owed();
    }

    /// Review was gone through for `before`, the PSBT before this device
    /// signed it: it vouches for the signed one.
    pub(crate) fn carry_review(&mut self, before: &Psbt) {
        // What Review showed stays the baseline: this device's own
        // signature adds no warning, and the inspection made after it is
        // compared with that.
        if self.reviewed.as_ref() == Some(before) {
            self.reviewed = Some(self.spend.psbt.clone());
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

    /// What Review shows of the inspection, as it is now: every warning
    /// (its level, kind and detail, so the output or input it is about) but
    /// a MuSig2 nonce replaced, and the keys that take part. Valid
    /// signatures and nonces a Collect adds are in none of it; a signature
    /// that fails, cannot be checked or uses another sighash brings its
    /// warning.
    pub(crate) fn review_shown(&self) -> ReviewShown {
        ReviewShown {
            warnings: self
                .inspection
                .warnings
                .iter()
                // A round's own nonce moving (a mend, a Collect of the
                // others' nonces) is not something Review showed or held.
                .filter(|w| w.kind != faraday_psbt::WarningKind::MusigNonceReplaced)
                .map(|w| (w.level, w.kind.clone(), w.detail.clone()))
                .collect(),
            keys: self.inspection.participating_keys.clone(),
        }
    }

    /// Review is needed again where the inspection was made again and what
    /// it shows is not what Review showed when it was passed: a warning
    /// (counted, with its detail) or a key that was never shown on Review,
    /// and the hold given there does not cover it. Review's step is not
    /// done, nothing is held and the PSBT is no longer reviewed. The same
    /// warnings (as multisets) and keys leave Review as it was.
    pub(crate) fn review_lapse_inspection(&mut self) {
        let Some(before) = self.shown.as_ref() else {
            return;
        };
        let now = self.review_shown();
        fn same<T: PartialEq>(a: &[T], b: &[T]) -> bool {
            let mut rest: Vec<&T> = b.iter().collect();
            a.len() == b.len()
                && a.iter().all(|k| {
                    rest.iter()
                        .position(|r| *r == k)
                        .map(|i| rest.swap_remove(i))
                        .is_some()
                })
        }
        let keys = |v: &[faraday_bip::keys::Fingerprint]| {
            v.iter().all(|k| before.keys.contains(k)) && before.keys.iter().all(|k| v.contains(k))
        };
        if same(&now.warnings, &before.warnings) && keys(&now.keys) {
            return;
        }
        self.shown = None;
        self.reviewed = None;
        self.done[step::TRANSACTION as usize] = false;
        self.held.clear();
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
                self.error = Some(Text::fill(
                    EN.primary_finish_step,
                    &[&Text::num(place as u64)],
                ));
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

    /// Whether the file now carries a verified signature of the master
    /// `fp` on every open input (for MuSig2, a partial this device wrote or
    /// the aggregate): the one answer for "signed", read by Collect's marks and by
    /// Your choices' key rows. It follows the PSBT as last read, so a
    /// signature taken back or a PSBT replaced stops it at once.
    pub(crate) fn key_signed(&self, fp: [u8; 4]) -> bool {
        self.signers.contains(&fp)
    }

    /// Whether the key `r` signs here, in the words "Signs here": the spend
    /// is not complete and no warning stops Sign (it refuses the whole PSBT
    /// then; a fee not yet held does not, a hold clears it), the key is one the spend uses
    /// (`participating_keys`), has inputs it can sign (`to_sign`) and no
    /// key the spend uses owes its backup (`Session::waiting`), and the key
    /// has not signed here (`signed_here`). Lookups only.
    pub(crate) fn signs_here(&self, app: &Faraday, r: [u8; 4]) -> bool {
        let waiting = app.session.waiting();
        let keys = &self.inspection.participating_keys;
        !self.complete
            && !crate::review::sign_is_stopped(&self.inspection)
            && self.to_sign.contains(&r)
            && !self.spend.signed_here.iter().any(|f| f.0 == r)
            && keys.iter().any(|k| k.0 == r)
            && !keys.iter().any(|k| waiting.iter().any(|w| w.0 == k.0))
    }

    /// The rows of Your choices for the keys at Signers: one per
    /// fingerprint of the wallet's keys, or, where no wallet is known, one
    /// per loaded key the PSBT names (`participating_keys` holds only
    /// those; a cosigner this device does not hold is not listed). A key
    /// reads "Signed" while its signature is in the file
    /// ([`SpendState::key_signed`]), and "Signs here" only where
    /// [`SpendState::signs_here`] says so and the key is loaded here; any
    /// other key has no mark. The mark follows the inspection and
    /// `to_sign`, not what the signer does: a refusal or skip found only
    /// while signing (a key mismatch with a false pubkey, an unsupported
    /// input, a leaf not carried, amounts refused, a key's own input
    /// already final) is not seen, so a key can read "Signs here" and
    /// then sign nothing; after signing here, `signed_here` removes the
    /// mark.
    /// Lookups only.
    pub(crate) fn key_rows(&self, app: &Faraday, opens: u8) -> Vec<crate::flow::Row> {
        let keys: Vec<(Option<[u8; 4]>, Option<Text>, bool)> =
            match self.wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => app
                    .session
                    .slots(wl)
                    .iter()
                    .map(|sl| {
                        (
                            sl.fingerprint.map(|f| f.0),
                            sl.held_by.clone(),
                            sl.held_by.is_some(),
                        )
                    })
                    .collect(),
                None => self
                    .inspection
                    .participating_keys
                    .iter()
                    .map(|fp| {
                        (
                            Some(fp.0),
                            app.session.keys.key_label(*fp).map(Text::of),
                            true,
                        )
                    })
                    .collect(),
            };
        let mut seen: Vec<[u8; 4]> = Vec::new();
        let mut rows = Vec::new();
        for (raw, name, here) in keys {
            if let Some(r) = raw {
                if seen.contains(&r) {
                    continue;
                }
                seen.push(r);
            }
            let fp = raw
                .map(|r| crate::wallet::fp_label(faraday_bip::keys::Fingerprint(r)))
                .unwrap_or_default();
            // A name that could be read as a mark is left out.
            let name = name.filter(|n| !name_reads_as_mark(n.as_str()));
            let head = match name
                .as_ref()
                .map(|n| n.trim())
                .filter(|n| !n.is_empty() && !n.eq_ignore_ascii_case(fp.as_str()))
            {
                Some(n) => text!("{} · {}", fp.as_str(), n),
                None => fp.clone(),
            };
            let mark = match raw {
                Some(r) if self.key_signed(r) => Some(EN.step_choice_signed),
                Some(r) if here && self.signs_here(app, r) => Some(EN.step_signs_here),
                _ => None,
            };
            rows.push(crate::flow::Row {
                label: Text::of(EN.step_choice_key),
                value: Some(head),
                mono: false,
                opens,
                current: false,
                mark,
            });
        }
        rows
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
            own_names: Vec::new(),
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
            shown: None,
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
    /// Set when a wallet was taken out of the list since the last frame
    /// ([`Faraday::remove_wallet`]).
    list_changed: bool,
    /// The chosen wallet's new name, while it is typed.
    renaming: Option<Text>,
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

    /// Marks that a wallet was taken out of the list during this event, so
    /// a press drawn on the list before it is dropped.
    pub(crate) fn set_list_changed(&mut self) {
        self.list_changed = true;
    }

    /// Whether a wallet was taken out since the last frame.
    pub(crate) fn list_changed(&self) -> bool {
        self.list_changed
    }

    /// Clears the mark, as a frame is drawn: presses from then on are
    /// drawn on the list as it is.
    pub(crate) fn clear_list_changed(&mut self) {
        self.list_changed = false;
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
    pub(crate) fn renaming(&self) -> Option<&Text> {
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
    pub(crate) fn set_renaming(&mut self, name: Option<Text>) {
        self.renaming = name;
    }

    /// Design units scrolled past in the wallet card's body.
    pub(crate) fn card_offset(&self) -> f32 {
        self.card_offset
    }

    /// Scrolls the wallet card's body to `to`.
    pub(crate) fn set_card_offset(&mut self, to: f32) {
        self.card_offset = to;
    }

    /// The transaction being signed, if one is.
    pub(crate) fn spend(&self) -> Option<&SpendState> {
        self.spend.as_ref()
    }

    /// Scrolls the open spend's step column to `to`, and follows the open
    /// step or not.
    pub(crate) fn scroll_spend_to(&mut self, to: f32, follow: bool) {
        if let Some(s) = self.spend.as_mut() {
            s.scroll = to;
            s.follow = follow;
        }
    }

    /// Scrolls the open spend's step column down by `by`; the page keeps
    /// it inside its end.
    pub(crate) fn scroll_spend_by(&mut self, by: f32) {
        if let Some(s) = self.spend.as_mut() {
            s.scroll += by;
            s.follow = false;
        }
    }

    /// Opens step `n` on the open spend, without scrolling to it.
    pub(crate) fn open_step(&mut self, n: u8) {
        if let Some(s) = self.spend.as_mut() {
            s.open = Some(n);
        }
    }

    /// Opens step `n` on the open spend and scrolls it into view.
    pub(crate) fn open_step_followed(&mut self, n: u8) {
        if let Some(s) = self.spend.as_mut() {
            s.open = Some(n);
            s.follow = true;
        }
    }

    /// Closes step `n` as done on the open spend.
    pub(crate) fn mark_step_done(&mut self, n: u8) {
        if let Some(s) = self.spend.as_mut() {
            s.done[n as usize] = true;
        }
    }

    /// Opens the first step of the open spend not done yet.
    pub(crate) fn open_next_undone(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            s.open = s.steps.iter().copied().find(|&n| !s.done[n as usize]);
        }
    }

    /// The open spend's error is `error`.
    pub(crate) fn set_spend_error(&mut self, error: Text) {
        if let Some(s) = self.spend.as_mut() {
            s.error = Some(error);
        }
    }

    /// A step was pressed on the open spend: it opens, or closes where it
    /// is the open one (not where `keep_open`: the Spend tab's page, where
    /// a step opened from another card stays open, and Sign a transaction,
    /// where a step is always open), and the error is cleared.
    pub(crate) fn press_step(&mut self, n: u8, keep_open: bool) {
        if let Some(s) = self.spend.as_mut() {
            s.open = if s.open == Some(n) && !keep_open {
                None
            } else {
                Some(n)
            };
            s.error = None;
            s.follow = true;
        }
    }

    /// Continue on step `n` of the open spend: it is done, Review's
    /// continue marks the PSBT reviewed, and the next step not done opens.
    /// False where Review's continue is stopped (its reason is the error).
    pub(crate) fn continue_step(&mut self, n: u8) -> bool {
        let Some(s) = self.spend.as_mut() else {
            return true;
        };
        if n == step::TRANSACTION && crate::review::stopped(s) {
            return false;
        }
        s.done[n as usize] = true;
        if n == step::TRANSACTION {
            s.mark_reviewed();
        }
        s.settle_owed();
        s.open = s.steps.iter().copied().find(|&i| !s.done[i as usize]);
        s.error = None;
        s.follow = true;
        true
    }

    /// Holds the open spend's holdable dangers on Review where a fee is
    /// what stops it, with the session's `inputs`. Whether it held.
    pub(crate) fn hold_review(&mut self, inputs: &InspectInputs) -> bool {
        crate::review::hold(self.spend.as_mut(), inputs)
    }

    /// Shows or hides the open spend's full table.
    pub(crate) fn toggle_table(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            s.table = !s.table;
        }
    }

    /// Shows or hides the open spend's finished transaction's hex.
    pub(crate) fn toggle_show_hex(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            s.show_hex = !s.show_hex;
        }
    }

    /// Opens Finish on the open spend (Collect done), or Review first where
    /// it is owed.
    pub(crate) fn open_finish(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            s.open_finish();
        }
    }

    /// Marks the steps before Sign done on the open spend, as a person who
    /// has been through them has, and its Review gone through.
    #[cfg(feature = "testkit")]
    pub(crate) fn mark_steps_before_sign_done(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            for n in s.steps.clone() {
                if n == step::SIGN {
                    break;
                }
                s.done[n as usize] = true;
            }
            s.mark_reviewed();
            // The card shown is Sign's, where the person has come to.
            s.open = Some(step::SIGN);
        }
    }

    /// Where a step before the one a press needs (never past Sign) is not
    /// done, opens it with the error that says so. Whether it did.
    pub(crate) fn stop_at_undone_step(&mut self) -> bool {
        let Some(s) = self.spend.as_mut() else {
            return false;
        };
        let last = if s.musig.is_none() && crate::spend_opens_round(&s.inspection) {
            step::NONCES
        } else {
            step::SIGN
        };
        // A file's round fields never stretch this past Sign.
        let before: Vec<u8> = s
            .steps
            .iter()
            .copied()
            .take_while(|&n| n != last && n != step::SIGN)
            .collect();
        if let Some(pos) = before.iter().position(|&n| !s.done[n as usize]) {
            s.open = Some(before[pos]);
            s.follow = true;
            s.error = Some(Text::fill(
                EN.primary_finish_step,
                &[&Text::num((pos + 1) as u64)],
            ));
            return true;
        }
        false
    }

    /// Opens Signers with `error`, scrolled into view.
    pub(crate) fn signers_shown_with(&mut self, error: Text) {
        if let Some(s) = self.spend.as_mut() {
            s.error = Some(error);
            s.open = Some(step::SIGNERS);
            s.follow = true;
        }
    }

    /// Costs what [`Session::sign_spend_job`] costs. The job that signs
    /// the open spend, with its PSBT as it was before; `None` with no spend.
    pub(crate) fn sign_job(
        &mut self,
        session: &Session,
        c: &Compute,
        seed: Option<&zeroize::Zeroizing<[u8; 32]>>,
    ) -> Option<(Result<faraday_keys::job::SignJob, Text>, Vec<u8>)> {
        let s = self.spend.as_mut()?;
        let psbt = s.spend.psbt.clone();
        let before = s.spend.psbt.to_bytes();
        Some((session.sign_spend_job(c, &psbt, seed, s), before))
    }

    /// A signing's answer kept: the round it left, where the spend still
    /// holds the PSBT `from` it was started on. Whether it was kept.
    pub(crate) fn keep_round(
        &mut self,
        from: &Psbt,
        round: Option<faraday_psbt::MusigSession>,
    ) -> bool {
        let Some(s) = self.spend.as_mut().filter(|s| s.spend.psbt == *from) else {
            return false;
        };
        s.musig = round;
        true
    }

    /// Ends a MuSig2 round every nonce of which has signed. Whether there
    /// is an open spend.
    pub(crate) fn end_spent_round(&mut self) -> bool {
        let Some(s) = self.spend.as_mut() else {
            return false;
        };
        // A session whose every nonce has signed is over: the next pass
        // over this transaction opens a new one, never the same nonce.
        if s.musig.as_ref().is_some_and(|m| m.is_empty()) {
            s.musig = None;
        }
        true
    }

    /// A signing's result moved into the open spend: the spend as the job
    /// left it, Review carried over to it and what was worked out. Where a
    /// signature was made (not only a nonce) Sign is done, and the PSBT to
    /// remember the amounts of is returned.
    pub(crate) fn take_signed(
        &mut self,
        spend: Spend,
        fresh: crate::signing::fresh::Fresh,
        nonce_only: bool,
    ) -> Option<Psbt> {
        let s = self.spend.as_mut()?;
        // Review gone through for the PSBT as it was vouches for what
        // this device signed of it.
        let prior = s.spend.psbt.clone();
        s.take_spend(spend);
        s.carry_review(&prior);
        s.take(fresh);
        s.error = None;
        if nonce_only {
            return None;
        }
        s.done[step::SIGN as usize] = true;
        Some(s.spend.psbt.clone())
    }

    /// What a signing leaves open on the spend once it is worked out again.
    pub(crate) fn open_after_sign(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            s.follow = true;
            if s.complete {
                s.done[step::COLLECT as usize] = true;
                s.open = Some(s.review_owed().map_or(step::FINISH, |(n, _)| n));
            } else if s.steps.contains(&step::COLLECT) {
                s.open = Some(step::COLLECT);
            } else {
                s.open = Some(step::FINISH);
            }
        }
    }

    /// What a Collect leaves open on the spend once it is worked out again.
    pub(crate) fn open_after_collect(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            s.follow = true;
            if s.complete {
                s.done[step::COLLECT as usize] = true;
                s.open = Some(s.review_owed().map_or(step::FINISH, |(n, _)| n));
            }
        }
    }

    /// Whether the open spend still holds `input`.
    pub(crate) fn spend_holds(&self, input: &Psbt) -> bool {
        self.spend.as_ref().is_some_and(|s| s.spend.psbt == *input)
    }

    /// A Collect's answer moved into the open spend, where it still holds
    /// the PSBT `input` the job was given. Whether it did. `error` is
    /// kept where there is one; with none, the error clears where
    /// something was `found`, and stays otherwise (the end says it).
    pub(crate) fn take_collected(
        &mut self,
        input: &Psbt,
        spend: Spend,
        error: Option<Text>,
        found: bool,
        fresh: Option<crate::signing::fresh::Fresh>,
    ) -> bool {
        let Some(s) = self.spend.as_mut().filter(|s| s.spend.psbt == *input) else {
            return false;
        };
        s.take_spend(spend);
        if error.is_some() {
            s.error = error;
        } else if found {
            s.error = None;
        }
        if let Some(f) = fresh {
            s.take(f);
            // The answer's inspection is the one Review will show now: a
            // warning or key it did not show before lapses Review, as a
            // redone inspection in `refresh` does.
            s.review_lapse_inspection();
        }
        true
    }

    /// The name the open spend's part-signed file went out under.
    pub(crate) fn set_part_name(&mut self, name: Text) {
        if let Some(s) = self.spend.as_mut() {
            s.out_names.part = Some(name);
        }
    }

    /// The name the open spend's signed PSBT went out under.
    pub(crate) fn set_signed_name(&mut self, name: Text) {
        if let Some(s) = self.spend.as_mut() {
            s.out_names.signed = Some(name);
        }
    }

    /// The name the open spend's finished transaction went out under.
    pub(crate) fn set_tx_name(&mut self, name: Text) {
        if let Some(s) = self.spend.as_mut() {
            s.out_names.tx = Some(name);
        }
    }

    /// The signed PSBT's file for the Outbox, marked as gone there: its
    /// name and bytes, where the spend is reviewed and held by no nonce.
    /// `None` where it is not; an error where a version 2 PSBT cannot be
    /// written.
    pub(crate) fn take_signed_file(&mut self) -> Option<Result<(Text, Vec<u8>), ()>> {
        let s = self
            .spend
            .as_mut()
            .filter(|s| !s.spend.nonce_hold && s.reviewed_now())?;
        let name = text!(
            "{}-signed.psbt",
            crate::result_stem(&s.spend.source).as_str()
        );
        let Ok(bytes) = s.spend.out_bytes() else {
            return Some(Err(()));
        };
        s.out_signed = true;
        Some(Ok((name, bytes)))
    }

    /// The finished transaction's file for the Outbox, marked as gone
    /// there, where the spend is reviewed and finished.
    pub(crate) fn take_tx_file(&mut self) -> Option<(Text, Vec<u8>)> {
        let s = self.spend.as_mut().filter(|s| s.reviewed_now())?;
        let hex = s.spend.finished_hex()?;
        let name = text!("{}-final.txn", crate::result_stem(&s.spend.source).as_str());
        s.out_tx = true;
        Some((name, hex.as_bytes().to_vec()))
    }

    /// The files of a finished, reviewed spend not made yet, marked as gone
    /// to the Outbox, each with whether it is the signed PSBT. An error
    /// where a version 2 PSBT cannot take the signatures back.
    pub(crate) fn take_result_files(&mut self) -> Result<Vec<(bool, Text, Vec<u8>)>, ()> {
        let Some(s) = self
            .spend
            .as_mut()
            .filter(|s| s.spend.finished.is_some() && s.reviewed_now())
        else {
            return Ok(Vec::new());
        };
        let stem = crate::result_stem(&s.spend.source);
        let stem = stem.as_str();
        let mut made = Vec::new();
        if !s.out_signed {
            let Ok(bytes) = s.spend.out_bytes() else {
                return Err(());
            };
            s.out_signed = true;
            made.push((true, text!("{stem}-signed.psbt"), bytes));
        }
        if !s.out_tx
            && let Some(hex) = s.spend.finished_hex()
        {
            s.out_tx = true;
            made.push((false, text!("{stem}-final.txn"), hex.as_bytes().to_vec()));
        }
        Ok(made)
    }

    /// A Collect found nothing to add from the file `name`.
    pub(crate) fn collect_found_nothing(&mut self, name: &str) {
        if let Some(s) = self.spend.as_mut() {
            s.error = Some(Text::fill(EN.collect_has_nothing, &[name]));
        }
    }

    /// Opens the register sheet's state over the open spend. Whether the
    /// inspection offers one.
    pub(crate) fn open_register(&mut self) -> bool {
        let Some(s) = self
            .spend
            .as_mut()
            .filter(|s| s.inspection.register.is_some())
        else {
            return false;
        };
        s.register = Some(crate::register::RegisterAsk::new(&s.spend.source));
        true
    }

    /// The register sheet shows `rows`, the chosen reading's.
    pub(crate) fn register_set_rows(&mut self, rows: Option<crate::wallet::Rows>) {
        if let Some(ask) = self.spend.as_mut().and_then(|s| s.register.as_mut()) {
            ask.rows = rows;
        }
    }

    /// The person's tick of key `key` on the register sheet's rows.
    pub(crate) fn register_tick(&mut self, key: usize) {
        if let Some(rows) = self
            .spend
            .as_mut()
            .and_then(|s| s.register.as_mut())
            .and_then(|a| a.rows.as_mut())
        {
            rows.tick(key);
        }
    }

    /// The register sheet's name field is being typed in.
    pub(crate) fn register_typing(&mut self) {
        if let Some(ask) = self.spend.as_mut().and_then(|s| s.register.as_mut()) {
            ask.typing = true;
        }
    }

    /// The register sheet reads the policy as reading `k`.
    pub(crate) fn register_reading(&mut self, k: u8) {
        if let Some(ask) = self.spend.as_mut().and_then(|s| s.register.as_mut()) {
            ask.reading = usize::from(k);
            ask.refused = None;
        }
    }

    /// The register sheet refused with `why`.
    pub(crate) fn register_refused(&mut self, why: Text) {
        if let Some(ask) = self.spend.as_mut().and_then(|s| s.register.as_mut()) {
            ask.refused = Some(why);
        }
    }

    /// A key pressed on the register sheet's field. Whether it took it.
    pub(crate) fn register_key(&mut self, key: faraday_shell_api::Key) -> bool {
        self.spend
            .as_mut()
            .and_then(|s| s.register.as_mut())
            .is_some_and(|ask| ask.key(key))
    }

    /// Closes the register sheet's state.
    pub(crate) fn close_register(&mut self) {
        if let Some(s) = self.spend.as_mut() {
            s.register = None;
        }
    }

    /// The other transactions made for the open spend, from the files
    /// `from` counts.
    pub(crate) fn keep_others(
        &mut self,
        made: Vec<crate::decode::check_others::Other>,
        from: Vec<OtherFrom>,
    ) {
        if let Some(s) = self.spend.as_mut() {
            s.others = made;
            s.others_from = Some(from);
            s.others_stale = false;
        }
    }

    /// The nonce check's hold on the open spend is `held`: where it
    /// changed, a held spend is not finished and its files are taken back
    /// (their names are returned), and a lifted one is finished again.
    /// The names, and whether it was lifted.
    pub(crate) fn apply_nonce_hold(&mut self, held: bool) -> (Vec<Text>, bool) {
        let mut taken_back = Vec::new();
        let mut lifted = false;
        let Some(s) = self.spend.as_mut() else {
            return (taken_back, lifted);
        };
        if held != s.spend.nonce_hold {
            s.spend.nonce_hold = held;
            if held {
                s.spend.finished = None;
                s.complete = false;
                // What this spend already put in the Outbox goes back.
                s.out_signed = false;
                s.out_tx = false;
                taken_back.extend(s.out_names.signed.take());
                taken_back.extend(s.out_names.tx.take());
                taken_back.extend(s.out_names.part.take());
            } else {
                s.complete = s.spend.finish().is_ok();
                lifted = s.complete;
            }
        }
        (taken_back, lifted)
    }

    /// Costs an inspection when the PSBT or the session changed. Keeps the
    /// open spend in step with the session: its inspection, signers,
    /// wallet and steps. `checked` says whether a wallet's id was
    /// checked this power-on.
    pub(crate) fn refresh(
        &mut self,
        c: &Compute,
        session: &Session,
        checked: impl Fn(WalletId) -> bool,
    ) {
        let Some(s) = self.spend.as_mut() else {
            return;
        };
        // Inspected once each time the PSBT or the session changes,
        // not on every press that refreshes the spend. The signers, the
        // wallet and whether it is complete are read from that
        // inspection, so they are worked out when it is made (a spend
        // opens with them worked out, `SpendState::opened_with`).
        let inputs = session.inspect_inputs();
        let view = s.musig.as_ref().map(|m| m.view());
        if !s.inspected_for(&inputs) {
            let fresh =
                crate::signing::fresh::worked_out(c, session, &mut s.spend, view.as_ref(), true);
            s.take(fresh);
            s.others_stale = true;
            s.review_lapse_inspection();
        }
        // The wallets the outputs go to, named once here for every draw.
        s.own_names = s
            .inspection
            .outputs
            .iter()
            .map(|o| o.own.as_ref().and_then(|own| session.own_wallet_name(own)))
            .collect();
        s.review_lapse();
        let wallet = s.wallet.and_then(|w| session.wallets.get(w));
        let kind = wallet.map(|w| Kind::of(w.policy()));
        s.needed = wallet.map(needed).unwrap_or(1);
        // With no wallet loaded the signers come from the PSBT alone,
        // and a second device may still be needed.
        let kind = kind.unwrap_or(if s.inspection.participating_keys.len() > 1 {
            Kind::Multi(faraday_bip::policy::Wrapper::Wsh)
        } else {
            Kind::Single(faraday_bip::keys::ScriptType::NativeSegwit)
        });
        s.steps = kind.steps();
        // Check is done already, this power-on, when this wallet was
        // checked before (`docs/WALLETS.md` §4): the flow opens on
        // the next card without a default instead.
        if let Some(wallet) = wallet
            && checked(wallet.id())
        {
            s.done[step::CHECK as usize] = true;
        }
        s.settle_owed();
        let stale = s.open.is_some_and(|open| !s.steps.contains(&open));
        if stale || !s.opened {
            s.open = s.steps.iter().copied().find(|&n| !s.done[n as usize]);
            s.opened = true;
        }
    }

    /// The transaction being signed scrolled to `y` units, if one is.
    pub(crate) fn set_spend_scroll(&mut self, y: f32) {
        if let Some(s) = self.spend.as_mut() {
            s.scroll = y;
        }
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
    /// The step shown: the open one, or with none open the first not done,
    /// or the last: a step is always open.
    pub(crate) fn shown(s: &SpendState) -> Option<u8> {
        s.open
            .or_else(|| s.steps.iter().copied().find(|&k| !s.done[usize::from(k)]))
            .or_else(|| s.steps.last().copied())
    }

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
        let mut steps = crate::trail::Steps::new(s.steps.clone(), SpendSteps::shown(s));
        steps.first = SpendSteps::first(s);
        steps.answered = s
            .steps
            .iter()
            .copied()
            .filter(|&k| s.done[usize::from(k)])
            .collect();
        Some(steps)
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::Step(k)
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        crate::screens::SPEND_TITLES[usize::from(k)]
    }

    fn contained(&self) -> Option<&dyn crate::flow::Contained> {
        Some(self)
    }
}

impl crate::flow::Contained for SpendSteps {
    fn rows(&self, app: &Faraday) -> Vec<crate::flow::Row> {
        let Some(s) = app.wallets.spend() else {
            return Vec::new();
        };
        let shown = SpendSteps::shown(s);
        let mut rows = Vec::new();
        for &k in &s.steps {
            let reached = s.done[usize::from(k)] || shown == Some(k);
            rows.push(crate::flow::Row {
                label: Text::of(crate::screens::SPEND_TITLES[usize::from(k)]),
                value: reached.then(|| crate::screens::step_summary(app, k, s.wallet, s.needed)),
                mono: false,
                opens: k,
                current: shown == Some(k),
                mark: None,
            });
            // The keys stay listed under Signers once chosen.
            if k == step::SIGNERS && reached {
                rows.extend(s.key_rows(app, k));
            }
        }
        rows
    }

    fn hide_shown(&self, app: &mut crate::Faraday) {
        // No step of this flow shows a secret.
        let _ = app;
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        false
    }

    fn still_fits(&self, _: &Faraday, changed: u8, later: u8) -> bool {
        // A wallet known anew owes Check again (the signing's fresh read); every
        // other answer stands.
        !(changed == step::WALLET && later == step::CHECK)
    }
}

// A spend under way.
impl crate::at_lock::AtLock for WalletsArea {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds::session(self.spend().is_some())
    }
    fn lock(&mut self) {
        self.clear_spend();
    }
}

/// A drawn character that reads as a plain letter of a mark: a Latin-1
/// accent as its base letter, `1 l ! |` as `i`, `5 $` as `s`; any other
/// character as itself.
fn fold_lookalike(c: char) -> char {
    match c {
        '1' | 'l' | '!' | '|' | '¡' | '¦' => 'i',
        '5' | '$' | '§' => 's',
        'à'..='å' => 'a',
        'ç' => 'c',
        'è'..='ë' => 'e',
        'ì'..='ï' => 'i',
        'ð' => 'd',
        'ñ' => 'n',
        'ò'..='ö' | 'ø' => 'o',
        'ù'..='ü' => 'u',
        'ý' | 'ÿ' => 'y',
        c => c,
    }
}

/// Whether a key's name could be read as the mark drawn after it in a
/// Signers row ("Signed", "Signs here"): with all but letters taken out
/// and case folded it holds "signed" or "signshere" ("Designed" too, which
/// is left out with the rest), or it holds a letter of another script that
/// looks like a Latin one (Cyrillic, Greek, fullwidth, mathematical
/// alphanumeric). Such a name is not shown there; the fingerprint is.
pub(crate) fn name_reads_as_mark(name: &str) -> bool {
    let like = |c: char| {
        matches!(c as u32,
            0x0370..=0x03FF | 0x1F00..=0x1FFF | 0x0400..=0x052F
            | 0xFF00..=0xFFEF | 0x1D400..=0x1D7FF)
    };
    if name.chars().any(like) {
        return true;
    }
    // Read both ways: letters only, and with lookalikes folded first (a
    // folded `!` is a letter, so "Signs! here" reads as a mark the first
    // way). The last nine letters of each hold "signshere".
    let reads = |fold: bool| {
        let mut last = ['\0'; 9];
        for c in name
            .chars()
            .flat_map(char::to_lowercase)
            .map(|c| if fold { fold_lookalike(c) } else { c })
            .filter(|c| c.is_alphabetic())
        {
            last.rotate_left(1);
            last[8] = c;
            let tail = |w: &str| w.chars().rev().zip(last.iter().rev()).all(|(a, b)| a == *b);
            if tail("signed") || tail("signshere") {
                return true;
            }
        }
        false
    };
    reads(false) || reads(true)
}

#[cfg(test)]
mod tests {
    use super::name_reads_as_mark;

    #[test]
    fn a_name_that_could_read_as_a_mark_is_left_out() {
        for n in [
            "Signed",
            "Sígned",
            "Signéd",
            "Signeð",
            "S1gned",
            "Slgned",
            "SlGNED",
            "S!gned",
            "5igned",
            "$igned",
            "Signs hére",
            "Cold · Signed",
            "signed",
            "S i g n e d",
            "Signs here",
            "SignsHere",
            "Signs-here!",
            "Signs! here",
            "Signs | here",
            "Sig1ned",
            "§igned",
            "Designed",
            "Sіgned",
            "Ѕigned",
            "Σigned",
            "Ｓｉｇｎｅｄ",
            "𝐒𝐢𝐠𝐧𝐞𝐝",
        ] {
            assert!(name_reads_as_mark(n), "{n}");
        }
        for n in [
            "Savings key",
            "Schlüssel",
            "Design 2",
            "Cold",
            "Sign",
            "Signs",
        ] {
            assert!(!name_reads_as_mark(n), "{n}");
        }
    }
}
