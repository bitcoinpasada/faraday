//! The seal count in the app (`docs/VAULT-HARDENING.md` §5.2 to §5.6):
//! the count shown at unlock and on the open vault's head, the highest
//! count seen of each slot since power-on (kept with the vault's summary,
//! `docs/VAULT.md` §11), the comparison of a copy against it, and the
//! `seals` lines of the backup plan that carry other vaults' counts across
//! a power-off.

use super::*;
use crate::plan::text::{SealLine, seals_in, with_seals};
use faraday_keys::vault::OpenVaults;

/// The highest seal count seen of one slot of a vault since power-on,
/// with the SHA-256 of the slot's 40 + *L* bytes at that count, so two
/// seals with one count are told apart. A slot is told by the vault's
/// salt (the summary's), its length and its position, which is never
/// shown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SealSeen {
    /// The slot length *L*.
    pub len: u32,
    /// The slot's position in the file, 0 to 3.
    pub position: u8,
    /// The highest count seen.
    pub count: u64,
    /// The digest of the slot at that count.
    pub digest: [u8; 32],
    /// The descriptor checksums of the wallets this slot keeps a plan for
    /// (the summary's `planned`, as last seen open): what says which
    /// plans place this vault, for their `seals` lines.
    pub planned: Vec<String>,
}

/// Where a copy stands against the highest count seen of its slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealOrder {
    /// The highest seen, or the first seen.
    Current,
    /// Lower than one seen: the highest count seen, and where.
    Older(u64, SealFrom),
    /// The highest seen, with other bytes.
    SameCount,
}

/// What the seal count says of the copy an open vault was opened from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealWarn {
    /// An older copy: its count, the higher count seen, and where.
    Older {
        /// The count of this copy.
        mine: u64,
        /// The higher count seen.
        seen: u64,
        /// Where the higher count was seen.
        from: SealFrom,
    },
    /// Another copy was sealed as many times, with other bytes.
    Same {
        /// The count both have.
        count: u64,
    },
}

/// Where a higher seal count than the copy opened was seen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealFrom {
    /// Opened, sealed or read since power-on.
    Power,
    /// Recorded by the backup plan of the vault of this name.
    Plan(String),
    /// A copy of the vault in Files.
    Files,
}

/// "once" or "12 times".
pub fn times(n: u64) -> String {
    if n == 1 {
        EN.vault_once.to_string()
    } else {
        fill(EN.vault_times, &[&n.to_string()])
    }
}

impl SealWarn {
    /// The sentence Unlock and the vault's head say.
    pub fn line(&self) -> String {
        match self {
            SealWarn::Older {
                mine,
                seen,
                from: SealFrom::Power,
            } => fill(EN.vault_seals_older, &[&times(*mine), &times(*seen)]),
            SealWarn::Older {
                mine,
                seen,
                from: SealFrom::Files,
            } => fill(EN.vault_seals_older_files, &[&times(*mine), &times(*seen)]),
            SealWarn::Older {
                mine,
                seen,
                from: SealFrom::Plan(name),
            } => fill(
                EN.vault_seals_older_plan,
                &[&times(*mine), name, &times(*seen)],
            ),
            SealWarn::Same { count } => fill(EN.vault_seals_same_count, &[&times(*count)]),
        }
    }
}

impl OpenVault {
    /// "Sealed 12 times · last 28 Oct 2026, 14:02": the count the slot was
    /// opened with, or the last seal wrote, and the time by the clock that
    /// sealed it, as its own. "Sealed once" for a count of one.
    pub fn seals_line(&self, store: &OpenVaults, tz_min: i16) -> String {
        let Some((_, seals)) = store.seal_state(self.id) else {
            return String::new();
        };
        let count = fill(EN.vault_seals, &[&times(seals.count)]);
        let when = match seals.last {
            Some(t) => {
                let local = local_secs(t, tz_min);
                let date = crate::gpg::date(local);
                let mut ymd = date.split('-');
                let (y, m, d) = (
                    ymd.next().unwrap_or(""),
                    ymd.next()
                        .and_then(|m| m.parse::<usize>().ok())
                        .unwrap_or(0),
                    ymd.next().and_then(|d| d.parse::<u32>().ok()).unwrap_or(0),
                );
                let month = EN
                    .vault_months
                    .split(' ')
                    .nth(m.wrapping_sub(1))
                    .unwrap_or("");
                fill(
                    EN.vault_sealed_last,
                    &[&fill(
                        EN.vault_sealed_date,
                        &[&d.to_string(), month, y, &time_of_day(local)],
                    )],
                )
            }
            None => EN.vault_sealed_no_time.to_string(),
        };
        // A count of one is sealed once, with no time kept: nothing more.
        if seals.count == 1 && seals.last.is_none() {
            return count;
        }
        fill(EN.vault_seals_joined, &[&count, &when])
    }

