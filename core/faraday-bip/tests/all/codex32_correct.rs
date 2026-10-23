//! Codex32 error correction (`docs/CODEX32.md` §8.4, §13.5, F94): what
//! the decoder proposes for a damaged string, and where it refuses.
//!
//! - python-codex32's frozen correction corpus
//!   (`tools/vectors/codex32-correction/`), drawn from the Haskell
//!   decoder of BlockstreamResearch/codex32 pull request 70: every case's
//!   expected string, or its refusal;
//! - every mix of substitutions and unreadable boxes within BIP 93's
//!   bound, at every length BIP 93 publishes a string of, and every run
//!   of unreadable boxes as long as the checksum;
//! - past the bound, nothing the bound does not justify is proposed;
//! - `ms1`, and the header boxes the strings already in fix, are never
//!   changed.
//!
//! python-codex32's decoder recomputes the rest over random damage, in
//! the crypto cross-check (`tools/crosscheck/area_codex32_correct.py`).

use crate::common::XorShift;
use faraday_bip::codex32::{Codex32, Error, Proposal, Refusal, correct};
use faraday_crypto::work::Compute;
use serde_json::Value;

const CORPUS: &str =
    include_str!("../../../../tools/vectors/codex32-correction/p70_correction_vectors.json");

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// BIP 93's strings, one of every length and both kinds: vector 1 (48),
/// a share of vector 3 (48), vectors 6, 7, 8 (54, 61, 67), 4 (74) and
/// 5 (127, the long checksum).
const STRINGS: [&str; 7] = [
    "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw",
    "ms13casha320zyxwvutsrqpnmlkjhgfedca2a8d0zehn8a0t",
    "ms10seedsqqqsyqcyq5rqwzqfpg9scrgwpugpzysn9vaqzzvs20xnl",
    "ms10seedsyqsjygeyy5nzw2pf9g4jctfw9ucrzv3nxs6nvdau84gz0632s0xs",
    "ms10seedsgpq5ys6yg4rywjzfff95cn2wfag9z5jn2324v46ct9d9hrcduqw8c3lccl",
    "ms10leetsllhdmn9m42vcsamx24zrxgs3qrl7ahwvhw4fnzrhve25gvezzyqqtum9pgv99ycma",
    "ms100c8vsm32zxfguhpchtlupzry9x8gf2tvdw0s3jn54khce6mua7lqpzygsfjd6an074rxvcemlh8wu3tk925acdefghjklmnpqrstuvwxy06fhpv80undvarhrak",
];

fn checksum_len(s: &str) -> usize {
    if s.len() == 127 { 15 } else { 13 }
}

fn fix(text: &str, held: Option<&Codex32>) -> Result<Proposal, Refusal> {
    correct(&Compute::unchecked(), text, held)
}

fn proposed(text: &str) -> String {
    match fix(text, None) {
        Ok(p) => p.string().encode().as_str().to_string(),
        Err(r) => panic!("{text}: refused, {r:?}"),
    }
}

fn refusal(text: &str, held: Option<&Codex32>) -> Refusal {
    match fix(text, held) {
        Ok(p) => panic!("{text}: proposed {}", p.string().encode().as_str()),
        Err(r) => r,
    }
}

/// `s` with the boxes at `subs` (0-based places in the string) changed
/// to another character and those at `unread` marked `?`.
fn damage(s: &str, subs: &[usize], unread: &[usize], rng: &mut XorShift) -> String {
    let mut b = s.as_bytes().to_vec();
    for &i in subs {
        let v = CHARSET.iter().position(|&c| c == b[i]).unwrap();
        let add = 1 + (rng.next_u64() % 31) as usize;
        b[i] = CHARSET[v ^ add];
    }
    for &i in unread {
        b[i] = b'?';
    }
    String::from_utf8(b).unwrap()
}

/// `count` distinct places from `from` to the end of `s`.
fn places(rng: &mut XorShift, s: &str, from: usize, count: usize) -> Vec<usize> {
    let mut out = Vec::new();
    while out.len() < count {
        let i = from + (rng.next_u64() as usize) % (s.len() - from);
        if !out.contains(&i) {
            out.push(i);
        }
    }
    out
}

