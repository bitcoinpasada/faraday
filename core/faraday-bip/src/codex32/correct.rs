//! The decoder of `docs/CODEX32.md` §8.4: a codex32 string whose
//! checksum fails, with some boxes perhaps marked `?` as unreadable, and
//! the one correction BIP 93's checksum can justify, or a refusal.
//!
//! BIP 93, "Error Correction": the checksum "can correct up to 4
//! character substitutions, up to 8 unreadable characters (called
//! erasures), or up to 13 consecutive erasures". Both checksums are BCH
//! codes whose generators have eight consecutive roots, so any two
//! strings that both hold differ in at least 9 boxes. This decoder
//! corrects:
//!
//! - `s` substituted boxes and `e` unreadable ones together where
//!   `2s + e` is at most 8, at every length. Within that bound a
//!   correction is unique: a second string within it would lie within 8
//!   boxes of the first.
//! - A run of 9 to 13 unreadable boxes in a row (9 to 15 on the long
//!   checksum of a 127-character string) where every other box is
//!   right. The run's characters are the one solution of the checksum's
//!   13 (or 15) equations, which are independent over any run that
//!   long; nothing checks the other boxes, so the proposal says it
//!   assumes them ([`Proposal::assumes_the_rest`]).
//!
//! Anything else is refused: more unreadable boxes than that, or not in
//! one run; more substitutions than the bound; a correction that would
//! change `ms1` or, where earlier strings of the set are in, the
//! threshold and identifier boxes they fix. Insertions, deletions and
//! swapped boxes are not searched for. A character outside the bech32
//! alphabet other than `?` is refused as it is by [`Codex32::parse`],
//! not read as unreadable.
//!
//! The decoder only proposes. Its result is written out and read again
//! by [`Codex32::parse`], which stays the only way in, and the screens
//! apply nothing without the person (§8.4).
//!
//! The arithmetic: syndromes are the failing residue evaluated at the
//! generator's eight consecutive roots in GF(1024); erasures are taken
//! out of them (Forney syndromes), Berlekamp–Massey finds the
//! substituted boxes' locator, a search of every box finds its roots,
//! and Forney's formula gives each box's change. A long run is solved
//! directly from the residue by Gaussian elimination over GF(32). The
//! roots and the field are checked in this file's tests; the decoder is
//! checked against python-codex32's frozen correction corpus and, in the
//! crypto cross-check, against its decoder (`docs/CODEX32.md` §13.5).

use faraday_crypto::work::Compute;
use faraday_crypto::{Zeroize, ZeroizeOnDrop};

use super::{
    CHARSET, CHECKSUM_LEN, Codex32, Error, HEADER_LEN, INV, LONG_CHECKSUM_LEN, MAX_DATA_LEN,
    MS32_CONST, MS32_LONG_CONST, VALID_LENGTHS, charval, gf_mul, long_polymod, note, polymod,
    residue_step, verify_checksum,
};

/// What BIP 93's bound allows: twice the substitutions plus the
/// unreadable boxes, at most this.
pub const CORRECTION_BOUND: usize = 8;
/// The most boxes one proposal changes: a run as long as the long
/// checksum.
pub const MAX_EDITS: usize = LONG_CHECKSUM_LEN;
/// The character that marks a box as unreadable.
pub const UNREADABLE: u8 = b'?';

// ---------------------------------------------------------------------
// GF(1024)
// ---------------------------------------------------------------------

/// GF(1024) as GF(32)[ζ] with ζ² = ζ + 1, an element `a + bζ` packed as
/// `a | b << 5`; GF(32) is BIP 93's, the elements with `b` = 0.
const fn mul32(a: u8, b: u8) -> u8 {
    let mut a = a;
    let mut res = 0u8;
    let mut i = 0;
    while i < 5 {
        if (b >> i) & 1 == 1 {
            res ^= a;
        }
        a <<= 1;
        if a >= 32 {
            a ^= 41;
        }
        i += 1;
    }
    res
}

const fn mul1024_raw(x: u16, y: u16) -> u16 {
    let (a0, b0) = ((x & 31) as u8, (x >> 5) as u8);
    let (a1, b1) = ((y & 31) as u8, (y >> 5) as u8);
    let bb = mul32(b0, b1);
    let lo = mul32(a0, a1) ^ bb;
    let hi = mul32(a0, b1) ^ mul32(a1, b0) ^ bb;
    lo as u16 | (hi as u16) << 5
}

