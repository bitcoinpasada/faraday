//! Every PSBT of BIP-174's, BIP-370's and BIP-371's Test Vectors sections
//! (`tools/vectors/psbt-bips/vectors.json`) read as the BIPs say: each
//! invalid one refused by the version 0 and the version 2 reader, each
//! valid one read by its version and written back byte for byte, and
//! BIP-370's locktimes. Then what the BIPs refuse that no vector shows:
//! a taproot signature whose sighash byte is 0 (BIP-341), bytes after
//! the last map; and a read PSBT's untouched fields written back as they
//! were read after another field changed.

use std::path::PathBuf;

use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;
use faraday_psbt::v2::PsbtV2;

use crate::common::unhex;

struct Case {
    bip: u64,
    name: String,
    expect: String,
    bytes: Vec<u8>,
}

fn cases() -> Vec<Case> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/psbt-bips/vectors.json");
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the vectors")).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|c| Case {
            bip: c["bip"].as_u64().unwrap(),
            name: c["case"].as_str().unwrap_or("(unnamed)").to_string(),
            expect: c["expect"].as_str().unwrap().to_string(),
            bytes: unhex(c["hex"].as_str().unwrap()),
        })
        .collect()
}

fn case(start: &str) -> Vec<u8> {
    cases()
        .into_iter()
        .find(|c| c.name.starts_with(start))
        .unwrap_or_else(|| panic!("no vector {start}"))
        .bytes
}

/// A PSBT's maps as (key, value) pairs, read by hand: the global map,
/// then `n` more.
type Maps = Vec<Vec<(Vec<u8>, Vec<u8>)>>;

fn compact(b: &[u8], at: &mut usize) -> usize {
    let first = b[*at];
    *at += 1;
    let size = match first {
        0xfd => 2,
        0xfe => 4,
        0xff => 8,
        n => return usize::from(n),
    };
    let mut v = [0u8; 8];
    v[..size].copy_from_slice(&b[*at..*at + size]);
    *at += size;
    usize::try_from(u64::from_le_bytes(v)).unwrap()
}

fn maps(b: &[u8], n: usize) -> Maps {
    let mut at = 5;
    let mut out = Vec::new();
    for _ in 0..=n {
        let mut map = Vec::new();
        loop {
            let k = compact(b, &mut at);
            if k == 0 {
                break;
            }
            let key = b[at..at + k].to_vec();
            at += k;
            let v = compact(b, &mut at);
            map.push((key, b[at..at + v].to_vec()));
            at += v;
        }
        out.push(map);
    }
    assert_eq!(at, b.len(), "the hand reader read every map");
    out
}

fn write(maps: &Maps) -> Vec<u8> {
    let mut out = b"psbt\xff".to_vec();
    for map in maps {
        for (k, v) in map {
            for part in [k, v] {
                match part.len() {
                    n @ 0..0xfd => out.push(n as u8),
                    n => {
                        out.push(0xfd);
                        out.extend_from_slice(&u16::try_from(n).unwrap().to_le_bytes());
                    }
                }
                out.extend_from_slice(part);
            }
        }
        out.push(0);
    }
    out
}

/// How many maps follow the global map of a version 0 PSBT.
fn n_maps(psbt: &Psbt) -> usize {
    psbt.inner().inputs.len() + psbt.inner().outputs.len()
}

fn says_v2(bytes: &[u8]) -> bool {
    let mut at = 5;
    loop {
        let k = compact(bytes, &mut at);
        if k == 0 {
            return false;
        }
        let key = &bytes[at..at + k];
        at += k;
        let v = compact(bytes, &mut at);
        if key == [0xfb] {
            return bytes[at..at + v] == 2u32.to_le_bytes();
        }
        at += v;
    }
}

