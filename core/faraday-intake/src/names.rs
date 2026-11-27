//! A wallet's name: public text, which holds no secret, so it lives
//! apart from the readers that do.

/// The most bytes a wallet's name keeps, the length a vault's name takes
/// in its document (`LABEL_MAX` of the vault area): cut at a character
/// boundary.
pub const WALLET_NAME_MAX: usize = 64;

/// `text` cut to at most `max` bytes at a character boundary, the spaces
/// the cut leaves at its end taken off.
pub fn cut_to(text: &str, max: usize) -> &str {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].trim_end()
}

/// A name carried in a file or typed, as one line of text, with no bound
/// on its length: every control character (Unicode `Cc`, line feeds and
/// carriage returns among them), the line and paragraph separators
/// U+2028 and U+2029 and the invisible format characters (zero-width,
/// bidirectional-control, word-joiner, byte-order-mark, soft-hyphen, tag
/// characters) taken out, and the spaces at either end. Borrows `text`
/// when it is already that.
pub fn one_line(text: &str) -> std::borrow::Cow<'_, str> {
    let dropped = |c: char| {
        c.is_control()
            || matches!(c,
                '\u{ad}' | '\u{61c}' | '\u{180e}' | '\u{200b}'..='\u{200f}'
                | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{206f}'
                | '\u{feff}' | '\u{fff9}'..='\u{fffb}' | '\u{e0000}'..='\u{e007f}')
    };
    let trimmed = text.trim_matches(|c: char| c.is_whitespace() || dropped(c));
    if trimmed.chars().any(dropped) {
        let kept: String = trimmed.chars().filter(|c| !dropped(*c)).collect();
        std::borrow::Cow::Owned(kept.trim().to_string())
    } else {
        std::borrow::Cow::Borrowed(trimmed)
    }
}

/// A wallet's name: [`one_line`] of `text`, at most [`WALLET_NAME_MAX`]
/// bytes kept (cut at a character boundary). Borrows `text` when it is
/// already that.
pub fn wallet_name(text: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    match one_line(text) {
        Cow::Borrowed(s) if s.len() <= WALLET_NAME_MAX => Cow::Borrowed(s),
        Cow::Borrowed(s) => Cow::Owned(cut_to(s, WALLET_NAME_MAX).to_string()),
        Cow::Owned(s) if s.len() <= WALLET_NAME_MAX => Cow::Owned(s),
        Cow::Owned(s) => Cow::Owned(cut_to(&s, WALLET_NAME_MAX).to_string()),
    }
}
