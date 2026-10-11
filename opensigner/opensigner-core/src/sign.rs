//! The Sign flow's state (UX.md §7.3): how the PSBT arrives, the
//! inspection the review steps show, which keys will sign, the risk
//! acknowledgement, and the outcome.
//!
//! Nothing here is secret. The PSBT and its inspection are private data
//! (amounts, addresses, key origins) and are dropped as soon as the flow
//! is left; the master keys stay in [`crate::load::LoadedKey`] and are
//! borrowed only for the moment [`SignFlow::sign`] runs.
//!
//! The result can be shown as a QR (UX.md F4, `docs/PLANNING.md` §16.20):
//! one static code when the payload fits ([`STATIC_QR_MAX_BYTES`]) *and*
//! its modules are wide enough on this display to be read
//! ([`osk_ui::widgets::QR_MIN_PITCH_MM`]), or an animated
//! `ur:crypto-psbt` whose parts advance on ticks every
//! [`crate::codes::UR_FRAME_MS`] milliseconds. `docs/DESIGN.md` §4.9
//! settles the part count: it "follows from the side and the floor, not
//! from a fixed fragment size", which [`crate::codes`] works out.

use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use osk_bip::keys::{Fingerprint, MasterKey, Network};
use osk_bip::policy::WalletPolicy;
use osk_codec::qr::{Ecc, Payload, QrMatrix};
use osk_psbt::bitcoin::Txid;
use osk_psbt::bitcoin::consensus::encode::serialize;
use osk_psbt::threshold::{CARRY_NAME, Carry, CarrySection, carry_bytes};
use osk_psbt::verify::{Determinism, InputSignatures, NonceMode, SignaturePresent, Verdict};
use osk_psbt::{
    Context, Error, InputSignature, Inspection, KeyRef, Level, MusigSession, MusigSessionView,
    Psbt, ShareKey, ShareRef, ThresholdRound, Warning, WarningKind,
};

use crate::codes::{self, UrKind, UrRun};
use crate::text;

/// Largest payload shown as one static QR: the byte-mode capacity of
/// version 40 at level L. Longer payloads animate as BC-UR.
pub const STATIC_QR_MAX_BYTES: usize = 2953;

/// One row of the Signatures page (`docs/PLANNING.md` §16.111): a
/// signature the transaction carries, what checking it came to, and,
/// for a key this device holds, which nonce rule made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureRow {
    /// The input it signs.
    pub input: usize,
    /// The key it is under: the master fingerprint where the PSBT names
    /// one, and otherwise the head of the public key.
    pub name: String,
    /// What checking it came to.
    pub verdict: Verdict,
    /// `None` where the key is not this device's; `Some(None)` where it
    /// is and no nonce rule reproduced the signature.
    pub determinism: Option<Option<NonceMode>>,
    /// The signature itself, which the row opens whole on Compare.
    pub bytes: Vec<u8>,
}

/// Where the wizard is once a PSBT is loaded (UX.md §7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Amount out, recipient, fee, change, inputs.
    Summary,
    /// One output per screen.
    Outputs,
    /// The inputs, one short row each.
    Inputs,
    /// The ranked warnings; only when there are any.
    Warnings,
    /// Key choice and the hold to sign.
    Confirm,
    /// Signatures, output file, transaction id.
    Result,
    /// The signatures the flow wrote, one row each, reached from the
    /// result.
    Signatures,
    /// The result as a QR code, reached from the result.
    Qr,
}

/// The review steps in order. [`SignFlow::next`] skips the ones that
/// have nothing to show.
const REVIEW: [Step; 5] = [
    Step::Summary,
    Step::Outputs,
    Step::Inputs,
    Step::Warnings,
    Step::Confirm,
];

/// Where the flow is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Choosing how the PSBT arrives.
    Entry,
    /// A file request is out; the shell has not answered yet.
    Waiting,
    /// The shell answered `FileUnavailable`; the entry rows stay.
    Unavailable,
    /// The bytes were not a PSBT; [`SignFlow::error`] says why.
    ParseError,
    /// A PSBT is loaded and inspected.
    Wizard(Step),
}

/// What signing produced.
pub struct Outcome {
    /// The threshold wallet's `(shares that must sign, partial
    /// signatures present)` after the pass, when the transaction spent
    /// one.
    pub threshold: Option<(usize, usize)>,
    /// The carry section this pass leaves behind, when signers are still
    /// to sign. It lives here until Save writes it or the flow is left,
    /// and is wiped either way (`docs/PLANNING.md` §16.103).
    pub carry: Option<CarrySection>,
    /// Per signed input.
    pub signatures: Vec<InputSignature>,
    /// MuSig2 public nonces this pass shared, which is what a pass that
    /// ended in round 1 produced (`docs/PLANNING.md` §16.100).
    pub nonces_shared: usize,
    /// Every input is final and the transaction was extracted.
    pub complete: bool,
    /// The transaction id, when complete.
    pub txid: Option<Txid>,
    /// What "Save to file" hands the shell: the binary signed PSBT, or the
    /// final transaction as hex text.
    pub bytes: Vec<u8>,
    /// The signed (and, when complete, finalized) PSBT, binary. What the
    /// animated UR carries.
    pub psbt: Vec<u8>,
    /// File name hint for the shell.
    pub name_hint: String,
    /// What became of "Save to file".
    pub save: Save,
}

