//! Every case of `tools/vectors/aqrv/aqrv.json` (`docs/AQRV.md` §11),
//! computed by libraries the format's authors did not write: a case
//! opens to what it says, with every intermediate value, and sealing its
//! inputs again gives its envelope byte for byte; a case does not open
//! for the stated reason, is not recognised, or is refused by the writer.

use std::collections::VecDeque;

use aqrv::{Item, Kind, Layout, NotOpened, Refused, Size, Step, open, recognise, seal, workings};
use faraday_crypto::work::Compute;
use serde_json::Value;
use zeroize::Zeroizing;

/// The memory from which a case is run alone (512 MiB and 1 GiB).
const HEAVY_KIB: u64 = 524_288;

fn compute() -> Compute {
    Compute::unchecked()
}

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/aqrv/aqrv.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn cases_where(keep: impl Fn(&Value) -> bool) -> Vec<Value> {
    let v = vectors();
    v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| keep(c))
        .cloned()
        .collect()
}

fn size_of(case: &Value) -> Size {
    Size::of_len(case["size"].as_u64().unwrap() as usize).unwrap()
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap()
}

/// The item a case says a reader shows.
fn expected_item(case: &Value) -> Item {
    let item = &case["expect"]["item"];
    match text(item, "type") {
        "entropy" => Item::Entropy(Zeroizing::new(unhex(text(item, "hex")))),
        "raw_key" => Item::RawKey(Zeroizing::new(unhex(text(item, "hex")).try_into().unwrap())),
        "passphrase" => Item::Passphrase(Zeroizing::new(text(item, "text").to_owned())),
        "note" => Item::Note(Zeroizing::new(text(item, "text").to_owned())),
        "codex32" => Item::Codex32(Zeroizing::new(text(item, "string").to_owned())),
        other => panic!("an item of type {other}"),
    }
}

/// The text of an item, or its bytes in hex, with its kind's name.
fn shown(item: &Item) -> (&'static str, String) {
    match item {
        Item::Entropy(b) => ("entropy", hex(b)),
        Item::RawKey(k) => ("raw_key", hex(&k[..])),
        Item::Passphrase(t) => ("passphrase", t.to_string()),
        Item::Note(t) => ("note", t.to_string()),
        Item::Codex32(s) => ("codex32", s.to_string()),
    }
}

fn kind_key(kind: Kind) -> String {
    format!("0x{:02X}", kind as u8)
}

/// Checks one case that opens: the header, every value, the item, and the
/// envelope that sealing the case's inputs makes.
fn check_opens(case: &Value) {
    let name = text(case, "name");
    let c = compute();
    let envelope = unhex(text(case, "envelope"));
    let passphrase = text(case, "passphrase");
    let size = size_of(case);

    let header = recognise(&envelope).unwrap();
    assert_eq!(header.size, size, "{name}");
    assert_eq!(
        u64::from(header.step.memory_kib()),
        case["memory_kib"].as_u64().unwrap(),
        "{name}"
    );

    let w = workings(&c, &envelope, passphrase).unwrap();
    let values = &case["values"];
    for (key, got) in [
        ("pw", &w.pw),
        ("PRK", &w.prk),
        ("K_enc", &w.k_enc),
        ("K_mac", &w.k_mac),
        ("keystream", &w.keystream),
        ("P", &w.p),
        ("C", &w.c),
    ] {
        assert_eq!(hex(got), text(values, key), "{name}: {key}");
    }
    assert_eq!(
        hex(w.message.as_ref().unwrap()),
        text(values, "hmac_message"),
        "{name}"
    );
    for (kind, tag) in &w.tags {
        let want = text(&values["tags"], &kind_key(*kind));
        assert_eq!(hex(&tag[..size.tag_len()]), want, "{name}: tag {kind:?}");
    }

    let opened = open(&c, &envelope, passphrase).unwrap();
    assert_eq!(opened.header, header, "{name}");
    let want = expected_item(case);
    assert_eq!(opened.item.kind(), want.kind(), "{name}");
    assert_eq!(
        kind_key(opened.item.kind().unwrap()),
        text(case, "kind"),
        "{name}"
    );
    assert_eq!(shown(&opened.item), shown(&want), "{name}");
    if let Item::Codex32(s) = &opened.item {
        let is_secret = faraday_bip::codex32::Codex32::parse(s).unwrap().is_secret();
        assert_eq!(is_secret, case["expect"]["item"]["is_secret"], "{name}");
    }

    // Sealing again, with the case's own draws in §6's order.
    let mut draws: VecDeque<Vec<u8>> = VecDeque::new();
    draws.push_back(unhex(text(case, "salt")));
    if let Some(fill) = case.get("last_byte_fill") {
        let fill = u8::from_str_radix(fill.as_str().unwrap().trim_start_matches("0x"), 16);
        draws.push_back(vec![fill.unwrap()]);
    }
    if !text(case, "padding").is_empty() {
        draws.push_back(unhex(text(case, "padding")));
    }
    let mut random = |buf: &mut [u8]| {
        let next = draws.pop_front().expect("a draw the case does not give");
        assert_eq!(next.len(), buf.len(), "{name}: a draw's length");
        buf.copy_from_slice(&next);
    };
    let sealed = seal(&c, &want, passphrase, header.step, size, &mut random).unwrap();
    assert!(draws.is_empty(), "{name}: a draw left unused");
    assert_eq!(hex(&sealed), hex(&envelope), "{name}: sealed again");
}

