//! Four small readers of text in `faraday-core`: a derivation path as the
//! Explore screen takes it (`explore::parse_path`), a time zone offset as
//! the vault's settings write it (`vault::parse_tz_offset`), a program's
//! `--version` output as the Starter reads a file it finds
//! (`parse_version_output`), and recovery words in each language
//! (`wallet::parse_words`).

#![no_main]

use faraday_bip::bip39::Language;
use faraday_core::{explore, parse_version_output, vault, wallet};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = explore::parse_path(text);
    let _ = vault::parse_tz_offset(text);
    let _ = parse_version_output(text);
    let c = Compute::unchecked();
    for lang in Language::ALL {
        let _ = wallet::parse_words(&c, lang, text);
    }
});
