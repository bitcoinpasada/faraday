//! The sample timestamp (`docs/MODES.md` §5.3): in Practice the Check a
//! timestamp tile opens with the proof, the hash of the file it stamps
//! and the real header of the block it names taken, and checks as a
//! counted match.

use crate::common::demo_app;
use faraday_core::{Action, Screen};
use faraday_ots::{Header, Network, Outcome, Proof};
use faraday_shell_api::App;

/// The height of the block with [`BLOCK_HASH`].
const BLOCK_HEIGHT: u32 = 912_095;
const BLOCK_HASH: &str = "000000000000000000007f0753c6209263c7072e51b725e89e75a0c708b90345";

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn the_sample_header_is_the_block_the_proof_names_and_its_merkle_root_is_the_proofs() {
    let inputs = faraday_samples::SAMPLES.inputs;
    let header_bytes = unhex(inputs.timestamp_header);
    assert_eq!(header_bytes.len(), 80);
    // The block's hash is the double SHA-256 of its header, written backwards.
    let mut hash = faraday_crypto::sha256(&faraday_crypto::sha256(&header_bytes));
    hash.reverse();
    assert_eq!(hex(&hash), BLOCK_HASH);
    let header = Header::parse(Network::Mainnet, &header_bytes).unwrap();
    let c = faraday_core::testkit::compute();
    let proof = Proof::parse(&c, inputs.timestamp_proof).unwrap();
    // The proof's path from the file's hash ends at the merkle root of the
    // header: that is what a matched attestation says.
    assert_eq!(hex(proof.file_digest()), inputs.timestamp_hash);
    let report = proof.verify(&c, &unhex(inputs.timestamp_hash), &header);
    assert!(report.committed());
    assert!(matches!(
        report.paths[0],
        Outcome::BitcoinMatched { block_hash, height, .. }
            if hex(&block_hash) == BLOCK_HASH && height == BLOCK_HEIGHT
    ));
}

#[test]
fn practice_opens_the_tile_with_the_sample_checked() {
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Check a timestamp")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::OtsCheck);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "The header commits to this file"));
    assert!(texts.iter().any(|t| t == BLOCK_HASH));
}
