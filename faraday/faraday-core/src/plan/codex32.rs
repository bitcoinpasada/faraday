//! A seed kept as codex32 strings (`docs/CODEX32.md` §9.2, §9.4, §9.5):
//! the seed's codex32 form on the plan, its strings as things the map
//! places, what the checks count of them, and The standard way for a key
//! that already has a set.
//!
//! A string is named by its seed and its share index, the index as its
//! bech32 value (`s`, the secret, is 16), so a chip, a plan line and a
//! check read the same string the same way whatever else changes: a
//! share added later has an index of its own, and a lost one replaced has
//! the index it had.
//!
//! **Two kinds of form.** A set the plan makes (§9.2): its `k` and `n`
//! are the person's, its strings made at the checklist from this device's
//! randomness, at the indices BIP 93 gives a split, in order. A set that
//! exists (§9.4): the key was made by Make codex32 shares or loaded from
//! strings, so its strings are on paper already; its `k` and indices are
//! the set's, fixed, and the checklist checks them rather than making any.

use super::*;
use faraday_vault::codex32::{SetFacts, State};

/// The bech32 characters, in value order.
pub const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// The secret's share index, as its value.
pub const SECRET: usize = 16;

/// A share index's character, from its value.
pub fn index_char(j: usize) -> char {
    char::from(CHARSET[j % 32])
}

/// A share index's value, from its character (either case).
pub fn index_of(c: u8) -> Option<usize> {
    CHARSET.iter().position(|&x| x == c.to_ascii_lowercase())
}

/// The string length a seed of `bytes` bytes is written in (BIP 93): 48
/// characters for 16 bytes, 74 for 32, 127 for 64; `None` for a length
/// codex32 has no string for.
pub fn length_for(bytes: usize) -> Option<u8> {
    Some(match bytes {
        16 => 48,
        20 => 54,
        24 => 61,
        28 => 67,
        32 => 74,
        64 => 127,
        _ => return None,
    })
}

/// A seed's codex32 form on the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C32 {
    /// The seed, by its place among the wallet's.
    pub seed: usize,
    /// How many strings rebuild the seed: 0 for the secret alone, else 2
    /// to 9.
    pub k: u8,
    /// Each string's length in characters.
    pub length: u8,
    /// The strings' share indices, as values, in their order: a split's
    /// in BIP 93's order, a share added after them.
    pub indices: Vec<usize>,
    /// The set's identifier, once the set exists: made by Make codex32
    /// shares, loaded from strings, or made at the checklist. While it is
    /// `None` the plan makes the set, and its `k` and `n` may change.
    pub identifier: Option<[u8; 4]>,
    /// Strings of the set known to exist whose index is not known
    /// (§9.4, loaded from strings): counted, never placed.
    pub unknown: u8,
}

impl C32 {
    /// A set the plan makes: `k` of `n` (`k` 0 for the secret alone, `n`
    /// then 1), `length` characters, at the indices a split has.
    pub fn planned(seed: usize, k: u8, n: u8, length: u8) -> C32 {
        let indices = if k == 0 {
            vec![SECRET]
        } else {
            faraday_bip::codex32::INDEX_ORDER
                .iter()
                .take(usize::from(n.clamp(k, faraday_bip::codex32::MAX_SHARES)))
                .filter_map(|&c| index_of(c))
                .collect()
        };
        C32 {
            seed,
            k,
            length,
            indices,
            identifier: None,
            unknown: 0,
        }
    }

    /// The set that exists, from the facts its key keeps: the strings
    /// destroyed are not among those to place.
    pub fn of_set(seed: usize, set: &SetFacts) -> C32 {
        C32 {
            seed,
            k: set.threshold,
            length: set.length,
            indices: set
                .strings
                .iter()
                .filter(|&&(_, s)| s != State::Destroyed)
                .filter_map(|&(c, _)| index_of(c))
                .collect(),
            identifier: Some(set.identifier),
            unknown: set.unknown,
        }
    }

    /// The set exists: its `k`, `n` and indices are fixed.
    pub fn fixed(&self) -> bool {
        self.identifier.is_some()
    }

