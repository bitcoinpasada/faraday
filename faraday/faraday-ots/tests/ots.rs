//! The reference's example proofs (`tools/vectors/opentimestamps/`) and
//! proofs built here, read and checked against block headers.

use faraday_crypto::work::Compute;
use faraday_ots::{Error, HashOp, Header, Network, Outcome, Proof};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-ots's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/opentimestamps")
}

fn example(name: &str) -> Vec<u8> {
    fs::read(vectors().join("examples").join(name)).unwrap()
}

fn header_at(height: u32) -> Header {
    let hex = fs::read_to_string(vectors().join(format!("headers/{height}.hex"))).unwrap();
    Header::parse(Network::Mainnet, &unhex(hex.trim())).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

const HEIGHTS: [u32; 6] = [129405, 358391, 455605, 465751, 466906, 912095];

#[test]
fn every_recorded_header_hashes_to_its_recorded_block_hash() {
    for h in HEIGHTS {
        let hash = fs::read_to_string(vectors().join(format!("headers/{h}.hash"))).unwrap();
        assert_eq!(hex(&header_at(h).block_hash()), hash.trim(), "height {h}");
    }
}

/// The report of `proof` for the file `data`, taken with the proof's own hash.
fn report_for(proof_name: &str, data: &[u8], height: u32) -> faraday_ots::Report {
    let proof = Proof::parse(&compute(), &example(proof_name)).unwrap();
    proof.verify(
        &compute(),
        &proof.file_op().digest(data),
        &header_at(height),
    )
}

fn matched(height: u32, report: &faraday_ots::Report, time: u32) {
    assert!(report.file_matches);
    assert!(report.committed());
    assert!(report.paths.iter().any(|o| matches!(
        o,
        Outcome::BitcoinMatched { height: h, time: t, .. } if *h == height && *t == time
    )));
}

#[test]
fn a_gdp_proof_through_sha256_only_is_committed_by_its_block() {
    let proof = Proof::parse(&compute(), &example("gdp2q25-2nd.pdf.ots")).unwrap();
    let r = proof.verify(&compute(), proof.file_digest(), &header_at(912095));
    matched(912095, &r, 1756386670);
    let Outcome::BitcoinMatched { block_hash, .. } = &r.paths[0] else {
        panic!()
    };
    assert_eq!(
        hex(block_hash),
        "000000000000000000007f0753c6209263c7072e51b725e89e75a0c708b90345"
    );
}

#[test]
fn a_path_through_ripemd160_reaches_its_block_and_is_counted() {
    // The reference's older proofs go through the calendar's RIPEMD-160.
    for (proof, file, height) in [
        ("hello-world.txt.ots", example("hello-world.txt"), 358391),
        ("empty.ots", Vec::new(), 129405),
    ] {
        let r = report_for(proof, &file, height);
        assert!(r.file_matches);
        assert!(r.committed());
        assert!(matches!(
            r.paths.as_slice(),
            [Outcome::BitcoinMatched { height: h, .. }] if *h == height
        ));
    }
}

#[test]
fn a_file_with_other_bytes_does_not_match_the_proof() {
    let r = report_for("hello-world.txt.ots", b"Hello World\n", 358391);
    assert!(!r.file_matches);
    assert!(r.paths.is_empty());
    assert!(!r.committed());
}

#[test]
fn a_proof_for_another_block_does_not_match_this_header() {
    let proof = Proof::parse(&compute(), &example("gdp2q25-2nd.pdf.ots")).unwrap();
    let r = proof.verify(&compute(), proof.file_digest(), &header_at(358391));
    assert!(!r.committed());
    assert_eq!(r.paths, vec![Outcome::BitcoinMismatch { height: 912095 }]);
}

#[test]
fn the_bad_stamp_does_not_reach_the_block_it_names() {
    let r = report_for("bad-stamp.txt.ots", &example("bad-stamp.txt"), 358391);
    assert!(r.file_matches);
    assert!(!r.committed());
    assert_eq!(r.paths, vec![Outcome::BitcoinMismatch { height: 358391 }]);
}

#[test]
fn a_proof_taken_with_sha1_is_named_and_not_counted() {
    let proof = Proof::parse(&compute(), &example("bitcoin.pdf.ots")).unwrap();
    assert_eq!(proof.file_op(), HashOp::Sha1);
    let r = proof.verify(&compute(), proof.file_digest(), &header_at(465751));
    assert!(!r.committed());
    assert_eq!(
        r.paths,
        vec![Outcome::BitcoinNotCounted {
            height: 465751,
            op: HashOp::Sha1,
            reaches_header: true
        }]
    );
}

#[test]
fn several_attestations_are_each_listed() {
    let r = report_for(
        "different-blockchains.txt.ots",
        &example("different-blockchains.txt"),
        455605,
    );
    matched(455605, &r, 1488561937);
    let pending = r
        .paths
        .iter()
        .filter(|o| matches!(o, Outcome::Pending { .. }))
        .count();
    assert_eq!(pending, 2);
    assert!(r.paths.contains(&Outcome::Pending {
        uri: "https://eth.ots.eternitywall.com".into()
    }));
    assert!(r.paths.contains(&Outcome::Ethereum { height: 3_283_713 }));
    assert_eq!(r.paths.len(), 4);
}

#[test]
fn a_path_through_sha1_is_named_and_not_counted() {
    // The reference counts this proof; the file's own hash is SHA-1.
    for f in ["a", "b"] {
        let data = fs::read(vectors().join("examples/sha1").join(f)).unwrap();
        let proof = Proof::parse(
            &compute(),
            &fs::read(vectors().join("examples/sha1/a-or-b.ots")).unwrap(),
        )
        .unwrap();
        assert_eq!(proof.file_op(), HashOp::Sha1);
        let r = proof.verify(&compute(), &HashOp::Sha1.digest(&data), &header_at(466906));
        assert!(r.file_matches);
        assert!(!r.committed());
        assert!(r.paths.contains(&Outcome::BitcoinNotCounted {
            height: 466906,
            op: HashOp::Sha1,
            reaches_header: true
        }));
    }
}

#[test]
fn pending_proofs_list_their_calendars() {
    let h = header_at(358391);
    let cases: [(&str, &str, &[&str]); 5] = [
        (
            "incomplete.txt",
            "incomplete.txt.ots",
            &["https://alice.btc.calendar.opentimestamps.org"],
        ),
        (
            "two-calendars.txt",
            "two-calendars.txt.ots",
            &[
                "https://alice.btc.calendar.opentimestamps.org",
                "https://bob.btc.calendar.opentimestamps.org",
            ],
        ),
        (
            "merkle1.txt",
            "merkle1.txt.ots",
            &[
                "https://alice.btc.calendar.opentimestamps.org",
                "https://bob.btc.calendar.opentimestamps.org",
            ],
        ),
        (
            "merkle2.txt",
            "merkle2.txt.ots",
            &[
                "https://alice.btc.calendar.opentimestamps.org",
                "https://bob.btc.calendar.opentimestamps.org",
            ],
        ),
        (
            "merkle3.txt",
            "merkle3.txt.ots",
            &[
                "https://alice.btc.calendar.opentimestamps.org",
                "https://bob.btc.calendar.opentimestamps.org",
            ],
        ),
    ];
    for (file, ots, uris) in cases {
        let proof = Proof::parse(&compute(), &example(ots)).unwrap();
        let r = proof.verify(&compute(), &Sha256::digest(example(file)), &h);
        assert!(r.file_matches, "{ots}");
        assert!(!r.committed());
        let got: Vec<&str> = r
            .paths
            .iter()
            .map(|o| match o {
                Outcome::Pending { uri } => uri.as_str(),
                other => panic!("{ots}: {other:?}"),
            })
            .collect();
        assert_eq!(got, uris, "{ots}");
    }
}

#[test]
fn an_unknown_notary_is_listed_by_its_tag() {
    let proof = Proof::parse(&compute(), &example("unknown-notary.txt.ots")).unwrap();
    let r = proof.verify(
        &compute(),
        &Sha256::digest(example("unknown-notary.txt")),
        &header_at(358391),
    );
    assert_eq!(
        r.paths,
        vec![Outcome::Unknown {
            tag: [1, 2, 3, 4, 5, 6, 7, 8]
        }]
    );
    let proof = Proof::parse(&compute(), &example("known-and-unknown-notary.txt.ots")).unwrap();
    let r = proof.verify(
        &compute(),
        &Sha256::digest(example("known-and-unknown-notary.txt")),
        &header_at(358391),
    );
    assert_eq!(r.paths.len(), 2);
    assert!(matches!(r.paths[0], Outcome::Pending { .. }));
    assert_eq!(
        r.paths[1],
        Outcome::Unknown {
            tag: [1, 2, 3, 4, 5, 6, 7, 8]
        }
    );
}

#[test]
fn the_references_invalid_examples_are_refused() {
    let read = |n: &str| fs::read(vectors().join("examples/invalid").join(n)).unwrap();
    assert_eq!(
        Proof::parse(&compute(), &read("bad-major-version.ots")).unwrap_err(),
        Error::NotVersion1(0xff)
    );
    assert_eq!(
        Proof::parse(&compute(), &read("exceeds-max-msg-length.ots")).unwrap_err(),
        Error::ResultTooLong
    );
    assert_eq!(
        Proof::parse(&compute(), &read("invalid-file-digest-type.ots")).unwrap_err(),
        Error::BadFileHashOp
    );
}

#[test]
fn every_shortened_example_is_refused() {
    for name in [
        "hello-world.txt.ots",
        "different-blockchains.txt.ots",
        "empty.ots",
    ] {
        let bytes = example(name);
        for n in 0..bytes.len() {
            assert!(
                Proof::parse(&compute(), &bytes[..n]).is_err(),
                "{name} cut to {n}"
            );
        }
    }
}

#[test]
fn a_changed_magic_or_trailing_byte_is_refused() {
    let mut b = example("hello-world.txt.ots");
    b.push(0);
    assert_eq!(
        Proof::parse(&compute(), &b).unwrap_err(),
        Error::TrailingBytes
    );
    let mut b = example("hello-world.txt.ots");
    b[3] ^= 1;
    assert_eq!(Proof::parse(&compute(), &b).unwrap_err(), Error::BadMagic);
    assert_eq!(Proof::parse(&compute(), b"").unwrap_err(), Error::Truncated);
    assert_eq!(Proof::parse(&compute(), b"x").unwrap_err(), Error::BadMagic);
}

// Proofs built here.

const MAGIC: &[u8] = b"\x00OpenTimestamps\x00\x00Proof\x00\xbf\x89\xe2\xe8\x84\xe8\x92\x94";

fn varuint(mut n: u64) -> Vec<u8> {
    let mut v = Vec::new();
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            v.push(b);
            return v;
        }
        v.push(b | 0x80);
    }
}

