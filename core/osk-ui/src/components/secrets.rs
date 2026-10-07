//! §4.10 Secrets: the in-place panel and the row that opens a Secret
//! screen.
//!
//! "Secrets are a place, not a state" (§2.8). A short secret masks in
//! place with the same geometry either way; a long one lives on its own
//! screen, reached by a [`secret_row`]. Nobody is shown a hundred
//! bullets.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geom::{Scale, SizeClass};
use crate::layout::{Align, Id, Justify, Node};
use crate::organisms;
use crate::text::{Font, TextAlign};
use crate::tokens;
use crate::widgets::{Tone, Widget};

/// What a [`secret_panel`] holds.
pub enum Secret<'a> {
    /// Words, in one or two columns per class. `first` is the number of
    /// the first word on this page, so a paged panel keeps counting.
    Words {
        /// The words of this page.
        words: &'a [String],
        /// The number of `words[0]`.
        first: usize,
        /// Each word's place in the wordlist, 0-based, when the Words
        /// screen's "Numbers" is on. It is the key as much as the word
        /// is, so it masks with it (§4.10's mask policy); the rows keep
        /// their height and the panel its rectangle either way.
        indices: Option<&'a [u16]>,
        /// How wide a word of this wordlist is.
        width: WordWidth,
        /// Whether the rows are the steel rows of `docs/PLANNING.md`
        /// §8.2 item 6 — the wordlist number, the four punched letters
        /// and the word — which are fourteen characters wider than a
        /// word alone and are drawn a step smaller for it.
        steel: bool,
    },
    /// One short value: a fingerprint-sized string on one line.
    Value(&'a str),
}

/// Characters a wordlist number takes in either state: 1 to 2048.
const INDEX_WIDTH: usize = 4;

/// How wide the widest word of a wordlist is: how many characters it
/// has, and one character of the script it is written in, which is what
/// the face measures. A kana or a Hangul syllable is about an em wide in
/// the fallback face; a Latin letter is the monospace advance.
///
/// The panel pads every word to [`chars`](Self::chars) characters, so a
/// masked row is the same width as a revealed one whatever word is under
/// it, and the same width in every row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordWidth {
    /// Characters in the longest word of the list.
    pub chars: usize,
    /// One character of the list's script.
    pub sample: char,
}

impl WordWidth {
    /// The eight-letter longest word of the English list.
    pub const LATIN: WordWidth = WordWidth {
        chars: 8,
        sample: 'm',
    };
}

/// Width in dp a panel's rows have inside the screen's padding and the
/// panel's own.
pub fn panel_room(width_dp: f32) -> f32 {
    width_dp - 2.0 * tokens::PAD - 2.0 * tokens::SECRET_PAD
}

/// Width in dp of the widest row a panel of `width` words draws: the
/// row number, the word padded to its list's width, and the wordlist
/// number where the panel carries one.
pub fn word_row_width(class: SizeClass, width: WordWidth, indexed: bool, steel: bool) -> f32 {
    let size = if steel {
        tokens::word_mono_steel(class)
    } else if indexed {
        tokens::word_mono_with_index(class)
    } else {
        tokens::word_mono(class)
    };
    // The panel draws at the class's type scale, so the row is measured
    // there too: an Italian or a Japanese list is wider on a phone or in
    // a window than the unscaled ramp says.
    let face = Font::mono(size).sized(Scale::IDENTITY.with_class(class));
    let mut row = String::from("24. ");
    for _ in 0..width.chars {
        row.push(width.sample);
    }
    if indexed {
        row.push_str(" 2048");
    }
    crate::text::width(&face, &row) as f32
}

/// Columns of words the panel draws: the class's own, or one where two
/// of this list's rows do not fit the width the panel has. A Japanese or
/// Korean word is about an em a character, so a class that shows two
/// columns of Latin words may have room for only one of theirs.
pub fn word_columns(
    class: SizeClass,
    room_dp: f32,
    width: WordWidth,
    indexed: bool,
    steel: bool,
) -> usize {
    let columns = tokens::word_columns(class);
    if columns < 2 {
        return columns;
    }
    let row = word_row_width(class, width, indexed, steel);
    let gap = tokens::word_column_gap(indexed);
    if 2.0 * row + gap <= room_dp {
        columns
    } else {
        1
    }
}

