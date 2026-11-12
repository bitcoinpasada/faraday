//! gzip (RFC 1952), read only: the body of a KDBX file a KeePass app
//! saved with compression. One member, its CRC-32 and length checked,
//! inflated into a buffer sized from the length it states. That length is
//! only trusted once the file's HMACs have verified (F206 point 5); it is
//! bounded by the ceiling and asked of the caller's budget before anything
//! is allocated, and inflating cannot write past the buffer.

use alloc::vec::Vec;

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::{DecompressorOxide, decompress, inflate_flags};
use zeroize::Zeroizing;

use super::Error;

const FHCRC: u8 = 2;
const FEXTRA: u8 = 4;
const FNAME: u8 = 8;
const FCOMMENT: u8 = 16;

/// The inflated bytes of the one gzip member `data` is, at most `limit`.
/// `budget` is asked with the length the member states, before the one
/// buffer of that length is allocated: `Err(mib)` is the memory that could
/// not be found, and nothing is allocated.
pub(super) fn inflate(
    data: &[u8],
    limit: usize,
    budget: &mut dyn FnMut(usize) -> Result<(), u32>,
) -> Result<Zeroizing<Vec<u8>>, Error> {
    if data.len() < 18 || data[0] != 0x1F || data[1] != 0x8B || data[2] != 8 {
        return Err(Error::Damaged);
    }
    let flags = data[3];
    if flags & 0xE0 != 0 {
        return Err(Error::Damaged);
    }
    let mut at = 10usize;
    if flags & FEXTRA != 0 {
        let len = u16::from_le_bytes([
            *data.get(at).ok_or(Error::Damaged)?,
            *data.get(at + 1).ok_or(Error::Damaged)?,
        ]) as usize;
        at += 2 + len;
    }
    for flag in [FNAME, FCOMMENT] {
        if flags & flag != 0 {
            let zero = data
                .get(at..)
                .and_then(|rest| rest.iter().position(|&b| b == 0))
                .ok_or(Error::Damaged)?;
            at += zero + 1;
        }
    }
    if flags & FHCRC != 0 {
        at += 2;
    }
    let trailer = data.len().checked_sub(8).ok_or(Error::Damaged)?;
    if at > trailer {
        return Err(Error::Damaged);
    }
    let crc = u32::from_le_bytes(
        data[trailer..trailer + 4]
            .try_into()
            .map_err(|_| Error::Damaged)?,
    );
    let size =
        u32::from_le_bytes(data[trailer + 4..].try_into().map_err(|_| Error::Damaged)?) as usize;
    if size > limit {
        return Err(Error::TooLarge);
    }
    budget(size).map_err(Error::Memory)?;
    let mut out = Zeroizing::new(Vec::new());
    out.try_reserve_exact(size)
        .map_err(|_| super::no_memory(size))?;
    out.resize(size, 0);
    let mut state = DecompressorOxide::new();
    let (status, used, written) = decompress(
        &mut state,
        &data[at..trailer],
        &mut out,
        0,
        inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    );
    if status != TINFLStatus::Done || used != trailer - at || written != size {
        return Err(Error::Damaged);
    }
    if crc32(&out) != crc {
        return Err(Error::Damaged);
    }
    Ok(out)
}

/// The CRC-32 of RFC 1952 (the reflected polynomial 0xEDB88320).
fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut n = 0;
        while n < 256 {
            let mut c = n as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[n] = c;
            n += 1;
        }
        table
    };
    let mut crc = !0u32;
    for &b in data {
        crc = TABLE[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}
