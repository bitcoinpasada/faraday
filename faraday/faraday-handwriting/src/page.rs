//! Reading a whole page: the words found, each read once by the network,
//! scored against the word lists of the forms, put in order, and the form
//! chosen by the number of words and the list they fit.
//!
//! No checksum is used, to choose a word, a form or an order. What cannot be
//! decided is returned as a named `Question` with everything found.
//!
//! Work is bounded for any frame: two detections (see `find_words`), at most
//! `MAX_READ` words and `MAX_READ` numbers read (more is reported as
//! `Page::too_many`, nothing read), each crop at most `MAX_CROP_PIXELS` and
//! all the crops together at most `MAX_TOTAL_CROP_PIXELS` (more is
//! `Page::too_many` too).

use core::fmt;

use zeroize::Zeroizing;

use crate::decode::logadd;
use crate::find::{Found, Rect, find_words};
use crate::order::{Item, MAX_NUMBER, arrange, pair_numbers};
use crate::read::{CLEAR_FREE_GAP, CLEAR_MARGIN, Logged, Reading, log_crop, path_gap};
use crate::word::Logits;
use crate::{BEAM, Error, bip39_words, slip39_words};

/// Most words, and most numbers, a page may hold to be read.
pub const MAX_READ: usize = 96;
/// Most pixels of a word's crop; a larger crop is reduced by a whole factor.
pub const MAX_CROP_PIXELS: usize = 1 << 22;
/// Most pixels of all the crops of a page together (words and numbers); a
/// page that would hold more is `Page::too_many`.
pub const MAX_TOTAL_CROP_PIXELS: usize = 1 << 25;
/// A crop reaches this share of the box's height past it, on every side.
const CROP_MARGIN: f32 = 0.3;
/// Another list fits the words better than the one of the form when its mean
/// score is more than this many log units higher per word.
///
/// Provisional: not measured on real handwriting.
pub const LIST_MARGIN: f32 = 1.0;
/// A word's gap from the network's free reading (how far below it the best
/// word's likeliest alignment is) counts for no more than this in the mean fit
/// of a list: no one box erases the others, and a word with no reading, or
/// whose crop cannot be read, counts as the worst.
const GAP_CLAMP: f32 = BEAM;
/// A list whose mean fit (minus the mean gap) is below this is not the list of
/// the words: a form is not chosen by count alone (`Question::Form`).
///
/// Provisional: set from what the published strips and the synthetic pages of
/// the tests give (see the README), not from real handwriting.
pub const FIT_FLOOR: f32 = -5.0;
/// A box more than this many times as high as the median box of the page may
/// hold two lines (a second test beside the box's own kernels, `Found::tall`).
const TALL_BOXES: f32 = 1.6;
/// A box beside one that touches the frame's edge, on its line and no further
/// from it than this many heights, may be a piece of the same word.
const EDGE_PIECE_HEIGHTS: f32 = 1.0;
/// Index of a class: the first digit.
const FIRST_DIGIT: usize = 27;

/// The forms the page can be (a count of words and a list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// 12, 15, 18, 21 or 24 words of the English BIP39 list (a Seed XOR
    /// part is one too).
    Bip39,
    /// 20, 22, 23, 25, 27, 28, 30, 31 or 33 words of the SLIP-39 list.
    Slip39,
    /// The form the caller gave (`Extra`).
    Extra,
}

/// One more list and set of counts than the built-in forms (for instance the
/// SLIP-39 list with 36, 48 and 56 words), given by the caller.
#[derive(Debug, Clone, Copy)]
pub struct Extra<'a> {
    /// The list (lowercase a-z, none empty or repeated).
    pub words: &'a [&'a str],
    /// The counts of words of the form.
    pub counts: &'a [usize],
}

