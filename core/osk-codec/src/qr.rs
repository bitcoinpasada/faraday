//! QR code encoding (ISO/IEC 18004 model 2): every version 1–40, the four
//! error-correction levels, numeric, alphanumeric and byte modes.
//!
//! The algorithm is a port of Project Nayuki's QR Code generator
//! (`qrcodegen`, MIT licensed, © Project Nayuki), restructured for
//! `no_std` and for secret payloads: no published QR encoder is `no_std`
//! (`docs/PLANNING.md` §16.20). What this port changes:
//!
//! - the module grid is a fixed bitset ([`QrMatrix`]) that zeroizes on
//!   drop, since a SeedQR *is* the seed;
//! - every intermediate buffer that holds payload bits (the bit stream,
//!   the data and error-correction codewords) is a [`Zeroizing`] vector;
//! - the error-correction level is never boosted, so a SeedQR comes out
//!   at the level the SeedQR specification names (L) and matches the
//!   reference tools module for module when they pick the same mask;
//! - one segment per code: the caller picks the mode through [`Payload`].
//!   SeedQR needs numeric mode explicitly, and text such as PSBT base64
//!   goes in byte mode because the alphanumeric alphabet has no lowercase.

use alloc::vec::Vec;
use core::fmt;

use zeroize::{Zeroize, Zeroizing};

/// Largest side of a QR code in modules (version 40).
pub const MAX_SIZE: usize = 177;
/// Bytes of the module bitset: `MAX_SIZE²` bits rounded up.
const MATRIX_BYTES: usize = MAX_SIZE.div_ceil(8) * MAX_SIZE;
/// Quiet zone the standard requires around a symbol, in modules.
pub const QUIET_ZONE: usize = 4;

/// Error-correction level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ecc {
    /// About 7 % of codewords recoverable.
    Low,
    /// About 15 %.
    Medium,
    /// About 25 %.
    Quartile,
    /// About 30 %.
    High,
}

impl Ecc {
    fn ordinal(self) -> usize {
        match self {
            Ecc::Low => 0,
            Ecc::Medium => 1,
            Ecc::Quartile => 2,
            Ecc::High => 3,
        }
    }

    fn format_bits(self) -> u32 {
        match self {
            Ecc::Low => 1,
            Ecc::Medium => 0,
            Ecc::Quartile => 3,
            Ecc::High => 2,
        }
    }
}

/// What to encode, and in which mode.
#[derive(Clone, Copy)]
pub enum Payload<'a> {
    /// ASCII digits `0`–`9` only, numeric mode: 3⅓ bits per digit. What
    /// SeedQR requires.
    Numeric(&'a [u8]),
    /// Digits, upper-case letters and ` $%*+-./:`, alphanumeric mode: 5½
    /// bits per character.
    Alphanumeric(&'a [u8]),
    /// Any bytes, 8 bits each. Text with lowercase letters goes here.
    Bytes(&'a [u8]),
}

/// Why a payload could not be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The payload does not fit version 40 at the requested level.
    TooLong,
    /// A character outside the chosen mode's alphabet.
    BadCharacter,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::TooLong => "payload too long for a QR code at this error-correction level",
            Error::BadCharacter => "character outside the QR mode's alphabet",
        })
    }
}

impl core::error::Error for Error {}

/// The modules of a QR code: `size × size`, dark = `true`, stored as a
/// fixed bitset so that no allocation holds a secret. Zeroized on drop.
/// Deliberately neither `Debug` nor `Clone`: share it through `Rc`.
pub struct QrMatrix {
    size: u8,
    version: u8,
    ecc: Ecc,
    bits: [u8; MATRIX_BYTES],
}

impl QrMatrix {
    fn blank(version: u8, ecc: Ecc) -> Self {
        QrMatrix {
            size: (usize::from(version) * 4 + 17) as u8,
            version,
            ecc,
            bits: [0; MATRIX_BYTES],
        }
    }

