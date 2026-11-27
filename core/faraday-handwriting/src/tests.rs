//! Tests against Haller's published vectors (`tests/data/*.bin`, written by
//! `tools/weights.py`): the logits and the maps bit for bit, the best five
//! words of each strip, and hostile sizes.

use sha2::{Digest, Sha256};

use crate::{
    BEAM, DETECTOR_HEIGHT, DETECTOR_OUTPUTS, DETECTOR_WIDTH, Error, INK_HEIGHT, MAX_WIDTH,
    detector_maps, log_softmax, read_word, word_logits, word_scores,
};

pub(crate) static CAPITALS: &[u8] = include_bytes!("../tests/data/capitals.bin");
static DETECTOR: &[u8] = include_bytes!("../tests/data/detector.bin");

/// The best scores of a strip may differ from the reference's by this much:
/// the reference's `expf` and `logf` are the platform's, ours are our own.
const SCORE_TOLERANCE: f32 = 1.0e-3;

pub(crate) struct Cursor<'a>(pub(crate) &'a [u8]);

impl<'a> Cursor<'a> {
    pub(crate) fn take(&mut self, n: usize) -> &'a [u8] {
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        a
    }

    pub(crate) fn u32(&mut self) -> usize {
        u32::from_le_bytes(self.take(4).try_into().unwrap()) as usize
    }

    pub(crate) fn f32(&mut self) -> f32 {
        f32::from_le_bytes(self.take(4).try_into().unwrap())
    }
}

fn sha(values: &[f32]) -> Vec<u8> {
    let mut h = Sha256::new();
    for v in values {
        h.update(v.to_le_bytes());
    }
    h.finalize().to_vec()
}

#[test]
fn word_logits_match_the_published_vectors_bit_for_bit() {
    let mut c = Cursor(CAPITALS);
    let n = c.u32();
    assert_eq!(n, 62);
    for i in 0..n {
        let width = c.u32();
        let ink = c.take(INK_HEIGHT * width);
        let (want_logits, _scores) = (c.take(32), c.take(32));
        c.take(5 * 16);
        let logits = word_logits(ink, width).unwrap();
        assert_eq!(sha(logits.values()), want_logits, "strip {i}");
    }
}

#[test]
fn best_five_words_are_the_published_ones() {
    let mut c = Cursor(CAPITALS);
    let n = c.u32();
    let (mut exact, mut worst) = (0, 0.0f32);
    for i in 0..n {
        let width = c.u32();
        let ink = c.take(INK_HEIGHT * width);
        c.take(32);
        let want_scores = c.take(32);
        let scores = read_word(ink, width).unwrap();
        if sha(scores.values()) == want_scores {
            exact += 1;
        }
        let best = scores.best(5);
        for rank in 0..5 {
            let raw = c.take(12);
            let word = std::str::from_utf8(raw).unwrap().trim_end_matches('\0');
            let want = c.f32();
            assert_eq!(best[rank].word(), word, "strip {i} rank {rank}");
            let diff = (best[rank].score - want).abs();
            assert!(
                diff <= SCORE_TOLERANCE,
                "strip {i} rank {rank}: {} against {want}",
                best[rank].score
            );
            worst = worst.max(diff);
        }
    }
    assert_eq!(exact, 61, "strips whose 2048 scores hash equal, of {n}");
    assert_eq!(worst, 0.0);
}

#[test]
fn detector_maps_match_the_published_vectors_bit_for_bit() {
    let mut c = Cursor(DETECTOR);
    let n = c.u32();
    assert_eq!(n, 2);
    for i in 0..n {
        let image = c.take(DETECTOR_HEIGHT * DETECTOR_WIDTH);
        let want = c.take(32);
        let mut max = [0f32; DETECTOR_OUTPUTS];
        for m in &mut max {
            *m = c.f32();
        }
        let maps = detector_maps(image).unwrap();
        assert_eq!(sha(maps.values()), want, "image {i}");
        for (k, m) in max.iter().enumerate() {
            let got = maps.channel(k).iter().copied().fold(f32::MIN, f32::max);
            assert_eq!(got.to_bits(), m.to_bits(), "image {i} channel {k}");
        }
    }
}

#[test]
fn wrong_sizes_are_refused() {
    assert_eq!(word_logits(&[], 0).unwrap_err(), Error::Size);
    assert_eq!(word_logits(&[0; 32], 0).unwrap_err(), Error::Size);
    assert_eq!(word_logits(&[0; 31], 1).unwrap_err(), Error::Size);
    assert_eq!(word_logits(&[0; 33], 1).unwrap_err(), Error::Size);
    assert_eq!(word_logits(&[0; 64], usize::MAX).unwrap_err(), Error::Size);
    let wide = vec![0u8; INK_HEIGHT * (MAX_WIDTH + 1)];
    assert_eq!(
        word_logits(&wide, MAX_WIDTH + 1).unwrap_err(),
        Error::TooWide
    );
    assert_eq!(detector_maps(&[]).unwrap_err(), Error::Size);
    assert_eq!(
        detector_maps(&vec![0; DETECTOR_HEIGHT * DETECTOR_WIDTH + 1]).unwrap_err(),
        Error::Size
    );
}

