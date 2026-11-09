//! A text file's part of a split seed, and the other things a file can be
//! that hold a secret, over the published SLIP-39 and BIP 93 vectors.

use crate::common::{parse, tools_vector};
use faraday_bip::json::Value;
use faraday_intake::{Part, file_words, is_kdbx, is_vault, looks_like_entries, part_of};

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
fn a_file_is_a_vault_or_an_older_keepass_database_by_how_it_begins() {
    let head = |major: u8| {
        let mut v = faraday_codec::vault_file::SIGNATURE.to_vec();
        v.extend_from_slice(&[1, 0, major, 0]);
        v
    };
    // KDBX 4 of any length within a vault's limits, whatever the name.
    for len in [350, 65_921, 70_000, 4_194_797] {
        let mut vault = head(4);
        vault.resize(len, 0);
        assert!(is_vault(&vault), "{len}");
        assert!(!is_kdbx(&vault), "{len}");
    }
    // A vault's head alone, or past the largest, is no vault but is KeePass's.
    let past = faraday_codec::vault_file::MAX_FILE_LEN as usize + 1;
    for len in [12, 300, past] {
        let mut file = head(4);
        file.resize(len, 0);
        assert!(!is_vault(&file), "{len}");
        assert!(is_kdbx(&file), "{len}");
    }
    // Version 3.1 is KeePass's, not a vault.
    let mut old = head(3);
    old.resize(70_000, 0);
    assert!(!is_vault(&old));
    assert!(is_kdbx(&old));
    // The four bytes FVLT began with no longer make a vault.
    let mut fvlt = b"FVLT".to_vec();
    fvlt.resize(65_921, 0);
    assert!(!is_vault(&fvlt));
    assert!(!is_kdbx(&fvlt));
    assert!(!is_vault(b"{\"wallet_type\": \"standard\"}"));
}

#[test]
fn text_with_otpauth_or_title_lines_is_entries_for_a_vault() {
    assert!(looks_like_entries("otpauth://totp/Example?secret=ABC"));
    assert!(looks_like_entries("OTPAUTH://totp/Example?secret=ABC"));
    assert!(looks_like_entries("Title: Bank\npassword: x"));
    assert!(!looks_like_entries("abandon abandon about"));
}

#[test]
fn only_a_keepass_file_of_version_3_or_older_is_named_older() {
    use faraday_intake::is_older_kdbx;
    let head = |major: u8| {
        let mut v = faraday_codec::vault_file::SIGNATURE.to_vec();
        v.extend_from_slice(&[0, 0, major, 0]);
        v
    };
    let sized = |major: u8, len: usize| {
        let mut v = head(major);
        v.resize(len, 0);
        v
    };
    assert!(is_older_kdbx(&sized(3, 70_000)));
    // KDBX 5, a KDBX 4 too long or too short, and a cut file are KeePass's,
    // and none is an older one.
    for file in [
        sized(5, 70_000),
        sized(4, faraday_codec::vault_file::MAX_FILE_LEN as usize + 1),
        sized(4, 300),
        head(4),
        head(4)[..10].to_vec(),
    ] {
        assert!(is_kdbx(&file) && !is_older_kdbx(&file), "{}", file.len());
    }
}

#[test]
fn every_keepass_file_of_the_vectors_is_told_as_it_is() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/kdbx");
    let (mut vaults, mut older) = (0, 0);
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        if path.extension().is_none_or(|x| x != "kdbx") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("-3.1-")
        {
            assert!(is_kdbx(&bytes) && faraday_intake::is_older_kdbx(&bytes));
            older += 1;
        } else {
            assert!(is_vault(&bytes) && !is_kdbx(&bytes), "{path:?}");
            vaults += 1;
        }
    }
    assert_eq!((vaults, older), (11, 1));
}