/// The counts of a BIP39 phrase: from the shortest to the longest of
/// `faraday_bip`'s rule, a word more for every 32 bits (three words).
pub fn bip39_counts() -> Vec<usize> {
    (faraday_bip::bip39::MIN_WORDS..=faraday_bip::bip39::MAX_WORDS)
        .step_by(3)
        .collect()
}

/// The counts of a SLIP-39 share: a secret of 16 to 32 bytes in steps of two
/// is `ceil(8 * bytes / 10)` words of 10 bits, and the header and the
/// checksum add what the shortest share has besides (so the shortest is 20
/// and the longest 33).
pub fn slip39_counts() -> Vec<usize> {
    use faraday_bip::slip39::{MAX_STRENGTH_BYTES, MIN_STRENGTH_BYTES, MIN_WORDS, RADIX_BITS};
    let value_words = |bytes: usize| (bytes * 8).div_ceil(RADIX_BITS);
    let fixed = MIN_WORDS.saturating_sub(value_words(MIN_STRENGTH_BYTES));
    (MIN_STRENGTH_BYTES..=MAX_STRENGTH_BYTES)
        .step_by(2)
        .map(|b| fixed + value_words(b))
        .collect()
}

/// A question the page could not decide. What was found stays in the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// The words are in several columns without usable numbers, or the layout
    /// could not be told: `List::words` is in the order `List::order` names and
    /// `List::across` is the other one; which one is the list is for the
    /// person.
    Order,
    /// The number of words fits no form.
    Count,
    /// The number of words fits a form whose list fits the words worse than
    /// another list (`List::fit`), or the count fits several forms, or no
    /// word of the list reads clear (noise boxes are a form by count).
    Form,
    /// A line number read is not its word's place plus one in `List::words`,
    /// or a number stands beside no word.
    Numbers,
    /// The page may hold one list or several, and the words may be split
    /// otherwise: lists cut apart only by a vertical gap where any run of two
    /// or more of them together fits a form or one has a question; columns
    /// side by side that no number tells apart, where a run of them that is not
    /// all, or a part above or below a blank band across them all (or a run
    /// of those parts), fits a form; or a layout that could not be told. The
    /// form is left open (`List::form` is none) for the last two.
    Split,
    /// A box may hold two lines, by its kernels (`Found::tall`) or because it
    /// is much higher than the others: a word may be missing, so the form is
    /// left open.
    Missing,
}

/// A number read beside a word.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number {
    /// 1 to 60, if the network read it clearly (by the rule of `read_crop`).
    pub value: Option<u8>,
    /// The number's box in the frame.
    pub rect: Rect,
}

/// A grey crop at the frame's resolution, wiped when dropped.
pub struct Crop {
    /// Columns.
    pub width: usize,
    /// Rows.
    pub height: usize,
    /// Row-major, 0 black to 255 white.
    pub gray: Zeroizing<Vec<u8>>,
}

impl fmt::Debug for Crop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Crop(..)")
    }
}

/// A word found on the page.
pub struct PageWord {
    /// The word's crop, with a margin.
    pub crop: Crop,
    /// The word's box in the frame (it may reach past the frame).
    pub rect: Rect,
    /// The line number beside the word, if one was found.
    pub number: Option<Number>,
    /// The box touches the frame's edge, or is a piece beside one that does
    /// on its line: the word may be cut off, and no reading of it is clear.
    pub edge: bool,
    /// The box may hold two lines (its kernels reach over more of its height
    /// than one line's, or it is much higher than the page's others): no
    /// reading of it is clear.
    pub tall: bool,
    /// The reading against each list of the forms (`Page::list_names`).
    pub readings: Vec<Reading>,
}

impl PageWord {
    /// The word's reading against list `list`.
    pub fn reading(&self, list: usize) -> Option<&Reading> {
        self.readings.get(list)
    }

    /// The word, if it reads clearly in list `list` (an edge word or a tall
    /// box never does).
    pub fn clear(&self, list: usize) -> Option<&str> {
        match self.readings.get(list)? {
            Reading::Clear(c) => Some(c.word()),
            _ => None,
        }
    }
}

