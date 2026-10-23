//! A USB touchscreen's HID report descriptor, which the device writes and
//! the stick reads at plug-in to scale its touches: `hid::ranges` over
//! any bytes. The stick shell is a binary crate, so its file is included
//! by path.

#![no_main]

#[path = "../../shells/stick/src/hid.rs"]
#[allow(dead_code, missing_docs)]
mod hid;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = hid::ranges(data);
});
