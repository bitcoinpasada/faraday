//! Bytewords (BCR-2020-012), minimal style: the alphabet a UR payload is
//! written in.
//!
//! Every byte is one of 256 four-letter words; the minimal style keeps
//! the first and last letter of each, so a byte is two characters and a
//! payload is twice its length. A four-byte CRC-32 of the payload is
//! appended before encoding and checked on the way back, which is what
//! makes a mis-read QR frame a rejection rather than a corrupt PSBT.
//!
//! Only the minimal style is here. The standard and URI styles (words
//! separated by spaces or dashes) are part of BCR-2020-012 but no UR
//! uses them, so they are not written.

use alloc::string::String;
use alloc::vec::Vec;

use super::Fault;

/// CRC-32/ISO-HDLC of `data`: the check value BCR-2020-012 appends.
///
/// Reflected, polynomial `0x04C1_1DB7` (`0xEDB8_8320` reversed),
/// initialised and finalised with all ones. Bitwise, so no table.
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

/// The 256 bytewords, in byte order (BCR-2020-012).
#[rustfmt::skip]
const WORDS: [&str; 256] = [
    "able", "acid", "also", "apex", "aqua", "arch", "atom", "aunt",
    "away", "axis", "back", "bald", "barn", "belt", "beta", "bias",
    "blue", "body", "brag", "brew", "bulb", "buzz", "calm", "cash",
    "cats", "chef", "city", "claw", "code", "cola", "cook", "cost",
    "crux", "curl", "cusp", "cyan", "dark", "data", "days", "deli",
    "dice", "diet", "door", "down", "draw", "drop", "drum", "dull",
    "duty", "each", "easy", "echo", "edge", "epic", "even", "exam",
    "exit", "eyes", "fact", "fair", "fern", "figs", "film", "fish",
    "fizz", "flap", "flew", "flux", "foxy", "free", "frog", "fuel",
    "fund", "gala", "game", "gear", "gems", "gift", "girl", "glow",
    "good", "gray", "grim", "guru", "gush", "gyro", "half", "hang",
    "hard", "hawk", "heat", "help", "high", "hill", "holy", "hope",
    "horn", "huts", "iced", "idea", "idle", "inch", "inky", "into",
    "iris", "iron", "item", "jade", "jazz", "join", "jolt", "jowl",
    "judo", "jugs", "jump", "junk", "jury", "keep", "keno", "kept",
    "keys", "kick", "kiln", "king", "kite", "kiwi", "knob", "lamb",
    "lava", "lazy", "leaf", "legs", "liar", "limp", "lion", "list",
    "logo", "loud", "love", "luau", "luck", "lung", "main", "many",
    "math", "maze", "memo", "menu", "meow", "mild", "mint", "miss",
    "monk", "nail", "navy", "need", "news", "next", "noon", "note",
    "numb", "obey", "oboe", "omit", "onyx", "open", "oval", "owls",
    "paid", "part", "peck", "play", "plus", "poem", "pool", "pose",
    "puff", "puma", "purr", "quad", "quiz", "race", "ramp", "real",
    "redo", "rich", "road", "rock", "roof", "ruby", "ruin", "runs",
    "rust", "safe", "saga", "scar", "sets", "silk", "skew", "slot",
    "soap", "solo", "song", "stub", "surf", "swan", "taco", "task",
    "taxi", "tent", "tied", "time", "tiny", "toil", "tomb", "toys",
    "trip", "tuna", "twin", "ugly", "undo", "unit", "urge", "user",
    "vast", "very", "veto", "vial", "vibe", "view", "visa", "void",
    "vows", "wall", "wand", "warm", "wasp", "wave", "waxy", "webs",
    "what", "when", "whiz", "wolf", "work", "yank", "yawn", "yell",
    "yoga", "yurt", "zaps", "zero", "zest", "zinc", "zone", "zoom",
];

/// How wide the perfect-hash table over the first and last letter is.
const HASH_SPAN: usize = 628;

/// `(25 * first + 11 * last) mod 628`: BCR-2020-012's lookup, which is
/// injective over the 256 words and therefore over their minimal forms,
/// since a minimal word is that same pair of letters.
const fn word_hash(first: u8, last: u8) -> usize {
    (25 * first as usize + 11 * last as usize) % HASH_SPAN
}

/// Which byte a hash belongs to, or `None` where no word hashes there.
const BYTE_BY_HASH: [Option<u8>; HASH_SPAN] = {
    let mut table = [None; HASH_SPAN];
    let mut byte = 0usize;
    while byte < 256 {
        let word = WORDS[byte].as_bytes();
        table[word_hash(word[0], word[3])] = Some(byte as u8);
        byte += 1;
    }
    table
};

