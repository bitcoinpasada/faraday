//! The reader against toml-test's TOML v1.0.0 cases for tables, inline
//! tables, arrays, keys and strings (`tools/vectors/toml/`): every invalid
//! document is refused and every valid one is accepted, but those that
//! Faraday's own rules refuse (`OUTSIDE`).

extern crate std;

use std::fs;
use std::path::{Path, PathBuf};
use std::string::String;
use std::vec::Vec;

use super::parse;

/// Valid documents the reader refuses by Faraday's own rule, with the
/// rule.
const OUTSIDE: &[(&str, &str)] = &[];

fn files(kind: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/toml/testdata")
        .join(kind);
    let mut out = Vec::new();
    for dir in fs::read_dir(&root).expect("the vendored cases") {
        for f in fs::read_dir(dir.unwrap().path()).unwrap() {
            out.push(f.unwrap().path());
        }
    }
    out.sort();
    out
}

fn name(p: &Path) -> String {
    let d = p.parent().unwrap().file_name().unwrap().to_string_lossy();
    std::format!("{d}/{}", p.file_name().unwrap().to_string_lossy())
}

#[test]
fn every_invalid_case_of_toml_test_is_refused() {
    let list = files("invalid");
    assert!(list.len() > 200);
    let mut wrongly = Vec::new();
    for p in &list {
        // A document that is not UTF-8 is refused before the reader.
        let ok = fs::read(p)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .is_some_and(|t| parse(&t).is_ok());
        if ok {
            wrongly.push(name(p));
        }
    }
    assert!(wrongly.is_empty(), "accepted: {wrongly:?}");
}

#[test]
fn every_valid_case_of_toml_test_is_accepted_but_what_faradays_rules_refuse() {
    let list = files("valid");
    assert!(list.len() > 100);
    let mut wrongly = Vec::new();
    for p in &list {
        let text = String::from_utf8(fs::read(p).unwrap()).unwrap();
        let n = name(p);
        let outside = OUTSIDE.iter().any(|(f, _)| *f == n);
        if parse(&text).is_ok() == outside {
            wrongly.push(n);
        }
    }
    assert!(wrongly.is_empty(), "wrong: {wrongly:?}");
}
