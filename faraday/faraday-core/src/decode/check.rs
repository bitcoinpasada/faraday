//! Check a signer: the flow's state and what it decides without the app
//! (`docs/SIGNER-CHECK.md` §3, §8, §11). The presses that need the app are
//! in the area's `mod.rs`; the cards are drawn by `check_screen.rs`.
//!
//! Everything kept here is public data (two PSBTs or transactions, the
//! comparison and the report on them), kept until a lock or power-off as
//! Decode's state is.

use crate::decode::{check_others, diff_words};
use crate::strings::EN;
use crate::wallet::{InspectInputs, PsbtSeen};
use crate::{Item, Screen, flow};
use faraday_bip::base64;
use faraday_bip::bitcoin::{OutPoint, Txid};
use faraday_bip::keys::{Fingerprint, Network};
use faraday_psbt::bump::{Bump, Line, Outcome};
use faraday_psbt::diff::Differences;
use faraday_psbt::signer::{Report, Verdict};
use faraday_psbt::{Inspection, Level, OutputKind, Psbt};
use faraday_text::{Text, text};
use zeroize::Zeroizing;

/// The two cards that choose a PSBT or a finished transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// The PSBT that went to the device.
    Sent,
    /// The PSBT that came back.
    Returned,
}

impl Slot {
    /// The card's place in the flow: Sent, Returned, then the Result.
    pub(crate) fn card(self) -> u8 {
        match self {
            Slot::Sent => 0,
            Slot::Returned => 1,
        }
    }
}

/// The card the Result is on.
pub(crate) const RESULT: u8 = 2;

/// A press on Check a signer (`Screen::SignerCheck`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerPress {
    /// The Tools tile: the flow opens, closed on its default where
    /// Files has one.
    Open,
    /// A card's header or its **Change**.
    Card(u8),
    /// The file in this place of the list the card shows, chosen for
    /// this card.
    Pick(Slot, usize),
    /// This card's **Scan**.
    Scan(Slot),
    /// The way back to where the flow was opened.
    Back,
    /// The Result's **Show as QR**: the Returned PSBT.
    Qr,
    /// The Result's **Swap**: the two exchanged and checked again.
    Swap,
    /// The Result's **Choose again**: the Returned card opens.
    Again,
    /// The Result's **Load {fp}** for the key in this place of
    /// [`Shown::load`]: Add a key opens, and its end comes back here.
    Load(usize),
    /// The Result's **Load the seed** for the key in this place of
    /// [`Shown::seeds`]: the sheet of ways to load it opens.
    Seed(usize),
    /// The sheet's **Type the words**: Add a key's form for the key, its
    /// way back this screen.
    SeedWords,
    /// The sheet's **Scan a SeedQR**: the same form with its scanner open.
    SeedScan,
    /// The sheet's Other forms: one form of the table, by its code, for the same form to type its key in.
    SeedForm(u8),
    /// The sheet's **Open a vault**: the key loads from the open vault
    /// that holds it, else a vault is unlocked to return here.
    SeedVault,
    /// Files' ⋮ **Check a signer** on this PSBT or finished transaction
    /// (From the stick, or For the stick when the first is true; its
    /// place in that list): the flow opens with it as Returned.
    File(bool, usize),
    /// Sign a transaction's review **Compare** on the other transaction
    /// in this file (as [`SignerPress::File`] names one): the flow opens
    /// with that file as Sent and the PSBT under review as Returned, and
    /// has run.
    FromReview(bool, usize),
}

/// Where a PSBT came from: a file in Files (From the stick, or For the
/// stick when `outbox`), or a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Origin {
    File { outbox: bool, name: Text },
    Scanned,
}

