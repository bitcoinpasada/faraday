//! A labelled picture drawn on `Canvas` in the baked fonts and written as PNG (09 §3.2).

use alloc::vec::Vec;

use faraday_crypto::Secret;
use faraday_text::Text;

/// A picture with one label, as PNG bytes. The one string is passed in by the caller (section 10
/// §6).
// Spec: 09 §3.2 names `labelled_png` without a signature; `faraday_codec::png` writes the PNG.
pub fn labelled_png(_label: &Text, _width: u16, _height: u16) -> Secret<Vec<u8>> {
    todo!()
}
