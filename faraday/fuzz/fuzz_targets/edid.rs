//! What the stick reads of the screen and the firmware at start: the
//! kernel's boot parameters (the EDID block inside them, the Secure Boot
//! byte) and an EDID as a monitor, a KVM's capture side or the Starter's
//! display check hands it over. The input is read as boot parameters and
//! as an EDID.

#![no_main]

use faraday_core::{SecureBootAtStart, edid};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(block) = edid::from_boot_params(data) {
        let _ = edid::display(block);
        let _ = edid::edid_name(block);
        let _ = edid::edid_maker(block);
    }
    let _ = SecureBootAtStart::from_boot_params(data);
    let _ = edid::display(data);
    let _ = edid::edid_name(data);
    let _ = edid::edid_maker(data);
});
