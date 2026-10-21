//! Every word Faraday shows, in one file (`docs/DECISIONS.md` F68).
//!
//! [`Strings`] holds them, one static per language ([`EN`]), so that a
//! second language is one more static and a change of wording is a change
//! to one file. Templates carry `{}` placeholders and are filled by
//! [`fill`], in order. The Learn pages and the self-test's check names are
//! the two exceptions (CLAUDE.md).

mod en;

pub use en::{EN, Strings};

use zeroize::Zeroizing;

/// Fills a template's `{}` placeholders from `args`, in order. A
/// placeholder with no argument is dropped; a spare argument is ignored.
///
/// OpenSigner's own (`opensigner-core`'s `strings` module at upstream
/// commit `d26b84e`).
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    let mut next = args.iter();
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        if let Some(a) = next.next() {
            out.push_str(a);
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// [`fill`] for a template a secret goes into: the text is built once in
/// a [`Zeroizing`] string of its final size, so no copy of the secret is
/// left behind by a reallocation, and it is wiped when dropped.
pub fn fill_secret(template: &str, args: &[&str]) -> Zeroizing<String> {
    let len = template.len() + args.iter().map(|a| a.len()).sum::<usize>();
    let mut out = Zeroizing::new(String::with_capacity(len));
    let mut rest = template;
    let mut next = args.iter();
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        if let Some(a) = next.next() {
            out.push_str(a);
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_substitutes_in_order() {
        assert_eq!(fill("Word {} of {}", &["7", "12"]), "Word 7 of 12");
        assert_eq!(fill("no placeholder", &["x"]), "no placeholder");
        assert_eq!(fill("{} and {}", &["a"]), "a and ");
        assert_eq!(fill("Key {}", &["73c5da0a"]), "Key 73c5da0a");
    }
}
