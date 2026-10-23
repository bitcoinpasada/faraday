//! The property lists macOS's `diskutil -plist` prints, which carry
//! strings a stick's maker chose (a volume name, a model): the Starter's
//! `plist::parse` over any text. The Starter shell is a binary crate, so
//! `plist.rs` is included by path.

#![no_main]

#[path = "../../shells/starter/src/macos/plist.rs"]
#[allow(dead_code, missing_docs)]
mod plist;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = plist::parse(text);
    }
});
