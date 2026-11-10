//! Compare two PSBTs, and the fee bump (`docs/COMPARE.md`): the flow's
//! state and what it decides without the app. The presses that need the
//! app are in `decode/mod.rs`; the cards are drawn by `screen.rs`.
//!
//! Everything kept here is public data (two PSBTs and the comparison of
//! them), kept until a lock or power-off as Check a signer's is.

use crate::decode::check::Origin;
use crate::strings::{EN, fill};
use crate::wallet::InspectInputs;
use crate::{Item, Screen, flow};
use faraday_bip::bitcoin::{OutPoint, Txid};
use faraday_bip::keys::Fingerprint;
use faraday_psbt::Psbt;
use faraday_psbt::bump::{Bump, Line, Outcome};
use faraday_psbt::diff::Differences;

pub(crate) mod others;
pub(crate) mod screen;
pub(crate) mod words;

/// The two cards that choose a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// The first file (the original of a fee bump).
    First,
    /// The second file (the replacement of a fee bump).
    Second,
}

impl Slot {
    /// The card's place in the flow: First, Second, then the Result.
    pub(crate) fn card(self) -> u8 {
        match self {
            Slot::First => 0,
            Slot::Second => 1,
        }
    }
}

/// The card the Result is on.
pub(crate) const RESULT: u8 = 2;

/// A press on Compare two PSBTs (`Screen::Compare`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparePress {
    /// The Tools tile: the flow opens, as it was where one is kept.
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
    /// The Result's **Swap**: the two exchanged and compared again.
    Swap,
    /// The Result's **Choose again**: the Second card opens.
    Again,
    /// The Result's **Check a signer**: it opens with First as Sent and
    /// Second as Returned.
    CheckSigner,
    /// The Result's **Load {fp}** for the key in this place of
    /// [`Shown::load`]: Add a key opens, and its end comes back here.
    Load(usize),
    /// Files' ⋮ **Compare** on this file (From the stick, or For the
    /// stick when the first is true; its place in that list): the flow
    /// opens with it as Second.
    File(bool, usize),
    /// Sign a transaction's review **Compare** on the other transaction
    /// in this file (as [`ComparePress::File`] names one): the flow
    /// opens with that file as First and the PSBT under review as
    /// Second, compared.
    FromReview(bool, usize),
}

/// One side of the comparison: the file as the person has it, read.
pub(crate) struct Side {
    /// The file's bytes as the person has them (raw, base64 or hex).
    pub bytes: Vec<u8>,
    /// Its version 0 form (a version 2 file through `PsbtV2::to_v0`),
    /// or a finished transaction as a PSBT whose inputs are final.
    pub psbt: Psbt,
    /// The PSBT's own version, 0 or 2; `None` for a finished transaction.
    pub version: Option<u8>,
    /// The outpoints it spends, sorted.
    pub outpoints: Vec<OutPoint>,
    /// What the closed card shows: the file's name, or "Scanned".
    pub name: String,
    /// Where it came from.
    pub origin: Origin,
    /// Whether Files holds no file of this name, in either box, with
    /// these bytes any more: the copy and its result are kept. Worked
    /// out by `Faraday::compare_sync` while the flow is on screen and on
    /// arriving at it, and made false again by a pick.
    pub differs: bool,
}

/// The flow's state.
pub(crate) struct CompareState {
    /// The first file.
    pub first: Option<Side>,
    /// The second file.
    pub second: Option<Side>,
    /// The comparison of the two files kept above, against the keys and
    /// wallets loaded, in words. Invariant: `shown` is `Some` only for the
    /// very `first` and `second` held now; it is replaced (or dropped)
    /// whenever either file changes (a pick, a scan, a swap) and whenever
    /// a key or wallet is loaded or dropped (`docs/COMPARE.md` §5.3).
    pub shown: Option<Shown>,
    /// The keys and wallets the result was made with: a press that leaves
    /// others makes it again (`Faraday::compare_sync`).
    pub made_with: Option<InspectInputs>,
    /// The ids of the finished transactions in Files that the lists show,
    /// worked out beside the file's bytes by `Faraday::compare_sync`, so
    /// drawing reads them.
    pub ids: Vec<(Vec<u8>, Txid)>,
    /// The card open.
    pub open: u8,
    /// Why a card refused what was given it.
    pub refused: [Option<String>; 2],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// Where the flow was opened from.
    pub back: Screen,
}

impl CompareState {
    pub(crate) fn new(back: Screen) -> CompareState {
        CompareState {
            first: None,
            second: None,
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
        }
    }

    pub(crate) fn side(&self, slot: Slot) -> Option<&Side> {
        match slot {
            Slot::First => self.first.as_ref(),
            Slot::Second => self.second.as_ref(),
        }
    }

