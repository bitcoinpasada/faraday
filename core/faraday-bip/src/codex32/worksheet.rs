//! The codex32 booklet's worksheets as numbers: the checksum worksheet,
//! row by row, and the translation worksheet with its translation,
//! recovery and fused symbols (`docs/CODEX32.md` §7).
//!
//! A box is a character's place in the whole string, counted from 1:
//! `m` is box 1, the threshold box 4, the share index box 9, the data
//! from box 10 (`docs/CODEX32.md` §3). Every accessor here takes and
//! gives box numbers.
//!
//! **The checksum worksheet** ([`ChecksumWorksheet`]) is the booklet's
//! ladder. Its first row is the first 13 characters after `ms1` (15 for
//! a long checksum); under it the residue of `ms`, the same on every
//! sheet; then, step by step, a sum row (the residue so far, which is
//! BIP 93's polymod over every character up to the row's last box) and
//! a lookup row (the checksum table's row for the sum row's two leading
//! symbols, written two boxes to the right); and last the final row,
//! which on a valid string reads `SECRETSHARE32` (`SECRETSHARE32EX` at
//! 127 characters). Every cell in a box's column is worked from that
//! box and the boxes left of it, so the sheet fills column by column as
//! characters arrive ([`ChecksumWorksheet::push`]), and the cells it
//! holds after each push are those the whole sheet holds.
//!
//! Where the characters after the first row are odd in number (54, 61,
//! 67 and 127 characters), the first step takes one character and its
//! lookup row is the table's row for one leading symbol; the booklet
//! prints sheets for 48 and 74 characters only, where every step takes
//! two.
//!
//! **The translation worksheet** ([`TranslationWorksheet`]) makes the
//! string at one share index from `k` strings of a set: each input's
//! characters times its translation symbol, and the sum. A translation
//! symbol is the product (the booklet's fusion) of one pair symbol per
//! other input ([`pair_symbol`]); towards `s` the pair symbols are the
//! booklet's recovery symbols, and towards a new share the products are
//! the columns of its derivation tables.
//!
//! Every value here is derived from secret material: the types are
//! wiped on drop and print nothing.

use faraday_crypto::work::Compute;
use faraday_crypto::{Zeroize, ZeroizeOnDrop};

use super::{
    CHECKSUM_LEN, Codex32, Error, HRP_EXPANDED_LEN, LONG_CHECKSUM_LEN, MAX_DATA_LEN, MAX_THRESHOLD,
    MS32_CONST, MS32_HRP_RESIDUE, MS32_LONG_CONST, SECRET_INDEX, VALID_LENGTHS, charval, check_set,
    gf_mul, lagrange, note, residue_step, verify_checksum,
};

/// The booklet's second alphabet, by symbol value: the glyph a
/// translation, recovery or fused symbol of value `v` is drawn as
/// (`code2` in the booklet's PostScript source). Value 0 is never a
/// symbol; the booklet draws it as `×`, the empty corner of its tables.
pub const SYMBOL_GLYPHS: [char; 32] = [
    '×', 'ℵ', 'α', 'β', 'Γ', 'Δ', 'ε', 'η', 'Θ', 'Λ', 'μ', 'Ξ', 'Π', 'ρ', 'Σ', 'Φ', 'Ψ', 'Ω', '@',
    '#', '%', '¢', '¥', '€', '¤', '⊕', '†', '‡', '§', '¶', '♦', '♥',
];

/// The PostScript glyph name of each symbol of [`SYMBOL_GLYPHS`], as the
/// booklet's `code2` array names it.
pub const SYMBOL_NAMES: [&str; 32] = [
    "multiply",
    "aleph",
    "alpha",
    "beta",
    "Gamma",
    "Delta",
    "epsilon",
    "eta",
    "Theta",
    "Lambda",
    "mu",
    "Xi",
    "Pi",
    "rho",
    "Sigma",
    "Phi",
    "Psi",
    "Omega",
    "at",
    "numbersign",
    "percent",
    "cent",
    "yen",
    "Euro",
    "currency",
    "circleplus",
    "dagger",
    "daggerdbl",
    "section",
    "paragraph",
    "diamond",
    "heart",
];

/// Boxes before the first one a worksheet holds: `m`, `s` and `1`.
const PREFIX: usize = 3;
/// The widest row: a long checksum's 15 characters.
const MAX_ROW: usize = LONG_CHECKSUM_LEN;
/// The most steps a sheet has: 127 characters, 124 after `ms1`, 15 in
/// the first row and the other 109 one step of one and 54 of two.
const MAX_STEPS: usize = (MAX_DATA_LEN - LONG_CHECKSUM_LEN).div_ceil(2);

