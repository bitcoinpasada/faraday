//! Tests of reading a page and of the stages around it, on synthetic pages
//! made from the published strips placed on light paper, and on hostile
//! frames.

use zeroize::Zeroizing;

use crate::find::second_stands;
use crate::find::{join, kernels, two_line_flags};
use crate::page::{
    digits_score, edge_flags, mean_fit, over_limit, read_found, read_numbers, tall_flags,
    within_crop_budget,
};
use crate::tests::{CAPITALS, Cursor};
use crate::{
    CLASSES, DETECTOR_OUTPUTS, Error, Extra, Form, Found, INK_HEIGHT, List, Logits, MAX_KERNELS,
    MAX_READ, MAX_SIDE, Maps, Page, Question, Reading, Rect, find_words, read_page, read_page_with,
};

/// A published strip: its ink map (255 ink, 0 paper) and the word it was
/// written as (empty for noise).
pub(crate) struct Strip {
    pub width: usize,
    pub ink: Vec<u8>,
    raw: [u8; 12],
}

impl Strip {
    pub(crate) fn word(&self) -> &str {
        let n = self.raw.iter().position(|&b| b == 0).unwrap_or(12);
        std::str::from_utf8(&self.raw[..n]).unwrap()
    }
}

pub(crate) fn strips() -> Vec<Strip> {
    let mut c = Cursor(CAPITALS);
    let n = c.u32();
    let mut out = Vec::new();
    for _ in 0..n {
        let width = c.u32();
        let ink = c.take(INK_HEIGHT * width).to_vec();
        c.take(64 + 5 * 16);
        out.push(Strip {
            width,
            ink,
            raw: [0; 12],
        });
    }
    // The labels follow the strips: what each was written as, a line number
    // possibly before the word.
    for strip in &mut out {
        let label = c.take(12);
        let n = label.iter().position(|&b| b == 0).unwrap_or(12);
        let start = label[..n]
            .iter()
            .position(|b| !b.is_ascii_digit())
            .unwrap_or(n);
        let word = &label[start..n];
        strip.raw[..word.len()].copy_from_slice(word);
    }
    out
}

/// A light page.
pub(crate) fn page(w: usize, h: usize) -> Vec<u8> {
    vec![235; w * h]
}

/// Pastes `s` with its top left at `(x, y)`, `scale` times as large (nearest
/// pixel), ink dark on the paper. Returns the box pasted: x0, y0, x1, y1.
pub(crate) fn paste(
    gray: &mut [u8],
    w: usize,
    (x, y): (isize, usize),
    s: &Strip,
    scale: usize,
) -> (usize, usize, usize, usize) {
    let (sw, sh) = (s.width * scale, INK_HEIGHT * scale);
    for dy in 0..sh {
        for dx in 0..sw {
            let ink = s.ink[(dy / scale) * s.width + dx / scale];
            let col = x + dx as isize;
            if ink > 0
                && (0..w as isize).contains(&col)
                && let Some(p) = gray.get_mut((y + dy) * w + col as usize)
            {
                *p = (235 - u16::from(ink) * 205 / 255) as u8;
            }
        }
    }
    (
        x.max(0) as usize,
        y,
        (x + sw as isize).clamp(0, w as isize) as usize,
        y + sh,
    )
}

/// A clear word is never another word than the one written; most are clear.
fn assert_read_rightly(l: &List, want: &[&Strip], most: usize) {
    assert_eq!(l.words.len(), want.len());
    let mut clear = 0;
    for (g, w) in l.words.iter().zip(want) {
        if let Some(g) = g.clear(0) {
            clear += 1;
            assert_eq!(g, w.word());
        }
    }
    assert!(clear >= most, "only {clear} of {} read clearly", want.len());
}

/// `n` strips down one column of a 600 by 800 page, twice their size.
fn one_column(all: &[Strip], n: usize) -> (usize, usize, Vec<u8>) {
    let (w, h) = (600, 800);
    let mut g = page(w, h);
    for (i, s) in all.iter().take(n).enumerate() {
        paste(&mut g, w, (40, 30 + i * 60), s, 2);
    }
    (w, h, g)
}

#[test]
fn twelve_words_down_a_page_read_as_one_bip39_list() {
    let all = strips();
    let (w, h, g) = one_column(&all, 12);
    let p = read_page(w, h, &g).unwrap();
    assert_eq!(p.list_names, vec![Form::Bip39, Form::Slip39]);
    assert!(!p.maybe_cut_off && !p.too_many);
    assert_eq!(p.unattached_numbers, 0);
    assert_eq!(p.lists.len(), 1);
    let l = &p.lists[0];
    assert_eq!((l.form, l.list), (Some(Form::Bip39), Some(0)));
    assert!(l.questions.is_empty(), "{:?}", l.questions);
    assert!(l.across.is_none());
    let want: Vec<&Strip> = all.iter().take(12).collect();
    assert_read_rightly(l, &want, 10);
    // The BIP39 list fits the words, SLIP-39's does not.
    assert!(l.fit[0] > l.fit[1] + 5.0, "{:?}", l.fit);
    // In order, top to bottom.
    for pair in l.words.windows(2) {
        assert!(pair[0].rect.y1 <= pair[1].rect.y1);
    }
    // Each word keeps the crop it was read from, with a margin.
    for wd in &l.words {
        assert_eq!(wd.crop.width * wd.crop.height, wd.crop.gray.len());
        assert!(wd.crop.height > 40);
    }
}

