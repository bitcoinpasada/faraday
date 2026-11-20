//! A text as a person sees it with its hidden characters made visible.

use std::borrow::Cow;

/// What stands for a control, zero-width or formatting character.
pub(crate) const HIDDEN_MARK: char = '\u{25CC}';
/// What stands for a line break other than a newline.
pub(crate) const BREAK_MARK: char = '\u{00B6}';

/// Whether `c` breaks a line other than as a newline does.
fn breaks(c: char) -> bool {
    matches!(
        c,
        '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

/// The characters that show nothing and are not controls: Unicode 16.0's
/// Default_Ignorable_Code_Point (DerivedCoreProperties.txt) together with
/// general category Cf (UnicodeData.txt), as sorted ranges.
const INVISIBLE: &[(u32, u32)] = &[
    (0xAD, 0xAD),
    (0x34F, 0x34F),
    (0x600, 0x605),
    (0x61C, 0x61C),
    (0x6DD, 0x6DD),
    (0x70F, 0x70F),
    (0x890, 0x891),
    (0x8E2, 0x8E2),
    (0x115F, 0x1160),
    (0x17B4, 0x17B5),
    (0x180B, 0x180F),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x206F),
    (0x3164, 0x3164),
    (0xFE00, 0xFE0F),
    (0xFEFF, 0xFEFF),
    (0xFFA0, 0xFFA0),
    (0xFFF0, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0000, 0xE0FFF),
];

/// Whether `c` shows nothing: a control character other than a line
/// break, or one Unicode leaves out when drawing (zero-width, joiner,
/// direction, invisible-operator, variation, tag and formatting
/// characters, a byte order mark, a soft hyphen).
fn hidden(c: char) -> bool {
    if c.is_control() {
        return c != '\n' && !breaks(c);
    }
    let u = c as u32;
    INVISIBLE
        .binary_search_by(|&(lo, hi)| {
            if hi < u {
                std::cmp::Ordering::Less
            } else if lo > u {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// `text` with every control, zero-width and line-break character other
/// than a newline drawn as a visible mark, one mark for one character
/// (a caret keeps its place in it); a newline stays the line break it
/// is. Borrowed when there is nothing to change.
pub(crate) fn visible(text: &str) -> Cow<'_, str> {
    if !text.chars().any(|c| hidden(c) || breaks(c)) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .map(|c| {
                if breaks(c) {
                    BREAK_MARK
                } else if hidden(c) {
                    HIDDEN_MARK
                } else {
                    c
                }
            })
            .collect(),
    )
}

/// The first line of `text` that has text, with its hidden characters
/// drawn as marks, without copying the rest of `text`.
pub(crate) fn first_line(text: &str) -> Cow<'_, str> {
    text.lines()
        .find(|l| !l.is_empty())
        .map_or(Cow::Borrowed(""), visible)
}

/// A message as it is drawn, kept while it is the same: its visible form
/// and the lines it breaks into at each width. Worked out the first time
/// a frame asks, not at each frame (`docs/SPEED.md` §3.3).
#[derive(Default)]
pub(crate) struct Shown(std::cell::RefCell<Option<Kept>>);

struct Kept {
    source: faraday_text::Text,
    visible: std::rc::Rc<str>,
    wraps: Vec<(faraday_ui::roles::WrapBasis, Lines)>,
}

/// The ranges of the lines a text breaks into.
type Lines = std::rc::Rc<[(usize, usize)]>;

impl Shown {
    fn kept<R>(&self, text: &str, f: impl FnOnce(&mut Kept) -> R) -> R {
        let mut k = self.0.borrow_mut();
        if k.as_ref().is_none_or(|k| k.source != *text) {
            *k = Some(Kept {
                source: faraday_text::Text::of(text),
                visible: visible(text).as_ref().into(),
                wraps: Vec::new(),
            });
        }
        f(k.as_mut().expect("kept above"))
    }

    /// `text` with its hidden characters drawn as marks.
    pub(crate) fn text(&self, text: &str) -> std::rc::Rc<str> {
        self.kept(text, |k| k.visible.clone())
    }

    /// The lines [`Shown::text`] of `text` breaks into at `basis`, from
    /// `make` when none are kept for it.
    pub(crate) fn lines(
        &self,
        text: &str,
        basis: faraday_ui::roles::WrapBasis,
        make: impl FnOnce(&str) -> Lines,
    ) -> Lines {
        self.kept(text, |k| {
            if let Some((_, l)) = k.wraps.iter().find(|(b, _)| *b == basis) {
                return l.clone();
            }
            let l = make(&k.visible);
            if k.wraps.len() >= 4 {
                k.wraps.remove(0);
            }
            k.wraps.push((basis, l.clone()));
            l
        })
    }
}

/// A row of Your choices for a flow whose answers are plain text: `value`
/// `None` is "Not chosen yet" (`docs/FLOW-CONTAINER.md` §1).
pub(crate) fn choice_row(
    label: &'static str,
    value: Option<&str>,
    mono: bool,
    (opens, current): (u8, bool),
) -> crate::flow::Row {
    crate::flow::Row {
        label: faraday_text::Text::of(label),
        value: value.map(faraday_text::Text::of),
        mono,
        opens,
        current,
    }
}
