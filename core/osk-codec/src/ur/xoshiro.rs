//! The xoshiro256** generator BCR-2020-005 chooses fountain fragments
//! with, and the weighted sampler that picks a part's degree.
//!
//! Nothing here is random in the security sense and nothing here may be
//! used for a secret. It is a deterministic function of a part's
//! sequence number and its message's checksum, which is exactly why it
//! works: the sender and the receiver derive the same fragment indices
//! from the part's own header without either telling the other.

use alloc::vec::Vec;

use osk_crypto::sha256;

/// xoshiro256**, seeded from a SHA-256 digest.
pub struct Xoshiro256 {
    s: [u64; 4],
}

/// What `rand_xoshiro` substitutes for an all-zero seed: the first 32
/// bytes of `SplitMix64(0)`. A SHA-256 digest is never all zero in
/// practice; the branch is here so that the stream is the same function
/// of the seed as the reference implementation's, without exception.
#[rustfmt::skip]
const ZERO_SEED_FALLBACK: [u8; 32] = [
    0xaf, 0xcd, 0x1d, 0x7b, 0x39, 0xa8, 0x20, 0xe2, 0xf4, 0x65, 0xb9, 0xa1, 0x6a, 0x9e, 0x78, 0x6e,
    0x4f, 0x45, 0x09, 0x80, 0x18, 0x5d, 0xc4, 0x06, 0xec, 0x81, 0x4c, 0x72, 0xa8, 0xb8, 0x8b, 0xf8,
];

impl Xoshiro256 {
    /// Seeded with SHA-256 of `bytes`, the four state words read from
    /// the digest big-endian.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_digest(sha256(bytes))
    }

    fn from_digest(digest: [u8; 32]) -> Self {
        let digest = if digest.iter().any(|&b| b != 0) {
            digest
        } else {
            ZERO_SEED_FALLBACK
        };
        let mut s = [0u64; 4];
        for (word, chunk) in s.iter_mut().zip(digest.chunks_exact(8)) {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(chunk);
            *word = u64::from_be_bytes(bytes);
        }
        Self { s }
    }

    /// The next 64 bits of the stream.
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// The next value in `[0, 1)`, from the top 53 bits.
    fn next_double(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1_u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }

    /// The next value in `low..=high`.
    ///
    /// A multiply by a double, not a modulus. Which fragments a part
    /// mixes follows from this arithmetic, so a "more correct" uniform
    /// integer here would produce a stream no coordinator can decode.
    pub fn next_int(&mut self, low: u64, high: u64) -> u64 {
        (self.next_double() * ((high - low + 1) as f64)) as u64 + low
    }

    /// The first `count` of `items` in shuffled order, each drawn by
    /// index from what is left.
    pub fn shuffled<T>(&mut self, mut items: Vec<T>, count: usize) -> Vec<T> {
        let count = count.min(items.len());
        let mut shuffled = Vec::with_capacity(count);
        while shuffled.len() < count {
            let index = self.next_int(0, (items.len() - 1) as u64) as usize;
            shuffled.push(items.remove(index));
        }
        shuffled
    }

    /// How many fragments the next part mixes: `1..=length`, weighted
    /// `1/i`, so most parts carry one or two fragments.
    pub fn choose_degree(&mut self, length: usize) -> usize {
        let weights: Vec<f64> = (1..=length).map(|x| 1.0 / x as f64).collect();
        Weighted::new(weights).next(self) + 1
    }
}

/// Vose's alias sampler over a fixed set of weights.
struct Weighted {
    aliases: Vec<usize>,
    probs: Vec<f64>,
}

impl Weighted {
    fn new(mut weights: Vec<f64>) -> Self {
        let count = weights.len();
        let summed: f64 = weights.iter().sum();
        for w in &mut weights {
            *w *= count as f64 / summed;
        }
        let (mut small, mut large): (Vec<usize>, Vec<usize>) = (1..=count)
            .map(|j| count - j)
            .partition(|&j| weights[j] < 1.0);

        let mut probs = alloc::vec![0.0; count];
        let mut aliases = alloc::vec![0; count];

        while let (Some(&a), Some(&g)) = (small.last(), large.last()) {
            small.pop();
            large.pop();
            probs[a] = weights[a];
            aliases[a] = g;
            weights[g] += weights[a] - 1.0;
            if weights[g] < 1.0 {
                small.push(g);
            } else {
                large.push(g);
            }
        }
        for g in large {
            probs[g] = 1.0;
        }
        for a in small {
            probs[a] = 1.0;
        }

        Self { aliases, probs }
    }

