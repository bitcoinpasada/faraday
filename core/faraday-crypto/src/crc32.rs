//! CRC-32 (the IEEE polynomial, as zlib, PNG, ZIP and GPT compute it).

/// CRC-32/ISO-HDLC of `data`, as zlib computes it. Costs one pass over
/// `data`, a shift and an exclusive-or per bit.
///
/// Reflected, polynomial `0x04C1_1DB7` (`0xEDB8_8320` reversed),
/// initialised and finalised with all ones. Bitwise, so no table. Not
/// a secret's hash: it detects a mis-read, and nothing more.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::crc32;

    #[test]
    fn crc32_matches_the_published_check_values() {
        assert_eq!(crc32(b"Hello, world!"), 0xebe6_c6e6);
        assert_eq!(crc32(b"Wolf"), 0x598c_84dc);
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
    }
}
