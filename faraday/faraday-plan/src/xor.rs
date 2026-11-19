//! A seed kept as Seed XOR parts (`docs/decisions/F276.md`): the set's
//! public facts on the plan, its parts as things the map places on paper.
//!
//! A part is named by its seed and its number in the set, from 0. Every
//! part is needed to rebuild the seed. The plan keeps how many parts
//! there are and each part's own master fingerprint, never a part: the
//! fingerprint is public and lets a part be checked without the others.

use super::*;
use faraday_text::{Text, text};

/// The most old sets of one seed a plan keeps parts of (F285).
pub const MAX_OLD_SETS: usize = 4;

/// The most parts a set has.
pub const MAX_PARTS: usize = faraday_entropy::MAX_XOR_PARTS;

/// A seed's Seed XOR set on the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xor {
    /// The seed, by its place among the wallet's.
    pub seed: usize,
    /// Each part's master fingerprint, in the parts' order.
    pub parts: Vec<[u8; 4]>,
}

impl Xor {
    /// Seed `seed`'s set as its key keeps it (`docs/VAULT.md` §7).
    pub fn of_facts(seed: usize, f: &faraday_vault::xor::XorFacts) -> Xor {
        Xor {
            seed,
            parts: f.parts.clone(),
        }
    }

    /// How many parts the set has: all of them rebuild the seed.
    pub fn n(&self) -> usize {
        self.parts.len()
    }

    /// The form's plan line: `xor {seed} {fingerprints}`, the
    /// fingerprints in hex, joined by commas.
    pub(crate) fn to_line(&self) -> Text {
        self.line("xor")
    }

    /// An old set's plan line: `xorold {seed} {fingerprints}` (F285).
    pub(crate) fn to_old_line(&self) -> Text {
        self.line("xorold")
    }

    fn line(&self, name: &str) -> Text {
        let mut out = text!("{name} {} ", self.seed);
        for (j, fp) in self.parts.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            out.push_str(&fp_code(*fp));
        }
        out.push('\n');
        out
    }

    /// [`Xor::to_line`]'s value read back (after `xor `); `None` for
    /// anything that is not a set a wallet of `seeds` seeds could have.
    pub(crate) fn from_value(value: &str, seeds: usize) -> Option<Xor> {
        let (seed, fps) = value.split_once(' ')?;
        let seed: usize = seed.parse().ok().filter(|&s| s < seeds)?;
        let parts = fps
            .split(',')
            .map(text::fp_read)
            .collect::<Option<Vec<[u8; 4]>>>()?;
        (faraday_entropy::MIN_XOR_PARTS..=MAX_PARTS)
            .contains(&parts.len())
            .then_some(Xor { seed, parts })
    }
}

/// The seed and part number of `w`, when it is a Seed XOR part.
pub fn part_of(w: What) -> Option<(usize, usize)> {
    match w {
        What::XorPart(i, j) => Some((i, j)),
        _ => None,
    }
}

/// The numbers of seed `i`'s parts among `ws`.
pub(crate) fn parts_of(ws: &[What], i: usize) -> Vec<usize> {
    ws.iter()
        .filter_map(|&w| part_of(w))
        .filter(|&(x, _)| x == i)
        .map(|(_, j)| j)
        .collect()
}

impl Answers {
    /// Seed `i`'s Seed XOR set, where it has one.
    pub fn xor(&self, i: usize) -> Option<&Xor> {
        self.xor.iter().find(|x| x.seed == i)
    }

    /// Sets seed `i`'s Seed XOR set, replacing the one it had, or takes
    /// it off with `None`. Parts the map placed whose number the new set
    /// lacks leave the map.
    pub fn set_xor(&mut self, shape: &Shape, i: usize, set: Option<Xor>) {
        let n = set.as_ref().map_or(0, Xor::n);
        self.xor.retain(|x| x.seed != i);
        if let Some(x) = set {
            self.xor.push(x);
            self.xor.sort_by_key(|x| x.seed);
        }
        if self.map.is_none() {
            return;
        }
        let mut m = stored_map(shape, self);
        for s in &mut m {
            s.holds
                .retain(|(w, _)| part_of(*w).is_none_or(|(x, j)| x != i || j < n));
        }
        self.set_map(m);
    }
}

impl Answers {
    /// Seed `i`'s old sets whose parts the plan keeps, oldest first.
    pub fn old_sets(&self, i: usize) -> impl Iterator<Item = &Xor> {
        self.xor_old.iter().filter(move |x| x.seed == i)
    }

    /// Seed `i`'s old set that has a part with fingerprint `fp`.
    pub fn old_set_with(&self, i: usize, fp: [u8; 4]) -> Option<&Xor> {
        self.old_sets(i).find(|x| x.parts.contains(&fp))
    }