    /// Side length in modules, 21–177.
    pub fn size(&self) -> usize {
        usize::from(self.size)
    }

    /// Version 1–40.
    pub fn version(&self) -> u8 {
        self.version
    }

    /// The error-correction level encoded.
    pub fn ecc(&self) -> Ecc {
        self.ecc
    }

    /// Whether the module at `(x, y)` is dark. Outside the grid: light.
    pub fn module(&self, x: usize, y: usize) -> bool {
        if x >= self.size() || y >= self.size() {
            return false;
        }
        let i = y * self.size() + x;
        self.bits[i >> 3] >> (i & 7) & 1 == 1
    }

    fn set(&mut self, x: usize, y: usize, dark: bool) {
        let i = y * self.size() + x;
        if dark {
            self.bits[i >> 3] |= 1 << (i & 7);
        } else {
            self.bits[i >> 3] &= !(1 << (i & 7));
        }
    }

    fn flip(&mut self, x: usize, y: usize) {
        let i = y * self.size() + x;
        self.bits[i >> 3] ^= 1 << (i & 7);
    }

    /// Renders the symbol as 8-bit luma, each module `scale × scale`
    /// pixels, dark modules 0 and light modules 255, with `quiet` modules
    /// of light border on every side. Returns `(width, height, pixels)`.
    /// Test and tooling helper: the on-device renderer is in `osk-ui`.
    pub fn to_luma(&self, scale: usize, quiet: usize) -> (usize, usize, Vec<u8>) {
        let side = (self.size() + 2 * quiet) * scale;
        let mut px = alloc::vec![255u8; side * side];
        for y in 0..self.size() {
            for x in 0..self.size() {
                if !self.module(x, y) {
                    continue;
                }
                for dy in 0..scale {
                    let row = ((y + quiet) * scale + dy) * side + (x + quiet) * scale;
                    px[row..row + scale].fill(0);
                }
            }
        }
        (side, side, px)
    }
}

impl Zeroize for QrMatrix {
    fn zeroize(&mut self) {
        self.bits.zeroize();
        self.size = 0;
        self.version = 0;
    }
}

