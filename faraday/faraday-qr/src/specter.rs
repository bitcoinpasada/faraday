//! Numbered text parts, `p<n>of<m> <text>`: what Specter Desktop writes and
//! SeedSigner reads for a PSBT or a descriptor too long for one code.

use std::collections::BTreeMap;

/// The most parts read.
pub const MAX_PARTS: usize = 1000;

/// The part number, the count and the text of `pNofM text`.
pub fn parse(text: &str) -> Option<(usize, usize, &str)> {
    let rest = text.strip_prefix('p')?;
    let (head, body) = rest.split_once(' ')?;
    let (n, m) = head.split_once("of")?;
    let (n, m): (usize, usize) = (n.parse().ok()?, m.parse().ok()?);
    (n >= 1 && n <= m && m <= MAX_PARTS && !body.is_empty()).then_some((n, m, body))
}

/// Parts being put together.
#[derive(Debug, Default)]
pub struct Collector {
    total: usize,
    parts: BTreeMap<usize, String>,
}

/// Where the parts stand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Got {
    /// Parts read, of how many, and the ones missing.
    Part(usize, usize, Vec<usize>),
    /// The whole text.
    Done(String),
    /// A part of another set, or the same part with other text.
    Refused(&'static str),
}

impl Collector {
    /// Takes one part.
    pub fn add(&mut self, n: usize, m: usize, body: &str) -> Got {
        if self.total != 0 && self.total != m {
            return Got::Refused("A numbered part of another set. Start again to read that one");
        }
        if let Some(seen) = self.parts.get(&n)
            && seen != body
        {
            return Got::Refused("A numbered part arrived twice with different text");
        }
        if self.parts.values().map(String::len).sum::<usize>() + body.len() > crate::bbqr::MAX_DATA
        {
            return Got::Refused("The numbered parts come to more than 360 KiB");
        }
        self.total = m;
        self.parts.insert(n, body.to_string());
        if self.parts.len() < m {
            let missing = (1..=m).filter(|i| !self.parts.contains_key(i)).collect();
            return Got::Part(self.parts.len(), m, missing);
        }
        Got::Done(self.parts.values().cloned().collect())
    }
}

/// `text` as numbered parts of at most `chunk` characters each.
pub fn encode(text: &str, chunk: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let chunk = chunk.max(1);
    let m = chars.len().div_ceil(chunk).max(1);
    (0..m)
        .map(|i| {
            let part: String = chars[i * chunk..chars.len().min((i + 1) * chunk)]
                .iter()
                .collect();
            format!("p{}of{m} {part}", i + 1)
        })
        .collect()
}
