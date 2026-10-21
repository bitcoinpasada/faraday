//! `faraday_bip::json` against RFC 8259's own examples and the cases its
//! grammar refuses, and over the wallet files other wallets write
//! (`faraday/faraday-core/tests/vectors/interop/`).

use faraday_bip::json::{self, Error, MAX_DEPTH, Value};

/// RFC 8259 §13, the first example.
const RFC_OBJECT: &str = r#"{
  "Image": {
    "Width":  800,
    "Height": 600,
    "Title":  "View from 15th Floor",
    "Thumbnail": {
      "Url":    "http://www.example.com/image/481989943",
      "Height": 125,
      "Width":  100
    },
    "Animated" : false,
    "IDs": [116, 943, 234, 38793]
  }
}"#;

/// RFC 8259 §13, the second example.
const RFC_ARRAY: &str = r#"[
  {
    "precision": "zip",
    "Latitude":  37.7668,
    "Longitude": -122.3959,
    "Address":   "",
    "City":      "SAN FRANCISCO",
    "State":     "CA",
    "Zip":       "94107",
    "Country":   "US"
  },
  {
    "precision": "zip",
    "Latitude":  37.371991,
    "Longitude": -122.026020,
    "Address":   "",
    "City":      "SUNNYVALE",
    "State":     "CA",
    "Zip":       "94085",
    "Country":   "US"
  }
]"#;

#[test]
fn the_rfcs_examples_read_as_the_values_they_write() {
    let v = json::parse(RFC_OBJECT).unwrap();
    let image = v.get("Image").unwrap();
    assert_eq!(image.get("Width").and_then(Value::as_u64), Some(800));
    assert_eq!(
        image.get("Title").and_then(Value::as_str),
        Some("View from 15th Floor")
    );
    assert_eq!(image.get("Animated"), Some(&Value::Bool(false)));
    let ids: Vec<u64> = image
        .get("IDs")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .filter_map(Value::as_u64)
        .collect();
    assert_eq!(ids, [116, 943, 234, 38793]);
    // The first "Height" in the order written is the image's own.
    assert_eq!(v.find("Height").and_then(Value::as_u64), Some(600));

    let v = json::parse(RFC_ARRAY).unwrap();
    assert_eq!(v.strings_named("City"), ["SAN FRANCISCO", "SUNNYVALE"]);
    let first = &v.as_array().unwrap()[0];
    assert_eq!(
        first.get("Longitude"),
        Some(&Value::Number("-122.3959".into()))
    );
    assert_eq!(first.get("Latitude").and_then(Value::as_u64), None);
}

#[test]
fn every_escape_is_undone_and_quote_writes_it_back() {
    let v = json::parse(r#""a\"b\\c\/d\b\f\n\r\t\u00e9\ud83d\ude00""#).unwrap();
    let s = "a\"b\\c/d\u{8}\u{c}\n\r\té😀";
    assert_eq!(v, Value::String(s.into()));
    assert_eq!(json::parse(&json::quote(s)), Ok(v));
    assert_eq!(json::quote("x\u{1}"), "\"x\\u0001\"");
}

#[test]
fn text_the_grammar_does_not_allow_is_refused() {
    for bad in [
        "",
        "{",
        "[1,]",
        "{\"a\":1,}",
        "{'a':1}",
        "01",
        "1.",
        ".5",
        "-",
        "1e",
        "+1",
        "tru",
        "nul",
        "\"a",
        "\"\\x\"",
        "\"\\u12\"",
        "\"\\ud800\"",
        "\"\\ud800\\u0041\"",
        "\"tab\there\"",
        "{\"a\" 1}",
        "{1:2}",
        "[1 2]",
        "1 2",
        "{} x",
    ] {
        assert!(
            matches!(json::parse(bad), Err(Error::Syntax(_))),
            "{bad:?} was read"
        );
    }
}

#[test]
fn a_key_named_twice_too_deep_or_too_long_is_refused() {
    assert_eq!(json::parse(r#"{"a":1,"a":2}"#), Err(Error::Duplicate));
    assert!(json::parse(r#"{"a":{"b":1},"b":{"b":2}}"#).is_ok());
    let deep = |n: usize| format!("{}{}", "[".repeat(n), "]".repeat(n));
    assert!(json::parse(&deep(MAX_DEPTH)).is_ok());
    assert_eq!(json::parse(&deep(MAX_DEPTH + 1)), Err(Error::TooDeep));
    let long = format!("\"{}\"", "a".repeat(json::MAX_SIZE));
    assert_eq!(json::parse(&long), Err(Error::TooLarge));
    assert!(json::parse("\u{feff} {} \n").is_ok());
}

/// Every JSON file other wallets wrote, among the interop vectors, reads
/// as JSON, and its writer's fields are where the file puts them.
#[test]
fn other_wallets_files_read_as_json() {
    let generic = include_str!(
        "../../../../faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardmk4-generic.json"
    );
    let v = json::parse(generic).unwrap();
    assert_eq!(v.get("xfp").and_then(Value::as_str), Some("086EE178"));
    assert!(v.get("bip84").and_then(|a| a.get("first")).is_some());
    let core = include_str!(
        "../../../../faraday/faraday-core/tests/vectors/interop/core/listdescriptors.json"
    );
    let v = json::parse(core).unwrap();
    assert!(v.strings_named("desc").len() >= 2);
    for text in [
        include_str!(
            "../../../../faraday/faraday-core/tests/vectors/interop/sparrow/specter-wallet.json"
        ),
        include_str!(
            "../../../../faraday/faraday-core/tests/vectors/interop/sparrow/electrum-multisig-wallet.json"
        ),
        include_str!(
            "../../../../faraday/faraday-core/tests/vectors/interop/sparrow/caravan-multisig-export-1.json"
        ),
        include_str!(
            "../../../../faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardQ.json"
        ),
    ] {
        assert!(json::parse(text).is_ok());
    }
}
