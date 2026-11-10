//! A Bitcoin Core descriptor wallet's records (`core_wallet::read`): the
//! input is a flags byte, four bytes of application id and then records,
//! each a one-byte key length, a two-byte little-endian value length, the
//! key and the value. Flag bit 0 gives the passphrase `faraday`, bits 1
//! and 2 pick the mode's network, bit 3 takes the application id as
//! written and not as the mode's network has it.

#![no_main]

use faraday_bip::keys::Network;
use faraday_core::core_wallet::read;
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&flags, rest)) = data.split_first() else {
        return;
    };
    let Some((id, mut rest)) = rest.split_first_chunk::<4>() else {
        return;
    };
    let mode = [
        Network::Mainnet,
        Network::Testnet,
        Network::Signet,
        Network::Regtest,
    ][usize::from(flags >> 1) & 3];
    let id = if flags & 8 != 0 {
        u32::from_le_bytes(*id)
    } else {
        match mode {
            Network::Mainnet => 0xF9BE_B4D9,
            Network::Testnet => 0x0B11_0907,
            Network::Signet => 0x0A03_CF40,
            Network::Regtest => 0xFABF_B5DA,
        }
    };
    let mut records = Vec::new();
    while let Some((head, tail)) = rest.split_first_chunk::<3>() {
        let klen = usize::from(head[0]);
        let vlen = usize::from(u16::from_le_bytes([head[1], head[2]]));
        if tail.len() < klen + vlen {
            break;
        }
        let (key, tail) = tail.split_at(klen);
        let (value, tail) = tail.split_at(vlen);
        records.push((key.to_vec(), value.to_vec()));
        rest = tail;
    }
    let passphrase = (flags & 1 != 0).then(|| SecretText::of("faraday"));
    let c = Compute::unchecked();
    let _ = read(&c, id, &records, mode, passphrase.as_ref());
});