fn varbytes(b: &[u8]) -> Vec<u8> {
    [varuint(b.len() as u64), b.to_vec()].concat()
}

fn proof_of(file_op: u8, digest: &[u8], tree: &[u8]) -> Vec<u8> {
    [MAGIC, &[1], &[file_op], digest, tree].concat()
}

fn attestation(tag: [u8; 8], payload: &[u8]) -> Vec<u8> {
    [&[0x00][..], &tag, &varbytes(payload)].concat()
}

const BTC: [u8; 8] = [0x05, 0x88, 0x96, 0x0d, 0x73, 0xd7, 0x19, 0x01];
const PENDING: [u8; 8] = [0x83, 0xdf, 0xe3, 0x0d, 0x2e, 0xf9, 0x0c, 0x8e];

fn parse(tree: &[u8]) -> Result<Proof, Error> {
    Proof::parse(&compute(), &proof_of(0x08, &[7; 32], tree))
}

#[test]
fn what_a_proof_may_not_hold_is_refused() {
    let btc = attestation(BTC, &varuint(1));
    // An unknown operation.
    assert_eq!(parse(&[0x09]).unwrap_err(), Error::UnknownOp);
    assert_eq!(parse(&[0xf4]).unwrap_err(), Error::UnknownOp);
    // A tag 0xff followed by another 0xff.
    assert_eq!(parse(&[0xff, 0xff]).unwrap_err(), Error::UnknownOp);
    // An append of nothing, and one over 4,096 bytes.
    assert_eq!(parse(&[0xf0, 0x00]).unwrap_err(), Error::EmptyArgument);
    let long = [vec![0xf0], varbytes(&[1; 4097])].concat();
    assert_eq!(parse(&long).unwrap_err(), Error::ArgumentTooLong);
    // A result over 4,096 bytes from arguments each within the limit.
    let big = [
        vec![0xf0],
        varbytes(&[1; 4096]),
        vec![0xf1],
        varbytes(&[2; 1]),
        btc.clone(),
    ]
    .concat();
    assert_eq!(parse(&big).unwrap_err(), Error::ResultTooLong);
    // A height over 32 bits.
    assert_eq!(
        parse(&attestation(BTC, &varuint(1 << 32))).unwrap_err(),
        Error::HeightTooLarge
    );
    // A payload over 8,192 bytes, and one with bytes after its height.
    assert_eq!(
        parse(&attestation([9; 8], &[0; 8193])).unwrap_err(),
        Error::PayloadTooLong
    );
    assert_eq!(
        parse(&attestation(BTC, &[1, 0])).unwrap_err(),
        Error::PayloadTrailingBytes
    );
    // A calendar address with a space, and one over 1,000 bytes.
    assert_eq!(
        parse(&attestation(PENDING, &varbytes(b"http://a b"))).unwrap_err(),
        Error::UriInvalid
    );
    assert_eq!(
        parse(&attestation(PENDING, &varbytes(&[b'a'; 1001]))).unwrap_err(),
        Error::UriTooLong
    );
    // Version 2, and a hash that is not a file hash.
    let v2 = [MAGIC, &[2][..], &[0x08], &[0; 32], &btc].concat();
    assert_eq!(
        Proof::parse(&compute(), &v2).unwrap_err(),
        Error::NotVersion1(2)
    );
    assert_eq!(
        Proof::parse(&compute(), &proof_of(0xf2, &[0; 32], &btc)).unwrap_err(),
        Error::BadFileHashOp
    );
}