    /// What the seal count says of the copy this vault was opened from,
    /// when it is not the newest seen.
    pub fn seal_warning(&self) -> Option<String> {
        self.seal_warn.as_ref().map(SealWarn::line)
    }
}

impl Vaults {
    /// The `seals` lines the first of this wallet's plan records `ours`
    /// in open vault `id` keeps.
    pub(crate) fn plan_seals_kept(&self, id: VaultId, ours: &[usize]) -> Vec<SealLine> {
        ours.iter()
            .filter_map(|i| self.records(id).get(*i))
            .filter_map(|r| r.public_text(field::PLAN_ANSWERS).map(seals_in))
            .next()
            .unwrap_or_default()
    }

    /// Raises the highest count seen of one slot of the vault with `salt`
    /// to `seen` when it is higher, and says where `seen` stands against
    /// it. A vault with no summary kept says `Current` and keeps nothing:
    /// summaries are made when a vault opens, before any count is seen.
    pub(crate) fn seals_raise(&mut self, salt: [u8; 32], seen: SealSeen) -> SealOrder {
        let Some(s) = self.summaries.iter_mut().find(|s| s.salt == salt) else {
            return SealOrder::Current;
        };
        let Some(e) = s
            .seals
            .iter_mut()
            .find(|e| e.len == seen.len && e.position == seen.position)
        else {
            s.seals.push(seen);
            return SealOrder::Current;
        };
        if seen.count > e.count {
            let planned = std::mem::take(&mut e.planned);
            *e = seen;
            if e.planned.is_empty() {
                e.planned = planned;
            }
            SealOrder::Current
        } else if seen.count < e.count {
            SealOrder::Older(e.count, SealFrom::Power)
        } else if seen.digest != e.digest {
            SealOrder::SameCount
        } else {
            SealOrder::Current
        }
    }

    /// Sets the plans each open slot's entry keeps from the summary, once
    /// the summaries are refreshed.
    pub(crate) fn seals_planned_sync(&mut self) {
        let open: Vec<(VaultId, Header)> = self.open.iter().map(|o| (o.id, *o.header())).collect();
        for (id, header) in open {
            let Some((pos, _)) = self.store().seal_state(id) else {
                continue;
            };
            if let Some(s) = self.summaries.iter_mut().find(|s| s.salt == header.salt) {
                let planned = s.planned.clone();
                if let Some(e) = s
                    .seals
                    .iter_mut()
                    .find(|e| e.len == header.slot_len && usize::from(e.position) == pos)
                {
                    e.planned = planned;
                }
            }
        }
    }

    /// Costs a SHA-256 over one slot of the file opened (counted as a
    /// file hash), and what `plan_seals_check` costs. Open vault `v`,
    /// just unlocked: its count raises the highest seen, a count lower
    /// than the highest seen, or the same with other bytes, is kept for
    /// Unlock and the vault's head to say (§5.3 item 1), and the backup
    /// plans of the open vaults are read for the counts they record
    /// (§5.6).
    pub(crate) fn seals_unlocked(&mut self, c: &Compute, v: usize, files: &[(&str, &[u8])]) {
        let Some(o) = self.open.get(v) else {
            return;
        };
        let (id, header) = (o.id, *o.header());
        let Some((pos, seals)) = self.store().seal_state(id) else {
            return;
        };
        let Some(digest) = self.store().opened_digest(c, id) else {
            return;
        };
        let order = self.seals_raise(
            header.salt,
            SealSeen {
                len: header.slot_len,
                position: pos as u8,
                count: seals.count,
                digest,
                planned: Vec::new(),
            },
        );
        self.seals_planned_sync();
        // The other copies of this vault in Files are read with the key
        // just opened: each is a copy seen.
        let order = self.seals_files_copies(c, v, pos, digest, files, order);
        let o = &mut self.open[v];
        o.seal_digest = digest;
        o.seal_warn = match order {
            SealOrder::Current => None,
            SealOrder::Older(seen, from) => Some(SealWarn::Older {
                mine: seals.count,
                seen,
                from,
            }),
            SealOrder::SameCount => Some(SealWarn::Same { count: seals.count }),
        };
        self.plan_seals_check();
    }

