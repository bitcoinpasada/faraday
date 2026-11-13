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
}

impl VaultArea {
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
            text.push_str(&format!("held {}\n", fp_text(*fp)));
        }
        let ours: Vec<usize> = self
            .described_of(c, open.id, kind::PLAN)
            .filter(|(_, _, d)| *d.same == wallet::same_wallet(w.policy()))
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
            None => self
                .records(open.id)
                .iter()
                .filter(|(i, _)| ours.contains(i))
                .flat_map(|(_, r)| {
                    r.public_all(field::PLAN_NOTE)
                        .map(faraday_text::Text::from_utf8_lossy)
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
        Some(PlanRecord { record, ours })
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