#[test]
fn a_tree_is_read_to_its_depth_limit_and_no_deeper() {
    let btc = attestation(BTC, &varuint(1));
    // The root is one level: 255 operations below it are the most.
    let chain = |n: usize| [vec![0x08; n], btc.clone()].concat();
    assert!(parse(&chain(255)).is_ok());
    assert_eq!(parse(&chain(256)).unwrap_err(), Error::TooDeep);
}

#[test]
fn a_height_of_32_bits_is_read() {
    let p = parse(&attestation(BTC, &varuint(u64::from(u32::MAX)))).unwrap();
    assert_eq!(p.tree().entries.len(), 1);
}

/// A header at the easiest target (regtest's) with the given merkle root,
/// its nonce found by trial.
fn mined(root: &[u8]) -> Header {
    Header::parse(Network::Regtest, &mined_bytes(root)).unwrap()
}

/// The 80 bytes of [`mined`]'s header.
fn mined_bytes(root: &[u8]) -> [u8; 80] {
    let mut raw = [0u8; 80];
    raw[0] = 4;
    raw[36..68].copy_from_slice(root);
    raw[68..72].copy_from_slice(&1_700_000_000u32.to_le_bytes());
    raw[72..76].copy_from_slice(&0x207f_ffffu32.to_le_bytes());
    for nonce in 0u32.. {
        raw[76..80].copy_from_slice(&nonce.to_le_bytes());
        if Header::parse(Network::Regtest, &raw).is_ok() {
            return raw;
        }
    }
    unreachable!()
}