    /// The `seals` lines of every open vault's plans, each compared with
    /// each open slot of the same salt, length and position and treated
    /// as a copy seen: a higher count makes that slot an older copy, the
    /// same count with another digest says so, otherwise nothing is said.
    /// Never refuses: the slot stays open (§5.6 reading).
    pub(crate) fn plan_seals_check(&mut self) {
        let mut lines: Vec<(String, SealLine)> = Vec::new();
        for o in &self.open {
            let label = o.label(self.store());
            for (_, r) in self.records(o.id).of(kind::PLAN) {
                if let Some(t) = r.public_text(field::PLAN_ANSWERS) {
                    lines.extend(seals_in(t).into_iter().map(|l| (label.clone(), l)));
                }
            }
        }
        for (label, l) in lines {
            for v in 0..self.open.len() {
                let o = &self.open[v];
                let header = *o.header();
                if header.salt != l.salt || header.slot_len != l.len {
                    continue;
                }
                let Some((pos, seals)) = self.store().seal_state(o.id) else {
                    continue;
                };
                if pos != usize::from(l.position) {
                    continue;
                }
                let (mine, digest) = (seals.count, o.seal_digest);
                let held = o.seal_warn.is_some();
                let held_older = match &o.seal_warn {
                    Some(SealWarn::Older { seen, .. }) => Some(*seen),
                    _ => None,
                };
                let warn = if l.count > mine && held_older.is_none_or(|s| l.count > s) {
                    Some(SealWarn::Older {
                        mine,
                        seen: l.count,
                        from: SealFrom::Plan(label.clone()),
                    })
                } else if l.count == mine && l.digest != digest && !held {
                    Some(SealWarn::Same { count: mine })
                } else {
                    None
                };
                // A line is a copy seen.
                self.seals_raise(
                    l.salt,
                    SealSeen {
                        len: l.len,
                        position: l.position,
                        count: l.count,
                        digest: l.digest,
                        planned: Vec::new(),
                    },
                );
                if warn.is_some() {
                    self.open[v].seal_warn = warn;
                }
            }
        }
    }

    /// Costs what `described` costs, the first time. Open vault `i`'s
    /// plans, each given the `seals` lines of the other vaults it places
    /// that this session knows: the highest count and digest seen, a
    /// line replaced only by a higher count or the same count seen now.
    /// Lines for vaults not seen this session are kept as they were. A
    /// vault whose lines change counts the change, so a lock seals it
    /// with the rest (§5.6 writing).
    pub(crate) fn seals_write(&mut self, c: &Compute, i: usize) {
        let Some(o) = self.open.get(i) else {
            return;
        };
        let (id, own) = (o.id, o.header().salt);
        let mut edits: Vec<(usize, String)> = Vec::new();
        for (k, r, d) in self.described_of(c, id, kind::PLAN) {
            let Some(old) = r.public_text(field::PLAN_ANSWERS) else {
                continue;
            };
            let kept = seals_in(old);
            let mut lines = kept.clone();
            for s in self.summaries.iter().filter(|s| s.salt != own) {
                for e in s.seals.iter().filter(|e| e.planned.contains(&d.checksum)) {
                    let new = SealLine {
                        salt: s.salt,
                        len: e.len,
                        position: e.position,
                        count: e.count,
                        digest: e.digest,
                    };
                    match lines.iter_mut().find(|l| l.same_slot(&new)) {
                        Some(l) if new.count >= l.count => *l = new,
                        Some(_) => {}
                        None => lines.push(new),
                    }
                }
            }
            if lines != kept {
                edits.push((k, with_seals(old, &lines)));
            }
        }
        if edits.is_empty() {
            return;
        }
        self.edit(id, |s| {
            for (k, text) in &edits {
                s.set_public(id, *k, field::PLAN_ANSWERS, text.as_bytes());
            }
        });
        self.changed(id);
    }

    /// Costs a decryption of the slot and a SHA-256 over it for each copy
    /// of open vault `v` in Files other than the file it was opened from
    /// (counted as file hashes). Each copy's count raises the highest
    /// seen; the order of the opened copy (digest `own`) against what was
    /// seen and all of them is given.
    fn seals_files_copies(
        &mut self,
        c: &Compute,
        v: usize,
        pos: usize,
        own: [u8; 32],
        files: &[(&str, &[u8])],
        mut order: SealOrder,
    ) -> SealOrder {
        let o = &self.open[v];
        let (id, header, name) = (o.id, *o.header(), o.name.clone());
        let copies = files.iter().filter(|(n, b)| {
            *n != name
                && fv::read_header(b)
                    .is_ok_and(|h| h.salt == header.salt && h.slot_len == header.slot_len)
        });
        let mine = self.store().seal_state(id).map_or(0, |(_, s)| s.count);
        for (_, bytes) in copies {
            let Some(seals) = self.store().copy_seals(c, id, bytes) else {
                continue;
            };
            let Some(digest) = faraday_keys::vault::slot_digest(c, header.slot_len, bytes, pos)
            else {
                continue;
            };
            self.seals_raise(
                header.salt,
                SealSeen {
                    len: header.slot_len,
                    position: pos as u8,
                    count: seals.count,
                    digest,
                    planned: Vec::new(),
                },
            );
            if seals.count > mine {
                let higher = match &order {
                    SealOrder::Older(n, _) => (*n).max(seals.count),
                    _ => seals.count,
                };
                order = SealOrder::Older(higher, SealFrom::Files);
            } else if seals.count == mine && digest != own && matches!(order, SealOrder::Current) {
                order = SealOrder::SameCount;
            }
        }
        order
    }