/// One side of the check: the file as the person has it, its version 0
/// form and its name.
pub(crate) struct Side {
    /// The file's bytes: for a PSBT as the person has them (raw, base64
    /// decoded where the file is text, [`raw_bytes`]), for a finished
    /// transaction as Files holds them (raw or hex).
    pub bytes: Zeroizing<Vec<u8>>,
    /// Its version 0 form (a version 2 file through `PsbtV2::to_v0`; for a
    /// finished transaction, the PSBT form of what it carries).
    pub psbt: Psbt,
    /// What the closed card shows: the file's name, or "Scanned".
    pub name: Text,
    /// Where it came from.
    pub origin: Origin,
    /// Whether Files holds no file of this name, in either box, with these
    /// bytes any more: the copy and its report are kept. Worked out by
    /// `Faraday::signer_check_sync` while the flow is on screen and on
    /// arriving at it, and made false again by a pick, so it is true
    /// of Files as it stands whenever the flow is drawn.
    pub differs: bool,
    /// Its unsigned transaction.
    pub txid: Txid,
    /// The PSBT's own version, 0 or 2; `None` for a finished transaction.
    pub version: Option<u8>,
    /// The outpoints it spends, sorted.
    pub outpoints: Vec<OutPoint>,
}

/// The flow's state.
pub(crate) struct CheckState {
    /// The PSBT that went to the device.
    pub sent: Option<Side>,
    /// The PSBT that came back.
    pub returned: Option<Side>,
    /// The check of the two PSBTs kept above, against the keys loaded:
    /// `Some` where both are PSBTs. Invariant: `report` and `shown` are
    /// `Some` only for the very `sent` and `returned` held now; they are
    /// replaced (or dropped) whenever either side changes (a pick, a scan,
    /// a swap) and whenever a key or wallet is loaded or dropped
    /// (SIGNER-CHECK §8.2).
    pub report: Option<Report>,
    /// The whole Result in words (the comparison, the fee bump and the
    /// report), worked out with the report and kept beside it, so drawing
    /// reads text and works nothing out. `Some` once both sides are in.
    pub shown: Option<Shown>,
    /// The keys and wallets the result was made with: a press that leaves
    /// others makes it again (`Faraday::signer_check_sync`).
    pub made_with: Option<InspectInputs>,
    /// The ids of the finished transactions in Files that the lists show,
    /// worked out beside the file's bytes by `Faraday::signer_check_sync`,
    /// so drawing reads them.
    pub ids: Vec<(Zeroizing<Vec<u8>>, Txid)>,
    /// The card open.
    pub open: u8,
    /// Why a card refused what was given it.
    pub refused: [Option<Text>; 2],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// Where the flow was opened from.
    pub back: Screen,
    /// The key whose seed the sheet of ways to load it is for, or that a
    /// way is loading.
    pub seed: Option<Fingerprint>,
}

impl CheckState {
    pub(crate) fn new(back: Screen) -> CheckState {
        CheckState {
            sent: None,
            returned: None,
            report: None,
            shown: None,
            made_with: None,
            ids: Vec::new(),
            open: 0,
            refused: [None, None],
            scroll: flow::Scroll {
                y: 0.0,
                follow: true,
            },
            back,
            seed: None,
        }
    }

    pub(crate) fn side(&self, slot: Slot) -> Option<&Side> {
        match slot {
            Slot::Sent => self.sent.as_ref(),
            Slot::Returned => self.returned.as_ref(),
        }
    }

    /// Puts `side` on `slot`, drops the report, and opens the card that
    /// follows: the next one asked, or the Result once both are in
    /// (the caller runs the check then). Returns whether both are in.
    pub(crate) fn put(&mut self, slot: Slot, side: Side) -> bool {
        self.report = None;
        self.shown = None;
        self.refused[usize::from(slot.card())] = None;
        match slot {
            Slot::Sent => {
                // The Returned file may not be the Sent one.
                if self
                    .returned
                    .as_ref()
                    .is_some_and(|r| r.origin != Origin::Scanned && r.origin == side.origin)
                {
                    self.returned = None;
                }
                self.sent = Some(side);
            }
            Slot::Returned => self.returned = Some(side),
        }
        self.scroll.follow = true;
        let both = self.sent.is_some() && self.returned.is_some();
        self.open = if both {
            RESULT
        } else if slot == Slot::Sent {
            Slot::Returned.card()
        } else {
            Slot::Sent.card()
        };
        both
    }

    /// Sent and Returned exchanged; the report goes, and the caller
    /// checks again.
    pub(crate) fn swap(&mut self) {
        std::mem::swap(&mut self.sent, &mut self.returned);
        self.refused = [None, None];
        self.report = None;
        self.shown = None;
    }

    /// The card is opened.
    pub(crate) fn open_card(&mut self, card: u8) {
        if card <= RESULT {
            if let Some(r) = self.refused.get_mut(usize::from(card)) {
                *r = None;
            }
            self.open = card;
            self.scroll.follow = true;
        }
    }

