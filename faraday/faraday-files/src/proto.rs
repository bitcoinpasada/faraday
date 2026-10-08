//! The pipe between the app's shell and the disk process (`PLAN.md`
//! §4.3): one request, one response, each a frame.
//!
//! A frame is a 4-byte little-endian length, a 4-byte sequence number
//! the answer repeats, and that many bytes. A shell that gave up waiting
//! for an answer knows the late one when it comes. The bytes are a tag
//! and the fields in order: a string is a 2-byte length
//! and UTF-8, bytes are a 4-byte length and the bytes, a list is a 2-byte
//! count and its items. Every length is checked against what is left of
//! the frame, and a frame against [`MAX_FRAME`], before anything is
//! allocated: each side treats the other as hostile.
//!
//! A file on its way through may be a seed written in the clear, and the
//! disk process lives on across the app's locks, so no copy is left
//! behind: a frame is built in a buffer sized for it (no move as it
//! grows), [`send`] wipes the frame it wrote, and each side wipes what it
//! received and what it answered once done with it ([`Request::wipe`],
//! [`Response::wipe`]).

use std::io::{Read, Write};

/// The largest frame either side sends or takes: the largest file plus
/// room for its name.
pub const MAX_FRAME: u32 = super::MAX_READ as u32 + 64 * 1024;

/// What the shell asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// The partitions handed out, with their files.
    List,
    /// One file's bytes.
    Read {
        /// The stick, as [`Stick::id`] named it.
        stick: String,
        /// The file.
        name: String,
    },
    /// A file written as `faraday_files::write_any` writes it.
    Write {
        /// The stick.
        stick: String,
        /// The name asked for.
        name: String,
        /// The contents.
        bytes: Vec<u8>,
    },
    /// The QR codes in a picture.
    ReadQr {
        /// The stick.
        stick: String,
        /// The picture.
        name: String,
    },
}

/// One stick partition, as the disk process lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stick {
    /// The partition and the disk it is on, as this boot numbered them:
    /// a stick pulled and put back is another id.
    pub id: String,
    /// Its volume label, or its node's name.
    pub label: String,
    /// On the disk the machine booted from.
    pub boot: bool,
    /// Its top-level files and their sizes.
    pub files: Vec<(String, u64)>,
}

/// What the disk process answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// The partitions.
    Sticks(Vec<Stick>),
    /// A file's bytes.
    Bytes(Vec<u8>),
    /// The name a file was written under.
    Written(String),
    /// The codes in a picture.
    Qr(Vec<Vec<u8>>),
    /// Why the request failed.
    Failed(String),
}

/// A malformed frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed;

struct Out(Vec<u8>);

impl Out {
    fn tag(t: u8) -> Out {
        Out::sized(t, 0)
    }
    /// A frame with room for `more` bytes after its tag, so that it never
    /// moves, and leaves a copy, as it fills.
    fn sized(t: u8, more: usize) -> Out {
        let mut v = Vec::with_capacity(more + 1);
        v.push(t);
        Out(v)
    }
    fn str(&mut self, s: &str) {
        // A string longer than a 2-byte length is cut at a character.
        let mut end = s.len().min(u16::MAX as usize);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        self.0.extend_from_slice(&(end as u16).to_le_bytes());
        self.0.extend_from_slice(&s.as_bytes()[..end]);
    }
    fn bytes(&mut self, b: &[u8]) {
        self.0.extend_from_slice(&(b.len() as u32).to_le_bytes());
        self.0.extend_from_slice(b);
    }
    fn count(&mut self, n: usize) {
        self.0
            .extend_from_slice(&(n.min(u16::MAX as usize) as u16).to_le_bytes());
    }
}

struct In<'a>(&'a [u8]);

