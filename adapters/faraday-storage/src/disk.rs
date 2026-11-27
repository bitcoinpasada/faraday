//! The disk process's client (02 §8, P11, P12): the two pipes, one request at a time, each
//! answered or failed `Silent` after its time. A vault too long for one frame goes in pieces.

use std::fs;
use std::io::Read as _;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use faraday_crypto::{Secret, Zeroize, Zeroizing, sha256};
use faraday_files::proto::{self, Kind, Problem, Request, Response};
use faraday_port::Failure;
use faraday_port::bytes::Bytes;
use faraday_port::storage::{
    self, Codes, NameRefused, OnTaken, Room, Sha256, StickId, StickInfo, StickMake, Unrenamed,
    Wiped, Written, Zeroing,
};

use crate::Sticks;

/// How long a request other than a write, a piece or a piece read may take.
const ASK: Duration = Duration::from_secs(15);
/// How long a write may take: a vault of four 4 MiB slots on a slow stick, written and read back.
const ASK_WRITE: Duration = Duration::from_secs(120);
/// How long one `ReadPart` may take: a piece is at most 16 MiB on a stick that may stall.
const ASK_READ_PART: Duration = Duration::from_secs(120);
/// How long a picture may take to be read for its codes.
const ASK_PICTURE: Duration = Duration::from_secs(60);

pub struct DiskSticks {
    tx: fs::File,
    answers: Receiver<(u32, Zeroizing<Vec<u8>>)>,
    seq: u32,
    /// The reader has ended: nothing more is read, so nothing more is written. Every ask fails.
    dead: bool,
    ids: Vec<StickId>,
    problems: Vec<(StickId, bool)>,
}