/// The multiplicative group's order.
const ORDER: usize = 1023;
/// `2 + ζ`, a generator of GF(1024)'s multiplicative group.
const PRIMITIVE: u16 = 2 | 1 << 5;

/// Powers and logarithms of [`PRIMITIVE`]; the powers run twice round
/// so that a product is one lookup without a reduction.
struct Field {
    exp: [u16; 2 * ORDER],
    log: [u16; 1024],
}

const FIELD: Field = {
    let mut exp = [0u16; 2 * ORDER];
    let mut log = [0u16; 1024];
    let mut v = 1u16;
    let mut i = 0;
    while i < ORDER {
        exp[i] = v;
        exp[i + ORDER] = v;
        log[v as usize] = i as u16;
        v = mul1024_raw(v, PRIMITIVE);
        i += 1;
    }
    Field { exp, log }
};

fn mul(a: u16, b: u16) -> u16 {
    if a == 0 || b == 0 {
        0
    } else {
        FIELD.exp[usize::from(FIELD.log[usize::from(a)]) + usize::from(FIELD.log[usize::from(b)])]
    }
}

/// The inverse of a nonzero element.
fn inv(a: u16) -> u16 {
    FIELD.exp[ORDER - usize::from(FIELD.log[usize::from(a)])]
}

/// The element whose logarithm is `e`, any `e`.
fn exp(e: usize) -> u16 {
    FIELD.exp[e % ORDER]
}

fn log(a: u16) -> usize {
    usize::from(FIELD.log[usize::from(a)])
}

// ---------------------------------------------------------------------
// The two codes
// ---------------------------------------------------------------------

/// One checksum as a BCH code: its generator's roots are `β^first` to
/// `β^(first + 7)`, and the box `r` places from the end of the data
/// part is located by `β^r`.
struct Code {
    /// Checksum characters, the generator's degree.
    checksum: usize,
    /// `β`, packed.
    beta: u16,
    /// The exponent of the first of the eight consecutive roots.
    first: usize,
    /// Whether this is the long checksum.
    long: bool,
}

/// The regular checksum: `β` = 8ζ, of order 93, roots `β^77` to `β^84`.
const SHORT: Code = Code {
    checksum: CHECKSUM_LEN,
    beta: 8 << 5,
    first: 77,
    long: false,
};

/// The long checksum: `β` = 25 + 6ζ, of order 1023, roots `β^1019` to
/// `β^1026`.
const LONG: Code = Code {
    checksum: LONG_CHECKSUM_LEN,
    beta: 25 | 6 << 5,
    first: 1019,
    long: true,
};

/// Syndromes, one a root.
const SYNDROMES: usize = 8;
/// Room for a polynomial of degree 16 over GF(1024): the products below
/// stay within it.
const POLY: usize = 17;

impl Code {
    fn for_data_len(n: usize) -> &'static Code {
        if super::HRP_EXPANDED_LEN + n >= 96 {
            &LONG
        } else {
            &SHORT
        }
    }

    /// The logarithm of the locator of the box `r` places from the end.
    fn locator_log(&self, r: usize) -> usize {
        log(self.beta) * r % ORDER
    }

    /// The residue that says how far the data part is from holding:
    /// zero where it holds, and otherwise the error's residue modulo the
    /// generator, as a GF(32) polynomial, coefficient `k` at `k`.
    fn residue(&self, data: &[u8]) -> [u8; LONG_CHECKSUM_LEN] {
        let r = if self.long {
            long_polymod(data, 0) ^ MS32_LONG_CONST
        } else {
            polymod(data, 0) ^ MS32_CONST
        };
        let mut out = [0u8; LONG_CHECKSUM_LEN];
        for (k, slot) in out.iter_mut().take(self.checksum).enumerate() {
            *slot = ((r >> (5 * k)) & 31) as u8;
        }
        out
    }

    /// The residue evaluated at the eight consecutive roots.
    fn syndromes(&self, residue: &[u8]) -> [u16; SYNDROMES] {
        let mut out = [0u16; SYNDROMES];
        for (j, slot) in out.iter_mut().enumerate() {
            let root = exp(log(self.beta) * (self.first + j));
            let mut acc = 0u16;
            for &coef in residue[..self.checksum].iter().rev() {
                acc = mul(acc, root) ^ u16::from(coef);
            }
            *slot = acc;
        }
        out
    }
}

