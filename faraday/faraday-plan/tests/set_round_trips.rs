//! A plan made for a seed kept as a set (codex32, SLIP-39 or Seed XOR)
//! reads back from its saved text as that set, and a set the crate's limits
//! do not allow (a threshold over the count, too many parts, a count
//! outside the set's range) is refused whatever else the plan says.
//!
//! Each property runs a fixed number of cases from a fixed seed, so a run is
//! the same everywhere and leaves no regression files.

use faraday_plan::codex32::{C32, CHARSET, SECRET};
use faraday_plan::slip39::{Group, Slip39};
use faraday_plan::xor::Xor;
use faraday_plan::{Answers, Seed, Shape};
use faraday_text::Text;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed};

fn config(cases: u32) -> Config {
    Config {
        cases,
        failure_persistence: None,
        max_shrink_iters: 8,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x0fa7_ada2),
        ..Config::default()
    }
}

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

/// The text of a plan holding the set `put` puts in, read back.
fn saved(put: impl FnOnce(&mut Answers, &Shape)) -> String {
    let shape = shape();
    let mut a = Answers::defaults(&shape);
    put(&mut a, &shape);
    String::from(&*a.to_text())
}

fn read(text: &str) -> Option<Answers> {
    Answers::from_text(&shape(), text)
}

/// A group of a SLIP-39 set within the standard's limits: 1 to 16 members,
/// a threshold of 1 only for a group of one, and the members made.
fn group() -> impl Strategy<Value = Group> {
    (1u8..=16, 1u8..=16, prop::collection::vec(any::<bool>(), 16)).prop_map(
        |(count, threshold, made)| {
            let (count, threshold) = if threshold == 1 {
                (1, 1)
            } else {
                (count.max(2), threshold.min(count.max(2)))
            };
            Group {
                threshold,
                count,
                made: (0..count).filter(|&m| made[usize::from(m)]).collect(),
            }
        },
    )
}

fn slip39() -> impl Strategy<Value = Slip39> {
    (
        0u16..1 << 15,
        0u8..=15,
        prop::collection::vec(group(), 1..=16),
        any::<prop::sample::Index>(),
    )
        .prop_map(|(identifier, exponent, groups, at)| Slip39 {
            seed: 0,
            identifier,
            exponent,
            group_threshold: u8::try_from(1 + at.index(groups.len())).unwrap(),
            groups,
        })
}

proptest! {
    #![proptest_config(config(200))]

    /// A SLIP-39 set of groups within the standard's limits, with any of its
    /// members made, reads back as the same set.
    #[test]
    fn a_slip39_set_reads_back_as_the_set(set in slip39()) {
        let text = saved(|a, s| a.set_slip39(s, 0, Some(set.clone())));
        let a = read(&text).expect("the plan reads");
        prop_assert_eq!(a.slip39(0), Some(&set));
        let again = a.to_text();
        prop_assert_eq!(again.as_str(), text.as_str());
    }

    /// A SLIP-39 set with a threshold over its count, a group threshold over
    /// its groups, too many groups or an identifier of more than 15 bits is
    /// refused.
    #[test]
    fn a_slip39_set_over_the_limits_is_refused(
        set in slip39(),
        fault in 0usize..4,
        over in 1u8..=4,
    ) {
        let mut bad = set;
        match fault {
            0 => {
                let g = &mut bad.groups[0];
                g.threshold = g.count + over;
            }
            1 => bad.group_threshold = u8::try_from(bad.groups.len()).unwrap() + over,
            2 => {
                let last = bad.groups[0].clone();
                bad.groups.resize(16 + usize::from(over), last);
            }
            _ => bad.identifier = (1 << 15) + u16::from(over),
        }
        let text = saved(|a, s| a.set_slip39(s, 0, Some(bad.clone())));
        prop_assert!(read(&text).is_none(), "{:?}", bad);
    }
}

