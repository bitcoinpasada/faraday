//! `osk_codec::ur::encode_single`, the public writer of a single-part UR
//! of any type: BCR-2020-005's own seed example, and what it writes is
//! read back by the crate's decoder with its type and bytes intact.

use osk_codec::ur::{Decoder, Message, encode_single};

/// BCR-2020-005 §"CBOR": the seed example, untagged CBOR
/// `a10150c7098580125e2ab0981253468b2dbc52` as `ur:seed`.
const SEED_CBOR: [u8; 19] = [
    0xa1, 0x01, 0x50, 0xc7, 0x09, 0x85, 0x80, 0x12, 0x5e, 0x2a, 0xb0, 0x98, 0x12, 0x53, 0x46, 0x8b,
    0x2d, 0xbc, 0x52,
];

#[test]
fn a_single_part_ur_of_any_type_is_the_published_one_and_reads_back() {
    let text = encode_single("seed", &SEED_CBOR);
    assert_eq!(
        text,
        "ur:seed/oyadgdstaslplabghydrpfmkbggufgludprfgmamdpwmox"
    );
    for (ur_type, cbor) in [("seed", &SEED_CBOR[..]), ("crypto-account", &SEED_CBOR[..])] {
        let mut d = Decoder::new();
        assert_eq!(d.receive(&encode_single(ur_type, cbor)), Ok(true));
        assert_eq!(
            d.message(),
            Some(&Message::Other {
                ur_type: ur_type.to_string(),
                cbor: cbor.to_vec(),
            })
        );
    }
}