/// Eight bytes from `/dev/urandom`.
fn random_u64() -> std::io::Result<u64> {
    let mut b = [0u8; 8];
    fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

impl DiskSticks {
    /// Opens the two FIFOs read-write, which on a FIFO never waits for the other end: a disk
    /// process that is not up yet answers when it is.
    pub fn open(requests: &Path, responses: &Path) -> Result<Self, Failure> {
        let rw = |p: &Path| fs::OpenOptions::new().read(true).write(true).open(p);
        let mut tx = rw(requests).map_err(|_| Failure::Unavailable)?;
        let mut rx = rw(responses).map_err(|_| Failure::Unavailable)?;
        // A new conversation, told apart from an earlier app's by a number drawn at random: the
        // disk process forgets what that one left, and every frame before its answer to this one
        // is the earlier app's, dropped.
        let token = random_u64().map_err(|_| Failure::Unavailable)?;
        let seq = u32::try_from(token >> 32).unwrap_or(0);
        let mut opening = Request::Open { token }.encode();
        let opened = proto::send(&mut tx, seq, &opening);
        opening.zeroize();
        opened.map_err(|_| Failure::Gone)?;
        let (send, answers) = mpsc::channel();
        std::thread::Builder::new()
            .name(String::from("disk-answers"))
            .spawn(move || {
                let mut open = false;
                while let Ok(Some((seq, frame))) = proto::receive(&mut rx) {
                    if !open {
                        open = Response::decode(&frame) == Ok(Response::Opened(token));
                        continue;
                    }
                    if send.send((seq, frame)).is_err() {
                        return;
                    }
                }
                // A frame that cannot be read leaves the pipe out of step: the channel ends here
                // and every ask fails at once.
            })
            .map_err(|_| Failure::Unavailable)?;
        Ok(DiskSticks {
            tx,
            answers,
            seq,
            dead: false,
            ids: Vec::new(),
            problems: Vec::new(),
        })
    }

    fn ask(&mut self, req: &Request) -> Result<Response, Failure> {
        if self.dead {
            return Err(Failure::Silent);
        }
        self.seq = self.seq.wrapping_add(1);
        let mut encoded = req.encode();
        let sent = proto::send(&mut self.tx, self.seq, &encoded);
        encoded.zeroize();
        sent.map_err(|_| Failure::Gone)?;
        let wait = match req {
            Request::Write { .. } | Request::Piece { .. } => ASK_WRITE,
            Request::ReadPart { .. } => ASK_READ_PART,
            Request::ReadQr { .. } => ASK_PICTURE,
            _ => ASK,
        };
        let until = Instant::now() + wait;
        loop {
            let left = until.saturating_duration_since(Instant::now());
            match self.answers.recv_timeout(left) {
                Ok((seq, frame)) if seq == self.seq => {
                    return Response::decode(&frame).map_err(|_| Failure::Malformed);
                }
                // An answer to an earlier request that timed out: dropped, and wiped with it.
                Ok(_) => {}
                Err(e) => {
                    self.dead |= e == RecvTimeoutError::Disconnected;
                    return Err(Failure::Silent);
                }
            }
        }
    }
}

/// The disk process says why as a kind (02 §8); the port's `Failure` of the same name.
fn failed(kind: Kind) -> Failure {
    match kind {
        Kind::Gone => Failure::Gone,
        Kind::TooLarge => Failure::TooLarge,
        Kind::NoRoom => Failure::NoRoom,
        Kind::ReadOnly => Failure::ReadOnly,
        Kind::NotFat => Failure::NotFat,
    }
}

fn hashes(of: &[Sha256]) -> Vec<[u8; 32]> {
    of.iter().map(|h| h.0).collect()
}

impl Sticks for DiskSticks {
    fn list(&mut self) -> Vec<StickInfo> {
        let Ok(Response::Sticks(sticks)) = self.ask(&Request::List) else {
            return Vec::new();
        };
        self.problems = sticks
            .iter()
            .filter_map(|s| match s.problem {
                None | Some(Problem::NotYet) => None,
                Some(p) => Some((StickId(s.id.clone()), p == Problem::ReadOnly)),
            })
            .collect();
        let found: Vec<StickInfo> = sticks
            .into_iter()
            .map(|s| StickInfo {
                id: StickId(s.id),
                label: s.label,
                boot: s.boot,
                serial: s.serial,
                files: s.files,
                vaults: s.vaults,
                make: StickMake {
                    maker: s.make.maker,
                    model: s.make.model,
                    size: s.make.size,
                },
            })
            .collect();
        self.ids = found.iter().map(|s| s.id.clone()).collect();
        found
    }

    fn problems(&mut self) -> Vec<(StickId, bool)> {
        self.problems.clone()
    }

    fn read(&mut self, ask: storage::Read) -> Result<Bytes, Failure> {
        let mut out: Vec<u8> = Vec::new();
        let mut known = 0;
        loop {
            let from = out.len() as u64;
            let answer = self.ask(&Request::ReadPart {
                stick: ask.stick.0.clone(),
                name: ask.name.clone(),
                from,
                len: u32::try_from(proto::PIECE).unwrap_or(u32::MAX),
            });
            let (total, mut part) = match answer {
                Ok(Response::Part { total, bytes }) => (total, bytes),
                Ok(Response::Failed { kind, .. }) => {
                    out.zeroize();
                    return Err(failed(kind));
                }
                Ok(_) => {
                    out.zeroize();
                    return Err(Failure::Malformed);
                }
                Err(e) => {
                    out.zeroize();
                    return Err(e);
                }
            };
            let taken = accept_part(&mut out, known, total, &part);
            part.zeroize();
            if !taken {
                out.zeroize();
                return Err(Failure::TooLarge);
            }
            known = total;
            if out.len() as u64 >= total {
                return Ok(Secret::new(out));
            }
        }
    }

    fn read_codes(&mut self, ask: storage::ReadCodes) -> Result<Codes, Failure> {
        match self.ask(&Request::ReadQr {
            stick: ask.stick.0,
            name: ask.name,
        })? {
            Response::Qr { bytes, codes } => Ok(Codes {
                picture: Secret::new(bytes),
                codes: codes.into_iter().map(Secret::new).collect(),
            }),
            Response::Failed { kind, .. } => Err(failed(kind)),
            _ => Err(Failure::Malformed),
        }
    }

    fn write(&mut self, ask: storage::Write) -> Result<Written, Failure> {
        let bytes = ask.bytes.expose();
        let piece = proto::PIECE;
        let last_at = if bytes.len() > piece {
            (bytes.len() - 1) / piece * piece
        } else {
            0
        };
        for (k, part) in bytes[..last_at].chunks(piece).enumerate() {
            let mut req = Request::Piece {
                stick: ask.stick.0.clone(),
                name: ask.name.clone(),
                at: (k * piece) as u64,
                total: bytes.len() as u64,
                bytes: part.to_vec(),
            };
            let answer = self.ask(&req);
            req.wipe();
            match answer? {
                Response::Held(_) => {}
                Response::Failed { kind, .. } => return Err(failed(kind)),
                _ => return Err(Failure::Malformed),
            }
        }
        let on_taken = match ask.on_taken {
            OnTaken::Ask => faraday_files::OnTaken::Ask,
            OnTaken::Beside => faraday_files::OnTaken::Beside,
            OnTaken::Over(sha) => faraday_files::OnTaken::Over(sha.0),
        };
        let mut req = Request::Write {
            stick: ask.stick.0.clone(),
            name: ask.name.clone(),
            copies: hashes(&ask.copies),
            replace: ask.replace,
            renamed: ask.renamed,
            on_taken,
            total: bytes.len() as u64,
            bytes: bytes[last_at..].to_vec(),
        };
        let answer = self.ask(&req);
        req.wipe();
        match answer? {
            Response::Written(wrote_as) => Ok(Written::Wrote { wrote_as }),
            Response::WriteNameTaken { on_stick, sha } => match sha {
                Some(sha) => Ok(Written::NameTaken {
                    on_stick,
                    sha: Sha256(sha),
                }),
                // The file in the way is over the read limit or cannot be read: no hash to name.
                None => Err(Failure::TooLarge),
            },
            Response::Failed { kind, .. } => Err(failed(kind)),
            _ => Err(Failure::Malformed),
        }
    }

    fn wipe(&mut self, ask: storage::Wipe) -> Result<Wiped, Failure> {
        match self.ask(&Request::Wipe {
            stick: ask.stick.0,
            name: ask.name,
        })? {
            Response::Wiped => Ok(Wiped::Zeroed),
            Response::WipedName => Ok(Wiped::NameOnly),
            Response::Failed { kind, .. } => Err(failed(kind)),
            _ => Err(Failure::Malformed),
        }
    }

    fn space(&mut self, ask: storage::Space) -> Result<Room, Failure> {
        match self.ask(&Request::Space { stick: ask.stick.0 })? {
            Response::Space { free, size } => Ok(Room { free, size }),
            Response::Failed { kind, .. } => Err(failed(kind)),
            _ => Err(Failure::Malformed),
        }
    }

    fn zero_free(&mut self, ask: storage::ZeroFree) -> Result<Zeroing, Failure> {
        match self.ask(&Request::ZeroFree {
            stick: ask.stick.0,
            from: ask.from,
        })? {
            Response::Zeroing { next, total, done } => Ok(Zeroing { next, total, done }),
            Response::Failed { kind, .. } => Err(failed(kind)),
            _ => Err(Failure::Malformed),
        }
    }

    fn make_empty(&mut self, ask: storage::MakeEmpty) -> Result<(), Failure> {
        match self.ask(&Request::MakeEmpty { stick: ask.stick.0 })? {
            Response::Zeroing { .. } => Ok(()),
            Response::Failed { kind, .. } => Err(failed(kind)),
            _ => Err(Failure::Malformed),
        }
    }

    fn rename(&mut self, ask: storage::Rename) -> Result<(), Unrenamed> {
        let answer = self
            .ask(&Request::Rename {
                stick: ask.stick.0,
                from: ask.from,
                to: ask.to,
                vaults: ask.vaults.iter().map(|v| hashes(v)).collect(),
            })
            .map_err(Unrenamed::Failed)?;
        match answer {
            Response::Renamed => Ok(()),
            Response::NameTaken { same_vault: false } => Err(Unrenamed::Taken),
            Response::NameTaken { same_vault: true } => Err(Unrenamed::SameVault),
            Response::Failed { kind, .. } => Err(Unrenamed::Failed(failed(kind))),
            _ => Err(Unrenamed::Failed(Failure::Malformed)),
        }
    }

    fn name(&mut self, ask: storage::Name) -> Result<(), NameRefused> {
        let answer = self
            .ask(&Request::Name {
                stick: ask.stick.0,
                label: ask.label,
            })
            .map_err(NameRefused::Failed)?;
        match answer {
            Response::Renamed => Ok(()),
            Response::Failed { kind, .. } => Err(NameRefused::Failed(failed(kind))),
            _ => Err(NameRefused::Failed(Failure::Malformed)),
        }
    }

    fn hash(&mut self, ask: storage::Hash) -> Result<Sha256, Failure> {
        let bytes = self.read(storage::Read {
            stick: ask.stick,
            name: ask.name,
        })?;
        Ok(Sha256(sha256(bytes.expose())))
    }

    fn has(&self, stick: &StickId) -> bool {
        self.ids.contains(stick)
    }
}

/// Takes one piece of a read into `out`: the total within the vault limit, every piece stating it
/// again, each the length asked (P12: never a cut file).
fn accept_part(out: &mut Vec<u8>, known: u64, total: u64, part: &[u8]) -> bool {
    let first = out.is_empty();
    let total_ok = if first {
        total <= faraday_files::MAX_VAULT_READ
    } else {
        total == known
    };
    let want = total
        .saturating_sub(out.len() as u64)
        .min(proto::PIECE as u64);
    if !total_ok
        || part.len() as u64 != want
        || (first
            && out
                .try_reserve_exact(usize::try_from(total).unwrap_or(usize::MAX))
                .is_err())
    {
        return false;
    }
    out.extend_from_slice(part);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: a read piece of the wrong length, or that states another total than the first, is refused.
    /// Regression: a file handed to the app cut short or longer than the disk process said.
    /// Owner: adapter
    #[test]
    fn a_piece_that_does_not_fit_what_was_said_is_refused() {
        let mut out = Vec::new();
        assert!(
            !accept_part(&mut out, 0, 4, b"ab"),
            "a first piece short of the total"
        );
        assert!(accept_part(&mut out, 0, 2, b"ab"));
        assert!(!accept_part(&mut out, 2, 9, b"c"), "another total");
        assert_eq!(out, b"ab");
    }

    /// Invariant: each failure kind the disk process reports reaches the app as the port's failure of that name.
    /// Regression: every disk failure coming out as `Gone` words a full or read-only stick as a stick that went.
    /// Owner: adapter
    #[test]
    fn a_disk_failure_keeps_its_kind() {
        assert_eq!(failed(Kind::Gone), Failure::Gone);
        assert_eq!(failed(Kind::TooLarge), Failure::TooLarge);
        assert_eq!(failed(Kind::NoRoom), Failure::NoRoom);
        assert_eq!(failed(Kind::ReadOnly), Failure::ReadOnly);
        assert_eq!(failed(Kind::NotFat), Failure::NotFat);
    }
}
