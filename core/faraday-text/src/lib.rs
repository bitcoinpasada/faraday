//! Heap text that wipes itself.
//!
//! [`Text`] holds UTF-8 in one buffer. Growing it copies into a buffer
//! twice the size and wipes the old one before it is freed, and clearing,
//! replacing and dropping it wipe the whole buffer, the bytes past the end
//! included, so no copy of what was typed is left in freed memory. Text
//! that may be a secret is built with it, never with the standard
//! library's growable text, which frees its old buffer as it is.
//!
//! [`text!`] builds one the way a format macro does, [`Text::fill`] fills a
//! template the way `faraday_strings::fill` does, and [`Num`] writes a
//! number on the stack.

#![no_std]

extern crate alloc;

mod num;
mod text;

pub use num::Num;
pub use text::Text;

/// Builds a [`Text`] the way a format macro builds text: `text!("{}-{}", a, b)`.
#[macro_export]
macro_rules! text {
    ($($arg:tt)*) => {{
        let mut out = $crate::Text::new();
        // Writing into a `Text` never fails.
        let _ = ::core::fmt::Write::write_fmt(&mut out, ::core::format_args!($($arg)*));
        out
    }};
}
