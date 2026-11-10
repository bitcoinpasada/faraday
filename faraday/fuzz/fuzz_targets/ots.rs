//! OpenTimestamps proofs (`faraday-ots`): any bytes read as a proof, and a
//! proof that reads is walked against a fixed real block header, from its
//! own digest and from a wrong one. Any bytes also read as a block header.
//! Nothing panics, a wrong digest lists no path, and a proof read twice is
//! the same proof.

#![no_main]

use faraday_crypto::work::Compute;
use faraday_ots::{Header, Network, Proof};
use libfuzzer_sys::fuzz_target;

/// Bitcoin block 358391's header.
const HEADER: [u8; 80] = [
    0x02, 0x00, 0x00, 0x00, 0xb9, 0x63, 0x94, 0x58, 0x5a, 0x28, 0x1b, 0x7e, 0x5f, 0x43, 0x8f, 0xd1,
    0xc9, 0xed, 0x49, 0x26, 0x45, 0xa1, 0xfd, 0x61, 0xcb, 0x38, 0x02, 0x04, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x7e, 0xe4, 0x45, 0xd2, 0x3a, 0xd0, 0x61, 0xaf, 0x4a, 0x36, 0xb8,
    0x09, 0x50, 0x1f, 0xab, 0x1a, 0xc4, 0xf2, 0xd7, 0xe7, 0xa7, 0x39, 0x81, 0x7d, 0xd0, 0xcb, 0xb7,
    0xec, 0x66, 0x1b, 0x8a, 0x1e, 0x37, 0x67, 0x55, 0xf5, 0x86, 0x16, 0x18, 0x62, 0x72, 0xde, 0xf6,
];

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    let _ = Header::parse(Network::Regtest, data);
    let header = Header::parse(Network::Mainnet, &HEADER).expect("the fixed header is a real one");
    if let Ok(proof) = Proof::parse(&c, data) {
        assert_eq!(Proof::parse(&c, data), Ok(proof.clone()));
        let own = proof.verify(&c, proof.file_digest(), &header);
        assert!(own.file_matches);
        let mut wrong = proof.file_digest().to_vec();
        wrong[0] ^= 1;
        let other = proof.verify(&c, &wrong, &header);
        assert!(!other.file_matches && other.paths.is_empty() && !other.committed());
    }
});
