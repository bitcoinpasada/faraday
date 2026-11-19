//! A seed kept as a SLIP-39 set (`docs/decisions/F276.md`): the set's
//! public facts on the plan, its shares as things the map places on paper.
//!
//! A share is named by its seed, its group and its member index in the
//! group, each from 0. The plan keeps the set's identifier, its iteration
//! exponent, how many groups are needed, each group's threshold and size,
//! and which members were made; never a share.

use super::*;
use faraday_text::{Text, text};

/// The most groups a set has, and the most members of a group.
pub const MAX: usize = faraday_bip::slip39::MAX_COUNT;

/// One group of a set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// How many of its members the group needs.
    pub threshold: u8,
    /// How many members the group has.
    pub count: u8,
    /// The members made, by index from 0, ascending.
    pub made: Vec<u8>,
}

/// A seed's SLIP-39 set on the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slip39 {
    /// The seed, by its place among the wallet's.
    pub seed: usize,
    /// The set's identifier (15 bits).
    pub identifier: u16,
    /// The iteration exponent (4 bits).
    pub exponent: u8,
    /// How many groups the set needs.
    pub group_threshold: u8,
    /// The groups, in order.
    pub groups: Vec<Group>,
}

impl Slip39 {
    /// Seed `seed`'s set as its key keeps it (`docs/VAULT.md` §7).
    pub fn of_facts(seed: usize, f: &faraday_vault::slip39::Slip39Facts) -> Slip39 {
        Slip39 {
            seed,
            identifier: f.identifier,
            exponent: f.exponent,
            group_threshold: f.threshold,
            groups: f
                .groups
                .iter()
                .map(|g| Group {
                    threshold: g.threshold,
                    count: g.count,
                    made: (0..g.count).filter(|&m| (g.made >> m) & 1 == 1).collect(),
                })
                .collect(),
        }
    }

    /// Whether member `m` of group `g` was made: only a made share exists,
    /// so only it counts or may be placed.
    pub fn has_member(&self, g: usize, m: usize) -> bool {
        self.groups
            .get(g)
            .is_some_and(|grp| u8::try_from(m).is_ok_and(|m| grp.made.contains(&m)))
    }

    /// Whether the shares `got`, as (group, member), rebuild the seed:
    /// enough members of enough groups.
    pub(crate) fn rebuilt_by(&self, got: &[(usize, usize)]) -> bool {
        let full = self
            .groups
            .iter()
            .enumerate()
            .filter(|&(g, grp)| {
                let members = got
                    .iter()
                    .filter(|&&(x, m)| x == g && self.has_member(g, m))
                    .count();
                members >= usize::from(grp.threshold)
            })
            .count();
        full >= usize::from(self.group_threshold)
    }

    /// The set's own soundness: SLIP-39's limits on its numbers, and a
    /// group threshold of 1 only with one group, as every set Faraday
    /// makes is.
    fn is_sound(&self) -> bool {
        let gt = usize::from(self.group_threshold);
        self.identifier < 1 << 15
            && self.exponent <= faraday_bip::slip39::MAX_ITERATION_EXPONENT
            && (1..=MAX).contains(&self.groups.len())
            && (1..=self.groups.len()).contains(&gt)
            && (gt > 1 || self.groups.len() == 1)
            && self.groups.iter().all(|g| {
                (1..=MAX).contains(&usize::from(g.count))
                    && (1..=g.count).contains(&g.threshold)
                    && (g.threshold > 1 || g.count == 1)
                    && g.made.iter().all(|&m| m < g.count)
                    && g.made.windows(2).all(|w| w[0] < w[1])
            })
    }

    /// The form's plan line: `slip39 {seed} {id} {exponent} {group
    /// threshold} {groups}`, a group as `{threshold}/{count}:{made}` with
    /// the members made as a comma list (`-` for none), groups joined by
    /// `;`.
    pub(crate) fn to_line(&self) -> Text {
        let mut out = text!(
            "slip39 {} {} {} {} ",
            self.seed,
            self.identifier,
            self.exponent,
            self.group_threshold
        );
        for (g, grp) in self.groups.iter().enumerate() {
            if g > 0 {
                out.push(';');
            }
            out.push_str(&text!("{}/{}:", grp.threshold, grp.count));
            if grp.made.is_empty() {
                out.push('-');
            }
            for (n, m) in grp.made.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                out.push_str(&Text::num(u64::from(*m)));
            }
        }
        out.push('\n');
        out
    }

    /// [`Slip39::to_line`]'s value read back (after `slip39 `); `None`
    /// for anything that is not a set a wallet of `seeds` seeds could
    /// have.
    pub(crate) fn from_value(value: &str, seeds: usize) -> Option<Slip39> {
        let mut parts = value.split(' ');
        let seed: usize = parts.next()?.parse().ok().filter(|&s| s < seeds)?;
        let identifier: u16 = parts.next()?.parse().ok()?;
        let exponent: u8 = parts.next()?.parse().ok()?;
        let group_threshold: u8 = parts.next()?.parse().ok()?;
        let groups = parts
            .next()?
            .split(';')
            .map(|g| {
                let (rule, made) = g.split_once(':')?;
                let (t, c) = rule.split_once('/')?;
                let made: Vec<u8> = match made {
                    "-" => Vec::new(),
                    s => s
                        .split(',')
                        .map(|m| m.parse().ok())
                        .collect::<Option<Vec<u8>>>()?,
                };
                Some(Group {
                    threshold: t.parse().ok()?,
                    count: c.parse().ok()?,
                    made,
                })
            })
            .collect::<Option<Vec<Group>>>()?;
        let set = Slip39 {
            seed,
            identifier,
            exponent,
            group_threshold,
            groups,
        };
        (parts.next().is_none() && set.is_sound()).then_some(set)
    }
}

/// The seed, group and member of `w`, when it is a SLIP-39 share.
pub fn share_of(w: What) -> Option<(usize, usize, usize)> {
    match w {
        What::Slip39Share(i, g, m) => Some((i, g, m)),
        _ => None,
    }
}

/// The (group, member) of seed `i`'s shares among `ws`.
pub(crate) fn shares_of(ws: &[What], i: usize) -> Vec<(usize, usize)> {
    ws.iter()
        .filter_map(|&w| share_of(w))
        .filter(|&(x, _, _)| x == i)
        .map(|(_, g, m)| (g, m))
        .collect()
}

impl Answers {
    /// Seed `i`'s SLIP-39 set, where it has one.
    pub fn slip39(&self, i: usize) -> Option<&Slip39> {
        self.slip39.iter().find(|s| s.seed == i)
    }

    /// Sets seed `i`'s SLIP-39 set, replacing the one it had, or takes it
    /// off with `None`. Shares the map placed that the new set lacks
    /// leave the map.
    pub fn set_slip39(&mut self, shape: &Shape, i: usize, set: Option<Slip39>) {
        let keep = set.clone();
        self.slip39.retain(|s| s.seed != i);
        if let Some(s) = set {
            self.slip39.push(s);
            self.slip39.sort_by_key(|s| s.seed);
        }
        if self.map.is_none() {
            return;
        }
        let mut m = stored_map(shape, self);
        for s in &mut m {
            s.holds.retain(|(w, _)| {
                share_of(*w).is_none_or(|(x, g, n)| {
                    x != i || keep.as_ref().is_some_and(|k| k.has_member(g, n))
                })
            });
        }
        self.set_map(m);
    }
}