    /// A finished transaction's id as `signer_check_sync` kept it.
    fn id_of(&self, bytes: &[u8]) -> Option<Txid> {
        self.ids
            .iter()
            .find(|(b, _)| b.as_slice() == bytes)
            .map(|(_, t)| *t)
    }

    /// A file's line in the lists: "{n} signatures · {id}" for a PSBT,
    /// "Finished · {id}" for a finished transaction. Reads what the file
    /// and `signer_check_sync` kept.
    pub(crate) fn line(&self, item: &Item) -> Text {
        if let Some(p) = item.psbt() {
            let id = faraday_ui::short(&text!("{}", p.txid));
            return if p.sigs == 1 {
                Text::fill(EN.signer_row_one, &[&id])
            } else {
                Text::fill(EN.signer_row, &[&Text::num(p.sigs as u64), &id])
            };
        }
        self.id_of(&item.bytes).map_or_else(Text::new, |t| {
            Text::fill(
                EN.compare_row_finished,
                &[&faraday_ui::short(&text!("{t}"))],
            )
        })
    }
}

/// The files the cards list: every PSBT and every finished transaction
/// in Files, From the stick then For the stick, as (outbox, place, item).
pub(crate) fn files<'a>(inbox: &'a [Item], outbox: &'a [Item]) -> Vec<(bool, usize, &'a Item)> {
    let of = |items: &'a [Item], out: bool| {
        items
            .iter()
            .enumerate()
            .filter(|(_, i)| {
                matches!(
                    i.kind,
                    crate::wallet::FileKind::Psbt | crate::wallet::FileKind::Transaction
                )
            })
            .map(|(k, i)| (out, k, i))
            .collect::<Vec<_>>()
    };
    let mut all = of(inbox, false);
    all.extend(of(outbox, true));
    all
}

/// The rows of one card: the place in [`files`] of each. First the files
/// that spend an input of the other card's file, those of its own
/// unsigned transaction first; then the rest, under "Other
/// transactions". The file chosen as the other card is not listed. The
/// split is where the label goes.
pub(crate) fn rows(
    all: &[(bool, usize, &Item)],
    slot: Slot,
    (sent, returned): (Option<&Side>, Option<&Side>),
) -> (Vec<usize>, Vec<usize>) {
    let beside = match slot {
        Slot::Returned => sent,
        Slot::Sent => returned,
    };
    let Some(s) = beside else {
        return ((0..all.len()).collect(), Vec::new());
    };
    let (mut own, mut spends, mut other) = (Vec::new(), Vec::new(), Vec::new());
    for (n, (outbox, _, item)) in all.iter().enumerate() {
        // A file of the other side's name that is not that copy is a file
        // of its own, listed.
        if !s.differs
            && s.origin
                == (Origin::File {
                    outbox: *outbox,
                    name: Text::of(&item.name),
                })
        {
            continue;
        }
        if item.unsigned_txid == Some(s.txid) {
            own.push(n);
        } else if check_others::shares(&item.outpoints, &s.outpoints) {
            spends.push(n);
        } else {
            other.push(n);
        }
    }
    own.extend(spends);
    (own, other)
}

/// The default of SIGNER-CHECK §3, "Which is which": exactly two PSBTs
/// in Files share an unsigned transaction and one carries fewer
/// signatures. Gives the places in [`files`] of Sent and Returned. A
/// lookup over what each file's `PsbtSeen` kept.
pub(crate) fn default_pair(all: &[(bool, usize, &Item)]) -> Option<(usize, usize)> {
    let seen = |n: usize| -> Option<&PsbtSeen> { all[n].2.psbt() };
    let mut found: Option<(usize, usize)> = None;
    for a in 0..all.len() {
        let Some(ta) = seen(a).map(|p| p.txid) else {
            continue;
        };
        let group: Vec<usize> = (0..all.len())
            .filter(|n| seen(*n).is_some_and(|p| p.txid == ta))
            .collect();
        if group.len() < 2 || group[0] != a {
            continue;
        }
        // A second transaction with several files has no default.
        if found.is_some() || group.len() != 2 {
            return None;
        }
        found = Some((group[0], group[1]));
    }
    let (a, b) = found?;
    let (sa, sb) = (seen(a)?.sigs, seen(b)?.sigs);
    match sa.cmp(&sb) {
        std::cmp::Ordering::Less => Some((a, b)),
        std::cmp::Ordering::Greater => Some((b, a)),
        std::cmp::Ordering::Equal => None,
    }
}

