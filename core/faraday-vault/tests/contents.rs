//! The TOML inside a vault (`docs/VAULT.md` §7): the three TOML values a
//! file vault keeps in KDBX custom data (a backup
//! plan, a seed's codex32 facts and its BIP-85 facts) written, read back and refused as §7 says.
#![allow(clippy::disallowed_methods)]

use faraday_vault::Record;
use faraday_vault::child::ChildOf;
use faraday_vault::codex32::{SetFacts, State};
use faraday_vault::contents::{
    MAX_VALUE, Why, read_child_facts, read_codex32_facts, read_plan, write_child_facts,
    write_codex32_facts, write_plan,
};
use faraday_vault::records::{field, kind};

const XPUB: &str = "[73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V";

fn fields(r: &Record) -> Vec<(u8, Vec<u8>)> {
    r.fields
        .iter()
        .map(|f| (f.number, f.bytes.to_vec()))
        .collect()
}

fn plan() -> Record {
    Record::new(kind::PLAN)
        .with(
            field::PLAN_WALLET,
            format!("wpkh({XPUB}/<0;1>/*)").as_bytes(),
        )
        .with(field::PLAN_ANSWERS, b"seeds 1\nplaces 2")
        .with(field::PLAN_PLACE, b"Home")
        .with(field::PLAN_PLACE, b"")
        .with(field::PLAN_HOLDS, b"vault holds seed 73c5da0a")
        .with(field::PLAN_NOTE, b"lost \"place0\" w0\\n")
}

fn facts() -> SetFacts {
    SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 1,
        strings: vec![(b'a', State::Made), (b'c', State::Placed)],
    }
}

fn child() -> ChildOf {
    ChildOf {
        parent: [0x73, 0xc5, 0xda, 0x0a],
        language: 0,
        words: 12,
        index: 7,
    }
}

#[test]
fn a_plan_is_written_and_read_back_with_every_field() {
    let text = write_plan(&plan()).unwrap();
    let back = read_plan(&text).unwrap();
    assert_eq!(back.kind, kind::PLAN);
    assert_eq!(fields(&back), fields(&plan()));
    // A plan with only its wallet.
    let bare = Record::new(kind::PLAN).with(field::PLAN_WALLET, b"wpkh(x)");
    let text = write_plan(&bare).unwrap();
    assert_eq!(&text[..], b"wallet = \"wpkh(x)\"\n");
    assert_eq!(fields(&read_plan(&text).unwrap()), fields(&bare));
}

#[test]
fn a_hand_edited_plan_reads_as_the_written_one() {
    let text = "# a plan\nmarks = ['lost \"place0\" w0\\n']\n\n\
        holds = [ \"vault holds seed 73c5da0a\" ]\nplaces=[\"Home\",'']\n\
        answers = [\"seeds 1\", \"places 2\"]\nwallet = '''\nwpkh(XPUB/<0;1>/*)'''\n"
        .replace("XPUB", XPUB);
    let back = read_plan(text.as_bytes()).unwrap();
    let mut got = fields(&back);
    let mut want = fields(&plan());
    got.sort();
    want.sort();
    assert_eq!(got, want);
}

/// A plan of `n` marks, each a single backslash in a literal string.
fn backslash_marks(n: usize) -> Vec<u8> {
    let mut t = b"wallet = \"x\"\nmarks = [".to_vec();
    for i in 0..n {
        if i > 0 {
            t.extend_from_slice(b", ");
        }
        t.extend_from_slice(b"'\\'");
    }
    t.extend_from_slice(b"]\n");
    t
}