impl fmt::Debug for PageWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PageWord(..)")
    }
}

/// The order a list's words are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Down the columns, left to right (one column: top to bottom), or by the
    /// line numbers read.
    Columns,
    /// Row by row across the columns: the numbers read run that way.
    Rows,
}

/// A list of words of the page.
pub struct List {
    /// The words in the order `order` names.
    pub words: Vec<PageWord>,
    /// Which order `words` is in.
    pub order: Order,
    /// The other order, as indices of `words`, when the order is a question
    /// (row by row if `order` is `Columns`, down the columns if it is `Rows`).
    pub across: Option<Vec<usize>>,
    /// The form, if the count and the list decide one.
    pub form: Option<Form>,
    /// The index in the page's lists of the form's list.
    pub list: Option<usize>,
    /// How well the words fit each list, in the page's lists: minus the mean,
    /// over the words, of how far below the network's free reading the best
    /// word's likeliest alignment in the list is (0 or less; 0 is as good as
    /// the free reading, lower is worse). Each word's gap counts for at most
    /// `BEAM` and a word with no reading for that.
    pub fit: Zeroizing<Vec<f32>>,
    /// What could not be decided.
    pub questions: Vec<Question>,
}

impl fmt::Debug for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "List({} words)", self.words.len())
    }
}

/// What a page holds.
pub struct Page {
    /// The lists found.
    pub lists: Vec<List>,
    /// The lists the readings are against: English BIP39, SLIP-39 and the
    /// caller's, in that order (the last only if given).
    pub list_names: Vec<Form>,
    /// A word touches the frame's edge: the page may be cut off.
    pub maybe_cut_off: bool,
    /// Line numbers that go with no word.
    pub unattached_numbers: usize,
    /// More than `MAX_READ` words or numbers (or kernels), or crops of more
    /// than `MAX_TOTAL_CROP_PIXELS` together: nothing was read.
    pub too_many: bool,
}

impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page({} lists)", self.lists.len())
    }
}

/// Where the crop of a box lies: left, top, columns, rows and the factor of
/// reduction (a whole factor, more than 1 only if the crop would hold more
/// than `MAX_CROP_PIXELS`), and the size of the result.
struct CropPlan {
    x0: usize,
    y0: usize,
    f: usize,
    ow: usize,
    oh: usize,
}

fn plan_crop(w: usize, h: usize, r: &Rect) -> Option<CropPlan> {
    let m = r.height().max(1.0) * CROP_MARGIN;
    let lo = |v: f32, max: usize| (v.max(0.0) as usize).min(max);
    let (x0, y0) = (lo(r.x0 - m, w), lo(r.y0 - m, h));
    let (x1, y1) = (lo(r.x1 + m + 1.0, w), lo(r.y1 + m + 1.0, h));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let (cw, ch) = (x1 - x0, y1 - y0);
    let mut f = 1usize;
    while (cw / f).max(1) * (ch / f).max(1) > MAX_CROP_PIXELS {
        f += 1;
    }
    Some(CropPlan {
        x0,
        y0,
        f,
        ow: (cw / f).max(1),
        oh: (ch / f).max(1),
    })
}

/// The crops of the boxes together hold no more than
/// `MAX_TOTAL_CROP_PIXELS`.
pub(crate) fn within_crop_budget(w: usize, h: usize, boxes: &[Rect]) -> bool {
    let mut total = 0usize;
    for r in boxes {
        if let Some(p) = plan_crop(w, h, r) {
            total = total.saturating_add(p.ow * p.oh);
        }
    }
    total <= MAX_TOTAL_CROP_PIXELS
}