#[test]
fn the_frozen_corpus_is_corrected_or_refused_as_published() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 57);
    for case in cases {
        let damaged = case["damaged"].as_str().unwrap();
        match case["expected"].as_str() {
            Some(want) => assert_eq!(proposed(damaged), want.to_ascii_lowercase(), "{damaged}"),
            None => {
                refusal(damaged, None);
            }
        }
    }
}

#[test]
fn every_mix_within_the_bound_is_corrected_at_every_length() {
    let mut rng = XorShift::new(93);
    for s in STRINGS {
        for subs in 0..=4usize {
            for unread in 0..=8 - 2 * subs {
                if subs + unread == 0 {
                    continue;
                }
                for _ in 0..12 {
                    let at = places(&mut rng, s, 3, subs + unread);
                    let text = damage(s, &at[..subs], &at[subs..], &mut rng);
                    let p = fix(&text, None).unwrap_or_else(|r| panic!("{text}: refused, {r:?}"));
                    assert_eq!(p.string().encode().as_str(), s, "{text}");
                    assert_eq!((p.substitutions(), p.erasures()), (subs, unread));
                    assert!(!p.assumes_the_rest());
                    // The edits name exactly the boxes damaged, in order,
                    // with what was there and what goes back.
                    let mut boxes = at.clone();
                    boxes.sort_unstable();
                    let edits = p.edits();
                    assert_eq!(edits.len(), boxes.len());
                    for (e, &i) in edits.iter().zip(&boxes) {
                        assert_eq!(e.position(), i + 1);
                        assert_eq!(e.was(), text.as_bytes()[i]);
                        assert_eq!(e.now(), s.as_bytes()[i]);
                        assert_eq!(e.was_unreadable(), at[subs..].contains(&i));
                    }
                }
            }
        }
    }
}

#[test]
fn a_box_is_named_by_its_number_from_m() {
    // Box 17 of vector 1 is an `x`; typed as `q`, the proposal is
    // "Box 17 · q → x".
    let s = STRINGS[0];
    let text = format!("{}q{}", &s[..16], &s[17..]);
    let p = fix(&text, None).unwrap_or_else(|r| panic!("{r:?}"));
    let e = &p.edits()[0];
    assert_eq!((e.position(), e.was(), e.now()), (17, b'q', b'x'));
}

#[test]
fn a_run_of_unreadable_boxes_as_long_as_the_checksum_is_filled() {
    for s in STRINGS {
        for run in 9..=checksum_len(s) {
            for start in 3..=s.len() - run {
                let at: Vec<usize> = (start..start + run).collect();
                let text = damage(s, &[], &at, &mut XorShift::new(1));
                let p = fix(&text, None).unwrap_or_else(|r| panic!("{text}: refused, {r:?}"));
                assert_eq!(p.string().encode().as_str(), s, "{text}");
                assert!(p.assumes_the_rest());
                assert_eq!((p.erasures(), p.substitutions()), (run, 0));
            }
        }
    }
}

#[test]
fn past_the_bound_nothing_is_proposed_that_the_bound_does_not_justify() {
    let mut rng = XorShift::new(127);
    for s in STRINGS {
        // Too many substitutions, or a mix past 2s + e = 8: the string
        // the damage came from is never proposed. Whatever is proposed
        // is another string the damage lies within the bound of.
        for (subs, unread) in [(5, 0), (6, 0), (4, 1), (3, 3), (1, 7), (2, 5)] {
            for _ in 0..40 {
                let at = places(&mut rng, s, 3, subs + unread);
                let text = damage(s, &at[..subs], &at[subs..], &mut rng);
                if let Ok(p) = fix(&text, None) {
                    assert_ne!(p.string().encode().as_str(), s, "{text}");
                    assert!(2 * p.substitutions() + p.erasures() <= 8);
                }
            }
        }
        // 9 unreadable boxes not in one run, and a run one longer than
        // the checksum, which 32 strings would fill: refused.
        for _ in 0..20 {
            let mut at = places(&mut rng, s, 3, 9);
            at.sort_unstable();
            if at[8] - at[0] == 8 {
                continue;
            }
            let text = damage(s, &[], &at, &mut rng);
            assert_eq!(refusal(&text, None), Refusal::TooManyUnreadable, "{text}");
        }
        let run = checksum_len(s) + 1;
        let at: Vec<usize> = (10..10 + run).collect();
        let text = damage(s, &[], &at, &mut rng);
        assert_eq!(refusal(&text, None), Refusal::TooManyUnreadable);
        // A run one box shorter than the checksum leaves one equation
        // over: here it catches a wrong box outside the run.
        let short = checksum_len(s) - 1;
        let at: Vec<usize> = (10..10 + short).collect();
        let text = damage(s, &[40], &at, &mut rng);
        assert_eq!(refusal(&text, None), Refusal::NotFound, "{text}");
    }
}