/// What became of "Save to file": the core asks the shell to write the
/// bytes and cannot call the file saved until the shell says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Save {
    /// Not asked for yet.
    Idle,
    /// A write is out; the shell has not answered.
    Waiting,
    /// The shell stored the bytes.
    Written,
    /// The shell stored nothing: no file channel, a cancelled picker, a
    /// card that is full or not there.
    Failed,
}

/// How the QR page shows the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrMode {
    /// One code: the final transaction as hex, or the PSBT as base64.
    Static,
    /// Animated `ur:crypto-psbt` parts.
    Ur,
}

/// The QR page's state: the code on screen and, for UR, the part cycle.
struct QrPage {
    mode: QrMode,
    matrix: Option<Rc<QrMatrix>>,
    run: Option<UrRun>,
}

/// The flow's state.
pub struct SignFlow {
    stage: Stage,
    network: Network,
    keys: Vec<KeyRef>,
    /// The wallets in use when the PSBT was loaded, which is what its
    /// change outputs are verified against.
    wallets: Vec<WalletPolicy>,
    psbt: Option<Psbt>,
    inspection: Option<Inspection>,
    /// Fingerprints chosen to sign, a subset of the participating keys.
    selected: Vec<Fingerprint>,
    /// The shares of threshold wallets this device holds.
    shares: Vec<ShareRef>,
    /// The carry file's section, when the transaction arrived as one.
    carry: Option<CarrySection>,
    /// The other shares chosen to sign at the first location: the
    /// "Then with" row's choice, `t - 1` identifiers of the record.
    others: Vec<u32>,
    /// The output on screen, while the outputs step is showing.
    output: usize,
    acknowledged: bool,
    error: Option<String>,
    outcome: Option<Outcome>,
    qr: Option<QrPage>,
    /// Whether one static code would be readable on this display, which
    /// is what [`SignFlow::show_qr`] chooses on.
    static_scans: bool,
    /// The largest QR version this display's square keeps at or above the
    /// pitch floor, from the frame the QR page was opened on (§4.9).
    qr_version: u8,
    /// Whether the flow is reading rather than signing: Tools › Decode a
    /// transaction, which walks the same review screens with no keys,
    /// no confirm step and Done at the end.
    reading: bool,
    /// The id of the raw transaction a reading flow was handed, when the
    /// PSBT wrapper it was inspected through is not the same bytes.
    read_txid: Option<Txid>,
    /// Every signature the transaction carries, checked (§16.111 rule
    /// 1). Taken from the inspection when the PSBT arrives and taken
    /// again from the signed PSBT once this device has signed, so that
    /// the page lists what the transaction holds now.
    signatures: Vec<InputSignatures>,
    /// Which nonce rule made each signature under a key this device
    /// holds (rule 3), which needs the master keys and so is filled by
    /// [`SignFlow::check`] rather than by the inspection.
    determinism: Vec<Determinism>,
    /// The information cards rule 3 raises, which are not the
    /// inspection's because the inspection has no key.
    checked_warnings: Vec<Warning>,
    /// The review step the Signatures page was opened from, which is
    /// where its chevron goes back to.
    signatures_from: Option<Step>,
}

impl Default for SignFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl SignFlow {
    /// A flow at the entry screen with nothing loaded.
    pub fn new() -> Self {
        SignFlow {
            stage: Stage::Entry,
            network: Network::Mainnet,
            keys: Vec::new(),
            wallets: Vec::new(),
            psbt: None,
            inspection: None,
            selected: Vec::new(),
            shares: Vec::new(),
            carry: None,
            others: Vec::new(),
            output: 0,
            acknowledged: false,
            error: None,
            outcome: None,
            qr: None,
            static_scans: false,
            qr_version: 1,
            reading: false,
            read_txid: None,
            signatures: Vec::new(),
            determinism: Vec::new(),
            checked_warnings: Vec::new(),
            signatures_from: None,
        }
    }

    /// Whether this flow reads a transaction rather than signing one.
    pub fn reading(&self) -> bool {
        self.reading
    }