#[test]
fn two_columns_are_asked_about_with_both_orders_given() {
    let all = strips();
    let (w, h) = (700, 800);
    let mut g = page(w, h);
    for (i, s) in all.iter().take(12).enumerate() {
        paste(
            &mut g,
            w,
            (40 + (i / 6) as isize * 330, 30 + (i % 6) * 100),
            s,
            2,
        );
    }
    let p = read_page(w, h, &g).unwrap();
    assert_eq!(p.lists.len(), 1);
    let l = &p.lists[0];
    assert_eq!(l.questions, vec![Question::Order]);
    // The form is not in question: twelve words of the BIP39 list.
    assert_eq!(l.form, Some(Form::Bip39));
    let want: Vec<&Strip> = all.iter().take(12).collect();
    // Down the columns.
    assert_read_rightly(l, &want, 10);
    // And row by row.
    let across = l.across.clone().unwrap();
    assert_eq!(across, vec![0, 6, 1, 7, 2, 8, 3, 9, 4, 10, 5, 11]);
}

#[test]
fn thirteen_words_are_all_kept_and_the_count_is_asked_about() {
    let all = strips();
    let (w, h) = (700, 800);
    let mut g = page(w, h);
    for (i, s) in all.iter().take(13).enumerate() {
        paste(&mut g, w, (40, 20 + i * 58), s, 2);
    }
    let p = read_page(w, h, &g).unwrap();
    assert_eq!(p.lists.len(), 1);
    let l = &p.lists[0];
    assert_eq!(l.words.len(), 13);
    assert_eq!(l.form, None);
    assert_eq!(l.list, None);
    assert_eq!(l.questions, vec![Question::Count]);
    let want: Vec<&Strip> = all.iter().take(13).collect();
    assert_read_rightly(l, &want, 11);
}

#[test]
fn a_word_cut_off_by_the_frame_is_kept_unclear_and_the_page_says_so() {
    let all = strips();
    let (w, h) = (600, 800);
    let mut g = page(w, h);
    for (i, s) in all.iter().take(12).enumerate() {
        paste(
            &mut g,
            w,
            (if i == 3 { -60 } else { 40 }, 30 + i * 60),
            s,
            2,
        );
    }
    let p = read_page(w, h, &g).unwrap();
    assert!(p.maybe_cut_off);
    let l = &p.lists[0];
    assert_eq!(l.words.len(), 12);
    let cut = &l.words[3];
    assert!(cut.edge);
    // No list reads it clearly, and its candidates are kept.
    for k in 0..p.list_names.len() {
        assert!(cut.clear(k).is_none(), "a word at the edge read as clear");
        match cut.reading(k) {
            Some(Reading::Unclear(c)) => assert!(c.len() > 1, "{} candidates", c.len()),
            other => panic!("{other:?}"),
        }
    }
    // The others are as before.
    for (i, wd) in l.words.iter().enumerate().filter(|(i, _)| *i != 3) {
        assert!(!wd.edge);
        if let Some(c) = wd.clear(0) {
            assert_eq!(c, all[i].word());
        }
    }
}

#[test]
fn a_word_split_by_the_frame_edge_is_asked_about_not_dropped() {
    let all = strips();
    let (w, h) = (600, 800);
    let mut g = page(w, h);
    for (i, s) in all.iter().take(12).enumerate() {
        paste(
            &mut g,
            w,
            (if i == 3 { -30 } else { 40 }, 30 + i * 60),
            s,
            2,
        );
    }
    let p = read_page(w, h, &g).unwrap();
    assert!(p.maybe_cut_off);
    let words: usize = p.lists.iter().map(|l| l.words.len()).sum();
    assert!(words >= 12, "{words}");
    for l in &p.lists {
        // Nothing reads clear at the edge, and a list that does not hold
        // twelve words is not settled.
        for wd in l.words.iter().filter(|w| w.edge) {
            assert!(wd.clear(0).is_none());
        }
        // A word read clearly is one that was written.
        for wd in &l.words {
            if let Some(c) = wd.clear(0) {
                assert!(all.iter().take(12).any(|s| s.word() == c), "{c}");
            }
        }
        if l.words.len() != 12 {
            assert!(l.form.is_none() && !l.questions.is_empty());
        }
    }
}

#[test]
fn ink_beside_a_word_makes_it_unclear_but_names_it_first() {
    let all = strips();
    let (w, h, mut g) = one_column(&all, 12);
    let word = paste(&mut page(w, h), w, (40, 30 + 5 * 60), &all[5], 2);
    for y in word.1 + 10..word.1 + 50 {
        for x in word.2 + 6..word.2 + 14 {
            g[y * w + x] = 100;
        }
    }
    let p = read_page(w, h, &g).unwrap();
    let l = &p.lists[0];
    assert_eq!(l.words.len(), 12);
    let beside = &l.words[5];
    assert!(beside.clear(0).is_none(), "read clearly with ink beside it");
    match beside.reading(0).unwrap() {
        Reading::Unclear(c) => assert_eq!(c[0].word(), all[5].word()),
        Reading::Clear(_) => panic!("clear"),
    }
    assert_eq!(l.form, Some(Form::Bip39));
}

