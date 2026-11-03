//! What a KDBX file's body holds once the cipher has been taken off, which
//! only a file whose HMACs verified reaches (`faraday_vault::kdbx`): a KeePass
//! app holding the passphrase can make an authenticated file with any
//! padding, any gzip stream and any inner header, so these are read as
//! hostile. The first byte chooses the shape (bit 0: AES-256's padding is
//! on the body; bit 1: gzip), the rest is the decrypted body. The seeds are
//! bodies of each shape.

// The entry exists only under cargo-fuzz's `--cfg fuzzing`; the plain
// `cargo check` of the fuzz crate (`just deps`) builds this file empty.
#![no_main]

#[cfg(fuzzing)]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Some((&shape, body)) = data.split_first() {
        let _ = faraday_vault::kdbx::fuzz_body(body, shape & 1 != 0, shape & 2 != 0);
    }
});
