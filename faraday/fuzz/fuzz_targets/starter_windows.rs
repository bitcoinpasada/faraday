//! What the Starter on Windows reads that Windows or its own elevated copy
//! wrote: `disks::Snapshot::parse` (the disks as PowerShell lists them),
//! `bitlocker::Recorded::parse` and `Answer::decode` (BitLocker's
//! answers and the elevated copy's lines), `bitlocker::Order::parse` (the
//! elevated copy's arguments) and `pipe::Message::decode` (the writer's
//! lines, which write back to a line that reads to the same message).

#![no_main]

use faraday_starter_shell::windows::bitlocker::{Answer, Order, Recorded};
use faraday_starter_shell::windows::disks::Snapshot;
use faraday_starter_shell::windows::pipe::Message;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = Answer::decode(data);
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = Snapshot::parse(text);
    let _ = Recorded::parse(text);
    let args: Vec<String> = text.split_whitespace().map(str::to_string).collect();
    let _ = Order::parse(&args);
    if let Ok(message) = Message::decode(text) {
        let line = message.encode();
        assert_eq!(Message::decode(line.trim_end_matches('\n')), Ok(message));
    }
});