/// The low `L` symbols of a residue, most significant first.
const fn digits(residue: u128, len: usize) -> [u8; MAX_ROW] {
    let mut out = [0u8; MAX_ROW];
    let mut i = 0;
    while i < len {
        out[i] = ((residue >> (5 * (len - 1 - i))) & 31) as u8;
        i += 1;
    }
    out
}

/// The residue of `ms` carried `len` characters on: the row under the
/// first row of every sheet.
const fn hrp_row(long: bool) -> [u8; MAX_ROW] {
    let len = if long {
        LONG_CHECKSUM_LEN
    } else {
        CHECKSUM_LEN
    };
    let mut r = MS32_HRP_RESIDUE;
    let mut i = 0;
    while i < len {
        r = residue_step(r, 0, long);
        i += 1;
    }
    digits(r, len)
}

const HRP_ROW: [u8; MAX_ROW] = hrp_row(false);
const LONG_HRP_ROW: [u8; MAX_ROW] = hrp_row(true);
/// `SECRETSHARE32`: the final row of every valid string's sheet.
const TARGET: [u8; MAX_ROW] = digits(MS32_CONST, CHECKSUM_LEN);
/// `SECRETSHARE32EX`: the same for the long checksum.
const LONG_TARGET: [u8; MAX_ROW] = digits(MS32_LONG_CONST, LONG_CHECKSUM_LEN);

/// The lookup row for the leading symbols `lead`: their residue carried
/// `len` characters on, from nothing.
fn lookup(lead: &[u8], len: usize, long: bool) -> [u8; MAX_ROW] {
    let mut r = 0u128;
    for &v in lead {
        r = residue_step(r, v, long);
    }
    for _ in 0..len {
        r = residue_step(r, 0, long);
    }
    digits(r, len)
}

/// The booklet's checksum table (its pages 15 and 16): the 13 symbols
/// the leading pair `a`, `b` of a sum row selects. Cheap: fifteen steps
/// of the residue.
pub fn checksum_table(a: u8, b: u8) -> [u8; CHECKSUM_LEN] {
    let row = lookup(&[a & 31, b & 31], CHECKSUM_LEN, false);
    let mut out = [0u8; CHECKSUM_LEN];
    out.copy_from_slice(&row[..CHECKSUM_LEN]);
    out
}

// ---------------------------------------------------------------------
// The checksum worksheet
// ---------------------------------------------------------------------

/// One string's checksum worksheet, filled as far as its characters
/// are given.
///
/// It keeps every cell it has worked out: drawing reads them through
/// the accessors and computes nothing. Its invariant: after `n`
/// characters are given, every cell in the columns of those `n` boxes,
/// and every lookup row whose leading cells lie in them, is what the
/// whole sheet of those characters holds ([`Self::of`]); the cells
/// right of them read as empty.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ChecksumWorksheet {
    /// The string's length in characters, `ms1` included.
    len: u8,
    /// How many characters after `ms1` are given.
    given: u8,
    /// The characters after `ms1`, as symbols.
    symbols: [u8; MAX_DATA_LEN],
    /// Sum row `j` (row `steps` is the final row).
    sums: [[u8; MAX_ROW]; MAX_STEPS + 1],
    /// Lookup row `j`, under sum row `j`.
    lookups: [[u8; MAX_ROW]; MAX_STEPS],
}

impl ChecksumWorksheet {
    /// A blank worksheet for a string of `chars` characters. Cheap.
    pub fn new(chars: usize) -> Result<Self, Error> {
        if !VALID_LENGTHS.contains(&chars) {
            return Err(Error::BadLength);
        }
        Ok(ChecksumWorksheet {
            len: chars as u8,
            given: 0,
            symbols: [0u8; MAX_DATA_LEN],
            sums: [[0u8; MAX_ROW]; MAX_STEPS + 1],
            lookups: [[0u8; MAX_ROW]; MAX_STEPS],
        })
    }

    /// Costs a checksum worksheet of one string. The whole sheet of a
    /// valid string, every box given: the verification sheet, whose
    /// final row reads the constant.
    pub fn of(c: &Compute, s: &Codex32) -> Self {
        note(c);
        let len = usize::from(s.len);
        let mut sheet = ChecksumWorksheet {
            len: (len + PREFIX) as u8,
            given: 0,
            symbols: [0u8; MAX_DATA_LEN],
            sums: [[0u8; MAX_ROW]; MAX_STEPS + 1],
            lookups: [[0u8; MAX_ROW]; MAX_STEPS],
        };
        for &v in &s.data[..len] {
            sheet.put(v);
        }
        sheet
    }

