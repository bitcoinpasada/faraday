//! Every word Faraday adds to OpenSigner's.
//!
//! OpenSigner's wording is `opensigner_core::strings`; Faraday's own is
//! here, in the same shape: [`Strings`], one static per language ([`EN`]).
//! Templates are filled by `opensigner_core::strings::fill`, re-exported
//! here as [`fill`] for the shells.

mod en;

pub use en::{EN, Strings};
pub use opensigner_core::strings::fill;

use zeroize::Zeroizing;

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
