//! A text file's part of a split seed, and the other things a file can be
//! that hold a secret, over the published SLIP-39 and BIP 93 vectors.

use crate::common::{parse, tools_vector};
use faraday_bip::json::Value;
use faraday_intake::{
    Part, VAULT_MAGIC, file_words, is_kdbx, is_vault, looks_like_entries, part_of,
};

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn every_share_of_the_valid_slip39_vectors_reads_as_a_slip39_share() {
    let vectors = parse(&tools_vector("slip39/vectors.json"));
    let mut read = 0;
    for case in vectors.as_array().unwrap() {
        let case = case.as_array().unwrap();
        // The vectors whose secret is empty are the invalid ones.
        if case[2].as_str().unwrap().is_empty() {
            continue;
        }
        for share in case[1].as_array().unwrap() {
            let share = share.as_str().unwrap();
            assert!(
                matches!(part_of(share), Some(Part::Slip39(_))),
                "{}: {share}",
                case[0].as_str().unwrap()
            );
            read += 1;
        }
    }
    assert!(read > 10, "{read}");
}

#[test]
fn a_slip39_share_with_a_wrong_word_is_not_a_part() {
    let vectors = parse(&tools_vector("slip39/vectors.json"));
    let share = vectors.as_array().unwrap()[0].as_array().unwrap()[1]
        .as_array()
        .unwrap()[0]
        .as_str()
        .unwrap()
        .to_string();
    let (first, rest) = share.split_once(' ').unwrap();
    let other = if first == "academic" {
        "acid"
    } else {
        "academic"
    };
    assert!(part_of(&format!("{other} {rest}")).is_none());
}

#[test]
fn every_codex32_string_of_the_bip93_vectors_reads_as_a_codex32_part() {
    let vectors = parse(&tools_vector("bip93/vectors.json"));
    let mut read = 0;
    for s in vectors.get("secrets").and_then(Value::as_array).unwrap() {
        let string = s.get("string").and_then(Value::as_str).unwrap();
        assert!(
            matches!(part_of(string), Some(Part::Codex32(_))),
            "{string}"
        );
        read += 1;
    }
    for set in vectors.get("sets").and_then(Value::as_array).unwrap() {
        for share in strings(set, "shares") {
            assert!(matches!(part_of(&share), Some(Part::Codex32(_))), "{share}");
            read += 1;
        }
    }
    assert!(read >= 6, "{read}");
}

#[test]
fn a_codex32_string_bip93_lists_as_invalid_is_not_a_part_mixed_case_included() {
    let vectors = parse(&tools_vector("bip93/vectors.json"));
    let mut mixed = 0;
    for case in vectors.get("invalid").and_then(Value::as_array).unwrap() {
        for string in strings(case, "strings") {
            if string != string.to_lowercase() && string != string.to_uppercase() {
                mixed += 1;
            }
            assert!(part_of(&string).is_none(), "{string}");
        }
    }
    assert!(mixed > 0, "the vectors hold a mixed-case string");
}

#[test]
fn a_file_of_a_persons_notes_and_numbered_words_gives_the_words_alone() {
    let text = "# my backup\n1. abandon\n2) abandon\n03 about\n\n";
    assert_eq!(file_words(text).as_str(), "abandon abandon about");
}

#[test]
fn a_file_is_a_vault_or_a_keepass_database_by_how_it_begins() {
    // The magic and one of the four vault lengths, whatever the name.
    assert_eq!(VAULT_MAGIC, faraday_codec::vault_file::MAGIC);
    let mut vault = VAULT_MAGIC.to_vec();
    vault.resize(faraday_codec::vault_file::file_len(65_536), 0);
    assert!(is_vault(&vault));
    // The magic at another length is not a vault.
    let mut other = VAULT_MAGIC.to_vec();
    other.extend_from_slice(&[1, 2, 3]);
    assert!(!is_vault(&other));
    assert!(!is_vault(&vault[..vault.len() - 1]));
    assert!(!is_vault(b"FVL"));
    assert!(!is_vault(b"{\"wallet_type\": \"standard\"}"));
    let kdbx = [0x03, 0xD9, 0xA2, 0x9A, 0x67, 0xFB, 0x4B, 0xB5, 0, 0];
    assert!(is_kdbx(&kdbx));
    assert!(!is_kdbx(&vault));
}

#[test]
fn text_with_otpauth_or_title_lines_is_entries_for_a_vault() {
    assert!(looks_like_entries("otpauth://totp/Example?secret=ABC"));
    assert!(looks_like_entries("Title: Bank\npassword: x"));
    assert!(!looks_like_entries("abandon abandon about"));
}
