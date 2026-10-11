//! `osk_codec::seedqr::from_digits` and `from_entropy`: the two ways a
//! scanned SeedQR becomes a mnemonic.
//!
//! The first byte picks the wordlist, the rest is the payload, so one
//! corpus entry covers a language and its digits together.

#![no_main]

use libfuzzer_sys::fuzz_target;
use osk_bip::bip39::Language;
use osk_codec::seedqr;

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let lang = Language::ALL[usize::from(first) % Language::ALL.len()];
    let _ = seedqr::from_digits(rest, lang);
    let _ = seedqr::from_entropy(rest, lang);
});