/// The crop of a box with its margin, at the frame's resolution (a whole
/// factor smaller only if it would hold more than `MAX_CROP_PIXELS`).
fn crop_of(w: usize, h: usize, gray: &[u8], r: &Rect) -> Option<Crop> {
    let CropPlan { x0, y0, f, ow, oh } = plan_crop(w, h, r)?;
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(ow * oh));
    for oy in 0..oh {
        for ox in 0..ow {
            let mut sum = 0usize;
            let mut n = 0usize;
            for dy in 0..f {
                let Some(row) = gray.get((y0 + oy * f + dy) * w..(y0 + oy * f + dy + 1) * w) else {
                    continue;
                };
                for dx in 0..f {
                    if let Some(&v) = row.get(x0 + ox * f + dx) {
                        sum += usize::from(v);
                        n += 1;
                    }
                }
            }
            out.push(sum.checked_div(n).unwrap_or(255) as u8);
        }
    }
    Some(Crop {
        width: ow,
        height: oh,
        gray: out,
    })
}

/// The log-likelihood of the digit string `digits` (the CTC forward pass),
/// the blank and the digit classes only.
pub(crate) fn digits_score(lp: &Logits, digits: &[u8]) -> f32 {
    const NEG: f32 = -1e30;
    let frames = lp.frames();
    let v = lp.values();
    let at = |class: usize, t: usize| v.get(class * frames + t).copied().unwrap_or(NEG);
    let class_of = |s: usize| -> usize {
        if s.is_multiple_of(2) {
            0
        } else {
            digits
                .get(s / 2)
                .map_or(0, |&d| FIRST_DIGIT + usize::from(d - b'0'))
        }
    };
    let states = 2 * digits.len() + 1;
    let mut alpha = vec![NEG; states];
    let mut next = vec![NEG; states];
    for (s, a) in alpha.iter_mut().enumerate().take(2) {
        *a = at(class_of(s), 0);
    }
    for t in 1..frames {
        for s in 0..states {
            let mut a = alpha.get(s).copied().unwrap_or(NEG);
            if s >= 1 {
                a = logadd(a, alpha.get(s - 1).copied().unwrap_or(NEG));
            }
            if s >= 2 && class_of(s) != 0 && class_of(s) != class_of(s - 2) {
                a = logadd(a, alpha.get(s - 2).copied().unwrap_or(NEG));
            }
            if let Some(n) = next.get_mut(s) {
                *n = a + at(class_of(s), t);
            }
        }
        core::mem::swap(&mut alpha, &mut next);
    }
    let last = alpha.get(states - 1).copied().unwrap_or(NEG);
    let before = alpha.get(states - 2).copied().unwrap_or(NEG);
    logadd(last, before)
}

/// The line number in a crop, 1 to 60, if it reads clearly: the best of the
/// 60 numbers leads the second by `CLEAR_MARGIN`, its own likeliest alignment
/// is within `CLEAR_FREE_GAP` of the network's free reading, and little ink is
/// left unread (the rule of `read_crop`).
fn read_number(c: &Crop) -> Result<Option<u8>, Error> {
    let Some(logged) = log_crop(c.width, c.height, &c.gray)? else {
        return Ok(None);
    };
    let mut best = (f32::NEG_INFINITY, 0u8);
    let mut second = f32::NEG_INFINITY;
    for n in 1..=MAX_NUMBER {
        let tens = n / 10;
        let digits = [b'0' + tens, b'0' + n % 10];
        let text = if tens == 0 { &digits[1..] } else { &digits[..] };
        let s = digits_score(&logged.lp, text);
        if s > best.0 {
            second = best.0;
            best = (s, n);
        } else if s > second {
            second = s;
        }
    }
    let (tens, ones) = (usize::from(best.1 / 10), usize::from(best.1 % 10));
    let classes: &[usize] = if tens == 0 {
        &[FIRST_DIGIT + ones]
    } else {
        &[FIRST_DIGIT + tens, FIRST_DIGIT + ones]
    };
    let clear = best.0 - second >= CLEAR_MARGIN
        && logged.little_unread()
        && path_gap(&logged.lp, logged.free, classes, false).is_some_and(|g| g <= CLEAR_FREE_GAP);
    Ok(clear.then_some(best.1))
}