    /// The string's length in characters, `ms1` included.
    pub fn chars(&self) -> usize {
        usize::from(self.len)
    }

    fn long(&self) -> bool {
        HRP_EXPANDED_LEN + self.chars() - PREFIX >= 96
    }

    /// Characters of checksum: 13, or 15 at 127 characters.
    pub fn checksum_len(&self) -> usize {
        if self.long() {
            LONG_CHECKSUM_LEN
        } else {
            CHECKSUM_LEN
        }
    }

    /// The last box before the checksum: 35 at 48 characters, 61 at 74.
    pub fn last_data_box(&self) -> usize {
        self.chars() - self.checksum_len()
    }

    /// The last box given so far; 3 when none is.
    pub fn last_given_box(&self) -> usize {
        PREFIX + usize::from(self.given)
    }

    /// Steps of the ladder: 16 at 48 characters, 29 at 74.
    pub fn steps(&self) -> usize {
        (self.chars() - PREFIX - self.checksum_len()).div_ceil(2)
    }

    fn odd(&self) -> bool {
        (self.chars() - PREFIX - self.checksum_len()) % 2 == 1
    }

    /// Characters step `j` takes: two, or one for the first step where
    /// the count is odd.
    pub fn step_width(&self, j: usize) -> usize {
        if j == 0 && self.odd() { 1 } else { 2 }
    }

    /// 0-based column (box minus 4) of sum row `j`'s first cell.
    fn start(&self, j: usize) -> usize {
        if self.odd() && j > 0 {
            2 * j - 1
        } else {
            2 * j
        }
    }

    /// The box of sum row `j`'s first cell: 4 for the first, then two
    /// boxes on a step. Row [`Self::steps`] is the final row.
    pub fn sum_box(&self, j: usize) -> usize {
        self.start(j) + PREFIX + 1
    }

    /// The box of lookup row `j`'s first cell: its sum row's, moved on
    /// by the step's width.
    pub fn lookup_box(&self, j: usize) -> usize {
        self.sum_box(j) + self.step_width(j)
    }