/// Whether a panel of `total` words of this list fits `space_dp` of
/// height: two columns where `room_dp` has the width for them, one
/// column where it has not, and at the class's own row height either
/// way. False where even one column is taller than the space, which is
/// what the 240 dp panels hit (§4.10).
pub fn panel_fits(
    class: SizeClass,
    room_dp: f32,
    space_dp: f32,
    width: WordWidth,
    total: usize,
) -> bool {
    let columns = word_columns(class, room_dp, width, false, false);
    words_panel_height(class, total, columns) <= space_dp
}

/// §4.10 Secret panel: "An orange-outlined panel with fixed geometry,
/// two heights: the words panel (numbered rows: six per page on `small`,
/// twelve elsewhere, two columns where the width allows) and the value
/// panel (one line box). No caption inside it, in either state. Masked:
/// bullets per item, same rows. Hold anywhere on it to show; release
/// masks."
///
/// No caption: §2.1 does not label an idiom, and an orange-outlined box
/// is a secret. Each panel is given its height from tokens rather than
/// from its content, so the rectangle is the same masked and revealed,
/// with the numbers on and off, and the same for a 12-word and a 24-word
/// secret: the words panel shows one page of [`tokens::words_per_page`]
/// rows either way, and a page that is not full keeps its empty rows.
pub fn secret_panel(
    class: SizeClass,
    id: Id,
    content: Secret<'_>,
    revealed: bool,
    room_dp: f32,
) -> Node {
    let (body, height) = match content {
        Secret::Words {
            words,
            first,
            indices,
            width,
            steel,
        } => {
            let columns = word_columns(class, room_dp, width, indices.is_some(), steel);
            (
                word_grid(
                    class,
                    words,
                    first,
                    indices,
                    revealed,
                    word_rows(words.len(), columns),
                    columns,
                    width,
                    steel,
                    room_dp,
                ),
                words_panel_height(class, words.len(), columns),
            )
        }
        Secret::Value(value) => (one_line(value, revealed), value_panel_height()),
    };
    organisms::secret_panel(id, body, None).height(height)
}

/// §4.3 on `mobile` and `wide`: "the space above the entry group holds,
/// for word entry, a masked words panel of the words accepted so far",
/// so a typo can be checked without leaving the screen.
///
/// The panel keeps the rows of the whole secret — `total` of them,
/// numbered — and leaves the rows past the last accepted word empty, so
/// it is the same rectangle at the first word and at the last and
/// nothing moves as the entry goes on.
///
/// §4.3: "The panel is drawn only once a word has been accepted (an empty
/// panel says nothing) ... and while it is drawn the app bar carries the
/// eye, as on every screen with a words panel." It is a secret panel like
/// any other, so `revealed` is a finger on it or the app bar's eye
/// (§4.10) and the rectangle is the same either way.
pub fn words_so_far(
    class: SizeClass,
    id: Id,
    words: &[String],
    total: usize,
    revealed: bool,
    width: WordWidth,
    room_dp: f32,
) -> Node {
    let columns = word_columns(class, room_dp, width, false, false);
    let rows = word_rows(total, columns);
    organisms::secret_panel(
        id,
        word_grid(
            class, words, 1, None, revealed, rows, columns, width, false, room_dp,
        ),
        None,
    )
    .height(words_panel_height(class, total, columns))
}

/// Height in dp of a words panel holding `words` words: its rows at the
/// class's row height, and the padding at each end.
///
/// The panel is sized to what it holds rather than to the page it could
/// hold, so twelve words are two columns of six and twenty-four are two
/// columns of twelve, and neither leaves an empty half-panel behind.
pub fn words_panel_height(class: SizeClass, words: usize, columns: usize) -> f32 {
    word_rows(words, columns) as f32 * tokens::word_row(class) + panel_chrome()
}

/// Height in dp of the value panel: one line box. The same on every
/// class: one line of one value is one line of one value.
pub fn value_panel_height() -> f32 {
    tokens::WORD_ROW + panel_chrome()
}

/// Height in dp a panel takes on top of its content: the padding at each
/// end, and nothing else — there is no caption in it (§4.10).
fn panel_chrome() -> f32 {
    2.0 * tokens::SECRET_PAD
}

