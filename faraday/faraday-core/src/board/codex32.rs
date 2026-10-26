//! The board's codex32 parts (`docs/CODEX32.md` §9.4, §9.5): a seed kept
//! as codex32 strings (its key's secret is a set that exists) is a group
//! in the list, its head the set and one
//! chip per string by its index; each string is placed as any chip is, in
//! the forms its zone has (on paper the string or its Codex32QR drawn, on
//! a stick a text file or a picture, in a vault a share record); and the
//! checks the strings add. What is here reads the plan it is given; the
//! presses reach it from `Faraday::board_act`.

use super::*;
use crate::plan::codex32::{C32, index_char, strings_at};

/// A seed's codex32 group, as the list draws it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct C32Group {
    /// The seed, by its place among the wallet's.
    pub seed: usize,
    /// "9a6a2580 · codex32 w0za · 2 of 3".
    pub head: String,
    /// The strings, a chip each.
    pub strings: Vec<Entry>,
}

/// A string's chip name: "C".
pub(crate) fn string_name(j: usize) -> String {
    index_char(j).to_ascii_uppercase().to_string()
}

/// A form's head line: "9a6a2580 · codex32 w0za · 2 of 3".
fn head(seed: &str, f: &C32) -> String {
    let split = if f.k == 0 {
        EN.c32_secret_alone.to_string()
    } else {
        fill(EN.c32_k_of_n, &[&f.k.to_string(), &f.n().to_string()])
    };
    fill(EN.c32_board_head_id, &[seed, &f.id_text(), &split])
}

/// The codex32 groups of the list: each seed here that has a set, its
/// strings a chip each. A seed with no set has none: a key of words is
/// not split into strings (`docs/CODEX32.md` §9.2). `entry` makes a list
/// chip's entry.
pub(crate) fn groups(shape: &Shape, a: &Answers, entry: &dyn Fn(Thing) -> Entry) -> Vec<C32Group> {
    shape
        .here()
        .into_iter()
        .filter_map(|i| {
            let f = a.c32(i)?;
            Some(C32Group {
                seed: i,
                head: head(shape.seeds[i].name.as_str(), f),
                strings: f
                    .indices
                    .iter()
                    .map(|&j| Entry {
                        name: string_name(j),
                        ..entry(Thing::String(i, j))
                    })
                    .collect(),
            })
        })
        .collect()
}

/// The checks the codex32 strings add (`docs/CODEX32.md` §9.5), each a
/// line under the board: a string kept nowhere; a place that keeps enough
/// strings to be the seed; a vault that holds a seed and a string of it,
/// or a set's seed whole; a place whose loss leaves too few strings; a
/// string as a file in the clear. `place` names a place.
pub(crate) fn warnings(
    shape: &Shape,
    a: &Answers,
    m: &[Spot],
    place: &dyn Fn(usize) -> String,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let places = plan::places_of(m).max(1);
    for f in &a.codex32 {
        let i = f.seed;
        let seed = shape.seeds.get(i).map_or("", |s| s.name.as_str());
        let set = f.id_text();
        let anywhere = strings_at(m, i, &|_| true);
        for &j in &f.indices {
            if !anywhere.contains(&j) {
                out.push(fill(EN.c32_warn_nowhere, &[&string_name(j), seed]));
            }
        }
        // What a place keeps, through the vaults whose sticks are there.
        let at_place = |p: usize| {
            let vaults: Vec<usize> = m
                .iter()
                .filter(|s| matches!(s.at, At::Place(q) | At::Stick(q, _) if q == p))
                .flat_map(|s| s.holds.iter())
                .filter_map(|(w, _)| match w {
                    What::VaultStick(v) => Some(*v),
                    _ => None,
                })
                .collect();
            strings_at(m, i, &|at| match at {
                At::Place(q) | At::Stick(q, _) => q == p,
                At::Vault(v) => vaults.contains(&v),
                _ => false,
            })
        };
        let need = f.need();
        for p in 0..places {
            let here = at_place(p);
            if f.k >= 2 && here.len() >= need {
                out.push(fill(
                    EN.c32_warn_place_seed,
                    &[&place(p), &here.len().to_string(), &set],
                ));
            }
        }
        if f.k >= 2 && anywhere.len() >= need {
            for p in 0..places {
                let rest = strings_at(
                    m,
                    i,
                    &|at| !matches!(at, At::Place(q) | At::Stick(q, _) if q == p),
                );
                if rest.len() < need {
                    out.push(fill(
                        EN.c32_warn_lose_place,
                        &[&place(p), &need.to_string(), &set],
                    ));
                    break;
                }
            }
        }
        for s in m.iter().filter(|s| matches!(s.at, At::Vault(_))) {
            let At::Vault(v) = s.at else {
                continue;
            };
            let seed_in = s
                .holds
                .iter()
                .any(|(w, _)| matches!(w, What::Seed(x) | What::SeedPassphrase(x) if *x == i));
            let string_in = s
                .holds
                .iter()
                .any(|(w, _)| matches!(w, What::Codex32Share(x, _) if *x == i));
            let vault = (v + 1).to_string();
            if seed_in && string_in {
                out.push(fill(EN.c32_warn_vault_both, &[&vault, seed]));
            } else if seed_in && f.k >= 2 {
                out.push(fill(EN.c32_warn_vault_whole, &[&vault, seed]));
            }
        }
        let file = m.iter().any(|s| {
            s.holds.iter().any(|(w, _)| {
                matches!(w, What::Codex32ShareFile(x, _) | What::Codex32QrFile(x, _) if *x == i)
            })
        });
        if file {
            out.push(fill(EN.c32_warn_file, &[&set]));
        }
    }
    out
}

