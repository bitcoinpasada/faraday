//! Every word Faraday adds to OpenSigner's.
//!
//! OpenSigner's wording is `opensigner_core::strings`; Faraday's own is
//! here, in the same shape: [`Strings`], one static per language ([`EN`]).
//! Templates are filled by `opensigner_core::strings::fill`.

mod en;

pub use en::{EN, Strings};