    /// Parses and inspects `bytes` for the Decode tool: the same review
    /// the Sign flow shows, with whatever keys and wallets this device
    /// holds, so change of a wallet in use is recognised. Nothing is
    /// signed, there is no confirm step, and the review ends at the last
    /// page it has to show.
    pub fn read(
        &mut self,
        bytes: &[u8],
        keys: Vec<KeyRef>,
        wallets: Vec<WalletPolicy>,
        shares: Vec<ShareRef>,
        network: Network,
        session: Option<&MusigSessionView>,
    ) {
        self.reading = true;
        self.read_txid = None;
        self.load(bytes, keys, wallets, shares, network, session);
    }

    /// The same, for a raw transaction wrapped in a PSBT: `txid` is the
    /// id of the transaction that arrived, which the wrapper does not
    /// carry.
    #[allow(clippy::too_many_arguments)]
    pub fn read_raw(
        &mut self,
        bytes: &[u8],
        txid: Txid,
        keys: Vec<KeyRef>,
        wallets: Vec<WalletPolicy>,
        shares: Vec<ShareRef>,
        network: Network,
        session: Option<&MusigSessionView>,
    ) {
        self.read(bytes, keys, wallets, shares, network, session);
        self.read_txid = Some(txid);
    }

    /// Whether the flow was given anything of this device's to recognise
    /// an output by. A reading flow with nothing has no verdict to give
    /// about change: it has not looked.
    pub fn has_context(&self) -> bool {
        !self.keys.is_empty() || !self.wallets.is_empty()
    }

    /// The id of the transaction as it stands, unsigned. What a reading
    /// review shows; a signing one shows the id of what it produced.
    pub fn unsigned_txid(&self) -> Option<Txid> {
        self.read_txid
            .or_else(|| Some(self.psbt.as_ref()?.unsigned_tx().compute_txid()))
    }

    /// The last review step this transaction has, which is where a
    /// reading flow's Done sits.
    pub fn last_review_step(&self) -> Step {
        REVIEW
            .iter()
            .rev()
            .copied()
            .find(|s| self.has(*s))
            .unwrap_or(Step::Summary)
    }

    /// Where the flow is.
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// The parse or signing error to show, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The inspection of the loaded PSBT.
    pub fn inspection(&self) -> Option<&Inspection> {
        self.inspection.as_ref()
    }