fn sha256(b: &[u8]) -> Vec<u8> {
    Sha256::digest(b).to_vec()
}

#[test]
fn each_operation_is_applied_in_order() {
    let digest = [5u8; 32];
    // append, prepend, hexlify, reverse, sha256.
    let mut v = digest.to_vec();
    v.extend_from_slice(b"ab");
    let mut w = b"cd".to_vec();
    w.extend_from_slice(&v);
    let hx: Vec<u8> = hex(&w).into_bytes();
    let rev: Vec<u8> = hx.iter().rev().copied().collect();
    let root = sha256(&rev);
    let tree = [
        vec![0xf0],
        varbytes(b"ab"),
        vec![0xf1],
        varbytes(b"cd"),
        vec![0xf3, 0xf2, 0x08],
        attestation(BTC, &varuint(7)),
    ]
    .concat();
    let proof = Proof::parse(&compute(), &proof_of(0x08, &digest, &tree)).unwrap();
    let r = proof.verify(&compute(), &digest, &mined(&root));
    assert!(r.committed());
    let r = proof.verify(&compute(), &digest, &mined(&[0; 32]));
    assert_eq!(r.paths, vec![Outcome::BitcoinMismatch { height: 7 }]);
}

#[test]
fn a_proof_whose_file_hash_is_ripemd160_is_counted() {
    let digest = HashOp::Ripemd160.digest(b"a file");
    assert_eq!(digest.len(), 20);
    let root = sha256(&digest);
    let tree = [vec![0x08], attestation(BTC, &varuint(7))].concat();
    let proof = Proof::parse(&compute(), &proof_of(0x03, &digest, &tree)).unwrap();
    assert_eq!(proof.file_op(), HashOp::Ripemd160);
    let r = proof.verify(&compute(), &digest, &mined(&root));
    assert!(r.file_matches);
    assert!(r.committed());
}