    /// How many strings rebuild the seed: one for the secret alone.
    pub fn need(&self) -> usize {
        usize::from(self.k).max(1)
    }

    /// How many strings the set has, known or not.
    pub fn n(&self) -> usize {
        self.indices.len() + usize::from(self.unknown)
    }

    /// The identifier as text, where the set exists.
    pub fn id_text(&self) -> Option<String> {
        self.identifier
            .map(|id| id.iter().map(|&b| char::from(b)).collect())
    }

    /// The form's plan line: `codex32 {seed} {k} {length} {indices} {id}
    /// {unknown}`, the indices as their characters and `-` for no
    /// identifier.
    pub fn to_line(&self) -> String {
        let indices: String = self.indices.iter().map(|&j| index_char(j)).collect();
        format!(
            "codex32 {} {} {} {} {} {}\n",
            self.seed,
            self.k,
            self.length,
            if indices.is_empty() {
                "-".to_string()
            } else {
                indices
            },
            self.id_text().unwrap_or_else(|| "-".to_string()),
            self.unknown
        )
    }

    /// [`C32::to_line`]'s value read back (after `codex32 `); `None` for
    /// anything that is not a form `seeds` seeds could have.
    pub fn from_value(value: &str, seeds: usize) -> Option<C32> {
        let mut parts = value.split(' ');
        let seed: usize = parts.next()?.parse().ok()?;
        let k: u8 = parts.next()?.parse().ok()?;
        let length: u8 = parts.next()?.parse().ok()?;
        let indices: Vec<usize> = match parts.next()? {
            "-" => Vec::new(),
            s => s.bytes().map(index_of).collect::<Option<Vec<usize>>>()?,
        };
        let identifier = match parts.next()? {
            "-" => None,
            s => Some(<[u8; 4]>::try_from(s.as_bytes()).ok()?),
        };
        let unknown: u8 = parts.next()?.parse().ok()?;
        let form = C32 {
            seed,
            k,
            length,
            indices,
            identifier,
            unknown,
        };
        let facts = SetFacts {
            threshold: k,
            identifier: identifier.unwrap_or(*b"qqqq"),
            length,
            unknown,
            strings: form
                .indices
                .iter()
                .map(|&j| (CHARSET[j], State::Made))
                .collect(),
        };
        (seed < seeds && parts.next().is_none() && facts.is_sound()).then_some(form)
    }
}

/// Whether `w` is a codex32 string of a seed, in any form: on paper, its
/// Codex32QR drawn, a file of either, a share record in a vault.
pub fn string_of(w: What) -> Option<(usize, usize)> {
    match w {
        What::Codex32Share(i, j)
        | What::Codex32QrPaper(i, j)
        | What::Codex32ShareFile(i, j)
        | What::Codex32QrFile(i, j) => Some((i, j)),
        _ => None,
    }
}

/// The thresholds of the plan's codex32 forms, as (seed, strings that
/// rebuild it): what the checks count strings against.
pub type Needs = Vec<(usize, usize)>;

impl Answers {
    /// Seed `i`'s codex32 form, where it has one.
    pub fn c32(&self, i: usize) -> Option<&C32> {
        self.codex32.iter().find(|f| f.seed == i)
    }

    /// The plan's codex32 thresholds ([`Needs`]).
    pub fn c32_needs(&self) -> Needs {
        self.codex32.iter().map(|f| (f.seed, f.need())).collect()
    }

    /// Sets seed `i`'s codex32 form, replacing the one it had, or takes
    /// it off with `None`. Strings the map placed whose index the new form
    /// lacks leave the map.
    pub fn set_c32(&mut self, shape: &Shape, i: usize, form: Option<C32>) {
        let keep: Vec<usize> = form.as_ref().map_or(Vec::new(), |f| f.indices.clone());
        self.codex32.retain(|f| f.seed != i);
        if let Some(f) = form {
            self.codex32.push(f);
            self.codex32.sort_by_key(|f| f.seed);
        }
        if self.map.is_none() && keep.is_empty() {
            return;
        }
        let mut m = stored_map(shape, self);
        for s in &mut m {
            s.holds
                .retain(|(w, _)| string_of(*w).is_none_or(|(si, j)| si != i || keep.contains(&j)));
        }
        self.set_map(m);
    }
}