#[test]
fn a_plan_that_could_not_be_written_again_is_refused_as_too_long() {
    // Read, the marks are short; written, each backslash is escaped.
    let long = backslash_marks(190_000);
    assert!(long.len() < MAX_VALUE);
    let r = read_plan(&long).unwrap_err();
    assert_eq!((r.why, r.key.as_str()), (Why::TooLong, "plan"));
    // A plan that fits both ways is read and written again.
    let fits = read_plan(&backslash_marks(100_000)).unwrap();
    assert!(write_plan(&fits).is_ok());
    // The edge: the most marks that are read are the most that are written.
    let (mut ok, mut bad) = (100_000, 190_000);
    while bad - ok > 1 {
        let mid = (ok + bad) / 2;
        if read_plan(&backslash_marks(mid)).is_ok() {
            ok = mid;
        } else {
            bad = mid;
        }
    }
    let last = read_plan(&backslash_marks(ok)).unwrap();
    assert!(write_plan(&last).is_ok());
    assert_eq!(
        read_plan(&backslash_marks(bad)).unwrap_err().why,
        Why::TooLong
    );
}

#[test]
fn a_plan_refuses_what_section_7_does_not_name() {
    let refused = |t: &str| {
        let r = read_plan(t.as_bytes()).unwrap_err();
        (r.why, r.key)
    };
    assert_eq!(
        refused("wallet = \"x\"\nextra = 1\n"),
        (Why::Unknown, "extra".into())
    );
    assert_eq!(
        refused("places = [\"a\"]\n"),
        (Why::Missing, "wallet".into())
    );
    assert_eq!(refused("wallet = 3\n"), (Why::WrongType, "wallet".into()));
    assert_eq!(
        refused("wallet = \"x\"\nplaces = [1]\n"),
        (Why::WrongType, "places".into())
    );
    assert_eq!(
        refused("wallet = \"x\"\nanswers = [\"a\\nb\"]\n"),
        (Why::BadValue, "answers".into())
    );
    // A syntax refusal names the line.
    assert_eq!(
        read_plan(b"wallet = \"x\"\n\nplaces = [\n")
            .unwrap_err()
            .why,
        Why::Syntax(3)
    );
    assert_eq!(
        read_plan(b"wallet = \"\xff\"").unwrap_err().why,
        Why::NotUtf8
    );
    // A plan is a plan record and no other.
    assert_eq!(
        write_plan(&Record::new(kind::NOTE).with(field::NOTE, b"x"))
            .unwrap_err()
            .why,
        Why::Unknown
    );
}

#[test]
fn a_codex32_set_is_written_and_read_back() {
    let text = write_codex32_facts(&facts()).unwrap();
    assert_eq!(
        std::str::from_utf8(&text).unwrap(),
        "{k = 2, id = \"name\", lengths = [48], unknown = 1, shares = \
         [{index = \"a\", state = \"made\"}, {index = \"c\", state = \"placed\"}]}"
    );
    let back = read_codex32_facts(&text).unwrap();
    assert_eq!(back.threshold, 2);
    assert_eq!(back.identifier, *b"name");
    assert_eq!(back.length, 48);
    assert_eq!(back.unknown, 1);
    assert_eq!(back.strings, facts().strings);
    // Every state, and a set with no strings known.
    let states = [
        State::Made,
        State::Placed,
        State::Lost,
        State::Exposed,
        State::Destroyed,
    ];
    let all = SetFacts {
        strings: states.iter().zip(b"acdef").map(|(s, c)| (*c, *s)).collect(),
        unknown: 0,
        ..facts()
    };
    let back = read_codex32_facts(&write_codex32_facts(&all).unwrap()).unwrap();
    assert_eq!(back.strings, all.strings);
    let secret = SetFacts {
        threshold: 0,
        unknown: 0,
        strings: vec![(b's', State::Made)],
        ..facts()
    };
    let back = read_codex32_facts(&write_codex32_facts(&secret).unwrap()).unwrap();
    assert_eq!((back.threshold, back.strings), (0, secret.strings));
}

