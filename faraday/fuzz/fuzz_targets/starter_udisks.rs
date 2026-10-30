//! What UDisks2 answers the Starter on Linux, as `busctl`-style text:
//! `linux::udisks::Objects::parse` over any text.

#![no_main]

use faraday_starter_shell::linux::udisks::Objects;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = Objects::parse(text);
});
