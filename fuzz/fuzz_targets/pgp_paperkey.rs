//! `faraday_pgp::paperkey` (`docs/GPG-PAPERKEY.md` §2): a paperkey typed
//! or scanned. The bytes as base16 text, as one typed line, and as the raw
//! form; whatever reads has every CRC right, its entries end where the
//! data does, and the header checks never pass more entries than keys.

#![no_main]

use faraday_pgp::crc24;
use faraday_pgp::paperkey::{self, check, entries, read_raw, read_text};
use libfuzzer_sys::fuzz_target;

/// What the data reads to, checked against itself: its entries cover it,
/// and checking it against their own fingerprints gives no more of them.
fn data_holds(data: &[u8]) {
    if let Ok(found) = entries(data) {
        let ends = 1 + found.iter().map(|e| 23 + e.secret.len()).sum::<usize>();
        assert_eq!(ends, data.len());
        let keys: Vec<[u8; 20]> = found.iter().map(|e| e.fingerprint).collect();
        if let Ok(p) = check(data, &keys, true) {
            assert!(p.fingerprints.len() <= keys.len());
            assert_eq!(p.total, Some(data.len()));
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(read) = read_text(text) {
            data_holds(&read);
        }
        if let Ok(line) = paperkey::line(1, text) {
            if !line.bytes.is_empty() {
                assert_eq!(crc24(&line.bytes), line.crc);
            }
        }
    }
    if let Ok(read) = read_raw(data) {
        assert_eq!(read.len() + 3, data.len());
        data_holds(&read);
    }
    // The bytes as data, checked against two fingerprints taken from them.
    if data.len() >= 40 {
        let keys = [
            <[u8; 20]>::try_from(&data[..20]).unwrap(),
            <[u8; 20]>::try_from(&data[20..40]).unwrap(),
        ];
        for ended in [false, true] {
            if let Ok(p) = check(&data[40..], &keys, ended) {
                assert!(p.fingerprints.len() <= 2);
            }
        }
    }
});
