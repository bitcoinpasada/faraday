//! The lengths of OpenPGP packets and subpackets, read in one place
//! (RFC 9580 §4.2 packet lengths, §5.2.3.7 subpacket lengths).
//!
//! The readers differ on purpose in what they take, and each says which
//! by the form it names: [`NewFormat`] for a packet header's length, and
//! [`subpacket_length`] and [`Subpackets`] for the subpackets of a
//! signature. Every function returns `None` or an error for a length that
//! is cut short, never panics, and leaves it to the caller to see that the
//! body is within the data (the length is a `usize`, up to what four
//! octets hold).

use core::fmt;

/// Which lengths of a new-format packet header a reader takes
/// (RFC 9580 §4.2.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NewFormat {
    /// The one-octet (0 to 191) and two-octet (192 to 8383) lengths. The
    /// five-octet form and the partial lengths are refused.
    OneOrTwoOctets,
    /// Those and the five-octet form (first octet 255). The partial
    /// lengths (224 to 254) are refused.
    Whole,
    /// Those and the partial lengths, for the data packets of a message.
    WholeOrPartial,
}

/// A new-format length: the body's length, or the length of one piece of
/// a body in partial pieces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Length {
    /// The body's length.
    Fixed(usize),
    /// A partial body length: the piece's length, and more follow.
    Partial(usize),
}

/// The length of an old-format packet (RFC 9580 §4.2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OldLength {
    /// The body's length.
    Fixed(usize),
    /// Length type 3: the packet runs to the end of the data.
    Indeterminate,
}

fn octet(bytes: &[u8], at: usize, i: usize) -> Option<usize> {
    bytes.get(at.checked_add(i)?).copied().map(usize::from)
}

fn four_octets(bytes: &[u8], at: usize) -> Option<usize> {
    let s = bytes.get(at..at.checked_add(4)?)?;
    usize::try_from(u32::from_be_bytes([s[0], s[1], s[2], s[3]])).ok()
}

/// The length that starts at `bytes[at]`, just after a new-format header's
/// tag octet, and the number of octets it takes. `None` when it is cut
/// short or is a form `accept` does not take.
pub fn new_format(bytes: &[u8], at: usize, accept: NewFormat) -> Option<(Length, usize)> {
    let first = octet(bytes, at, 0)?;
    Some(match first {
        0..=191 => (Length::Fixed(first), 1),
        192..=223 => (
            Length::Fixed(((first - 192) << 8) + octet(bytes, at, 1)? + 192),
            2,
        ),
        224..=254 if accept == NewFormat::WholeOrPartial => {
            (Length::Partial(1 << (first & 0x1f)), 1)
        }
        255 if accept != NewFormat::OneOrTwoOctets => {
            (Length::Fixed(four_octets(bytes, at.checked_add(1)?)?), 5)
        }
        _ => return None,
    })
}

/// A body length alone: [`new_format`] for a reader that takes no partial
/// lengths.
pub fn new_format_whole(bytes: &[u8], at: usize, accept: NewFormat) -> Option<(usize, usize)> {
    match new_format(bytes, at, accept)? {
        (Length::Fixed(n), octets) => Some((n, octets)),
        (Length::Partial(_), _) => None,
    }
}

/// The length of the old-format packet whose header octet is `bytes[at]`
/// (its length type in the low two bits), and the number of octets the
/// length takes after it. `None` when it is cut short.
pub fn old_format(bytes: &[u8], at: usize) -> Option<(OldLength, usize)> {
    let header = *bytes.get(at)?;
    let from = at.checked_add(1)?;
    Some(match header & 3 {
        0 => (OldLength::Fixed(octet(bytes, from, 0)?), 1),
        1 => (
            OldLength::Fixed(octet(bytes, from, 0)? << 8 | octet(bytes, from, 1)?),
            2,
        ),
        2 => (OldLength::Fixed(four_octets(bytes, from)?), 4),
        _ => (OldLength::Indeterminate, 0),
    })
}

/// The length of the subpacket that starts at `bytes[at]` and the number
/// of octets it takes (RFC 9580 §5.2.3.7): one octet to 191, two for 192
/// to 254, and five for 255. There are no partial lengths. `None` when it
/// is cut short.
pub fn subpacket_length(bytes: &[u8], at: usize) -> Option<(usize, usize)> {
    let first = octet(bytes, at, 0)?;
    Some(match first {
        0..=191 => (first, 1),
        192..=254 => (((first - 192) << 8) + octet(bytes, at, 1)? + 192, 2),
        _ => (four_octets(bytes, at.checked_add(1)?)?, 5),
    })
}

/// A subpacket area that does not hold whole subpackets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BadArea;

impl fmt::Display for BadArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a subpacket area is cut short")
    }
}

/// One subpacket of a signature's area.
#[derive(Clone, Copy, Debug)]
pub struct Subpacket<'a> {
    /// The type, without the critical bit.
    pub kind: u8,
    /// Whether the type octet had the critical bit.
    pub critical: bool,
    /// What follows the type octet.
    pub body: &'a [u8],
}

/// The subpackets of an area in order. A subpacket whose length is cut
/// short, runs past the area or leaves no room for its type octet is an
/// `Err`, and the iteration ends there.
pub struct Subpackets<'a> {
    area: &'a [u8],
    at: usize,
}

impl<'a> Subpackets<'a> {
    /// The subpackets of `area`.
    pub fn new(area: &'a [u8]) -> Self {
        Subpackets { area, at: 0 }
    }
}

impl<'a> Iterator for Subpackets<'a> {
    type Item = Result<Subpacket<'a>, BadArea>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.area.len() {
            return None;
        }
        let read = (|| {
            let (n, octets) = subpacket_length(self.area, self.at)?;
            let from = self.at.checked_add(octets)?;
            let sub = self.area.get(from..from.checked_add(n)?)?;
            let (&kind, body) = sub.split_first()?;
            Some((from + n, kind, body))
        })();
        match read {
            Some((end, kind, body)) => {
                self.at = end;
                Some(Ok(Subpacket {
                    kind: kind & 0x7f,
                    critical: kind & 0x80 != 0,
                    body,
                }))
            }
            None => {
                self.at = self.area.len();
                Some(Err(BadArea))
            }
        }
    }
}
