//! The TOML inside a vault (`docs/VAULT.md` §2.1, §7): the QR vault's
//! keys written and read back, what a QR vault must not hold, and the
//! three TOML values a file vault keeps in KDBX custom data (a backup
//! plan, a seed's codex32 facts and its BIP-85 facts) written, read back
//! and refused as §7 says.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use faraday_vault::Record;
use faraday_vault::child::ChildOf;
use faraday_vault::codex32::{SetFacts, State};
use faraday_vault::contents::{
    MAX_VALUE, QR_LEN, QR_NOTE_MAX, QR_PASSPHRASE_MAX, Why, read_child_facts, read_codex32_facts,
    read_plan, read_qr, write_child_facts, write_codex32_facts, write_plan, write_qr,
};
use faraday_vault::layouts::words_payload;
use faraday_vault::records::{field, kind};

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

const TWELVE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const TWENTY_FOUR: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
const XPUB: &str = "[73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V";
const SHARE: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";

fn words(phrase: &str) -> Vec<u8> {
    let c = compute();
    let m = Mnemonic::parse(&c, Language::English, phrase).unwrap();
    words_payload(&m).to_vec()
}

fn fields(r: &Record) -> Vec<(u8, Vec<u8>)> {
    r.fields
        .iter()
        .map(|f| (f.number, f.bytes.to_vec()))
        .collect()
}

fn padded(text: String) -> Vec<u8> {
    let mut b = text.into_bytes();
    b.resize(QR_LEN, b'\n');
    b
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

fn qr_items() -> Vec<Record> {
    vec![
        Record::new(kind::KEY).with(field::KEY, &words(TWELVE)),
        Record::new(kind::KEY).with(field::KEY, &words(TWENTY_FOUR)),
        Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, SHARE.as_bytes())
            .with(field::CODEX32_KEY, &[0x73, 0xc5, 0xda, 0x0a]),
        Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &[0x73, 0xc5, 0xda, 0x0a])
            .with(field::PASSPHRASE, b"a short passphrase"),
    ]
}

#[test]
fn a_qr_vault_holds_one_item_in_one_length() {
    let c = compute();
    for item in qr_items() {
        let text = write_qr(&item).unwrap();
        assert_eq!(text.len(), QR_LEN);
        let back = read_qr(&c, &text).unwrap();
        assert_eq!(back.kind, item.kind);
        assert_eq!(fields(&back), fields(&item));
    }
}

#[test]
fn a_twelve_word_seed_and_a_twenty_four_word_one_make_codes_of_one_size() {
    let a = write_qr(&qr_items()[0]).unwrap();
    let b = write_qr(&qr_items()[1]).unwrap();
    assert_eq!(a.len(), b.len());
}

#[test]
fn a_qr_vault_refuses_what_it_cannot_hold() {
    let c = compute();
    // A 15-word seed, a seed with a label, a long passphrase.
    let fifteen = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon address";
    let m = Mnemonic::parse(&c, Language::English, fifteen).unwrap();
    let r = Record::new(kind::KEY).with(field::KEY, &words_payload(&m));
    assert_eq!(write_qr(&r).unwrap_err().why, Why::NotQr);
    let labelled = qr_items()[0].clone().with(field::KEY_LABEL, b"x");
    assert_eq!(write_qr(&labelled).unwrap_err().why, Why::NotQr);
    let long = Record::new(kind::PASSPHRASE)
        .with(field::PASSPHRASE_KEY, &[0; 4])
        .with(field::PASSPHRASE, &[b'p'; 200]);
    assert_eq!(write_qr(&long).unwrap_err().why, Why::TooLong);
    let wallet = Record::new(kind::SECURE_BOOT).with(1, &[1; 16]);
    assert_eq!(write_qr(&wallet).unwrap_err().why, Why::NotQr);
    // Wrong length, two items.
    let mut text = write_qr(&qr_items()[0]).unwrap().to_vec();
    text.push(b'\n');
    assert_eq!(read_qr(&c, &text).unwrap_err().why, Why::NotQr);
    let two = format!("words = \"{TWELVE}\"\nnote = \"a\"");
    let mut two = two.into_bytes();
    two.resize(QR_LEN, b'\n');
    assert_eq!(read_qr(&c, &two).unwrap_err().why, Why::Unknown);
    // Keys the form does not name, beside the ones it does.
    for extra in ["list = \"english\"", "load = false", "held = true"] {
        let mut text = format!("words = \"{TWELVE}\"\n{extra}").into_bytes();
        text.resize(QR_LEN, b'\n');
        let r = read_qr(&c, &text).unwrap_err();
        assert_eq!(
            (r.why, r.key.as_str()),
            (Why::Unknown, extra.split(' ').next().unwrap())
        );
    }
    let mut text =
        format!("codex32 = \"{SHARE}\"\nfor = \"73c5da0a\"\nwords = \"{TWELVE}\"").into_bytes();
    text.resize(QR_LEN, b'\n');
    assert_eq!(read_qr(&c, &text).unwrap_err().why, Why::Unknown);
    let edge = |n: usize| {
        Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &[0; 4])
            .with(field::PASSPHRASE, &vec![b'p'; n])
    };
    assert_eq!(write_qr(&edge(QR_PASSPHRASE_MAX)).unwrap().len(), QR_LEN);
    let r = write_qr(&edge(QR_PASSPHRASE_MAX + 1)).unwrap_err();
    assert_eq!((r.why, r.key.as_str()), (Why::TooLong, "passphrase"));
}