impl Drop for QrMatrix {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Encodes `payload` at level `ecc` in the smallest version that fits.
pub fn encode(payload: Payload<'_>, ecc: Ecc) -> Result<QrMatrix, Error> {
    encode_bounded(payload, ecc, 40)
}

/// [`encode`] restricted to versions up to `max_version` (1–40).
pub fn encode_bounded(payload: Payload<'_>, ecc: Ecc, max_version: u8) -> Result<QrMatrix, Error> {
    let max_version = max_version.clamp(1, 40);
    let (mode, count, data_bits) = segment_bits(payload)?;
    // Smallest version whose data capacity holds the segment.
    let mut version = 1u8;
    loop {
        let capacity = num_data_codewords(version, ecc) * 8;
        let needed = total_bits(mode, count, data_bits.len_bits(), version);
        match needed {
            Some(n) if n <= capacity => break,
            _ if version >= max_version => return Err(Error::TooLong),
            _ => version += 1,
        }
    }
    let capacity = num_data_codewords(version, ecc) * 8;

    // Mode indicator, character count, data, terminator, pad to a byte,
    // then alternating pad bytes up to the capacity.
    let mut bb = BitWriter::with_capacity(capacity / 8);
    bb.append(mode.indicator(), 4);
    bb.append(count as u32, mode.count_bits(version));
    for i in 0..data_bits.len_bits() {
        bb.append(u32::from(data_bits.bit(i)), 1);
    }
    let terminator = 4.min(capacity - bb.len_bits());
    bb.append(0, terminator as u8);
    let to_byte = bb.len_bits().wrapping_neg() & 7;
    bb.append(0, to_byte as u8);
    let mut pad = 0xEC;
    while bb.len_bits() < capacity {
        bb.append(pad, 8);
        pad ^= 0xEC ^ 0x11;
    }
    let data = bb.into_bytes();

    let mut qr = QrMatrix::blank(version, ecc);
    let mut function = FunctionMask::new(qr.size());
    draw_function_patterns(&mut qr, &mut function);
    let all = add_ecc_and_interleave(version, ecc, &data);
    draw_codewords(&mut qr, &function, &all);
    let mut best = 0u8;
    let mut best_penalty = i32::MAX;
    for mask in 0..8u8 {
        apply_mask(&mut qr, &function, mask);
        draw_format_bits(&mut qr, &mut function, ecc, mask);
        let penalty = penalty_score(&qr);
        if penalty < best_penalty {
            best = mask;
            best_penalty = penalty;
        }
        apply_mask(&mut qr, &function, mask);
    }
    apply_mask(&mut qr, &function, best);
    draw_format_bits(&mut qr, &mut function, ecc, best);
    Ok(qr)
}

/// Largest byte-mode payload that fits version `version` at `ecc`.
pub fn byte_capacity(version: u8, ecc: Ecc) -> usize {
    let version = version.clamp(1, 40);
    let bits = num_data_codewords(version, ecc) * 8;
    bits.saturating_sub(4 + usize::from(Mode::Byte.count_bits(version))) / 8
}

/// Largest alphanumeric-mode payload, in characters, that fits version
/// `version` at `ecc`.
///
/// Alphanumeric mode packs two characters into 11 bits and a lone last
/// character into 6, which is why this is not a byte count divided by
/// anything. A caller that has to split a message into codes of a known
/// version — an animated BC-UR run at the module-pitch floor — sizes its
/// fragments from this.
pub fn alnum_capacity(version: u8, ecc: Ecc) -> usize {
    let version = version.clamp(1, 40);
    let bits = num_data_codewords(version, ecc) * 8;
    let bits = bits.saturating_sub(4 + usize::from(Mode::Alphanumeric.count_bits(version)));
    let mut chars = bits / 11 * 2;
    if bits % 11 >= 6 {
        chars += 1;
    }
    chars
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
}

impl Mode {
    fn indicator(self) -> u32 {
        match self {
            Mode::Numeric => 0x1,
            Mode::Alphanumeric => 0x2,
            Mode::Byte => 0x4,
        }
    }

    fn count_bits(self, version: u8) -> u8 {
        let i = usize::from((version + 7) / 17);
        match self {
            Mode::Numeric => [10, 12, 14][i],
            Mode::Alphanumeric => [9, 11, 13][i],
            Mode::Byte => [8, 16, 16][i],
        }
    }
}

const ALPHANUMERIC: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// The data bits of the one segment: mode, character count, bits.
fn segment_bits(payload: Payload<'_>) -> Result<(Mode, usize, BitWriter), Error> {
    match payload {
        Payload::Numeric(digits) => {
            let mut bb = BitWriter::with_capacity(digits.len() / 3 * 10 / 8 + 2);
            let mut acc = 0u32;
            let mut n = 0u8;
            for &d in digits {
                if !d.is_ascii_digit() {
                    return Err(Error::BadCharacter);
                }
                acc = acc * 10 + u32::from(d - b'0');
                n += 1;
                if n == 3 {
                    bb.append(acc, 10);
                    acc = 0;
                    n = 0;
                }
            }
            if n > 0 {
                bb.append(acc, n * 3 + 1);
            }
            Ok((Mode::Numeric, digits.len(), bb))
        }
        Payload::Alphanumeric(text) => {
            let mut bb = BitWriter::with_capacity(text.len() * 11 / 16 + 2);
            let mut acc = 0u32;
            let mut n = 0u8;
            for &c in text {
                let i = ALPHANUMERIC
                    .iter()
                    .position(|&a| a == c)
                    .ok_or(Error::BadCharacter)?;
                acc = acc * 45 + i as u32;
                n += 1;
                if n == 2 {
                    bb.append(acc, 11);
                    acc = 0;
                    n = 0;
                }
            }
            if n > 0 {
                bb.append(acc, 6);
            }
            Ok((Mode::Alphanumeric, text.len(), bb))
        }
        Payload::Bytes(bytes) => {
            let mut bb = BitWriter::with_capacity(bytes.len());
            for &b in bytes {
                bb.append(u32::from(b), 8);
            }
            Ok((Mode::Byte, bytes.len(), bb))
        }
    }
}

/// Bits the segment takes in `version`, or `None` when its character
/// count does not fit the count field.
fn total_bits(mode: Mode, count: usize, data_bits: usize, version: u8) -> Option<usize> {
    let cc = mode.count_bits(version);
    if count >= 1usize << cc {
        return None;
    }
    Some(4 + usize::from(cc) + data_bits)
}

/// A big-endian bit stream in a zeroizing byte vector.
struct BitWriter {
    bytes: Zeroizing<Vec<u8>>,
    len: usize,
}

impl BitWriter {
    fn with_capacity(bytes: usize) -> Self {
        BitWriter {
            bytes: Zeroizing::new(Vec::with_capacity(bytes + 1)),
            len: 0,
        }
    }