// ---------------------------------------------------------------------
// Polynomials over GF(1024), lowest coefficient first
// ---------------------------------------------------------------------

fn eval(p: &[u16], x: u16) -> u16 {
    p.iter().rev().fold(0, |acc, &c| mul(acc, x) ^ c)
}

/// `a · b`, cut to its first `keep` coefficients.
fn product(a: &[u16], b: &[u16], keep: usize) -> [u16; POLY] {
    let mut out = [0u16; POLY];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            if i + j < keep.min(POLY) {
                out[i + j] ^= mul(x, y);
            }
        }
    }
    out
}

/// The formal derivative: in characteristic 2, the odd terms moved down.
fn derivative(p: &[u16]) -> [u16; POLY] {
    let mut out = [0u16; POLY];
    for i in (1..p.len()).step_by(2) {
        out[i - 1] = p[i];
    }
    out
}

/// Berlekamp–Massey: the shortest linear recurrence that generates `t`,
/// as its connection polynomial and its length.
fn berlekamp_massey(t: &[u16]) -> ([u16; POLY], usize) {
    let mut c = [0u16; POLY];
    let mut b = [0u16; POLY];
    c[0] = 1;
    b[0] = 1;
    let (mut l, mut m, mut last) = (0usize, 1usize, 1u16);
    for (n, &tn) in t.iter().enumerate() {
        let mut d = tn;
        for (i, &ci) in c.iter().enumerate().take(l + 1).skip(1) {
            d ^= mul(ci, t[n - i]);
        }
        if d == 0 {
            m += 1;
            continue;
        }
        let before = c;
        let coef = mul(d, inv(last));
        for (ci, &bi) in c[m..].iter_mut().zip(&b) {
            *ci ^= mul(coef, bi);
        }
        if 2 * l <= n {
            l = n + 1 - l;
            b = before;
            last = d;
            m = 1;
        } else {
            m += 1;
        }
    }
    (c, l)
}

// ---------------------------------------------------------------------
// The proposal
// ---------------------------------------------------------------------

/// One box a proposal changes.
#[derive(Default, Zeroize)]
pub struct Edit {
    position: u8,
    was: u8,
    now: u8,
}

impl Edit {
    /// The box, counted from 1 at `m` (`docs/CODEX32.md` §3).
    pub fn position(&self) -> usize {
        usize::from(self.position)
    }

    /// The character the string had there, lowercase, or `?` where it
    /// was unreadable.
    pub fn was(&self) -> u8 {
        self.was
    }

    /// The character the proposal puts there, lowercase.
    pub fn now(&self) -> u8 {
        self.now
    }

    /// Whether the box was marked unreadable.
    pub fn was_unreadable(&self) -> bool {
        self.was == UNREADABLE
    }
}

/// The one correction of a string the checksum justifies: the string
/// it proposes, parsed, and the boxes it changes in order. Wiped on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Proposal {
    string: Codex32,
    edits: [Edit; MAX_EDITS],
    count: u8,
    run: bool,
}

impl Proposal {
    /// The string proposed, which [`Codex32::parse`] has read.
    pub fn string(&self) -> &Codex32 {
        &self.string
    }

    /// The boxes it changes, first box first; none where the string
    /// given already held.
    pub fn edits(&self) -> &[Edit] {
        &self.edits[..usize::from(self.count)]
    }

    /// Boxes changed that were readable: substitutions.
    pub fn substitutions(&self) -> usize {
        self.edits().iter().filter(|e| !e.was_unreadable()).count()
    }

    /// Boxes filled that were marked unreadable.
    pub fn erasures(&self) -> usize {
        self.edits().iter().filter(|e| e.was_unreadable()).count()
    }

    /// Whether this fills a run of more than 8 unreadable boxes, which
    /// holds only if every other box is right: nothing is left over to
    /// check them (§8.4's caution, "This assumes every other box is
    /// right").
    pub fn assumes_the_rest(&self) -> bool {
        self.run
    }
}

/// Why no correction is proposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The text is not a codex32 string with some boxes unreadable: a
    /// character outside printable ASCII, mixed case, no `ms1`, a length
    /// no master seed has, or a character that is neither bech32 nor `?`.
    Malformed(Error),
    /// The threshold and identifier boxes are unreadable or differ from
    /// those of the strings already in, which fix them.
    HeaderFixed,
    /// More unreadable boxes than can be filled: over 8, and not one run
    /// of at most the checksum's length.
    TooManyUnreadable,
    /// No correction within the bound: more boxes are wrong than the
    /// checksum can repair, or the one correction would change a box
    /// that is fixed.
    NotFound,
    /// The one correction within the bound is not a codex32 string
    /// [`Codex32::parse`] takes.
    NotAString(Error),
}