#[test]
fn a_mark_that_is_no_word_is_counted_as_one_and_asked_about() {
    let all = strips();
    let (w, h) = (700, 800);
    let mut g = page(w, h);
    for (i, s) in all.iter().take(12).enumerate() {
        paste(&mut g, w, (40, 30 + i * 60), s, 2);
    }
    for y in 300..340 {
        for x in 330..370 {
            if (x / 6) % 2 == 0 {
                g[y * w + x] = 120;
            }
        }
    }
    let p = read_page(w, h, &g).unwrap();
    let l = &p.lists[0];
    assert_eq!(l.words.len(), 13);
    assert!(l.questions.contains(&Question::Count));
    assert_eq!(l.form, None);
    // The mark is a word that does not read clearly; no clear word is a
    // word that was not written.
    let mark = l
        .words
        .iter()
        .find(|w| {
            let (cx, cy) = ((w.rect.x0 + w.rect.x1) / 2.0, (w.rect.y0 + w.rect.y1) / 2.0);
            (330.0..370.0).contains(&cx) && (300.0..340.0).contains(&cy)
        })
        .expect("the mark has its own box");
    assert!(mark.clear(0).is_none());
    for wd in &l.words {
        if let Some(c) = wd.clear(0) {
            assert!(all.iter().take(12).any(|s| s.word() == c), "{c}");
        }
    }
}

#[test]
fn an_unusable_extra_list_is_refused() {
    let all = strips();
    let (w, h, g) = one_column(&all, 12);
    let bad = Extra {
        words: &[],
        counts: &[12],
    };
    assert_eq!(
        read_page_with(w, h, &g, Some(bad)).err(),
        Some(Error::WordList)
    );
    let repeated = Extra {
        words: &["ab", "ab"],
        counts: &[12],
    };
    assert_eq!(
        read_page_with(w, h, &g, Some(repeated)).err(),
        Some(Error::WordList)
    );
}

#[test]
fn an_extra_list_is_read_beside_the_others() {
    let all = strips();
    let (w, h, g) = one_column(&all, 12);
    let list: Vec<&str> = vec!["fly", "gate", "shed", "spray", "push", "april"];
    let extra = Extra {
        words: &list,
        counts: &[36],
    };
    let p = read_page_with(w, h, &g, Some(extra)).unwrap();
    assert_eq!(p.list_names, vec![Form::Bip39, Form::Slip39, Form::Extra]);
    let l = &p.lists[0];
    assert_eq!(l.fit.len(), 3);
    assert_eq!(l.words[0].readings.len(), 3);
    assert_eq!(l.form, Some(Form::Bip39));
    // The words are read as in a page without the extra list.
    let want: Vec<&Strip> = all.iter().take(12).collect();
    assert_read_rightly(l, &want, 10);
    // What reads clearly in the extra list is one of its words.
    for w in &l.words {
        if let Some(c) = w.clear(2) {
            assert!(list.contains(&c), "{c}");
        }
    }
}

// --- Hostile frames ---------------------------------------------------------

fn noise(n: usize, mut seed: u64) -> Vec<u8> {
    (0..n)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 24) as u8
        })
        .collect()
}

/// Reads a hostile frame: no panic, no error, and no more words than may be
/// read.
fn survives(w: usize, h: usize, g: &[u8]) -> Page {
    let found = find_words(w, h, g).unwrap();
    assert!(found.words.len() <= MAX_KERNELS && found.numbers.len() <= MAX_KERNELS);
    let p = read_page(w, h, g).unwrap();
    let words: usize = p.lists.iter().map(|l| l.words.len()).sum();
    assert!(words <= MAX_READ);
    p
}

#[test]
fn an_empty_frame_has_no_words() {
    let p = survives(0, 0, &[]);
    assert!(p.lists.is_empty() && !p.too_many);
    for (w, h) in [(0, 5), (5, 0), (0, 100_000), (100_000, 0)] {
        assert!(survives(w, h, &[]).lists.is_empty());
    }
}

#[test]
fn a_frame_of_the_wrong_size_is_refused() {
    assert_eq!(read_page(4, 4, &[0; 15]).err(), Some(Error::Size));
    assert_eq!(find_words(4, 4, &[0; 17]).err(), Some(Error::Size));
    assert_eq!(read_page(usize::MAX, 2, &[0; 4]).err(), Some(Error::Size));
}

#[test]
fn tiny_frames_have_no_words() {
    for v in [0u8, 128, 255] {
        assert!(survives(1, 1, &[v]).lists.is_empty());
        assert!(survives(2, 3, &[v; 6]).lists.is_empty());
        assert!(survives(7, 7, &[v; 49]).lists.is_empty());
    }
}

#[test]
fn flat_and_all_dark_frames_have_no_words() {
    for v in [0u8, 40, 128, 235, 255] {
        let p = survives(600, 800, &vec![v; 600 * 800]);
        assert!(p.lists.is_empty(), "{v}");
        assert!(!p.too_many);
    }
}