    /// The residue of `ms`, under the first row (boxes 4 onward): the
    /// same on every sheet of this length.
    pub fn hrp_row(&self) -> &'static [u8] {
        if self.long() {
            &LONG_HRP_ROW[..LONG_CHECKSUM_LEN]
        } else {
            &HRP_ROW[..CHECKSUM_LEN]
        }
    }

    /// What the final row of a valid string reads: `SECRETSHARE32`, or
    /// `SECRETSHARE32EX` at 127 characters.
    pub fn target(&self) -> &'static [u8] {
        if self.long() {
            &LONG_TARGET[..LONG_CHECKSUM_LEN]
        } else {
            &TARGET[..CHECKSUM_LEN]
        }
    }

    /// The character given at box `b`, if it is.
    pub fn symbol(&self, b: usize) -> Option<u8> {
        let col = b.checked_sub(PREFIX + 1)?;
        (col < usize::from(self.given)).then(|| self.symbols[col])
    }

    /// Cell `i` (from 0) of sum row `j`, if its box is given. Row
    /// [`Self::steps`] is the final row.
    pub fn sum_cell(&self, j: usize, i: usize) -> Option<u8> {
        if j > self.steps() || i >= self.checksum_len() {
            return None;
        }
        (self.start(j) + i < usize::from(self.given)).then(|| self.sums[j][i])
    }

    /// Cell `i` (from 0) of lookup row `j`, once the leading cells of
    /// sum row `j` that select it are given.
    pub fn lookup_cell(&self, j: usize, i: usize) -> Option<u8> {
        if j >= self.steps() || i >= self.checksum_len() {
            return None;
        }
        let lead_end = self.start(j) + self.step_width(j);
        (lead_end <= usize::from(self.given)).then(|| self.lookups[j][i])
    }

    /// Cell `i` of the final row, if its box is given.
    pub fn final_cell(&self, i: usize) -> Option<u8> {
        self.sum_cell(self.steps(), i)
    }

    /// The first final-row cell that is worked out and differs from
    /// [`Self::target`], counted from 0.
    pub fn first_difference(&self) -> Option<usize> {
        let target = self.target();
        (0..self.checksum_len()).find(|&i| self.final_cell(i).is_some_and(|v| v != target[i]))
    }

    /// Whether every box is given and the final row reads the target.
    pub fn holds(&self) -> bool {
        usize::from(self.given) == self.chars() - PREFIX && self.first_difference().is_none()
    }

    /// Costs a column of the worksheet, and the checksum table's row
    /// where the character completes a sum row's leading pair. Gives
    /// the next box its character (a symbol, `0..32`) and works out
    /// every cell of that box's column.
    pub fn push(&mut self, c: &Compute, symbol: u8) -> Result<(), Error> {
        note(c);
        let col = usize::from(self.given);
        if col >= self.chars() - PREFIX {
            return Err(Error::BadLength);
        }
        if symbol >= 32 {
            return Err(Error::BadCharacter {
                position: (col + PREFIX) as u8,
            });
        }
        self.put(symbol);
        Ok(())
    }

    /// Takes back the last character given; the cells of its column
    /// read as empty again. Cheap.
    pub fn pop(&mut self) {
        if self.given > 0 {
            self.given -= 1;
            self.symbols[usize::from(self.given)] = 0;
        }
    }

    /// Costs a checksum of one string and the columns it fills. Once
    /// every box before the checksum is given, makes the checksum and
    /// gives its boxes: the generation sheet, whose final row is the
    /// target by construction. The cells are those the booklet works
    /// from the bottom row up.
    pub fn finish(&mut self, c: &Compute) -> Result<(), Error> {
        note(c);
        let given = usize::from(self.given);
        if given + PREFIX != self.last_data_box() {
            return Err(Error::BadLength);
        }
        let len = self.checksum_len();
        let long = self.long();
        let mut r = MS32_HRP_RESIDUE;
        for &v in &self.symbols[..given] {
            r = residue_step(r, v, long);
        }
        for _ in 0..len {
            r = residue_step(r, 0, long);
        }
        let constant = if long { MS32_LONG_CONST } else { MS32_CONST };
        let checksum = digits(r ^ constant, len);
        for &v in &checksum[..len] {
            self.put(v);
        }
        Ok(())
    }

    /// Gives the next box `v` and fills its column.
    fn put(&mut self, v: u8) {
        let col = usize::from(self.given);
        self.symbols[col] = v;
        self.given += 1;
        let len = self.checksum_len();
        let long = self.long();
        if col < len {
            self.sums[0][col] = v ^ self.hrp_row()[col];
        }
        let steps = self.steps();
        for j in 0..steps {
            let next = self.start(j + 1);
            if col < next || col >= next + len {
                continue;
            }
            let i = col - next;
            let t = i + self.step_width(j);
            let above = if t < len {
                self.sums[j][t]
            } else {
                self.symbols[self.start(j) + t]
            };
            self.sums[j + 1][i] = above ^ self.lookups[j][i];
        }
        for j in 0..steps {
            let w = self.step_width(j);
            if self.start(j) + w - 1 == col {
                self.lookups[j] = lookup(&self.sums[j][..w], len, long);
            }
        }
    }
}

// ---------------------------------------------------------------------
// Translation, recovery and fusion
// ---------------------------------------------------------------------

/// The symbol a translation worksheet towards `target` gives the input
/// at index `own` for the other input at index `other`:
/// `(target − other) / (own − other)` in GF(32). Towards `s` it is the
/// booklet's recovery symbol (its Recovery table, column `own`, row
/// `other`); the product of an input's pair symbols is its translation
/// symbol. `None` where an index is not a bech32 character or `own` is
/// `other`. Cheap.
pub fn pair_symbol(own: u8, other: u8, target: u8) -> Option<u8> {
    let (i, j, x) = (charval(own)?, charval(other)?, charval(target)?);
    (i != j).then(|| gf_mul(x ^ j, super::INV[usize::from(i ^ j)]))
}

/// The translation symbols of the inputs at `indices` towards `target`,
/// into `out`: the Lagrange weights BIP 93 interpolates with, which are
/// the booklet's derivation table columns towards a new share. Refuses
/// an index outside the alphabet, an index twice, or a target among the
/// inputs. Cheap: `k²` multiplications.
pub fn translation_symbols(indices: &[u8], target: u8, out: &mut [u8]) -> Result<(), Error> {
    let mut xs = [0u8; MAX_THRESHOLD as usize];
    if indices.is_empty() || indices.len() > xs.len() || out.len() < indices.len() {
        return Err(Error::WrongShareCount);
    }
    for (n, (slot, &c)) in xs.iter_mut().zip(indices).enumerate() {
        *slot = charval(c).ok_or(Error::BadShareIndex)?;
        if indices[..n].iter().any(|&e| charval(e) == Some(*slot)) {
            return Err(Error::DuplicateIndex);
        }
    }
    let x = charval(target).ok_or(Error::BadShareIndex)?;
    let xs = &xs[..indices.len()];
    if xs.contains(&x) {
        return Err(Error::BadShareIndex);
    }
    lagrange(xs, x, &mut out[..xs.len()]);
    Ok(())
}