/// A codex32 set: the secret alone, or a threshold of 2 to 9 with up to 31
/// strings, known or not.
fn c32() -> impl Strategy<Value = C32> {
    (
        prop::sample::select(vec![0u8, 2, 3, 4, 5, 6, 7, 8, 9]),
        prop::sample::select(vec![48u8, 54, 61, 67, 74, 127]),
        prop::collection::vec(prop::sample::select(CHARSET.to_vec()), 4),
        prop::collection::vec(any::<bool>(), 32),
        0u8..=31,
    )
        .prop_map(|(k, length, id, kept, unknown)| {
            let indices: Vec<usize> = if k == 0 {
                vec![SECRET]
            } else {
                (0..32).filter(|&j| kept[j] && j != SECRET).collect()
            };
            let unknown = if k == 0 {
                0
            } else {
                unknown.min(u8::try_from(31 - indices.len()).unwrap())
            };
            C32 {
                seed: 0,
                k,
                length,
                indices,
                identifier: <[u8; 4]>::try_from(id).unwrap(),
                unknown,
            }
        })
}

proptest! {
    #![proptest_config(config(200))]

    /// A codex32 set, whole or shared, reads back as the same set, with how
    /// many strings it needs and has.
    #[test]
    fn a_codex32_set_reads_back_as_the_set(set in c32()) {
        let text = saved(|a, s| a.set_c32(s, 0, Some(set.clone())));
        let a = read(&text).expect("the plan reads");
        prop_assert_eq!(a.c32(0), Some(&set));
        prop_assert_eq!(a.c32(0).map(C32::need), Some(set.need()));
        prop_assert_eq!(a.c32(0).map(C32::n), Some(set.n()));
        let again = a.to_text();
        prop_assert_eq!(again.as_str(), text.as_str());
    }

    /// A codex32 set with a threshold of 1 or over 9, a length the standard
    /// has none of, more than 31 strings, or a secret kept with others is
    /// refused.
    #[test]
    fn a_codex32_set_over_the_limits_is_refused(
        set in c32(),
        fault in 0usize..5,
    ) {
        let mut bad = set;
        match fault {
            0 => bad.k = 1,
            1 => bad.k = 10,
            2 => bad.length = 50,
            3 => {
                bad.k = 2;
                bad.indices = (0..32).collect();
                bad.unknown = 1;
            }
            _ => {
                bad.k = 0;
                bad.indices = vec![SECRET, 0];
            }
        }
        let text = saved(|a, s| a.set_c32(s, 0, Some(bad.clone())));
        prop_assert!(read(&text).is_none(), "{:?}", bad);
    }
}

proptest! {
    #![proptest_config(config(200))]

    /// A Seed XOR set of 2 to 4 parts reads back as the same set, with its
    /// parts' fingerprints in order.
    #[test]
    fn a_seed_xor_set_reads_back_as_the_set(
        parts in prop::collection::vec(any::<[u8; 4]>(), 2..=4),
    ) {
        let set = Xor { seed: 0, parts };
        let text = saved(|a, s| a.set_xor(s, 0, Some(set.clone())));
        let a = read(&text).expect("the plan reads");
        prop_assert_eq!(a.xor(0), Some(&set));
        prop_assert_eq!(a.xor(0).map(Xor::n), Some(set.n()));
    }

    /// A Seed XOR set of fewer than 2 or more than 4 parts is refused.
    #[test]
    fn a_seed_xor_set_of_too_few_or_too_many_parts_is_refused(
        parts in prop::collection::vec(any::<[u8; 4]>(), 0..=12)
            .prop_filter("outside 2 to 4", |p| !(2..=4).contains(&p.len())),
    ) {
        let set = Xor { seed: 0, parts };
        let text = saved(|a, s| a.set_xor(s, 0, Some(set.clone())));
        prop_assert!(read(&text).is_none(), "{} parts", set.n());
    }
}
