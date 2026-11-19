//! What the checks count of a seed kept as a set of pieces: codex32
//! strings, Seed XOR parts, SLIP-39 shares. Each set rebuilds its seed
//! from enough of its pieces, wherever they are (`docs/CODEX32.md` §9.5,
//! `docs/decisions/F276.md`).

use super::*;

/// How one seed's set is rebuilt.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Rule {
    /// codex32: `k` strings of the seed, or its secret alone.
    Codex32(usize, usize),
    /// Seed XOR: all `n` parts.
    Xor(usize, usize),
    /// SLIP-39: the set's own quorum.
    Slip39(usize, slip39::Slip39),
    /// An old Seed XOR set (F285): every one of its parts kept, named by
    /// fingerprint. Old and new parts never combine.
    XorOld(usize, Vec<[u8; 4]>),
    /// An old SLIP-39 set (F293): its own quorum met by kept shares of its
    /// identifier. Old and new shares never combine.
    Slip39Old(usize, slip39::Slip39),
}

/// The plan's sets, as the checks count pieces against them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Needs(Vec<Rule>);

/// The seed `w` is a piece of, when it is a piece of a set.
pub fn piece_of(w: What) -> Option<usize> {
    codex32::string_of(w)
        .map(|(i, _)| i)
        .or_else(|| xor::part_of(w).map(|(i, _)| i))
        .or(match w {
            What::XorKept(i, _) => Some(i),
            _ => None,
        })
        .or_else(|| slip39::share_of(w).map(|(i, _, _)| i))
        .or_else(|| slip39::kept_share_of(w).map(|(i, _, _, _)| i))
}

impl Needs {
    /// Whether seed `i` is kept as a set.
    pub fn covers(&self, i: usize) -> bool {
        self.0.iter().any(|r| r.seed() == i)
    }

    /// Whether the pieces `ws` of seed `i` rebuild it, by one of its sets.
    pub fn enough(&self, i: usize, ws: &[What]) -> bool {
        self.0
            .iter()
            .filter(|r| r.seed() == i)
            .any(|r| r.enough(ws))
    }

    /// Whether the piece `w` of seed `i` is one of the pieces of a set of
    /// it that `ws` rebuild: a kept old part counts only while its old set
    /// is whole, and a new part only for the set in use.
    pub fn counts(&self, i: usize, w: What, ws: &[What]) -> bool {
        self.0
            .iter()
            .filter(|r| r.seed() == i && r.enough(ws))
            .any(|r| match (r, w) {
                (Rule::XorOld(_, fps), What::XorKept(_, fp)) => fps.contains(&fp),
                (Rule::Slip39Old(_, set), What::Slip39Kept(_, id, _, _)) => set.identifier == id,
                (Rule::XorOld(..) | Rule::Slip39Old(..), _)
                | (_, What::XorKept(..) | What::Slip39Kept(..)) => false,
                _ => true,
            })
    }

    /// The seeds the pieces `seen` rebuild.
    pub fn rebuilt(&self, seen: &[What]) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for r in &self.0 {
            let i = r.seed();
            if !out.contains(&i) && r.enough(seen) {
                out.push(i);
            }
        }
        out
    }
}

impl Rule {
    fn seed(&self) -> usize {
        match self {
            Rule::Codex32(i, _)
            | Rule::Xor(i, _)
            | Rule::Slip39(i, _)
            | Rule::XorOld(i, _)
            | Rule::Slip39Old(i, _) => *i,
        }
    }

    /// Whether `ws` (pieces of any seed) rebuild this rule's seed.
    fn enough(&self, ws: &[What]) -> bool {
        match self {
            Rule::Codex32(i, k) => {
                let mut js: Vec<usize> = ws
                    .iter()
                    .filter_map(|&w| codex32::string_of(w))
                    .filter(|&(x, _)| x == *i)
                    .map(|(_, j)| j)
                    .collect();
                js.sort_unstable();
                js.dedup();
                js.len() >= *k || js.contains(&codex32::SECRET)
            }
            Rule::Xor(i, n) => {
                let mut js: Vec<usize> = xor::parts_of(ws, *i);
                js.retain(|j| j < n);
                js.sort_unstable();
                js.dedup();
                js.len() >= *n
            }
            Rule::XorOld(i, fps) => fps.iter().all(|&fp| ws.contains(&What::XorKept(*i, fp))),
            Rule::Slip39(i, set) => {
                let mut got: Vec<(usize, usize)> = slip39::shares_of(ws, *i);
                got.sort_unstable();
                got.dedup();
                set.rebuilt_by(&got)
            }
            Rule::Slip39Old(i, set) => {
                let mut got = slip39::kept_shares_of(ws, *i, set.identifier);
                got.sort_unstable();
                got.dedup();
                set.rebuilt_by(&got)
            }
        }
    }
}

