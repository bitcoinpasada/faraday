//! A wallet's name: public text, which holds no secret, so it lives
//! apart from the readers that do.

/// A wallet's name as one line of text, the form every name a wallet is
/// given takes, whether it came from a file, a vault record or the
/// keyboard: every control character (Unicode `Cc`, line feeds and
/// carriage returns among them) and the line and paragraph separators
/// U+2028 and U+2029 taken out, and the spaces at either end. Borrows
/// `text` when it is already that.
pub fn wallet_name(text: &str) -> std::borrow::Cow<'_, str> {
    let dropped = |c: char| c.is_control() || c == '\u{2028}' || c == '\u{2029}';
    let trimmed = text.trim_matches(|c: char| c.is_whitespace() || dropped(c));
    if trimmed.chars().any(dropped) {
        let kept: String = trimmed.chars().filter(|c| !dropped(*c)).collect();
        std::borrow::Cow::Owned(kept.trim().to_string())
    } else {
        std::borrow::Cow::Borrowed(trimmed)
    }
}