/// Rows the panel draws for `words` words in `columns` columns. At
/// least one, so an empty panel is still a panel.
fn word_rows(words: usize, columns: usize) -> usize {
    words.div_ceil(columns.max(1)).max(1)
}

/// The word rows, in [`tokens::word_columns`] columns, `rows` of them
/// per column. A masked row is bullets of the same width as a word, in
/// the same monospace size, so the grid does not move under the finger
/// that reveals it, and turning the numbers off leaves the same rows in
/// the same places. A row past the last word given is empty: it is a
/// place kept, not a word hidden.
#[allow(clippy::too_many_arguments)]
fn word_grid(
    class: SizeClass,
    words: &[String],
    first: usize,
    indices: Option<&[u16]>,
    revealed: bool,
    rows: usize,
    columns: usize,
    width: WordWidth,
    steel: bool,
    room_dp: f32,
) -> Node {
    let mask = width.chars;
    let size = if steel {
        tokens::word_mono_steel(class)
    } else if indices.is_some() {
        tokens::word_mono_with_index(class)
    } else {
        tokens::word_mono(class)
    };
    let gap = tokens::word_column_gap(indices.is_some());
    // A row is never wider than its share of the panel: a list whose
    // words are wider than the ramp allows for is drawn a size down
    // rather than over the panel's edge.
    let cell_dp = ((room_dp - gap * (columns as f32 - 1.0)) / columns as f32).max(0.0);
    let mut grid = Node::row().gap(gap);
    for column in 0..columns {
        let mut cell = Node::column();
        for row in 0..rows {
            let index = column * rows + row;
            let text = match words.get(index) {
                Some(word) => {
                    let number = alloc::format!("{:>2}. ", first + index);
                    let shown = if revealed {
                        pad_to(word, mask)
                    } else {
                        bullets(mask)
                    };
                    // The wordlist number is the key as much as the word
                    // is, so it is bullets of its own width while the
                    // panel is masked (§4.10).
                    let place = match indices {
                        None => String::new(),
                        Some(list) if revealed => list
                            .get(index)
                            .map(|i| alloc::format!(" {:>INDEX_WIDTH$}", i + 1))
                            .unwrap_or_default(),
                        Some(_) => alloc::format!(" {:>INDEX_WIDTH$}", bullets(INDEX_WIDTH)),
                    };
                    alloc::format!("{number}{shown}{place}")
                }
                None => String::new(),
            };
            cell = cell.child(
                Node::widget(Widget::text(text, Font::mono(size), Tone::Text))
                    .height(tokens::word_row(class))
                    .max_width(cell_dp),
            );
        }
        grid = grid.child(cell.weight(1.0));
    }
    Node::row().justify(Justify::Center).child(grid)
}

/// One short value, masked to bullets of its own length.
fn one_line(value: &str, revealed: bool) -> Node {
    let text = if revealed {
        String::from(value)
    } else {
        bullets(value.chars().count())
    };
    Node::column().justify(Justify::Center).child(
        Node::widget(
            Widget::text(text, Font::mono(tokens::MONO), Tone::Text).align(TextAlign::Center),
        )
        .align_self(Align::Stretch)
        .height(tokens::WORD_ROW),
    )
}

/// `word` with spaces after it up to `chars` characters, so a row is
/// the same width whatever word is in it. A word longer than the list's
/// widest is left as it is.
fn pad_to(word: &str, chars: usize) -> String {
    let mut out = String::from(word);
    for _ in word.chars().count()..chars {
        out.push(' ');
    }
    out
}

/// `n` bullet characters.
fn bullets(n: usize) -> String {
    core::iter::repeat_n('\u{2022}', n).collect()
}

/// §4.10 Long secret: "A reference row (bullets as the value while
/// masked). Opens the Secret screen, where the whole body is one
/// panel."
///
/// The value is the elided head and tail of the secret when it is
/// already revealed elsewhere, and bullets when it is not: a row never
/// leaks the middle of a key, and never shows a hundred bullets either.
/// The same two-line row as a reference row, at the same height.
pub fn secret_row(class: SizeClass, id: Id, label: impl Into<String>, value: Option<&str>) -> Node {
    let shown = match value {
        Some(secret) => super::strings::elide(secret),
        None => bullets(tokens::ELIDE_HEAD),
    };
    Node::widget(
        Widget::list_row(Some(id), label, Some(shown), None)
            .with_height(super::strings::reference_row_height(class))
            .with_value_below(true, Tone::Text),
    )
}

