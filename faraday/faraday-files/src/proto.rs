//! The pipe between the app's shell and the disk process (`docs/history/PLAN.md`
//! §4.3): one request, one response, each a frame.
//!
//! A frame is a 4-byte little-endian length, a 4-byte sequence number
//! the answer repeats, and that many bytes. A shell that gave up waiting
//! for an answer knows the late one when it comes. A picture's codes
//! ([`Request::ReadQr`]) are answered once read, after the requests asked
//! meanwhile: the shell has one picture asked at a time and knows its
//! answer by its number. The bytes are a tag
//! and the fields in order: a string is a 2-byte length
//! and UTF-8, bytes are a 4-byte length and the bytes, a flag is one byte,
//! 0 or 1, a list is a 2-byte count and its items. Every length is checked against what is left of
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

use zeroize::Zeroizing;

/// The largest frame either side sends or takes: the largest file plus
/// room for its name, or for a picture's codes beside its bytes.
pub const MAX_FRAME: u32 = super::MAX_READ as u32 + 1024 * 1024;

/// The disk process's answer to a request on a stick it does not hand
/// out to this process, one that is no longer clean: a token, not
/// wording, which the app shows in its own words.
pub const NOT_OPEN: &str = "stick-not-open";

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
        /// Over a file of that name, the person having chosen Replace.
        replace: bool,
        /// The person renamed this file For the stick: a vault going over
        /// its same vault takes this name (`faraday_files::write_file`).
        renamed: bool,
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
    /// A file deleted with zeros over it first (`faraday_files::wipe`).
    Wipe {
        /// The stick.
        stick: String,
        /// The file.
        name: String,
    },
    /// The free space and the whole data space, in bytes.
    Space {
        /// The stick.
        stick: String,
    },
    /// One step of zeros over the free space, from cluster `from`.
    ZeroFree {
        /// The stick.
        stick: String,
        /// The cluster the step starts at.
        from: u32,
    },
    /// The volume made empty, its layout and label kept.
    MakeEmpty {
        /// The stick.
        stick: String,
    },
    /// A file renamed in place (`faraday_files::rename`).
    Rename {
        /// The stick.
        stick: String,
        /// The file.
        from: String,
        /// Its new name.
        to: String,
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
    /// Its FAT volume serial number, when its boot sector states one.
    pub serial: Option<u32>,
    /// Its top-level files and their sizes.
    pub files: Vec<(String, u64)>,
    /// The files among them that are vaults by their contents and are
    /// not named `.fvlt` (`docs/VAULT-HARDENING.md` §2.1), read once
    /// when the listing was made.
    pub vaults: Vec<String>,
    /// Why it cannot be used for a backup, when it cannot: found by the disk
    /// process, which alone opens the partition.
    pub problem: Option<Problem>,
}

/// Why a stick or card cannot be used (`docs/BACKUP-FLOW.md` §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// No FAT filesystem Faraday reads: exFAT, NTFS, ext4, or none.
    NotFat,
    /// The device is read-only: a card's lock switch, a stick's.
    ReadOnly,
    /// Not handed out yet: this process cannot open it. It waits; nothing
    /// is wrong with it that is known.
    NotYet,
}

impl Problem {
    fn code(p: Option<Problem>) -> u8 {
        match p {
            None => 0,
            Some(Problem::NotFat) => 1,
            Some(Problem::ReadOnly) => 2,
            Some(Problem::NotYet) => 3,
        }
    }

