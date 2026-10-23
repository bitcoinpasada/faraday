//! An aezeed (Lightning) typed as 24 words: the input's first byte is the
//! passphrase's length, then the passphrase, then the rest is taken two
//! bytes at a time as word indices into `aezeed::decode_at_cost` with a
//! cost of 16, so a run stays fast (the product's own cost is the same
//! code with a larger `N`).

#![no_main]

use faraday_bip::aezeed::{self, NUM_WORDS};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&plen, rest)) = data.split_first() else {
        return;
    };
    let plen = usize::from(plen).min(rest.len());
    let (pass, rest) = rest.split_at(plen);
    let mut words = [0u16; NUM_WORDS];
    for (w, b) in words.iter_mut().zip(rest.chunks(2)) {
        let v = u16::from(b[0]) | b.get(1).map_or(0, |&h| u16::from(h) << 8);
        *w = v % 2048;
    }
    let c = Compute::unchecked();
    let _ = aezeed::decode_at_cost(&c, &words, pass, 16);
});