/// Costs a base64 decode for a text file whose size allows the same.
/// Whether a file's `item` bytes are the `kept` bytes of [`raw_bytes`].
pub(crate) fn same_bytes(item: &[u8], kept: &[u8]) -> bool {
    if item == kept {
        return true;
    }
    if item.starts_with(b"psbt\xff") {
        return false;
    }
    // Base64 of n characters decodes to at most n / 4 * 3 bytes and at
    // least two fewer.
    let n = item.trim_ascii().len();
    if kept.len() > n / 4 * 3 || kept.len() + 2 < n / 4 * 3 {
        return false;
    }
    *raw_bytes(item) == kept
}

/// Costs a base64 decode for a text file. The file's bytes as the
/// person has them: raw, base64 decoded where the file is text.
pub(crate) fn raw_bytes(bytes: &[u8]) -> Zeroizing<Vec<u8>> {
    if bytes.starts_with(b"psbt\xff") {
        return Zeroizing::new(bytes.to_vec());
    }
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| base64::decode(t.trim()).ok())
        .unwrap_or_else(|| Zeroizing::new(bytes.to_vec()))
}

/// What the verdict line says, and in which tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    Ok,
    Warn,
    Err,
}

/// The verdict as words (SIGNER-CHECK §5) and its tone.
pub(crate) fn verdict_line(report: &Report) -> (Text, Tone) {
    match report.verdict {
        Verdict::Clean { finalized, nonces } => {
            let text = if nonces {
                Text::of(EN.signer_only_nonces)
            } else if finalized == 1 {
                Text::of(EN.signer_only_one_finalized)
            } else if finalized > 1 {
                Text::fill(EN.signer_only_finalized, &[&Text::num(finalized as u64)])
            } else {
                Text::of(EN.signer_only_signatures)
            };
            (text, Tone::Ok)
        }
        Verdict::Findings { count, worst } => {
            let text = if count == 1 {
                Text::of(EN.signer_one_finding)
            } else {
                Text::fill(EN.signer_findings, &[&Text::num(count as u64)])
            };
            let tone = if worst >= Level::Danger {
                Tone::Err
            } else {
                Tone::Warn
            };
            (text, tone)
        }
        Verdict::NotSameTransaction => (Text::of(EN.signer_not_the_same), Tone::Err),
    }
}

impl BumpShown {
    /// The result's tone: Only the fee is raised in `OK`, More than the
    /// fee changed in `ERR`, Cannot tell in `WARN` (COMPARE §3.3).
    pub(crate) fn tone(&self) -> Tone {
        match self.outcome {
            Outcome::OnlyFee => Tone::Ok,
            Outcome::MoreChanged => Tone::Err,
            Outcome::CannotTell => Tone::Warn,
        }
    }
}

/// The fee bump result in words.
pub(crate) struct BumpShown {
    /// What it came to; the screen tones it.
    pub outcome: Outcome,
    /// The result line.
    pub result: &'static str,
    /// The lines, in the rules' order.
    pub lines: Vec<Text>,
    /// The facts under them.
    pub facts: Vec<diff_words::Fact>,
}

/// The facts of the Result: the check's where it ran on one unsigned
/// transaction, else the comparison's.
pub(crate) enum Facts {
    /// Label and value, one fact each (SIGNER-CHECK §4).
    Check(Vec<(Text, Text)>),
    /// The comparison's: the transaction, the fees, the counts.
    Compare(Vec<diff_words::Fact>),
}

