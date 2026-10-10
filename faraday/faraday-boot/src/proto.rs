//! The pipe between the app's shell and the boot copier (`PLAN.md`
//! §5.5): one request, one response, each a frame.
//!
//! The framing is the disk process's (`faraday-files::proto`): a 4-byte
//! little-endian length, a 4-byte sequence number the answer repeats, and
//! that many bytes, a tag and the fields in order, a string a 2-byte
//! length and UTF-8. The limit is its own and small, [`MAX_FRAME`]: no
//! file ever crosses this pipe. The app asks for a listing, for the
//! source to be read, for a partition to be written, and for the source
//! to be forgotten; it never hands the copier bytes to write, so a
//! compromised app can at most have the running Faraday written.
//!
//! Every length is checked against what is left of the frame, and a frame
//! against [`MAX_FRAME`], before anything is allocated: each side treats
//! the other as hostile.

use std::io::{Read, Write};

/// The largest frame either side sends or takes. A listing of
/// [`MAX_PARTS`] partitions fits with room to spare.
pub const MAX_FRAME: u32 = 4096;

/// The most partitions a listing carries.
pub const MAX_PARTS: usize = 16;

/// The longest string a frame carries: an id, a release string, a
/// reason.
pub const MAX_STR: usize = 120;

/// What the shell asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// The boot partitions handed to the copier, with their versions.
    List,
    /// Read the boot partition that holds the running Faraday into
    /// memory.
    ReadSource,
    /// Write the source read over this boot partition, read back and
    /// compare.
    Write {
        /// The partition, as [`Part::id`] named it.
        target: String,
    },
    /// Drop the source from memory: the flow has ended.
    Forget,
}

/// A boot partition on a Faraday stick, as the copier lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The partition and the disk it is on, as this boot numbered them:
    /// a stick pulled and put back is another id.
    pub id: String,
    /// Its size in bytes.
    pub size: u64,
    /// The Faraday release string it carries, or `None` for a stick
    /// made before Faraday put one in its kernel (0.1.0 and earlier).
    pub release: Option<String>,
    /// It is the partition the source was read from.
    pub source: bool,
}

/// What the copier answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// The boot partitions handed out now.
    Parts(Vec<Part>),
    /// The source was read and holds the running Faraday.
    Source {
        /// The partition it was read from.
        id: String,
        /// The running kernel's release string, which it holds.
        release: String,
        /// Its size in bytes.
        size: u64,
    },
    /// The partition was written, read back and matched.
    Written {
        /// The partition.
        id: String,
        /// The release string it now carries.
        release: String,
    },
    /// The stick went while it was being written or read back: it does
    /// not boot until written again.
    Pulled,
    /// The source is forgotten.
    Forgotten,
    /// Why the request failed.
    Failed(String),
}

/// A malformed frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed;

struct Out(Vec<u8>);

impl Out {
    fn tag(t: u8) -> Out {
        Out(vec![t])
    }
    fn str(&mut self, s: &str) {
        // A string longer than the limit is cut at a character.
        let mut end = s.len().min(MAX_STR);
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        self.0.extend_from_slice(&(end as u16).to_le_bytes());
        self.0.extend_from_slice(&s.as_bytes()[..end]);
    }
    fn u64(&mut self, n: u64) {
        self.0.extend_from_slice(&n.to_le_bytes());
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
    fn u64(&mut self) -> Result<u64, Malformed> {
        let mut a = [0u8; 8];
        a.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(a))
    }
    fn str(&mut self) -> Result<String, Malformed> {
        let n = self.u16()?;
        if n > MAX_STR {
            return Err(Malformed);
        }
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| Malformed)
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
    /// The frame's bytes.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Request::List => Out::tag(1).0,
            Request::ReadSource => Out::tag(2).0,
            Request::Write { target } => {
                let mut o = Out::tag(3);
                o.str(target);
                o.0
            }
            Request::Forget => Out::tag(4).0,
        }
    }

    /// Reads a frame's bytes.
    pub fn decode(b: &[u8]) -> Result<Request, Malformed> {
        let mut i = In(b);
        let r = match i.u8()? {
            1 => Request::List,
            2 => Request::ReadSource,
            3 => Request::Write { target: i.str()? },
            4 => Request::Forget,
            _ => return Err(Malformed),
        };
        i.end()?;
        Ok(r)
    }
}