    /// Costs a SHA-256 over one slot (counted as a file hash). Open vault
    /// `i` has just been sealed into `bytes`: its new count and digest
    /// raise the highest seen.
    pub(crate) fn seals_sealed(&mut self, c: &Compute, i: usize, bytes: &[u8]) {
        let Some(o) = self.open.get(i) else {
            return;
        };
        let (id, header) = (o.id, *o.header());
        let Some((pos, seals)) = self.store().seal_state(id) else {
            return;
        };
        let Some(digest) = faraday_keys::vault::slot_digest(c, header.slot_len, bytes, pos) else {
            return;
        };
        self.seals_raise(
            header.salt,
            SealSeen {
                len: header.slot_len,
                position: pos as u8,
                count: seals.count,
                digest,
                planned: Vec::new(),
            },
        );
        self.seals_planned_sync();
        let o = &mut self.open[i];
        o.seal_digest = digest;
        o.seal_warn = None;
    }
}

/// An unlock stopped at the older-copy dialog (`docs/VAULT-HARDENING.md`
/// §5.3, owner 2026-10-29): the slot opened is held, its key in the
/// store, and nothing of it loads or shows until **Open** is pressed;
/// **Keep locked** closes it. Faraday cannot tell an older copy from one
/// put back by someone else, so the person decides; it is never refused.
#[derive(Debug, Clone)]
pub struct OlderHold {
    /// The open vault held.
    pub id: VaultId,
    /// It was unlocked on the boot import's sheet.
    pub(crate) for_import: bool,
    /// The sheet it was unlocked on, shown again after the dialog.
    pub(crate) sheet: Option<crate::Sheet>,
    /// What was remembered of the vault before this copy opened.
    summary: Option<VaultSummary>,
}

impl VaultArea {
    /// The summary remembered of the vault open vault `v` is a copy of,
    /// before an unlock refreshes it: what **Keep locked** puts back.
    pub(crate) fn summary_before(&self, v: usize) -> Option<VaultSummary> {
        let salt = self.state.open.get(v)?.header().salt;
        self.state
            .summaries
            .iter()
            .find(|s| s.salt == salt)
            .cloned()
    }

    /// Holds open vault `v` at the older-copy dialog when its unlock
    /// found a warning (§5.3 items 1 and 2, §5.6); true when it holds.
    pub(crate) fn older_hold(
        &mut self,
        v: usize,
        (for_import, sheet): (bool, Option<crate::Sheet>),
        summary: Option<VaultSummary>,
    ) -> bool {
        let Some(o) = self.state.open.get(v).filter(|o| o.seal_warn.is_some()) else {
            return false;
        };
        self.state.older = Some(Box::new(OlderHold {
            id: o.id,
            for_import,
            sheet,
            summary,
        }));
        true
    }

    /// Takes the hold, if any.
    pub(crate) fn older_take(&mut self) -> Option<OlderHold> {
        self.state.older.take().map(|h| *h)
    }

    /// **Keep locked**: the held vault closes, its key wiped, and what
    /// was remembered of it before is put back, the counts seen kept.
    pub(crate) fn older_close(&mut self, h: OlderHold) {
        let Some(i) = self.state.open.iter().position(|o| o.id == h.id) else {
            return;
        };
        let salt = self.state.open[i].header().salt;
        self.state.close(i);
        self.state.current = 0;
        let at = self.state.summaries.iter().position(|s| s.salt == salt);
        match (at, h.summary) {
            (Some(at), Some(mut was)) => {
                was.seals = std::mem::take(&mut self.state.summaries[at].seals);
                self.state.summaries[at] = was;
            }
            (Some(at), None) => {
                self.state.summaries.remove(at);
            }
            (None, _) => {}
        }
        self.state.seals_planned_sync();
    }
}
