//! A vault keeps a key's Seed XOR set and SLIP-39 set as plain facts and
//! no part or share (`docs/VAULT.md` §7): written and read in a key
//! record and in a file vault's custom data, refused when they are not a
//! set, and a key record without them reads as it always did.
#![allow(clippy::disallowed_methods)]

use faraday_vault::contents::{
    Why, read_slip39_facts, read_xor_facts, write_slip39_facts, write_xor_facts,
};
use faraday_vault::records::{Fault, field, kind};
use faraday_vault::slip39::{Group, Slip39Facts, slip39_of};
use faraday_vault::xor::{XorFacts, xor_of};
use faraday_vault::{Record, slip39, xor};

fn key() -> Record {
    Record::new(kind::KEY).with(
        field::KEY,
        &faraday_vault::layouts::seed_payload(&[7; 16]).unwrap(),
    )
}

fn halves() -> XorFacts {
    XorFacts {
        parts: vec![[0xde, 0xad, 0xbe, 0xef], [1, 2, 3, 4], [0; 4]],
    }
}

fn two_groups() -> Slip39Facts {
    Slip39Facts {
        identifier: 0x7abc,
        exponent: 3,
        threshold: 2,
        groups: vec![
            Group {
                threshold: 2,
                count: 3,
                made: 0b101,
            },
            Group {
                threshold: 2,
                count: 16,
                made: 0x8000,
            },
        ],
    }
}

#[test]
fn a_seed_xor_set_is_kept_as_its_parts_fingerprints_and_nothing_else() {
    let payload = xor::payload(&halves()).unwrap();
    assert_eq!(payload, b"\x03\xde\xad\xbe\xef\x01\x02\x03\x04\0\0\0\0");
    let r = key().with(field::KEY_XOR, &payload);
    assert_eq!(r.check(), Ok(()));
    assert_eq!(xor_of(r.field(field::KEY_XOR).unwrap()), Some(halves()));
}

#[test]
fn a_slip39_set_is_kept_as_its_public_facts_and_nothing_else() {
    let payload = slip39::payload(&two_groups()).unwrap();
    assert_eq!(
        payload,
        b"\xbc\x7a\x03\x02\x02\x02\x03\x05\x00\x02\x10\x00\x80"
    );
    let r = key().with(field::KEY_SLIP39, &payload);
    assert_eq!(r.check(), Ok(()));
    assert_eq!(
        slip39_of(r.field(field::KEY_SLIP39).unwrap()),
        Some(two_groups())
    );
}

#[test]
fn a_key_record_without_the_sets_reads_as_before_and_has_none() {
    let r = key().with(field::KEY_LABEL, b"Savings");
    assert_eq!(r.check(), Ok(()));
    assert!(r.field(field::KEY_XOR).is_none() && r.field(field::KEY_SLIP39).is_none());
}

#[test]
fn what_is_not_a_set_is_not_written_and_a_record_holding_it_is_refused() {
    let xor_bad = |parts: Vec<[u8; 4]>| XorFacts { parts };
    assert_eq!(xor::payload(&xor_bad(vec![[1; 4]])), None, "one part");
    assert_eq!(xor::payload(&xor_bad(vec![[1; 4]; 5])), None, "5 parts");
    let bad = |f: &dyn Fn(&mut Slip39Facts)| {
        let mut s = two_groups();
        f(&mut s);
        s
    };
    for facts in [
        bad(&|s| s.identifier = 0x8000),
        bad(&|s| s.exponent = 16),
        bad(&|s| {
            s.groups[0] = Group {
                threshold: 1,
                count: 3,
                made: 0b101,
            }
        }),
        bad(&|s| s.threshold = 3),
        bad(&|s| s.threshold = 0),
        bad(&|s| s.threshold = 1),
        bad(&|s| s.groups[0].threshold = 4),
        bad(&|s| s.groups[0].count = 17),
        bad(&|s| s.groups[0].made = 0b1000),
        bad(&|s| s.groups.clear()),
    ] {
        assert_eq!(slip39::payload(&facts), None, "{facts:?}");
    }
    // Bytes that are not the layout: a count that disagrees with the length.
    let r = key().with(field::KEY_XOR, b"\x03\x01\x02\x03\x04");
    assert_eq!(r.check(), Err(Fault::Value(kind::KEY, field::KEY_XOR)));
    let mut raw = slip39::payload(&two_groups()).unwrap();
    raw.pop();
    let r = key().with(field::KEY_SLIP39, &raw);
    assert_eq!(r.check(), Err(Fault::Value(kind::KEY, field::KEY_SLIP39)));
}

#[test]
fn a_set_is_written_in_a_file_vaults_custom_data_and_read_back() {
    let text = write_xor_facts(&halves()).unwrap();
    assert_eq!(
        std::str::from_utf8(&text).unwrap(),
        "{parts = [\"deadbeef\", \"01020304\", \"00000000\"]}"
    );
    assert_eq!(read_xor_facts(&text).unwrap(), halves());
    let text = write_slip39_facts(&two_groups()).unwrap();
    assert_eq!(
        std::str::from_utf8(&text).unwrap(),
        "{id = 31420, exponent = 3, threshold = 2, groups = [{threshold = 2, count = 3, \
         made = [0, 2]}, {threshold = 2, count = 16, made = [15]}]}"
    );
    assert_eq!(read_slip39_facts(&text).unwrap(), two_groups());
}

#[test]
fn custom_data_that_is_not_a_set_is_refused_naming_the_key() {
    let xor = |t: &str| {
        let r = read_xor_facts(t.as_bytes()).unwrap_err();
        (r.why, r.key)
    };
    assert_eq!(xor("{parts = [\"01020304\"]}"), (Why::BadSplit, "".into()));
    assert_eq!(
        xor("{parts = [\"0102030\", \"01020304\"]}"),
        (Why::BadHex, "parts".into())
    );
    assert_eq!(
        xor("{parts = [\"01020304\", \"01020304\"], x = 1}"),
        (Why::Unknown, "x".into())
    );
    assert_eq!(xor("{}"), (Why::Missing, "parts".into()));
    let slip = |t: &str| {
        let r = read_slip39_facts(t.as_bytes()).unwrap_err();
        (r.why, r.key)
    };
    let ok =
        "{id = 1, exponent = 0, threshold = 1, groups = [{threshold = 2, count = 2, made = [0]}]}";
    assert_eq!(read_slip39_facts(ok.as_bytes()).unwrap().groups[0].made, 1);
    assert_eq!(
        slip(
            "{id = 1, exponent = 0, threshold = 1, groups = [{threshold = 2, count = 2, made = [0]}, \
             {threshold = 2, count = 2, made = [0]}]}"
        ),
        (Why::BadSplit, "".into())
    );
    assert_eq!(
        slip(
            "{id = 1, exponent = 0, threshold = 1, groups = [{threshold = 2, count = 2, made = [2]}]}"
        ),
        (Why::Range, "groups.made".into())
    );
    assert_eq!(
        slip(
            "{id = 1, exponent = 0, threshold = 1, groups = [{threshold = 2, count = 2, made = [0, 0]}]}"
        ),
        (Why::BadSplit, "groups.made".into())
    );
    assert_eq!(
        slip(
            "{id = 1, exponent = 0, threshold = 2, groups = [{threshold = 2, count = 2, made = []}]}"
        ),
        (Why::BadSplit, "".into())
    );
    assert_eq!(
        slip("{id = 32768, exponent = 0, threshold = 1, groups = []}"),
        (Why::Range, "id".into())
    );
}
