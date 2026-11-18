//! A backup plan kept in a vault (`docs/WALLETS.md` §5): the record that
//! holds a wallet's plan, built from the answers, and the old one taken
//! out and put back when the new one does not fit.

use super::VaultArea;
use crate::strings::{EN, fill};
use crate::wallet::{self, fp_text};
use crate::{Medium, plan};
use faraday_crypto::work::Compute;
use faraday_keys::vault::VaultId;
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

/// Where each seed of a plan kept for the wallet `from` is in the wallet
/// `to`, which has the same keys in another order: seeds are numbered by
/// first appearance of a key's fingerprint, so the number of the seed
/// of `from` that comes `i`th is the `i`th entry. None when the
/// fingerprints do not map one-to-one.
pub(crate) fn seed_map(
    from: &faraday_bip::policy::WalletPolicy,
    to: &faraday_bip::policy::WalletPolicy,
) -> Option<Vec<usize>> {
    let fps = |p: &faraday_bip::policy::WalletPolicy| {
        let mut v: Vec<faraday_bip::keys::Fingerprint> = Vec::new();
        for f in p.keys().iter().filter_map(|k| k.fingerprint()) {
            if !v.contains(&f) {
                v.push(f);
            }
        }
        v
    };
    let (a, b) = (fps(from), fps(to));
    if a.len() != b.len() {
        return None;
    }
    a.iter()
        .map(|f| b.iter().position(|g| g == f))
        .collect::<Option<Vec<usize>>>()
        .filter(|m| (0..b.len()).all(|j| m.contains(&j)))
}

/// A plan record's answers text, for the wallet `wallet` it was kept for
/// (`d`) in whichever order its keys were then: read in `wallet`'s seed
/// numbers. None for a record that does not map onto the wallet's keys.
pub(crate) fn answers_in_order(
    d: &super::Described,
    text: &str,
    wallet: &faraday_bip::policy::WalletPolicy,
) -> Option<faraday_text::Text> {
    if same_keys(d, wallet) {
        return Some(faraday_text::Text::of(text));
    }
    plan::text::renumber_seeds(text, &seed_map(&d.policy, wallet)?)
}

/// Whether `d`'s keys are the same keys in the same places as `wallet`'s,
/// which a plan's shares need (a share holds the keys of its window).
pub(crate) fn same_keys(d: &super::Described, wallet: &faraday_bip::policy::WalletPolicy) -> bool {
    d.policy.keys() == wallet.keys()
}

/// Whether plan record `r`, kept for `d`, reads for `wallet`: its answers
/// are there and come out in the wallet's seed numbers.
pub(crate) fn reads_for(
    d: &super::Described,
    r: faraday_keys::vault::RecordView<'_>,
    wallet: &faraday_bip::policy::WalletPolicy,
) -> bool {
    r.public_text(faraday_vault::records::field::PLAN_ANSWERS)
        .is_some_and(|t| answers_in_order(d, t, wallet).is_some())
}

/// A note line of a plan record kept for `d`, read for `wallet` of
/// `seeds` seeds, in its seed numbers. None for a note that does not
/// read, is of a seed it does not have or a record that does not map.
pub(crate) fn note_in_order(
    d: &super::Described,
    line: &str,
    wallet: &faraday_bip::policy::WalletPolicy,
    seeds: usize,
) -> Option<plan::Note> {
    let note = plan::Note::from_text(line, usize::MAX)?;
    let note = if same_keys(d, wallet) {
        note
    } else {
        note.renumbered(&seed_map(&d.policy, wallet)?)?
    };
    let on_seed = note.on().and_then(|(_, w)| w.seed_index());
    on_seed.is_none_or(|i| i < seeds).then_some(note)
}

/// What a plan is written from.
pub(crate) struct PlanSource<'a> {
    /// The wallet the plan is for.
    pub wallet: &'a wallet::Wallet,
    /// The fingerprints of the keys made here with their backup pending.
    pub held: Vec<faraday_bip::keys::Fingerprint>,
    /// The plan's shape for the wallet.
    pub shape: plan::Shape,
    /// The medium files move on.
    pub medium: Medium,
    /// The answers.
    pub answers: &'a plan::Answers,
    /// The places' names.
    pub names: &'a [String],
    /// Work a later plan dropped, kept on the map as what it is.
    pub extras: &'a [String],
    /// The notes on the map, or those the record kept.
    pub notes: Option<&'a [plan::Note]>,
}