fn opens(case: &Value) -> bool {
    case["expect"]["outcome"] == "opens"
}

fn heavy(case: &Value) -> bool {
    case["memory_kib"].as_u64().is_some_and(|k| k >= HEAVY_KIB)
}

#[test]
fn every_case_that_opens_opens_and_seals_again() {
    for case in cases_where(|c| opens(c) && !heavy(c)) {
        check_opens(&case);
    }
}

/// One after another, so that at most 1 GiB is held by these.
#[test]
fn the_512_mib_and_1_gib_cases_open_and_seal_again() {
    let cases = cases_where(|c| opens(c) && heavy(c));
    assert!(!cases.is_empty());
    for case in cases {
        check_opens(&case);
    }
}

/// The rule a case's `reason` names, for a tag that matched.
fn layout_of(reason: &str) -> Layout {
    if reason == "len is 0" {
        Layout::LenZero
    } else if reason.starts_with("len is ") {
        Layout::LenPastRoom
    } else if reason.contains("not UTF-8") {
        Layout::NotUtf8
    } else if reason.ends_with("is reserved") {
        Layout::Codex32Size
    } else if reason.contains("over the") {
        Layout::Codex32TooLong
    } else if reason.starts_with("the threshold field") {
        Layout::Codex32Threshold
    } else if reason.starts_with("threshold 0") {
        Layout::Codex32ThresholdZeroNotS
    } else {
        panic!("a reason this test does not know: {reason}")
    }
}

#[test]
fn every_case_that_does_not_open_is_refused_for_its_reason() {
    let cases = cases_where(|c| c["expect"]["outcome"] == "does_not_open");
    assert!(!cases.is_empty());
    for case in cases {
        let name = text(&case, "name");
        let envelope = unhex(text(&case, "envelope"));
        let got = open(&compute(), &envelope, text(&case, "passphrase"));
        let why = text(&case["expect"], "why");
        match got {
            Ok(_) => panic!("{name} opened"),
            Err(NotOpened::NoTag) => assert_eq!(why, "no kind's tag matches", "{name}"),
            Err(NotOpened::Layout(kind, rule)) => {
                assert_eq!(
                    why,
                    format!(
                        "the tag of kind {} matches and the item fails its layout",
                        kind_key(kind)
                    ),
                    "{name}"
                );
                assert_eq!(rule, layout_of(text(&case["expect"], "reason")), "{name}");
            }
            Err(other) => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn every_case_that_is_not_recognised_asks_for_no_passphrase() {
    let cases = cases_where(|c| c["expect"]["outcome"] == "not_recognised");
    assert!(!cases.is_empty());
    for case in cases {
        let name = text(&case, "name");
        let envelope = unhex(text(&case, "envelope"));
        assert!(recognise(&envelope).is_none(), "{name}");
        let got = open(&compute(), &envelope, "any passphrase");
        assert!(matches!(got, Err(NotOpened::NotRecognised)), "{name}");
    }
}

#[test]
fn the_writer_refuses_a_64_byte_codex32_seed_and_a_bad_checksum() {
    let v = vectors();
    let refused = v["writer_refuses"].as_array().unwrap();
    assert!(!refused.is_empty());
    for entry in refused {
        for size in [Size::Small, Size::Large] {
            let item = Item::Codex32(Zeroizing::new(text(entry, "string").to_owned()));
            let got = seal(&compute(), &item, "pass", Step::Mib64, size, &mut |_| {
                panic!("a refusal comes before any draw")
            });
            assert!(
                matches!(got, Err(Refused::Codex32(_))),
                "{}",
                text(entry, "reason")
            );
        }
    }
}