    fn of(code: u8) -> Result<Option<Problem>, Malformed> {
        match code {
            0 => Ok(None),
            1 => Ok(Some(Problem::NotFat)),
            2 => Ok(Some(Problem::ReadOnly)),
            3 => Ok(Some(Problem::NotYet)),
            _ => Err(Malformed),
        }
    }
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
    /// A picture: its bytes as they are, and its codes' payloads.
    Qr {
        /// The picture's bytes, unchanged.
        bytes: Vec<u8>,
        /// Each code's payload.
        codes: Vec<Vec<u8>>,
    },
    /// The file is gone, zeros over it first.
    Wiped,
    /// The free space and the whole data space, in bytes.
    Space {
        /// Bytes no file holds.
        free: u64,
        /// Bytes in all.
        size: u64,
    },
    /// How far zeroing has gone.
    Zeroing {
        /// The cluster the next step starts at.
        next: u32,
        /// The volume's clusters.
        total: u32,
        /// Every free cluster is zeros.
        done: bool,
    },
    /// The file has its new name.
    Renamed,
    /// A rename refused, nothing written: another file has the name,
    /// ignoring case, and is the same vault when `same_vault`.
    NameTaken {
        /// That file is the same vault, by identity.
        same_vault: bool,
    },
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
            Request::Write {
                stick,
                name,
                replace,
                renamed,
                bytes,
            } => {
                let mut o = Out::sized(3, 4 + stick.len() + name.len() + 2 + 4 + bytes.len());
                o.str(stick);
                o.str(name);
                o.0.push(u8::from(*replace));
                o.0.push(u8::from(*renamed));
                o.bytes(bytes);
                o
            }
            Request::ReadQr { stick, name } => {
                let mut o = Out::tag(4);
                o.str(stick);
                o.str(name);
                o
            }
            Request::Wipe { stick, name } => {
                let mut o = Out::tag(5);
                o.str(stick);
                o.str(name);
                o
            }
            Request::Space { stick } => {
                let mut o = Out::tag(6);
                o.str(stick);
                o
            }
            Request::ZeroFree { stick, from } => {
                let mut o = Out::tag(7);
                o.str(stick);
                o.0.extend_from_slice(&u64::from(*from).to_le_bytes());
                o
            }
            Request::MakeEmpty { stick } => {
                let mut o = Out::tag(8);
                o.str(stick);
                o
            }
            Request::Rename { stick, from, to } => {
                let mut o = Out::tag(9);
                o.str(stick);
                o.str(from);
                o.str(to);
                o
            }
        };
        o.0
    }

    /// Reads a frame's bytes.
    pub fn decode(b: &[u8]) -> Result<Request, Malformed> {
        let mut i = In(b);
        let mut r = match i.u8()? {
            1 => Request::List,
            2 => Request::Read {
                stick: i.str()?,
                name: i.str()?,
            },
            3 => Request::Write {
                stick: i.str()?,
                name: i.str()?,
                replace: match i.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(Malformed),
                },
                renamed: match i.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(Malformed),
                },
                bytes: i.bytes()?,
            },
            4 => Request::ReadQr {
                stick: i.str()?,
                name: i.str()?,
            },
            5 => Request::Wipe {
                stick: i.str()?,
                name: i.str()?,
            },
            6 => Request::Space { stick: i.str()? },
            7 => Request::ZeroFree {
                stick: i.str()?,
                from: u32::try_from(i.u64()?).map_err(|_| Malformed)?,
            },
            8 => Request::MakeEmpty { stick: i.str()? },
            9 => Request::Rename {
                stick: i.str()?,
                from: i.str()?,
                to: i.str()?,
            },
            _ => return Err(Malformed),
        };
        // A file read in full is wiped when bytes follow it.
        if let Err(e) = i.end() {
            r.wipe();
            return Err(e);
        }
        Ok(r)
    }
}

