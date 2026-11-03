//! A KDBX file as a vault reads it from a stick (`faraday_vault::kdbx`):
//! its first bytes, its header and the cost the header states, then the
//! whole file opened. The header's hash is made right before the second
//! look, so a changed header gets as far as the key derivation and the
//! authentication; only a cost small enough for a run to stay fast is
//! opened. The seeds are files whose passphrase is the one used here, so
//! the body's inflation and inner header are read too.

#![no_main]

use faraday_crypto::sha256;
use faraday_crypto::work::Compute;
use faraday_vault::kdbx;
use libfuzzer_sys::fuzz_target;

const PASSPHRASE: &str = "kdbx vector passphrase";

/// Where the header's end field ends, by the field lengths it states.
fn header_end(file: &[u8]) -> Option<usize> {
    let mut at = 12usize;
    loop {
        let id = *file.get(at)?;
        let len = u32::from_le_bytes(file.get(at + 1..at + 5)?.try_into().ok()?) as usize;
        at = at.checked_add(5)?.checked_add(len)?;
        if id == 0 {
            return (at <= file.len()).then_some(at);
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let _ = kdbx::recognise(data);
    let _ = kdbx::inspect(data);
    let mut file = data.to_vec();
    if let Some(end) = header_end(&file)
        && file.len() >= end + 32
    {
        let hash = sha256(&file[..end]);
        file[end..end + 32].copy_from_slice(&hash);
    }
    if let Ok(header) = kdbx::inspect(&file)
        && header.cost.memory_kib <= 8 * 1024
        && header.cost.passes <= 2
        && header.cost.lanes <= 2
    {
        let _ = kdbx::open(&Compute::unchecked(), &file, PASSPHRASE);
    }
});