/// A plan record built for an open vault, with where the one it replaces
/// stands.
pub(crate) struct PlanRecord {
    /// The record to write.
    pub record: Record,
    /// The places of the plan records the wallet already has there.
    pub ours: Vec<usize>,
    /// A record among `ours` is kept for the other key order and does not
    /// read for the wallet: nothing is written over it.
    pub refused: bool,
}

impl VaultArea {
    /// Costs what `described_of` costs. Whether open vault `v` keeps a plan
    /// record of `w` for the other key order that `w` does not read (its
    /// shares hold the other order's keys): nothing is written over it.
    pub(crate) fn plan_refused(&self, c: &Compute, v: usize, w: &crate::wallet::Wallet) -> bool {
        let Some(open) = self.open.get(v) else {
            return false;
        };
        let same = wallet::same_wallet(w.policy());
        self.described_of(c, open.id, kind::PLAN).any(|(_, r, d)| {
            *d.same == *same && !same_keys(d, w.policy()) && !reads_for(d, r, w.policy())
        })
    }

    /// Costs what `described_of` costs. The plan record for the wallet
    /// `src` in open vault `v` (record type 11), over the one it kept for
    /// the wallet; None when `v` is not open or the one kept is the same.
    pub(crate) fn plan_record(
        &self,
        c: &Compute,
        v: usize,
        src: &PlanSource<'_>,
    ) -> Option<PlanRecord> {
        let open = self.open.get(v)?;
        let (w, answers, shape) = (src.wallet, src.answers, &src.shape);
        let mut text = answers.to_text();
        for fp in &src.held {
            text.push_str(&format!("held {}\n", &*fp_text(*fp)));
        }
        let refused = self.plan_refused(c, v, w);
        let ours: Vec<usize> = self
            .described_of(c, open.id, kind::PLAN)
            .filter(|(_, _, d)| *d.same == *wallet::same_wallet(w.policy()))
            .map(|(i, _, _)| i)
            .collect();
        let text = plan::text::with_seals(&text, &self.plan_seals_kept(open.id, &ours));
        let mut record = Record::new(kind::PLAN)
            .with(
                field::PLAN_WALLET,
                crate::Faraday::wallet_text(w).as_bytes(),
            )
            .with(field::PLAN_ANSWERS, text.as_bytes());
        for p in 0..answers.places {
            let name = src.names.get(p).map_or("", |n| n.trim());
            record.push(field::PLAN_PLACE, name.as_bytes());
        }
        // A place with what its sticks keep.
        for spot in plan::place_map(c, shape, answers) {
            let at = match spot.at {
                // The map's field names a place by number only; the name
                // itself lives in field 3 (`field::PLAN_PLACE`) alone.
                plan::At::Place(p) | plan::At::Stick(p, _) => {
                    fill(EN.plan_store_place, &[&(p + 1).to_string()])
                }
                plan::At::Vault(v) => String::from(&*answers.vault_name(shape, v)),
                plan::At::Files => fill(EN.plan_store_of_files, &[src.medium.cap()]),
                plan::At::Software => EN.plan_store_watch_only.to_string(),
                plan::At::Away => EN.plan_store_on_its_own.to_string(),
            };
            let holds: Vec<String> = spot
                .holds
                .iter()
                .map(|(h, _)| String::from(&*h.label(shape, src.medium.cap())))
                .collect();
            record.push(
                field::PLAN_HOLDS,
                format!("{at}: {}", holds.join(", ")).as_bytes(),
            );
        }
        // Work a later plan dropped stays on the map as what it is.
        for line in src.extras {
            record.push(field::PLAN_HOLDS, line.as_bytes());
        }
        // The notes on the map: those given, else those the record kept.
        let kept: Vec<faraday_text::Text> = match src.notes {
            Some(n) => n.iter().map(plan::Note::to_text).collect(),
            // In this wallet's seed numbers, whatever order the key was
            // in when they were kept.
            None => self
                .described_of(c, open.id, kind::PLAN)
                .filter(|(i, _, _)| ours.contains(i))
                .flat_map(|(_, r, d)| {
                    r.public_all(field::PLAN_NOTE)
                        .filter_map(|b| {
                            let line = faraday_text::Text::from_utf8_lossy(b);
                            if same_keys(d, w.policy()) {
                                return Some(line);
                            }
                            note_in_order(d, &line, w.policy(), usize::MAX).map(|n| n.to_text())
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
        };
        for n in &kept {
            record.push(field::PLAN_NOTE, n.as_bytes());
        }
        // Kept as it was: nothing to write back.
        let unchanged = self
            .records(open.id)
            .iter()
            .any(|(i, r)| ours.contains(&i) && r.same_fields(&record));
        if unchanged {
            return None;
        }
        Some(PlanRecord {
            record,
            ours,
            refused,
        })
    }

    /// Costs what `described_of` costs, and `place_map`. The lines open
    /// vault `v`'s plan record of `w` holds besides its places' own: work a
    /// later plan dropped (`PlanSource::extras`), the record having been
    /// made from `answers`.
    pub(crate) fn plan_extras_of(
        &self,
        c: &Compute,
        v: usize,
        w: &crate::wallet::Wallet,
        shape: &plan::Shape,
        answers: &plan::Answers,
    ) -> Vec<faraday_text::Text> {
        let Some(open) = self.open.get(v) else {
            return Vec::new();
        };
        let want = wallet::same_wallet(w.policy());
        self.described_of(c, open.id, kind::PLAN)
            .find(|(_, r, d)| *d.same == *want && reads_for(d, *r, w.policy()))
            .map(|(_, r, d)| {
                // The spots the record itself was made with, else the given
                // plan's.
                let own = r
                    .public_text(field::PLAN_ANSWERS)
                    .and_then(|t| answers_in_order(d, t, w.policy()))
                    .and_then(|t| plan::Answers::from_text(shape, &t));
                let spots = plan::place_map(c, shape, own.as_ref().unwrap_or(answers)).len();
                r.public_all(field::PLAN_HOLDS)
                    .skip(spots)
                    .map(faraday_text::Text::from_utf8_lossy)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Costs what `plan_refused` costs. Open vault `v` no longer keeps a
    /// plan for `w`, which the person left it out of: the records it kept
    /// for the wallet go, unless they are for the other key order `w` does
    /// not read.
    pub(crate) fn plan_forget(&mut self, c: &Compute, v: usize, w: &crate::wallet::Wallet) {
        let Some(open) = self.open.get(v) else {
            return;
        };
        let id = open.id;
        if self.plan_refused(c, v, w) {
            return;
        }
        let same = wallet::same_wallet(w.policy());
        let ours: Vec<usize> = self
            .described_of(c, id, kind::PLAN)
            .filter(|(_, _, d)| *d.same == *same)
            .map(|(i, _, _)| i)
            .collect();
        if ours.is_empty() {
            return;
        }
        self.plan_take(id, &ours);
        self.changed(id);
    }

    /// Takes the plan records at `ours` out of vault `id`, returning them to
    /// put back by [`VaultArea::plan_restore`].
    pub(crate) fn plan_take(&mut self, id: VaultId, ours: &[usize]) -> Vec<Record> {
        let old: Vec<Record> = self
            .records(id)
            .iter()
            .filter(|(i, _)| ours.contains(i))
            .filter_map(|(_, r)| r.to_public())
            .collect();
        if !ours.is_empty() {
            // `remove_where` asks each record once, in order: the count is
            // the record's place before any went (its own place shifts).
            let mut seen = 0;
            self.edit(id, |s| {
                s.remove_where(id, |_| {
                    seen += 1;
                    ours.contains(&(seen - 1))
                })
            });
        }
        old
    }

    /// Puts back what [`VaultArea::plan_take`] took when the new record did
    /// not fit (`fitted` false); otherwise notes the change if any went.
    pub(crate) fn plan_restore(&mut self, id: VaultId, old: Vec<Record>, fitted: bool) {
        if !fitted {
            // It did not fit: the record it kept stays.
            self.edit(id, |s| {
                for r in old {
                    s.insert(id, r);
                }
            });
        } else if !old.is_empty() {
            self.changed(id);
        }
    }
}