impl Response {
    /// Wipes the file or codes it carries, if any.
    pub fn wipe(&mut self) {
        match self {
            Response::Bytes(b) => zeroize::Zeroize::zeroize(b),
            Response::Qr { bytes, codes } => {
                zeroize::Zeroize::zeroize(bytes);
                codes.iter_mut().for_each(zeroize::Zeroize::zeroize);
            }
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
                    match s.serial {
                        Some(n) => {
                            o.0.push(1);
                            o.0.extend_from_slice(&n.to_le_bytes());
                        }
                        None => o.0.push(0),
                    }
                    o.count(s.files.len());
                    for (n, len) in s.files.iter().take(u16::MAX as usize) {
                        o.str(n);
                        o.0.extend_from_slice(&len.to_le_bytes());
                    }
                    o.count(s.vaults.len());
                    for n in s.vaults.iter().take(u16::MAX as usize) {
                        o.str(n);
                    }
                    o.0.push(Problem::code(s.problem));
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
            Response::Qr { bytes, codes } => {
                let size = codes.iter().map(|c| 4 + c.len()).sum::<usize>();
                let mut o = Out::sized(0x84, 4 + bytes.len() + 2 + size);
                o.bytes(bytes);
                o.count(codes.len());
                for c in codes.iter().take(u16::MAX as usize) {
                    o.bytes(c);
                }
                o
            }
            Response::Wiped => Out::tag(0x85),
            Response::Space { free, size } => {
                let mut o = Out::tag(0x86);
                o.0.extend_from_slice(&free.to_le_bytes());
                o.0.extend_from_slice(&size.to_le_bytes());
                o
            }
            Response::Zeroing { next, total, done } => {
                let mut o = Out::tag(0x87);
                o.0.extend_from_slice(&u64::from(*next).to_le_bytes());
                o.0.extend_from_slice(&u64::from(*total).to_le_bytes());
                o.0.push(u8::from(*done));
                o
            }
            Response::Renamed => Out::tag(0x88),
            Response::NameTaken { same_vault } => {
                let mut o = Out::tag(0x89);
                o.0.push(u8::from(*same_vault));
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
        let mut r = match i.u8()? {
            0x81 => {
                let n = i.u16()?;
                let mut sticks = Vec::new();
                for _ in 0..n {
                    let id = i.str()?;
                    let label = i.str()?;
                    let boot = i.u8()? != 0;
                    let serial = if i.u8()? != 0 {
                        Some(i.u32()? as u32)
                    } else {
                        None
                    };
                    let k = i.u16()?;
                    let mut files = Vec::new();
                    for _ in 0..k {
                        files.push((i.str()?, i.u64()?));
                    }
                    let k = i.u16()?;
                    let mut vaults = Vec::new();
                    for _ in 0..k {
                        vaults.push(i.str()?);
                    }
                    let problem = Problem::of(i.u8()?)?;
                    sticks.push(Stick {
                        id,
                        label,
                        boot,
                        serial,
                        files,
                        vaults,
                        problem,
                    });
                }
                Response::Sticks(sticks)
            }
            0x82 => Response::Bytes(i.bytes()?),
            0x83 => Response::Written(i.str()?),
            0x84 => {
                // Wiped on every return: a count or code that does not
                // read leaves nothing of the picture behind.
                let mut bytes = Zeroizing::new(i.bytes()?);
                let n = i.u16()?;
                let mut codes: Zeroizing<Vec<Vec<u8>>> = Zeroizing::new(Vec::new());
                for _ in 0..n {
                    codes.push(i.bytes()?);
                }
                Response::Qr {
                    bytes: core::mem::take(&mut *bytes),
                    codes: core::mem::take(&mut *codes),
                }
            }
            0x85 => Response::Wiped,
            0x86 => Response::Space {
                free: i.u64()?,
                size: i.u64()?,
            },
            0x87 => Response::Zeroing {
                next: u32::try_from(i.u64()?).map_err(|_| Malformed)?,
                total: u32::try_from(i.u64()?).map_err(|_| Malformed)?,
                done: i.u8()? != 0,
            },
            0x88 => Response::Renamed,
            0x89 => Response::NameTaken {
                same_vault: match i.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(Malformed),
                },
            },
            0xFF => Response::Failed(i.str()?),
            _ => return Err(Malformed),
        };
        // A file or codes read in full are wiped when bytes follow them.
        if let Err(e) = i.end() {
            r.wipe();
            return Err(e);
        }
        Ok(r)
    }
}

/// Sends one frame. It goes in one write, so frames from one writer are
/// never interleaved with another's.
pub fn send(w: &mut dyn Write, seq: u32, payload: &[u8]) -> std::io::Result<()> {
    let len = u32::try_from(payload.len())
        .ok()
        .filter(|n| *n <= MAX_FRAME)
        // not wording: the pipe's own error; faraday-storage says what it shows
        .ok_or_else(|| std::io::Error::other("frame too large"))?;
    let mut frame = Vec::with_capacity(payload.len() + 8);
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&seq.to_le_bytes());
    frame.extend_from_slice(payload);
    let sent = w.write_all(&frame).and_then(|()| w.flush());
    zeroize::Zeroize::zeroize(&mut frame);
    sent
}