/// The translation worksheet towards one share index from `k` strings
/// of a set, filled as far as each input's characters are given.
///
/// It keeps the translation symbols, each input's pair symbols, the
/// inputs' characters, their translated rows and the sum row: drawing
/// reads them and computes nothing. Its invariant: a translated cell is
/// held for every box its input has given, and a sum cell for every box
/// every input has given; each is what the whole sheet of those
/// characters holds ([`Self::of`]).
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct TranslationWorksheet {
    /// Characters after `ms1` in each string.
    data_len: u8,
    /// Inputs: the threshold `k`.
    k: u8,
    /// Each input's share index, as a symbol.
    inputs: [u8; MAX_THRESHOLD as usize],
    /// The target's share index, as a symbol.
    target: u8,
    /// Each input's translation symbol.
    symbols: [u8; MAX_THRESHOLD as usize],
    /// Each input's pair symbol for each other input (0 on the diagonal).
    pairs: [[u8; MAX_THRESHOLD as usize]; MAX_THRESHOLD as usize],
    /// How many characters after `ms1` each input has given.
    filled: [u8; MAX_THRESHOLD as usize],
    /// Each input's characters after `ms1`.
    given: [[u8; MAX_DATA_LEN]; MAX_THRESHOLD as usize],
    /// Each input's characters times its translation symbol.
    translated: [[u8; MAX_DATA_LEN]; MAX_THRESHOLD as usize],
    /// The translated rows added: the new string.
    sum: [u8; MAX_DATA_LEN],
}

impl TranslationWorksheet {
    /// A blank sheet for strings of `chars` characters, from the inputs
    /// at share indices `indices` (in the order their rows are drawn)
    /// towards `target`. Refuses as [`translation_symbols`] does, and a
    /// length no master seed has. Cheap: `k²` multiplications.
    pub fn new(chars: usize, indices: &[u8], target: u8) -> Result<Self, Error> {
        if !VALID_LENGTHS.contains(&chars) {
            return Err(Error::BadLength);
        }
        let mut symbols = [0u8; MAX_THRESHOLD as usize];
        translation_symbols(indices, target, &mut symbols)?;
        let mut inputs = [0u8; MAX_THRESHOLD as usize];
        let mut pairs = [[0u8; MAX_THRESHOLD as usize]; MAX_THRESHOLD as usize];
        for (i, &own) in indices.iter().enumerate() {
            inputs[i] = charval(own).ok_or(Error::BadShareIndex)?;
            for (j, &other) in indices.iter().enumerate() {
                pairs[i][j] = pair_symbol(own, other, target).unwrap_or(0);
            }
        }
        Ok(TranslationWorksheet {
            data_len: (chars - PREFIX) as u8,
            k: indices.len() as u8,
            inputs,
            target: charval(target).ok_or(Error::BadShareIndex)?,
            symbols,
            pairs,
            filled: [0u8; MAX_THRESHOLD as usize],
            given: [[0u8; MAX_DATA_LEN]; MAX_THRESHOLD as usize],
            translated: [[0u8; MAX_DATA_LEN]; MAX_THRESHOLD as usize],
            sum: [0u8; MAX_DATA_LEN],
        })
    }

    /// Costs an interpolation of `k` strings. The whole sheet from
    /// exactly `k` strings of one set towards a share index none of them
    /// carries, `s` included where the secret is not among them.
    pub fn of(c: &Compute, strings: &[Codex32], target: u8) -> Result<Self, Error> {
        note(c);
        check_set(strings, true)?;
        if strings.len() != usize::from(strings[0].threshold()) {
            return Err(Error::WrongShareCount);
        }
        let mut indices = [0u8; MAX_THRESHOLD as usize];
        for (slot, s) in indices.iter_mut().zip(strings) {
            *slot = s.share_index();
        }
        let chars = usize::from(strings[0].len) + PREFIX;
        let mut sheet = Self::new(chars, &indices[..strings.len()], target)?;
        for (i, s) in strings.iter().enumerate() {
            for &v in &s.data[..usize::from(s.len)] {
                sheet.put(i, v);
            }
        }
        Ok(sheet)
    }

