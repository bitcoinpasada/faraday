//! What a caller of `Text` notices.

use std::collections::HashMap;

use faraday_text::{Text, text};

#[test]
fn fill_matches_the_strings_crate() {
    // Each result is what `faraday_strings::fill` gives for the same
    // template and arguments.
    let cases: [(&str, &[&str], &str); 6] = [
        ("Word {} of {}", &["7", "12"], "Word 7 of 12"),
        ("no placeholder", &["x"], "no placeholder"),
        ("{} and {}", &["a"], "a and "),
        ("Key {}", &["73c5da0a"], "Key 73c5da0a"),
        ("{}{}{}", &["ä", "", "€"], "ä€"),
        ("{}", &["a", "spare"], "a"),
    ];
    for (template, args, want) in cases {
        assert_eq!(Text::fill(template, args), want);
    }
}

#[test]
fn text_macro_formats() {
    let t = text!("{}-{:03}-{}", "a", 7, 'z');
    assert_eq!(t, "a-007-z");
    assert_eq!(text!(""), "");
}

#[test]
fn numbers_at_their_limits() {
    assert_eq!(&*Text::num(0), "0");
    assert_eq!(&*Text::num(u64::MAX), "18446744073709551615");
    assert_eq!(&*Text::int(0), "0");
    assert_eq!(&*Text::int(-1), "-1");
    assert_eq!(&*Text::int(i64::MAX), "9223372036854775807");
    assert_eq!(&*Text::int(i64::MIN), "-9223372036854775808");
}

#[test]
fn growth_keeps_the_content() {
    let mut t = Text::new();
    let phrase = "abandon é ".repeat(300);
    for c in phrase.chars() {
        t.push(c);
    }
    assert_eq!(t, phrase.as_str());
    assert_eq!(t.pop(), Some(' '));
    t.truncate(7);
    assert_eq!(t, "abandon");
    t.set("zoo");
    assert_eq!(t, "zoo");
    t.clear();
    assert!(t.is_empty());
}

#[test]
fn truncate_off_a_boundary_keeps_everything() {
    let mut t = Text::of("é");
    t.truncate(1);
    assert_eq!(t, "é");
}

#[test]
fn lossy_text_replaces_broken_bytes() {
    let bytes = b"ok \xff\xfe end \xe2\x82";
    assert_eq!(
        Text::from_utf8_lossy(bytes),
        String::from_utf8_lossy(bytes).as_ref()
    );
    assert_eq!(Text::from_utf8_lossy("héllo".as_bytes()), "héllo");
}

#[test]
fn a_map_is_looked_up_by_str() {
    let mut m: HashMap<Text, u32> = HashMap::new();
    m.insert(Text::of("zoo"), 1);
    assert_eq!(m.get("zoo"), Some(&1));
    assert_eq!(m.get("abandon"), None);
    assert_eq!(Text::of("a"), Text::of("a"));
    assert!(Text::of("a") < Text::of("b"));
}

#[test]
fn collecting_and_copying() {
    let t: Text = "abc".chars().collect();
    let u: Text = ["ab", "c"].into_iter().collect();
    assert_eq!(t, u);
    let v = t.clone();
    assert_eq!(v, "abc");
    assert_eq!(format!("{v:?}"), "Text(3 bytes)");
}