    /// Puts `side` on `slot`, drops the result, and opens the card that
    /// follows: the other one where it is empty, the Result once both are
    /// in (the caller compares then). Returns whether both are in.
    pub(crate) fn put(&mut self, slot: Slot, side: Side) -> bool {
        self.shown = None;
        self.refused[usize::from(slot.card())] = None;
        let other = match slot {
            Slot::First => &mut self.second,
            Slot::Second => &mut self.first,
        };
        // The other card may not hold this file.
        if other
            .as_ref()
            .is_some_and(|o| o.origin != Origin::Scanned && o.origin == side.origin)
        {
            *other = None;
        }
        match slot {
            Slot::First => self.first = Some(side),
            Slot::Second => self.second = Some(side),
        }
        self.scroll.follow = true;
        let both = self.first.is_some() && self.second.is_some();
        self.open = if both {
            RESULT
        } else if slot == Slot::First {
            Slot::Second.card()
        } else {
            Slot::First.card()
        };
        both
    }

    /// First and Second exchanged; the result goes, and the caller
    /// compares again.
    pub(crate) fn swap(&mut self) {
        std::mem::swap(&mut self.first, &mut self.second);
        self.refused = [None, None];
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

    /// A finished transaction's id as `sync` kept it.
    fn id_of(&self, bytes: &[u8]) -> Option<Txid> {
        self.ids.iter().find(|(b, _)| b == bytes).map(|(_, t)| *t)
    }

    /// A file's line in the lists: "{n} signatures · {id}" for a PSBT,
    /// "Finished · {id}" for a finished transaction. Reads what the file
    /// and `sync` kept.
    pub(crate) fn line(&self, item: &Item) -> String {
        if let Some(p) = item.psbt() {
            let id = faraday_ui::short(&p.txid.to_string());
            return if p.sigs == 1 {
                fill(EN.signer_row_one, &[&id])
            } else {
                fill(EN.signer_row, &[&p.sigs.to_string(), &id])
            };
        }
        self.id_of(&item.bytes).map_or_else(String::new, |t| {
            fill(
                EN.compare_row_finished,
                &[&faraday_ui::short(&t.to_string())],
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

/// The rows of one card: the place in [`files`] of each, the files that
/// spend an input the other side spends first and then the rest, the file
/// chosen as the other side not listed. The split is where the "Other
/// transactions" label goes.
pub(crate) fn rows(
    all: &[(bool, usize, &Item)],
    slot: Slot,
    (first, second): (Option<&Side>, Option<&Side>),
) -> (Vec<usize>, Vec<usize>) {
    let (mut same, mut other) = (Vec::new(), Vec::new());
    let beside = match slot {
        Slot::Second => first,
        Slot::First => second,
    };
    for (n, (outbox, _, item)) in all.iter().enumerate() {
        match beside {
            Some(s) => {
                // A file of the other side's name that is not that copy is
                // a file of its own, listed.
                if !s.differs
                    && s.origin
                        == (Origin::File {
                            outbox: *outbox,
                            name: item.name.clone(),
                        })
                {
                    continue;
                }
                if others::shares(&item.outpoints, &s.outpoints) {
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

/// How a fee bump result is toned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    Ok,
    Warn,
    Err,
}

/// The fee bump result in words.
pub(crate) struct BumpShown {
    /// What it came to; the screen tones it.
    pub outcome: Outcome,
    /// The result line.
    pub result: &'static str,
    /// The lines, in the rules' order.
    pub lines: Vec<String>,
    /// The facts under them.
    pub facts: Vec<words::Fact>,
}

impl BumpShown {
    /// The result's tone: Only the fee is raised in `OK`, More than the
    /// fee changed in `ERR`, Cannot tell in `WARN` (§3.3).
    pub(crate) fn tone(&self) -> Tone {
        match self.outcome {
            Outcome::OnlyFee => Tone::Ok,
            Outcome::MoreChanged => Tone::Err,
            Outcome::CannotTell => Tone::Warn,
        }
    }
}

/// The Result card's text, in the order it is drawn.
pub(crate) struct Shown {
    /// The facts.
    pub facts: Vec<words::Fact>,
    /// The fee bump result, where §3.1 says it applies.
    pub bump: Option<BumpShown>,
    /// A line per difference, or "No differences".
    pub differences: Vec<String>,
    /// Whether **Check a signer** is offered: the two are one unsigned
    /// transaction and both are PSBTs.
    pub can_check: bool,
    /// The keys the result says are not loaded, one **Load** each.
    pub load: Vec<Fingerprint>,
}

/// Costs the text and nothing beyond it. The comparison in words.
/// `psbts` is whether both sides are PSBTs.
pub(crate) fn shown(d: &Differences, bump: Option<&Bump>, psbts: bool) -> Shown {
    let mut load: Vec<Fingerprint> = Vec::new();
    for l in bump.map_or(&[][..], |b| &b.lines[..]) {
        if let Line::ChangeNotLoaded { key, .. } | Line::InputNotLoaded { key, .. } = l
            && !load.contains(key)
        {
            load.push(*key);
        }
    }
    Shown {
        facts: words::compare_facts(d),
        bump: bump.map(|b| BumpShown {
            outcome: b.result,
            result: words::result(b.result),
            lines: b.lines.iter().map(words::line).collect(),
            facts: words::bump_facts(b),
        }),
        differences: words::differences(d),
        can_check: psbts && d.same_transaction(),
        load,
    }
}