impl In<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], Malformed> {
        if self.0.len() < n {
            return Err(Malformed);
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8, Malformed> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<usize, Malformed> {
        let b = self.take(2)?;
        Ok(usize::from(u16::from_le_bytes([b[0], b[1]])))
    }
    fn u32(&mut self) -> Result<usize, Malformed> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    }
    fn u64(&mut self) -> Result<u64, Malformed> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }
    fn str(&mut self) -> Result<String, Malformed> {
        let n = self.u16()?;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| Malformed)
    }
    fn bytes(&mut self) -> Result<Vec<u8>, Malformed> {
        let n = self.u32()?;
        Ok(self.take(n)?.to_vec())
    }
    fn end(&self) -> Result<(), Malformed> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(Malformed)
        }
    }
}

impl Request {
    /// Wipes the file it carries, if any.
    pub fn wipe(&mut self) {
        if let Request::Write { bytes, .. } = self {
            zeroize::Zeroize::zeroize(bytes);
        }
    }

    /// The frame's bytes.
    pub fn encode(&self) -> Vec<u8> {
        let o = match self {
            Request::List => Out::tag(1),
            Request::Read { stick, name } => {
                let mut o = Out::tag(2);
                o.str(stick);
                o.str(name);
                o
            }
            Request::Write { stick, name, bytes } => {
                let mut o = Out::sized(3, 4 + stick.len() + name.len() + 4 + bytes.len());
                o.str(stick);
                o.str(name);
                o.bytes(bytes);
                o
            }
            Request::ReadQr { stick, name } => {
                let mut o = Out::tag(4);
                o.str(stick);
                o.str(name);
                o
            }
        };
        o.0
    }

    /// Reads a frame's bytes.
    pub fn decode(b: &[u8]) -> Result<Request, Malformed> {
        let mut i = In(b);
        let r = match i.u8()? {
            1 => Request::List,
            2 => Request::Read {
                stick: i.str()?,
                name: i.str()?,
            },
            3 => Request::Write {
                stick: i.str()?,
                name: i.str()?,
                bytes: i.bytes()?,
            },
            4 => Request::ReadQr {
                stick: i.str()?,
                name: i.str()?,
            },
            _ => return Err(Malformed),
        };
        i.end()?;
        Ok(r)
    }
}

impl Response {
    /// Wipes the file or codes it carries, if any.
    pub fn wipe(&mut self) {
        match self {
            Response::Bytes(b) => zeroize::Zeroize::zeroize(b),
            Response::Qr(codes) => codes.iter_mut().for_each(zeroize::Zeroize::zeroize),
            _ => {}
        }
    }

    /// The frame's bytes.
    pub fn encode(&self) -> Vec<u8> {
        let o = match self {
            Response::Sticks(sticks) => {
                let mut o = Out::tag(0x81);
                o.count(sticks.len());
                for s in sticks.iter().take(u16::MAX as usize) {
                    o.str(&s.id);
                    o.str(&s.label);
                    o.0.push(u8::from(s.boot));
                    o.count(s.files.len());
                    for (n, len) in s.files.iter().take(u16::MAX as usize) {
                        o.str(n);
                        o.0.extend_from_slice(&len.to_le_bytes());
                    }
                }
                o
            }
            Response::Bytes(b) => {
                let mut o = Out::sized(0x82, 4 + b.len());
                o.bytes(b);
                o
            }
            Response::Written(n) => {
                let mut o = Out::tag(0x83);
                o.str(n);
                o
            }
            Response::Qr(codes) => {
                let size = codes.iter().map(|c| 4 + c.len()).sum::<usize>();
                let mut o = Out::sized(0x84, 2 + size);
                o.count(codes.len());
                for c in codes.iter().take(u16::MAX as usize) {
                    o.bytes(c);
                }
                o
            }
            Response::Failed(why) => {
                let mut o = Out::tag(0xFF);
                o.str(why);
                o
            }
        };
        o.0
    }

