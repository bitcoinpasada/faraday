//! Check a signer: the flow's state and what it decides without the app
//! (`docs/SIGNER-CHECK.md` §3, §8). The presses that need the app are in
//! the area's `mod.rs`; the cards are drawn by `check_screen.rs`.
//!
//! Everything kept here is public data (two PSBTs and the report on
//! them), kept until a lock or power-off as Decode's state is.

use crate::strings::{EN, fill};
use crate::wallet::{InspectInputs, PsbtSeen};
use crate::{Item, Screen, flow};
use faraday_bip::base64;
use faraday_bip::bitcoin::Txid;
use faraday_bip::keys::{Fingerprint, Network};
use faraday_psbt::signer::{Report, Verdict};
use faraday_psbt::{Inspection, Level, OutputKind, Psbt};
use zeroize::Zeroizing;

/// The two cards that choose a PSBT.
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
    /// Files' ⋮ **Check a signer** on this PSBT (From the stick, or For
    /// the stick when the first is true; its place in that list): the
    /// flow opens with it as Returned.
    File(bool, usize),
}

/// Where a PSBT came from: a file in Files (From the stick, or For the
/// stick when `outbox`), or a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Origin {
    File { outbox: bool, name: String },
    Scanned,
}

/// One side of the check: the file as the person has it, its version 0
/// form and its name.
pub(crate) struct Side {
    /// The file's bytes as the person has them: raw, base64 decoded
    /// where the file is text.
    pub bytes: Zeroizing<Vec<u8>>,
    /// Its version 0 form (a version 2 file through `PsbtV2::to_v0`).
    pub psbt: Psbt,
    /// What the closed card shows: the file's name, or "Scanned".
    pub name: String,
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
}

/// The flow's state.
pub(crate) struct CheckState {
    /// The PSBT that went to the device.
    pub sent: Option<Side>,
    /// The PSBT that came back.
    pub returned: Option<Side>,
    /// The check of the two PSBTs kept above, against the keys loaded.
    /// Invariant: `report` is `Some` only for the very `sent` and
    /// `returned` held now; it is replaced (or dropped) whenever either
    /// PSBT changes (a pick, a scan) and whenever a key is loaded or
    /// dropped (SIGNER-CHECK §8.2).
    pub report: Option<Report>,
    /// The report in words, worked out with it and kept beside it, so
    /// drawing reads text and works nothing out. `Some` exactly where
    /// `report` is.
    pub shown: Option<Shown>,
    /// The keys and wallets the report was made with: a press that leaves
    /// others makes it again (`Faraday::signer_check_sync`).
    pub made_with: Option<InspectInputs>,
    /// The card open.
    pub open: u8,
    /// Why a card refused what was given it.
    pub refused: [Option<String>; 2],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// Where the flow was opened from.
    pub back: Screen,
}

impl CheckState {
    pub(crate) fn new(back: Screen) -> CheckState {
        CheckState {
            sent: None,
            returned: None,
            report: None,
            shown: None,
            made_with: None,
            open: 0,
            refused: [None, None],
            scroll: flow::Scroll {
                y: 0.0,
                follow: true,
            },
            back,
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
}

/// A PSBT's files as the card lists them: every PSBT in Files, From the
/// stick then For the stick, as (outbox, place, item).
pub(crate) fn files<'a>(inbox: &'a [Item], outbox: &'a [Item]) -> Vec<(bool, usize, &'a Item)> {
    let of = |items: &'a [Item], out: bool| {
        items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.psbt().is_some())
            .map(|(k, i)| (out, k, i))
            .collect::<Vec<_>>()
    };
    let mut all = of(inbox, false);
    all.extend(of(outbox, true));
    all
}

