//! A handwritten word's crop and the networks: the first bytes make a width
//! and a height (the crop is the data that follows, cut to fit), which go to
//! `read_crop` and `clean_crop` with a small fixed word list; the same bytes
//! are an ink map of the width for `word_logits` and `word_scores_in`.

#![no_main]

use faraday_handwriting::{
    BEAM, INK_HEIGHT, clean_crop, log_softmax, read_crop, word_logits, word_scores_in,
};
use libfuzzer_sys::fuzz_target;

const WORDS: [&str; 4] = ["abandon", "ability", "able", "zoo"];

fuzz_target!(|data: &[u8]| {
    let Some((head, body)) = data.split_first_chunk::<4>() else {
        return;
    };
    let width = usize::from(u16::from_le_bytes([head[0], head[1]]));
    let height = usize::from(u16::from_le_bytes([head[2], head[3]]));
    let Some(n) = width.checked_mul(height).filter(|&n| n <= body.len()) else {
        return;
    };
    let gray = &body[..n];
    let _ = clean_crop(width, height, gray);
    let _ = read_crop(width, height, gray, &WORDS);

    // The ink map: `INK_HEIGHT` rows of `width` columns.
    let Some(m) = width.checked_mul(INK_HEIGHT).filter(|&m| m <= body.len()) else {
        return;
    };
    if let Ok(mut logits) = word_logits(&body[..m], width) {
        log_softmax(&mut logits);
        let _ = word_scores_in(&logits, BEAM, &WORDS);
    }
});