#[test]
fn ms1_and_the_header_the_set_fixes_are_never_changed() {
    let mut rng = XorShift::new(48);
    let share_a = STRINGS[1];
    let share_c = "ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr";
    let held = Codex32::parse(share_a).unwrap();

    // `ms1` unreadable or wrong: not a codex32 string at all.
    for i in 0..3 {
        let text = damage(share_c, &[], &[i], &mut rng);
        assert_eq!(refusal(&text, None), Refusal::Malformed(Error::BadPrefix));
    }

    // Unheld, the threshold and identifier boxes are corrected like any.
    for i in 3..8 {
        let text = damage(share_c, &[i], &[], &mut rng);
        assert_eq!(proposed(&text), share_c);
    }

    // Held, they must read as the strings already in: wrong or
    // unreadable there is refused, and damage elsewhere is corrected.
    for i in 3..8 {
        let wrong = damage(share_c, &[i], &[], &mut rng);
        assert_eq!(refusal(&wrong, Some(&held)), Refusal::HeaderFixed);
        let unread = damage(share_c, &[], &[i], &mut rng);
        assert_eq!(refusal(&unread, Some(&held)), Refusal::HeaderFixed);
    }
    for _ in 0..20 {
        let at = places(&mut rng, share_c, 8, 6);
        let text = damage(share_c, &at[..2], &at[2..], &mut rng);
        let p = fix(&text, Some(&held)).unwrap_or_else(|r| panic!("{text}: {r:?}"));
        assert_eq!(p.string().encode().as_str(), share_c);
    }
    // The share index is not fixed by the set: box 9 is corrected.
    let text = damage(share_c, &[8], &[], &mut rng);
    let p = fix(&text, Some(&held)).unwrap_or_else(|r| panic!("{r:?}"));
    assert_eq!(p.string().encode().as_str(), share_c);
}

#[test]
fn a_string_that_holds_needs_no_edits() {
    for s in STRINGS {
        let p = fix(s, None).unwrap_or_else(|r| panic!("{r:?}"));
        assert!(p.edits().is_empty());
        assert_eq!(p.string().encode().as_str(), s);
    }
    // Upper case, as a Codex32QR code reads, is corrected and proposed
    // in lower case.
    let upper = STRINGS[5].to_ascii_uppercase();
    let text = format!("{}Q{}", &upper[..20], &upper[21..]);
    assert_eq!(proposed(&text), STRINGS[5]);
}

#[test]
fn what_is_not_a_damaged_string_is_refused() {
    let s = STRINGS[0];
    let mixed = format!("M{}", &s[1..]);
    assert_eq!(refusal(&mixed, None), Refusal::Malformed(Error::MixedCase));
    // `b` is not bech32 and not `?`: refused, not read as unreadable.
    let b = format!("{}b{}", &s[..20], &s[21..]);
    assert_eq!(
        refusal(&b, None),
        Refusal::Malformed(Error::BadCharacter { position: 20 })
    );
    assert_eq!(
        refusal(&s[..47], None),
        Refusal::Malformed(Error::BadLength)
    );
    let wide = format!("{}é{}", &s[..20], &s[22..]);
    assert_eq!(
        refusal(&wide, None),
        Refusal::Malformed(Error::NotPrintable)
    );
}
