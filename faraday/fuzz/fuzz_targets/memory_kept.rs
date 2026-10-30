//! What the app wrote to its kept list and reads back at the next start:
//! `memory::decode` (the signed amounts, which writes back to what it
//! read), `backup_flow::decode` (a backup in progress), and the whole
//! kept list as `StorageEvent::Restored` carries it, each entry holding
//! the input, which runs `Receipt::decode` and `backups::decode` with the
//! rest. The files are the stick's or the host's own, so a damaged one
//! must be dropped, never panic.

#![no_main]

use faraday_core::{StorageEvent, backup_flow, memory, testkit};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

/// The kept entries `Faraday::restore_kept` reads.
const NAMES: [&str; 16] = [
    "settings",
    "stick-settings",
    "signed-amounts",
    "checked-wallets",
    "family",
    "clock",
    "receipt",
    "renamed",
    "backup-flow",
    "backup-gate",
    "backups",
    "vault-from",
    "vault-summaries",
    "secret-out",
    "pictures",
    "idle",
];

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    let signed = memory::decode(data);
    assert_eq!(memory::decode(&memory::encode(&signed)), signed);
    let _ = backup_flow::decode(&c, data);

    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: NAMES
            .iter()
            .map(|n| ((*n).to_string(), data.to_vec()))
            .collect(),
    });
});