/// A form of the table: its id, its list (index in the page's lists), counts.
pub(crate) struct Row {
    pub(crate) form: Form,
    pub(crate) list: usize,
    pub(crate) counts: Vec<usize>,
}

/// The form of a list of `n` words whose mean fit in each list is `fit`: the
/// form whose counts hold `n` and whose list the words fit, as its form and
/// list. `Count` if no form holds `n`; `Form` if several forms hold it and
/// their lists fit alike, or another list fits the words better than the
/// form's by more than `LIST_MARGIN`, or the form's list fits the words worse
/// than `FIT_FLOOR` (no reading at all counts as the worst fit). No checksum
/// is used.
pub(crate) fn choose_form(n: usize, fit: &[f32], table: &[Row]) -> Result<(Form, usize), Question> {
    let fitting: Vec<&Row> = table.iter().filter(|r| r.counts.contains(&n)).collect();
    let fit_of = |list: usize| fit.get(list).copied().unwrap_or(-1e9);
    let Some(&first) = fitting.first() else {
        return Err(Question::Count);
    };
    let mut best = first;
    let mut tied = false;
    for r in fitting.iter().skip(1) {
        if fit_of(r.list) > fit_of(best.list) + LIST_MARGIN {
            best = r;
            tied = false;
        } else if fit_of(r.list) + LIST_MARGIN >= fit_of(best.list) && r.list != best.list {
            tied = true;
        }
    }
    let other_better = (0..fit.len()).any(|k| fit_of(k) > fit_of(best.list) + LIST_MARGIN);
    if tied || other_better || fit_of(best.list) < FIT_FLOOR {
        return Err(Question::Form);
    }
    Ok((best.form, best.list))
}

/// How well each of `lists` fits the words whose gaps from the free reading
/// are `rows` (for each word, for each list: how far below the free reading
/// the best word's likeliest alignment is): minus the mean gap, each clamped to
/// `GAP_CLAMP`, a word with no gap counting as `GAP_CLAMP`. 0 or less.
pub(crate) fn mean_fit(rows: &[&[Option<f32>]], lists: usize) -> Zeroizing<Vec<f32>> {
    Zeroizing::new(
        (0..lists)
            .map(|k| {
                let sum: f32 = rows
                    .iter()
                    .map(|gp| {
                        gp.get(k)
                            .copied()
                            .flatten()
                            .filter(|g| !g.is_nan())
                            .unwrap_or(GAP_CLAMP)
                            .clamp(0.0, GAP_CLAMP)
                    })
                    .sum();
                if rows.is_empty() {
                    -GAP_CLAMP
                } else {
                    -sum / rows.len() as f32
                }
            })
            .collect(),
    )
}

/// Whether some run of neighbouring parts of `sizes` (at least `shortest`
/// parts, all of them only if `whole`) holds as many words as a form's count.
fn some_run_fits(sizes: &[usize], table: &[Row], shortest: usize, whole: bool) -> bool {
    (0..sizes.len()).any(|from| {
        let mut sum = 0usize;
        sizes.iter().enumerate().skip(from).any(|(to, &n)| {
            sum = sum.saturating_add(n);
            let parts = to - from + 1;
            parts >= shortest
                && (whole || parts < sizes.len())
                && table.iter().any(|r| r.counts.contains(&sum))
        })
    })
}

/// More than a page may hold to be read: too many boxes or kernels.
pub(crate) fn over_limit(found: &Found) -> bool {
    found.too_many || found.words.len() > MAX_READ || found.numbers.len() > MAX_READ
}

fn touches_frame(r: &Rect, w: usize, h: usize) -> bool {
    r.x0 <= 0.0 || r.y0 <= 0.0 || r.x1 >= w as f32 || r.y1 >= h as f32
}

