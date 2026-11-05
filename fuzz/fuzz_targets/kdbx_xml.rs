//! The XML document of a KDBX file (`faraday_vault::kdbx::xml`) and what
//! the app reads of it (`faraday_vault::kdbx::items`): what a KeePass app
//! holding the passphrase can put in an authenticated file, read as
//! hostile. The input is the document as the inner header leaves it; when
//! it reads, it is written and read again and the two writes must agree,
//! and Faraday's items are read from both and must agree. The seeds are
//! documents of pykeepass and of Faraday, and inputs that stopped the target.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = faraday_vault::kdbx::fuzz_xml(data);
    let _ = faraday_vault::kdbx::fuzz_items(&faraday_crypto::work::Compute::unchecked(), data);
});