#[test]
fn a_qr_vault_may_hold_a_note_or_nothing() {
    let c = compute();
    let note = |t: &[u8]| Record::new(kind::NOTE).with(field::NOTE, t);
    let back = |r: &Record| {
        let text = write_qr(r).unwrap();
        assert_eq!(text.len(), QR_LEN);
        read_qr(&c, &text).unwrap()
    };
    // The most a note holds, in bytes of TOML text.
    let edge = |n: usize| note("a".repeat(n).as_bytes());
    assert_eq!(
        back(&edge(QR_NOTE_MAX)).field(field::NOTE).unwrap().len(),
        QR_NOTE_MAX
    );
    assert_eq!(
        write_qr(&edge(QR_NOTE_MAX + 1)).unwrap_err().why,
        Why::TooLong
    );
    // Quotes and backslashes are two bytes each.
    let esc = note("\"".repeat(QR_NOTE_MAX / 2).as_bytes());
    assert_eq!(
        back(&esc).field(field::NOTE).unwrap().len(),
        QR_NOTE_MAX / 2
    );
    let over = note("\\".repeat(QR_NOTE_MAX / 2 + 1).as_bytes());
    assert_eq!(write_qr(&over).unwrap_err().why, Why::TooLong);
    // Line feeds and other text.
    let text = "line one\nline two\ttab \u{e9}";
    assert_eq!(
        back(&note(text.as_bytes())).field(field::NOTE).unwrap(),
        text.as_bytes()
    );
    // A shorter way of writing the same line holds no more than the limit
    // once written: it is refused on read, naming the key.
    let long = "a".repeat(QR_NOTE_MAX + 1);
    for line in [format!("note=\"{long}\"\n"), format!("note='{long}'\n")] {
        let r = read_qr(&c, &padded(line)).unwrap_err();
        assert_eq!((r.why, r.key.as_str()), (Why::TooLong, "note"));
    }
    let long = "a".repeat(QR_PASSPHRASE_MAX + 1);
    for line in [
        format!("passphrase=\"{long}\"\nfor=\"73c5da0a\"\n"),
        format!("passphrase='{long}'\nfor=\"73c5da0a\"\n"),
    ] {
        let r = read_qr(&c, &padded(line)).unwrap_err();
        assert_eq!((r.why, r.key.as_str()), (Why::TooLong, "passphrase"));
    }
    // A literal string takes a line break or a backslash as it is, and the
    // code writes each as two bytes: what no longer fits once written is
    // refused on read, and what does fit is read back and written again.
    for (open, close, ch) in [("'''\n", "'''", "\n"), ("'", "'", "\\")] {
        for (n, fits) in [(QR_NOTE_MAX / 2, true), (QR_NOTE_MAX / 2 + 1, false)] {
            let line = format!("note = {open}{}{close}\n", ch.repeat(n));
            let r = read_qr(&c, &padded(line));
            if fits {
                let back = r.unwrap();
                assert_eq!(back.field(field::NOTE).unwrap(), ch.repeat(n).as_bytes());
                assert_eq!(write_qr(&back).unwrap().len(), QR_LEN);
            } else {
                let r = r.unwrap_err();
                assert_eq!((r.why, r.key.as_str()), (Why::TooLong, "note"));
            }
        }
    }
    // Nothing: the padding alone.
    let empty = write_qr(&note(b"")).unwrap();
    assert!(empty.iter().all(|&b| b == b'\n'));
    assert_eq!(back(&note(b"")).field(field::NOTE).unwrap(), b"");
    // Two forms mixed.
    let mut mixed = format!("words = \"{TWELVE}\"\nnote = \"a\"").into_bytes();
    mixed.resize(QR_LEN, b'\n');
    assert_eq!(read_qr(&c, &mixed).unwrap_err().why, Why::Unknown);
    let mut mixed = String::from("note = \"a\"\nfor = \"73c5da0a\"").into_bytes();
    mixed.resize(QR_LEN, b'\n');
    assert_eq!(read_qr(&c, &mixed).unwrap_err().why, Why::Unknown);
}
