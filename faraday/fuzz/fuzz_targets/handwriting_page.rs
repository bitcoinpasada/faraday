//! A handwritten page: the first two bytes make a width and a height of at
//! most 96 (a small frame, so a run is quick), and the bytes that follow, used
//! again from the start if they are too few, are the frame's grey pixels. The
//! frame goes to `read_page_with` once, with the built-in lists and a caller's
//! list (so one detector pass per input reaches `find_words` through it; the
//! separate calls of `find_words` and `read_page` were dropped, they ran the
//! detector again on the same input).

#![no_main]

use faraday_handwriting::{Extra, read_page_with};
use libfuzzer_sys::fuzz_target;

const WORDS: [&str; 4] = ["abandon", "ability", "able", "zoo"];

fuzz_target!(|data: &[u8]| {
    let Some((head, body)) = data.split_first_chunk::<2>() else {
        return;
    };
    if body.is_empty() {
        return;
    }
    let width = usize::from(head[0]) % 97;
    let height = usize::from(head[1]) % 97;
    let gray: Vec<u8> = body.iter().copied().cycle().take(width * height).collect();
    let extra = Extra {
        words: &WORDS,
        counts: &[4],
    };
    let _ = read_page_with(width, height, &gray, Some(extra));
});