#[test]
fn every_bip_vector_is_read_as_its_bip_says() {
    let c = Compute::unchecked();
    let mut seen = [0usize; 4];
    for case in cases() {
        let Case {
            bip,
            name,
            expect,
            bytes,
        } = case;
        match expect.as_str() {
            "invalid" => {
                assert!(
                    Psbt::parse_bytes(&c, &bytes).is_err(),
                    "BIP-{bip}, version 0 reads an invalid PSBT: {name}"
                );
                assert!(
                    PsbtV2::parse_bytes(&c, &bytes).is_err(),
                    "BIP-{bip}, version 2 reads an invalid PSBT: {name}"
                );
                seen[0] += 1;
            }
            "valid" | "signer-fails" if says_v2(&bytes) => {
                let p = PsbtV2::parse_bytes(&c, &bytes)
                    .unwrap_or_else(|e| panic!("BIP-{bip}: {name}: {e}"));
                assert_eq!(p.to_bytes(), bytes, "BIP-{bip}: {name}");
                seen[1] += 1;
            }
            "valid" | "signer-fails" => {
                let p = Psbt::parse_bytes(&c, &bytes)
                    .unwrap_or_else(|e| panic!("BIP-{bip}: {name}: {e}"));
                assert_eq!(p.to_bytes(), bytes, "BIP-{bip}, written back: {name}");
                seen[2] += 1;
            }
            lock => {
                let p = PsbtV2::parse_bytes(&c, &bytes)
                    .unwrap_or_else(|e| panic!("BIP-{bip}: {name}: {e}"));
                let want = lock
                    .strip_prefix("locktime ")
                    .and_then(|n| n.parse::<u32>().ok());
                assert_eq!(p.locktime().ok(), want, "BIP-{bip}: {name}");
                seen[3] += 1;
            }
        }
    }
    // 51 invalid; 13 valid version 2; 20 valid version 0; 10 locktimes.
    assert_eq!(seen, [51, 13, 20, 10]);
}

#[test]
fn bip370s_version_0_with_version_2_fields_are_refused() {
    let c = Compute::unchecked();
    let v0_with: Vec<Case> = cases()
        .into_iter()
        .filter(|c| {
            c.name.starts_with("PSBTv0 but with PSBT_") && !c.name.contains("PSBT_GLOBAL_VERSION")
        })
        .collect();
    assert_eq!(v0_with.len(), 12);
    for case in v0_with {
        assert!(
            Psbt::parse_bytes(&c, &case.bytes).is_err(),
            "read: {}",
            case.name
        );
    }
}

#[test]
fn bip370s_invalid_locktimes_and_a_missing_script_are_refused() {
    let c = Compute::unchecked();
    for name in [
        "PSBTv2 missing PSBT_OUT_SCRIPT",
        "PSBTv2 with PSBT_IN_REQUIRED_TIME_LOCKTIME less than 500000000",
        "PSBTv2 with PSBT_IN_REQUIRED_HEIGHT_LOCKTIME greater than or equal to 500000000",
        "PSBTv2 with PSBT_IN_REQUIRED_HEIGHT_LOCKTIME of 0",
    ] {
        assert!(
            PsbtV2::parse_bytes(&c, &case(name)).is_err(),
            "read: {name}"
        );
    }
}

/// BIP-370's "required fields only" PSBT with one pair added to a map:
/// map 0 is the global map, 1 its input, 2 and 3 its outputs.
fn v2_with(map: usize, key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut m = maps(&case("1 input, 2 output PSBTv2, required fields only"), 3);
    m[map].push((key.to_vec(), value.to_vec()));
    write(&m)
}

#[test]
fn version_2_checks_its_locktime_fields_sizes_on_reading() {
    let c = Compute::unchecked();
    let base = case("1 input, 2 output PSBTv2, required fields only");
    assert!(PsbtV2::parse_bytes(&c, &base).is_ok());
    for (map, key, value, ok) in [
        (0, 0x03, &[0u8; 4][..], true),
        (0, 0x03, &[0u8; 3][..], false),
        (0, 0x03, &[0u8; 5][..], false),
        (0, 0x06, &[0u8; 1][..], true),
        (0, 0x06, &[0u8; 2][..], false),
        (0, 0x06, &[][..], false),
        (1, 0x11, &600_000_000u32.to_le_bytes()[..], true),
        (1, 0x11, &[0x00, 0x65, 0xcd, 0x1d, 0x00][..], false),
        (1, 0x12, &10_000u32.to_le_bytes()[..], true),
        (1, 0x12, &[0x10, 0x27, 0x00][..], false),
        (1, 0x12, &499_999_999u32.to_le_bytes()[..], true),
        (1, 0x12, &500_000_000u32.to_le_bytes()[..], false),
        (1, 0x11, &499_999_999u32.to_le_bytes()[..], false),
        (1, 0x11, &500_000_000u32.to_le_bytes()[..], true),
    ] {
        let got = PsbtV2::parse_bytes(&c, &v2_with(map, &[key], value)).is_ok();
        assert_eq!(got, ok, "map {map}, field {key:#04x}, value {value:02x?}");
    }
}

#[test]
fn a_silent_payment_output_alone_may_leave_its_script_out() {
    let c = Compute::unchecked();
    let mut m = maps(&case("1 input, 2 output PSBTv2, required fields only"), 3);
    m[3].retain(|(k, _)| k != &[0x04]);
    assert!(PsbtV2::parse_bytes(&c, &write(&m)).is_err());
    m[3].push((vec![0x09], vec![0x02; 66]));
    assert!(PsbtV2::parse_bytes(&c, &write(&m)).is_ok());
}