#[test]
fn noise_never_panics() {
    for seed in [1u64, 2, 0x9e37_79b9_7f4a_7c15] {
        for (w, h) in [(288, 384), (600, 800), (1000, 300)] {
            survives(w, h, &noise(w * h, seed));
        }
    }
}

#[test]
fn thousands_of_specks_are_reported_or_read_without_panic() {
    // Specks of 2 by 2 pixels, one in every 4 by 4 of the detector's input.
    let (w, h) = (288, 384);
    let mut g = vec![235u8; w * h];
    for y in (0..h - 2).step_by(4) {
        for x in (0..w - 2).step_by(4) {
            for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                g[(y + dy) * w + x + dx] = 20;
            }
        }
    }
    survives(w, h, &g);
    // Larger specks on a large page.
    let (w, h) = (2400, 3200);
    let mut g = vec![235u8; w * h];
    for y in (0..h - 6).step_by(16) {
        for x in (0..w - 6).step_by(16) {
            for dy in 0..6 {
                g[(y + dy) * w + x..(y + dy) * w + x + 6].fill(20);
            }
        }
    }
    survives(w, h, &g);
}

#[test]
fn a_page_of_dark_stripes_is_survived() {
    let (w, h) = (600, 800);
    let mut g = vec![235u8; w * h];
    for y in 0..h {
        if (y / 6) % 2 == 0 {
            g[y * w..(y + 1) * w].fill(20);
        }
    }
    survives(w, h, &g);
    for x in 0..w {
        if (x / 6) % 2 == 0 {
            for y in 0..h {
                g[y * w + x] = 20;
            }
        }
    }
    survives(w, h, &g);
}

#[test]
fn very_large_frames_are_survived() {
    // 48 million pixels, flat and with a word on it.
    let (w, h) = (8000, 6000);
    let mut g = vec![235u8; w * h];
    survives(w, h, &g);
    let all = strips();
    paste(&mut g, w, (3000, 2800), &all[0], 20);
    survives(w, h, &g);
}

#[test]
fn very_long_and_thin_frames_are_survived() {
    for (w, h) in [
        (20_000, 1),
        (1, 20_000),
        (15_000, 3),
        (3, 15_000),
        (20_000, 20),
        (20, 20_000),
    ] {
        let flat = vec![200u8; w * h];
        assert!(survives(w, h, &flat).lists.is_empty());
        survives(w, h, &noise(w * h, 7));
    }
}

#[test]
fn a_frame_with_a_side_over_the_bound_is_refused() {
    let flat = vec![200u8; MAX_SIDE + 1];
    assert_eq!(find_words(MAX_SIDE + 1, 1, &flat).err(), Some(Error::Size));
    assert_eq!(read_page(1, MAX_SIDE + 1, &flat).err(), Some(Error::Size));
    // At the bound it is read.
    assert!(survives(MAX_SIDE, 1, &flat[..MAX_SIDE]).lists.is_empty());
}

// --- Limits -----------------------------------------------------------------

fn boxes(n: usize) -> Vec<Rect> {
    (0..n)
        .map(|i| Rect {
            x0: 0.0,
            y0: i as f32 * 30.0,
            x1: 50.0,
            y1: i as f32 * 30.0 + 20.0,
        })
        .collect()
}

#[test]
fn more_than_the_most_boxes_or_kernels_is_too_many() {
    let found = |words, numbers, too_many| Found {
        words: boxes(words),
        numbers: boxes(numbers),
        too_many,
        tall: Vec::new(),
    };
    assert!(!over_limit(&found(MAX_READ, MAX_READ, false)));
    assert!(over_limit(&found(MAX_READ + 1, 0, false)));
    assert!(over_limit(&found(0, MAX_READ + 1, false)));
    assert!(over_limit(&found(1, 1, true)));
}

#[test]
fn more_kernels_than_the_most_are_not_listed() {
    // Pairs of lit pixels on a made-up map, every second row, three columns apart.
    let n = Maps::ROWS * Maps::COLUMNS;
    let mut data = vec![0.0f32; DETECTOR_OUTPUTS * n];
    let mut pairs = 0;
    for row in (0..Maps::ROWS).step_by(2) {
        for col in (0..Maps::COLUMNS - 1).step_by(3) {
            if pairs < MAX_KERNELS + 1 {
                data[row * Maps::COLUMNS + col] = 1.0;
                data[row * Maps::COLUMNS + col + 1] = 1.0;
                pairs += 1;
            }
        }
    }
    let many = Maps {
        data: Zeroizing::new(data.clone()),
    };
    assert!(kernels(&many, 0).is_none());
    // One fewer is listed.
    let mut fewer = data;
    fewer[0] = 0.0;
    fewer[1] = 0.0;
    let few = Maps {
        data: Zeroizing::new(fewer),
    };
    assert_eq!(kernels(&few, 0).map(|k| k.len()), Some(MAX_KERNELS));
}

#[test]
fn crops_of_too_many_pixels_together_are_too_many() {
    let (w, h) = (6000, 6000);
    let whole = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: w as f32,
        y1: h as f32,
    };
    assert!(within_crop_budget(w, h, &[whole; 4]));
    assert!(!within_crop_budget(w, h, &[whole; 9]));
    // Small boxes, many of them, are well within.
    assert!(within_crop_budget(w, h, &boxes(MAX_READ)));
}

