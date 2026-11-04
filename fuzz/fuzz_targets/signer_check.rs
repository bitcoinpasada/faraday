//! `faraday_psbt::signer::check` and `nonce_view`: the file sent to a
//! signer and the file it returned, both from an online side and the
//! second from a possibly hostile device.
//!
//! The input is two files: a two byte big-endian length, that many bytes
//! of the sent file, then the rest as the returned file. Each is read as a
//! version 0 file, or as a version 2 file taken to version 0, as the
//! crate reads one. Where both read, `check` runs with an empty context.
//! It never panics, and a file checked against itself has no Blocked or
//! Danger finding other than a repeated nonce, the file taken as
//! `bitcoin` writes it (`to_bytes` keeps the bytes as read, and a file
//! with an odd encoding is E1 against its own fields): a file that already carries two signatures with one
//! nonce is reported for them whatever it is compared with, and bytes the
//! fields do not account for are E1 on the returned file.

#![no_main]

use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_psbt::signer::{Kind, check, nonce_view};
use faraday_psbt::v2::PsbtV2;
use faraday_psbt::{Context, Level, Psbt};
use libfuzzer_sys::fuzz_target;

/// A file as the crate reads one: version 0, else version 2 as version 0.
fn read(c: &Compute, bytes: &[u8]) -> Option<Psbt> {
    if let Ok(p) = Psbt::parse_bytes(c, bytes) {
        return Some(p);
    }
    PsbtV2::parse_bytes(c, bytes).ok()?.to_v0(c).ok()
}

fn run(
    c: &Compute,
    sent: &Psbt,
    returned: &Psbt,
    sb: &[u8],
    rb: &[u8],
) -> faraday_psbt::signer::Report {
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    check(c, sent, returned, sb, rb, &ctx)
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let c = Compute::unchecked();
    let n = (u16::from_be_bytes([data[0], data[1]]) as usize).min(data.len() - 2);
    let (sb, rb) = data[2..].split_at(n);

    if let (Some(sent), Some(returned)) = (read(&c, sb), read(&c, rb)) {
        let _ = run(&c, &sent, &returned, sb, rb);
        let _ = nonce_view(&sent, &returned);
    }
    // One file against itself.
    if let Some(p) = read(&c, sb) {
        let own = p.inner().serialize();
        let report = run(&c, &p, &p, &own, &own);
        assert!(
            report
                .findings
                .iter()
                .all(|f| matches!(f.kind, Kind::RepeatedNonce { .. })
                    || !matches!(f.level, Level::Blocked | Level::Danger)),
            "a file against itself: {:?}",
            report.findings
        );
        let _ = nonce_view(&p, &p);
    }
});