/// A string as the decoder reads it: the data part's symbols with the
/// unreadable boxes at zero, and which they are.
struct Damaged {
    data: [u8; MAX_DATA_LEN],
    unreadable: [bool; MAX_DATA_LEN],
    len: usize,
}

impl Zeroize for Damaged {
    fn zeroize(&mut self) {
        self.data.zeroize();
        self.unreadable.zeroize();
    }
}

impl Drop for Damaged {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Damaged {
    /// Reads the text as [`Codex32::parse`] does, with `?` a box of its
    /// own.
    fn read(text: &str) -> Result<Self, Error> {
        let b = text.as_bytes();
        if b.iter().any(|&c| !(33..=126).contains(&c)) {
            return Err(Error::NotPrintable);
        }
        if b.iter().any(u8::is_ascii_lowercase) && b.iter().any(u8::is_ascii_uppercase) {
            return Err(Error::MixedCase);
        }
        if b.len() < 3
            || !b[0].eq_ignore_ascii_case(&b'm')
            || !b[1].eq_ignore_ascii_case(&b's')
            || b[2] != b'1'
        {
            return Err(Error::BadPrefix);
        }
        if !VALID_LENGTHS.contains(&b.len()) {
            return Err(Error::BadLength);
        }
        let mut out = Damaged {
            data: [0u8; MAX_DATA_LEN],
            unreadable: [false; MAX_DATA_LEN],
            len: b.len() - 3,
        };
        for (i, &c) in b[3..].iter().enumerate() {
            if c == UNREADABLE {
                out.unreadable[i] = true;
            } else {
                out.data[i] = charval(c).ok_or(Error::BadCharacter {
                    position: (i + 3) as u8,
                })?;
            }
        }
        Ok(out)
    }

    fn erased(&self) -> usize {
        self.unreadable[..self.len].iter().filter(|&&u| u).count()
    }

