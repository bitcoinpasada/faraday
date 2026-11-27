//! Tests of the image clean-up, of reading a crop, and of the word lists:
//! synthetic crops (a dark word-like mark on light paper), hostile sizes, and
//! the published strips run through the crop path.

use crate::tests::{CAPITALS, Cursor};
use crate::{BEAM, Cleaned, Error, INK_HEIGHT, MAX_INK_WIDTH, Reading};
use crate::{bip39_words, clean_crop, log_softmax, read_crop, slip39_words, unread_ink};
use crate::{word_logits, word_scores, word_scores_in};

// --- Scores name the words of their own list -------------------------------

#[test]
fn best_names_words_of_the_list_the_scores_were_made_from() {
    let ink = vec![255u8; INK_HEIGHT * 40];
    let mut logits = word_logits(&ink, 40).unwrap();
    log_softmax(&mut logits);
    let list = ["ab", "abc", "z"];
    let small = word_scores_in(&logits, BEAM, &list).unwrap();
    let best = small.best(10);
    assert_eq!(best.len(), 3);
    for c in best.iter() {
        assert_eq!(c.word(), list[usize::from(c.index)]);
    }
    let slip = slip39_words();
    assert_eq!(slip.len(), 1024);
    let scores = word_scores_in(&logits, BEAM, &slip).unwrap();
    assert_eq!(scores.values().len(), 1024);
    for c in scores.best(2000).iter() {
        assert_eq!(c.word(), slip[usize::from(c.index)]);
    }
    let english = word_scores(&logits, BEAM).unwrap();
    for c in english.best(5).iter() {
        assert_eq!(c.word(), bip39_words()[usize::from(c.index)]);
    }
}

// --- Image clean-up ---------------------------------------------------------

type Box4 = (usize, usize, usize, usize);

/// A grey crop filled with `paper`.
fn paper(width: usize, height: usize, paper: u8) -> Vec<u8> {
    vec![paper; width * height]
}

/// A word-like mark in `x0..x1` by `y0..y1`: upright strokes and bars, drawn
/// in `dark`.
fn mark(crop: &mut [u8], width: usize, area: Box4, dark: u8) {
    mark_scaled(crop, width, area, dark, 1);
}

/// The same mark with strokes `s` times as thick and as far apart.
fn mark_scaled(crop: &mut [u8], width: usize, (x0, y0, x1, y1): Box4, dark: u8, s: usize) {
    let mid = (y0 + y1) / 2;
    for y in y0..y1 {
        for x in x0..x1 {
            let stroke = (x - x0) % (12 * s) < 3 * s;
            let bar = (mid..mid + 2 * s).contains(&y) && (x - x0) % (24 * s) < 14 * s;
            if stroke || bar {
                crop[y * width + x] = dark;
            }
        }
    }
}

fn find(width: usize, height: usize, crop: &[u8]) -> Cleaned {
    clean_crop(width, height, crop).unwrap().unwrap()
}

fn assert_encloses(c: &Cleaned, (x0, y0, x1, y1): Box4) {
    let (l, t, r, b) = c.bounds();
    assert!(l <= x0 && t <= y0 && r >= x1 && b >= y1, "{:?}", c.bounds());
    // Not far wider than the word: the margins are a fifth of its height.
    assert!(l + 40 >= x0 && r <= x1 + 40, "{:?}", c.bounds());
}

/// The box holds the word and reaches at most a third of the word's height
/// beyond it on any side (the reduction of a big crop moves edges by a few
/// blocks), so a box of the whole crop fails.
fn assert_tight(c: &Cleaned, (x0, y0, x1, y1): Box4) {
    let (l, t, r, b) = c.bounds();
    let slack = (y1 - y0) / 3;
    assert!(l <= x0 && t <= y0 && r >= x1 && b >= y1, "{:?}", c.bounds());
    assert!(
        l + slack >= x0 && t + slack >= y0 && r <= x1 + slack && b <= y1 + slack,
        "{:?}",
        c.bounds()
    );
}