    /// Reads a frame's bytes.
    pub fn decode(b: &[u8]) -> Result<Response, Malformed> {
        let mut i = In(b);
        let r = match i.u8()? {
            0x81 => {
                let n = i.u16()?;
                let mut sticks = Vec::new();
                for _ in 0..n {
                    let id = i.str()?;
                    let label = i.str()?;
                    let boot = i.u8()? != 0;
                    let k = i.u16()?;
                    let mut files = Vec::new();
                    for _ in 0..k {
                        files.push((i.str()?, i.u64()?));
                    }
                    sticks.push(Stick {
                        id,
                        label,
                        boot,
                        files,
                    });
                }
                Response::Sticks(sticks)
            }
            0x82 => Response::Bytes(i.bytes()?),
            0x83 => Response::Written(i.str()?),
            0x84 => {
                let n = i.u16()?;
                let mut codes = Vec::new();
                for _ in 0..n {
                    codes.push(i.bytes()?);
                }
                Response::Qr(codes)
            }
            0xFF => Response::Failed(i.str()?),
            _ => return Err(Malformed),
        };
        i.end()?;
        Ok(r)
    }
}

/// Sends one frame. It goes in one write, so frames from one writer are
/// never interleaved with another's.
pub fn send(w: &mut dyn Write, seq: u32, payload: &[u8]) -> std::io::Result<()> {
    let len = u32::try_from(payload.len())
        .ok()
        .filter(|n| *n <= MAX_FRAME)
        .ok_or_else(|| std::io::Error::other("frame too large"))?;
    let mut frame = Vec::with_capacity(payload.len() + 8);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&seq.to_le_bytes());
    frame.extend_from_slice(payload);
    let sent = w.write_all(&frame).and_then(|()| w.flush());
    zeroize::Zeroize::zeroize(&mut frame);
    sent
}

/// Takes one frame and its sequence number; `None` when the other end has
/// closed.
pub fn receive(r: &mut dyn Read) -> std::io::Result<Option<(u32, Vec<u8>)>> {
    let mut head = [0u8; 8];
    match r.read_exact(&mut head) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let seq = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if len > MAX_FRAME {
        return Err(std::io::Error::other("frame too large"));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    Ok(Some((seq, buf)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_reads_back_as_written() {
        let requests = [
            Request::List,
            Request::Read {
                stick: "sdb1@7".into(),
                name: "savings-unsigned.psbt".into(),
            },
            Request::Write {
                stick: "sdb1@7".into(),
                name: "vault.ofv".into(),
                bytes: vec![1, 2, 3],
            },
            Request::ReadQr {
                stick: "sdb1@7".into(),
                name: "wallet-qr.png".into(),
            },
        ];
        for r in requests {
            assert_eq!(Request::decode(&r.encode()), Ok(r));
        }
        let responses = [
            Response::Sticks(vec![Stick {
                id: "sdb1@7".into(),
                label: "TESTSTICK".into(),
                boot: false,
                files: vec![("a.psbt".into(), 1234), ("café.txt".into(), 0)],
            }]),
            Response::Bytes(vec![9; 100]),
            Response::Written("signed-2.psbt".into()),
            Response::Qr(vec![b"wpkh(...)".to_vec(), vec![]]),
            Response::Failed("the stick is full".into()),
        ];
        for r in responses {
            assert_eq!(Response::decode(&r.encode()), Ok(r));
        }
    }

    #[test]
    fn a_frame_that_lies_about_its_lengths_is_refused() {
        let mut b = Request::Read {
            stick: "s".into(),
            name: "n".into(),
        }
        .encode();
        b[1] = 200;
        assert_eq!(Request::decode(&b), Err(Malformed));
        let mut b = Response::Bytes(vec![1, 2]).encode();
        b.push(0);
        assert_eq!(Response::decode(&b), Err(Malformed));
        let mut big = Vec::new();
        big.extend_from_slice(&(MAX_FRAME + 1).to_le_bytes());
        big.extend_from_slice(&1u32.to_le_bytes());
        assert!(receive(&mut big.as_slice()).is_err());
    }
}