/// Takes one frame and its sequence number, the frame wiped when it is
/// dropped; `None` when the other end has closed.
pub fn receive(r: &mut dyn Read) -> std::io::Result<Option<(u32, Zeroizing<Vec<u8>>)>> {
    let mut head = [0u8; 8];
    match r.read_exact(&mut head) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let seq = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if len > MAX_FRAME {
        // not wording: the pipe's own error, as above
        return Err(std::io::Error::other("frame too large"));
    }
    let mut buf = Zeroizing::new(vec![0u8; len as usize]);
    r.read_exact(&mut buf)?;
    Ok(Some((seq, buf)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticks_with_a_problem_read_back_as_written() {
        let r = Response::Sticks(vec![
            Stick {
                id: "sdc1@8".into(),
                label: "BIG".into(),
                boot: false,
                serial: None,
                files: Vec::new(),
                vaults: Vec::new(),
                problem: Some(Problem::NotFat),
            },
            Stick {
                id: "sdd1@9".into(),
                label: "LOCKED".into(),
                boot: false,
                serial: Some(1),
                files: vec![("a.txt".into(), 1)],
                vaults: Vec::new(),
                problem: Some(Problem::ReadOnly),
            },
        ]);
        assert_eq!(Response::decode(&r.encode()), Ok(r));
    }

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
                name: "vault.fvlt".into(),
                replace: false,
                renamed: true,
                bytes: vec![1, 2, 3],
            },
            Request::Write {
                stick: "sdb1@7".into(),
                name: "wallet.json".into(),
                replace: true,
                renamed: false,
                bytes: vec![4, 5],
            },
            Request::ReadQr {
                stick: "sdb1@7".into(),
                name: "wallet-qr.png".into(),
            },
            Request::Wipe {
                stick: "sdb1@7".into(),
                name: "old-seed.txt".into(),
            },
            Request::Space {
                stick: "sdb1@7".into(),
            },
            Request::ZeroFree {
                stick: "sdb1@7".into(),
                from: 70_000,
            },
            Request::MakeEmpty {
                stick: "sdb1@7".into(),
            },
            Request::Rename {
                stick: "sdb1@7".into(),
                from: "vault.fvlt".into(),
                to: "savings.fvlt".into(),
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
                serial: Some(0xcafe_f00d),
                files: vec![("a.psbt".into(), 1234), ("café.txt".into(), 0)],
                vaults: vec!["VAULT~1.FVL".into()],
                problem: None,
            }]),
            Response::Bytes(vec![9; 100]),
            Response::Written("signed-2.psbt".into()),
            Response::Qr {
                bytes: b"\x89PNG...".to_vec(),
                codes: vec![b"wpkh(...)".to_vec(), vec![]],
            },
            Response::Wiped,
            Response::Space {
                free: 7 << 30,
                size: 15 << 30,
            },
            Response::Zeroing {
                next: 70_000,
                total: 490_000,
                done: false,
            },
            Response::Renamed,
            Response::NameTaken { same_vault: true },
            Response::NameTaken { same_vault: false },
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