/// What strings of each codex32 seed a spot keeps, counted against the
/// plan's [`Needs`]: the seeds they rebuild.
pub fn rebuilt(needs: &[(usize, usize)], seen: &[(usize, usize)]) -> Vec<usize> {
    needs
        .iter()
        .filter(|&&(i, k)| {
            let mut js: Vec<usize> = seen
                .iter()
                .filter(|&&(si, _)| si == i)
                .map(|&(_, j)| j)
                .collect();
            js.sort_unstable();
            js.dedup();
            js.len() >= k || js.contains(&SECRET)
        })
        .map(|&(i, _)| i)
        .collect()
}

/// The distinct strings of seed `i` the spots `at` keep on the map
/// `boxes`, by index.
pub fn strings_at(boxes: &[Spot], i: usize, at: &dyn Fn(At) -> bool) -> Vec<usize> {
    let mut js: Vec<usize> = boxes
        .iter()
        .filter(|s| at(s.at))
        .flat_map(|s| s.holds.iter())
        .filter_map(|(w, _)| string_of(*w).filter(|&(si, _)| si == i).map(|(_, j)| j))
        .collect();
    js.sort_unstable();
    js.dedup();
    js
}

/// Costs a map of the plan. The standard way for a wallet some of whose
/// keys already have a codex32 set (§9.5, `docs/BACKUP-FLOW.md` §17.21): as The standard way
/// places it, with each such seed's strings on paper in place of its
/// words, one string per place: the seed's own place keeps its first
/// string, and each further string a place of its own, with the whole
/// description as the standard way puts it at every place. `sets` is
/// each seed's set, where its key here has one. Read-only until Customize
/// backup, as any new plan.
pub fn standard(c: &Compute, shape: &Shape, sets: &[Option<SetFacts>]) -> Answers {
    let mut a = Answers::preset(shape, Preset::Standard);
    let forms: Vec<C32> = sets
        .iter()
        .enumerate()
        .filter(|&(i, s)| s.is_some() && shape.seeds.get(i).is_some_and(|s| s.here))
        .filter_map(|(i, s)| s.as_ref().map(|s| C32::of_set(i, s)))
        .filter(|f| !f.indices.is_empty())
        .collect();
    if forms.is_empty() {
        return a;
    }
    let mut m = map(c, shape, &a);
    for f in &forms {
        let i = f.seed;
        let home = m.iter().position(|s| {
            matches!(s.at, At::Place(_))
                && s.holds
                    .iter()
                    .any(|(w, _)| matches!(w, What::Words(x) | What::SeedQr(x) if *x == i))
        });
        for s in &mut m {
            s.holds
                .retain(|(w, _)| !matches!(w, What::Words(x) | What::SeedQr(x) if *x == i));
        }
        let sheet = |m: &[Spot]| {
            m.iter().any(|s| {
                matches!(s.at, At::Place(_)) && s.holds.iter().any(|(w, _)| *w == What::Sheet)
            })
        };
        let with_sheet = sheet(&m);
        for (n, &j) in f.indices.iter().enumerate() {
            let string = (What::Codex32Share(i, j), Tag::Secret);
            match home.filter(|_| n == 0) {
                Some(h) => m[h].holds.insert(0, string),
                None => {
                    let p = m.iter().filter(|s| matches!(s.at, At::Place(_))).count();
                    let mut holds = vec![string];
                    if with_sheet {
                        holds.push((What::Sheet, Tag::Public));
                    }
                    let at = m
                        .iter()
                        .rposition(|s| matches!(s.at, At::Place(_) | At::Stick(..)))
                        .map_or(0, |x| x + 1);
                    m.insert(
                        at,
                        Spot {
                            at: At::Place(p),
                            holds,
                        },
                    );
                }
            }
        }
    }
    a.codex32 = forms;
    a.set_map(m);
    a
}