/// The Result card's text, in the order it is drawn (SIGNER-CHECK §11.5).
pub(crate) struct Shown {
    /// The fee bump result, where it applies.
    pub bump: Option<BumpShown>,
    /// The verdict line and its tone; `None` where a side is a finished
    /// transaction ("The signer check needs two PSBTs").
    pub verdict: Option<(Text, Tone)>,
    /// The facts.
    pub facts: Facts,
    /// A line per finding in the report's order, with its level.
    pub findings: Vec<(Level, Text)>,
    /// The lines stated, which are no finding.
    pub stated: Vec<Text>,
    /// Whether the findings section is drawn: where the check ran, not
    /// where the two PSBTs share no input.
    pub has_findings: bool,
    /// A line per difference, or "No differences".
    pub differences: Vec<Text>,
    /// Whether **Show as QR** is offered: the Returned side is a PSBT.
    pub qr: bool,
    /// The keys that signed and are not loaded, one **Load the seed** row
    /// each (SIGNER-CHECK §12).
    pub seeds: Vec<Fingerprint>,
    /// The keys only the fee bump names and that are not loaded, one
    /// **Load** each.
    pub load: Vec<Fingerprint>,
}

/// The rows for the nonces the signer's file recomputed.
fn nonce_facts(facts: &mut Vec<(Text, Text)>, r: &faraday_psbt::signer::Recomputed) {
    let label = EN.signer_nonces_recomputed;
    let counts = Text::fill(
        EN.signer_nonces_of,
        &[&Text::num(r.recomputed as u64), &Text::num(r.of as u64)],
    );
    if r.recomputed == 0 && !r.not_loaded.is_empty() {
        for fp in &r.not_loaded {
            facts.push((
                Text::of(label),
                Text::fill(EN.signer_nonces_none, &[&text!("{fp}")]),
            ));
        }
    } else {
        facts.push((Text::of(label), counts));
        for fp in &r.not_loaded {
            facts.push((
                Text::of(label),
                Text::fill(EN.signer_nonces_not_loaded, &[&text!("{fp}")]),
            ));
        }
    }
}

/// The facts of the check where the two are one unsigned transaction,
/// `txid`, with the Returned PSBT's `inspection` for what it sends, its
/// fee and its change (SIGNER-CHECK §4).
fn check_facts(report: &Report, txid: &Txid, inspection: &Inspection) -> Vec<(Text, Text)> {
    let mut facts = vec![(
        Text::of(EN.signer_transaction),
        faraday_ui::grouped(&text!("{txid}")),
    )];
    let f = &report.facts;
    facts.push((
        Text::of(EN.signer_signatures_added),
        text!("{}", f.added.len() + f.partial_sigs_added),
    ));
    if !f.finalized.is_empty() {
        facts.push((
            Text::of(EN.signer_inputs_finalized),
            text!("{}", f.finalized.len()),
        ));
    }
    if let Some(r) = &f.recomputed
        && (r.of > 0 || !r.not_loaded.is_empty())
    {
        nonce_facts(&mut facts, r);
    }
    facts.push((
        Text::of(EN.step_sends),
        super::btc(inspection.amount_to_others.to_sat()),
    ));
    facts.push((
        Text::of(EN.step_fee),
        if inspection.fee_rate_sat_vb.is_some() {
            super::btc(inspection.fee.to_sat())
        } else {
            Text::of(EN.step_fee_not_known)
        },
    ));
    // Verified where an output is change on the change chain and none
    // claims a key it could not verify.
    let verified = inspection.outputs.iter().any(|o| {
        matches!(
            o.kind,
            OutputKind::Change { change: true, .. } | OutputKind::WalletChange { change: true, .. }
        )
    }) && !inspection
        .outputs
        .iter()
        .any(|o| matches!(o.kind, OutputKind::UnverifiedChange { .. }));
    facts.push((
        Text::of(EN.step_change),
        Text::of(if verified {
            EN.signer_change_verified
        } else {
            EN.signer_change_not_verified
        }),
    ));
    facts
}

