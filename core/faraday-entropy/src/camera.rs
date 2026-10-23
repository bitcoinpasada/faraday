//! Camera noise: the digests of a run of camera frames.

use crate::{Error, RawEntropy, Strength, Warnings};
use faraday_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// Most frames an accumulator holds.
pub const MAX_FRAMES: usize = 8;

/// Frames asked for at 128 bits.
///
/// A camera sensor's read noise is worth a fraction of a bit per pixel
/// and a frame carries hundreds of thousands of pixels, so one frame
/// already holds far more than 256 bits — if the sensor is what it
/// claims to be. The count is not an entropy calculation, then, but a
/// guard against a single frame that happens to be still: three
/// pictures of a moving scene differ in the sensor's noise and in what
/// the person pointed the camera at.
pub const CAMERA_FRAMES_128: usize = 3;

/// Frames asked for at 256 bits, for the same reason twice over. Every
/// length in between takes [`CAMERA_FRAMES_128`] frames per 128 bits,
/// rounded up, which is what [`CameraNoise::needed`] asks for.
pub const CAMERA_FRAMES_256: usize = 6;

/// Distinct luma values a frame must have to be taken. A covered lens
/// or a saturated one gives a frame of a handful of values, and such a
/// frame carries no noise worth hashing.
pub const CAMERA_MIN_DISTINCT: u32 = 32;

/// What one camera frame looked like, stated on the result rather than
/// judged (except [`FrameStats::distinct`], which is the one refusal).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    /// Pixels in the frame.
    pub pixels: u32,
    /// How many of the 256 luma values appear at all.
    pub distinct: u32,
    /// Mean luma.
    pub mean: f32,
    /// Variance of the luma.
    pub variance: f32,
}

impl FrameStats {
    /// Reads a frame's luma bytes.
    pub fn of(luma: &[u8]) -> Self {
        let mut counts = [0u32; 256];
        for &b in luma {
            counts[usize::from(b)] += 1;
        }
        let n = luma.len() as u32;
        let distinct = counts.iter().filter(|&&c| c > 0).count() as u32;
        let (mut sum, mut sum_sq) = (0f64, 0f64);
        for (v, &c) in counts.iter().enumerate() {
            let (v, c) = (v as f64, f64::from(c));
            sum += v * c;
            sum_sq += v * v * c;
        }
        let (mean, variance) = if n == 0 {
            (0.0, 0.0)
        } else {
            let m = sum / f64::from(n);
            (m, sum_sq / f64::from(n) - m * m)
        };
        FrameStats {
            pixels: n,
            distinct,
            mean: mean as f32,
            variance: variance as f32,
        }
    }

    /// Whether the frame carries enough variation to be taken.
    pub fn usable(self) -> bool {
        self.distinct >= CAMERA_MIN_DISTINCT
    }
}

/// Accumulates camera frames.
///
/// Each frame's luma bytes are hashed as they arrive — a frame is
/// hundreds of kilobytes and nothing here allocates — and the entropy
/// is SHA-256 over those per-frame digests in order, truncated to the
/// strength. The digests are what a person would have to keep to
/// reproduce the result, and they are what the mix commits to.
///
/// This source trusts the camera and the shell that delivers its
/// frames, which dice do not.
pub struct CameraNoise {
    digests: [[u8; 32]; MAX_FRAMES],
    len: u8,
    last: Option<FrameStats>,
}

impl Default for CameraNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for CameraNoise {
    fn zeroize(&mut self) {
        for d in &mut self.digests {
            d.zeroize();
        }
        self.len.zeroize();
        self.last = None;
    }
}

impl Drop for CameraNoise {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for CameraNoise {}

impl CameraNoise {
    /// No frames yet.
    pub fn new() -> Self {
        CameraNoise {
            digests: [[0u8; 32]; MAX_FRAMES],
            len: 0,
            last: None,
        }
    }