#[test]
fn a_sha1_step_before_sha256_steps_is_named_and_not_counted() {
    let digest = [5u8; 32];
    let mut v = HashOp::Sha1.digest(&digest);
    v.extend_from_slice(&[9; 12]);
    let root = sha256(&sha256(&v));
    let tree = [
        vec![0x02, 0xf0],
        varbytes(&[9; 12]),
        vec![0x08, 0x08],
        attestation(BTC, &varuint(7)),
    ]
    .concat();
    let proof = Proof::parse(&compute(), &proof_of(0x08, &digest, &tree)).unwrap();
    let r = proof.verify(&compute(), &digest, &mined(&root));
    assert_eq!(
        r.paths,
        vec![Outcome::BitcoinNotCounted {
            height: 7,
            op: HashOp::Sha1,
            reaches_header: true
        }]
    );
    assert!(!r.committed());
}

#[test]
fn the_first_uncounted_hash_on_a_path_is_the_one_named() {
    let digest = [5u8; 32];
    let root = HashOp::Keccak256.digest(
        &HashOp::Sha1
            .digest(&digest)
            .iter()
            .chain(&[9u8; 12])
            .copied()
            .collect::<Vec<u8>>(),
    );
    let tree = [
        vec![0x02, 0xf0],
        varbytes(&[9; 12]),
        vec![0x67],
        attestation(BTC, &varuint(7)),
    ]
    .concat();
    let proof = Proof::parse(&compute(), &proof_of(0x08, &digest, &tree)).unwrap();
    let r = proof.verify(&compute(), &digest, &mined(&root));
    assert_eq!(
        r.paths,
        vec![Outcome::BitcoinNotCounted {
            height: 7,
            op: HashOp::Sha1,
            reaches_header: true
        }]
    );
}

#[test]
fn a_path_through_keccak_or_sha1_is_named_and_not_counted() {
    let digest = [5u8; 32];
    for (tag, op) in [(0x67u8, HashOp::Keccak256), (0x02, HashOp::Sha1)] {
        let mut root = op.digest(&digest);
        // Pad to 32 bytes so the value could equal a merkle root.
        let tree = if root.len() == 20 {
            root.extend_from_slice(&[9; 12]);
            [
                vec![tag, 0xf0],
                varbytes(&[9; 12]),
                attestation(BTC, &varuint(7)),
            ]
            .concat()
        } else {
            [vec![tag], attestation(BTC, &varuint(7))].concat()
        };
        let proof = Proof::parse(&compute(), &proof_of(0x08, &digest, &tree)).unwrap();
        let r = proof.verify(&compute(), &digest, &mined(&root));
        assert_eq!(
            r.paths,
            vec![Outcome::BitcoinNotCounted {
                height: 7,
                op,
                reaches_header: true
            }]
        );
        assert!(!r.committed());
    }
}