/// The minimal-style bytewords of `data` with its CRC-32 appended.
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(2 * (data.len() + 4));
    let checksum = crc32(data).to_be_bytes();
    for &byte in data.iter().chain(checksum.iter()) {
        let word = WORDS[byte as usize].as_bytes();
        out.push(char::from(word[0]));
        out.push(char::from(word[3]));
    }
    out
}

/// The payload behind a minimal-style bytewords string, checksum
/// stripped and verified. `encoded` must already be lower case.
pub fn decode(encoded: &str) -> Result<Vec<u8>, Fault> {
    if !encoded.is_ascii() {
        return Err(Fault::NonAscii);
    }
    if !encoded.len().is_multiple_of(2) {
        return Err(Fault::InvalidLength);
    }
    let bytes = encoded.as_bytes();
    let mut data = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let byte = BYTE_BY_HASH[word_hash(pair[0], pair[1])].ok_or(Fault::InvalidWord)?;
        let word = WORDS[byte as usize].as_bytes();
        if word[0] != pair[0] || word[3] != pair[1] {
            return Err(Fault::InvalidWord);
        }
        data.push(byte);
    }
    if data.len() < 4 {
        return Err(Fault::InvalidChecksum);
    }
    let (payload, checksum) = data.split_at(data.len() - 4);
    if crc32(payload).to_be_bytes() != checksum {
        return Err(Fault::InvalidChecksum);
    }
    data.truncate(data.len() - 4);
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_published_check_values() {
        assert_eq!(crc32(b"Hello, world!"), 0xebe6_c6e6);
        assert_eq!(crc32(b"Wolf"), 0x598c_84dc);
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn bytewords_round_trip() {
        let input = alloc::vec![0, 1, 2, 128, 255];
        assert_eq!(encode(&input), "aeadaolazmjendeoti");
        assert_eq!(decode("aeadaolazmjendeoti").unwrap(), input);
        assert_eq!(decode(&encode(&[])).unwrap(), Vec::<u8>::new());
        assert_eq!(decode(&encode(&[0])).unwrap(), alloc::vec![0]);
        assert_eq!(encode(&[0]), "aetdaowslg");
    }

    #[test]
    fn a_hundred_bytes_from_the_specification() {
        let input: [u8; 100] = [
            245, 215, 20, 198, 241, 235, 69, 59, 209, 205, 165, 18, 150, 158, 116, 135, 229, 212,
            19, 159, 17, 37, 239, 240, 253, 11, 109, 191, 37, 242, 38, 120, 223, 41, 156, 189, 242,
            254, 147, 204, 66, 163, 216, 175, 191, 72, 169, 54, 32, 60, 144, 230, 210, 137, 184,
            197, 33, 113, 88, 14, 157, 31, 177, 46, 1, 115, 205, 69, 225, 150, 65, 235, 58, 144,
            65, 240, 133, 69, 113, 247, 63, 53, 242, 165, 160, 144, 26, 13, 79, 237, 133, 71, 82,
            69, 254, 165, 138, 41, 85, 24,
        ];
        let encoded = "yktsbbswwnwmfefrttsnonbgmtnnjyltvwtybwne\
                       bydawswtzcbdjnrsdawzdsksurdtnsrywzzemusf\
                       fwottppersfdptencxfnmhvatdldroskcljshdba\
                       ntctpadmadjksnfevymtfpwmftmhfpwtlpfejsyl\
                       fhecwzonnbmhcybtgwwelpflgmfezeonledtgocs\
                       fzhycypf";
        assert_eq!(encode(&input), encoded);
        assert_eq!(decode(encoded).unwrap(), input.to_vec());
    }

    #[test]
    fn a_frame_read_wrong_is_refused() {
        // One word changed: "zero" for "zoom", which still hashes to a
        // word but no longer matches the checksum.
        assert_eq!(decode("aeadaolazojendeowf"), Err(Fault::InvalidChecksum));
        // Letter pairs that hash to a valid byte but are not its word.
        assert_eq!(decode("abmuammdwe"), Err(Fault::InvalidWord));
        // Shorter than the checksum it must carry.
        assert_eq!(decode("aeadao"), Err(Fault::InvalidChecksum));
        // An odd number of characters cannot be pairs of letters.
        assert_eq!(decode("aea"), Err(Fault::InvalidLength));
        assert_eq!(decode("₿"), Err(Fault::NonAscii));
    }
}
