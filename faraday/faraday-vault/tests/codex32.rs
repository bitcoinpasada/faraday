//! A vault keeps a codex32 key's set as its plain facts and no string,
//! and one string of a set without its seed, as a codex32 share record
//! (`docs/VAULT.md` §7, `docs/CODEX32.md` §9.4, §9.5); the committed
//! vector holds both, and a slot holding neither reads as it always did.

#[path = "../examples/vectors.rs"]
mod vectors;

use faraday_vault::codex32::{SetFacts, State, set_of, set_payload};
use faraday_vault::records::{Fault, field, kind};
use faraday_vault::{Contents, Record};

fn committed(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/vault")
        .join(name);
    std::fs::read(path).expect("the committed vector")
}

fn two_of_three() -> SetFacts {
    SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 0,
        strings: vec![
            (b'a', State::Placed),
            (b'c', State::Placed),
            (b'd', State::Made),
        ],
    }
}

/// A slot of `records`, encoded and read back.
fn round_trip(records: Vec<Record>) -> Result<Contents, Fault> {
    let plain = Contents { records }
        .encode(
            65_536 - faraday_vault::COMMIT_LEN,
            faraday_vault::Seals::FIRST,
        )
        .map_err(|_| Fault::Truncated)?;
    Contents::decode(&plain).map(|(c, _)| c)
}

#[test]
fn the_codex32_vector_opens_with_its_key_set_and_share() {
    let c = vectors::compute();
    let bytes = committed(vectors::CODEX32.0);
    let (_, got) = faraday_vault::open(&c, &bytes, vectors::CODEX32.1[0]).expect("opens");
    let key = got.of(kind::KEY).next().expect("a key record").1;
    let set = set_of(key.field(field::KEY_CODEX32).expect("the set")).expect("reads");
    assert_eq!(set, two_of_three());
    assert_eq!(set.state(b'd'), Some(State::Made));
    let share = got
        .of(kind::CODEX32_SHARE)
        .next()
        .expect("a share record")
        .1;
    assert_eq!(share.text(field::CODEX32_STRING), Some(vectors::VECTOR_2.1));
    assert_eq!(
        share.field(field::CODEX32_KEY),
        Some(&vectors::VECTOR_2.2[..])
    );
}

#[test]
fn a_key_record_keeps_the_sets_facts_and_no_string() {
    let payload = set_payload(&two_of_three()).expect("sound");
    // k, the identifier, the length, the unknown count, and two bytes a
    // string: nothing a string could be read from.
    assert_eq!(payload, b"\x02name\x30\x00a\x01c\x01d\x00");
    let back = round_trip(vec![
        Record::new(kind::KEY)
            .with(
                field::KEY,
                &faraday_vault::layouts::seed_payload(&[7; 16]).unwrap(),
            )
            .with(field::KEY_CODEX32, &payload),
    ])
    .expect("reads");
    let key = &back.records[0];
    assert_eq!(
        set_of(key.field(field::KEY_CODEX32).unwrap()),
        Some(two_of_three())
    );
    assert!(
        !key.fields
            .iter()
            .any(|f| f.bytes.windows(3).any(|w| w == b"ms1"))
    );
}

#[test]
fn a_set_of_strings_whose_indices_are_not_all_known_reads_back() {
    let facts = SetFacts {
        threshold: 3,
        identifier: *b"w0za",
        length: 74,
        unknown: 2,
        strings: vec![
            (b'c', State::Made),
            (b'd', State::Lost),
            (b'e', State::Exposed),
        ],
    };
    assert_eq!(set_of(&set_payload(&facts).unwrap()), Some(facts));
    let secret = SetFacts {
        threshold: 0,
        identifier: *b"leet",
        length: 127,
        unknown: 0,
        strings: vec![(b's', State::Destroyed)],
    };
    assert_eq!(set_of(&set_payload(&secret).unwrap()), Some(secret));
}

#[test]
fn a_set_that_is_not_a_codex32_set_refuses_the_slot() {
    let bad = |f: &dyn Fn(&mut SetFacts)| {
        let mut s = two_of_three();
        f(&mut s);
        s
    };
    let refused = [
        bad(&|s| s.threshold = 1),
        bad(&|s| s.threshold = 10),
        bad(&|s| s.identifier = *b"nAme"),
        bad(&|s| s.identifier = *b"nbme"),
        bad(&|s| s.length = 49),
        bad(&|s| s.strings.push((b'a', State::Made))),
        bad(&|s| s.strings.push((b'B', State::Made))),
        bad(&|s| s.unknown = 29),
        bad(&|s| {
            s.threshold = 0;
            s.strings = vec![(b'a', State::Made)];
        }),
    ];
    for facts in refused {
        assert_eq!(set_payload(&facts), None, "{facts:?}");
    }
    let mut raw = set_payload(&two_of_three()).unwrap();
    raw[8] = 9;
    let r = Record::new(kind::KEY)
        .with(
            field::KEY,
            &faraday_vault::layouts::seed_payload(&[7; 16]).unwrap(),
        )
        .with(field::KEY_CODEX32, &raw);
    assert_eq!(r.check(), Err(Fault::Value(kind::KEY, field::KEY_CODEX32)));
    assert_eq!(
        Record::new(kind::KEY)
            .with(
                field::KEY,
                &faraday_vault::layouts::seed_payload(&[7; 16]).unwrap()
            )
            .with(field::KEY_CODEX32, &raw[..8])
            .check(),
        Err(Fault::Value(kind::KEY, field::KEY_CODEX32))
    );
}

#[test]
fn a_codex32_share_record_round_trips_and_holds_only_a_share() {
    let fp = vectors::VECTOR_2.2;
    let share = |s: &str| {
        Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, s.as_bytes())
            .with(field::CODEX32_KEY, &fp)
    };
    let back = round_trip(vec![share(vectors::VECTOR_2.1)]).expect("reads");
    assert_eq!(
        back.records[0].text(field::CODEX32_STRING),
        Some(vectors::VECTOR_2.1)
    );
    // The secret is the seed, kept as a key record; upper case, a broken
    // checksum and an unsplit secret are not a share.
    for s in [
        "ms12names6xqguzttxkeqnjsjzv4jv3nz5k3kwgsphuh6evw",
        "MS12NAMECACDEFGHJKLMNPQRSTUVWXYZ023FTR2GDZMPY6PN",
        "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pp",
        "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw",
    ] {
        assert_eq!(
            share(s).check(),
            Err(Fault::Value(kind::CODEX32_SHARE, field::CODEX32_STRING)),
            "{s}"
        );
    }
    let no_key = Record::new(kind::CODEX32_SHARE)
        .with(field::CODEX32_STRING, vectors::VECTOR_2.1.as_bytes());
    assert_eq!(
        no_key.check(),
        Err(Fault::Missing(kind::CODEX32_SHARE, field::CODEX32_KEY))
    );
}