/// For each box, whether the word may be cut off by the frame: it touches the
/// frame's edge, or it is on the line of one that does, close enough to be
/// a piece of the same word.
pub(crate) fn edge_flags(boxes: &[Rect], w: usize, h: usize) -> Vec<bool> {
    let touch: Vec<bool> = boxes.iter().map(|r| touches_frame(r, w, h)).collect();
    boxes
        .iter()
        .enumerate()
        .map(|(i, a)| {
            touch.get(i).copied().unwrap_or(false)
                || boxes.iter().zip(&touch).any(|(b, &t)| {
                    let low = a.height().min(b.height());
                    let tall = a.height().max(b.height());
                    let across = a.y1.min(b.y1) - a.y0.max(b.y0);
                    let gap = (a.x0 - b.x1).max(b.x0 - a.x1).max(0.0);
                    t && across >= 0.5 * low && gap <= EDGE_PIECE_HEIGHTS * tall
                })
        })
        .collect()
}

/// For each box, whether it is much higher than the median box (it may hold
/// two lines). Three boxes at least are needed to tell.
pub(crate) fn tall_flags(boxes: &[Rect]) -> Vec<bool> {
    let mut heights: Vec<f32> = boxes.iter().map(Rect::height).collect();
    if heights.len() < 3 {
        return vec![false; boxes.len()];
    }
    heights.sort_by(f32::total_cmp);
    let n = heights.len();
    let med = (heights.get((n - 1) / 2).copied().unwrap_or(0.0)
        + heights.get(n / 2).copied().unwrap_or(0.0))
        / 2.0;
    boxes
        .iter()
        .map(|r| r.height() > TALL_BOXES * med)
        .collect()
}

/// Each number box read once; whether one touches the frame's edge, where it
/// may be cut ("12" read as "2"): such a number has no value.
pub(crate) fn read_numbers(
    w: usize,
    h: usize,
    gray: &[u8],
    boxes: &[Rect],
) -> Result<(Vec<Number>, bool), Error> {
    let mut numbers: Vec<Number> = Vec::new();
    let mut cut_off = false;
    for r in boxes {
        let cut = touches_frame(r, w, h);
        cut_off |= cut;
        let value = match crop_of(w, h, gray, r) {
            Some(c) if !cut => read_number(&c)?,
            _ => None,
        };
        numbers.push(Number { value, rect: *r });
    }
    Ok((numbers, cut_off))
}

/// Costs `read_page_with`: two detector runs and one run of the word network
/// for each word and each number found, scoring against each list.
///
/// Reads a page (see `read_page_with`) with the built-in forms.
pub fn read_page(w: usize, h: usize, gray: &[u8]) -> Result<Page, Error> {
    read_page_with(w, h, gray, None)
}

/// Costs two runs of the detector (`find_words`), then one run of the word
/// network and a decode against each list for each word, and one run for each
/// number (up to `MAX_READ` of each, so a page is read in seconds on the
/// Pi's panel); reads the page in a frame (`w` by `h`, row-major, 0 black to
/// 255 white): finds the words and numbers (`find_words`), reads each word once with the
/// network and judges it against each list by the rule of `read_crop` (clear
/// or unclear), puts the words in order, and chooses the form.
///
/// The form is the one whose counts hold the number of words (BIP39 and
/// SLIP-39 counts do not overlap) and whose list the words fit; a count that
/// fits no form, or fits one whose list fits the words worse than another
/// list by `LIST_MARGIN` per word, or fits no list (a mean fit under
/// `FIT_FLOOR`), is a `Question` on that list with all the words in it. A box
/// touching the frame's edge, or standing beside one that does on its line,
/// makes its word unclear (the candidates are kept) and the page
/// `maybe_cut_off`; a number box at the edge has no value. A box much higher
/// than the page's others is unclear too and the list asks `Missing`.
/// `extra` adds a list and counts to the table.
///
/// Work: see the module. A frame of any size, flat, noise or all dark never
/// panics; a side over `MAX_SIDE` is `Error::Size`.
pub fn read_page_with(
    w: usize,
    h: usize,
    gray: &[u8],
    extra: Option<Extra<'_>>,
) -> Result<Page, Error> {
    let found = find_words(w, h, gray)?;
    read_found(w, h, gray, &found, extra)
}

