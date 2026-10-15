//! `faraday-apk`'s reader over an APK's bytes: the ZIP's end, its central
//! directory, every local header and the binary manifest are the file's to
//! choose (`docs/APK-SIGNING.md` §5). A file that reads is also digested
//! the way signing does, with an end record counted at the offset of a
//! signing block, and read again by a source whose bytes change under the
//! reader after some reads, since a stick can change a file between two
//! reads of it.

#![no_main]

use faraday_apk::sign::{content_digest, padding_before_block};
use faraday_apk::zip::{Apk, Slice, Source, names_manifest};
use faraday_apk::{axml, pem};
use libfuzzer_sys::fuzz_target;

/// A source whose read number `at` finds one byte flipped.
struct Flaky<'a> {
    bytes: &'a [u8],
    reads: usize,
    at: usize,
}

impl Source for Flaky<'_> {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn read_at(&mut self, at: u64, buf: &mut [u8]) -> bool {
        self.reads += 1;
        let ok = Slice(self.bytes).read_at(at, buf);
        if ok && self.reads == self.at && !buf.is_empty() {
            let i = (self.at * 7) % buf.len();
            buf[i] ^= 0x55;
        }
        ok
    }
}

fuzz_target!(|data: &[u8]| {
    let _ = names_manifest(data);
    let _ = pem::read(data);
    let _ = axml::parse(data);
    let Some((&mode, file)) = data.split_first() else {
        return;
    };
    let mut plain = Slice(file);
    if let Ok(apk) = Apk::read(&mut plain) {
        let padding = padding_before_block(apk.cd_offset);
        let _ = content_digest(&mut plain, &apk, padding);
    }
    let mut flaky = Flaky {
        bytes: file,
        reads: 0,
        at: 1 + usize::from(mode % 16),
    };
    if let Ok(apk) = Apk::read(&mut flaky) {
        let _ = content_digest(&mut flaky, &apk, 0);
    }
});