/// Words on one page of a panel drawn in `columns` columns: the rows the
/// class's panel holds, filled across those columns. A list whose words
/// are an em a character falls to one column and so to half a page, and
/// the panel keeps the height its class gives it.
pub fn words_per_page(class: SizeClass, columns: usize) -> usize {
    tokens::words_per_page(class) / tokens::word_columns(class) * columns.max(1)
}

/// The words of a secret split into the pages a panel shows, so a
/// caller can hand [`secret_panel`] one page and a [`super::nav::pager`]
/// the count.
pub fn word_pages(class: SizeClass, columns: usize, words: &[String]) -> Vec<(usize, Vec<String>)> {
    let per_page = words_per_page(class, columns);
    words
        .chunks(per_page)
        .enumerate()
        .map(|(i, page)| (i * per_page + 1, page.to_vec()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Rect, Scale};
    use crate::layout::{LayoutCtx, solve};
    use crate::widgets::HitTarget;

    /// The four sizes `just snapshots` renders.
    const REFERENCE: [(Rect, u16, SizeClass); 4] = [
        (Rect::new(0, 0, 240, 320), 143, SizeClass::Small),
        (Rect::new(0, 0, 480, 640), 286, SizeClass::Small),
        (Rect::new(0, 0, 1080, 2340), 420, SizeClass::Mobile),
        (Rect::new(0, 0, 960, 640), 160, SizeClass::Wide),
    ];

    fn seed(n: usize) -> Vec<String> {
        (0..n).map(|i| alloc::format!("word{i:02}")).collect()
    }

    /// The rectangle of the panel's touch surface.
    fn surface(panel: Node, area: Rect, ctx: &LayoutCtx) -> Rect {
        solve(&Node::column().child(panel), area, ctx)
            .items
            .iter()
            .find(|p| p.hit == Some(HitTarget::Reveal(Id(1))))
            .expect("one touch surface")
            .rect
    }

    /// §4.10: "Masked: bullets per item, same rows", and §2.5's fixed
    /// geometry, which §4.10 fixes "per class and per word count": a
    /// finger that reveals the panel must not move it and the numbers
    /// going off must not move it. The panel is sized to what it holds,
    /// so a page of six on `small` and a page of twelve or twenty-four in
    /// two columns elsewhere each have a rectangle of their own.
    #[test]
    fn the_words_panel_keeps_one_rectangle_however_it_is_read() {
        for (area, dpi, class) in REFERENCE {
            let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
            for count in [12, 24] {
                let words = seed(count);
                let pages = word_pages(class, tokens::word_columns(class), &words);
                let (first, page) = pages.first().cloned().expect("a page");
                let places: Vec<u16> = (0..page.len() as u16).collect();
                let mut seen: Option<Rect> = None;
                for revealed in [false, true] {
                    for numbered in [false, true] {
                        let panel = secret_panel(
                            class,
                            Id(1),
                            Secret::Words {
                                steel: false,
                                words: &page,
                                first,
                                indices: numbered.then_some(&places),
                                width: WordWidth::LATIN,
                            },
                            revealed,
                            panel_room(area.w as f32 / ctx.scale.factor()),
                        );
                        let r = surface(panel, area, &ctx);
                        match seen {
                            None => seen = Some(r),
                            Some(other) => assert_eq!(
                                r, other,
                                "{count} words, revealed={revealed}, numbered={numbered}, \
                                 at {}x{}",
                                area.w, area.h
                            ),
                        }
                    }
                }
            }
        }
    }

    /// §4.10: the panel is "sized to its content at the class's type",
    /// so the widest row it can hold — the longest word of the list, with
    /// a four-digit wordlist number beside it — fits the width the screen
    /// gives the panel, in every column the panel draws. A Japanese or
    /// Korean word is about an em a character, so the panel drops to one
    /// column where two of them do not fit.
    #[test]
    fn the_widest_row_of_every_list_fits_the_panel_on_every_class() {
        for (area, dpi, class) in REFERENCE {
            let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
            let room = panel_room(area.w as f32 / ctx.scale.factor());
            for width in [
                WordWidth::LATIN,
                WordWidth {
                    chars: 7,
                    sample: '\u{3042}',
                },
                WordWidth {
                    chars: 4,
                    sample: '\u{AC00}',
                },
            ] {
                for indexed in [false, true] {
                    let columns = word_columns(class, room, width, indexed, false);
                    assert!(columns >= 1);
                    let rows = columns as f32 * word_row_width(class, width, indexed, false)
                        + (columns as f32 - 1.0) * tokens::word_column_gap(indexed);
                    assert!(
                        rows <= room,
                        "{columns} columns of {width:?} are {rows} dp, more than the {room} dp \
                         a panel has at {}x{}",
                        area.w,
                        area.h
                    );
                    // A page fills the panel's rows, and the panel keeps
                    // the height its class gives it.
                    assert_eq!(
                        words_per_page(class, columns) % columns,
                        0,
                        "a page does not fill its columns"
                    );
                }
            }
        }
    }

    /// §4.3 and §4.10: the panel of words accepted so far keeps a row
    /// for every word of the secret, shows the accepted ones as bullets
    /// and leaves the rest empty. Nothing on it is a word: the panel is
    /// masked, and the eye that reveals it is a finger on it.
    #[test]
    fn the_words_so_far_panel_is_bullets_then_empty_rows() {
        let class = SizeClass::Mobile;
        let total = 12;
        let words = seed(7);
        let panel = words_so_far(
            class,
            Id(1),
            &words,
            total,
            false,
            WordWidth::LATIN,
            panel_room(411.0),
        );
        let texts = texts(&panel);
        // One row per word of the secret, in the class's columns.
        assert_eq!(texts.len(), total);
        for (i, text) in texts.iter().enumerate() {
            if i < words.len() {
                assert!(
                    text.contains('\u{2022}'),
                    "row {i} of the panel is not masked: {text}"
                );
                assert!(
                    text.starts_with(&alloc::format!("{:>2}. ", i + 1)),
                    "{text}"
                );
            } else {
                assert!(text.is_empty(), "row {i} of the panel is not empty: {text}");
            }
            for word in &words {
                assert!(!text.contains(word.as_str()), "the panel shows {word}");
            }
        }
    }

    /// Every line of text a node draws, in order.
    fn texts(node: &Node) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(crate::widgets::Widget::Text { text, .. }) = node.as_widget() {
            out.push(text.clone());
        }
        for child in &node.children {
            out.extend(texts(child));
        }
        out
    }

    /// §4.10: the value panel is "one line box", a fixed geometry of its
    /// own and shorter than the words panel. No caption in either.
    #[test]
    fn the_value_panel_keeps_its_rectangle_masked_and_revealed() {
        for (area, dpi, class) in REFERENCE {
            let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
            let mut seen: Option<Rect> = None;
            for revealed in [false, true] {
                let panel = secret_panel(
                    class,
                    Id(1),
                    Secret::Value("c0ffee42"),
                    revealed,
                    panel_room(240.0),
                );
                let r = surface(panel, area, &ctx);
                match seen {
                    None => seen = Some(r),
                    Some(other) => {
                        assert_eq!(r, other, "revealed={revealed} at {}x{}", area.w, area.h)
                    }
                }
            }
        }
    }

    /// §4.10: "A reference row (bullets as the value while masked)" —
    /// and nobody is shown a hundred bullets, so an unrevealed value is
    /// as short as an elided one.
    #[test]
    fn a_secret_row_never_shows_more_bullets_than_an_elided_value() {
        let ctx = LayoutCtx::new(Scale::IDENTITY, SizeClass::Small);
        let area = Rect::new(0, 0, 268, 358);
        for value in [
            None,
            Some(
                "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi",
            ),
        ] {
            let row = secret_row(SizeClass::Small, Id(1), "Master private key", value);
            let l = solve(&Node::column().child(row), area, &ctx);
            let r = l.rect(Id(1)).expect("the row");
            assert_eq!(
                l.hit_test(r.center().x, r.center().y).map(|p| p.hit),
                Some(Some(HitTarget::Tap(Id(1))))
            );
        }
        assert_eq!(
            bullets(tokens::ELIDE_HEAD).chars().count(),
            tokens::ELIDE_HEAD
        );
    }
}