    /// The signing outcome, once signed.
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }

    /// The warnings, blocks first, then dangers, cautions and info.
    ///
    /// The inspection's own, and the cards the determinism check adds:
    /// a signature under a key this device holds that no nonce rule
    /// reproduces is information the inspection cannot give, because it
    /// has no key (`docs/PLANNING.md` §16.111 rule 3).
    pub fn ranked_warnings(&self) -> Vec<&Warning> {
        let mut w: Vec<&Warning> = self
            .inspection
            .as_ref()
            .map(|i| i.warnings.iter().filter(|w| self.raised(w)).collect())
            .unwrap_or_default();
        w.extend(self.checked_warnings.iter());
        w.sort_by_key(|w| core::cmp::Reverse(w.level));
        w
    }

    /// Every signature the transaction carries, as the Signatures page
    /// lists them (§16.111 rule 1).
    pub fn signature_rows(&self) -> Vec<SignatureRow> {
        self.signatures
            .iter()
            .flat_map(|i| i.signatures.iter())
            .map(|sig| SignatureRow {
                input: sig.input,
                name: sig.name(),
                verdict: sig.verdict,
                determinism: self.determinism_of(sig),
                bytes: sig.bytes.clone(),
            })
            .collect()
    }

    /// How many signatures the transaction carries, which is the value
    /// of the review's Signatures row.
    pub fn signature_count(&self) -> usize {
        self.signatures.iter().map(|i| i.signatures.len()).sum()
    }

    /// Which nonce rule made `sig`, where it is under a key this device
    /// holds and the check has run.
    fn determinism_of(&self, sig: &SignaturePresent) -> Option<Option<NonceMode>> {
        let found = self
            .determinism
            .iter()
            .find(|d| d.input == sig.input && d.key == sig.key)?;
        Some(found.mode)
    }

    /// Checks the signatures under keys this device holds against the
    /// nonce rules that could have made them (§16.111 rule 3). Called
    /// whenever a transaction is loaded, read or inspected again; the
    /// master keys are borrowed for the call and nothing of them is
    /// kept.
    pub fn check(&mut self, masters: &[&MasterKey]) {
        self.determinism.clear();
        self.checked_warnings.clear();
        let Some(psbt) = self.psbt.as_ref() else {
            return;
        };
        self.determinism = osk_psbt::verify::deterministic(psbt, masters);
        for d in &self.determinism {
            if d.mode.is_none() {
                self.checked_warnings.push(Warning {
                    level: Level::Info,
                    kind: WarningKind::Nondeterministic {
                        input: d.input,
                        key: alloc::format!("{}", d.fingerprint),
                    },
                    text: alloc::format!(
                        "input {}: the signature of {} matches no nonce rule this device has",
                        d.input,
                        d.fingerprint
                    ),
                });
            }
        }
    }

    /// Opens the Signatures page, remembering the step to come back to.
    pub fn open_signatures(&mut self, from: Step) {
        self.signatures_from = Some(from);
        self.stage = Stage::Wizard(Step::Signatures);
    }

    /// Whether a warning is one this flow raises. Two of them are about
    /// signing rather than about the transaction — an output claiming a
    /// key that is not loaded, and no key that can sign — and a review
    /// that signs nothing has nothing to say about either.
    fn raised(&self, w: &Warning) -> bool {
        !self.reading
            || !matches!(
                w.kind,
                WarningKind::UnverifiedChange | WarningKind::NoParticipatingKey
            )
    }

    /// Whether the inspection carries a danger warning.
    pub fn has_danger(&self) -> bool {
        self.ranked_warnings()
            .iter()
            .any(|w| w.level == Level::Danger)
    }

    /// Whether the inspection carries a warning that blocks signing. No
    /// acknowledgement is offered for one, and the review stops at the
    /// warnings.
    pub fn has_blocked(&self) -> bool {
        self.ranked_warnings()
            .iter()
            .any(|w| w.level == Level::Blocked)
    }

    /// Number of danger warnings.
    pub fn danger_count(&self) -> usize {
        self.ranked_warnings()
            .iter()
            .filter(|w| w.level == Level::Danger)
            .count()
    }

    /// Whether `fp` is chosen to sign.
    pub fn is_selected(&self, fp: Fingerprint) -> bool {
        self.selected.contains(&fp)
    }

    /// Whether the risk acknowledgement is on.
    pub fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    /// Number of outputs in the transaction.
    pub fn outputs(&self) -> usize {
        self.inspection.as_ref().map_or(0, |i| i.outputs.len())
    }

    /// The output the outputs step shows, 0-based.
    pub fn output(&self) -> usize {
        self.output.min(self.outputs().saturating_sub(1))
    }

    /// Shows the previous or the next output, staying inside the run.
    pub fn set_output(&mut self, index: usize) {
        self.output = index.min(self.outputs().saturating_sub(1));
    }

    /// Continue on the outputs step: the next output, or the step after
    /// the run when this was the last one (`docs/DESIGN.md` §4.1 keeps
    /// the pager for runs of pages the app bar does not already count).
    pub fn next_output(&mut self) -> bool {
        if self.output() + 1 < self.outputs() {
            self.output = self.output() + 1;
            return true;
        }
        false
    }

    /// Whether `step` has anything to show for this transaction. A
    /// blocked transaction has no confirm step: the review ends at the
    /// warnings that stopped it.
    fn has(&self, step: Step) -> bool {
        match step {
            Step::Warnings => !self.ranked_warnings().is_empty(),
            // Nothing is signed by a flow that is only reading, so it
            // has no confirm step at all.
            Step::Confirm => !self.reading && !self.has_blocked(),
            _ => true,
        }
    }

    /// The review step after `step`, skipping the empty ones.
    pub fn after(&self, step: Step) -> Option<Step> {
        let i = REVIEW.iter().position(|s| *s == step)?;
        REVIEW[i + 1..].iter().copied().find(|s| self.has(*s))
    }

    /// The review step before `step`, skipping the empty ones.
    pub fn before(&self, step: Step) -> Option<Step> {
        let i = REVIEW.iter().position(|s| *s == step)?;
        REVIEW[..i].iter().rev().copied().find(|s| self.has(*s))
    }

    /// Whether any danger has been accepted with the separate gesture,
    /// so the review can go on to the confirm step. A blocked warning is
    /// never accepted: there is no gesture for it.
    pub fn can_acknowledge(&self) -> bool {
        !self.has_blocked() && (self.acknowledged || !self.has_danger())
    }

    /// Whether this pass will end in round 1 for every MuSig2
    /// participant this device holds, which is what the hold's label
    /// says (§16.100). False when nothing on the transaction is MuSig2.
    pub fn shares_nonce(&self) -> bool {
        let Some(insp) = self.inspection.as_ref() else {
            return false;
        };
        let mine: Vec<&osk_psbt::MusigInput> = insp
            .inputs
            .iter()
            .filter_map(|i| i.musig.as_ref())
            .filter(|m| !m.ours.is_empty())
            .collect();
        !mine.is_empty() && mine.iter().all(|m| m.will_share_nonce)
    }

    /// Whether the hold-to-sign button is live: a key is chosen, nothing
    /// is blocked, and any danger has been acknowledged with the separate
    /// gesture.
    pub fn can_sign(&self) -> bool {
        match self.threshold_first() {
            // A threshold spend at the first location signs with the
            // share this device holds and the `t - 1` others the person
            // chose, and the hold is dead until exactly that many are
            // chosen (§16.103).
            Some(t) => self.can_acknowledge() && self.others.len() + 1 == t,
            None => {
                let threshold_only = self.selected.is_empty()
                    && self.threshold().is_some_and(|t| !t.ours.is_empty());
                (!self.selected.is_empty() || threshold_only) && self.can_acknowledge()
            }
        }
    }

    // ----- the QR page -----

    /// The static payload: the final transaction's hex, or the PSBT as
    /// base64 text (byte mode; base64 has lowercase letters, and text is
    /// what every phone scanner and coordinator reads).
    fn static_text(out: &Outcome) -> Vec<u8> {
        if out.complete {
            out.bytes.clone()
        } else {
            osk_psbt::base64::encode(&out.psbt).into_bytes()
        }
    }

    /// The bytes one static code of the result carries, whether or not
    /// they fit one: what a saved picture of the code holds.
    pub fn static_payload(&self) -> Option<Vec<u8>> {
        self.outcome.as_ref().map(Self::static_text)
    }

    /// Whether the result fits one static code.
    pub fn static_qr_fits(&self) -> bool {
        self.outcome
            .as_ref()
            .is_some_and(|o| Self::static_text(o).len() <= STATIC_QR_MAX_BYTES)
    }

    /// Whether one static code is readable on the display the page was
    /// opened on: the modules of the code this payload needs are at
    /// least [`QR_MIN_PITCH_MM`] wide there.
    pub fn static_qr_scans(&self) -> bool {
        self.static_scans
    }

    /// The module pitch in millimetres one static code would have in a
    /// square of `side_px` on a `dpi` display; `None` when the payload
    /// does not fit one code at all.
    pub fn static_qr_pitch_mm(&self, side_px: i32, dpi: u16) -> Option<f32> {
        let out = self.outcome.as_ref()?;
        let text = Self::static_text(out);
        if text.len() > STATIC_QR_MAX_BYTES {
            return None;
        }
        let matrix = osk_codec::qr::encode(Payload::Bytes(&text), Ecc::Low).ok()?;
        Some(osk_ui::widgets::qr_module_pitch_mm(
            side_px,
            matrix.size(),
            dpi,
        ))
    }

    /// Opens the QR page in the shape a camera can actually read: one
    /// static code when its modules are wide enough on this display,
    /// animated parts otherwise (§4.9). The toggle row still switches
    /// between them.
    ///
    /// `side_px` is the square the code is drawn in, which is
    /// [`osk_ui::tokens::qr_side`] for the class. The version that square
    /// allows at the pitch floor is kept, because it is what the animated
    /// run's fragment size — and so its part count — comes from.
    pub fn show_qr(&mut self, now_ms: u64, side_px: i32, dpi: u16) {
        self.qr_version = codes::max_version(side_px, dpi);
        let text = self.outcome.as_ref().map(Self::static_text);
        self.static_scans = text.as_ref().is_some_and(|t| {
            t.len() <= STATIC_QR_MAX_BYTES && codes::fits_one_code(t, self.qr_version)
        });
        let mode = if self.static_scans {
            QrMode::Static
        } else {
            QrMode::Ur
        };
        self.set_qr_mode(mode, now_ms);
        self.stage = Stage::Wizard(Step::Qr);
    }

    /// The largest QR version this display keeps at or above the pitch
    /// floor at the class's side, from the frame the page was opened on.
    pub fn qr_version(&self) -> u8 {
        self.qr_version
    }

    /// Switches the QR page to `mode` and encodes its first code.
    pub fn set_qr_mode(&mut self, mode: QrMode, now_ms: u64) {
        let Some(out) = self.outcome.as_ref() else {
            return;
        };
        let page = match mode {
            QrMode::Static => {
                let text = Self::static_text(out);
                QrPage {
                    mode,
                    matrix: osk_codec::qr::encode(Payload::Bytes(&text), Ecc::Low)
                        .ok()
                        .map(Rc::new),
                    run: None,
                }
            }
            QrMode::Ur => QrPage {
                mode,
                matrix: None,
                run: UrRun::new(UrKind::Psbt, &out.psbt, self.qr_version, now_ms),
            },
        };
        self.qr = Some(page);
    }

    /// A tick at `now_ms`: advances an animated UR when its frame is due.
    /// Returns whether the code on screen changed.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        if self.stage != Stage::Wizard(Step::Qr) {
            return false;
        }
        self.qr
            .as_mut()
            .and_then(|p| p.run.as_mut())
            .is_some_and(|run| run.tick(now_ms))
    }

    /// The QR page's mode, while it is open.
    pub fn qr_mode(&self) -> Option<QrMode> {
        self.qr.as_ref().map(|p| p.mode)
    }

    /// The code on the QR page.
    pub fn qr_matrix(&self) -> Option<Rc<QrMatrix>> {
        let p = self.qr.as_ref()?;
        match &p.run {
            Some(run) => run.matrix(),
            None => p.matrix.clone(),
        }
    }

    /// `(part shown, fragments in the message)` of an animated UR. The
    /// part number keeps counting past the fragment count: later parts
    /// are fountain mixes.
    pub fn qr_part(&self) -> Option<(usize, usize)> {
        Some(self.qr.as_ref()?.run.as_ref()?.part())
    }

    /// The PSBT the animated run splits, for a test that has to measure
    /// every part of it.
    pub fn ur_payload(&self) -> Option<&[u8]> {
        Some(self.outcome.as_ref()?.psbt.as_slice())
    }

    // ----- transitions -----

    /// A file request went out.
    pub fn request(&mut self) {
        self.stage = Stage::Waiting;
        self.error = None;
    }

    /// The shell had no file.
    pub fn unavailable(&mut self) {
        if self.stage == Stage::Waiting {
            self.stage = Stage::Unavailable;
        }
    }

    /// The shell answered `FileCancelled`: the entry rows come back
    /// exactly as they were, with nothing said about the file.
    pub fn cancelled(&mut self) {
        if self.stage == Stage::Waiting {
            self.stage = Stage::Entry;
        }
    }

    /// Parses `bytes` (binary or base64, surrounding whitespace ignored)
    /// and inspects the PSBT for `keys` on `network`. On success the flow
    /// is at the summary step with every participating key selected;
    /// otherwise at [`Stage::ParseError`].
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        &mut self,
        bytes: &[u8],
        keys: Vec<KeyRef>,
        wallets: Vec<WalletPolicy>,
        shares: Vec<ShareRef>,
        network: Network,
        session: Option<&MusigSessionView>,
    ) {
        // A carry file is a PSBT with the secret nonces of the signers
        // still to sign bound to it (§16.103). It reaches Sign the way a
        // PSBT does; what it adds is the section.
        let (carry, body) = match Carry::looks_like_carry(bytes) {
            false => (None, bytes.to_vec()),
            true => match Carry::parse(bytes) {
                Ok(carry) => (Some(carry.section), carry.psbt),
                Err(e) => {
                    self.clear();
                    self.error = Some(e.to_string());
                    self.stage = Stage::ParseError;
                    return;
                }
            },
        };
        match parse_psbt(&body) {
            Ok(psbt) => {
                let ctx = Context {
                    network,
                    keys: &keys,
                    wallets: &wallets,
                    musig_session: session,
                    shares: &shares,
                    carry: carry.as_ref(),
                };
                let inspection = osk_psbt::inspect(&psbt, &ctx);
                self.selected = inspection.participating_keys.clone();
                self.output = 0;
                self.signatures = inspection.signatures.clone();
                self.determinism.clear();
                self.checked_warnings.clear();
                self.signatures_from = None;
                self.inspection = Some(inspection);
                self.psbt = Some(psbt);
                self.keys = keys;
                self.wallets = wallets;
                self.shares = shares;
                self.carry = carry;
                self.others.clear();
                self.network = network;
                self.acknowledged = false;
                self.error = None;
                self.outcome = None;
                self.stage = Stage::Wizard(Step::Summary);
            }
            Err(e) => {
                self.clear();
                self.error = Some(e.to_string());
                self.stage = Stage::ParseError;
            }
        }
    }

    /// Inspects the loaded PSBT again, with the keys and wallets this
    /// device holds now (`docs/PLANNING.md` §16.110 rule 3).
    ///
    /// A key added from the transaction's Keys review is a key the
    /// transaction may name, so the review it comes back to has to be
    /// the review of the same transaction read again: the step, the
    /// output on screen and everything else the person has walked stays
    /// where it was.
    pub fn reinspect(
        &mut self,
        keys: Vec<KeyRef>,
        wallets: Vec<WalletPolicy>,
        shares: Vec<ShareRef>,
        network: Network,
        session: Option<&MusigSessionView>,
    ) {
        let Some(psbt) = self.psbt.as_ref() else {
            return;
        };
        let ctx = Context {
            network,
            keys: &keys,
            wallets: &wallets,
            musig_session: session,
            shares: &shares,
            carry: self.carry.as_ref(),
        };
        let inspection = osk_psbt::inspect(psbt, &ctx);
        self.selected = inspection.participating_keys.clone();
        self.signatures = inspection.signatures.clone();
        self.inspection = Some(inspection);
        self.keys = keys;
        self.wallets = wallets;
        self.shares = shares;
        self.network = network;
    }

    /// How many shares must sign, when a threshold input of this
    /// device's is at the first location and the signer set is still to
    /// be chosen. `None` at every other location and for every other
    /// transaction.
    pub fn threshold_first(&self) -> Option<usize> {
        let insp = self.inspection.as_ref()?;
        let ti = insp
            .inputs
            .iter()
            .filter_map(|i| i.threshold.as_ref())
            .find(|t| !t.ours.is_empty())?;
        ti.rounds
            .contains(&ThresholdRound::First)
            .then_some(ti.info.t)
    }

    /// The record's other shares, by identifier, which is what the
    /// "Then with" row offers at the first location.
    pub fn threshold_candidates(&self) -> Vec<u32> {
        let Some(insp) = self.inspection.as_ref() else {
            return Vec::new();
        };
        let Some(ti) = insp
            .inputs
            .iter()
            .filter_map(|i| i.threshold.as_ref())
            .find(|t| !t.ours.is_empty())
        else {
            return Vec::new();
        };
        (0..ti.info.n() as u32)
            .filter(|id| !ti.ours.contains(id))
            .collect()
    }

    /// The others chosen to sign, for the shells and the tests.
    pub fn others(&self) -> &[u32] {
        &self.others
    }

    /// Whether share `id` is one of the others chosen to sign.
    pub fn is_other(&self, id: u32) -> bool {
        self.others.contains(&id)
    }

    /// Flips whether share `id` is one of them.
    pub fn toggle_other(&mut self, id: u32) {
        match self.others.iter().position(|o| *o == id) {
            Some(at) => {
                self.others.remove(at);
            }
            None => self.others.push(id),
        }
    }

    /// Whether this device has anything that can sign this transaction:
    /// a loaded key that is a participant, or a share of the threshold
    /// wallet it spends.
    pub fn participates(&self) -> bool {
        self.inspection
            .as_ref()
            .is_some_and(|i| !i.participating_keys.is_empty())
            || self.threshold().is_some_and(|t| !t.ours.is_empty())
    }

    /// The threshold wallet this transaction spends, when it spends one.
    pub fn threshold(&self) -> Option<&osk_psbt::ThresholdSummary> {
        self.inspection.as_ref()?.threshold.as_ref()
    }

    /// Moves to `step`.
    pub fn go(&mut self, step: Step) {
        self.stage = Stage::Wizard(step);
    }

    /// One step back. Returns `false` when there is nothing before the
    /// current stage inside the flow, so the caller leaves the screen.
    pub fn back(&mut self) -> bool {
        match self.stage {
            // There is no entry screen behind a transaction: it arrived,
            // and the way back from the first thing it shows is off the
            // screen entirely.
            Stage::Entry
            | Stage::Waiting
            | Stage::Unavailable
            | Stage::ParseError
            | Stage::Wizard(Step::Summary | Step::Result) => false,
            // Inside the outputs run the chevron walks back through it.
            Stage::Wizard(Step::Outputs) if self.output > 0 => {
                self.output -= 1;
                true
            }
            Stage::Wizard(
                step @ (Step::Outputs | Step::Inputs | Step::Warnings | Step::Confirm),
            ) => {
                if let Some(prev) = self.before(step) {
                    if prev == Step::Outputs {
                        self.output = self.outputs().saturating_sub(1);
                    }
                    self.stage = Stage::Wizard(prev);
                }
                true
            }
            Stage::Wizard(Step::Signatures) => {
                let back = self.signatures_from.take().unwrap_or(Step::Result);
                self.stage = Stage::Wizard(back);
                true
            }
            Stage::Wizard(Step::Qr) => {
                self.qr = None;
                self.stage = Stage::Wizard(Step::Result);
                true
            }
        }
    }

    /// Flips whether participating key number `i` signs.
    pub fn toggle_key(&mut self, i: usize) {
        let Some(fp) = self
            .inspection
            .as_ref()
            .and_then(|insp| insp.participating_keys.get(i).copied())
        else {
            return;
        };
        match self.selected.iter().position(|f| *f == fp) {
            Some(p) => {
                self.selected.remove(p);
            }
            None => self.selected.push(fp),
        }
    }

    /// Flips the risk acknowledgement.
    pub fn toggle_acknowledged(&mut self) {
        self.acknowledged = !self.acknowledged;
    }

    /// Signs with the selected keys, taken from `masters` by fingerprint,
    /// then finalizes when every input is complete. Danger warnings are
    /// overridden only when acknowledged, and a blocked warning is never
    /// overridden — [`SignFlow::can_sign`] is false while one stands. On
    /// success the flow is at the result step; on failure it stays at
    /// Confirm with the error set.
    pub fn sign(
        &mut self,
        masters: &[&MasterKey],
        shares: &[ShareKey],
        nonce: osk_psbt::Nonce,
        aux: osk_psbt::Aux,
        session: &mut Option<MusigSession>,
        seed: [u8; 32],
    ) {
        if !self.can_sign() {
            return;
        }
        let Some(psbt) = self.psbt.as_mut() else {
            return;
        };
        let view = session.as_ref().map(|s| s.view());
        let ctx = Context {
            network: self.network,
            keys: &self.keys,
            wallets: &self.wallets,
            musig_session: view.as_ref(),
            shares: &self.shares,
            carry: self.carry.as_ref(),
        };
        let force = self.acknowledged;
        let result = match osk_psbt::sign(
            psbt,
            masters,
            &self.selected,
            &ctx,
            force,
            nonce,
            aux,
            session,
            seed,
            shares,
            &self.others,
        ) {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        // The transaction now carries this device's own signatures, and
        // the page lists what is on it. They are read back here, before
        // finalization clears the key origins that name them: a
        // finalized witness says which key signed, not which master it
        // came from (§16.111).
        let view = session.as_ref().map(|s| s.view());
        let ctx = Context {
            network: self.network,
            keys: &self.keys,
            wallets: &self.wallets,
            musig_session: view.as_ref(),
            shares: &self.shares,
            carry: self.carry.as_ref(),
        };
        self.signatures = osk_psbt::verify::verify_signatures(psbt, &ctx);
        self.determinism = osk_psbt::verify::deterministic(psbt, masters);
        drop(view);

        let (complete, txid, bytes, name_hint) = if result.complete {
            match osk_psbt::finalize(psbt) {
                Ok(Some(tx)) => (
                    true,
                    Some(tx.compute_txid()),
                    text::hex(&serialize(&tx)).into_bytes(),
                    "signed.txn",
                ),
                Ok(None) => (false, None, psbt.to_bytes(), "signed.psbt"),
                Err(e) => {
                    self.error = Some(e.to_string());
                    return;
                }
            }
        } else {
            (false, None, psbt.to_bytes(), "signed.psbt")
        };
        // A pass that leaves signers still to sign hands out the carry
        // file, not the PSBT: the secret nonce travels with the
        // transaction it is bound to, in one file (§16.103).
        let threshold = result.threshold.as_ref().map(|t| (t.t, t.partial_sigs));
        let carry = result.threshold.and_then(|t| t.carry);
        let (bytes, name_hint) = match &carry {
            Some(section) => (carry_bytes(section, &psbt.to_bytes()), CARRY_NAME),
            None => (bytes, name_hint),
        };
        self.outcome = Some(Outcome {
            threshold,
            carry,
            signatures: result.signed_inputs,
            nonces_shared: result.nonces_shared,
            complete,
            txid,
            bytes,
            psbt: psbt.to_bytes(),
            name_hint: String::from(name_hint),
            save: Save::Idle,
        });
        self.error = None;
        self.stage = Stage::Wizard(Step::Result);
    }

    /// Marks a write as out, waiting for the shell's answer.
    pub fn mark_saving(&mut self) {
        self.set_save(Save::Waiting);
    }

    /// Marks the bytes as stored: the shell answered `FileWritten`.
    pub fn mark_saved(&mut self) {
        self.set_save(Save::Written);
    }

    /// Marks the save as not made: the shell answered `FileNotWritten`.
    pub fn mark_not_saved(&mut self) {
        self.set_save(Save::Failed);
    }

    fn set_save(&mut self, save: Save) {
        if let Some(o) = self.outcome.as_mut() {
            o.save = save;
        }
    }

    /// Drops everything loaded and returns to the entry screen.
    pub fn clear(&mut self) {
        self.psbt = None;
        self.inspection = None;
        self.keys.clear();
        self.wallets.clear();
        self.shares.clear();
        self.carry = None;
        self.others.clear();
        self.selected.clear();
        self.output = 0;
        self.acknowledged = false;
        self.error = None;
        self.outcome = None;
        self.qr = None;
        self.static_scans = false;
        self.signatures.clear();
        self.determinism.clear();
        self.checked_warnings.clear();
        self.signatures_from = None;
        self.stage = Stage::Entry;
    }
}

/// Parses a PSBT from raw file content: binary when it starts with the
/// magic, base64 text otherwise. Whitespace around either is ignored.
pub fn parse_psbt(bytes: &[u8]) -> Result<Psbt, Error> {
    let trimmed = bytes.trim_ascii();
    if trimmed.starts_with(b"psbt\xff") {
        return Psbt::parse_bytes(trimmed);
    }
    match core::str::from_utf8(trimmed) {
        Ok(text) => Psbt::parse_base64(text),
        Err(_) => Err(Error::NotPsbt),
    }
}
