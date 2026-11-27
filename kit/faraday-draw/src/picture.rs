//! A labelled picture drawn on `Canvas` in the baked fonts and written as PNG (09 §3.2).

use alloc::vec::Vec;

use faraday_codec::qr::QrMatrix;
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;

/// Costs what `faraday_codec::png` costs. `code` with `title` and then each of `lines` under
/// it, as a PNG file. The words are the caller's (section 10 §6). The pixels drawn are wiped
/// before it returns, and the file is a wiping buffer, since the code may be a secret's.
pub fn labelled_png(
    _c: &Compute,
    _code: &QrMatrix,
    _title: &str,
    _lines: &[&str],
) -> Secret<Vec<u8>> {
    todo!()
}