// --- Boxes at the edge, tall boxes -------------------------------------------

fn line(x0: f32, x1: f32, y: f32) -> Rect {
    Rect {
        x0,
        y0: y,
        x1,
        y1: y + 20.0,
    }
}

#[test]
fn a_box_at_the_frame_or_beside_one_is_an_edge_box() {
    let (w, h) = (600, 800);
    let rects = [
        line(-10.0, 80.0, 100.0), // reaches past the left edge
        line(85.0, 150.0, 102.0), // its other piece, on its line, close
        line(40.0, 130.0, 200.0), // inside
        line(40.0, 130.0, 300.0),
        line(500.0, 600.0, 400.0), // touches the right edge
        line(330.0, 400.0, 400.0), // on that line but far from it
        line(40.0, 130.0, 780.0),  // touches the bottom edge
    ];
    assert_eq!(
        edge_flags(&rects, w, h),
        vec![true, true, false, false, true, false, true]
    );
}

#[test]
fn a_much_higher_box_may_hold_two_lines() {
    let mut rects = boxes(10);
    rects.push(Rect {
        x0: 0.0,
        y0: 400.0,
        x1: 50.0,
        y1: 445.0,
    });
    let flags = tall_flags(&rects);
    assert_eq!(flags.iter().filter(|&&t| t).count(), 1);
    assert!(flags.last().copied().unwrap_or(false));
    // Fewer than three boxes cannot tell.
    assert_eq!(tall_flags(&rects[9..]), vec![false, false]);
}

#[test]
fn a_number_box_at_the_edge_has_no_value_and_the_page_may_be_cut_off() {
    let all = strips();
    let (w, h) = (600, 800);
    let mut g = page(w, h);
    paste(&mut g, w, (-20, 100), &all[0], 2);
    paste(&mut g, w, (200, 100), &all[1], 2);
    let at_edge = Rect {
        x0: -20.0,
        y0: 100.0,
        x1: 60.0,
        y1: 164.0,
    };
    let inside = Rect {
        x0: 200.0,
        y0: 100.0,
        x1: 280.0,
        y1: 164.0,
    };
    let (n, cut) = read_numbers(w, h, &g, &[at_edge]).unwrap();
    assert!(cut);
    assert_eq!(n[0].value, None);
    let (_, cut) = read_numbers(w, h, &g, &[inside]).unwrap();
    assert!(!cut);
}

#[test]
fn a_second_look_that_loses_or_cuts_a_word_does_not_replace_the_first() {
    let found = |words: Vec<Rect>, numbers: Vec<Rect>| Found {
        words,
        numbers,
        too_many: false,
        tall: Vec::new(),
    };
    let (frame, region) = ((600, 800), (100, 100, 500, 700));
    let first = found(
        vec![line(150.0, 250.0, 200.0), line(150.0, 250.0, 300.0)],
        vec![line(110.0, 140.0, 200.0)],
    );
    // The same words, a little moved: stands.
    let same = found(
        vec![line(152.0, 252.0, 202.0), line(152.0, 252.0, 301.0)],
        vec![line(112.0, 142.0, 201.0)],
    );
    assert!(second_stands(&first, &same, region, frame));
    // One word lost, another split in two (same count): the first stays.
    let lost = found(
        vec![line(150.0, 200.0, 200.0), line(205.0, 250.0, 200.0)],
        vec![line(110.0, 140.0, 200.0)],
    );
    assert!(!second_stands(&first, &lost, region, frame));
    // A number lost.
    let no_number = found(same.words.clone(), vec![]);
    assert!(!second_stands(&first, &no_number, region, frame));
    // A word touching the region's border, which is not the frame's.
    let cut = found(
        vec![line(100.0, 250.0, 200.0), line(150.0, 250.0, 300.0)],
        vec![line(110.0, 140.0, 200.0)],
    );
    assert!(!second_stands(&first, &cut, region, frame));
    // The same border where it is the frame's: not a cut.
    let at_frame = (0, 100, 500, 700);
    assert!(second_stands(&first, &cut, at_frame, frame));
    // More words and numbers than the first found stand.
    let more = found(
        vec![
            line(150.0, 250.0, 200.0),
            line(150.0, 250.0, 300.0),
            line(150.0, 250.0, 400.0),
        ],
        vec![line(110.0, 140.0, 200.0), line(110.0, 140.0, 300.0)],
    );
    assert!(second_stands(&first, &more, region, frame));
}

// --- The number's digits ------------------------------------------------------

/// Logits of one class for each frame (log-probability 0.97, the rest share
/// the remainder); a frame of two classes gives each 0.48.
fn logits_of(frames: &[&[usize]]) -> Logits {
    let t = frames.len();
    let mut data = vec![0.0f32; CLASSES * t];
    for (ti, classes) in frames.iter().enumerate() {
        let top = 0.97 / classes.len() as f32;
        let rest = 0.03 / (CLASSES - classes.len()) as f32;
        for k in 0..CLASSES {
            data[k * t + ti] = if classes.contains(&k) { top } else { rest }.ln();
        }
    }
    Logits {
        data: Zeroizing::new(data),
        frames: t,
    }
}