#[test]
fn any_width_and_content_reads_without_panic() {
    let mut seed = 0x9e37_79b9u32;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for width in (1..=40).chain([63, 64, 65, 127, 200]) {
        for fill in [0u8, 255, 1] {
            let ink = vec![fill; INK_HEIGHT * width];
            let s = read_word(&ink, width).unwrap();
            assert_eq!(s.values().len(), 2048);
        }
        let noise: Vec<u8> = (0..INK_HEIGHT * width).map(|_| next() as u8).collect();
        let mut logits = word_logits(&noise, width).unwrap();
        assert_eq!(logits.frames(), crate::frames(width));
        log_softmax(&mut logits);
        let exact = word_scores(&logits, 0.0).unwrap();
        assert_eq!(exact.best(3).len(), 3);
    }
}

#[test]
fn detector_reads_flat_and_noisy_images() {
    let mut seed = 0x1234_5678u32;
    let noisy: Vec<u8> = (0..DETECTOR_HEIGHT * DETECTOR_WIDTH)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect();
    let maps = detector_maps(&noisy).unwrap();
    assert_eq!(maps.values().len(), DETECTOR_OUTPUTS * 192 * 144);
    assert!(maps.values().iter().all(|v| v.is_finite()));
    let flat = vec![255u8; DETECTOR_HEIGHT * DETECTOR_WIDTH];
    assert_eq!(
        detector_maps(&flat).unwrap().values().len(),
        DETECTOR_OUTPUTS * 192 * 144
    );
    assert!(
        detector_maps(&vec![0; DETECTOR_HEIGHT * DETECTOR_WIDTH])
            .unwrap()
            .channel(DETECTOR_OUTPUTS)
            .is_empty()
    );
}

#[test]
fn nothing_secret_is_printed() {
    let ink = vec![255u8; INK_HEIGHT * 8];
    let logits = word_logits(&ink, 8).unwrap();
    assert_eq!(format!("{logits:?}"), "Logits(2 frames)");
    let scores = read_word(&ink, 8).unwrap();
    assert_eq!(format!("{scores:?}"), "Scores(2048)");
    assert_eq!(format!("{:?}", scores.best(1)[0]), "Candidate(..)");
}

#[test]
fn a_small_word_list_is_accepted_and_a_bad_one_refused() {
    let ink = vec![255u8; INK_HEIGHT * 8];
    let mut logits = word_logits(&ink, 8).unwrap();
    log_softmax(&mut logits);
    let ok = crate::word_scores_in(&logits, BEAM, &["ab", "abc", "z"]).unwrap();
    assert_eq!(ok.values().len(), 3);
    let bad: [&[&str]; 5] = [&[], &[""], &["ab", "ab"], &["Ab"], &["a-b"]];
    for list in bad {
        assert_eq!(
            crate::word_scores_in(&logits, BEAM, list).unwrap_err(),
            Error::WordList
        );
    }
}

#[test]
fn the_flat_weight_and_vector_files_are_the_ones_published() {
    let files: [(&[u8], &str); 5] = [
        (
            include_bytes!("../weights/word.bin"),
            "7912ba87527ea77bec568a5408c5a63cc13a7baa85d0117c7597d994ae92d226",
        ),
        (
            include_bytes!("../weights/detector.bin"),
            "fdcd940d394317a5326f2c312a954c856081596f5cca216e8f67aa69ccbad199",
        ),
        (
            include_bytes!("../weights/logadd.bin"),
            "47840924716cb10c359b1a19eb62999c2cb6ff02d2c07be8f842022b3f107958",
        ),
        (
            CAPITALS,
            "83aabff8c7f5257586e304b552498aec6e8f5b9cbcd198eb55f90af6f823b946",
        ),
        (
            DETECTOR,
            "c3581acb91f299be04d5aa2fce48c708ffd92ad7ad7c4eef4e0071f6e31f5438",
        ),
    ];
    for (bytes, want) in files {
        let got: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(got, want);
    }
}

#[test]
fn a_word_list_with_too_many_letters_is_refused() {
    let ink = vec![255u8; INK_HEIGHT * 8];
    let mut logits = word_logits(&ink, 8).unwrap();
    log_softmax(&mut logits);
    let long = "a".repeat(crate::decode::MAX_LIST_LETTERS + 1);
    assert_eq!(
        crate::word_scores_in(&logits, BEAM, &[long.as_str()]).unwrap_err(),
        Error::WordList
    );
}
