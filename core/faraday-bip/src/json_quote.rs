//! A string written as a JSON string ([`crate::json::quote`]).
//!
//! The quotation mark is written `'\u{22}'` here, so that
//! `tools/lint-strings.sh` does not read it as the start of a string.

use alloc::format;
use alloc::string::String;

/// A string written as a JSON string, quotes and escapes included.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\u{22}');
    for c in s.chars() {
        match c {
            '\u{22}' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('\u{22}');
    out
}