#[test]
fn digit_strings_are_scored_by_what_the_frames_hold() {
    const D: usize = 27;
    let blank = [0usize];
    // "1": a one, a blank, a one is "11", not "1".
    let eleven = logits_of(&[&[D + 1], &blank, &[D + 1]]);
    let (one, two_ones, twelve) = (
        digits_score(&eleven, b"1"),
        digits_score(&eleven, b"11"),
        digits_score(&eleven, b"12"),
    );
    assert!(two_ones > one + 3.0 && two_ones > twelve + 3.0);
    // A one held over two frames is one "1".
    let held = logits_of(&[&[D + 1], &[D + 1]]);
    assert!(digits_score(&held, b"1") > digits_score(&held, b"11") + 3.0);
    // A one then a two is "12", and not "21" or "1".
    let twelve_frames = logits_of(&[&[D + 1], &[D + 2]]);
    let s12 = digits_score(&twelve_frames, b"12");
    assert!(s12 > digits_score(&twelve_frames, b"21") + 3.0);
    assert!(s12 > digits_score(&twelve_frames, b"1") + 3.0);
    // A one that could be a seven: both score alike, a two does not.
    let near = logits_of(&[&[D + 1, D + 7]]);
    let (a, b) = (digits_score(&near, b"1"), digits_score(&near, b"7"));
    assert!((a - b).abs() < 0.01, "{a} {b}");
    assert!(a > digits_score(&near, b"2") + 3.0);
}

// --- Kernels that may hold two lines ------------------------------------------

/// The map rows a height of 1.0 stands for in the network's output.
const HEIGHT_ROWS: f32 = 8.0;

/// A made-up map: each block `(first row, one past the last, first column,
/// one past the last, height in rows)` lit in the words' kernel channel, with
/// the network's height written over it.
fn lit_map(blocks: &[(usize, usize, usize, usize, f32)]) -> Maps {
    let n = Maps::ROWS * Maps::COLUMNS;
    let mut data = vec![0.0f32; DETECTOR_OUTPUTS * n];
    for &(y0, y1, x0, x1, h) in blocks {
        for y in y0..y1 {
            for x in x0..x1 {
                data[y * Maps::COLUMNS + x] = 1.0;
                data[7 * n + y * Maps::COLUMNS + x] = h / HEIGHT_ROWS;
            }
        }
    }
    Maps {
        data: Zeroizing::new(data),
    }
}

/// How many boxes the blocks join into, and how many of them are flagged.
fn two_lines_in(blocks: &[(usize, usize, usize, usize, f32)]) -> (usize, usize) {
    let boxes = join(kernels(&lit_map(blocks), 0).unwrap());
    let flags = two_line_flags(&boxes);
    (boxes.len(), flags.iter().filter(|&&t| t).count())
}

#[test]
fn a_kernel_reaching_over_more_than_a_line_is_flagged() {
    // One line: a kernel of 0.6 to 0.73 of the height (a line is 10 rows).
    assert_eq!(two_lines_in(&[(40, 46, 20, 60, 10.0)]), (1, 0));
    assert_eq!(two_lines_in(&[(40, 47, 20, 60, 9.6)]), (1, 0));
    // A kernel merged over two lines, 14 rows at a height of 1.25 lines.
    assert_eq!(two_lines_in(&[(40, 54, 20, 60, 12.5)]), (1, 1));
}

#[test]
fn two_tight_lines_bridged_into_one_kernel_are_flagged_at_any_height() {
    // Lines of 10 rows a line apart: the bridged kernel reaches 14.5 to 17.3
    // rows (0.45 to 0.73 of a line, and the pitch), and the network gives the
    // box 1.0 to 1.6 lines.
    for reach in [15, 16, 17] {
        for h in [10.0, 13.0, 16.0] {
            assert_eq!(
                two_lines_in(&[(40, 40 + reach, 20, 60, h)]),
                (1, 1),
                "reach {reach} height {h}"
            );
        }
    }
    // Two lines whose box the network gave as the two lines' whole (20 rows)
    // are not told by the kernel: the page's median rule is what asks.
    assert_eq!(two_lines_in(&[(40, 55, 20, 60, 20.0)]), (1, 0));
}

#[test]
fn two_kernels_joined_into_one_box_are_flagged_by_both_reaches() {
    // Two kernels of 4 rows, a row apart, on one line: one box reaching 9
    // rows of its 10.
    assert_eq!(
        two_lines_in(&[(40, 44, 20, 60, 10.0), (45, 49, 30, 70, 10.0)]),
        (1, 1)
    );
    // Two kernels of 2 rows, a row apart: joined, and no more than a line.
    assert_eq!(
        two_lines_in(&[(40, 42, 20, 60, 10.0), (43, 45, 30, 70, 10.0)]),
        (1, 0)
    );
    // Kernels 3 rows apart are two lines, not joined, and none is flagged.
    assert_eq!(
        two_lines_in(&[(40, 44, 20, 60, 10.0), (47, 51, 30, 70, 10.0)]),
        (2, 0)
    );
}

// --- Pages read from boxes given ---------------------------------------------

