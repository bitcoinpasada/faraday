//! A single-part UR of any registry type: `ur:<type>/<bytewords>`, the
//! minimal bytewords of the CBOR with its CRC-32 (BCR-2020-005, -012).
//! For `ur:crypto-account`, which `osk-codec`'s encoder (`crypto-psbt` and
//! `bytes` only) does not write. The word list and the checksum are a
//! copy of `osk-codec/src/ur/bytewords.rs` at upstream c418768, whose
//! module is private to that crate; `osk-codec` reads what this writes.

fn crc32(data: &[u8]) -> u32 {
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

/// `ur:<ur_type>/<minimal bytewords of cbor and its CRC-32>`.
pub fn single(ur_type: &str, cbor: &[u8]) -> String {
    let mut out = format!("ur:{ur_type}/");
    let checksum = crc32(cbor).to_be_bytes();
    for &byte in cbor.iter().chain(checksum.iter()) {
        let word = WORDS[byte as usize].as_bytes();
        out.push(char::from(word[0]));
        out.push(char::from(word[3]));
    }
    out
}