#[test]
fn a_codex32_set_may_be_written_another_way_and_refuses_what_it_does_not_name() {
    let read = |t: &str| read_codex32_facts(t.as_bytes());
    // Equal lengths in the array are read; spaces and string forms vary.
    let ok = read("{ k=2, id='name', lengths=[48, 48], unknown=1, shares=[] }").unwrap();
    assert_eq!((ok.threshold, ok.length, ok.strings.len()), (2, 48, 0));
    let refused = |t: &str| {
        let r = read(t).unwrap_err();
        (r.why, r.key)
    };
    assert_eq!(
        refused("{k = 2, id = \"name\", lengths = [48, 74], unknown = 1}"),
        (Why::BadCodex32, "lengths".into())
    );
    assert_eq!(
        refused("{k = 2, id = \"name\", lengths = [48], unknown = 1, x = 1}"),
        (Why::Unknown, "x".into())
    );
    assert_eq!(
        refused("{id = \"name\", lengths = [48], unknown = 1}"),
        (Why::Missing, "k".into())
    );
    assert_eq!(
        refused("{k = 1, id = \"name\", lengths = [48], unknown = 1}"),
        (Why::BadCodex32, "".into())
    );
    assert_eq!(
        refused(
            "{k = 2, id = \"name\", lengths = [48], unknown = 1, shares = [{index = \"a\", state = \"gone\"}]}"
        ),
        (Why::BadValue, "shares.state".into())
    );
    // The same index twice is not a set.
    assert_eq!(
        refused(
            "{k = 2, id = \"name\", lengths = [48], unknown = 0, shares = [{index = \"a\", state = \"made\"}, {index = \"a\", state = \"lost\"}]}"
        ),
        (Why::BadCodex32, "".into())
    );
    // A second key after the table, a table that is not one, and text that
    // is not TOML.
    assert_eq!(
        refused("{k = 2, id = \"name\", lengths = [48], unknown = 1}\nother = 1").0,
        Why::Unknown
    );
    assert_eq!(refused("[1]").0, Why::WrongType);
    assert!(matches!(refused("{k = ").0, Why::Syntax(1)));
}

#[test]
fn a_bip85_origin_is_written_and_read_back() {
    let text = write_child_facts(&child()).unwrap();
    assert_eq!(
        std::str::from_utf8(&text).unwrap(),
        "{parent = \"73c5da0a\", application = 39, language = 0, words = 12, index = 7}"
    );
    let back = read_child_facts(&text).unwrap();
    assert_eq!(
        (back.parent, back.language, back.words, back.index),
        ([0x73, 0xc5, 0xda, 0x0a], 0, 12, 7)
    );
    let top = ChildOf {
        index: 0x7fff_ffff,
        words: 24,
        language: 8,
        ..child()
    };
    let back = read_child_facts(&write_child_facts(&top).unwrap()).unwrap();
    assert_eq!(
        (back.index, back.words, back.language),
        (0x7fff_ffff, 24, 8)
    );
}

#[test]
fn a_bip85_origin_refuses_what_it_does_not_name() {
    let refused = |t: &str| {
        let r = read_child_facts(t.as_bytes()).unwrap_err();
        (r.why, r.key)
    };
    let good = "parent = \"73c5da0a\", application = 39, language = 0, words = 12, index = 7";
    assert!(read_child_facts(format!("{{{good}}}").as_bytes()).is_ok());
    assert_eq!(
        refused(&format!("{{{good}, extra = 1}}")),
        (Why::Unknown, "extra".into())
    );
    assert_eq!(
        refused(&format!("{{{}}}", good.replace("39", "40"))),
        (Why::BadChild, "application".into())
    );
    assert_eq!(
        refused(&format!("{{{}}}", good.replace("73c5da0a", "73C5DA0A"))),
        (Why::BadHex, "parent".into())
    );
    assert_eq!(
        refused(&format!("{{{}}}", good.replace("words = 12", "words = 13"))).0,
        Why::BadChild
    );
    assert_eq!(
        refused("{parent = \"73c5da0a\", application = 39, language = 0, words = 12}"),
        (Why::Missing, "index".into())
    );
    assert_eq!(refused(&format!("{{{good}}}\nx = 1")).0, Why::Unknown);
    assert_eq!(refused("\"73c5da0a\"").0, Why::WrongType);
}
