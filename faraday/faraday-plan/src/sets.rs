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
                (Rule::XorOld(..), _) | (_, What::XorKept(..)) => false,
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
            Rule::Codex32(i, _) | Rule::Xor(i, _) | Rule::Slip39(i, _) | Rule::XorOld(i, _) => *i,
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
                .collect(),
        )
    }
}

/// `a` brought in step with the Seed XOR and SLIP-39 sets its keys keep
/// (`docs/decisions/F276.md`): a seed here whose key has a set of a kind
/// and whose plan has none of that kind takes it, its parts not placed
/// yet. One set of a kind per seed: a set the plan has that is not its
/// key's gives way to the key's.
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
                Some(cur) if *cur != set => {
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
            let set = slip39::Slip39::of_facts(i, f);
            match a.slip39(i) {
                Some(cur) if *cur == set => {}
                // A plan whose set is not its key's, as a codex32 form
                // that is not the key's: the key's set is the plan's.
                // Shares placed of a set with another identifier are of
                // no set of this key and leave the map; of the same set,
                // those the key's set made stay.
                Some(cur) => {
                    if cur.identifier != set.identifier {
                        a.set_slip39(shape, i, None);
                    }
                    a.set_slip39(shape, i, Some(set));
                }
                None => a.set_slip39(shape, i, Some(set)),
            }
        }
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

    fn shape() -> Shape {
        Shape {
            m: 1,
            paths: Vec::new(),
            keys: 1,
            seeds: vec![Seed {
                name: faraday_text::Text::of("a"),
                here: true,
                shares: false,
                passphrase: false,
            }],
            splits: false,
            vaults: 0,
            seed_alone_fails: false,
        }
    }

    fn facts(identifier: u16, made: u16) -> faraday_vault::slip39::Slip39Facts {
        faraday_vault::slip39::Slip39Facts {
            identifier,
            exponent: 0,
            threshold: 1,
            groups: vec![faraday_vault::slip39::Group {
                threshold: 2,
                count: 3,
                made,
            }],
        }
    }

    fn placed(a: &Answers, sh: &Shape) -> usize {
        stored_map(sh, a)
            .iter()
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .filter(|w| matches!(w, What::Slip39Share(..)))
            .count()
    }

    #[test]
    fn a_plan_whose_slip39_set_is_not_its_keys_takes_the_keys() {
        let sh = shape();
        let mut a = Answers::defaults(&sh);
        sync(&mut a, &sh, &[], &[Some(facts(5, 0b111))]);
        let key = |f| Some(slip39::Slip39::of_facts(0, &f));
        assert_eq!(a.slip39(0).cloned(), key(facts(5, 0b111)));
        sync(&mut a, &sh, &[], &[Some(facts(9, 0b011))]);
        assert_eq!(a.slip39(0).cloned(), key(facts(9, 0b011)));
    }

    #[test]
    fn shares_placed_of_another_slip39_set_leave_the_map() {
        let sh = shape();
        let mut a = Answers::defaults(&sh);
        sync_placed(&mut a, &sh, &[], &[Some(facts(5, 0b111))]);
        assert_eq!(placed(&a, &sh), 3);
        sync(&mut a, &sh, &[], &[Some(facts(9, 0b111))]);
        assert_eq!(placed(&a, &sh), 0);
    }

    #[test]
    fn shares_the_keys_set_did_not_make_leave_the_map() {
        let sh = shape();
        let mut a = Answers::defaults(&sh);
        sync_placed(&mut a, &sh, &[], &[Some(facts(5, 0b111))]);
        sync(&mut a, &sh, &[], &[Some(facts(5, 0b011))]);
        assert_eq!(placed(&a, &sh), 2);
    }
}