/// Costs the text and nothing beyond it; the inspection is the caller's.
/// The comparison `d`, the fee bump where it applies and, where both
/// sides are PSBTs, the check's `report` with the Returned PSBT's
/// `inspection`, in words (SIGNER-CHECK §11.5). `qr` is whether the
/// Returned side is a PSBT.
pub(crate) fn shown(
    d: &Differences,
    bump: Option<&Bump>,
    checked: Option<(&Report, &Inspection)>,
    network: Network,
    qr: bool,
) -> Shown {
    use super::check_words as words;
    let seeds: Vec<Fingerprint> = checked
        .and_then(|(r, _)| r.facts.recomputed.as_ref())
        .map_or_else(Vec::new, |r| r.not_loaded.clone());
    let mut load: Vec<Fingerprint> = Vec::new();
    for l in bump.map_or(&[][..], |b| &b.lines[..]) {
        if let Line::ChangeNotLoaded { key, .. } | Line::InputNotLoaded { key, .. } = l
            && !seeds.contains(key)
            && !load.contains(key)
        {
            load.push(*key);
        }
    }
    let facts = match checked {
        Some((report, inspection)) if d.same_transaction() => {
            Facts::Check(check_facts(report, &d.first_txid, inspection))
        }
        _ => Facts::Compare(diff_words::compare_facts(d)),
    };
    let mut findings = Vec::new();
    let mut stated = Vec::new();
    let mut has_findings = false;
    if let Some((report, _)) = checked {
        findings = report
            .findings
            .iter()
            .map(|f| {
                let line = words::finding(f, network);
                (
                    f.level,
                    Text::fill(EN.signer_finding_line, &[words::level_word(f.level), &line]),
                )
            })
            .collect();
        stated = report
            .info
            .iter()
            .map(|i| Text::of(&words::stated(i)))
            .collect();
        has_findings = report.verdict != Verdict::NotSameTransaction;
    }
    Shown {
        bump: bump.map(|b| BumpShown {
            outcome: b.result,
            result: diff_words::result(b.result),
            lines: b
                .lines
                .iter()
                .map(|l| Text::of(&diff_words::line(l)))
                .collect(),
            facts: diff_words::bump_facts(b),
        }),
        verdict: checked.map(|(r, _)| verdict_line(r)),
        facts,
        findings,
        stated,
        has_findings,
        differences: diff_words::differences(d)
            .iter()
            .map(|l| Text::of(l))
            .collect(),
        qr,
        seeds,
        load,
    }
}

/// Check a signer's cards, plugged into Back and Forward (F104).
pub(crate) struct SignerSteps;

/// The one [`SignerSteps`].
pub(crate) static SIGNER_STEPS: SignerSteps = SignerSteps;

impl crate::trail::StepFlow for SignerSteps {
    fn steps(&self, app: &crate::Faraday) -> Option<crate::trail::Steps> {
        let s = app.signer_check()?;
        let mut steps = crate::trail::Steps::new((0..=RESULT).collect(), Some(s.open));
        steps.answered = (0..=RESULT)
            .filter(|&k| match k {
                0 => s.sent.is_some(),
                1 => s.returned.is_some(),
                _ => s.shown.is_some(),
            })
            .collect();
        Some(steps)
    }

    fn open(&self, _: &crate::Faraday, k: u8) -> crate::Action {
        crate::Action::SignerCheck(SignerPress::Card(k))
    }

    fn name(&self, _: &crate::Faraday, k: u8) -> &'static str {
        match k {
            0 => EN.signer_sent,
            1 => EN.signer_returned,
            _ => EN.signer_result,
        }
    }

    fn contained(&self) -> Option<&dyn crate::flow::Contained> {
        Some(self)
    }
}

impl crate::flow::Contained for SignerSteps {
    fn rows(&self, app: &crate::Faraday) -> Vec<crate::flow::Row> {
        let Some(s) = app.signer_check() else {
            return Vec::new();
        };
        (0..=RESULT)
            .map(|k| {
                let side = match k {
                    0 => s.side(Slot::Sent),
                    1 => s.side(Slot::Returned),
                    _ => None,
                };
                let value = match side {
                    Some(d) if d.differs => Some(Text::fill(EN.signer_file_differs, &[&d.name])),
                    Some(d) => Some(d.name.clone()),
                    None => None,
                };
                crate::flow::Row {
                    label: faraday_text::Text::of(crate::trail::StepFlow::name(self, app, k)),
                    value,
                    mono: false,
                    opens: k,
                    current: s.open == k,
                    mark: None,
                }
            })
            .collect()
    }

    fn hide_shown(&self, app: &mut crate::Faraday) {
        // No step of this flow shows a secret.
        let _ = app;
    }

    fn count_fixed(&self, _: &crate::Faraday) -> bool {
        true
    }

    /// Choosing a file again drops the report and so the Result; the
    /// other file stays.
    fn still_fits(&self, _: &crate::Faraday, _changed: u8, later: u8) -> bool {
        later < RESULT
    }
}