    fn next(&self, xoshiro: &mut Xoshiro256) -> usize {
        let r1 = xoshiro.next_double();
        let r2 = xoshiro.next_double();
        let i = (self.probs.len() as f64 * r1) as usize;
        if r2 < self.probs[i] {
            i
        } else {
            self.aliases[i]
        }
    }
}

#[cfg(test)]
pub(crate) mod test_utils {
    use super::Xoshiro256;
    use alloc::vec::Vec;

    impl Xoshiro256 {
        /// `n` bytes of the stream, one `next_int(0, 255)` each: how the
        /// BCR-2020-005 vectors name their messages.
        pub(crate) fn next_bytes(&mut self, n: usize) -> Vec<u8> {
            (0..n).map(|_| self.next_int(0, 255) as u8).collect()
        }
    }

    /// The seeded message the published vectors are taken over.
    pub(crate) fn make_message(seed: &str, size: usize) -> Vec<u8> {
        Xoshiro256::from_bytes(seed.as_bytes()).next_bytes(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn the_stream_matches_the_reference_implementation() {
        let mut rng = Xoshiro256::from_bytes(b"Wolf");
        let expected = [
            42, 81, 85, 8, 82, 84, 76, 73, 70, 88, 2, 74, 40, 48, 77, 54, 88, 7, 5, 88, 37, 25, 82,
            13, 69, 59, 30, 39, 11, 82, 19, 99, 45, 87, 30, 15, 32, 22, 89, 44, 92, 77, 29, 78, 4,
            92, 44, 68, 92, 69, 1, 42, 89, 50, 37, 84, 63, 34, 32, 3, 17, 62, 40, 98, 82, 89, 24,
            43, 85, 39, 15, 3, 99, 29, 20, 42, 27, 10, 85, 66, 50, 35, 69, 70, 70, 74, 30, 13, 72,
            54, 11, 5, 70, 55, 91, 52, 10, 43, 43, 52,
        ];
        for e in expected {
            assert_eq!(rng.next_u64() % 100, e);
        }

        let mut rng =
            Xoshiro256::from_bytes(&super::super::bytewords::crc32(b"Wolf").to_be_bytes());
        let expected = [
            88, 44, 94, 74, 0, 99, 7, 77, 68, 35, 47, 78, 19, 21, 50, 15, 42, 36, 91, 11, 85, 39,
            64, 22, 57, 11, 25, 12, 1, 91, 17, 75, 29, 47, 88, 11, 68, 58, 27, 65, 21, 54, 47, 54,
            73, 83, 23, 58, 75, 27, 26, 15, 60, 36, 30, 21, 55, 57, 77, 76, 75, 47, 53, 76, 9, 91,
            14, 69, 3, 95, 11, 73, 20, 99, 68, 61, 3, 98, 36, 98, 56, 65, 14, 80, 74, 57, 63, 68,
            51, 56, 24, 39, 53, 80, 57, 51, 81, 3, 1, 30,
        ];
        for e in expected {
            assert_eq!(rng.next_u64() % 100, e);
        }

        let mut rng = Xoshiro256::from_bytes(b"Wolf");
        let expected = [
            6, 5, 8, 4, 10, 5, 7, 10, 4, 9, 10, 9, 7, 7, 1, 1, 2, 9, 9, 2, 6, 4, 5, 7, 8, 5, 4, 2,
            3, 8, 7, 4, 5, 1, 10, 9, 3, 10, 2, 6, 8, 5, 7, 9, 3, 1, 5, 2, 7, 1, 4, 4, 4, 4, 9, 4,
            5, 5, 6, 9, 5, 1, 2, 8, 3, 3, 2, 8, 4, 3, 2, 1, 10, 8, 9, 3, 10, 8, 5, 5, 6, 7, 10, 5,
            8, 9, 4, 6, 4, 2, 10, 2, 1, 7, 9, 6, 7, 4, 2, 5,
        ];
        for e in expected {
            assert_eq!(rng.next_int(1, 10), e);
        }
    }

    #[test]
    fn shuffles_match_the_reference_implementation() {
        let mut rng = Xoshiro256::from_bytes(b"Wolf");
        let values = alloc::vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let expected = [
            [6, 4, 9, 3, 10, 5, 7, 8, 1, 2],
            [10, 8, 6, 5, 1, 2, 3, 9, 7, 4],
            [6, 4, 5, 8, 9, 3, 2, 1, 7, 10],
            [7, 3, 5, 1, 10, 9, 4, 8, 2, 6],
            [8, 5, 7, 10, 2, 1, 4, 3, 9, 6],
            [4, 3, 5, 6, 10, 2, 7, 8, 9, 1],
            [5, 1, 3, 9, 4, 6, 2, 10, 7, 8],
            [2, 1, 10, 8, 9, 4, 7, 6, 3, 5],
            [6, 7, 10, 4, 8, 9, 2, 3, 1, 5],
            [10, 2, 1, 7, 9, 5, 6, 3, 4, 8],
        ];
        for e in expected {
            assert_eq!(rng.shuffled(values.clone(), values.len()), e.to_vec());
        }

        // A partial shuffle is the prefix of the full one.
        let mut full_rng = Xoshiro256::from_bytes(b"Wolf");
        let full = full_rng.shuffled(values.clone(), values.len());
        for count in 0..=values.len() {
            let mut rng = Xoshiro256::from_bytes(b"Wolf");
            assert_eq!(rng.shuffled(values.clone(), count), full[..count]);
        }
    }

    #[test]
    fn the_degree_sampler_matches_the_reference_implementation() {
        let weights = alloc::vec![1.0, 2.0, 4.0, 8.0];
        let mut rng = Xoshiro256::from_bytes(b"Wolf");
        let sampler = Weighted::new(weights);
        let expected = [
            3, 3, 3, 3, 3, 3, 3, 0, 2, 3, 3, 3, 3, 1, 2, 2, 1, 3, 3, 2, 3, 3, 1, 1, 2, 1, 1, 3, 1,
            3, 1, 2, 0, 2, 1, 0, 3, 3, 3, 1, 3, 3, 3, 3, 1, 3, 2, 3, 2, 2, 3, 3, 3, 3, 2, 3, 3, 0,
            3, 3, 3, 3, 1, 2, 3, 3, 2, 2, 2, 1, 2, 2, 1, 2, 3, 1, 3, 0, 3, 2, 3, 3, 3, 3, 3, 3, 3,
            3, 2, 3, 1, 3, 3, 2, 0, 2, 2, 3, 1, 1,
        ];
        for e in expected {
            assert_eq!(sampler.next(&mut rng), e);
        }
    }

    #[test]
    fn degrees_for_the_specification_message() {
        let expected = [
            11, 3, 6, 5, 2, 1, 2, 11, 1, 3, 9, 10, 10, 4, 2, 1, 1, 2, 1, 1, 5, 2, 4, 10, 3, 2, 1,
            1, 3, 11, 2, 6, 2, 9, 9, 2, 6, 7, 2, 5, 2, 4, 3, 1, 6, 11, 2, 11, 3, 1, 6, 3, 1, 4, 5,
            3, 6, 1, 1, 3, 1, 2, 2, 1, 4, 5, 1, 1, 9, 1, 1, 6, 4, 1, 5, 1, 2, 2, 3, 1, 1, 5, 2, 6,
            1, 7, 11, 1, 8, 1, 5, 1, 1, 2, 2, 6, 4, 10, 1, 2,
        ];
        for (nonce, e) in (1..=expected.len()).zip(expected) {
            let mut rng = Xoshiro256::from_bytes(format!("Wolf-{nonce}").as_bytes());
            assert_eq!(rng.choose_degree(11), e);
        }
    }
}
