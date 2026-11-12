//! The 32-byte size as text (`https://github.com/bitcoinpasada/aqrv`, `AQRV.md` §3, "The 32-byte size as
//! text", and §9): the envelope is one number, written in base 45 as
//! exactly 47 characters of QR's alphanumeric table.

/// QR's alphanumeric table; a character's index is its digit.
const TABLE: &[u8; 45] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// The text's length: 45^47 is more than 256^32 and 45^46 is less.
const TEXT_LEN: usize = 47;

/// Why a text is not a 32-byte envelope (§9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotText {
    /// Not 47 characters.
    Length,
    /// A character outside the table, a lower-case letter among them.
    Character,
    /// The first character is not `A`, so the value is not in the range
    /// of an envelope's first byte (§3).
    First,
    /// The value is 256^32 or more, so it is not 32 bytes.
    OutOfRange,
}

/// §3: the 32 bytes as a number, big-endian, written in base 45 as 47
/// characters, most significant first. Nothing is trimmed or padded: an
/// envelope always takes all 47.
pub fn to_text(envelope: &[u8; 32]) -> String {
    let mut number = *envelope;
    let mut text = [0u8; TEXT_LEN];
    for ch in text.iter_mut().rev() {
        // One long division by 45; what is left over is the digit.
        let mut rest = 0u32;
        for byte in number.iter_mut() {
            let part = rest * 256 + u32::from(*byte);
            *byte = (part / 45) as u8;
            rest = part % 45;
        }
        *ch = TABLE[rest as usize];
    }
    String::from_utf8(text.to_vec()).expect("the table is ASCII")
}

/// §3 and §9: the 32 bytes a text stands for, or why it is not one. The
/// text is taken exactly as given: nothing is trimmed and a lower-case
/// letter is not read as its capital. The header byte is not looked at
/// here: [`recognise`](crate::recognise) does that on the bytes.
pub fn from_text(text: &str) -> Result<[u8; 32], NotText> {
    if text.chars().count() != TEXT_LEN {
        return Err(NotText::Length);
    }
    let mut digits = [0u8; TEXT_LEN];
    for (digit, ch) in digits.iter_mut().zip(text.bytes()) {
        let index = TABLE.iter().position(|&t| t == ch);
        *digit = index.ok_or(NotText::Character)? as u8;
    }
    if digits[0] != 10 {
        return Err(NotText::First);
    }
    let mut number = [0u8; 32];
    for digit in digits {
        // number = number * 45 + digit; a carry out of the top byte
        // means 256^32 or more, and the value only grows from there.
        let mut carry = u32::from(digit);
        for byte in number.iter_mut().rev() {
            let part = u32::from(*byte) * 45 + carry;
            *byte = part as u8;
            carry = part >> 8;
        }
        if carry != 0 {
            return Err(NotText::OutOfRange);
        }
    }
    Ok(number)
}