    /// Whether `set` can be one more old set of its seed: at most
    /// [`MAX_OLD_SETS`] a seed, none of its fingerprints in another.
    pub(crate) fn can_keep_old(&self, set: &Xor) -> bool {
        self.old_sets(set.seed).count() < MAX_OLD_SETS
            && set
                .parts
                .iter()
                .all(|&fp| self.old_set_with(set.seed, fp).is_none())
    }

    /// Records `set` as an old set; false (nothing recorded) when
    /// [`Answers::can_keep_old`] refuses it.
    pub(crate) fn add_old_set(&mut self, set: Xor) -> bool {
        let ok = self.can_keep_old(&set);
        if ok {
            self.xor_old.push(set);
        }
        ok
    }

    /// Drops each old set none of whose parts is on the map: with no map
    /// of its own, none is.
    pub(crate) fn drop_unkept_old_sets(&mut self) {
        let on: Vec<What> = self
            .map
            .iter()
            .flatten()
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .collect();
        self.xor_old.retain(|x| {
            x.parts
                .iter()
                .any(|&fp| on.contains(&What::XorKept(x.seed, fp)))
        });
        self.slip39_old.retain(|z| {
            on.iter().any(|w| {
                matches!(*w, What::Slip39Kept(i, id, _, _) if i == z.seed && id == z.identifier)
            })
        });
    }

    /// Seed `i`'s Seed XOR set replaced by `new` (**Make new parts**,
    /// `docs/decisions/F285.md`): each part of the current set the map
    /// placed stays at its place as a kept old part, the set recorded
    /// whole as an old set. `None` (nothing changed) when the seed has no
    /// set, `new` is not a set of it, or the plan could not keep the old
    /// one ([`Answers::can_keep_old`]) while it has parts on the map.
    /// A current set with no part on the map is not recorded: nothing
    /// of it is kept.
    pub(crate) fn replace_xor(&mut self, i: usize, new: Xor) -> Option<()> {
        let cur = self.xor(i)?.clone();
        if new.seed != i || !(faraday_entropy::MIN_XOR_PARTS..=MAX_PARTS).contains(&new.n()) {
            return None;
        }
        let kept = |w: What| match w {
            What::XorPart(x, j) if x == i => cur.parts.get(j).map(|&fp| What::XorKept(i, fp)),
            _ => None,
        };
        let mut m = self.map.clone().unwrap_or_default();
        let placed = m
            .iter()
            .any(|s| s.holds.iter().any(|(w, _)| kept(*w).is_some()));
        if placed && !self.can_keep_old(&cur) {
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
            self.xor_old.push(cur);
        }
        self.xor.retain(|x| x.seed != i);
        self.xor.push(new);
        self.xor.sort_by_key(|x| x.seed);
        if self.map.is_some() {
            self.set_map(m);
        }
        Some(())
    }

    /// **Make new parts** (`docs/decisions/F285.md`): the key's set is
    /// now `new`, its parts placed one to a place as a first split's are,
    /// and the old set's placed parts stay as kept old parts
    /// ([`Answers::replace_xor`]). Notes on the old parts follow them
    /// ([`notes_after_new_parts`], called with the old set read first).
    pub fn make_new_parts(&mut self, shape: &Shape, i: usize, new: Xor) -> Option<()> {
        self.replace_xor(i, new)?;
        let fresh: Vec<What> = self
            .xor(i)
            .map(|x| (0..x.n()).map(|j| What::XorPart(i, j)).collect())
            .unwrap_or_default();
        sets::place_pieces(self, shape, fresh);
        Some(())
    }

    /// I destroyed it: the kept old part of seed `i` with fingerprint
    /// `fp` leaves the map wherever it is, and its old set's line goes
    /// when none of the set's parts is left on the map. Its notes go
    /// ([`notes_after_destroyed`]).
    pub fn destroy_kept(&mut self, i: usize, fp: [u8; 4]) {
        let Some(mut m) = self.map.clone() else {
            return;
        };
        for s in &mut m {
            s.holds.retain(|(w, _)| *w != What::XorKept(i, fp));
        }
        self.set_map(m);
    }
}

