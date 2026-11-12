//! `aqrv::open` as it is, Argon2id included, on an envelope at the
//! smallest memory step (64 MiB, §5; any other step is not run, a run
//! being quick only there) under the vectors' passphrase: it never
//! panics, and an envelope that opens is recognised and of a kind its
//! size holds. Only the seeds (the vectors' own envelopes) are likely to
//! open: the tag is 4 or 8 bytes. `aqrv_envelope` goes through the rest of
//! `open` with the key given.

#![no_main]

use aqrv::{Step, open, recognise};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

/// The passphrase of the vectors in `tools/vectors/aqrv/aqrv.json`.
const PASSPHRASE: &str = "aqrv vector passphrase";

fuzz_target!(|data: &[u8]| {
    let Some(header) = recognise(data) else { return };
    if header.step != Step::Mib64 {
        return;
    }
    if let Ok(opened) = open(&Compute::unchecked(), data, PASSPHRASE) {
        assert_eq!(opened.header, header);
        let kind = opened.item.kind().expect("an item read has a kind");
        assert!(opened.header.size.holds(kind));
    }
});