/// `a` brought in step with the sets its keys have (`docs/CODEX32.md`
/// §9.4): a seed here whose key has a set takes that set as its form,
/// fixed, in place of one the plan was to make or an older record of it.
/// A plan saved before the set existed reads with it, its strings not
/// placed yet.
pub fn sync(a: &mut Answers, shape: &Shape, sets: &[Option<SetFacts>]) {
    for (i, set) in sets.iter().enumerate() {
        let Some(set) = set else {
            continue;
        };
        if !shape.seeds.get(i).is_some_and(|s| s.here) {
            continue;
        }
        let form = C32::of_set(i, set);
        if a.c32(i) != Some(&form) {
            a.set_c32(shape, i, Some(form));
        }
    }
}

/// The most places a plan of `shape` and `a` has: [`max_places`], and
/// each codex32 seed's further strings a place more (§9.5: three places
/// for 2 of 3).
pub fn max_places_of(shape: &Shape, a: &Answers) -> usize {
    max_places(shape)
        + a.codex32
            .iter()
            .map(|f| f.indices.len().saturating_sub(1))
            .sum::<usize>()
}

/// The blank templates the codex32 strings on paper take, by length:
/// (characters, how many), each string written by hand on one
/// (`docs/CODEX32.md` §9.3).
pub fn templates_of(a: &Answers, m: &Mapped) -> Vec<(u8, usize)> {
    let mut out: Vec<(u8, usize)> = Vec::new();
    for s in m.map.iter().filter(|s| matches!(s.at, At::Place(_))) {
        for (w, _) in &s.holds {
            if let What::Codex32Share(i, _) = w
                && let Some(f) = a.c32(*i)
            {
                match out.iter_mut().find(|(l, _)| *l == f.length) {
                    Some((_, n)) => *n += 1,
                    None => out.push((f.length, 1)),
                }
            }
        }
    }
    out.sort_unstable();
    out
}

/// The strings of seed `i` on paper at the plan's places, by index, each
/// once, in the set's order: what its checklist item makes or checks.
pub fn paper_strings(a: &Answers, m: &Mapped, i: usize) -> Vec<usize> {
    let placed = strings_at(&m.map, i, &|at| matches!(at, At::Place(_)));
    a.c32(i).map_or(Vec::new(), |f| {
        f.indices
            .iter()
            .copied()
            .filter(|j| placed.contains(j))
            .collect()
    })
}

/// The places that keep string `j` of seed `i`, on paper.
pub fn places_of_string(m: &Mapped, i: usize, j: usize) -> Vec<usize> {
    m.map
        .iter()
        .filter_map(|s| match s.at {
            At::Place(p)
                if s.holds
                    .iter()
                    .any(|(w, _)| matches!(w, What::Codex32Share(x, y) | What::Codex32QrPaper(x, y) if *x == i && *y == j)) =>
            {
                Some(p)
            }
            _ => None,
        })
        .collect()
}

impl Mapped {
    /// The codex32 strings the map keeps in vault `v`, each as a share
    /// record (`docs/VAULT.md` §7 type 15): seed and index, of a set split
    /// with a threshold of 2 or more (the secret is the seed, kept as a
    /// key record).
    pub fn vault_strings(&self, a: &Answers, v: usize) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = holds_in(&self.map, At::Vault(v))
            .into_iter()
            .filter_map(|w| match w {
                What::Codex32Share(i, j) => Some((i, j)),
                _ => None,
            })
            .filter(|&(i, j)| j != SECRET && a.c32(i).is_some_and(|f| f.k >= 2))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The codex32 strings the map keeps as files on a stick at a place:
    /// each seed and index, of a set split with a threshold of 2 or more.
    pub fn stick_strings(&self, a: &Answers) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = self
            .map
            .iter()
            .filter(|s| matches!(s.at, At::Stick(..)))
            .flat_map(|s| s.holds.iter())
            .filter_map(|(w, _)| match *w {
                What::Codex32ShareFile(i, j) | What::Codex32QrFile(i, j) => Some((i, j)),
                _ => None,
            })
            .filter(|&(i, j)| j != SECRET && a.c32(i).is_some_and(|f| f.k >= 2))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}