/// The rows of one card: the file's place in [`files`] for each, the
/// other side's transaction first and then the rest, the file chosen as
/// the other side not listed. The Sent card follows the Returned file
/// the same way, as where Files opened the flow on one. The split is
/// where the "Other transactions" label goes.
pub(crate) fn rows(
    all: &[(bool, usize, &Item)],
    slot: Slot,
    (sent, returned): (Option<&Side>, Option<&Side>),
) -> (Vec<usize>, Vec<usize>) {
    let (mut same, mut other) = (Vec::new(), Vec::new());
    let beside = match slot {
        Slot::Returned => sent,
        Slot::Sent => returned,
    };
    // A file of the other side's name that is not that copy is a file of
    // its own, listed.
    let kept = |s: &Side| !s.differs;
    for (n, (outbox, _, item)) in all.iter().enumerate() {
        match beside {
            Some(s) => {
                if kept(s)
                    && s.origin
                        == (Origin::File {
                            outbox: *outbox,
                            name: item.name.clone(),
                        })
                {
                    continue;
                }
                if item.psbt().is_some_and(|p| p.txid == s.txid) {
                    same.push(n);
                } else {
                    other.push(n);
                }
            }
            None => same.push(n),
        }
    }
    (same, other)
}

/// The default of SIGNER-CHECK §3, "Which is which": exactly two PSBTs
/// in Files share an unsigned transaction and one carries fewer
/// signatures. Gives the places in [`files`] of Sent and Returned. A
/// lookup over what each file's `PsbtSeen` kept.
pub(crate) fn default_pair(all: &[(bool, usize, &Item)]) -> Option<(usize, usize)> {
    let seen = |n: usize| -> Option<&PsbtSeen> { all[n].2.psbt() };
    let mut found: Option<(usize, usize)> = None;
    for a in 0..all.len() {
        let ta = seen(a)?.txid;
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
    raw_bytes(item) == kept
}

/// Costs a base64 decode for a text file. The file's bytes as the
/// person has them: raw, base64 decoded where the file is text.
pub(crate) fn raw_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.starts_with(b"psbt\xff") {
        return bytes.to_vec();
    }
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| base64::decode(t.trim()).ok())
        .unwrap_or_else(|| bytes.to_vec())
}

/// What the verdict line says, and in which tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    Ok,
    Warn,
    Err,
}

/// The verdict as words (SIGNER-CHECK §5) and its tone.
pub(crate) fn verdict_line(report: &Report) -> (String, Tone) {
    match report.verdict {
        Verdict::Clean { finalized, nonces } => {
            let text = if nonces {
                EN.signer_only_nonces.to_string()
            } else if finalized == 1 {
                EN.signer_only_one_finalized.to_string()
            } else if finalized > 1 {
                crate::strings::fill(EN.signer_only_finalized, &[&finalized.to_string()])
            } else {
                EN.signer_only_signatures.to_string()
            };
            (text, Tone::Ok)
        }
        Verdict::Findings { count, worst } => {
            let text = if count == 1 {
                EN.signer_one_finding.to_string()
            } else {
                crate::strings::fill(EN.signer_findings, &[&count.to_string()])
            };
            let tone = if worst >= Level::Danger {
                Tone::Err
            } else {
                Tone::Warn
            };
            (text, tone)
        }
        Verdict::NotSameTransaction => (EN.signer_not_the_same.to_string(), Tone::Err),
    }
}

/// The Result card's text, in the order it is drawn.
pub(crate) struct Shown {
    /// Label and value, one fact each.
    pub facts: Vec<(String, String)>,
    /// A line per finding in the report's order, with its level.
    pub findings: Vec<(Level, String)>,
    /// The lines stated, which are no finding.
    pub stated: Vec<String>,
    /// Whether the findings section is drawn: not where the two PSBTs
    /// share no input.
    pub has_findings: bool,
    /// The keys that signed and whose seed is not loaded, one **Load**
    /// each.
    pub load: Vec<Fingerprint>,
}

