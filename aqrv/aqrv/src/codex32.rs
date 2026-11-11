//! A codex32 string as an item (`docs/AQRV.md` §3.1): a 4-bit head, then
//! every character after `ms1` and before the checksum as its 5-bit
//! value, most significant bit first. The checksum is not stored; a
//! reader makes it with faraday-bip's, so no third BIP 93 checksum exists.

use faraday_bip::codex32::{CHARSET, Codex32, Error, charval, checksum, symbol_char};
use zeroize::Zeroizing;

use crate::{Layout, Refused, Size};

/// §3.1's five forms, by the 3-bit `size` field: the payload's characters
/// `c` and the item's bytes.
const FORMS: [(usize, usize); 5] = [(26, 21), (32, 25), (39, 29), (45, 33), (52, 37)];

/// The characters of `ms1` and of the checksum, which are not stored.
const PREFIX: usize = 3;
const CHECKSUM: usize = 13;

/// The value of the character `0`, the only threshold with the index `s`.
const ZERO: u8 = 15;
/// The value of the share index `s`.
const INDEX_S: u8 = 16;

/// §3.1 Writing: packs `string`, or refuses it. Gives the item's bytes
/// with the unused low bits zero, and the mask of those bits.
pub(crate) fn pack(string: &str, size: Size) -> Result<(Zeroizing<Vec<u8>>, u8), Refused> {
    // BIP 93's own rules: prefix, one case, checksum, threshold.
    Codex32::parse(string).map_err(Refused::Codex32)?;
    let chars = string.as_bytes();
    let field = FORMS
        .iter()
        .position(|&(c, _)| PREFIX + 6 + c + CHECKSUM == chars.len())
        .ok_or(Refused::Codex32(Error::BadLength))?;
    let (c, item_len) = FORMS[field];
    if item_len > size.room() {
        return Err(Refused::Codex32TooLongForSmall);
    }

    let mut bytes = Zeroizing::new(vec![0u8; item_len]);
    put_bits(&mut bytes, 0, field as u8, 3);
    put_bits(&mut bytes, 3, u8::from(chars[0].is_ascii_uppercase()), 1);
    for (i, &ch) in chars[PREFIX..PREFIX + 6 + c].iter().enumerate() {
        let position = (PREFIX + i) as u8;
        let value = charval(ch).ok_or(Refused::Codex32(Error::BadCharacter { position }))?;
        put_bits(&mut bytes, 4 + 5 * i, value, 5);
    }
    let unused = item_len * 8 - (4 + 5 * (6 + c));
    Ok((bytes, (1u8 << unused) - 1))
}

/// §3.1 Reading: the string an item stands for, or the rule it fails.
/// `p` is the whole room.
pub(crate) fn unpack(p: &[u8]) -> Result<Zeroizing<String>, Layout> {
    let field = usize::from(get_bits(p, 0, 3));
    let upper = get_bits(p, 3, 1) == 1;
    let &(c, item_len) = FORMS.get(field).ok_or(Layout::Codex32Size)?;
    if item_len > p.len() {
        return Err(Layout::Codex32TooLong);
    }

    // The threshold, the identifier, the index and the payload.
    let mut symbols = Zeroizing::new(Vec::with_capacity(6 + c + CHECKSUM));
    for i in 0..6 + c {
        symbols.push(get_bits(p, 4 + 5 * i, 5));
    }
    let threshold = CHARSET[usize::from(symbols[0])];
    if threshold != b'0' && !(b'2'..=b'9').contains(&threshold) {
        return Err(Layout::Codex32Threshold);
    }
    if symbols[0] == ZERO && symbols[5] != INDEX_S {
        return Err(Layout::Codex32ThresholdZeroNotS);
    }

    // The checksum is made here, so it always passes (§3.1, §7).
    let sum = checksum(&symbols);
    symbols.extend_from_slice(&sum);
    let mut string = Zeroizing::new(String::with_capacity(PREFIX + symbols.len()));
    string.push_str("ms1");
    string.extend(symbols.iter().map(|&v| char::from(symbol_char(v))));
    if upper {
        string.make_ascii_uppercase();
    }
    Ok(string)
}

/// Sets `n` bits (at most 8) of `buf` from bit position `pos`, counted
/// from the most significant bit of the first byte. The bits are zero.
fn put_bits(buf: &mut [u8], pos: usize, value: u8, n: usize) {
    for i in 0..n {
        let bit = (value >> (n - 1 - i)) & 1;
        let at = pos + i;
        buf[at / 8] |= bit << (7 - at % 8);
    }
}

/// The `n` bits (at most 8) of `buf` from bit position `pos`.
fn get_bits(buf: &[u8], pos: usize, n: usize) -> u8 {
    let mut value = 0;
    for at in pos..pos + n {
        value = (value << 1) | ((buf[at / 8] >> (7 - at % 8)) & 1);
    }
    value
}
