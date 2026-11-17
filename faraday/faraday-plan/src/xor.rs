//! A seed kept as Seed XOR parts (`docs/decisions/F276.md`): the set's
//! public facts on the plan, its parts as things the map places on paper.
//!
//! A part is named by its seed and its number in the set, from 0. Every
//! part is needed to rebuild the seed. The plan keeps how many parts
//! there are and each part's own master fingerprint, never a part: the
//! fingerprint is public and lets a part be checked without the others.

use super::*;
use faraday_text::{Text, text};

/// The most parts a set has.
pub const MAX_PARTS: usize = 9;

/// A seed's Seed XOR set on the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xor {
    /// The seed, by its place among the wallet's.
    pub seed: usize,
    /// Each part's master fingerprint, in the parts' order.
    pub parts: Vec<[u8; 4]>,
}

impl Xor {
    /// How many parts the set has: all of them rebuild the seed.
    pub fn n(&self) -> usize {
        self.parts.len()
    }

    /// The form's plan line: `xor {seed} {fingerprints}`, the
    /// fingerprints in hex, joined by commas.
    pub(crate) fn to_line(&self) -> Text {
        let mut out = text!("xor {} ", self.seed);
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
        (2..=MAX_PARTS)
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
