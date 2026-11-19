//! The board's Seed XOR and SLIP-39 parts (`docs/decisions/F276.md`): a
//! seed split into a set that exists is a group in the list, its head the
//! set and one chip per part made. A part is paper only: it goes on a
//! place's paper and nowhere else, and reads "Not checked" until its copy
//! typed back here matched. What is here reads the plan it is given.

use super::codex32::C32Group;
use super::*;

/// What a part's line reads: where it is kept, and that it was not typed
/// back here.
fn line(count: &str) -> String {
    if count.is_empty() {
        EN.backups_not_checked.to_string()
    } else {
        fill(EN.board_part_count, &[count, EN.backups_not_checked])
    }
}

/// The Seed XOR and SLIP-39 groups of the list: each seed here whose key
/// keeps such a set, a chip per part (per share made). `entry` makes a
/// list chip's entry.
pub(crate) fn groups(shape: &Shape, a: &Answers, entry: &dyn Fn(Thing) -> Entry) -> Vec<C32Group> {
    let part = |t: Thing, name: String| {
        let e = entry(t);
        Entry {
            name,
            count: line(&e.count),
            ..e
        }
    };
    let mut out = Vec::new();
    for i in shape.here() {
        let seed = shape.seeds[i].name.as_str();
        if let Some(x) = a.xor(i) {
            out.push(C32Group {
                seed: i,
                head: fill(EN.board_xor_head, &[seed, &x.n().to_string()]),
                strings: (0..x.n())
                    .map(|j| {
                        part(
                            Thing::Part(i, j),
                            fill(EN.board_xor_part, &[&(j + 1).to_string()]),
                        )
                    })
                    .collect(),
            });
        }
        // Each old set whose parts the plan keeps: a group of its own,
        // a chip per part still on the map.
        for x in a.old_sets(i) {
            let chips: Vec<Entry> = x
                .parts
                .iter()
                .map(|&fp| (fp, entry(Thing::Kept(i, fp))))
                .filter(|(_, e)| !e.count.is_empty())
                .map(|(fp, e)| Entry {
                    name: fill(
                        EN.board_xor_old_chip,
                        &[&crate::wallet_backup::old_part_number(a, i, fp).to_string()],
                    ),
                    ..e
                })
                .collect();
            if !chips.is_empty() {
                out.push(C32Group {
                    seed: i,
                    head: fill(EN.board_xor_old_head, &[seed]),
                    strings: chips,
                });
            }
        }
        if let Some(z) = a.slip39(i) {
            let mut chips = Vec::new();
            for (g, grp) in z.groups.iter().enumerate() {
                for &m in &grp.made {
                    let m = usize::from(m);
                    chips.push(part(
                        Thing::Slip(i, g, m),
                        fill(
                            EN.board_slip39_part,
                            &[&(g + 1).to_string(), &(m + 1).to_string()],
                        ),
                    ));
                }
            }
            out.push(C32Group {
                seed: i,
                head: fill(
                    EN.board_slip39_head,
                    &[
                        seed,
                        &z.group_threshold.to_string(),
                        &z.groups.len().to_string(),
                    ],
                ),
                strings: chips,
            });
        }
        // Each old SLIP-39 set whose shares the plan keeps: a group of
        // its own, a chip per kept share still on the map.
        for z in a.old_slip39_sets(i) {
            let mut chips = Vec::new();
            for (g, grp) in z.groups.iter().enumerate() {
                for &m in &grp.made {
                    let m = usize::from(m);
                    let e = entry(Thing::SlipKept(i, z.identifier, g, m));
                    if !e.count.is_empty() {
                        chips.push(Entry {
                            name: fill(
                                EN.board_slip39_old_chip,
                                &[&(g + 1).to_string(), &(m + 1).to_string()],
                            ),
                            ..e
                        });
                    }
                }
            }
            if !chips.is_empty() {
                out.push(C32Group {
                    seed: i,
                    head: fill(EN.board_slip39_old_head, &[seed]),
                    strings: chips,
                });
            }
        }
    }
    out
}
