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

    /// Whether `other` is the same set (F293): identifier, iteration
    /// exponent, group threshold, group count and each group's member
    /// threshold agree, what every share carries. Which members were made
    /// and a group's member count are not compared.
    pub fn same_set(&self, other: &Slip39) -> bool {
        self.identifier == other.identifier
            && self.exponent == other.exponent
            && self.group_threshold == other.group_threshold
            && self.groups.len() == other.groups.len()
            && self
                .groups
                .iter()
                .zip(&other.groups)
                .all(|(a, b)| a.threshold == b.threshold)
    }

    /// Whether the shares `got`, as (group, member), rebuild the seed:
    /// enough members of enough groups.
    pub fn rebuilt_by(&self, got: &[(usize, usize)]) -> bool {
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
        self.line("slip39")
    }

    /// An old set's plan line: `slip39old {seed} ...`, the value as
    /// [`Slip39::to_line`] writes it (F293).
    pub(crate) fn to_old_line(&self) -> Text {
        self.line("slip39old")
    }

    fn line(&self, name: &str) -> Text {
        let mut out = text!(
            "{name} {} {} {} {} ",
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
        What::Written(c) if c.form == KeyForm::Slip39 => c.member(),
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

/// The seed, old set identifier, group and member of `w`, when it is a
/// kept old SLIP-39 share (F293).
pub fn kept_share_of(w: What) -> Option<(usize, u16, usize, usize)> {
    match w {
        What::Slip39Kept(i, id, g, m) => Some((i, id, g, m)),
        _ => None,
    }
}

/// The (group, member) of seed `i`'s kept shares of the old set with
/// identifier `id` among `ws`.
pub(crate) fn kept_shares_of(ws: &[What], i: usize, id: u16) -> Vec<(usize, usize)> {
    ws.iter()
        .filter_map(|&w| kept_share_of(w))
        .filter(|&(x, y, _, _)| x == i && y == id)
        .map(|(_, _, g, m)| (g, m))
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

impl Answers {
    /// Seed `i`'s old SLIP-39 sets whose shares the plan keeps, oldest
    /// first (F293).
    pub fn old_slip39_sets(&self, i: usize) -> impl Iterator<Item = &Slip39> {
        self.slip39_old.iter().filter(move |z| z.seed == i)
    }

    /// Seed `i`'s old SLIP-39 set with identifier `id`.
    pub fn old_slip39_with(&self, i: usize, id: u16) -> Option<&Slip39> {
        self.old_slip39_sets(i).find(|z| z.identifier == id)
    }

    /// Whether `set` can be one more old set of its seed: at most
    /// [`xor::MAX_OLD_SETS`] a seed, its identifier not already an old
    /// set's.
    pub fn can_keep_old_slip39(&self, set: &Slip39) -> bool {
        self.old_slip39_sets(set.seed).count() < xor::MAX_OLD_SETS
            && self.old_slip39_with(set.seed, set.identifier).is_none()
    }

    /// Records `set` as an old set; false (nothing recorded) when
    /// [`Answers::can_keep_old_slip39`] refuses it.
    pub(crate) fn add_old_slip39(&mut self, set: Slip39) -> bool {
        let ok = self.can_keep_old_slip39(&set);
        if ok {
            self.slip39_old.push(set);
        }
        ok
    }

    /// Seed `i`'s SLIP-39 set replaced by `new` (F293): each share of the
    /// current set the map placed stays at its place as a kept old
    /// share, the set recorded whole as an old set. `None` (nothing
    /// changed) when the seed has no set, `new` is not a sound set of it,
    /// or the plan could not keep the old one
    /// ([`Answers::can_keep_old_slip39`]) while it has shares on the map.
    /// A current set with no share on the map is not recorded: nothing of
    /// it is kept. `new`'s shares are not placed.
    pub(crate) fn replace_slip39(&mut self, i: usize, new: Slip39) -> Option<()> {
        let cur = self.slip39(i)?.clone();
        if new.seed != i || !new.is_sound() {
            return None;
        }
        let kept = |w: What| match w {
            What::Written(c) if c.form == KeyForm::Slip39 && c.seed == i => c
                .member()
                .map(|(_, g, m)| What::Slip39Kept(i, cur.identifier, g, m)),
            _ => None,
        };
        let mut m = self.map.clone().unwrap_or_default();
        let placed = m
            .iter()
            .any(|s| s.holds.iter().any(|(w, _)| kept(*w).is_some()));
        if placed && !self.can_keep_old_slip39(&cur) {
            return None;
        }
        for s in &mut m {
            for (w, _) in &mut s.holds {
                if let Some(k) = kept(*w) {
                    *w = k;
                }
            }
        }
        if placed {
            self.slip39_old.push(cur);
        }
        self.slip39.retain(|z| z.seed != i);
        self.slip39.push(new);
        self.slip39.sort_by_key(|z| z.seed);
        if self.map.is_some() {
            self.set_map(m);
        }
        Some(())
    }

    /// I destroyed it: the kept old share of seed `i`, old set `id`,
    /// group `g`, member `m` leaves the map wherever it is, and its old
    /// set's line goes when none of the set's shares is left on the map.
    /// Its notes go ([`notes_after_destroyed_slip39`]).
    pub fn destroy_kept_slip39(&mut self, i: usize, id: u16, g: usize, m: usize) {
        let Some(mut map) = self.map.clone() else {
            return;
        };
        for s in &mut map {
            s.holds.retain(|(w, _)| *w != What::Slip39Kept(i, id, g, m));
        }
        self.set_map(map);
    }
}

/// The notes of seed `i`'s current set `old`'s shares as they follow
/// [`sync`](crate::sets::sync) keeping that set as an old one: a note on
/// a share is on the kept old share with the same group and member.
pub fn notes_after_new_shares(notes: &[Note], i: usize, old: &Slip39) -> Vec<Note> {
    let kept = |w: What| match w {
        What::Written(c) if c.form == KeyForm::Slip39 && c.seed == i => c
            .member()
            .map(|(_, g, m)| What::Slip39Kept(i, old.identifier, g, m)),
        _ => None,
    };
    notes
        .iter()
        .map(|n| match n {
            Note::Checked(at, w, d) => Note::Checked(*at, kept(*w).unwrap_or(*w), d.clone()),
            Note::Marked(at, w, m) => Note::Marked(*at, kept(*w).unwrap_or(*w), *m),
            other => other.clone(),
        })
        .collect()
}

/// `notes` without those on the kept old share of seed `i`, old set `id`,
/// group `g`, member `m` ([`Answers::destroy_kept_slip39`]).
pub fn notes_after_destroyed_slip39(
    notes: &[Note],
    i: usize,
    id: u16,
    g: usize,
    m: usize,
) -> Vec<Note> {
    notes
        .iter()
        .filter(|n| {
            n.on()
                .is_none_or(|(_, w)| w != What::Slip39Kept(i, id, g, m))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape() -> Shape {
        Shape {
            m: 1,
            paths: Vec::new(),
            keys: 1,
            seeds: vec![Seed {
                name: Text::of("a"),
                here: true,
                shares: false,
                passphrase: false,
            }],
            splits: false,
            vaults: 0,
            seed_alone_fails: false,
            text_files: false,
        }
    }

    /// A one-group set `threshold` of `count`, members 0..`made` made.
    fn set(id: u16, threshold: u8, count: u8, made: u8) -> Slip39 {
        Slip39 {
            seed: 0,
            identifier: id,
            exponent: 0,
            group_threshold: 1,
            groups: vec![Group {
                threshold,
                count,
                made: (0..made).collect(),
            }],
        }
    }

    fn facts(s: &Slip39) -> faraday_vault::slip39::Slip39Facts {
        faraday_vault::slip39::Slip39Facts {
            identifier: s.identifier,
            exponent: s.exponent,
            threshold: s.group_threshold,
            groups: s
                .groups
                .iter()
                .map(|g| faraday_vault::slip39::Group {
                    threshold: g.threshold,
                    count: g.count,
                    made: g.made.iter().fold(0, |b, &m| b | 1 << m),
                })
                .collect(),
        }
    }

    fn share(m: usize) -> What {
        What::slip39_share(0, 0, m)
    }

    fn kept(id: u16, m: usize) -> What {
        What::Slip39Kept(0, id, 0, m)
    }

    /// A plan with `cur` as its set and its shares `0` and `1` placed,
    /// at one place or at two.
    fn planned(cur: Slip39, one_place: bool) -> Answers {
        let sh = shape();
        let mut a = Answers::defaults(&sh);
        a.set_slip39(&sh, 0, Some(cur));
        if one_place {
            sets::place_pieces(&mut a, &sh, vec![share(0)]);
            sets::place_pieces(&mut a, &sh, vec![share(1)]);
        } else {
            sets::place_pieces(&mut a, &sh, vec![share(0), share(1)]);
        }
        a
    }

    fn on_map(a: &Answers) -> Vec<What> {
        stored_map(&shape(), a)
            .iter()
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .collect()
    }

    fn sync_to(a: &mut Answers, key: &Slip39) {
        sets::sync(a, &shape(), &[None], &[Some(facts(key))]);
    }

    /// A plan with four old sets kept and a current one, shares placed.
    fn full() -> Answers {
        let sh = shape();
        let mut a = planned(set(5, 2, 3, 3), false);
        for id in 6..=9 {
            sync_to(&mut a, &set(id, 2, 3, 3));
            sets::place_pieces(&mut a, &sh, vec![share(0), share(1)]);
        }
        a
    }

    #[test]
    fn another_set_at_one_place_gives_the_key_there_and_the_plan_takes_the_key_set() {
        let mut a = planned(set(5, 2, 3, 3), true);
        let key = set(6, 2, 3, 3);
        sync_to(&mut a, &key);
        assert_eq!(a.slip39(0), Some(&key));
        let on = on_map(&a);
        assert!(
            !on.iter()
                .any(|w| matches!(w, What::Written(c) if c.form == KeyForm::Slip39))
        );
        assert!(on.contains(&kept(5, 0)) && on.contains(&kept(5, 1)));
        assert_eq!(a.set_needs().rebuilt(&on), vec![0]);
    }

    #[test]
    fn old_shares_at_two_places_give_the_key_only_together() {
        let mut a = planned(set(5, 2, 3, 3), false);
        sync_to(&mut a, &set(6, 2, 3, 3));
        let needs = a.set_needs();
        assert!(needs.rebuilt(&[kept(5, 0)]).is_empty());
        assert!(needs.rebuilt(&[kept(5, 1)]).is_empty());
        assert_eq!(needs.rebuilt(&[kept(5, 0), kept(5, 1)]), vec![0]);
        a.destroy_kept_slip39(0, 5, 0, 0);
        let left = on_map(&a);
        assert!(a.set_needs().rebuilt(&left).is_empty());
        assert!(a.to_text().contains("slip39old 0 5 "));
        a.destroy_kept_slip39(0, 5, 0, 1);
        assert_eq!(a.old_slip39_sets(0).count(), 0);
        assert!(!a.to_text().contains("slip39old"));
    }

    #[test]
    fn one_kept_and_one_current_share_rebuild_nothing() {
        let mut a = planned(set(5, 2, 3, 3), false);
        sync_to(&mut a, &set(6, 2, 3, 3));
        let needs = a.set_needs();
        assert!(needs.rebuilt(&[kept(5, 0), share(1)]).is_empty());
        assert!(!needs.counts(0, share(1), &[kept(5, 0), kept(5, 1), share(1)]));
        assert!(needs.counts(0, kept(5, 0), &[kept(5, 0), kept(5, 1)]));
    }

    #[test]
    fn a_placed_share_the_key_does_not_list_stays_and_counts() {
        let sh = shape();
        let mut a = Answers::defaults(&sh);
        a.set_slip39(&sh, 0, Some(set(5, 2, 3, 3)));
        sets::place_pieces(&mut a, &sh, vec![share(0), share(2)]);
        // The key lists members 0 and 1 only.
        sync_to(&mut a, &set(5, 2, 3, 2));
        assert_eq!(a.old_slip39_sets(0).count(), 0);
        assert!(on_map(&a).contains(&share(2)));
        assert!(a.slip39(0).is_some_and(|z| z.has_member(0, 2)));
        assert_eq!(a.set_needs().rebuilt(&[share(0), share(2)]), vec![0]);
    }

    #[test]
    fn a_plan_without_kept_shares_writes_the_same_text_and_kept_ones_read_back() {
        let sh = shape();
        let mut a = planned(set(5, 2, 3, 3), false);
        let text = a.to_text();
        assert!(!text.contains("slip39old") && !text.contains("slipk"));
        assert!(text.contains("slip0.0.0") && text.contains("slip0.0.1"));
        let back = Answers::from_text(&sh, &text).expect("reads");
        assert_eq!(back.to_text(), text);
        sync_to(&mut a, &set(6, 2, 3, 3));
        let text = a.to_text();
        assert!(text.contains("slip39old 0 5 "));
        assert!(text.contains("slipk0.5.0.0") && text.contains("slipk0.5.0.1"));
        let back = Answers::from_text(&sh, &text).expect("reads");
        assert_eq!(back, a);
        assert_eq!(back.to_text(), text);
    }

    #[test]
    fn a_fifth_old_set_a_repeated_identifier_and_stray_kept_shares_are_refused() {
        let sh = shape();
        let a = full();
        assert_eq!(a.old_slip39_sets(0).count(), xor::MAX_OLD_SETS);
        assert!(!a.can_keep_old_slip39(&set(30, 2, 3, 3)));
        assert!(!a.can_keep_old_slip39(&set(5, 2, 3, 3)));
        let text = a.to_text();
        assert!(Answers::from_text(&sh, &text).is_some());
        let mut five = text.clone();
        five.push_str("slip39old 0 40 0 1 2/3:0,1,2\n");
        assert!(Answers::from_text(&sh, &five).is_none());
        let mut twice = text.clone();
        twice.push_str("slip39old 0 5 0 1 2/3:0,1,2\n");
        assert!(Answers::from_text(&sh, &twice).is_none());
        // A kept share of no old set, or of a member not made.
        let (head, tail) = text.split_once("slipk0.5.0.0").expect("a kept share");
        let mut other = Text::of(head);
        other.push_str("slipk0.77.0.0");
        other.push_str(tail);
        assert!(Answers::from_text(&sh, &other).is_none());
        let mut absent = Text::of(head);
        absent.push_str("slipk0.5.0.9");
        absent.push_str(tail);
        assert!(Answers::from_text(&sh, &absent).is_none());
        // Off a place.
        let mut b = a.clone();
        let mut m = stored_map(&sh, &b);
        let w = (kept(5, 0), Tag::Secret);
        m.iter_mut().for_each(|s| s.holds.retain(|h| *h != w));
        m.push(Spot {
            at: At::Files,
            holds: vec![w],
        });
        b.set_map(m);
        assert!(Answers::from_text(&sh, &b.to_text()).is_none());
    }

    #[test]
    fn a_plan_that_cannot_keep_the_old_set_keeps_its_own() {
        let mut a = full();
        let own = a.slip39(0).cloned().expect("a current set");
        let before = a.clone();
        let key = set(20, 2, 3, 3);
        sync_to(&mut a, &key);
        assert_eq!(a, before);
        assert!(sets::keeps_own_slip39(&a, 0, &facts(&key)));
        // The plan's own set, or one it keeps as old, is not that case.
        assert!(!sets::keeps_own_slip39(&a, 0, &facts(&own)));
        assert!(!sets::keeps_own_slip39(&a, 0, &facts(&set(5, 2, 3, 3))));
    }
}
