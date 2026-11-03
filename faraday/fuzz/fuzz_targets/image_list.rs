//! The release's image list `IMAGES`: `ImageList::parse` over any bytes.
//! It is built into the Starter and read from the release, so the input
//! is the build's own; it must still refuse rather than panic.

#![no_main]

use faraday_payload::ImageList;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = ImageList::parse(data);
});