    fn len_bits(&self) -> usize {
        self.len
    }

    fn bit(&self, i: usize) -> bool {
        self.bytes[i >> 3] >> (7 - (i & 7)) & 1 == 1
    }

    fn append(&mut self, value: u32, bits: u8) {
        debug_assert!(bits <= 31 && value >> bits == 0);
        for i in (0..bits).rev() {
            if self.len & 7 == 0 {
                self.bytes.push(0);
            }
            let b = (value >> i) & 1;
            let last = self.bytes.len() - 1;
            self.bytes[last] |= (b as u8) << (7 - (self.len & 7));
            self.len += 1;
        }
    }

    fn into_bytes(self) -> Zeroizing<Vec<u8>> {
        self.bytes
    }
}

/// Which modules are function patterns (not masked, not data).
struct FunctionMask {
    size: usize,
    bits: [u8; MATRIX_BYTES],
}

impl FunctionMask {
    fn new(size: usize) -> Self {
        FunctionMask {
            size,
            bits: [0; MATRIX_BYTES],
        }
    }

    fn get(&self, x: usize, y: usize) -> bool {
        let i = y * self.size + x;
        self.bits[i >> 3] >> (i & 7) & 1 == 1
    }

    fn mark(&mut self, x: usize, y: usize) {
        let i = y * self.size + x;
        self.bits[i >> 3] |= 1 << (i & 7);
    }
}

fn set_function(qr: &mut QrMatrix, f: &mut FunctionMask, x: usize, y: usize, dark: bool) {
    qr.set(x, y, dark);
    f.mark(x, y);
}

fn draw_function_patterns(qr: &mut QrMatrix, f: &mut FunctionMask) {
    let size = qr.size();
    for i in 0..size {
        set_function(qr, f, 6, i, i % 2 == 0);
        set_function(qr, f, i, 6, i % 2 == 0);
    }
    draw_finder(qr, f, 3, 3);
    draw_finder(qr, f, size - 4, 3);
    draw_finder(qr, f, 3, size - 4);
    let positions = alignment_positions(qr.version(), size);
    let n = positions.len();
    for i in 0..n {
        for j in 0..n {
            let corner = (i == 0 && j == 0) || (i == 0 && j == n - 1) || (i == n - 1 && j == 0);
            if !corner {
                draw_alignment(qr, f, positions[i], positions[j]);
            }
        }
    }
    draw_format_bits(qr, f, qr.ecc(), 0);
    draw_version(qr, f);
}

fn draw_finder(qr: &mut QrMatrix, f: &mut FunctionMask, cx: usize, cy: usize) {
    let size = qr.size() as i32;
    for dy in -4i32..=4 {
        for dx in -4i32..=4 {
            let (x, y) = (cx as i32 + dx, cy as i32 + dy);
            if (0..size).contains(&x) && (0..size).contains(&y) {
                let dist = dx.abs().max(dy.abs());
                set_function(qr, f, x as usize, y as usize, dist != 2 && dist != 4);
            }
        }
    }
}

fn draw_alignment(qr: &mut QrMatrix, f: &mut FunctionMask, cx: usize, cy: usize) {
    for dy in -2i32..=2 {
        for dx in -2i32..=2 {
            let (x, y) = ((cx as i32 + dx) as usize, (cy as i32 + dy) as usize);
            set_function(qr, f, x, y, dx.abs().max(dy.abs()) != 1);
        }
    }
}

fn alignment_positions(version: u8, size: usize) -> Vec<usize> {
    if version == 1 {
        return Vec::new();
    }
    let v = usize::from(version);
    let n = v / 7 + 2;
    let step = if version == 32 {
        26
    } else {
        (v * 4 + n * 2 + 1) / (n * 2 - 2) * 2
    };
    let mut out: Vec<usize> = (0..n - 1).map(|i| size - 7 - i * step).collect();
    out.push(6);
    out.reverse();
    out
}

fn draw_format_bits(qr: &mut QrMatrix, f: &mut FunctionMask, ecc: Ecc, mask: u8) {
    let data = ecc.format_bits() << 3 | u32::from(mask);
    let mut rem = data;
    for _ in 0..10 {
        rem = (rem << 1) ^ ((rem >> 9) * 0x537);
    }
    let bits = (data << 10 | rem) ^ 0x5412;
    let bit = |i: usize| (bits >> i) & 1 == 1;
    let size = qr.size();
    for i in 0..6 {
        set_function(qr, f, 8, i, bit(i));
    }
    set_function(qr, f, 8, 7, bit(6));
    set_function(qr, f, 8, 8, bit(7));
    set_function(qr, f, 7, 8, bit(8));
    for i in 9..15 {
        set_function(qr, f, 14 - i, 8, bit(i));
    }
    for i in 0..8 {
        set_function(qr, f, size - 1 - i, 8, bit(i));
    }
    for i in 8..15 {
        set_function(qr, f, 8, size - 15 + i, bit(i));
    }
    set_function(qr, f, 8, size - 8, true);
}

fn draw_version(qr: &mut QrMatrix, f: &mut FunctionMask) {
    let version = qr.version();
    if version < 7 {
        return;
    }
    let data = u32::from(version);
    let mut rem = data;
    for _ in 0..12 {
        rem = (rem << 1) ^ ((rem >> 11) * 0x1F25);
    }
    let bits = data << 12 | rem;
    let size = qr.size();
    for i in 0..18 {
        let dark = (bits >> i) & 1 == 1;
        let a = size - 11 + i % 3;
        let b = i / 3;
        set_function(qr, f, a, b, dark);
        set_function(qr, f, b, a, dark);
    }
}

fn add_ecc_and_interleave(version: u8, ecc: Ecc, data: &[u8]) -> Zeroizing<Vec<u8>> {
    debug_assert_eq!(data.len(), num_data_codewords(version, ecc));
    let blocks = table(&NUM_ERROR_CORRECTION_BLOCKS, version, ecc);
    let ecc_len = table(&ECC_CODEWORDS_PER_BLOCK, version, ecc);
    let raw = num_raw_data_modules(version) / 8;
    let short_blocks = blocks - raw % blocks;
    let short_len = raw / blocks;
    let divisor = rs_divisor(ecc_len);
    let mut out: Vec<Zeroizing<Vec<u8>>> = Vec::with_capacity(blocks);
    let mut k = 0;
    for i in 0..blocks {
        let dat_len = short_len - ecc_len + usize::from(i >= short_blocks);
        let mut block = Zeroizing::new(Vec::with_capacity(short_len + 1));
        block.extend_from_slice(&data[k..k + dat_len]);
        k += dat_len;
        let rem = rs_remainder(&block, &divisor);
        if i < short_blocks {
            block.push(0);
        }
        block.extend_from_slice(&rem);
        out.push(block);
    }
    let mut result = Zeroizing::new(Vec::with_capacity(raw));
    for i in 0..=short_len {
        for (j, block) in out.iter().enumerate() {
            if i != short_len - ecc_len || j >= short_blocks {
                result.push(block[i]);
            }
        }
    }
    result
}

fn draw_codewords(qr: &mut QrMatrix, f: &FunctionMask, data: &[u8]) {
    let size = qr.size();
    debug_assert_eq!(data.len(), num_raw_data_modules(qr.version()) / 8);
    let mut i = 0usize;
    let mut right = size as i32 - 1;
    while right >= 1 {
        if right == 6 {
            right = 5;
        }
        for vert in 0..size {
            for j in 0..2 {
                let x = (right - j) as usize;
                let upward = (right + 1) & 2 == 0;
                let y = if upward { size - 1 - vert } else { vert };
                if !f.get(x, y) && i < data.len() * 8 {
                    let dark = data[i >> 3] >> (7 - (i & 7)) & 1 == 1;
                    qr.set(x, y, dark);
                    i += 1;
                }
            }
        }
        right -= 2;
    }
    debug_assert_eq!(i, data.len() * 8);
}

fn apply_mask(qr: &mut QrMatrix, f: &FunctionMask, mask: u8) {
    let size = qr.size();
    for y in 0..size {
        for x in 0..size {
            let invert = match mask {
                0 => (x + y) % 2 == 0,
                1 => y % 2 == 0,
                2 => x % 3 == 0,
                3 => (x + y) % 3 == 0,
                4 => (x / 3 + y / 2) % 2 == 0,
                5 => x * y % 2 + x * y % 3 == 0,
                6 => (x * y % 2 + x * y % 3) % 2 == 0,
                _ => ((x + y) % 2 + x * y % 3) % 2 == 0,
            };
            if invert && !f.get(x, y) {
                qr.flip(x, y);
            }
        }
    }
}

const PENALTY_N1: i32 = 3;
const PENALTY_N2: i32 = 3;
const PENALTY_N3: i32 = 40;
const PENALTY_N4: i32 = 10;

fn penalty_score(qr: &QrMatrix) -> i32 {
    let size = qr.size();
    let mut result = 0i32;
    for y in 0..size {
        let mut run_color = false;
        let mut run = 0i32;
        let mut history = FinderPenalty::new(size as i32);
        for x in 0..size {
            if qr.module(x, y) == run_color {
                run += 1;
                if run == 5 {
                    result += PENALTY_N1;
                } else if run > 5 {
                    result += 1;
                }
            } else {
                history.add(run);
                if !run_color {
                    result += history.count() * PENALTY_N3;
                }
                run_color = qr.module(x, y);
                run = 1;
            }
        }
        result += history.terminate(run_color, run) * PENALTY_N3;
    }
    for x in 0..size {
        let mut run_color = false;
        let mut run = 0i32;
        let mut history = FinderPenalty::new(size as i32);
        for y in 0..size {
            if qr.module(x, y) == run_color {
                run += 1;
                if run == 5 {
                    result += PENALTY_N1;
                } else if run > 5 {
                    result += 1;
                }
            } else {
                history.add(run);
                if !run_color {
                    result += history.count() * PENALTY_N3;
                }
                run_color = qr.module(x, y);
                run = 1;
            }
        }
        result += history.terminate(run_color, run) * PENALTY_N3;
    }
    for y in 0..size - 1 {
        for x in 0..size - 1 {
            let c = qr.module(x, y);
            if c == qr.module(x + 1, y) && c == qr.module(x, y + 1) && c == qr.module(x + 1, y + 1)
            {
                result += PENALTY_N2;
            }
        }
    }
    let mut dark = 0i32;
    for y in 0..size {
        for x in 0..size {
            dark += i32::from(qr.module(x, y));
        }
    }
    let total = (size * size) as i32;
    let k = ((dark * 20 - total * 10).abs() + total - 1) / total - 1;
    result + k * PENALTY_N4
}

/// Run-length history for the finder-like-pattern penalty.
struct FinderPenalty {
    size: i32,
    history: [i32; 7],
}

impl FinderPenalty {
    fn new(size: i32) -> Self {
        FinderPenalty {
            size,
            history: [0; 7],
        }
    }

