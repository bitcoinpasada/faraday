//! A CBOR message that declares a long array or map costs what it
//! holds, not what it says: nothing much is reserved for a count before
//! its items are read, and one message builds a bounded number of items.
//! An animated code that mixes many fragments is held in bounded memory.

use faraday_codec::cbor::Value;

#[global_allocator]
static ALLOC: faraday_alloc_count::Counting = faraday_alloc_count::Counting;

fn largest_allocation_of(bytes: &[u8]) -> (bool, usize) {
    faraday_alloc_count::reset_largest_allocation();
    let ok = Value::decode(bytes).is_ok();
    (ok, faraday_alloc_count::largest_allocation())
}

#[test]
fn a_map_that_declares_many_pairs_reserves_little_before_it_fails() {
    // 30000 pairs declared and the bytes for them present, but the first
    // pair's value is not an item.
    let mut cbor = vec![0xb9, 0x75, 0x30, 0xf6, 0xff];
    cbor.resize(3 + 60_000, 0xf6);
    let (ok, largest) = largest_allocation_of(&cbor);
    assert!(!ok);
    assert!(largest <= 16 * 1024, "one allocation of {largest} bytes");
}

#[test]
fn an_array_that_declares_many_items_reserves_little_before_it_fails() {
    let mut cbor = vec![0x99, 0x75, 0x30, 0xff];
    cbor.resize(3 + 30_000, 0xf6);
    let (ok, largest) = largest_allocation_of(&cbor);
    assert!(!ok);
    assert!(largest <= 16 * 1024, "one allocation of {largest} bytes");
}

#[test]
fn an_array_of_more_items_than_one_message_may_hold_is_refused() {
    let mut cbor = vec![0x99, 0xff, 0xff];
    cbor.resize(3 + 65_535, 0xf6);
    assert!(Value::decode(&cbor).is_err());
}

#[test]
fn nested_arrays_that_are_each_small_but_hold_too_many_items_together_are_refused() {
    // 255 arrays of 255 nulls: 65,281 items, every array far below the
    // limit of one message.
    let mut cbor = vec![0x98, 0xff];
    for _ in 0..255 {
        cbor.extend_from_slice(&[0x98, 0xff]);
        cbor.resize(cbor.len() + 255, 0xf6);
    }
    assert!(Value::decode(&cbor).is_err());
}

#[test]
fn an_animated_code_of_2048_fragments_whose_plain_parts_are_all_missing_is_held_in_bounded_memory()
{
    let message = vec![0x5a; 2048 * 16];
    let mut encoder = faraday_codec::ur::Encoder::bytes(&message, 16).unwrap();
    let mut decoder = faraday_codec::ur::Decoder::new();
    let baseline = faraday_alloc_count::live_bytes();
    let mut peak = 0;
    for _ in 0..(4 * 2048 + 200) {
        let part = encoder.next_part();
        if encoder.emitted() <= encoder.fragment_count() {
            continue;
        }
        let ended = decoder.receive(part.as_str());
        peak = peak.max(faraday_alloc_count::live_bytes() - baseline);
        if matches!(ended, Ok(true) | Err(_)) {
            break;
        }
    }
    // The 4n+64 held parts of 256 bytes of index and 16 of data each,
    // with the maps around them, are a few megabytes; the encoder's own
    // fragments are in the baseline.
    assert!(peak < 16 << 20, "the decoder held {peak} bytes");
}

#[test]
fn an_honest_array_of_hundreds_of_items_decodes() {
    let mut cbor = vec![0x99, 0x01, 0x00];
    cbor.resize(3 + 256, 0x01);
    assert!(Value::decode(&cbor).is_ok());
}

#[test]
fn a_map_keeps_its_keys_unique_and_its_depth_limit() {
    assert!(Value::decode(&[0xa2, 0x01, 0x02, 0x01, 0x03]).is_err());
    let mut deep = vec![0x81; 17];
    deep.push(0xf6);
    assert!(Value::decode(&deep).is_err());
    let mut ok = vec![0x81; 16];
    ok.push(0xf6);
    assert!(Value::decode(&ok).is_ok());
}
