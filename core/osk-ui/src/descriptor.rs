//! Reading an output descriptor as structure rather than as a run of
//! characters (UX review 2026-09-07, §2.9).
//!
//! A descriptor is text a person checks token by token: the script
//! function, the key origin, the extended key, the chain suffix and the
//! checksum. Chunking it in fours cuts across every one of those, so
//! this module splits it into the tokens instead, and shortens only the
//! extended key, which is the one token nobody compares by eye.
//!
//! Pure text: [`organisms::descriptor`](crate::organisms::descriptor)
//! turns the tokens into a wrapped, coloured row.

use alloc::string::String;
use alloc::vec::Vec;

/// Characters kept at each end of a shortened extended key.
pub const KEY_HEAD: usize = 8;

/// Shortest run of key characters that is shortened at all: below this
/// the whole token is already shorter than its elision.
const KEY_MIN: usize = 2 * KEY_HEAD + 4;

/// The character the middle of a key is replaced by.
pub const ELLIPSIS: &str = "\u{2026}";

/// What a token is, which is what decides its colour and whether it
/// opens anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A function name, a bracket, a comma, a threshold.
    Syntax,
    /// A key origin, `[73c5da0a/84h/0h/0h]`, whole.
    Origin,
    /// An extended key, shortened in the middle.
    Key,
    /// A derivation suffix, `/<0;1>/*`, whole.
    Path,
    /// The `#checksum`, whole.
    Checksum,
}

/// One token of a descriptor: what is drawn, what it stands for, and
/// what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The text as it is drawn: a [`Kind::Key`] is shortened, every
    /// other kind is itself.
    pub text: String,
    /// The text as it is in the descriptor.
    pub full: String,
    /// What the token is.
    pub kind: Kind,
}

/// Splits `text` into the tokens a reader checks. Punctuation that
/// closes a token — `)` and `,` — joins the token in front of it, so a
/// line never begins with a bracket.
pub fn tokens(text: &str) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let (end, kind) = match c {
            '[' => (
                text[i..].find(']').map_or(bytes.len(), |p| i + p + 1),
                Kind::Origin,
            ),
            '/' => (
                run(bytes, i + 1, |c| !matches!(c, ',' | ')' | '(' | '#')),
                Kind::Path,
            ),
            '#' => (bytes.len(), Kind::Checksum),
            ')' | ',' => (i + 1, Kind::Syntax),
            '(' => (i + 1, Kind::Syntax),
            _ => {
                let end = run(bytes, i, |c| {
                    !matches!(c, '[' | ']' | '(' | ')' | ',' | '/' | '#')
                });
                let kind = if end - i >= KEY_MIN {
                    Kind::Key
                } else {
                    Kind::Syntax
                };
                (end, kind)
            }
        };
        let full = &text[i..end];
        // Punctuation belongs to the token beside it, and one run of
        // script syntax is one token: wrapping happens between the
        // parts a reader compares, not inside them.
        let joins_left = kind == Kind::Syntax
            && (matches!(c, ')' | ',') || out.last().is_some_and(|t| t.kind == Kind::Syntax));
        match out.last_mut() {
            Some(last) if joins_left => {
                last.text.push_str(full);
                last.full.push_str(full);
            }
            _ => out.push(Token {
                text: display(full, kind),
                full: String::from(full),
                kind,
            }),
        }
        i = end;
    }
    out
}

/// The extended key of `text`, whole, when it has one.
pub fn key(text: &str) -> Option<String> {
    tokens(text)
        .into_iter()
        .find(|t| t.kind == Kind::Key)
        .map(|t| t.full)
}

/// A key shortened to its first and last [`KEY_HEAD`] characters.
pub fn shorten(key: &str) -> String {
    let n = key.chars().count();
    if n < KEY_MIN {
        return String::from(key);
    }
    let head: String = key.chars().take(KEY_HEAD).collect();
    let tail: String = key.chars().skip(n - KEY_HEAD).collect();
    let mut out = head;
    out.push_str(ELLIPSIS);
    out.push_str(&tail);
    out
}

fn display(text: &str, kind: Kind) -> String {
    match kind {
        Kind::Key => shorten(text),
        _ => String::from(text),
    }
}

/// The end of the run of bytes from `start` for which `keep` holds.
fn run(bytes: &[u8], start: usize, keep: impl Fn(char) -> bool) -> usize {
    let mut i = start;
    while i < bytes.len() && keep(bytes[i] as char) {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    const DESC: &str = "wpkh([73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nQ6fgfPJTVsMnqhq8YnSzZfJkfSyBSEQ1UB6NxbrbHtjFVaShjhkyxKGRJyMUHBoNRAqvhbsSXqTDgeQg/<0;1>/*)#kdfx2n7d";

    #[test]
    fn the_origin_and_the_checksum_are_each_one_token() {
        let t = tokens(DESC);
        let kinds: Vec<Kind> = t.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                Kind::Syntax,
                Kind::Origin,
                Kind::Key,
                Kind::Path,
                Kind::Checksum
            ]
        );
        assert_eq!(t[0].text, "wpkh(");
        assert_eq!(t[1].text, "[73c5da0a/84h/0h/0h]");
        assert_eq!(t[3].text, "/<0;1>/*)");
        assert_eq!(t[4].text, "#kdfx2n7d");
    }

    #[test]
    fn the_key_keeps_eight_characters_at_each_end() {
        let t = tokens(DESC);
        let key = &t[2];
        assert_eq!(key.text, "xpub6Cat\u{2026}XqTDgeQg");
        assert!(key.full.starts_with("xpub6CatWdiZiodmU"));
        assert!(key.full.ends_with("SXqTDgeQg"));
    }

    #[test]
    fn a_nested_script_keeps_its_two_function_names() {
        let t = tokens(
            "sh(wpkh([aabbccdd/49h/0h/0h]xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGHePY8b6dhL1DcpUkMbc9DfR7QsvpuF1qNiXsCB6JHgHVgQfHdaPuzGVdM/0/*))#8rap84p3",
        );
        assert_eq!(t[0].text, "sh(wpkh(");
        assert_eq!(t[1].kind, Kind::Origin);
        assert_eq!(t[3].text, "/0/*))");
        assert_eq!(t[4].text, "#8rap84p3");
    }

    #[test]
    fn a_short_string_is_never_shortened() {
        assert_eq!(shorten("xpub123"), "xpub123");
        assert_eq!(key("wpkh(xpub123)"), None);
    }
}
