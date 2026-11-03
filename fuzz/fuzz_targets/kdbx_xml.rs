//! The XML document of a KDBX file (`faraday_vault::kdbx::xml`): what a
//! KeePass app holding the passphrase can put in an authenticated file, read
//! as hostile. The input is the document as the inner header leaves it;
//! when it reads, it is written and read again and the two writes must
//! agree. The seeds are documents of pykeepass and of Faraday.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = faraday_vault::kdbx::fuzz_xml(data);
});