    /// Takes one frame's luma bytes. Returns the frame's statistics,
    /// and whether it was taken: a frame with fewer than
    /// [`CAMERA_MIN_DISTINCT`] luma values is refused, and so is one
    /// past [`MAX_FRAMES`].
    pub fn push(&mut self, luma: &[u8]) -> (FrameStats, bool) {
        let stats = FrameStats::of(luma);
        self.last = Some(stats);
        if !stats.usable() || self.len() >= MAX_FRAMES {
            return (stats, false);
        }
        self.digests[self.len()] = sha256(luma);
        self.len += 1;
        (stats, true)
    }

    /// Frames taken.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether no frame has been taken.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The last frame's statistics, taken or refused.
    pub fn last(&self) -> Option<FrameStats> {
        self.last
    }

    /// Forgets every frame.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Frames needed for `strength`: [`CAMERA_FRAMES_128`] per 128
    /// bits, rounded up, which is 3 at 128 bits and
    /// [`CAMERA_FRAMES_256`] at 256 — 4, 5 and 6 in between.
    pub fn needed(strength: Strength) -> usize {
        (CAMERA_FRAMES_128 * strength.bits()).div_ceil(128)
    }

    /// Whether enough frames are in for `strength`.
    pub fn is_enough(&self, strength: Strength) -> bool {
        self.len() >= Self::needed(strength)
    }

    /// SHA-256 over the frames' digests, in order.
    pub fn commitment(&self) -> [u8; 32] {
        let mut buf = [0u8; MAX_FRAMES * 32];
        for (i, d) in self.digests[..self.len()].iter().enumerate() {
            buf[i * 32..(i + 1) * 32].copy_from_slice(d);
        }
        let h = sha256(&buf[..self.len() * 32]);
        buf.zeroize();
        h
    }

    /// The commitment truncated to the strength.
    pub fn entropy(&self, strength: Strength) -> Result<RawEntropy, Error> {
        if !self.is_enough(strength) {
            return Err(Error::TooFew {
                have: self.len(),
                need: Self::needed(strength),
            });
        }
        let mut hash = self.commitment();
        let e = RawEntropy::new(&hash[..strength.bytes()]);
        hash.zeroize();
        Ok(e)
    }

    /// The sanity flags for `strength`: a frame is a picture, and the
    /// only thing to say about a run of them is whether there are
    /// enough.
    pub fn warnings(&self, strength: Strength) -> Warnings {
        Warnings {
            too_few: !self.is_enough(strength),
            ..Warnings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, Strength};
    use faraday_crypto::sha256;

    #[test]
    fn a_covered_lens_is_refused_and_frames_hash_in_order() {
        let mut c = CameraNoise::new();
        let covered = [7u8; 4096];
        let (stats, taken) = c.push(&covered);
        assert!(!taken);
        assert_eq!(stats.distinct, 1);
        assert_eq!(stats.mean, 7.0);
        assert_eq!(stats.variance, 0.0);
        assert_eq!(stats.pixels, 4096);
        assert!(c.is_empty());

        // A gradient of 32 values is the least a frame may carry.
        let edge: [u8; 256] = core::array::from_fn(|i| (i % 32) as u8);
        let (stats, taken) = c.push(&edge);
        assert_eq!(stats.distinct, 32);
        assert!(taken);

        let mut c = CameraNoise::new();
        let frames: [[u8; 512]; 3] =
            core::array::from_fn(|f| core::array::from_fn(|i| (i * 7 + f * 3) as u8));
        for f in &frames {
            assert!(c.push(f).1);
        }
        assert_eq!(c.len(), 3);
        assert!(c.is_enough(Strength::Bits128));
        assert!(!c.is_enough(Strength::Bits256));
        // The definition, computed here: SHA-256 over the frames'
        // SHA-256 digests, in order.
        let mut buf = [0u8; 96];
        for (i, f) in frames.iter().enumerate() {
            buf[i * 32..(i + 1) * 32].copy_from_slice(&sha256(f));
        }
        assert_eq!(c.commitment(), sha256(&buf));
        assert_eq!(
            c.entropy(Strength::Bits128).unwrap().as_bytes(),
            &sha256(&buf)[..16]
        );
        assert_eq!(
            c.entropy(Strength::Bits256).err(),
            Some(Error::TooFew { have: 3, need: 6 })
        );
    }
}