    /// Whether the unreadable boxes are one run, one after another.
    fn one_run(&self) -> bool {
        let marks = &self.unreadable[..self.len];
        match (
            marks.iter().position(|&u| u),
            marks.iter().rposition(|&u| u),
        ) {
            (Some(a), Some(z)) => marks[a..=z].iter().all(|&u| u),
            _ => true,
        }
    }
}

/// Costs a decode of one string: its residue and syndromes, a
/// Berlekamp–Massey run over at most 8 of them and a search of every box
/// for the changed ones, or, for a run of more than 8 unreadable boxes,
/// a linear solve of at most 15 unknowns. The one correction BIP 93's
/// checksum justifies for `text`, a codex32 string in which `?` marks a
/// box that cannot be read.
///
/// `held` is a string of the same set already in, whose threshold and
/// identifier boxes (4 to 8) are fixed: `text` must carry them, and no
/// correction changes them. `ms1` is never changed.
///
/// A string that already holds and has no `?` is proposed as it is,
/// with no edits.
pub fn correct(c: &Compute, text: &str, held: Option<&Codex32>) -> Result<Proposal, Refusal> {
    note(c);
    let damaged = Damaged::read(text).map_err(Refusal::Malformed)?;
    let n = damaged.len;
    let fixed = if let Some(h) = held {
        if damaged.unreadable[..HEADER_LEN - 1].iter().any(|&u| u)
            || damaged.data[..HEADER_LEN - 1] != h.data[..HEADER_LEN - 1]
        {
            return Err(Refusal::HeaderFixed);
        }
        HEADER_LEN - 1
    } else {
        0
    };
    let code = Code::for_data_len(n);
    let erased = damaged.erased();
    let (changes, run) = if erased <= CORRECTION_BOUND {
        (bch(code, &damaged, fixed).ok_or(Refusal::NotFound)?, false)
    } else if erased <= code.checksum && damaged.one_run() {
        (run_of(code, &damaged).ok_or(Refusal::NotFound)?, true)
    } else {
        return Err(Refusal::TooManyUnreadable);
    };
    proposal(&damaged, &changes, fixed, run)
}

/// The changes found: for each data box, the value added to it.
struct Changes([u8; MAX_DATA_LEN]);

impl Drop for Changes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Errors and erasures within `2s + e ≤ 8`, by the BCH decoder.
fn bch(code: &Code, d: &Damaged, fixed: usize) -> Option<Changes> {
    let n = d.len;
    let syndromes = code.syndromes(&code.residue(&d.data[..n]));

    // The erasure locator Γ(x) = Π (1 − X_k x).
    let mut gamma = [0u16; POLY];
    gamma[0] = 1;
    let mut e = 0usize;
    for i in (0..n).filter(|&i| d.unreadable[i]) {
        let x = exp(code.locator_log(n - 1 - i));
        gamma = product(&gamma[..=e], &[1, x], e + 2);
        e += 1;
    }

    // Forney syndromes: the erasures taken out, 8 − e left.
    let modified = product(&syndromes, &gamma[..=e], SYNDROMES);
    let (lambda, l) = berlekamp_massey(&modified[e..SYNDROMES]);
    if 2 * l + e > CORRECTION_BOUND {
        return None;
    }

    // Every box whose locator is a root of Λ is a substitution; there
    // must be exactly l of them, none fixed and none unreadable.
    let mut found = 0usize;
    let mut wrong = [false; MAX_DATA_LEN];
    for (i, w) in wrong.iter_mut().enumerate().take(n) {
        let x_inv = exp(ORDER - code.locator_log(n - 1 - i));
        if eval(&lambda[..=l], x_inv) == 0 {
            if i < fixed || d.unreadable[i] {
                return None;
            }
            *w = true;
            found += 1;
        }
    }
    if found != l {
        return None;
    }

    // Forney: each box's change from Ω = SΨ mod x^8 and Ψ = ΛΓ.
    let psi = product(&lambda[..=l], &gamma[..=e], POLY);
    let omega = product(&syndromes, &psi[..=l + e], SYNDROMES);
    let dpsi = derivative(&psi[..=l + e]);
    let mut changes = Changes([0u8; MAX_DATA_LEN]);
    for i in (0..n).filter(|&i| wrong[i] || d.unreadable[i]) {
        let xl = code.locator_log(n - 1 - i);
        let x_inv = exp(ORDER - xl);
        let den = eval(&dpsi[..l + e], x_inv);
        if den == 0 {
            return None;
        }
        let scale = exp(xl * (1 + ORDER - code.first % ORDER));
        let value = mul(mul(eval(&omega[..SYNDROMES], x_inv), scale), inv(den));
        if value >> 5 != 0 || (wrong[i] && value == 0) {
            return None;
        }
        changes.0[i] = value as u8;
    }
    Some(changes)
}

/// A run of unreadable boxes longer than 8, by solving the checksum's
/// equations for them; every other box is taken as right.
fn run_of(code: &Code, d: &Damaged) -> Option<Changes> {
    let n = d.len;
    let k = code.checksum;
    let target = code.residue(&d.data[..n]);
    let boxes: [usize; MAX_EDITS] = {
        let mut out = [0usize; MAX_EDITS];
        for (slot, i) in out.iter_mut().zip((0..n).filter(|&i| d.unreadable[i])) {
            *slot = i;
        }
        out
    };
    let e = d.erased();

    // Column j: x^r mod g for the box r places from the end, the
    // residue a 1 in that box would leave.
    let mut m = [[0u8; MAX_EDITS + 1]; LONG_CHECKSUM_LEN];
    for (j, &i) in boxes[..e].iter().enumerate() {
        let mut r = 1u128;
        for _ in 0..n - 1 - i {
            r = residue_step(r, 0, code.long);
        }
        for (row, cells) in m.iter_mut().take(k).enumerate() {
            cells[j] = ((r >> (5 * row)) & 31) as u8;
        }
    }
    for (row, cells) in m.iter_mut().take(k).enumerate() {
        cells[e] = target[row];
    }

    // Gaussian elimination over GF(32).
    for col in 0..e {
        let pivot = (col..k).find(|&r| m[r][col] != 0)?;
        m.swap(col, pivot);
        let scale = INV[usize::from(m[col][col])];
        for cell in m[col].iter_mut().take(e + 1) {
            *cell = gf_mul(*cell, scale);
        }
        let pivot_row = m[col];
        for (r, row) in m.iter_mut().enumerate().take(k) {
            let f = row[col];
            if r != col && f != 0 {
                for (cell, &p) in row.iter_mut().zip(&pivot_row).take(e + 1) {
                    *cell ^= gf_mul(f, p);
                }
            }
        }
    }
    // The rows left over must read zero: the equations agree.
    if m[e..k].iter().any(|row| row[e] != 0) {
        return None;
    }
    let mut changes = Changes([0u8; MAX_DATA_LEN]);
    for (j, &i) in boxes[..e].iter().enumerate() {
        changes.0[i] = m[j][e];
    }
    m.zeroize();
    Some(changes)
}

/// The corrected string, checked to hold and to stay within the bound,
/// read again by [`Codex32::parse`], with its edits.
fn proposal(d: &Damaged, changes: &Changes, fixed: usize, run: bool) -> Result<Proposal, Refusal> {
    let n = d.len;
    let mut data = [0u8; MAX_DATA_LEN];
    let mut edits: [Edit; MAX_EDITS] = Default::default();
    let mut count = 0usize;
    let mut substituted = 0usize;
    for i in 0..n {
        data[i] = d.data[i] ^ changes.0[i];
        if !d.unreadable[i] && changes.0[i] == 0 {
            continue;
        }
        if i < fixed || count == MAX_EDITS {
            data.zeroize();
            return Err(Refusal::NotFound);
        }
        if !d.unreadable[i] {
            substituted += 1;
        }
        edits[count] = Edit {
            position: (i + 4) as u8,
            was: if d.unreadable[i] {
                UNREADABLE
            } else {
                CHARSET[usize::from(d.data[i])]
            },
            now: CHARSET[usize::from(data[i])],
        };
        count += 1;
    }
    let erased = d.erased();
    let within = if run {
        substituted == 0
    } else {
        2 * substituted + erased <= CORRECTION_BOUND
    };
    if !within || !verify_checksum(&data[..n]) {
        data.zeroize();
        return Err(Refusal::NotFound);
    }

    let mut text = [0u8; super::MAX_STRING_LEN];
    text[..3].copy_from_slice(b"ms1");
    for (slot, &v) in text[3..3 + n].iter_mut().zip(&data[..n]) {
        *slot = CHARSET[usize::from(v)];
    }
    data.zeroize();
    let parsed = core::str::from_utf8(&text[..3 + n])
        .map_err(|_| Error::NotPrintable)
        .and_then(Codex32::parse);
    text.zeroize();
    let string = parsed.map_err(Refusal::NotAString)?;
    Ok(Proposal {
        string,
        edits,
        count: count as u8,
        run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator of a checksum as a GF(1024) polynomial, lowest
    /// coefficient first: `x^n` plus the low coefficients that
    /// `residue_step` adds for a carried-out `x^n`.
    fn generator(code: &Code) -> [u16; POLY] {
        // x^n mod g is the residue of feeding a 1 then n zeros; g is
        // x^n plus that.
        let mut r = 1u128;
        for _ in 0..code.checksum {
            r = residue_step(r, 0, code.long);
        }
        let mut g = [0u16; POLY];
        for (k, slot) in g.iter_mut().take(code.checksum).enumerate() {
            *slot = ((r >> (5 * k)) & 31) as u16;
        }
        g[code.checksum] = 1;
        g
    }

    #[test]
    fn the_field_has_a_generator() {
        let mut seen = [false; 1024];
        for e in 0..ORDER {
            let v = exp(e);
            assert!(v != 0 && !seen[usize::from(v)]);
            seen[usize::from(v)] = true;
        }
        // GF(32) sits in it as the elements with no ζ part.
        for a in 0..32u8 {
            for b in 0..32u8 {
                assert_eq!(mul(u16::from(a), u16::from(b)), u16::from(gf_mul(a, b)));
            }
        }
    }

    #[test]
    fn each_generator_has_eight_consecutive_roots() {
        for (code, period) in [(&SHORT, 93usize), (&LONG, 1023)] {
            let g = generator(code);
            for j in 0..SYNDROMES {
                let root = exp(log(code.beta) * (code.first + j));
                assert_eq!(eval(&g[..=code.checksum], root), 0, "root {j}");
            }
            // β's order is the code's period, longer than any data part.
            assert_eq!(exp(log(code.beta) * period), 1);
            for p in 1..period {
                if period % p == 0 {
                    assert_ne!(exp(log(code.beta) * p), 1);
                }
            }
        }
    }
}
