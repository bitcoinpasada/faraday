//! What a painted frame did wrong that a person would see, found from
//! where its text and icons left their ink ([`crate::canvas::Ink`]).
//!
//! The snapshot tool's `--audit` walks every script and asks this after
//! each step, so a screen nobody opened by hand is still looked at. It
//! finds three things:
//!
//! - **Overlap**: two strings, or a string and an icon, whose ink boxes
//!   cross — a value drawn over its label — or that sit side by side
//!   with less than [`TOUCH_DP`] between them — a label run into its
//!   glyph.
//! - **Data in a text face**: a string that carries something read
//!   character by character — a fingerprint, a checksum, an address, an
//!   extended key, a path, a descriptor — drawn in Noto Sans rather than
//!   the mono face (`docs/DESIGN.md` §3).
//! - **Cut text**: a string whose ink runs past the side of its clip
//!   ([`crate::canvas::Canvas::cut_texts`]), which the caller adds.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::canvas::{Ink, InkKind};
use crate::fonts::Family;

/// Pixels two ink boxes must share on both axes before they count as
/// crossing: anti-aliased edges of neighbours touch without meaning it.
const OVERLAP_MIN_PX: i32 = 2;

/// Space in dp under which two neighbours on one line are touching. A
/// row puts a full gap (8 dp) between its glyph and its label and the
/// record puts one between its columns, so anything this close was not
/// meant to be.
const TOUCH_DP: f32 = 3.0;

/// What is wrong with a frame, one line each. `px_per_dp` is the
/// display's scale.
pub fn check(ink: &[Ink], px_per_dp: f32) -> Vec<String> {
    let touch = ((TOUCH_DP * px_per_dp + 0.5) as i32).max(1);
    let mut found = Vec::new();
    for (i, a) in ink.iter().enumerate() {
        if let InkKind::Text { text, family } = &a.kind
            && !matches!(family, Family::Mono | Family::Cjk | Family::Icon)
            && let Some(what) = data_in(text)
        {
            found.push(format!("{what} not in mono: {text:?}"));
        }
        for b in &ink[i + 1..] {
            if a.kind == b.kind {
                // One string drawn twice in two colours — a hold
                // button's label on both sides of its fill.
                continue;
            }
            let r = a.rect.intersect(&b.rect);
            if r.w >= OVERLAP_MIN_PX && r.h >= OVERLAP_MIN_PX {
                found.push(format!("overlap: {} and {}", name(&a.kind), name(&b.kind)));
                continue;
            }
            // Side by side: the vertical spans share a third of the
            // shorter one, and the space between is under TOUCH_DP.
            let shared = a.rect.bottom().min(b.rect.bottom()) - a.rect.y.max(b.rect.y);
            let shorter = a.rect.h.min(b.rect.h);
            let apart = (b.rect.x - a.rect.right()).max(a.rect.x - b.rect.right());
            if shared * 3 > shorter && (0..touch).contains(&apart) && !joined(a, b) {
                found.push(format!("touching: {} and {}", name(&a.kind), name(&b.kind)));
            }
        }
    }
    found
}

/// Whether `a` and `b` are two runs of one line drawn end to end — a
/// descriptor's function, origin, key and checksum in two colours, a
/// sentence with a fingerprint in the mono face — which meet at a space
/// or at the punctuation that joins them. Two labels side by side meet
/// letter to letter or digit to digit, and are not joined.
fn joined(a: &Ink, b: &Ink) -> bool {
    let (left, right) = if a.rect.x <= b.rect.x { (a, b) } else { (b, a) };
    match (&left.kind, &right.kind) {
        (InkKind::Text { text: l, .. }, InkKind::Text { text: r, .. }) => {
            let end = l.chars().next_back().is_some_and(|c| !c.is_alphanumeric());
            let start = r.chars().next().is_some_and(|c| !c.is_alphanumeric());
            end || start
        }
        _ => false,
    }
}

fn name(kind: &InkKind) -> String {
    match kind {
        InkKind::Text { text, .. } => format!("{text:?}"),
        InkKind::Icon(icon) => format!("icon {icon:?}"),
    }
}

/// What kind of character-by-character data `text` carries, if any.
fn data_in(text: &str) -> Option<&'static str> {
    let tokens = text
        .split(|c: char| c.is_whitespace() || c == '\u{00b7}' || c == ',')
        .filter(|t| !t.is_empty());
    for t in tokens {
        let t = t.trim_matches(|c: char| matches!(c, '"' | '\u{201c}' | '\u{201d}' | '.' | ':'));
        if is_fingerprint(t) {
            return Some("fingerprint");
        }
        if t.len() == 9 && t.starts_with('#') && t[1..].chars().all(|c| c.is_ascii_alphanumeric()) {
            return Some("checksum");
        }
        if ["bc1", "tb1", "bcrt1"].iter().any(|p| t.starts_with(p)) && t.len() >= 14 {
            return Some("address");
        }
        if t.len() >= 20
            && [
                "xpub", "tpub", "ypub", "zpub", "upub", "vpub", "Ypub", "Zpub", "xprv", "tprv",
            ]
            .iter()
            .any(|p| t.starts_with(p))
        {
            return Some("extended key");
        }
        if is_path(t) {
            return Some("path");
        }
        if [
            "wpkh(",
            "wsh(",
            "sh(",
            "tr(",
            "pkh(",
            "multi(",
            "sortedmulti(",
            "musig(",
        ]
        .iter()
        .any(|f| t.starts_with(f))
            || matches!(t, "wpkh" | "wsh" | "pkh" | "tr" | "sh(wpkh)")
        {
            return Some("descriptor");
        }
        if t.len() >= 16 && t.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some("hex");
        }
    }
    None
}

/// Eight lower-case hex characters with a digit among them, alone or as
/// an origin's head (`[73c5da0a/84h`).
fn is_fingerprint(t: &str) -> bool {
    let t = t.trim_start_matches('[');
    let head = t.split('/').next().unwrap_or(t);
    head.len() == 8
        && head
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        && head.chars().any(|c| c.is_ascii_digit())
}

/// A derivation path: `m/84h/0h/0h`, or a run of two or more steps.
fn is_path(t: &str) -> bool {
    let body = t.strip_prefix("m/").unwrap_or(t);
    let steps: Vec<&str> = body.split('/').collect();
    let step = |s: &str| {
        let n = s.trim_end_matches(['h', '\'', 'H']);
        !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '*')
    };
    (t.starts_with("m/") && steps.iter().all(|s| step(s)))
        || (steps.len() >= 3 && steps.iter().all(|s| step(s)))
}