/// Strips of the published set pasted at scale 1, each `(x, y, strip)`, on a
/// light page; the boxes pasted, as the words found.
fn laid(w: usize, h: usize, all: &[Strip], spots: &[(isize, usize, usize)]) -> (Vec<u8>, Found) {
    let mut g = page(w, h);
    let mut words = Vec::new();
    for &(x, y, k) in spots {
        let (x0, y0, x1, y1) = paste(&mut g, w, (x, y), &all[k], 1);
        words.push(Rect {
            x0: x0 as f32,
            y0: y0 as f32,
            x1: x1 as f32,
            y1: y1 as f32,
        });
    }
    let found = Found {
        words,
        numbers: Vec::new(),
        too_many: false,
        tall: Vec::new(),
    };
    (g, found)
}

/// A column of `n` words from `x`, the strips 0 to 11 over again, a line
/// pitch of 56 from `y`.
fn down(x: isize, y: usize, n: usize) -> Vec<(isize, usize, usize)> {
    (0..n).map(|i| (x, y + i * 56, i % 12)).collect()
}

const GAP: usize = 200;

#[test]
fn lists_cut_apart_by_a_gap_that_fit_a_form_whole_are_asked_about() {
    let all = strips();
    let mut spots = Vec::new();
    for b in 0..3 {
        spots.extend(down(20, 20 + b * (12 * 56 + GAP), 12));
    }
    let (g, found) = laid(300, 2700, &all, &spots);
    let p = read_found(300, 2700, &g, &found, None).unwrap();
    assert_eq!(p.lists.len(), 3);
    for l in &p.lists {
        assert_eq!(l.words.len(), 12);
        assert!(l.questions.contains(&Question::Split), "{:?}", l.questions);
    }
    // Blocks of 12 and 7 words fit no form together, but one has a question
    // (7 is no count): both are asked about.
    let mut spots = down(20, 20, 12);
    spots.extend(down(20, 20 + 12 * 56 + GAP, 7));
    let (g, found) = laid(300, 1500, &all, &spots);
    let p = read_found(300, 1500, &g, &found, None).unwrap();
    assert_eq!(p.lists.len(), 2);
    assert!(
        p.lists
            .iter()
            .all(|l| l.questions.contains(&Question::Split))
    );
}

#[test]
fn four_columns_of_six_ask_whether_they_are_one_list() {
    let all = strips();
    let mut spots = Vec::new();
    for c in 0..4 {
        spots.extend(down(20 + c * 300, 20, 6));
    }
    let (g, found) = laid(1300, 400, &all, &spots);
    let p = read_found(1300, 400, &g, &found, None).unwrap();
    assert_eq!(p.lists.len(), 1);
    let l = &p.lists[0];
    assert_eq!(l.words.len(), 24);
    assert!(l.questions.contains(&Question::Split), "{:?}", l.questions);
    assert_eq!(l.form, None);
    assert_eq!(l.list, None);
}

#[test]
fn three_columns_of_four_twice_down_a_page_ask_about_each_part() {
    let all = strips();
    let mut spots = Vec::new();
    for part in 0..2 {
        for c in 0..3 {
            spots.extend(down(20 + c * 300, 20 + part * (4 * 56 + GAP), 4));
        }
    }
    let (g, found) = laid(1000, 900, &all, &spots);
    let p = read_found(1000, 900, &g, &found, None).unwrap();
    assert!(!p.lists.is_empty());
    for l in &p.lists {
        assert!(l.questions.contains(&Question::Split), "{:?}", l.questions);
        assert_eq!(l.form, None);
    }
}

#[test]
fn boxes_that_are_not_a_layout_ask_split_and_order() {
    // Two words on every line, each pair further right than the line above.
    let all = strips();
    let mut spots = Vec::new();
    for i in 0..8 {
        spots.push((10 + 20 * i as isize, 20 + i * 56, i));
        spots.push((120 + 20 * i as isize, 20 + i * 56, 8 + i % 4));
    }
    let (g, found) = laid(400, 500, &all, &spots);
    let p = read_found(400, 500, &g, &found, None).unwrap();
    let l = &p.lists[0];
    assert!(l.questions.contains(&Question::Split), "{:?}", l.questions);
    assert!(l.questions.contains(&Question::Order));
    assert_eq!(l.form, None);
}

#[test]
fn a_box_that_may_hold_two_lines_asks_missing_and_keeps_its_candidates() {
    let all = strips();
    let (g, mut found) = laid(300, 800, &all, &down(20, 20, 12));
    found.tall = vec![false; 12];
    found.tall[3] = true;
    let p = read_found(300, 800, &g, &found, None).unwrap();
    let l = &p.lists[0];
    assert!(l.questions.contains(&Question::Missing));
    assert_eq!((l.form, l.list), (None, None));
    let tall = &l.words[3];
    assert!(tall.tall && tall.clear(0).is_none());
    match tall.reading(0).unwrap() {
        Reading::Unclear(c) => assert!(c.len() > 1, "{} candidates", c.len()),
        Reading::Clear(_) => panic!("a tall box read as clear"),
    }
    // Without the flag the same page holds a form.
    found.tall.clear();
    let p = read_found(300, 800, &g, &found, None).unwrap();
    assert_eq!(p.lists[0].form, Some(Form::Bip39));
}