/// Reads the page `found` stands for in the frame (`read_page_with` after the
/// detection).
///
/// Costs one run of the word network for each word and each number found,
/// scoring against each list.
pub(crate) fn read_found(
    w: usize,
    h: usize,
    gray: &[u8],
    found: &Found,
    extra: Option<Extra<'_>>,
) -> Result<Page, Error> {
    let slip = slip39_words();
    let mut lists: Vec<&[&str]> = vec![bip39_words(), slip.as_slice()];
    let mut names = vec![Form::Bip39, Form::Slip39];
    let mut table = vec![
        Row {
            form: Form::Bip39,
            list: 0,
            counts: bip39_counts(),
        },
        Row {
            form: Form::Slip39,
            list: 1,
            counts: slip39_counts(),
        },
    ];
    if let Some(e) = extra {
        lists.push(e.words);
        names.push(Form::Extra);
        table.push(Row {
            form: Form::Extra,
            list: 2,
            counts: e.counts.to_vec(),
        });
    }
    for l in &lists {
        crate::decode::check_list(l)?;
    }
    let mut page = Page {
        lists: Vec::new(),
        list_names: names,
        maybe_cut_off: false,
        unattached_numbers: 0,
        too_many: over_limit(found)
            || !within_crop_budget(
                w,
                h,
                &found
                    .words
                    .iter()
                    .chain(found.numbers.iter())
                    .copied()
                    .collect::<Vec<_>>(),
            ),
    };
    if page.too_many {
        return Ok(page);
    }
    let (paired, unattached) = pair_numbers(&found.words, &found.numbers);
    page.unattached_numbers = unattached;

    // Each number once, each word once.
    let (numbers, number_cut) = read_numbers(w, h, gray, &found.numbers)?;
    page.maybe_cut_off |= number_cut;
    let edges = edge_flags(&found.words, w, h);
    let talls = tall_flags(&found.words);
    let mut words: Vec<Option<PageWord>> = Vec::new();
    let mut gaps: Vec<Vec<Option<f32>>> = Vec::new();
    for (i, r) in found.words.iter().enumerate() {
        let crop = crop_of(w, h, gray, r).unwrap_or(Crop {
            width: 0,
            height: 0,
            gray: Zeroizing::new(Vec::new()),
        });
        let logged: Option<Logged> = log_crop(crop.width, crop.height, &crop.gray)?;
        let edge = edges.get(i).copied().unwrap_or(false);
        let tall =
            talls.get(i).copied().unwrap_or(false) || found.tall.get(i).copied().unwrap_or(false);
        page.maybe_cut_off |= edge;
        // An edge word or a tall box keeps the candidates an unclear reading
        // lists, and is never clear.
        let mut readings = Vec::new();
        let mut gap = Vec::new();
        for l in &lists {
            match &logged {
                Some(lg) => {
                    let (rd, g) = lg.judge_gap(l, edge || tall)?;
                    readings.push(rd);
                    gap.push(g);
                }
                None => {
                    readings.push(Reading::Unclear(Zeroizing::new(Vec::new())));
                    gap.push(None);
                }
            }
        }
        let number = paired
            .get(i)
            .copied()
            .flatten()
            .and_then(|n| numbers.get(n).copied());
        words.push(Some(PageWord {
            crop,
            rect: *r,
            number,
            edge,
            tall,
            readings,
        }));
        gaps.push(gap);
    }

    let items: Vec<Item> = words
        .iter()
        .flatten()
        .map(|pw| Item {
            rect: pw.rect,
            value: pw.number.and_then(|n| n.value),
        })
        .collect();
    // Lists a vertical gap alone cut apart: where each stands in `page.lists`.
    let mut gap_cut: Vec<usize> = Vec::new();
    for group in arrange(&items) {
        let mut take = |i: usize| -> Option<(PageWord, Vec<Option<f32>>)> {
            let word = words.get_mut(i)?.take()?;
            Some((word, gaps.get(i).cloned().unwrap_or_default()))
        };
        let across = group.across.clone();
        // Words in the order of the group; `across` indices become positions.
        let positions: Vec<usize> = group.order.clone();
        let mut ordered: Vec<(PageWord, Vec<Option<f32>>)> = Vec::new();
        for &i in &positions {
            if let Some(x) = take(i) {
                ordered.push(x);
            }
        }
        let across_pos: Option<Vec<usize>> = across.map(|a| {
            a.iter()
                .filter_map(|i| positions.iter().position(|p| p == i))
                .collect()
        });
        let n = ordered.len();
        let rows: Vec<&[Option<f32>]> = ordered.iter().map(|(_, gp)| gp.as_slice()).collect();
        let fit = mean_fit(&rows, lists.len());
        let mut questions = Vec::new();
        if across_pos.is_some() {
            questions.push(Question::Order);
        }
        if group.numbers_disagree {
            questions.push(Question::Numbers);
        }
        let (mut form, mut list) = match choose_form(n, &fit, &table) {
            Ok((f, l)) => (Some(f), Some(l)),
            Err(q) => {
                questions.push(q);
                (None, None)
            }
        };
        // Columns side by side, or parts of them above and below a blank band
        // across them all, where a run that is not all of them fits a form; a
        // layout that could not be told: the page may hold several lists.
        if group.unclear
            || some_run_fits(&group.sides, &table, 1, false)
            || some_run_fits(&group.bands, &table, 1, false)
        {
            questions.push(Question::Split);
            form = None;
            list = None;
        }
        // No word reads clear in any list: boxes of noise pass the floor and
        // are a form by count, so the form is asked.
        if !ordered
            .iter()
            .any(|(pw, _)| (0..lists.len()).any(|k| pw.clear(k).is_some()))
        {
            if !questions.contains(&Question::Form) {
                questions.push(Question::Form);
            }
            form = None;
            list = None;
        }
        // A box that may hold two lines: a word may be missing.
        if ordered.iter().any(|(pw, _)| pw.tall) {
            questions.push(Question::Missing);
            form = None;
            list = None;
        }
        if group.cut_by_gap {
            gap_cut.push(page.lists.len());
        }
        page.lists.push(List {
            words: ordered.into_iter().map(|(w, _)| w).collect(),
            order: if group.rows_first {
                Order::Rows
            } else {
                Order::Columns
            },
            across: across_pos,
            form,
            list,
            fit,
            questions,
        });
    }
    // Lists cut apart by a gap alone: asked when any run of two or more of
    // them together fits a form (the whole included), or when one has a
    // question.
    let sizes: Vec<usize> = gap_cut
        .iter()
        .filter_map(|&k| page.lists.get(k))
        .map(|l| l.words.len())
        .collect();
    let a_question = gap_cut
        .iter()
        .filter_map(|&k| page.lists.get(k))
        .any(|l| !l.questions.is_empty());
    if some_run_fits(&sizes, &table, 2, true) || a_question {
        for &k in &gap_cut {
            if let Some(l) = page.lists.get_mut(k) {
                l.questions.push(Question::Split);
            }
        }
    }
    // A number beside no word is a word or a number not found.
    if page.unattached_numbers > 0 {
        for l in &mut page.lists {
            if !l.questions.contains(&Question::Numbers) {
                l.questions.push(Question::Numbers);
            }
        }
    }
    Ok(page)
}