#[test]
fn a_dark_mark_on_light_paper_is_found_and_scaled() {
    let (w, h) = (300, 80);
    let word = (60, 20, 240, 60);
    let mut crop = paper(w, h, 230);
    mark(&mut crop, w, word, 30);
    let c = find(w, h, &crop);
    assert_encloses(&c, word);
    assert_eq!(c.ink().len(), INK_HEIGHT * c.width());
    assert!((8..=MAX_INK_WIDTH).contains(&c.width()));
    assert!(c.ink().iter().any(|&v| v > 200));
    for y in 0..INK_HEIGHT {
        assert_eq!(c.ink()[y * c.width()], 0, "the margin stays paper");
    }
}

#[test]
fn uneven_light_does_not_hide_the_mark() {
    let (w, h) = (300, 80);
    let word = (60, 20, 240, 60);
    let mut crop = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            crop[y * w + x] = (250 - x * 140 / w) as u8;
        }
    }
    mark(&mut crop, w, word, 40);
    assert_encloses(&find(w, h, &crop), word);
}

#[test]
fn a_ruled_line_is_left_out_of_the_word() {
    let (w, h) = (300, 100);
    let word = (60, 20, 240, 60);
    let mut crop = paper(w, h, 235);
    mark(&mut crop, w, word, 30);
    for y in 80..82 {
        for x in 0..w {
            crop[y * w + x] = 120;
        }
    }
    let c = find(w, h, &crop);
    assert_encloses(&c, word);
    let (_, _, _, bottom) = c.bounds();
    assert!(bottom < 80, "the box reaches the line: {:?}", c.bounds());
    for row in c.ink().chunks(c.width()) {
        assert!(row.iter().filter(|&&v| v > 100).count() < c.width() * 9 / 10);
    }
}

#[test]
fn a_mark_cut_by_the_edge_is_read_from_the_edge() {
    let (w, h) = (200, 70);
    let mut crop = paper(w, h, 225);
    mark(&mut crop, w, (0, 0, 120, 40), 25);
    let c = find(w, h, &crop);
    let (l, t, r, _) = c.bounds();
    assert_eq!((l, t), (0, 0));
    assert!(r >= 120);
}

#[test]
fn a_tall_crop_is_reduced_and_its_box_is_in_crop_coordinates() {
    let (w, h) = (900, 400);
    let word = (200, 100, 700, 300);
    let mut crop = paper(w, h, 220);
    mark(&mut crop, w, word, 20);
    let c = find(w, h, &crop);
    assert_tight(&c, word);
}

#[test]
fn crops_without_a_word_give_nothing_and_hostile_crops_do_not_panic() {
    assert_eq!(clean_crop(3, 3, &[0; 8]).unwrap_err(), Error::Size);
    assert_eq!(clean_crop(usize::MAX, 2, &[0; 8]).unwrap_err(), Error::Size);
    assert_eq!(unread_ink(3, 3, &[]).unwrap_err(), Error::Size);
    assert!(clean_crop(0, 5, &[]).unwrap().is_none());
    assert!(clean_crop(5, 0, &[]).unwrap().is_none());
    assert!(clean_crop(0, 0, &[]).unwrap().is_none());
    for fill in [0u8, 128, 255] {
        assert!(clean_crop(50, 40, &vec![fill; 2000]).unwrap().is_none());
        assert_eq!(unread_ink(50, 40, &vec![fill; 2000]).unwrap(), None);
    }
    for (w, h) in [(1, 1), (1, 30), (30, 1), (2, 2), (7, 5), (100, 3), (64, 64)] {
        let mut seed = 12_345u32 + (w * h) as u32;
        let noise: Vec<u8> = (0..w * h)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as u8
            })
            .collect();
        if let Some(c) = clean_crop(w, h, &noise).unwrap() {
            assert!(c.width() <= MAX_INK_WIDTH);
        }
        unread_ink(w, h, &noise).unwrap();
    }
}