impl Answers {
    /// The plan's sets ([`Needs`]).
    pub(crate) fn set_needs(&self) -> Needs {
        Needs(
            self.codex32
                .iter()
                .map(|f| Rule::Codex32(f.seed, f.need()))
                .chain(self.xor.iter().map(|x| Rule::Xor(x.seed, x.n())))
                .chain(self.slip39.iter().map(|s| Rule::Slip39(s.seed, s.clone())))
                .chain(
                    self.xor_old
                        .iter()
                        .map(|x| Rule::XorOld(x.seed, x.parts.clone())),
                )
                .chain(
                    self.slip39_old
                        .iter()
                        .map(|s| Rule::Slip39Old(s.seed, s.clone())),
                )
                .collect(),
        )
    }
}

/// `a` brought in step with the Seed XOR and SLIP-39 sets its keys keep
/// (`docs/decisions/F276.md`): a seed here whose key has a set of a kind
/// and whose plan has none of that kind takes it, its parts not placed
/// yet. One set of a kind per seed: a set the plan already has stays.
/// `xor` and `slip39` are each seed's set, where its key has one.
pub fn sync(
    a: &mut Answers,
    shape: &Shape,
    xor: &[Option<faraday_vault::xor::XorFacts>],
    slip39: &[Option<faraday_vault::slip39::Slip39Facts>],
) {
    let here = |i: usize| shape.seeds.get(i).is_some_and(|s| s.here);
    for (i, f) in xor.iter().enumerate() {
        if let Some(f) = f
            && here(i)
        {
            let set = xor::Xor::of_facts(i, f);
            match a.xor(i) {
                None => a.set_xor(shape, i, Some(set)),
                // A plan whose set is not its key's (F285): the plan's
                // placed parts become kept old parts and the key's set
                // is the current one, not placed yet. A plan that cannot
                // keep them (too many old sets) stays as it is.
                // A set the plan keeps as an old one already: a stale
                // copy of the key brings nothing back.
                Some(cur) if *cur != set && a.old_sets(i).all(|o| *o != set) => {
                    let _ = a.replace_xor(i, set);
                }
                Some(_) => {}
            }
        }
    }
    for (i, f) in slip39.iter().enumerate() {
        if let Some(f) = f
            && here(i)
        {
            let key = slip39::Slip39::of_facts(i, f);
            match a.slip39(i).cloned() {
                None => a.set_slip39(shape, i, Some(key)),
                // The same set (F293): the key's facts, its members made
                // and the plan's placed shares the key does not list.
                Some(cur) if cur.same_set(&key) => {
                    let merged = a.merged_slip39(&cur, key);
                    if merged != cur {
                        a.slip39.retain(|z| z.seed != i);
                        a.slip39.push(merged);
                        a.slip39.sort_by_key(|z| z.seed);
                    }
                }
                // A set the plan keeps as an old one already: a stale
                // copy of the key brings nothing back.
                Some(_) if a.old_slip39_sets(i).any(|o| o.same_set(&key)) => {}
                // Another set (F293): the plan's placed shares become
                // kept old shares and the key's set is the current one,
                // not placed yet. A plan that cannot keep them stays as
                // it is.
                Some(_) => {
                    let _ = a.replace_slip39(i, key);
                }
            }
        }
    }
}