/// The forms a placed chip of `t` offers in zone `z`: [`forms_of`], less a
/// codex32 string's Codex32QR where its strings are not 48 characters,
/// the one length the Codex32QR profile has (`docs/CODEX32.md` §10.4).
pub(crate) fn forms_shown(a: &Answers, z: Zone, t: Thing) -> Vec<What> {
    let qr = match t {
        Thing::String(i, _) => a.c32(i).is_none_or(|f| f.length == 48),
        _ => true,
    };
    forms_of(z, t)
        .into_iter()
        .filter(|w| qr || !matches!(w, What::Codex32QrPaper(..) | What::Codex32QrFile(..)))
        .collect()
}

/// Place `p`'s lines of fact counting codex32 shares (`docs/CODEX32.md`
/// §9.5): for each seed kept as strings, the distinct indices of its set
/// across the place's zones (its paper, its sticks' files), "1 of 2 shares
/// of 9A6A2580 · alone: nothing" while too few, "2 of 2 shares of
/// 9A6A2580: the seed" once enough; a string in a vault whose stick the
/// place keeps counts with that vault's passphrase, as a vault's seed does
/// (`plan::alone`).
pub(crate) fn place_lines(
    shape: &Shape,
    a: &Answers,
    m: &plan::Mapped,
    p: usize,
) -> Vec<(String, plan::AloneKind)> {
    if a.codex32.is_empty() {
        return Vec::new();
    }
    let vaults: Vec<usize> = m
        .map
        .iter()
        .filter(|s| matches!(s.at, At::Stick(q, _) if q == p))
        .flat_map(|s| s.holds.iter())
        .filter_map(|(w, _)| match *w {
            What::VaultStick(v) => Some(v),
            _ => None,
        })
        .collect();
    let mut out = Vec::new();
    for f in &a.codex32 {
        let Some(seed) = shape.seeds.get(f.seed) else {
            continue;
        };
        let clear = strings_at(&m.map, f.seed, &|at| match at {
            At::Place(q) | At::Stick(q, _) => q == p,
            _ => false,
        });
        let mut with = clear.clone();
        with.extend(strings_at(
            &m.map,
            f.seed,
            &|at| matches!(at, At::Vault(v) if vaults.contains(&v)),
        ));
        with.sort_unstable();
        with.dedup();
        if with.is_empty() {
            continue;
        }
        let need = f.need();
        let whole = |js: &[usize]| js.len() >= need || js.contains(&plan::codex32::SECRET);
        let (n, k) = (clear.len().to_string(), need.to_string());
        out.push(if whole(&clear) {
            (
                fill(EN.c32_place_seed, &[&n, &k, &seed.name]),
                plan::AloneKind::SeesBalance,
            )
        } else if whole(&with) {
            (
                fill(
                    EN.c32_place_vault,
                    &[&with.len().to_string(), &k, &seed.name],
                ),
                plan::AloneKind::NeedsVault,
            )
        } else {
            (
                fill(
                    EN.c32_place_nothing,
                    &[&with.len().to_string(), &k, &seed.name],
                ),
                plan::AloneKind::Nothing,
            )
        });
    }
    out
}

/// Vault `v`'s lines on the backup checklist's vault item (`docs/BACKUP-FLOW.md`
/// §17.23): for each seed kept as strings whose strings the plan puts in
/// the vault, the seed by its place among the wallet's, the share indices
/// kept, and the line "1 of 2 shares of 9A6A2580" ([`place_lines`] counts the
/// same way, by the strings of the set a place keeps). A vault keeps shares
/// only: a set's secret goes in as a key record.
pub(crate) fn vault_lines(
    shape: &Shape,
    a: &Answers,
    m: &plan::Mapped,
    v: usize,
) -> Vec<(usize, Vec<usize>, String)> {
    let strings = m.vault_strings(a, v);
    let mut out = Vec::new();
    for f in &a.codex32 {
        let Some(seed) = shape.seeds.get(f.seed) else {
            continue;
        };
        let js: Vec<usize> = strings
            .iter()
            .filter(|&&(i, _)| i == f.seed)
            .map(|&(_, j)| j)
            .collect();
        if js.is_empty() {
            continue;
        }
        let line = fill(
            EN.c32_shares_of,
            &[&js.len().to_string(), &f.need().to_string(), &seed.name],
        );
        out.push((f.seed, js, line));
    }
    out
}

/// Place `p`'s lines of fact (`docs/DESIGN.md` §4.17): what it sees of the
/// balance alone, how near it brings a spend, and the codex32 shares it
/// keeps ([`place_lines`]).
pub(crate) fn facts(
    shape: &Shape,
    a: &Answers,
    m: &plan::Mapped,
    p: usize,
    alone: plan::Alone,
) -> Vec<(String, plan::AloneKind)> {
    let (t, k) = alone.balance_tag();
    let mut v = vec![(t.to_string(), k)];
    v.extend(plan::reach_on(shape, m, p).map(plan::Reach::tag));
    v.extend(place_lines(shape, a, m, p));
    v
}