#[test]
fn huge_crops_are_bounded() {
    let wide = vec![200u8; 200_000];
    assert!(clean_crop(200_000, 1, &wide).unwrap().is_none());
    let tall = vec![200u8; 150_000];
    assert!(clean_crop(1, 150_000, &tall).unwrap().is_none());
    let (w, h) = (6000, 2000);
    let mut big = paper(w, h, 230);
    let word = (1000, 600, 5000, 1400);
    mark_scaled(&mut big, w, word, 20, 20);
    assert_tight(&find(w, h, &big), word);
}

#[test]
fn faint_ink_beside_the_word_is_reported_and_none_is_zero() {
    let (w, h) = (400, 80);
    let mut crop = paper(w, h, 240);
    mark(&mut crop, w, (60, 20, 240, 60), 20);
    assert_eq!(unread_ink(w, h, &crop).unwrap(), Some(0.0));
    // Pale strokes just after the word, too light to be read as ink.
    mark(&mut crop, w, (250, 20, 290, 60), 205);
    let unread = unread_ink(w, h, &crop).unwrap().unwrap();
    assert!(unread > 0.5, "{unread}");
}

#[test]
fn a_ruled_line_through_the_word_leaves_the_unread_ink_unknown() {
    let (w, h) = (400, 80);
    let mut crop = paper(w, h, 240);
    mark(&mut crop, w, (60, 20, 240, 60), 20);
    for px in crop.iter_mut().skip(45 * w).take(2 * w) {
        *px = 20;
    }
    assert_eq!(unread_ink(w, h, &crop).unwrap(), None);
}

#[test]
fn a_dashed_line_far_below_the_word_leaves_the_unread_ink_unknown() {
    let (w, h) = (400, 80);
    let mut crop = paper(w, h, 240);
    mark(&mut crop, w, (60, 10, 240, 40), 20);
    // Dashes of 40 columns, shorter than a quarter of the crop, every 60,
    // dark; between them a trace of ink (0.14 of the strongest): too faint
    // for the line to stay whole where the word is read, whole where the
    // faint ink is counted.
    for y in 62..72 {
        for x in 0..w {
            crop[y * w + x] = if x % 60 < 40 { 20 } else { 190 };
        }
    }
    // The word is read from the line's rows ...
    let found = find(w, h, &crop);
    let (_, top, _, _) = found.bounds();
    assert!(top >= 55, "the word box starts at row {top}");
    // ... so the faint ink of the other part is not the word's: unknown.
    assert_eq!(unread_ink(w, h, &crop).unwrap(), None);
}

// --- The likeliest alignment ------------------------------------------------

/// Log-likelihoods of `frames` frames where the named class has the given
/// value at its frame and every other class -20.
fn logits_of(frames: usize, likely: &[(usize, usize, f32)]) -> crate::Logits {
    let mut data = vec![-20.0f32; crate::CLASSES * frames];
    for &(k, t, v) in likely {
        data[k * frames + t] = v;
    }
    crate::Logits {
        data: zeroize::Zeroizing::new(data),
        frames,
    }
}

#[test]
fn a_repeated_letter_needs_a_blank_between_and_a_number_may_stand_before() {
    use crate::read::{best_path, path_gap};
    // "aa" over a, blank, a: the likely path, with nothing lost.
    let lp = logits_of(3, &[(1, 0, -1.0), (0, 1, -2.0), (1, 2, -3.0)]);
    assert_eq!(best_path(&lp, &[1, 1], false), -6.0);
    assert_eq!(path_gap(&lp, -6.0, &[1, 1], false), Some(0.0));
    // "a" alone has to leave one of the a's: far below the free reading.
    assert!(path_gap(&lp, -6.0, &[1], false).unwrap() > 10.0);
    // Two frames are too few for "aa".
    let two = logits_of(2, &[(1, 0, -1.0), (1, 1, -1.0)]);
    assert_eq!(path_gap(&two, -2.0, &[1, 1], false), None);
    // A digit (class 28) before the word counts only with digits allowed.
    let numbered = logits_of(2, &[(28, 0, -1.0), (1, 1, -1.0)]);
    assert_eq!(best_path(&numbered, &[1], true), -2.0);
    assert!(best_path(&numbered, &[1], false) < -20.0);
}

