//! Words in a file or typed: the first byte picks the language, the rest
//! is text for `Mnemonic::parse`, for `forms::typed_mnemonic` (what Files
//! does with a words file, and what typing does) and for
//! `faraday_intake::file_words`, which strips notes and numbering first.
//! The other word lists are reached the same way, one language at a time.

#![no_main]

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::forms::typed_mnemonic;
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let lang = Language::ALL[usize::from(first) % Language::ALL.len()];
    let c = Compute::unchecked();
    let _ = Mnemonic::parse(&c, lang, text);
    let _ = typed_mnemonic(lang, text);
    let words = faraday_intake::file_words(text);
    let _ = typed_mnemonic(lang, &words);
});
