//! What a person making or keeping codex32 shares would notice, against
//! BIP 93's published vectors (`tools/vectors/bip93/vectors.json`):
//! the characters typed one at a time make the published strings; a key
//! loaded from a master seed writes its shares again and they recover it;
//! a key of words does too, at any threshold and size.

use faraday_bip::bip39::{Language, Mnemonic};
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
            .codex32_shares(&c, 0, k, n, &[7; 32], Reveal::PaperForm)
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
        .codex32_shares(&c, 0, 2, 3, &[1; 32], Reveal::PaperForm)
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
        .codex32_shares(&c, 0, 0, 1, &[0; 32], Reveal::PaperForm)
        .unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!(recovers(&[one[0].as_str()]), seed);
    assert!(matches!(
        keys.codex32_shares(&c, 0, 0, 2, &[0; 32], Reveal::PaperForm),
        Err(Paper::Refused(_))
    ));
}

#[test]
fn a_key_of_words_writes_its_bip39_seed_as_shares_of_any_size() {
    let c = compute();
    let m = Mnemonic::parse(&c, Language::English, ABANDON).unwrap();
    let want = m.to_seed(&c, b"").unwrap().expose().to_vec();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words(&c, ABANDON, "abandon", None).unwrap();
    assert!(!keys.holds_seed_bytes(0));
    let shares = keys
        .codex32_shares(&c, 0, 4, 7, &[9; 32], Reveal::PaperForm)
        .unwrap();
    assert_eq!(shares.len(), 7);
    assert!(shares.iter().all(|s| s.len() == 127));
    let strings: Vec<&str> = shares.iter().map(|s| s.as_str()).collect();
    assert_eq!(recovers(&strings[3..]), want);
}

#[test]
fn shares_are_refused_for_a_split_that_bip93_does_not_allow() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(&c, &seed_of("3"), "").unwrap();
    for (k, n) in [(1u8, 3u8), (10, 12), (3, 2), (2, 32)] {
        assert!(
            matches!(
                keys.codex32_shares(&c, 0, k, n, &[3; 32], Reveal::PaperForm),
                Err(Paper::Refused(_))
            ),
            "{k} of {n}"
        );
    }
    assert!(matches!(
        keys.codex32_shares(&c, 5, 2, 3, &[3; 32], Reveal::PaperForm),
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
