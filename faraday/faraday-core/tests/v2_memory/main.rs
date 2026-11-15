//! A version 2 PSBT that declares more inputs than it holds asks for no
//! memory beyond what its bytes need.

use faraday_crypto::work::{Compute, Work};
use faraday_psbt::v2::PsbtV2;

#[global_allocator]
static ALLOC: faraday_alloc_count::Counting = faraday_alloc_count::Counting;

fn entry(out: &mut Vec<u8>, ty: u8, value: &[u8]) {
    out.extend([1, ty, value.len() as u8]);
    out.extend(value);
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: this test binary cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_: Work| {})
}

#[test]
fn a_huge_declared_input_count_reserves_nothing_before_a_map_is_read() {
    let left = 1_000_000usize;
    let mut b = b"psbt\xff".to_vec();
    entry(&mut b, 0xfb, &2u32.to_le_bytes());
    entry(&mut b, 0x02, &2u32.to_le_bytes());
    // A compact size of one million, backed by that many empty maps.
    entry(&mut b, 0x04, &[0xfe, 0x40, 0x42, 0x0f, 0x00]);
    entry(&mut b, 0x05, &[0]);
    b.push(0);
    b.extend(vec![0u8; left]);
    let c = compute();
    faraday_alloc_count::reset_largest_allocation();
    assert!(PsbtV2::parse_bytes(&c, &b).is_err());
    let largest = faraday_alloc_count::largest_allocation();
    assert!(largest < 64 * 1024, "one allocation of {largest} bytes");
}