#[test]
fn a_taproot_signature_whose_sighash_byte_is_0_is_refused() {
    let c = Compute::unchecked();
    // BIP-371's key path input with its signature: version 0.
    let signed = case(
        "PSBT with one P2TR key only input with internal key, its derivation path, and signature",
    );
    let psbt = Psbt::parse_bytes(&c, &signed).unwrap();
    let mut m = maps(&signed, n_maps(&psbt));
    let sig = m[1].iter_mut().find(|(k, _)| k == &[0x13]).unwrap();
    assert_eq!(sig.1.len(), 64);
    for (last, ok) in [(0x00, false), (0x01, true), (0x81, true)] {
        let mut changed = m.clone();
        let sig = changed[1].iter_mut().find(|(k, _)| k == &[0x13]).unwrap();
        sig.1.push(last);
        let got = Psbt::parse_bytes(&c, &write(&changed));
        assert_eq!(got.is_ok(), ok, "version 0, sighash byte {last:#04x}");
    }
    // Version 2: a key path and a script path signature.
    let script_key = [&[0x14][..], &[0x02; 32], &[0x03; 32]].concat();
    for key in [vec![0x13], script_key] {
        for (sig, ok) in [
            ([&[0x5a; 64][..], &[0x00]].concat(), false),
            ([&[0x5a; 64][..], &[0x01]].concat(), true),
            (vec![0x5a; 64], true),
        ] {
            let got = PsbtV2::parse_bytes(&c, &v2_with(1, &key, &sig)).is_ok();
            assert_eq!(
                got,
                ok,
                "version 2, key {:02x}, {} bytes",
                key[0],
                sig.len()
            );
        }
    }
}

#[test]
fn bytes_after_the_last_map_are_refused() {
    let c = Compute::unchecked();
    let valid = case("PSBT with one P2TR key only input with internal key and its derivation path");
    assert!(Psbt::parse_bytes(&c, &valid).is_ok());
    for tail in [&[0u8][..], &[0x01, 0x00], &[0xff; 3]] {
        let bytes = [&valid[..], tail].concat();
        assert!(Psbt::parse_bytes(&c, &bytes).is_err(), "tail {tail:02x?}");
    }
    let v2 = case("1 input, 2 output PSBTv2, required fields only");
    assert!(PsbtV2::parse_bytes(&c, &[&v2[..], &[0]].concat()).is_err());
}

#[test]
fn a_global_version_of_0_is_written_back_as_read() {
    let c = Compute::unchecked();
    let valid = case("PSBT with one P2TR key only input with internal key and its derivation path");
    let mut m = maps(&valid, n_maps(&Psbt::parse_bytes(&c, &valid).unwrap()));
    m[0].push((vec![0xfb], 0u32.to_le_bytes().to_vec()));
    let with = write(&m);
    let psbt = Psbt::parse_bytes(&c, &with).unwrap();
    assert_eq!(psbt.to_bytes(), with);
    // `bitcoin` alone leaves it out.
    assert_ne!(psbt.inner().serialize(), with);
}

#[test]
fn untouched_fields_are_written_as_read_after_another_changes() {
    let c = Compute::unchecked();
    // BIP-371's output with a taproot tree, whose leaves `bitcoin`
    // writes back in another depth-first order.
    let tree =
        case("PSBT with one P2TR script path only output with dummy internal key, taproot tree");
    let mut psbt = Psbt::parse_bytes(&c, &tree).unwrap();
    assert_eq!(psbt.to_bytes(), tree);
    assert_ne!(psbt.inner().serialize(), tree);
    let n = n_maps(&psbt);
    let first_output = 1 + psbt.inner().inputs.len();
    let read = maps(&tree, n);
    let tap_tree = |m: &Maps| {
        m[first_output]
            .iter()
            .find(|(k, _)| k == &[0x06])
            .map(|(_, v)| v.clone())
            .expect("a tree")
    };
    // A change elsewhere: the input's sighash type set.
    psbt.inner_mut().inputs[0].sighash_type =
        Some(faraday_psbt::bitcoin::psbt::PsbtSighashType::from_u32(1));
    let written = psbt.to_bytes();
    let now = maps(&written, n);
    assert_eq!(tap_tree(&now), tap_tree(&read), "the tree as read");
    assert_eq!(now[0], read[0], "the global map as read");
    assert!(
        now[1].contains(&(vec![0x03], 1u32.to_le_bytes().to_vec())),
        "the change is written"
    );
    // And it reads back to the same PSBT.
    assert_eq!(Psbt::parse_bytes(&c, &written).unwrap(), psbt);
}