#[test]
fn a_header_is_80_bytes_and_meets_its_own_target() {
    let raw = unhex(
        fs::read_to_string(vectors().join("headers/358391.hex"))
            .unwrap()
            .trim(),
    );
    assert_eq!(
        Header::parse(Network::Mainnet, &raw[..79]).unwrap_err(),
        Error::HeaderLength
    );
    assert_eq!(
        Header::parse(Network::Mainnet, &[raw.clone(), vec![0]].concat()).unwrap_err(),
        Error::HeaderLength
    );
    assert_eq!(
        Header::parse(Network::Mainnet, &[]).unwrap_err(),
        Error::HeaderLength
    );
    // A changed nonce no longer meets the real network's target.
    let mut bad = raw.clone();
    bad[76] ^= 1;
    assert_eq!(
        Header::parse(Network::Mainnet, &bad).unwrap_err(),
        Error::HeaderWork
    );
    // A zero, a negative and an oversized target.
    for bits in [0u32, 0x0480_0001, 0xff12_3456, 0x2200_0100] {
        let mut b = raw.clone();
        b[72..76].copy_from_slice(&bits.to_le_bytes());
        assert_eq!(
            Header::parse(Network::Mainnet, &b).unwrap_err(),
            Error::HeaderTarget,
            "{bits:#x}"
        );
    }
}

#[test]
fn keccak256_of_nothing_is_the_published_value() {
    // The empty string's Keccak-256 (the original padding, not SHA3-256).
    assert_eq!(
        hex(&HashOp::Keccak256.digest(b"")),
        "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
    );
}

#[test]
fn a_proof_over_64_kib_is_refused() {
    // A proof that reads, with zero bytes after its tree to 64 KiB, is
    // refused for them; one byte more is refused for its size.
    let mut b = proof_of(0x08, &[7; 32], &attestation(BTC, &varuint(1)));
    b.resize(65536, 0);
    assert_eq!(
        Proof::parse(&compute(), &b).unwrap_err(),
        Error::TrailingBytes
    );
    b.push(0);
    assert_eq!(
        Proof::parse(&compute(), &b).unwrap_err(),
        Error::ProofTooLarge
    );
}

#[test]
fn a_tree_whose_operations_make_over_16_mib_is_refused() {
    // A prepend that makes a value of 4,095 bytes, then distinct one-byte
    // appends at that node, each making 4,096 bytes and followed by 250
    // reversals of it, each at a node of its own.
    let end = attestation([9; 8], &[]);
    let build = |n: usize| {
        let mut tree = vec![0xf1];
        tree.extend(varbytes(&[1; 4063]));
        for i in 0..n {
            if i + 1 < n {
                tree.push(0xff);
            }
            tree.push(0xf0);
            tree.extend(varbytes(&[i as u8]));
            tree.extend(vec![0xf2; 250]);
            tree.extend(&end);
        }
        proof_of(0x08, &[7; 32], &tree)
    };
    // 16 of them make 15.7 MiB, 17 make 16.7.
    assert!(Proof::parse(&compute(), &build(16)).is_ok());
    let big = build(17);
    assert!(big.len() < 65536);
    assert_eq!(
        Proof::parse(&compute(), &big).unwrap_err(),
        Error::TooMuchWork
    );
}

#[test]
fn a_tree_of_over_16384_entries_is_refused() {
    // Distinct appends, each followed by a chain of reversals and an
    // attestation, side by side: the values are a few bytes.
    let end = attestation([9; 8], &[]);
    let build = |entries: usize| {
        let mut tree = Vec::new();
        let mut left = entries;
        let mut i = 0u16;
        while left > 0 {
            let chain = (left - 2).min(252);
            left -= chain + 2;
            if left > 0 {
                tree.push(0xff);
            }
            tree.push(0xf0);
            tree.extend(varbytes(&i.to_be_bytes()));
            tree.extend(vec![0xf2; chain]);
            tree.extend(&end);
            i += 1;
        }
        proof_of(0x08, &[7; 32], &tree)
    };
    assert!(Proof::parse(&compute(), &build(16384)).is_ok());
    let big = build(16385);
    assert!(big.len() < 65536);
    assert_eq!(
        Proof::parse(&compute(), &big).unwrap_err(),
        Error::TooManyEntries
    );
}

