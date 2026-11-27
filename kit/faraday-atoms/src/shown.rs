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
    fn from(_text: &'s Text) -> Self {
        todo!()
    }
}

impl From<&'static str> for PlainRef<'_> {
    fn from(_text: &'static str) -> Self {
        todo!()
    }
}

impl<'s> Shown<'s> {
    /// Text that is no secret: a `Text` the app holds, or a fixed string.
    pub fn plain(_text: impl Into<PlainRef<'s>>) -> Self {
        todo!()
    }

    /// A secret on show: a `SecretText`, a reveal's text or the passphrase part's view.
    pub fn secret(_text: &'s str) -> Self {
        todo!()
    }
}
