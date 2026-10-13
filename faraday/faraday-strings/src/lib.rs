//! Every word Faraday adds to OpenSigner's.
//!
//! OpenSigner's wording is `opensigner_core::strings`; Faraday's own is
//! here, in the same shape: [`Strings`], one static per language ([`EN`]).
//! Templates are filled by [`fill`], which behaves as
//! `opensigner_core::strings::fill`, so that a crate that holds no part of
//! OpenSigner's core can fill them too.

mod en;

pub use en::{EN, Strings};

use zeroize::Zeroizing;

/// Fills a template's `{}` placeholders from `args`, in order. A
/// placeholder with no argument is dropped; a spare argument is ignored.
///
/// Copied from `opensigner/opensigner-core/src/strings/mod.rs` at upstream
/// commit `d26b84e`.
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