// --- Reading a crop ---------------------------------------------------------

#[test]
fn a_crop_with_no_word_reads_as_unclear_and_empty() {
    let words = bip39_words();
    let blank = paper(100, 50, 240);
    match read_crop(100, 50, &blank, words).unwrap() {
        Reading::Unclear(list) => assert!(list.is_empty()),
        Reading::Clear(_) => panic!("a blank crop read as a word"),
    }
    assert_eq!(
        read_crop(100, 50, &blank, &[]).unwrap_err(),
        Error::WordList
    );
    assert_eq!(
        read_crop(100, 50, &blank, &["Ab"]).unwrap_err(),
        Error::WordList
    );
    assert_eq!(read_crop(100, 51, &blank, words).unwrap_err(), Error::Size);
}

#[test]
fn a_mark_that_is_no_word_is_not_clear_and_lists_candidates() {
    let (w, h) = (300, 80);
    let mut crop = paper(w, h, 230);
    mark(&mut crop, w, (60, 20, 240, 60), 30);
    let slip = slip39_words();
    for words in [bip39_words(), slip.as_slice()] {
        match read_crop(w, h, &crop, words).unwrap() {
            Reading::Unclear(list) => {
                assert_eq!(list.len(), crate::UNCLEAR_CANDIDATES);
                for pair in list.windows(2) {
                    assert!(pair[0].score >= pair[1].score);
                }
                for c in list.iter() {
                    assert_eq!(c.word(), words[usize::from(c.index)]);
                }
            }
            Reading::Clear(c) => panic!("a stripe pattern read as {}", c.word()),
        }
    }
}

/// The published strips as grey crops (ink dark on white), with what each
/// strip was written as: the label, a line number possibly before the word,
/// or nothing for noise.
fn published_strips() -> Vec<(usize, Vec<u8>, &'static str)> {
    let mut c = Cursor(CAPITALS);
    let n = c.u32();
    let mut strips = Vec::new();
    for _ in 0..n {
        let width = c.u32();
        let gray: Vec<u8> = c
            .take(INK_HEIGHT * width)
            .iter()
            .map(|&v| 255 - v)
            .collect();
        c.take(64 + 5 * 16);
        strips.push((width, gray, ""));
    }
    for strip in &mut strips {
        let raw = c.take(12);
        let label = std::str::from_utf8(raw).unwrap().trim_end_matches('\0');
        strip.2 = label.trim_start_matches(|ch: char| ch.is_ascii_digit());
    }
    assert!(c.0.is_empty());
    strips
}

/// A clear reading is the word the strip was written as, and a strip whose
/// label is no BIP39 word, or that holds noise, never reads clearly. The
/// rule's constants are set on these strips.
#[test]
fn a_clear_reading_of_a_published_strip_is_what_was_written() {
    let words = bip39_words();
    let (mut clear, mut wrong, mut unlisted) = (0, Vec::new(), 0);
    let strips = published_strips();
    for (i, (width, gray, label)) in strips.iter().enumerate() {
        let listed = words.contains(label);
        if !listed {
            unlisted += 1;
        }
        if let Reading::Clear(got) = read_crop(*width, INK_HEIGHT, gray, words).unwrap() {
            clear += 1;
            if got.word() != *label {
                let read = words.iter().find(|w| **w == got.word());
                wrong.push((i, *label, read));
            }
        }
    }
    eprintln!(
        "{clear} of {} strips read clearly ({unlisted} unlisted)",
        strips.len()
    );
    assert!(
        wrong.is_empty(),
        "a clear reading was not what was written: {wrong:?}"
    );
    assert!(clear * 3 >= strips.len(), "only {clear} read clearly");
}
