//! What a person making or keeping codex32 shares would notice, against
//! BIP 93's published vectors (`tools/vectors/bip93/vectors.json`):
//! the characters typed one at a time make the published strings; a key
//! loaded from a master seed writes its shares again and they recover it;
//! a key of words does too, at any threshold and size.

use faraday_bip::codex32::{Codex32, Length, Scheme};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_keys::{Keys, Paper, Reveal, WorkSet, codex32_seed, identity};
use serde_json::Value;

const BIP93: &str = include_str!("../../../tools/vectors/bip93/vectors.json");
const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(count)
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Fresh bytes enough for any set: nine of a 64-byte seed.
const R: usize = 8 * 103;

/// The random shares are the fresh bytes as they were given, a symbol a
/// byte (its low five bits), the first share first: nothing stretches
/// or hashes them.
#[test]
fn a_set_s_random_shares_are_the_fresh_bytes() {
    let c = compute();
    let seed = unhex("ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100");
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(&c, &seed, "").unwrap();
    let random: Vec<u8> = (0..R).map(|i| (i * 7 + 3) as u8).collect();
    let need = 2 * Codex32::parse(
        keys.codex32_shares(0, 0, 1, &[], Reveal::PaperForm)
            .unwrap()[0]
            .as_str(),
    )
    .unwrap()
    .payload()
    .len();
    let shares = keys
        .codex32_shares(0, 3, 5, &random, Reveal::PaperForm)
        .unwrap();
    let mut drawn = Vec::new();
    for s in &shares[..2] {
        drawn.extend_from_slice(Codex32::parse(s.as_str()).unwrap().payload());
    }
    let want: Vec<u8> = random[..need].iter().map(|b| b & 31).collect();
    assert_eq!(drawn, want);
    // One byte fewer than the shares need is refused.
    assert!(matches!(
        keys.codex32_shares(0, 3, 5, &random[..need - 1], Reveal::PaperForm),
        Err(Paper::Refused(_))
    ));
}

fn sym(c: u8) -> u8 {
    CHARSET.iter().position(|&x| x == c).unwrap() as u8
}