    /// The threshold: how many inputs the sheet has.
    pub fn threshold(&self) -> usize {
        usize::from(self.k)
    }

    /// The share index of input `i`, lowercase.
    pub fn input_index(&self, i: usize) -> u8 {
        super::CHARSET[usize::from(self.inputs[i])]
    }

    /// The share index the sheet makes, lowercase.
    pub fn target_index(&self) -> u8 {
        super::CHARSET[usize::from(self.target)]
    }

    /// Whether the sheet makes the secret, from shares: the booklet's
    /// recovery, whose pair symbols are its recovery symbols.
    pub fn is_recovery(&self) -> bool {
        self.target == SECRET_INDEX
    }

    /// Input `i`'s translation symbol, a value `1..32`; its glyph is
    /// [`SYMBOL_GLYPHS`]`[v]`.
    pub fn symbol(&self, i: usize) -> u8 {
        self.symbols[i]
    }

    /// Input `i`'s pair symbol for input `j` (`j` not `i`), whose
    /// product over every `j` is input `i`'s symbol.
    pub fn pair(&self, i: usize, j: usize) -> u8 {
        self.pairs[i][j]
    }

    /// Input `i`'s character at box `b`, if given.
    pub fn given_cell(&self, i: usize, b: usize) -> Option<u8> {
        let col = b.checked_sub(PREFIX + 1)?;
        (col < usize::from(self.filled[i])).then(|| self.given[i][col])
    }

    /// Input `i`'s character at box `b` times its symbol, if given.
    pub fn translated_cell(&self, i: usize, b: usize) -> Option<u8> {
        let col = b.checked_sub(PREFIX + 1)?;
        (col < usize::from(self.filled[i])).then(|| self.translated[i][col])
    }

    /// The new string's character at box `b`, once every input has
    /// given that box.
    pub fn sum_cell(&self, b: usize) -> Option<u8> {
        let col = b.checked_sub(PREFIX + 1)?;
        let all = self.filled[..self.threshold()].iter().min().copied();
        (col < usize::from(all.unwrap_or(0))).then(|| self.sum[col])
    }

    /// Costs a column of the sheet. Gives input `i` its next character
    /// (a symbol, `0..32`), translating it, and the sum cell of that box
    /// where every input now has it.
    pub fn push(&mut self, c: &Compute, i: usize, symbol: u8) -> Result<(), Error> {
        note(c);
        if i >= self.threshold() {
            return Err(Error::WrongShareCount);
        }
        let col = usize::from(self.filled[i]);
        if col >= usize::from(self.data_len) {
            return Err(Error::BadLength);
        }
        if symbol >= 32 {
            return Err(Error::BadCharacter {
                position: (col + PREFIX) as u8,
            });
        }
        self.put(i, symbol);
        Ok(())
    }

    /// Takes back input `i`'s last character; its translated cell and
    /// the sum cell of its box read as empty again. Cheap.
    pub fn pop(&mut self, i: usize) {
        if i < self.threshold() && self.filled[i] > 0 {
            self.filled[i] -= 1;
            let col = usize::from(self.filled[i]);
            self.given[i][col] = 0;
            self.translated[i][col] = 0;
        }
    }

    fn put(&mut self, i: usize, v: u8) {
        let col = usize::from(self.filled[i]);
        self.given[i][col] = v;
        self.translated[i][col] = gf_mul(v, self.symbols[i]);
        self.filled[i] += 1;
        let k = self.threshold();
        if self.filled[..k].iter().all(|&f| usize::from(f) > col) {
            self.sum[col] = self.translated[..k]
                .iter()
                .fold(0, |acc, row| acc ^ row[col]);
        }
    }

    /// The string the sheet makes, once every input has given every box
    /// and the sum's checksum holds (it does by construction when every
    /// input's does). Cheap: one checksum.
    pub fn result(&self) -> Option<Codex32> {
        let len = usize::from(self.data_len);
        let k = self.threshold();
        if self.filled[..k].iter().any(|&f| usize::from(f) < len) {
            return None;
        }
        let mut data = [0u8; MAX_DATA_LEN];
        data[..len].copy_from_slice(&self.sum[..len]);
        verify_checksum(&data[..len]).then_some(Codex32 {
            data,
            len: len as u8,
        })
    }
}