/// Whether seed `i`'s Seed XOR set on the plan `a` is another set than
/// its key's `f` that the plan keeps as its own: the screen's "This plan
/// keeps another set than the key's" (F293). Run after [`sync`]: a plan
/// that took the key's set, or that keeps the key's set as an old one, is
/// not in this case.
pub fn keeps_own_xor(a: &Answers, i: usize, f: &faraday_vault::xor::XorFacts) -> bool {
    let key = xor::Xor::of_facts(i, f);
    a.xor(i).is_some_and(|cur| *cur != key) && a.old_sets(i).all(|o| *o != key)
}

/// [`keeps_own_xor`] for seed `i`'s SLIP-39 set against its key's `f`.
pub fn keeps_own_slip39(a: &Answers, i: usize, f: &faraday_vault::slip39::Slip39Facts) -> bool {
    let key = slip39::Slip39::of_facts(i, f);
    a.slip39(i).is_some_and(|cur| !cur.same_set(&key))
        && a.old_slip39_sets(i).all(|o| !o.same_set(&key))
}

impl Answers {
    /// `key` (the same set as `cur`, seed `cur.seed`) with its members
    /// made and, beside them, the plan's placed shares of the set that
    /// the key does not list (F293).
    fn merged_slip39(&self, cur: &slip39::Slip39, mut key: slip39::Slip39) -> slip39::Slip39 {
        let on: Vec<What> = self
            .map
            .iter()
            .flatten()
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .collect();
        for (g, m) in slip39::shares_of(&on, cur.seed) {
            let Some(grp) = key.groups.get_mut(g) else {
                continue;
            };
            let Ok(m) = u8::try_from(m) else { continue };
            if !grp.made.contains(&m) {
                grp.made.push(m);
                grp.made.sort_unstable();
            }
            grp.count = grp.count.max(m + 1);
        }
        key
    }
}

/// [`sync`] for a plan just started: each part of each set also placed on
/// paper, one to a place (the first parts at the places there are, the
/// rest at places of their own). A part already placed stays where it is.
pub fn sync_placed(
    a: &mut Answers,
    shape: &Shape,
    xor: &[Option<faraday_vault::xor::XorFacts>],
    slip39: &[Option<faraday_vault::slip39::Slip39Facts>],
) {
    sync(a, shape, xor, slip39);
    let mut parts: Vec<What> = a
        .xor
        .iter()
        .flat_map(|x| (0..x.n()).map(|j| What::XorPart(x.seed, j)))
        .collect();
    for z in &a.slip39 {
        for (g, grp) in z.groups.iter().enumerate() {
            parts.extend(
                grp.made
                    .iter()
                    .map(|&m| What::Slip39Share(z.seed, g, usize::from(m))),
            );
        }
    }
    place_pieces(a, shape, parts);
}

/// Places each of `parts` on paper, one to a place (the first at the
/// places there are, the rest at places of their own); a part already on
/// the map stays where it is.
pub(crate) fn place_pieces(a: &mut Answers, shape: &Shape, parts: Vec<What>) {
    if parts.is_empty() {
        return;
    }
    let mut m = stored_map(shape, a);
    let mut next = 0usize;
    for w in parts {
        if m.iter().any(|s| s.holds.iter().any(|(x, _)| *x == w)) {
            continue;
        }
        let places = m.iter().filter(|s| matches!(s.at, At::Place(_))).count();
        match m.iter().position(|s| s.at == At::Place(next)) {
            Some(k) => m[k].holds.push((w, Tag::Secret)),
            None => {
                let at = m
                    .iter()
                    .rposition(|s| matches!(s.at, At::Place(_) | At::Stick(..)))
                    .map_or(0, |x| x + 1);
                m.insert(
                    at,
                    Spot {
                        at: At::Place(places),
                        holds: vec![(w, Tag::Secret)],
                    },
                );
            }
        }
        next += 1;
    }
    a.set_map(m);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_xor_parts_past_the_set_do_not_rebuild_it() {
        let needs = Needs(vec![Rule::Xor(0, 2)]);
        let seen = [What::XorPart(0, 0), What::XorPart(0, 7)];
        assert!(needs.rebuilt(&seen).is_empty());
        let both = [What::XorPart(0, 0), What::XorPart(0, 1)];
        assert_eq!(needs.rebuilt(&both), vec![0]);
    }
}
