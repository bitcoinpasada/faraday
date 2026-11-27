//! Every text a kit model holds, and whether it is a secret (04 §11).

use faraday_text::Text;

/// Every text a kit model holds. Its variants are private.
#[derive(Clone, Copy)]
pub struct Shown<'s>(Kind<'s>);

#[derive(Clone, Copy)]
enum Kind<'s> {
    Plain(&'s str),
    Secret(&'s str),
}

/// A text that is no secret: a `Text` the app holds or a fixed string.
#[derive(Clone, Copy)]
pub struct PlainRef<'s>(Plain<'s>);

#[derive(Clone, Copy)]
enum Plain<'s> {
    Text(&'s Text),
    Fixed(&'static str),
}

impl<'s> From<&'s Text> for PlainRef<'s> {
    fn from(text: &'s Text) -> Self {
        PlainRef(Plain::Text(text))
    }
}

impl From<&'static str> for PlainRef<'_> {
    fn from(text: &'static str) -> Self {
        PlainRef(Plain::Fixed(text))
    }
}

impl<'s> Shown<'s> {
    /// Text that is no secret: a `Text` the app holds, or a fixed string.
    pub fn plain(text: impl Into<PlainRef<'s>>) -> Self {
        Shown(Kind::Plain(match text.into().0 {
            Plain::Text(t) => t.as_str(),
            Plain::Fixed(s) => s,
        }))
    }

    /// A secret on show: a `SecretText`, a reveal's text or the passphrase part's view.
    pub fn secret(text: &'s str) -> Self {
        Shown(Kind::Secret(text))
    }

    /// The text, to draw.
    pub fn as_str(&self) -> &'s str {
        match self.0 {
            Kind::Plain(s) | Kind::Secret(s) => s,
        }
    }

    /// Whether it is a secret on show: drawing it sets the frame's `secret` flag (04 §11).
    pub fn is_secret(&self) -> bool {
        matches!(self.0, Kind::Secret(_))
    }
}

/// An address or a txid in groups of four.
pub fn grouped(s: &str) -> Text {
    let mut out = Text::with_room(s.len() + s.len() / 4);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && i % 4 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The first two groups of four, an ellipsis and the last four
/// characters.
pub fn short(s: &str) -> Text {
    let count = s.chars().count();
    if count <= 16 {
        return grouped(s);
    }
    let head_end = s.char_indices().nth(8).map_or(s.len(), |(i, _)| i);
    let tail_start = s.char_indices().nth(count - 4).map_or(s.len(), |(i, _)| i);
    let mut out = grouped(&s[..head_end]);
    out.push_str(" … ");
    out.push_str(&s[tail_start..]);
    out
}

/// An amount in BTC with eight decimals.
pub fn btc(sat: u64) -> Text {
    faraday_text::text!("{}.{:08}", sat / 100_000_000, sat % 100_000_000)
}

/// A whole number with thousands separators.
pub fn thousands(n: u64) -> Text {
    let s = Text::num(n);
    let mut out = Text::with_room(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