#[test]
fn what_the_reference_refuses_is_refused_here() {
    let btc = attestation(BTC, &varuint(1));
    // A first version byte other than 1, even the start of a longer number.
    let v = |version: &[u8]| [MAGIC, version, &[0x08], &[0; 32], &btc].concat();
    assert_eq!(
        Proof::parse(&compute(), &v(&[0x81, 0x00])).unwrap_err(),
        Error::NotVersion1(0x81)
    );
    assert_eq!(
        Proof::parse(&compute(), &v(&[0x02])).unwrap_err(),
        Error::NotVersion1(2)
    );
    // A number in ten bytes, and in nine.
    let mut ten = vec![0x85];
    ten.extend([0x80; 8]);
    ten.push(0x00);
    assert_eq!(
        parse(&attestation(BTC, &ten)).unwrap_err(),
        Error::VaruintTooLong
    );
    let mut nine = vec![0x85];
    nine.extend([0x80; 7]);
    nine.push(0x00);
    assert!(parse(&attestation(BTC, &nine)).is_ok());
    // The same operation twice at one node: an append of the same bytes, a
    // hash, a reversal; two appends of different bytes are read.
    let twice = |op: &[u8]| {
        [
            vec![0xff],
            op.to_vec(),
            btc.clone(),
            op.to_vec(),
            btc.clone(),
        ]
        .concat()
    };
    for op in [&[0x08][..], &[0xf2], &[0xf3], &[0xf0, 1, 5]] {
        assert_eq!(
            parse(&twice(op)).unwrap_err(),
            Error::RepeatedOperation,
            "{op:?}"
        );
    }
    let two = [
        vec![0xff, 0xf0, 1, 5],
        btc.clone(),
        vec![0xf0, 1, 6],
        btc.clone(),
    ]
    .concat();
    assert!(parse(&two).is_ok());
    // A repeated operation is refused as repeated, whatever is below it:
    // here its subtree is cut short.
    let cut = [vec![0xff, 0xf2], btc.clone(), vec![0xf2, 0x00]].concat();
    assert_eq!(parse(&cut).unwrap_err(), Error::RepeatedOperation);
    // Keccak-256 as the file's hash.
    assert_eq!(
        Proof::parse(&compute(), &proof_of(0x67, &[0; 32], &btc)).unwrap_err(),
        Error::FileHashKeccak256
    );
}

#[test]
fn a_header_target_above_its_networks_limit_is_refused() {
    let raw = unhex(
        fs::read_to_string(vectors().join("headers/358391.hex"))
            .unwrap()
            .trim(),
    );
    // The real mainnet header's target is within every network's limit.
    for n in [
        Network::Mainnet,
        Network::Testnet3,
        Network::Testnet4,
        Network::Signet,
        Network::Regtest,
    ] {
        assert!(Header::parse(n, &raw).is_ok(), "{n:?}");
    }
    // A header made at regtest's target meets regtest's limit only.
    let bytes = mined_bytes(&[3; 32]);
    assert!(Header::parse(Network::Regtest, &bytes).is_ok());
    for n in [
        Network::Mainnet,
        Network::Testnet3,
        Network::Testnet4,
        Network::Signet,
    ] {
        assert_eq!(
            Header::parse(n, &bytes).unwrap_err(),
            Error::HeaderTargetTooEasy,
            "{n:?}"
        );
    }
}

#[test]
fn a_whole_file_hash_through_compute_is_the_same_as_the_plain_one() {
    let data = example("bad-stamp.txt");
    for op in [
        faraday_ots::HashOp::Sha1,
        faraday_ots::HashOp::Ripemd160,
        faraday_ots::HashOp::Sha256,
        faraday_ots::HashOp::Keccak256,
    ] {
        assert_eq!(op.digest_whole(&compute(), &data), op.digest(&data));
    }
}