/// The notes of seed `i`'s current set `old`'s parts as they follow
/// [`Answers::make_new_parts`]: a note on part `j` is on the kept old
/// part with that part's fingerprint.
pub fn notes_after_new_parts(notes: &[Note], i: usize, old: &Xor) -> Vec<Note> {
    let kept = |w: What| match w {
        What::XorPart(x, j) if x == i => old.parts.get(j).map(|&fp| What::XorKept(i, fp)),
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

/// `notes` without those on the kept old part of seed `i` with
/// fingerprint `fp` ([`Answers::destroy_kept`]).
pub fn notes_after_destroyed(notes: &[Note], i: usize, fp: [u8; 4]) -> Vec<Note> {
    notes
        .iter()
        .filter(|n| n.on().is_none_or(|(_, w)| w != What::XorKept(i, fp)))
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
        }
    }

    fn set(n: u8, base: u8) -> Xor {
        Xor {
            seed: 0,
            parts: (0..n).map(|j| [base, base, base, j]).collect(),
        }
    }

    /// A plan whose key has a two-part set, its parts placed.
    fn planned() -> Answers {
        let sh = shape();
        let mut a = Answers::defaults(&sh);
        a.set_xor(&sh, 0, Some(set(2, 1)));
        sets::place_pieces(&mut a, &sh, vec![What::XorPart(0, 0), What::XorPart(0, 1)]);
        a
    }

    fn on_map(a: &Answers) -> Vec<What> {
        stored_map(&shape(), a)
            .iter()
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .collect()
    }

    fn kept(base: u8, j: u8) -> What {
        What::XorKept(0, [base, base, base, j])
    }

    #[test]
    fn a_plan_without_kept_parts_writes_the_same_text() {
        let a = planned();
        let text = a.to_text();
        assert!(!text.contains("xorold") && !text.contains("xork"));
        assert!(text.contains("xor 0 01010100,01010101\n"));
        assert!(text.contains("xorp0.0") && text.contains("xorp0.1"));
        let back = Answers::from_text(&shape(), &text).expect("reads");
        assert_eq!(back.to_text(), text);
    }

    #[test]
    fn kept_old_parts_read_back() {
        let sh = shape();
        let mut a = planned();
        assert!(a.make_new_parts(&sh, 0, set(3, 2)).is_some());
        assert_eq!(a.xor(0), Some(&set(3, 2)));
        assert_eq!(a.old_sets(0).collect::<Vec<_>>(), vec![&set(2, 1)]);
        let on = on_map(&a);
        assert!(on.contains(&kept(1, 0)) && on.contains(&kept(1, 1)));
        assert!(!on.contains(&What::XorPart(0, 0)) || a.xor(0).is_some());
        assert!(on.contains(&What::XorPart(0, 2)));
        let text = a.to_text();
        assert!(text.contains("xorold 0 01010100,01010101\n"));
        assert!(text.contains("xork0.01010100"));
        let back = Answers::from_text(&sh, &text).expect("reads");
        assert_eq!(back, a);
        assert_eq!(back.to_text(), text);
    }

    #[test]
    fn a_kept_part_off_a_place_or_of_no_old_set_is_refused() {
        let sh = shape();
        let mut a = planned();
        a.make_new_parts(&sh, 0, set(2, 2)).expect("made");
        let text = a.to_text();
        // No old set for the part.
        let mut no_set = Text::new();
        for l in text.lines().filter(|l| !l.starts_with("xorold")) {
            no_set.push_str(l);
            no_set.push('\n');
        }
        assert!(Answers::from_text(&sh, &no_set).is_none());
        // A fingerprint the old set lacks.
        let (head, tail) = text.split_once("xork0.01010100").expect("a kept part");
        let mut other = Text::of(head);
        other.push_str("xork0.ffffffff");
        other.push_str(tail);
        assert!(Answers::from_text(&sh, &other).is_none());
        // Off a place.
        let mut m = stored_map(&sh, &a);
        let w = (What::XorKept(0, [1, 1, 1, 0]), Tag::Secret);
        m.iter_mut().for_each(|s| s.holds.retain(|h| *h != w));
        m.push(Spot {
            at: At::Files,
            holds: vec![w],
        });
        let mut b = a.clone();
        b.set_map(m);
        assert!(Answers::from_text(&sh, &b.to_text()).is_none());
    }

    #[test]
    fn a_whole_kept_old_set_rebuilds_the_seed() {
        let mut a = planned();
        a.make_new_parts(&shape(), 0, set(2, 2)).expect("made");
        let needs = a.set_needs();
        let old = [kept(1, 0), kept(1, 1)];
        assert_eq!(needs.rebuilt(&old), vec![0]);
        assert!(needs.enough(0, &old));
    }

    #[test]
    fn one_old_part_destroyed_and_the_old_set_rebuilds_nothing() {
        let sh = shape();
        let mut a = planned();
        a.make_new_parts(&sh, 0, set(2, 2)).expect("made");
        a.destroy_kept(0, [1, 1, 1, 0]);
        assert!(!on_map(&a).contains(&kept(1, 0)));
        assert!(on_map(&a).contains(&kept(1, 1)));
        let old: Vec<What> = on_map(&a)
            .into_iter()
            .filter(|w| matches!(w, What::XorKept(..)))
            .collect();
        assert!(a.set_needs().rebuilt(&old).is_empty());
    }

    #[test]
    fn old_and_new_parts_never_combine() {
        let mut a = planned();
        a.make_new_parts(&shape(), 0, set(2, 2)).expect("made");
        let needs = a.set_needs();
        let mixed = [kept(1, 0), What::XorPart(0, 1)];
        assert!(needs.rebuilt(&mixed).is_empty());
        let mixed = [kept(1, 0), kept(1, 1), What::XorPart(0, 0)];
        assert_eq!(needs.rebuilt(&mixed), vec![0]);
        assert!(
            !needs
                .rebuilt(&[What::XorPart(0, 0), kept(1, 1)])
                .contains(&0)
        );
    }

    #[test]
    fn a_kept_part_of_a_broken_old_set_counts_for_no_whole_one() {
        let sh = shape();
        let mut a = planned();
        a.make_new_parts(&sh, 0, set(2, 2)).expect("made");
        a.make_new_parts(&sh, 0, set(2, 3)).expect("made");
        a.destroy_kept(0, [2, 2, 2, 0]);
        let needs = a.set_needs();
        // The first old set is whole; of the second only one part is left.
        let held = [kept(1, 0), kept(1, 1), kept(2, 1)];
        assert!(needs.counts(0, kept(1, 0), &held));
        assert!(needs.counts(0, kept(1, 1), &held));
        assert!(!needs.counts(0, kept(2, 1), &held));
    }

    #[test]
    fn a_fifth_old_set_is_refused() {
        let sh = shape();
        let mut a = planned();
        for base in 2..=5 {
            assert!(a.make_new_parts(&sh, 0, set(2, base)).is_some());
        }
        assert_eq!(a.old_sets(0).count(), MAX_OLD_SETS);
        let before = a.clone();
        assert!(a.make_new_parts(&sh, 0, set(2, 6)).is_none());
        assert_eq!(a, before);
        // One old set gone, there is room again.
        a.destroy_kept(0, [1, 1, 1, 0]);
        a.destroy_kept(0, [1, 1, 1, 1]);
        assert!(a.make_new_parts(&sh, 0, set(2, 6)).is_some());
    }

    #[test]
    fn an_old_set_with_no_part_on_the_map_is_dropped() {
        let sh = shape();
        let mut a = planned();
        a.make_new_parts(&sh, 0, set(2, 2)).expect("made");
        a.destroy_kept(0, [1, 1, 1, 0]);
        assert_eq!(a.old_sets(0).count(), 1);
        a.destroy_kept(0, [1, 1, 1, 1]);
        assert_eq!(a.old_sets(0).count(), 0);
        assert!(!a.to_text().contains("xorold"));
        // A saved line with no part of it on the map reads as none.
        let mut text = planned().to_text();
        text.push_str("xorold 0 01010100,01010101\n");
        let read = Answers::from_text(&sh, &text).expect("reads");
        assert_eq!(read.old_sets(0).count(), 0);
    }

    #[test]
    fn a_plan_whose_set_is_not_its_keys_keeps_its_parts_as_old() {
        let sh = shape();
        let mut a = planned();
        let facts = faraday_vault::xor::XorFacts {
            parts: set(3, 2).parts,
        };
        sets::sync_placed(&mut a, &sh, &[Some(facts)], &[]);
        assert_eq!(a.xor(0), Some(&set(3, 2)));
        assert_eq!(a.old_sets(0).count(), 1);
        let on = on_map(&a);
        assert!(on.contains(&kept(1, 0)) && on.contains(&kept(1, 1)));
        for j in 0..3 {
            assert!(on.contains(&What::XorPart(0, j)));
        }
    }

    #[test]
    fn the_parts_item_counts_the_new_set_only() {
        let sh = shape();
        let mut a = planned();
        a.make_new_parts(&sh, 0, set(2, 2)).expect("made");
        let parts = |a: &Answers| {
            let map = stored_map(&sh, a);
            let m = Mapped {
                places: by_place(&map),
                map,
                needs: a.set_needs(),
                check: std::cell::OnceCell::new(),
            };
            checklist_on(&sh, a, &m)
                .iter()
                .filter(|i| **i == Item::Parts(0))
                .count()
        };
        assert_eq!(parts(&a), 1);
        // With the new parts gone, kept old parts make no item.
        let mut b = a.clone();
        b.set_xor(&sh, 0, Some(set(2, 2)));
        let mut m = stored_map(&sh, &b);
        m.iter_mut()
            .for_each(|s| s.holds.retain(|(w, _)| !matches!(w, What::XorPart(..))));
        b.set_map(m);
        assert_eq!(parts(&b), 0);
    }
}
