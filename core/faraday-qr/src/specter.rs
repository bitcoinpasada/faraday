//! Numbered text parts, `p<n>of<m> <text>`: what Specter Desktop writes and
//! SeedSigner reads for a PSBT or a descriptor too long for one code.

use faraday_text::{Text, text};
use std::collections::BTreeMap;

/// The most parts read.
pub const MAX_PARTS: usize = 1000;

/// The part number, the count and the text of `pNofM text`.
pub fn parse(text: &str) -> Option<(usize, usize, &str)> {
    let rest = text.strip_prefix('p')?;
    let (head, body) = rest.split_once(' ')?;
    let (n, m) = head.split_once("of")?;
    // Digits only: `parse` would also take a leading `+`, which no
    // writer puts there and SeedSigner's reader does not match.
    if [n, m]
        .iter()
        .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let (n, m): (usize, usize) = (n.parse().ok()?, m.parse().ok()?);
    (n >= 1 && n <= m && m <= MAX_PARTS && !body.is_empty()).then_some((n, m, body))
}

/// Parts being put together. The parts read are wiped when it is
/// dropped: a transfer may be a secret.
#[derive(Default)]
pub struct Collector {
    total: usize,
    parts: BTreeMap<usize, Text>,
}

impl std::fmt::Debug for Collector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Collector(..)")
    }
}

impl Drop for Collector {
    fn drop(&mut self) {
        // Each part is a `Text`, which wipes itself as it is dropped.
        self.parts.clear();
    }
}

/// Where the parts stand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Got {
    /// Parts read, of how many, and the ones missing.
    Part(usize, usize, Vec<usize>),
    /// The whole text.
    Done(Text),
    /// A part of another set, the same part with other text, or too
    /// much.
    Refused(Fault),
}

/// Why a numbered part was refused: a kind, which the app words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// A part of another set.
    OtherSet,
    /// The same part arrived twice with different text.
    Conflict,
    /// The parts come to more than [`crate::bbqr::MAX_DATA`].
    TooLarge,
}

impl Collector {
    /// Parts read, of how many; `None` before the first part.
    pub fn progress(&self) -> Option<(usize, usize)> {
        (self.total != 0).then_some((self.parts.len(), self.total))
    }

    /// Takes one part.
    pub fn add(&mut self, n: usize, m: usize, body: &str) -> Got {
        if self.total != 0 && self.total != m {
            return Got::Refused(Fault::OtherSet);
        }
        if let Some(seen) = self.parts.get(&n)
            && seen != body
        {
            return Got::Refused(Fault::Conflict);
        }
        if self.parts.values().map(|t| t.len()).sum::<usize>() + body.len() > crate::bbqr::MAX_DATA
        {
            return Got::Refused(Fault::TooLarge);
        }
        self.total = m;
        self.parts.insert(n, Text::of(body));
        if self.parts.len() < m {
            let missing = (1..=m).filter(|i| !self.parts.contains_key(i)).collect();
            return Got::Part(self.parts.len(), m, missing);
        }
        // Joined straight from the parts, which stay to catch a part
        // read again with other text: no copy of a part is made.
        Got::Done(self.parts.values().map(|t| t.as_str()).collect())
    }
}

/// `text` as numbered parts of at most `chunk` characters each.
pub fn encode(text: &str, chunk: usize) -> Vec<Text> {
    let chars: Vec<char> = text.chars().collect();
    let chunk = chunk.max(1);
    let m = chars.len().div_ceil(chunk).max(1);
    (0..m)
        .map(|i| {
            let part: Text = chars[i * chunk..chars.len().min((i + 1) * chunk)]
                .iter()
                .copied()
                .collect();
            text!("p{}of{m} {}", i + 1, &*part)
        })
        .collect()
}