impl Response {
    /// The frame's bytes.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Response::Parts(parts) => {
                let mut o = Out::tag(0x81);
                let parts = &parts[..parts.len().min(MAX_PARTS)];
                o.0.push(parts.len() as u8);
                for p in parts {
                    o.str(&p.id);
                    o.u64(p.size);
                    match &p.release {
                        Some(r) => {
                            o.0.push(1);
                            o.str(r);
                        }
                        None => o.0.push(0),
                    }
                    o.0.push(u8::from(p.source));
                }
                o.0
            }
            Response::Source { id, release, size } => {
                let mut o = Out::tag(0x82);
                o.str(id);
                o.str(release);
                o.u64(*size);
                o.0
            }
            Response::Written { id, release } => {
                let mut o = Out::tag(0x83);
                o.str(id);
                o.str(release);
                o.0
            }
            Response::Pulled => Out::tag(0x84).0,
            Response::Forgotten => Out::tag(0x85).0,
            Response::Failed(why) => {
                let mut o = Out::tag(0xFF);
                o.str(why);
                o.0
            }
        }
    }

    /// Reads a frame's bytes.
    pub fn decode(b: &[u8]) -> Result<Response, Malformed> {
        let mut i = In(b);
        let r = match i.u8()? {
            0x81 => {
                let n = usize::from(i.u8()?);
                if n > MAX_PARTS {
                    return Err(Malformed);
                }
                let mut parts = Vec::with_capacity(n);
                for _ in 0..n {
                    let id = i.str()?;
                    let size = i.u64()?;
                    let release = match i.u8()? {
                        0 => None,
                        1 => Some(i.str()?),
                        _ => return Err(Malformed),
                    };
                    let source = match i.u8()? {
                        0 => false,
                        1 => true,
                        _ => return Err(Malformed),
                    };
                    parts.push(Part {
                        id,
                        size,
                        release,
                        source,
                    });
                }
                Response::Parts(parts)
            }
            0x82 => Response::Source {
                id: i.str()?,
                release: i.str()?,
                size: i.u64()?,
            },
            0x83 => Response::Written {
                id: i.str()?,
                release: i.str()?,
            },
            0x84 => Response::Pulled,
            0x85 => Response::Forgotten,
            0xFF => Response::Failed(i.str()?),
            _ => return Err(Malformed),
        };
        i.end()?;
        Ok(r)
    }
}

/// Sends one frame, in one write.
pub fn send(w: &mut dyn Write, seq: u32, payload: &[u8]) -> std::io::Result<()> {
    let len = u32::try_from(payload.len())
        .ok()
        .filter(|n| *n <= MAX_FRAME)
        .ok_or_else(|| std::io::Error::other("frame too large"))?;
    let mut frame = Vec::with_capacity(payload.len() + 8);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&seq.to_le_bytes());
    frame.extend_from_slice(payload);
    w.write_all(&frame).and_then(|()| w.flush())
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
        for r in [
            Request::List,
            Request::ReadSource,
            Request::Write {
                target: "sdb1@sdb#7".into(),
            },
            Request::Forget,
        ] {
            assert_eq!(Request::decode(&r.encode()), Ok(r));
        }
        for r in [
            Response::Parts(vec![
                Part {
                    id: "sda1@sda#4".into(),
                    size: 48 << 20,
                    release: Some("6.6.84-faraday-0.2.0+4d0680b1a2b3".into()),
                    source: true,
                },
                Part {
                    id: "sdb1@sdb#7".into(),
                    size: 48 << 20,
                    release: None,
                    source: false,
                },
            ]),
            Response::Source {
                id: "sda1@sda#4".into(),
                release: "6.6.84-faraday-0.2.0+4d0680b1a2b3".into(),
                size: 48 << 20,
            },
            Response::Written {
                id: "sdb1@sdb#7".into(),
                release: "6.6.84-faraday-0.2.0+4d0680b1a2b3".into(),
            },
            Response::Pulled,
            Response::Forgotten,
            Response::Failed("the stick is gone".into()),
        ] {
            assert_eq!(Response::decode(&r.encode()), Ok(r));
        }
    }

    #[test]
    fn a_frame_that_lies_about_its_lengths_or_size_is_refused() {
        let mut b = Request::Write { target: "s".into() }.encode();
        b[1] = 200;
        assert_eq!(Request::decode(&b), Err(Malformed));
        let mut b = Response::Pulled.encode();
        b.push(0);
        assert_eq!(Response::decode(&b), Err(Malformed));
        // A listing claiming more partitions than a frame may carry.
        assert_eq!(Response::decode(&[0x81, 200]), Err(Malformed));
        let mut big = Vec::new();
        big.extend_from_slice(&(MAX_FRAME + 1).to_le_bytes());
        big.extend_from_slice(&1u32.to_le_bytes());
        assert!(receive(&mut big.as_slice()).is_err());
        // The largest listing fits a frame.
        let long = "x".repeat(MAX_STR + 50);
        let parts = Response::Parts(
            (0..MAX_PARTS + 4)
                .map(|_| Part {
                    id: long.clone(),
                    size: u64::MAX,
                    release: Some(long.clone()),
                    source: false,
                })
                .collect(),
        );
        let mut sink = Vec::new();
        assert!(send(&mut sink, 1, &parts.encode()).is_ok());
    }
}
