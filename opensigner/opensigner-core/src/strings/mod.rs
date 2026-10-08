//! Every word a user can read.
//!
//! No view holds a string literal a user can see; they all come from
//! [`Strings`], one static per language ([`EN`]). `tools/lint-strings.sh`
//! enforces that, so a second language is one more static and a change of
//! wording is a change to one file.
//!
//! Templates carry `{}` placeholders and are filled by [`fill`], in order.
//! Joins with no words in them (`"{} · {}"`) stay in the view.

use alloc::string::String;

mod en;

pub use en::{EN, Strings};

/// Fills a template's `{}` placeholders from `args`, in order. A
/// placeholder with no argument is dropped; a spare argument is ignored.
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

/// Fills a template with one argument.
pub fn fill1(template: &str, a: &str) -> String {
    fill(template, &[a])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_substitutes_in_order() {
        assert_eq!(fill("Word {} of {}", &["7", "12"]), "Word 7 of 12");
        assert_eq!(fill("no placeholder", &["x"]), "no placeholder");
        assert_eq!(fill("{} and {}", &["a"]), "a and ");
        assert_eq!(fill1("Key {}", "73c5da0a"), "Key 73c5da0a");
    }
}