/// Costs the text and nothing beyond it; the inspection is the caller's.
/// The report in words, with the Returned PSBT's `inspection` for what it
/// sends, its fee and its change (SIGNER-CHECK §4).
pub(crate) fn shown(
    report: &Report,
    (sent, returned): (&Txid, &Txid),
    inspection: &Inspection,
    network: Network,
) -> Shown {
    use super::check_words as words;
    let id = |t: &Txid| faraday_ui::grouped(&t.to_string());
    let mut facts = Vec::new();
    let same = report.verdict != Verdict::NotSameTransaction;
    if sent == returned {
        facts.push((EN.signer_transaction.to_string(), id(sent)));
    } else {
        facts.push((EN.signer_sent.to_string(), id(sent)));
        facts.push((EN.signer_returned.to_string(), id(returned)));
    }
    if same {
        let f = &report.facts;
        facts.push((
            EN.signer_signatures_added.to_string(),
            (f.added.len() + f.partial_sigs_added).to_string(),
        ));
        if !f.finalized.is_empty() {
            facts.push((
                EN.signer_inputs_finalized.to_string(),
                f.finalized.len().to_string(),
            ));
        }
        if let Some(r) = &f.recomputed
            && (r.of > 0 || !r.not_loaded.is_empty())
        {
            let label = EN.signer_nonces_recomputed;
            let counts = fill(
                EN.signer_nonces_of,
                &[&r.recomputed.to_string(), &r.of.to_string()],
            );
            if r.recomputed == 0 && !r.not_loaded.is_empty() {
                for fp in &r.not_loaded {
                    facts.push((
                        label.to_string(),
                        fill(EN.signer_nonces_none, &[&fp.to_string()]),
                    ));
                }
            } else {
                facts.push((label.to_string(), counts));
                for fp in &r.not_loaded {
                    facts.push((
                        label.to_string(),
                        fill(EN.signer_nonces_not_loaded, &[&fp.to_string()]),
                    ));
                }
            }
        }
        facts.push((
            EN.step_sends.to_string(),
            super::btc(inspection.amount_to_others.to_sat()),
        ));
        facts.push((
            EN.step_fee.to_string(),
            if inspection.fee_rate_sat_vb.is_some() {
                super::btc(inspection.fee.to_sat())
            } else {
                EN.step_fee_not_known.to_string()
            },
        ));
        // Verified where an output is change on the change chain and none
        // claims a key it could not verify.
        let verified = inspection.outputs.iter().any(|o| {
            matches!(
                o.kind,
                OutputKind::Change { change: true, .. }
                    | OutputKind::WalletChange { change: true, .. }
            )
        }) && !inspection
            .outputs
            .iter()
            .any(|o| matches!(o.kind, OutputKind::UnverifiedChange { .. }));
        facts.push((
            EN.step_change.to_string(),
            if verified {
                EN.signer_change_verified
            } else {
                EN.signer_change_not_verified
            }
            .to_string(),
        ));
    }
    Shown {
        facts,
        findings: report
            .findings
            .iter()
            .map(|f| {
                let line = words::finding(f, network);
                (
                    f.level,
                    fill(EN.signer_finding_line, &[words::level_word(f.level), &line]),
                )
            })
            .collect(),
        stated: report.info.iter().map(words::stated).collect(),
        has_findings: same,
        load: report
            .facts
            .recomputed
            .as_ref()
            .map_or_else(Vec::new, |r| r.not_loaded.clone()),
    }
}

/// Check a signer's cards, plugged into Back and Forward (F104).
pub(crate) struct SignerSteps;

/// The one [`SignerSteps`].
pub(crate) static SIGNER_STEPS: SignerSteps = SignerSteps;

impl crate::trail::StepFlow for SignerSteps {
    fn steps(&self, app: &crate::Faraday) -> Option<crate::trail::Steps> {
        let s = app.signer_check()?;
        Some(crate::trail::Steps::new(
            (0..=RESULT).collect(),
            Some(s.open),
        ))
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
}