/// A published set (vector 2 or 3) in index order, lowercase.
fn published(vector: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(BIP93).unwrap();
    let case = v["sets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["vector"] == vector)
        .unwrap();
    let mut all: Vec<String> = case["shares"]
        .as_array()
        .unwrap()
        .iter()
        .chain(case["derived"].as_object().unwrap().values())
        .map(|s| s.as_str().unwrap().to_lowercase())
        .collect();
    all.sort_by_key(|s| {
        b"acdefghjklmnpqrtuvwxyz023456789"
            .iter()
            .position(|&x| x == s.as_bytes()[8])
    });
    all
}

fn seed_of(vector: &str) -> Vec<u8> {
    let v: Value = serde_json::from_str(BIP93).unwrap();
    let hex = v["sets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["vector"] == vector)
        .unwrap()["seed"]
        .as_str()
        .unwrap()
        .to_string();
    unhex(&hex)
}

/// A work set for `vector`'s scheme, its initial shares typed in.
fn typed(c: &Compute, vector: &str) -> (WorkSet, Vec<String>) {
    let all = published(vector);
    let k = all[0].as_bytes()[3] - b'0';
    let id: [u8; 4] = all[0].as_bytes()[4..8].try_into().unwrap();
    let scheme = Scheme::new(k, all.len() as u8, id, Length::Chars48).unwrap();
    let mut work = WorkSet::new(c, scheme).unwrap();
    for (share, s) in all[..usize::from(k)].iter().enumerate() {
        for &ch in &s.as_bytes()[9..9 + 26] {
            work.push(c, share, sym(ch)).unwrap();
        }
    }
    (work, all)
}

#[test]
fn characters_typed_one_at_a_time_make_the_published_strings() {
    let c = compute();
    for vector in ["2", "3"] {
        let (work, all) = typed(&c, vector);
        assert!(work.is_complete(), "vector {vector}");
        let made = work.strings(Reveal::PaperForm).expect("the strings");
        let made: Vec<&str> = made.iter().map(|s| s.as_str()).collect();
        assert_eq!(made, all.iter().map(String::as_str).collect::<Vec<_>>());
    }
}

#[test]
fn the_set_is_not_made_before_every_character_is_in() {
    let c = compute();
    let (mut work, all) = typed(&c, "3");
    assert!(work.strings(Reveal::PaperForm).is_some());
    work.pop(1);
    assert!(!work.is_complete());
    assert!(work.strings(Reveal::PaperForm).is_none());
    assert_eq!(work.entered(1), 25);
    // Typing the character again makes the same set.
    work.push(&c, 1, sym(all[1].as_bytes()[9 + 25])).unwrap();
    let made = work.strings(Reveal::PaperForm).unwrap();
    assert_eq!(made[4].as_str(), all[4]);
}

#[test]
fn a_share_takes_no_character_past_its_last() {
    let c = compute();
    let (mut work, _) = typed(&c, "2");
    assert!(work.push(&c, 0, 0).is_err());
    assert!(work.push(&c, 7, 0).is_err());
}

#[test]
fn each_worksheet_ends_on_the_constant_once_its_share_is_complete() {
    let c = compute();
    let (mut work, _) = typed(&c, "3");
    for share in 0..3 {
        let sheet = work.worksheet(share, Reveal::PaperForm).expect("a sheet");
        assert!(sheet.holds(), "share {share}");
    }
    work.pop(2);
    assert!(!work.worksheet(2, Reveal::PaperForm).unwrap().holds());
}

#[test]
fn only_a_paper_form_lends_the_work_sets_rows_and_strings() {
    let c = compute();
    let (work, _) = typed(&c, "2");
    for why in [Reveal::Words, Reveal::SeedQr, Reveal::CopyByHand] {
        assert!(work.worksheet(0, why).is_none());
        assert!(work.strings(why).is_none());
        assert!(work.translation(&c, b'd', why).is_none());
    }
}

#[test]
fn the_derived_share_is_made_by_its_translation_worksheet() {
    let c = compute();
    let (work, all) = typed(&c, "2");
    let sheet = work
        .translation(&c, b'd', Reveal::PaperForm)
        .expect("a worksheet");
    assert_eq!(
        sheet.result().unwrap().encode().as_str(),
        all[2],
        "the derived share D of vector 2"
    );
}

#[test]
fn the_live_translation_worksheets_fill_to_the_published_strings() {
    let c = compute();
    for vector in ["2", "3"] {
        let all = published(vector);
        let k = usize::from(all[0].as_bytes()[3] - b'0');
        let id: [u8; 4] = all[0].as_bytes()[4..8].try_into().unwrap();
        let scheme = Scheme::new(k as u8, all.len() as u8, id, Length::Chars48).unwrap();
        let mut work = WorkSet::new(&c, scheme).unwrap();
        let targets = work.targets();
        assert_eq!(
            targets.len(),
            all.len() - k + 1,
            "every derived share and s"
        );
        assert_eq!(*targets.last().unwrap(), b's');
        for (share, s) in all[..k].iter().enumerate() {
            for (n, &ch) in s.as_bytes()[9..9 + 26].iter().enumerate() {
                work.push(&c, share, sym(ch)).unwrap();
                // Share A's translated rows fill while A is entered; no
                // sum cell fills before the last share's entry.
                let sheet = work.live_translation(0, Reveal::PaperForm).unwrap();
                assert!(sheet.translated_cell(share, 10 + n).is_some());
                assert_eq!(sheet.sum_cell(10 + n).is_some(), share == k - 1);
            }
        }
        for (n, &target) in targets.iter().enumerate() {
            let live = work.live_translation(n, Reveal::PaperForm).unwrap();
            let whole = work.translation(&c, target, Reveal::PaperForm).unwrap();
            for b in 4..=48 {
                assert_eq!(
                    live.sum_cell(b),
                    whole.sum_cell(b),
                    "vector {vector} {target} box {b}"
                );
            }
            let made = live.result().expect("a valid string").encode();
            if target != b's' {
                assert!(all.iter().any(|s| s == made.as_str()), "vector {vector}");
            }
        }
        // A character taken back empties its column in every sheet.
        work.pop(k - 1);
        let live = work.live_translation(0, Reveal::PaperForm).unwrap();
        assert!(live.sum_cell(35).is_none());
        assert!(live.sum_cell(36).is_none());
        assert!(live.sum_cell(34).is_some());
    }
}

#[test]
fn the_finished_set_loads_as_the_key_it_carries_with_its_seed_kept() {
    let c = compute();
    let (work, all) = typed(&c, "3");
    assert_eq!(all.len(), 5);
    let mut keys = Keys::on(Network::Mainnet);
    let before = work.fingerprint(&c).expect("a fingerprint");
    let fp = keys.add_codex32(&c, &work, "made by hand").unwrap();
    assert_eq!(fp, before);
    assert_eq!(
        fp,
        identity::fingerprint_of_seed(&c, &seed_of("3")).unwrap(),
        "vector 3's key"
    );
    assert!(keys.holds_seed_bytes(0));
}

#[test]
fn an_unfinished_set_loads_nothing() {
    let c = compute();
    let (mut work, _) = typed(&c, "2");
    work.pop(0);
    let mut keys = Keys::on(Network::Mainnet);
    assert!(keys.add_codex32(&c, &work, "x").is_err());
    assert!(keys.is_empty());
}

fn recovers(strings: &[&str]) -> Vec<u8> {
    let parts: Vec<Codex32> = strings.iter().map(|s| Codex32::parse(s).unwrap()).collect();
    codex32_seed(&parts).unwrap().to_vec()
}

#[test]
fn a_key_loaded_from_a_seed_writes_its_shares_and_they_recover_it() {
    let c = compute();
    let seed = seed_of("3");
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(&c, &seed, "from a seed").unwrap();
    assert!(keys.info(0).is_some_and(|i| !i.has_words()));
    for (k, n) in [(2u8, 2u8), (2, 3), (3, 5), (5, 9), (9, 31)] {
        let shares = keys
            .codex32_shares(0, k, n, &[7; R], Reveal::PaperForm)
            .unwrap_or_else(|_| panic!("{k} of {n}"));
        assert_eq!(shares.len(), usize::from(n));
        let strings: Vec<&str> = shares.iter().map(|s| s.as_str()).collect();
        assert_eq!(recovers(&strings[..usize::from(k)]), seed, "{k} of {n}");
        assert_eq!(
            recovers(&strings[usize::from(n - k)..]),
            seed,
            "the last {k} of {n}"
        );
    }
}

#[test]
fn a_key_loaded_from_a_long_seed_writes_shares_at_its_length() {
    let c = compute();
    let seed = unhex("ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100");
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(&c, &seed, "").unwrap();
    let shares = keys
        .codex32_shares(0, 2, 3, &[1; R], Reveal::PaperForm)
        .unwrap();
    assert!(shares.iter().all(|s| s.len() == 74));
    let strings: Vec<&str> = shares.iter().map(|s| s.as_str()).collect();
    assert_eq!(recovers(&strings[1..]), seed);
}

#[test]
fn without_a_threshold_the_key_gives_its_secret_alone() {
    let c = compute();
    let v: Value = serde_json::from_str(BIP93).unwrap();
    let vector = v["secrets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["vector"] == "4")
        .unwrap();
    let seed = unhex(vector["seed"].as_str().unwrap());
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(&c, &seed, "").unwrap();
    let one = keys
        .codex32_shares(0, 0, 1, &[0; R], Reveal::PaperForm)
        .unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(recovers(&[one[0].as_str()]), seed);
    assert!(matches!(
        keys.codex32_shares(0, 0, 2, &[0; R], Reveal::PaperForm),
        Err(Paper::Refused(_))
    ));
}

#[test]
fn a_key_of_words_is_never_split_into_codex32_strings() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words(&c, ABANDON, "abandon", None).unwrap();
    assert!(!keys.holds_seed_bytes(0));
    for (k, n) in [(0u8, 1u8), (2, 3), (4, 7)] {
        assert_eq!(
            keys.codex32_shares(0, k, n, &[9; R], Reveal::PaperForm)
                .err(),
            Some(Paper::Words),
            "{k} of {n}"
        );
    }
}

#[test]
fn shares_are_refused_for_a_split_that_bip93_does_not_allow() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(&c, &seed_of("3"), "").unwrap();
    for (k, n) in [(1u8, 3u8), (10, 12), (3, 2), (2, 32)] {
        assert!(
            matches!(
                keys.codex32_shares(0, k, n, &[3; R], Reveal::PaperForm),
                Err(Paper::Refused(_))
            ),
            "{k} of {n}"
        );
    }
    assert!(matches!(
        keys.codex32_shares(5, 2, 3, &[3; R], Reveal::PaperForm),
        Err(Paper::NoWords)
    ));
}

static CODEX32_WORK: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Counts the codex32 work the token is asked to note.
fn count(w: faraday_crypto::work::Work) {
    if w == faraday_crypto::work::Work::Codex32 {
        CODEX32_WORK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[test]
fn a_characters_worksheet_column_is_counted_as_codex32_work() {
    let c = compute();
    let (mut work, all) = typed(&c, "2");
    work.pop(0);
    let before = CODEX32_WORK.load(std::sync::atomic::Ordering::Relaxed);
    work.push(&c, 0, sym(all[0].as_bytes()[9 + 25])).unwrap();
    let after = CODEX32_WORK.load(std::sync::atomic::Ordering::Relaxed);
    assert!(
        after > before,
        "the last character's columns, the checksum and the set"
    );
}

/// A key with a codex32 set (`docs/CODEX32.md` §9.4 to §9.7), against
/// BIP 93's vector 2 (`tools/vectors/bip93/vectors.json`): the key Make
/// codex32 shares loads knows its set's plain facts; its vault record
/// keeps them and no string, and they come back when it loads; strings
/// typed back are told to rebuild the key or not; a lost share is made
/// again exactly; a new set of the same seed rebuilds the same key.
mod set {
    use std::collections::BTreeSet;

    use faraday_bip::codex32::{Codex32, Length, Scheme};
    use faraday_bip::keys::Network;
    use faraday_crypto::work::Compute;
    use faraday_keys::vault::{self, OpenVaults, RecordView};
    use faraday_keys::{Keys, Reveal, WorkSet, identity};
    use faraday_vault::codex32::{State, set_of};
    use faraday_vault::records::{field, kind};
    use faraday_vault::{Contents, Cost, Record};

    /// Vector 2: shares A and C, D derived, its secret, and its seed.
    const A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
    const C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
    const D: &str = "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg";
    const S: &str = "ms12names6xqguzttxkeqnjsjzv4jv3nz5k3kwgsphuh6evw";
    const SEED: &str = "d1808e096b35b209ca12132b264662a5";
    /// Vector 3's first share: another set, another seed.
    const OTHER: &str = "ms13casha320zyxwvutsrqpnmlkjhgfedca2a8d0zehn8a0t";

    use super::{R, compute};

    const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

    fn parse(s: &str) -> Codex32 {
        Codex32::parse(s).expect("a vector string")
    }

    /// Vector 2's set made as a person makes it: A and C typed in, a
    /// character at a time, n = 3; loaded as the key.
    fn made_here(c: &Compute) -> Keys {
        let scheme = Scheme::new(2, 3, *b"name", Length::Chars48).unwrap();
        let mut work = WorkSet::new(c, scheme).unwrap();
        for (share, s) in [A, C].iter().enumerate() {
            for ch in &s.as_bytes()[9..9 + 26] {
                let v = CHARSET.iter().position(|x| x == ch).unwrap() as u8;
                work.push(c, share, v).unwrap();
            }
        }
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_codex32(c, &work, "Shares").expect("the key");
        keys
    }

    #[test]
    fn a_key_made_from_codex32_shares_knows_its_set() {
        let c = compute();
        let keys = made_here(&c);
        let set = keys[0].codex32.clone().expect("the set");
        assert_eq!(set.threshold, 2);
        assert_eq!(&set.identifier, b"name");
        assert_eq!(set.length, 48);
        assert_eq!(set.indices(), b"acd");
        assert!(set.strings.iter().all(|&(_, s)| s == State::Made));
        let seed: Vec<u8> = (0..16)
            .map(|i| u8::from_str_radix(&SEED[2 * i..2 * i + 2], 16).unwrap())
            .collect();
        assert_eq!(
            keys[0].fingerprint,
            identity::fingerprint_of_seed(&c, &seed).unwrap()
        );
        assert_eq!(keys[0].fingerprint.0, [0xfa, 0xb6, 0x86, 0x8a]);
    }

    #[test]
    fn its_vault_record_keeps_the_seed_and_the_sets_facts_and_they_load_back() {
        let c = compute();
        let keys = made_here(&c);
        let record = keys
            .key_record(
                &c,
                0,
                None,
                false,
                Reveal::VaultEntry(faraday_keys::Part::Words),
            )
            .expect("a seed key has a record");
        let cost = Cost {
            memory_kib: faraday_vault::CREATE_MIN_MEMORY_KIB,
            passes: faraday_vault::CREATE_MIN_PASSES,
            lanes: 1,
        };
        let file = vault::create(
            &c,
            faraday_vault::SLOT_SIZES[0],
            cost,
            &["pass"],
            &[Contents::default()],
            &[7u8; 32],
        )
        .unwrap();
        let mut store = OpenVaults::default();
        let (id, _) = store.unlock(&c, file, "pass").expect("opens");
        let at = store.insert(id, record).unwrap();
        let r: RecordView<'_> = store.records(id).get(at).unwrap();
        // The set is public, and holds no string.
        let facts = set_of(r.public(field::KEY_CODEX32).expect("public")).unwrap();
        assert_eq!(facts, keys[0].codex32.clone().unwrap());
        assert!(r.public(field::KEY).is_none(), "the seed is secret");
        let mut again = Keys::on(Network::Mainnet);
        let loaded = store.load_from_vault(&c, &mut again, id, &BTreeSet::from([at]));
        assert_eq!(loaded, vec![keys[0].fingerprint]);
        assert_eq!(again[0].codex32, keys[0].codex32);
        assert!(again.holds_seed_bytes(0));
    }

    #[test]
    fn a_codex32_share_records_string_is_secret() {
        assert!(vault::is_secret(kind::CODEX32_SHARE, field::CODEX32_STRING));
        assert!(!vault::is_secret(kind::CODEX32_SHARE, field::CODEX32_KEY));
        assert!(!vault::is_secret(kind::KEY, field::KEY_CODEX32));
        let r = Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, C.as_bytes())
            .with(field::CODEX32_KEY, &[0xfa, 0xb6, 0x86, 0x8a]);
        let view = RecordView::of(&r);
        assert!(view.public(field::CODEX32_STRING).is_none());
        assert_eq!(view.len(field::CODEX32_STRING), Some(48));
    }

    #[test]
    fn strings_typed_back_rebuild_their_key_and_no_other() {
        let c = compute();
        let keys = made_here(&c);
        assert_eq!(
            keys.codex32_rebuilds(&c, 0, &[parse(A), parse(C)]),
            Some(true)
        );
        assert_eq!(
            keys.codex32_rebuilds(&c, 0, &[parse(D), parse(A)]),
            Some(true)
        );
        assert_eq!(keys.codex32_rebuilds(&c, 0, &[parse(S)]), Some(true));
        // One share of two is not enough to say.
        assert_eq!(keys.codex32_rebuilds(&c, 0, &[parse(A)]), None);
        // Vector 3's strings make another seed.
        let other = [
            parse(OTHER),
            parse("ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr"),
            parse("ms13cashd0wsedstcdcts64cd7wvy4m90lm28w4ffupqs7rm"),
        ];
        assert_eq!(keys.codex32_rebuilds(&c, 0, &other), Some(false));
    }

    #[test]
    fn a_lost_share_is_made_again_exactly() {
        let c = compute();
        let keys = made_here(&c);
        let d = keys
            .codex32_derive(&c, 0, &[parse(A), parse(C)], b'd', Reveal::PaperForm)
            .expect("from two strings");
        assert_eq!(d.as_str(), D);
        // One string and the key's seed are not enough: the secret's padding
        // bits are not in the seed, so the share made would be of another set.
        assert!(
            keys.codex32_derive(&c, 0, &[parse(A)], b'd', Reveal::PaperForm)
                .is_err()
        );
        // The secret, typed, counts as one.
        let d = keys
            .codex32_derive(&c, 0, &[parse(S), parse(A)], b'd', Reveal::PaperForm)
            .expect("from the secret and one string");
        assert_eq!(d.as_str(), D);
        let c_again = keys
            .codex32_derive(&c, 0, &[parse(A), parse(D)], b'c', Reveal::PaperForm)
            .expect("C replaced");
        assert_eq!(c_again.as_str(), C);
        // Not a paper form, or strings of another key: nothing.
        assert!(
            keys.codex32_derive(&c, 0, &[parse(A), parse(C)], b'e', Reveal::Words)
                .is_err()
        );
        let other = [
            parse(OTHER),
            parse("ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr"),
            parse("ms13cashd0wsedstcdcts64cd7wvy4m90lm28w4ffupqs7rm"),
        ];
        assert!(
            keys.codex32_derive(&c, 0, &other, b'e', Reveal::PaperForm)
                .is_err()
        );
    }

    #[test]
    fn a_new_set_of_the_same_seed_rebuilds_the_same_key() {
        let c = compute();
        let keys = made_here(&c);
        let made = keys
            .codex32_new_set(&c, 0, (3, 5), *b"w0za", &[9; R], Reveal::PaperForm)
            .expect("a new set");
        assert_eq!(made.len(), 5);
        let strings: Vec<Codex32> = made.iter().map(|s| parse(s)).collect();
        assert!(
            strings
                .iter()
                .all(|s| &s.identifier() == b"w0za" && s.threshold() == 3)
        );
        for pick in [[0, 1, 2], [2, 3, 4], [0, 2, 4]] {
            let three: Vec<Codex32> = pick.iter().map(|&i| parse(&made[i])).collect();
            assert_eq!(keys.codex32_rebuilds(&c, 0, &three), Some(true), "{pick:?}");
        }
        // Fresh randomness makes another set.
        let again = keys
            .codex32_new_set(&c, 0, (3, 5), *b"w0za", &[10; R], Reveal::PaperForm)
            .unwrap();
        assert_ne!(again[1].as_str(), made[1].as_str());
    }
}

/// Make the rest, Add a share and Check a string (`docs/CODEX32.md`
/// §8.1 to §8.3): sets made of `k` strings typed in, against BIP 93's
/// vectors 2 (2 of 3) and 3 (3 of 5, the secret one of its three).
mod given {
    use faraday_bip::codex32::Codex32;
    use faraday_bip::keys::Network;
    use faraday_keys::{Keys, Making, Reveal, StringCheck, WorkSet, identity};

    use super::{compute, published, seed_of};

    /// Vector 3's secret.
    const CASH_S: &str = "ms13cashsllhdmn9m42vcsamx24zrxgs3qqjzqud4m0d6nln";
    /// Vector 2's secret.
    const NAME_S: &str = "ms12names6xqguzttxkeqnjsjzv4jv3nz5k3kwgsphuh6evw";

    fn parse(s: &str) -> Codex32 {
        Codex32::parse(s).expect("a vector string")
    }

    fn strings(work: &WorkSet) -> Vec<String> {
        work.strings(Reveal::PaperForm)
            .expect("the strings")
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn make_the_rest_from_k_strings_writes_the_published_ones() {
        let c = compute();
        for (vector, secret) in [("2", NAME_S), ("3", CASH_S)] {
            let all = published(vector);
            let k = usize::from(all[0].as_bytes()[3] - b'0');
            let given: Vec<Codex32> = all[..k].iter().map(|s| parse(s)).collect();
            let work = WorkSet::given(&c, &given, Making::Rest(all.len() as u8)).unwrap();
            assert_eq!(strings(&work), all[k..], "vector {vector}");
            // Each string made is its translation worksheet's sum, and the
            // last sheet makes the secret.
            let targets = work.targets();
            assert_eq!(*targets.last().unwrap(), b's');
            for (n, &t) in targets.iter().enumerate() {
                let sheet = work.live_translation(n, Reveal::PaperForm).unwrap();
                let made = sheet.result().expect("a valid string").encode();
                let want = if t == b's' {
                    secret.to_string()
                } else {
                    all[k + n].clone()
                };
                assert_eq!(made.as_str(), want, "vector {vector} {}", char::from(t));
            }
            // Every input's worksheet ends on the constant.
            for s in 0..k {
                assert!(work.worksheet(s, Reveal::PaperForm).unwrap().holds());
            }
            assert_eq!(work.inputs(), &b"acd"[..k]);
            assert_eq!(
                work.fingerprint(&c).unwrap(),
                identity::fingerprint_of_seed(&c, &seed_of(vector)).unwrap()
            );
        }
    }

    #[test]
    fn the_whole_set_of_two_initial_shares_is_the_published_set_in_index_order() {
        let c = compute();
        let all = published("2");
        let given = [parse(&all[1]), parse(&all[0])];
        let work = WorkSet::given(&c, &given, Making::All(3)).unwrap();
        assert_eq!(strings(&work), all);
        assert_eq!(work.targets(), b"ds");
    }

    #[test]
    fn add_a_share_from_any_k_strings_of_the_set_is_the_published_one() {
        let c = compute();
        let all = published("3");
        // C, E and the secret make F; the secret, D and F make A.
        let three = [parse(&all[1]), parse(&all[3]), parse(CASH_S)];
        let work = WorkSet::given(&c, &three, Making::Add(b'f')).unwrap();
        assert_eq!(strings(&work), [all[4].clone()]);
        assert_eq!(work.targets(), b"f");
        let three = [parse(CASH_S), parse(&all[2]), parse(&all[4])];
        let work = WorkSet::given(&c, &three, Making::Add(b'a')).unwrap();
        assert_eq!(strings(&work), [all[0].clone()]);
        // Vector 2: A and C make D.
        let two = published("2");
        let work = WorkSet::given(&c, &[parse(&two[0]), parse(&two[1])], Making::Add(b'd'));
        assert_eq!(strings(&work.unwrap()), [two[2].clone()]);
    }

    #[test]
    fn an_index_given_the_secrets_or_too_few_strings_are_refused() {
        let c = compute();
        let two = published("2");
        let pair = [parse(&two[0]), parse(&two[1])];
        for at in *b"acsb" {
            assert!(WorkSet::given(&c, &pair, Making::Add(at)).is_err(), "{at}");
        }
        assert!(WorkSet::given(&c, &pair[..1], Making::Add(b'd')).is_err());
        // Two sets mixed make nothing.
        let three = published("3");
        let mixed = [parse(&two[0]), parse(&three[1])];
        assert!(WorkSet::given(&c, &mixed, Making::Add(b'd')).is_err());
        // Make the rest with nothing left to make.
        assert!(WorkSet::given(&c, &pair, Making::Rest(2)).is_err());
    }

    #[test]
    fn a_set_of_strings_typed_in_takes_no_character_and_loads_its_key_and_set() {
        let c = compute();
        let all = published("3");
        let given = [parse(&all[2]), parse(&all[4]), parse(CASH_S)];
        let mut work = WorkSet::given(&c, &given, Making::Rest(4)).unwrap();
        assert!(work.push(&c, 0, 0).is_err());
        work.pop(0);
        // The secret is among them: no sheet towards it, A is made.
        assert_eq!(work.targets(), b"a");
        assert_eq!(strings(&work), [all[0].clone()]);
        let mut keys = Keys::on(Network::Mainnet);
        let fp = keys.add_codex32(&c, &work, "typed").unwrap();
        assert_eq!(
            fp,
            identity::fingerprint_of_seed(&c, &seed_of("3")).unwrap()
        );
        let set = keys[0].codex32.clone().expect("the set");
        assert_eq!(set.threshold, 3);
        assert_eq!(set.indices(), b"dfsa");
    }

    #[test]
    fn check_a_string_reads_its_final_row() {
        let c = compute();
        let all = published("2");
        let check = StringCheck::of(&c, &all[0]).unwrap();
        let sheet = check.worksheet(Reveal::PaperForm).unwrap();
        assert!(sheet.holds());
        assert_eq!(sheet.first_difference(), None);
        // The same string upper case is the same sheet.
        let upper = StringCheck::of(&c, &all[0].to_uppercase()).unwrap();
        assert!(upper.worksheet(Reveal::PaperForm).unwrap().holds());
        // One box changed: the checksum fails, the final row says where.
        let mut bad = all[0].clone().into_bytes();
        bad[16] = if bad[16] == b'q' { b'p' } else { b'q' };
        let bad = String::from_utf8(bad).unwrap();
        let check = StringCheck::of(&c, &bad).unwrap();
        let sheet = check.worksheet(Reveal::PaperForm).unwrap();
        assert!(!sheet.holds());
        assert!(sheet.first_difference().is_some());
        assert!(check.worksheet(Reveal::Words).is_none());
        // Mixed case, a character outside bech32, a length no string has.
        let mixed = "ms12NAMEa320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
        assert!(StringCheck::of(&c, mixed).is_none());
        let outside = "ms12nameb320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
        assert!(StringCheck::of(&c, outside).is_none());
        assert!(StringCheck::of(&c, "ms12name").is_none());
    }
}