    fn add(&mut self, mut run: i32) {
        if self.history[0] == 0 {
            run += self.size;
        }
        for i in (0..6).rev() {
            self.history[i + 1] = self.history[i];
        }
        self.history[0] = run;
    }

    fn count(&self) -> i32 {
        let h = &self.history;
        let n = h[1];
        let core = n > 0 && h[2] == n && h[3] == n * 3 && h[4] == n && h[5] == n;
        i32::from(core && h[0] >= n * 4 && h[6] >= n)
            + i32::from(core && h[6] >= n * 4 && h[0] >= n)
    }

    fn terminate(mut self, run_color: bool, mut run: i32) -> i32 {
        if run_color {
            self.add(run);
            run = 0;
        }
        run += self.size;
        self.add(run);
        self.count()
    }
}

fn num_raw_data_modules(version: u8) -> usize {
    let v = usize::from(version);
    let mut result = (16 * v + 128) * v + 64;
    if v >= 2 {
        let n = v / 7 + 2;
        result -= (25 * n - 10) * n - 55;
        if v >= 7 {
            result -= 36;
        }
    }
    result
}

fn num_data_codewords(version: u8, ecc: Ecc) -> usize {
    num_raw_data_modules(version) / 8
        - table(&ECC_CODEWORDS_PER_BLOCK, version, ecc)
            * table(&NUM_ERROR_CORRECTION_BLOCKS, version, ecc)
}

fn table(t: &[[u8; 41]; 4], version: u8, ecc: Ecc) -> usize {
    usize::from(t[ecc.ordinal()][usize::from(version)])
}

fn rs_divisor(degree: usize) -> Vec<u8> {
    let mut result = alloc::vec![0u8; degree - 1];
    result.push(1);
    let mut root = 1u8;
    for _ in 0..degree {
        for j in 0..degree {
            result[j] = gf_mul(result[j], root);
            if j + 1 < result.len() {
                result[j] ^= result[j + 1];
            }
        }
        root = gf_mul(root, 2);
    }
    result
}

fn rs_remainder(data: &[u8], divisor: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut result = Zeroizing::new(alloc::vec![0u8; divisor.len()]);
    for &b in data {
        let factor = b ^ result.remove(0);
        result.push(0);
        for (x, &y) in result.iter_mut().zip(divisor) {
            *x ^= gf_mul(y, factor);
        }
    }
    result
}

/// Multiplication in GF(2⁸) with the QR polynomial x⁸+x⁴+x³+x²+1.
fn gf_mul(x: u8, y: u8) -> u8 {
    let mut z = 0u8;
    for i in (0..8).rev() {
        z = (z << 1) ^ ((z >> 7) * 0x1D);
        z ^= ((y >> i) & 1) * x;
    }
    z
}

/// Error-correction codewords per block, by level then version (index 0
/// unused). From ISO/IEC 18004 table 9.
static ECC_CODEWORDS_PER_BLOCK: [[u8; 41]; 4] = [
    [
        0, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28,
        30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
    [
        0, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28,
        28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    ],
    [
        0, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30,
        30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
    [
        0, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24,
        30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
];

/// Number of error-correction blocks, by level then version.
static NUM_ERROR_CORRECTION_BLOCKS: [[u8; 41]; 4] = [
    [
        0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13,
        14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25,
    ],
    [
        0, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21,
        23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49,
    ],
    [
        0, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29,
        34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68,
    ],
    [
        0, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32,
        35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81,
    ],
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The error of a result whose success type is not `Debug`.
    fn err<T>(r: Result<T, Error>) -> Error {
        match r {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        }
    }

    #[test]
    fn capacities_match_the_standard() {
        // Data codewords: v1-L 19, v1-H 9, v2-L 34, v3-L 55, v40-L 2956.
        assert_eq!(num_data_codewords(1, Ecc::Low), 19);
        assert_eq!(num_data_codewords(1, Ecc::High), 9);
        assert_eq!(num_data_codewords(2, Ecc::Low), 34);
        assert_eq!(num_data_codewords(3, Ecc::Low), 55);
        assert_eq!(num_data_codewords(40, Ecc::Low), 2956);
        assert_eq!(byte_capacity(40, Ecc::Low), 2953);
        assert_eq!(byte_capacity(1, Ecc::Low), 17);
        assert_eq!(alignment_positions(7, 45), alloc::vec![6, 22, 38]);
        assert_eq!(
            alignment_positions(32, 145),
            alloc::vec![6, 34, 60, 86, 112, 138]
        );
    }

    #[test]
    fn seedqr_sizes_follow_the_specification() {
        // 48 digits → 25×25 (version 2), 96 → 29×29; 16 bytes → 21×21,
        // 32 bytes → 25×25, all at level L (SeedQR README).
        let d48 = [b'1'; 48];
        assert_eq!(encode(Payload::Numeric(&d48), Ecc::Low).unwrap().size(), 25);
        let d96 = [b'7'; 96];
        assert_eq!(encode(Payload::Numeric(&d96), Ecc::Low).unwrap().size(), 29);
        assert_eq!(
            encode(Payload::Bytes(&[0u8; 16]), Ecc::Low).unwrap().size(),
            21
        );
        assert_eq!(
            encode(Payload::Bytes(&[0xffu8; 32]), Ecc::Low)
                .unwrap()
                .size(),
            25
        );
    }

    #[test]
    fn known_answer_version_1() {
        // "HELLO WORLD" alphanumeric at level Q: the canonical example
        // (ISO/IEC 18004 annex I): version 1, 21 modules, and the format
        // information for level Q with the chosen mask reads back.
        let qr = encode(Payload::Alphanumeric(b"HELLO WORLD"), Ecc::Quartile).unwrap();
        assert_eq!(qr.size(), 21);
        assert_eq!(qr.version(), 1);
        // Finder patterns in three corners, dark module next to the
        // bottom-left finder, timing pattern alternates.
        assert!(qr.module(0, 0) && qr.module(20, 0) && qr.module(0, 20));
        assert!(!qr.module(1, 1) && qr.module(3, 3));
        assert!(qr.module(8, 13), "always-dark module");
        for i in 8..13 {
            assert_eq!(qr.module(i, 6), i % 2 == 0);
        }
    }

    #[test]
    fn rejects_bad_characters_and_oversize() {
        assert_eq!(
            err(encode(Payload::Numeric(b"12a"), Ecc::Low)),
            Error::BadCharacter
        );
        assert_eq!(
            err(encode(Payload::Alphanumeric(b"abc"), Ecc::Low)),
            Error::BadCharacter
        );
        let big = alloc::vec![0u8; 2954];
        assert_eq!(err(encode(Payload::Bytes(&big), Ecc::Low)), Error::TooLong);
        assert!(encode(Payload::Bytes(&big[..2953]), Ecc::Low).is_ok());
        assert_eq!(
            err(encode_bounded(Payload::Bytes(&big[..100]), Ecc::Low, 1)),
            Error::TooLong
        );
    }

    #[test]
    fn luma_render_has_a_quiet_zone() {
        let qr = encode(Payload::Bytes(b"x"), Ecc::Low).unwrap();
        let (w, h, px) = qr.to_luma(2, QUIET_ZONE);
        assert_eq!((w, h), (58, 58));
        assert!(px[..w * 8].iter().all(|&p| p == 255), "top border light");
        assert_eq!(px[8 * w + 8], 0, "finder corner dark");
    }
}
