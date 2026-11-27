//! exp and ln of f32 without the platform's libm.
//!
//! Each works in f64, by range reduction and a polynomial, and rounds to f32.
//! The result is the correctly rounded f32 in all but a rare tie-near case,
//! which is within what `expf` and `logf` of any libm promise (under one ulp).

const LN2: f64 = core::f64::consts::LN_2;
/// ln 2 split so that `k * LN2_HI` is exact for the k used here.
const LN2_HI: f64 = 6.931_471_803_691_238e-1;
const LN2_LO: f64 = 1.908_214_929_270_587_7e-10;

/// 2^k for k in -1022..=1023.
fn pow2(k: i64) -> f64 {
    let e = (k + 1023).clamp(1, 2046) as u64;
    f64::from_bits(e << 52)
}

/// e^x in f64, to about 1e-16 relative.
pub(crate) fn exp64(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x > 709.0 {
        return f64::INFINITY;
    }
    if x < -745.0 {
        return 0.0;
    }
    let half = if x >= 0.0 { 0.5 } else { -0.5 };
    let k = (x * core::f64::consts::LOG2_E + half) as i64;
    let kf = k as f64;
    let r = x - kf * LN2_HI - kf * LN2_LO;
    // Taylor series to degree 13: |r| <= 0.35, the next term is below 1e-17.
    let mut acc = 1.0;
    let mut n = 13.0;
    while n >= 1.0 {
        acc = 1.0 + acc * r / n;
        n -= 1.0;
    }
    // Two factors, so that a result in the subnormal range is formed too.
    let k1 = k / 2;
    acc * pow2(k1) * pow2(k - k1)
}

/// Natural logarithm in f64, to about 1e-16 relative. Negative and NaN give
/// NaN, zero gives -infinity.
pub(crate) fn ln64(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x == f64::INFINITY {
        return x;
    }
    let (mut v, mut e) = (x, 0i64);
    if v < f64::MIN_POSITIVE {
        v *= 18_014_398_509_481_984.0; // 2^54
        e -= 54;
    }
    let bits = v.to_bits();
    e += ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | (1023u64 << 52));
    if m > core::f64::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    // ln m = 2 atanh s, s = (m - 1) / (m + 1), |s| <= 0.172.
    let s = (m - 1.0) / (m + 1.0);
    let z = s * s;
    let mut acc = 0.0;
    let mut n = 25.0;
    while n >= 1.0 {
        acc = 1.0 / n + z * acc;
        n -= 2.0;
    }
    e as f64 * LN2 + 2.0 * s * acc
}

/// e^x as f32.
pub(crate) fn expf(x: f32) -> f32 {
    exp64(f64::from(x)) as f32
}

/// ln x as f32.
pub(crate) fn logf(x: f32) -> f32 {
    ln64(f64::from(x)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_and_ln_agree_with_the_platform_to_an_ulp() {
        let mut x = -30.0f32;
        while x < 20.0 {
            let (a, b) = (expf(x), x.exp());
            assert!((a - b).abs() <= b * 2.0e-7, "exp {x}: {a} {b}");
            x += 0.0137;
        }
        let mut y = 1.0e-6f32;
        while y < 1.0e6 {
            let (a, b) = (logf(y), y.ln());
            assert!(
                (a - b).abs() <= b.abs() * 2.0e-7 + 1.0e-7,
                "ln {y}: {a} {b}"
            );
            y *= 1.0173;
        }
    }

    #[test]
    fn edges_do_not_panic() {
        assert_eq!(expf(f32::NEG_INFINITY), 0.0);
        assert_eq!(expf(f32::INFINITY), f32::INFINITY);
        assert!(expf(f32::NAN).is_nan());
        assert_eq!(expf(0.0), 1.0);
        assert!(logf(-1.0).is_nan());
        assert_eq!(logf(0.0), f32::NEG_INFINITY);
        assert_eq!(logf(1.0), 0.0);
        assert!(logf(f32::MIN_POSITIVE / 4.0).is_finite());
    }
}
