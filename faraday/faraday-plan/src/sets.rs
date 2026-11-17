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
}

/// The plan's sets, as the checks count pieces against them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Needs(Vec<Rule>);

/// The seed `w` is a piece of, when it is a piece of a set.
pub fn piece_of(w: What) -> Option<usize> {
    codex32::string_of(w)
        .map(|(i, _)| i)
        .or_else(|| xor::part_of(w).map(|(i, _)| i))
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
            Rule::Codex32(i, _) | Rule::Xor(i, _) | Rule::Slip39(i, _) => *i,
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
                .collect(),
        )
    }
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
