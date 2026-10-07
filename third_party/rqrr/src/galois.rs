//! GF(2^8) for the Reed-Solomon decoder.
//!
//! Upstream generated this field, and a second GF(2^4) for the format
//! information, with the `g2p` proc macro. This module is the same
//! field: the QR code field of ISO/IEC 18004, polynomials over GF(2)
//! reduced by x^8 + x^4 + x^3 + x^2 + 1 (`0b1_0001_1101`), with 2 as the
//! generator of the multiplicative group — the element `g2p`'s
//! `find_generator` returns for this modulus, being the smallest one
//! that generates all 255 non-zero elements.
//!
//! The tables are built at compile time, as upstream's were.

/// x^8 + x^4 + x^3 + x^2 + 1, the QR code field's modulus.
const MODULUS: u16 = 0b1_0001_1101;

/// `EXP[i]` is the generator to the power `i`, for `i` in 0..255, with
/// the table doubled so that a sum of two logarithms needs no reduction.
const EXP: [u8; 510] = {
    let mut table = [0u8; 510];
    let mut value: u16 = 1;
    let mut i = 0;
    while i < 255 {
        table[i] = value as u8;
        table[i + 255] = value as u8;
        value <<= 1;
        if value & 0x100 != 0 {
            value ^= MODULUS;
        }
        i += 1;
    }
    table
};

/// `LOG[x]` is the power of the generator that equals `x`. `LOG[0]` is
/// never read: every caller tests for zero first.
const LOG: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 255 {
        table[EXP[i] as usize] = i as u8;
        i += 1;
    }
    table
};

/// An element of GF(2^8).
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct GF256(pub u8);

impl GF256 {
    pub const ZERO: GF256 = GF256(0);
    pub const ONE: GF256 = GF256(1);
    pub const GENERATOR: GF256 = GF256(2);

    /// This element raised to the power `p`.
    pub fn pow(self, p: usize) -> GF256 {
        if self == GF256::ZERO {
            return if p == 0 { GF256::ONE } else { GF256::ZERO };
        }
        let log = LOG[self.0 as usize] as usize;
        GF256(EXP[(log * p) % 255])
    }
}

impl core::ops::Add for GF256 {
    type Output = GF256;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: GF256) -> GF256 {
        GF256(self.0 ^ rhs.0)
    }
}

impl core::ops::AddAssign for GF256 {
    fn add_assign(&mut self, rhs: GF256) {
        *self = *self + rhs;
    }
}

impl core::ops::Mul for GF256 {
    type Output = GF256;

    fn mul(self, rhs: GF256) -> GF256 {
        if self == GF256::ZERO || rhs == GF256::ZERO {
            return GF256::ZERO;
        }
        let a = LOG[self.0 as usize] as usize;
        let b = LOG[rhs.0 as usize] as usize;
        GF256(EXP[a + b])
    }
}

impl core::ops::MulAssign for GF256 {
    fn mul_assign(&mut self, rhs: GF256) {
        *self = *self * rhs;
    }
}

impl core::ops::Div for GF256 {
    type Output = GF256;

    /// Division, with division by zero reading as zero.
    ///
    /// Upstream's generated field panicked there. The two call sites
    /// test the divisor first — `correct_block` returns a decode error
    /// when sigma's derivative vanishes, and Berlekamp-Massey's `b` is
    /// one or the last non-zero discrepancy — so the value is never
    /// used; this keeps a frame from ever reaching a panic through it.
    fn div(self, rhs: GF256) -> GF256 {
        if rhs == GF256::ZERO || self == GF256::ZERO {
            return GF256::ZERO;
        }
        let a = LOG[self.0 as usize] as usize;
        let b = LOG[rhs.0 as usize] as usize;
        GF256(EXP[a + 255 - b])
    }
}