#[test]
fn a_box_over_blank_paper_lowers_the_fit_of_every_list() {
    let all = strips();
    let mut spots = down(20, 20, 12);
    let blank = (20, 20 + 5 * 56);
    spots.remove(5);
    let (g, mut found) = laid(300, 800, &all, &spots);
    found.words.insert(
        5,
        Rect {
            x0: blank.0 as f32,
            y0: blank.1 as f32,
            x1: 120.0,
            y1: (blank.1 + 32) as f32,
        },
    );
    let p = read_found(300, 800, &g, &found, None).unwrap();
    let l = &p.lists[0];
    assert_eq!(l.words.len(), 12);
    for k in 0..p.list_names.len() {
        assert!(l.fit[k] < -1.5, "fit {} in list {k}", l.fit[k]);
    }
}

#[test]
fn a_word_with_no_gap_or_a_huge_one_counts_as_the_worst_in_the_mean_fit() {
    let rows: [&[Option<f32>]; 3] = [&[Some(0.0)], &[Some(1e9)], &[None]];
    let fit = mean_fit(&rows, 1);
    assert_eq!(fit.len(), 1);
    assert!(
        (fit[0] + 2.0 * crate::BEAM / 3.0).abs() < 1e-3,
        "{}",
        fit[0]
    );
    let rows: [&[Option<f32>]; 2] = [&[Some(f32::NAN)], &[Some(0.0)]];
    assert!((mean_fit(&rows, 1)[0] + crate::BEAM / 2.0).abs() < 1e-3);
    // No words, or no reading in a list: the worst.
    assert_eq!(mean_fit(&[], 2)[1], -crate::BEAM);
}

#[test]
fn boxes_of_noise_ask_the_form_and_the_right_strips_do_not() {
    let all = strips();
    let spots = down(20, 20, 12);
    // Twelve boxes of random pixels.
    let (mut g, found) = laid(300, 800, &all, &spots);
    for r in &found.words {
        let (x0, y0, x1, y1) = (r.x0 as usize, r.y0 as usize, r.x1 as usize, r.y1 as usize);
        let n = noise((x1 - x0) * (y1 - y0), 7 + y0 as u64);
        for (k, y) in (y0..y1).enumerate() {
            let row = &n[k * (x1 - x0)..(k + 1) * (x1 - x0)];
            g[y * 300 + x0..y * 300 + x1].copy_from_slice(row);
        }
    }
    let p = read_found(300, 800, &g, &found, None).unwrap();
    let l = &p.lists[0];
    assert!(l.questions.contains(&Question::Form), "{:?}", l.questions);
    assert_eq!((l.form, l.list), (None, None));
    // Twelve copies of the published strip that holds noise.
    let spots: Vec<(isize, usize, usize)> = (0..12).map(|i| (20, 20 + i * 56, 61)).collect();
    let (g, found) = laid(300, 800, &all, &spots);
    let p = read_found(300, 800, &g, &found, None).unwrap();
    let l = &p.lists[0];
    assert!(l.questions.contains(&Question::Form), "{:?}", l.questions);
    assert_eq!((l.form, l.list), (None, None));
    // The twelve published strips of a BIP39 list do not.
    let (g, found) = laid(300, 800, &all, &down(20, 20, 12));
    let p = read_found(300, 800, &g, &found, None).unwrap();
    let l = &p.lists[0];
    assert!(!l.questions.contains(&Question::Form), "{:?}", l.questions);
    assert_eq!(l.form, Some(Form::Bip39));
}

#[test]
fn a_clear_reading_in_an_extra_list_is_the_word_written() {
    let all = strips();
    let (w, h, g) = one_column(&all, 12);
    let list: Vec<&str> = all.iter().take(6).map(Strip::word).collect();
    let extra = Extra {
        words: &list,
        counts: &[12],
    };
    let p = read_page_with(w, h, &g, Some(extra)).unwrap();
    let l = &p.lists[0];
    let mut clear = 0;
    for (i, wd) in l.words.iter().enumerate() {
        if let Some(c) = wd.clear(2) {
            assert!(i < 6, "word {i} read as {c}, which is not in its list");
            assert_eq!(c, all[i].word());
            clear += 1;
        }
    }
    assert!(clear >= 2, "only {clear} of the six read clearly");
}

#[test]
fn a_number_beside_the_frame_is_cut_and_one_inside_reads() {
    // Strip 37 was written as "14wish": its first 30% is the number.
    let all = strips();
    assert_eq!(all[37].word(), "wish");
    let (w, h) = (600, 800);
    let box_at = |x: f32| Rect {
        x0: x,
        y0: 100.0,
        x1: x + 0.3 * (all[37].width * 2) as f32,
        y1: 164.0,
    };
    let mut g = page(w, h);
    paste(&mut g, w, (200, 100), &all[37], 2);
    let (n, cut) = read_numbers(w, h, &g, &[box_at(200.0)]).unwrap();
    assert!(!cut);
    assert_eq!(n[0].value, Some(14));
    let mut g = page(w, h);
    paste(&mut g, w, (4, 100), &all[37], 2);
    let (n, cut) = read_numbers(w, h, &g, &[box_at(0.0)]).unwrap();
    assert!(cut);
    assert_eq!(n[0].value, None);
}
