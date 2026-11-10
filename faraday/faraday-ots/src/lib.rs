//! OpenTimestamps proofs (`.ots`), read and checked offline
//! (`docs/decisions/F230.md`). A proof shows that a file existed by the time
//! a Bitcoin block was mined: a tree of operations takes the file's hash to
//! values that a block's merkle root, or a calendar's promise of one,
//! commits to. The reference is python-opentimestamps.
//!
//! [`Proof::parse`] reads the file and every refusal is an [`Error`] of its
//! own. [`Header::parse`] reads an 80-byte Bitcoin block header and refuses
//! one whose hash does not meet its own target. [`Proof::verify`] walks
//! every path of the tree and says, for each attestation at its end, what
//! it shows ([`Outcome`]). Faraday cannot know which header is in the chain:
//! a match says the file is committed to by the header given, and nothing
//! more.
//!
//! A Bitcoin path counts only when the file's hash and every hashing
//! operation on the path are SHA-256: a path through SHA-1, RIPEMD-160 or
//! Keccak-256 is reported with the operation named and is not counted. The
//! reference counts such a path.
//!
//! A proof is read within limits: 64 KiB of file, 16,384 entries in its
//! tree, and 16 MiB of values made by its operations over the whole tree,
//! each refused by an [`Error`] of its own.
//!
//! Holds no secrets. `no_std` + `alloc`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use faraday_crypto::work::{Compute, Work};
use sha1::Digest;

/// The longest argument of an append or prepend, and the longest value a
/// path may reach (the reference's `MAX_RESULT_LENGTH`).
pub const MAX_VALUE_LEN: usize = 4096;
/// The deepest tree read: the reference's recursion limit counts the root
/// as one level, so a chain of 255 operations is the longest.
pub const MAX_DEPTH: usize = 256;
/// The longest proof file read.
pub const MAX_PROOF_LEN: usize = 64 * 1024;
/// The most entries (operations and attestations) a proof's tree holds.
pub const MAX_ENTRIES: usize = 16_384;
/// The most bytes the operations of a whole tree may make, added up.
pub const MAX_WORK_BYTES: usize = 16 * 1024 * 1024;
/// The most bytes a number is written in (the reference's `MAX_VARUINT_BYTES`).
const MAX_VARUINT_BYTES: u32 = 9;
/// The longest attestation payload.
const MAX_PAYLOAD_LEN: usize = 8192;
/// The longest calendar address of a pending attestation.
const MAX_URI_LEN: usize = 1000;

const MAGIC: [u8; 31] = *b"\x00OpenTimestamps\x00\x00Proof\x00\xbf\x89\xe2\xe8\x84\xe8\x92\x94";
const TAG_PENDING: [u8; 8] = [0x83, 0xdf, 0xe3, 0x0d, 0x2e, 0xf9, 0x0c, 0x8e];
const TAG_BITCOIN: [u8; 8] = [0x05, 0x88, 0x96, 0x0d, 0x73, 0xd7, 0x19, 0x01];
const TAG_LITECOIN: [u8; 8] = [0x06, 0x86, 0x9a, 0x0d, 0x73, 0xd7, 0x1b, 0x45];
const TAG_ETHEREUM: [u8; 8] = [0x30, 0xfe, 0x80, 0x87, 0xb5, 0xc7, 0xea, 0xd7];

/// Why a proof or a header was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A proof file over 64 KiB.
    ProofTooLarge,
    /// A tree of more than 16,384 entries.
    TooManyEntries,
    /// A tree whose operations make more than 16 MiB of values in all.
    TooMuchWork,
    /// The file does not begin with the OpenTimestamps proof magic.
    BadMagic,
    /// A first version byte other than 1; the byte read.
    NotVersion1(u8),
    /// The proof ends inside a value.
    Truncated,
    /// Bytes follow the proof's tree.
    TrailingBytes,
    /// The file's hash is named by something other than SHA-1, RIPEMD-160
    /// or SHA-256.
    BadFileHashOp,
    /// The file's hash is Keccak-256, which the reference does not take
    /// for a file.
    FileHashKeccak256,
    /// An operation tag Faraday does not know.
    UnknownOp,
    /// An append or prepend with no bytes.
    EmptyArgument,
    /// An append or prepend argument over 4,096 bytes.
    ArgumentTooLong,
    /// An operation's result over 4,096 bytes.
    ResultTooLong,
    /// A tree deeper than 256 levels.
    TooDeep,
    /// A number written in more than 9 bytes.
    VaruintTooLong,
    /// The same operation twice at one node.
    RepeatedOperation,
    /// A block height that does not fit 32 bits.
    HeightTooLarge,
    /// An attestation payload over 8,192 bytes.
    PayloadTooLong,
    /// An attestation payload with bytes after its fields.
    PayloadTrailingBytes,
    /// A calendar address over 1,000 bytes.
    UriTooLong,
    /// A calendar address with a character other than letters, digits and
    /// `. - _ / :`.
    UriInvalid,
    /// A block header that is not 80 bytes.
    HeaderLength,
    /// A header whose target is zero, negative or too large to hold.
    HeaderTarget,
    /// A header whose target is above the highest the network allows.
    HeaderTargetTooEasy,
    /// A header whose hash is above its own target.
    HeaderWork,
}

/// The Bitcoin network a header is checked for: it sets the easiest target
/// a header may state (the proof-of-work limit of Bitcoin Core's chain
/// parameters).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network {
    /// Mainnet.
    Mainnet,
    /// Testnet3.
    Testnet3,
    /// Testnet4.
    Testnet4,
    /// The default signet.
    Signet,
    /// Regtest.
    Regtest,
}

impl Network {
    /// The easiest target, as the compact `nBits` Bitcoin Core gives it.
    fn limit_bits(self) -> u32 {
        match self {
            Network::Mainnet | Network::Testnet3 | Network::Testnet4 => 0x1d00_ffff,
            Network::Signet => 0x1e03_77ae,
            Network::Regtest => 0x207f_ffff,
        }
    }
}

/// A hash a proof's operations name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HashOp {
    /// SHA-1.
    Sha1,
    /// RIPEMD-160.
    Ripemd160,
    /// SHA-256, the only hash a Bitcoin path is trusted through.
    Sha256,
    /// Keccak-256 (the original padding, not SHA3-256).
    Keccak256,
}

impl HashOp {
    /// The hash of `data`.
    pub fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            HashOp::Sha1 => sha1::Sha1::digest(data).to_vec(),
            HashOp::Ripemd160 => ripemd::Ripemd160::digest(data).to_vec(),
            HashOp::Sha256 => faraday_crypto::sha256(data).to_vec(),
            HashOp::Keccak256 => sha3::Keccak256::digest(data).to_vec(),
        }
    }

    /// Costs a pass over every byte: the hash of a whole file, `data`,
    /// counted as [`Work::FileHash`]. The same value as [`HashOp::digest`].
    pub fn digest_whole(self, c: &Compute, data: &[u8]) -> Vec<u8> {
        c.count(Work::FileHash);
        self.digest(data)
    }

    /// The length of its output in bytes.
    pub fn digest_len(self) -> usize {
        match self {
            HashOp::Sha1 | HashOp::Ripemd160 => 20,
            HashOp::Sha256 | HashOp::Keccak256 => 32,
        }
    }

    /// The name the reference gives it.
    pub fn name(self) -> &'static str {
        match self {
            HashOp::Sha1 => "sha1",
            HashOp::Ripemd160 => "ripemd160",
            HashOp::Sha256 => "sha256",
            HashOp::Keccak256 => "keccak256",
        }
    }

    fn from_tag(tag: u8) -> Option<HashOp> {
        match tag {
            0x02 => Some(HashOp::Sha1),
            0x03 => Some(HashOp::Ripemd160),
            0x08 => Some(HashOp::Sha256),
            0x67 => Some(HashOp::Keccak256),
            _ => None,
        }
    }
}

/// One operation of a path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    /// The value, then these bytes.
    Append(Vec<u8>),
    /// These bytes, then the value.
    Prepend(Vec<u8>),
    /// The value's hash.
    Hash(HashOp),
    /// The value's bytes in reverse order.
    Reverse,
    /// The value as lowercase hex text.
    Hexlify,
}

impl Op {
    /// The value this operation makes from `value`.
    fn apply(&self, value: &[u8]) -> Result<Vec<u8>, Error> {
        if value.len() > MAX_VALUE_LEN {
            return Err(Error::ResultTooLong);
        }
        let out = match self {
            Op::Append(arg) => [value, arg].concat(),
            Op::Prepend(arg) => [arg.as_slice(), value].concat(),
            Op::Hash(h) => h.digest(value),
            Op::Reverse => value.iter().rev().copied().collect(),
            Op::Hexlify => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let mut s = Vec::with_capacity(value.len() * 2);
                for b in value {
                    s.push(HEX[usize::from(b >> 4)]);
                    s.push(HEX[usize::from(b & 15)]);
                }
                s
            }
        };
        if out.len() > MAX_VALUE_LEN {
            return Err(Error::ResultTooLong);
        }
        Ok(out)
    }
}

/// What an attestation at the end of a path claims.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Attestation {
    /// A calendar's promise to commit the value to Bitcoin: not complete.
    Pending {
        /// The calendar's address, in the characters the reference allows.
        uri: String,
    },
    /// The value is in a Bitcoin block's merkle root.
    Bitcoin {
        /// The block's height.
        height: u32,
    },
    /// The value is in a Litecoin block's merkle root.
    Litecoin {
        /// The block's height.
        height: u32,
    },
    /// The value is in an Ethereum block.
    Ethereum {
        /// The block's height.
        height: u32,
    },
    /// A kind Faraday does not know, by its tag.
    Unknown {
        /// The eight tag bytes.
        tag: [u8; 8],
        /// The payload as written.
        payload: Vec<u8>,
    },
}

/// One thing a value leads to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// The path ends here.
    Attestation(Attestation),
    /// The path goes on through an operation.
    Op(Op, Node),
}

/// A value in the tree and everything that follows from it, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// What follows.
    pub entries: Vec<Entry>,
}

/// A proof, read in full.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    file_op: HashOp,
    file_digest: Vec<u8>,
    tree: Node,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// Entries read so far, over the whole tree.
    entries: usize,
    /// Bytes the operations have made so far, over the whole tree.
    made: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated)?;
        let s = self.bytes.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(s)
    }

    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    /// The reference's varuint: seven bits a byte, low first, in at most
    /// 9 bytes (a value of 63 bits). A tenth byte is refused.
    fn varuint(&mut self) -> Result<u64, Error> {
        let mut value = 0u64;
        for i in 0..MAX_VARUINT_BYTES {
            let b = self.byte()?;
            value |= u64::from(b & 0x7f) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(Error::VaruintTooLong)
    }

    fn height(&mut self) -> Result<u32, Error> {
        u32::try_from(self.varuint()?).map_err(|_| Error::HeightTooLarge)
    }

    fn at_end(&self) -> bool {
        self.pos == self.bytes.len()
    }
}

fn read_attestation(r: &mut Reader) -> Result<Attestation, Error> {
    let tag: [u8; 8] = r.take(8)?.try_into().map_err(|_| Error::Truncated)?;
    let len = r.varuint()?;
    if len > MAX_PAYLOAD_LEN as u64 {
        return Err(Error::PayloadTooLong);
    }
    let payload = r.take(len as usize)?;
    let mut p = Reader {
        bytes: payload,
        pos: 0,
        entries: 0,
        made: 0,
    };
    let att = match tag {
        TAG_PENDING => {
            let n = p.varuint()?;
            if n > MAX_URI_LEN as u64 {
                return Err(Error::UriTooLong);
            }
            let raw = p.take(n as usize)?;
            let ok = raw.iter().all(|b| {
                b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'/' | b':')
            });
            if !ok {
                return Err(Error::UriInvalid);
            }
            // Every allowed byte is ASCII, so this is text.
            let uri = String::from_utf8(raw.to_vec()).map_err(|_| Error::UriInvalid)?;
            Attestation::Pending { uri }
        }
        TAG_BITCOIN => Attestation::Bitcoin {
            height: p.height()?,
        },
        TAG_LITECOIN => Attestation::Litecoin {
            height: p.height()?,
        },
        TAG_ETHEREUM => Attestation::Ethereum {
            height: p.height()?,
        },
        _ => {
            return Ok(Attestation::Unknown {
                tag,
                payload: payload.to_vec(),
            });
        }
    };
    if !p.at_end() {
        return Err(Error::PayloadTrailingBytes);
    }
    Ok(att)
}

/// One tag of a node's entries: an attestation (0x00) or an operation.
fn read_entry(
    r: &mut Reader,
    tag: u8,
    value: &[u8],
    limit: usize,
    ops: &mut BTreeSet<Op>,
) -> Result<Entry, Error> {
    r.entries += 1;
    if r.entries > MAX_ENTRIES {
        return Err(Error::TooManyEntries);
    }
    if tag == 0x00 {
        return Ok(Entry::Attestation(read_attestation(r)?));
    }
    let op = match tag {
        0xf0 | 0xf1 => {
            let n = r.varuint()?;
            if n > MAX_VALUE_LEN as u64 {
                return Err(Error::ArgumentTooLong);
            }
            if n == 0 {
                return Err(Error::EmptyArgument);
            }
            let arg = r.take(n as usize)?.to_vec();
            if tag == 0xf0 {
                Op::Append(arg)
            } else {
                Op::Prepend(arg)
            }
        }
        0xf2 => Op::Reverse,
        0xf3 => Op::Hexlify,
        _ => Op::Hash(HashOp::from_tag(tag).ok_or(Error::UnknownOp)?),
    };
    // As the reference does, before the operation is applied or anything
    // below it is read.
    if !ops.insert(op.clone()) {
        return Err(Error::RepeatedOperation);
    }
    let next = op.apply(value)?;
    r.made = r.made.saturating_add(next.len());
    if r.made > MAX_WORK_BYTES {
        return Err(Error::TooMuchWork);
    }
    let node = read_node(r, &next, limit - 1)?;
    Ok(Entry::Op(op, node))
}

/// The reference's `Timestamp.deserialize`: tags 0xff mean another follows.
fn read_node(r: &mut Reader, value: &[u8], limit: usize) -> Result<Node, Error> {
    if limit == 0 {
        return Err(Error::TooDeep);
    }
    let mut entries = Vec::new();
    let mut ops: BTreeSet<Op> = BTreeSet::new();
    loop {
        let first = r.byte()?;
        let (tag, last) = if first == 0xff {
            (r.byte()?, false)
        } else {
            (first, true)
        };
        let entry = read_entry(r, tag, value, limit, &mut ops)?;
        entries.push(entry);
        if last {
            // A node usually holds one entry: keep no spare room.
            entries.shrink_to_fit();
            return Ok(Node { entries });
        }
    }
}

impl Proof {
    /// Costs every operation of the tree worked out once, at most 16 MiB
    /// of values (hashes of up to 4,096 bytes each): reads a proof file in
    /// full, its magic, version 1, the hash the file is taken with and its
    /// digest, and the tree. Every value the tree's operations make is
    /// worked out as it is read, and one over 4,096 bytes refuses the
    /// proof, as does a file over 64 KiB, a tree of over 16,384 entries and
    /// a tree whose operations make over 16 MiB in all.
    pub fn parse(c: &Compute, bytes: &[u8]) -> Result<Proof, Error> {
        c.count(Work::FileHash);
        if bytes.len() > MAX_PROOF_LEN {
            return Err(Error::ProofTooLarge);
        }
        if !bytes.starts_with(&MAGIC) {
            return Err(if MAGIC.starts_with(bytes) {
                Error::Truncated
            } else {
                Error::BadMagic
            });
        }
        let mut r = Reader {
            bytes,
            pos: MAGIC.len(),
            entries: 0,
            made: 0,
        };
        let version = r.byte()?;
        if version != 1 {
            return Err(Error::NotVersion1(version));
        }
        let file_op = HashOp::from_tag(r.byte()?).ok_or(Error::BadFileHashOp)?;
        if file_op == HashOp::Keccak256 {
            return Err(Error::FileHashKeccak256);
        }
        let file_digest = r.take(file_op.digest_len())?.to_vec();
        let tree = read_node(&mut r, &file_digest, MAX_DEPTH)?;
        if !r.at_end() {
            return Err(Error::TrailingBytes);
        }
        Ok(Proof {
            file_op,
            file_digest,
            tree,
        })
    }

    /// The hash the proof takes the file with.
    pub fn file_op(&self) -> HashOp {
        self.file_op
    }

    /// The file's digest the proof states.
    pub fn file_digest(&self) -> &[u8] {
        &self.file_digest
    }

    /// The tree below the file's digest.
    pub fn tree(&self) -> &Node {
        &self.tree
    }

    /// Costs the operations of the tree worked out again, at most 16 MiB of
    /// values: walks every path from `file_digest` (the file's hash under
    /// [`Proof::file_op`]) and reports what each attestation shows against
    /// `header`. A digest that is not the proof's own digest ends the walk:
    /// the report says so and lists no path.
    pub fn verify(&self, c: &Compute, file_digest: &[u8], header: &Header) -> Report {
        c.count(Work::FileHash);
        let mut report = Report {
            file_matches: file_digest == self.file_digest.as_slice(),
            paths: Vec::new(),
        };
        if report.file_matches {
            let untrusted = (self.file_op != HashOp::Sha256).then_some(self.file_op);
            walk(
                &self.tree,
                file_digest,
                untrusted,
                header,
                &mut report.paths,
            );
        }
        report
    }
}

fn walk(
    node: &Node,
    value: &[u8],
    untrusted: Option<HashOp>,
    header: &Header,
    out: &mut Vec<Outcome>,
) {
    for entry in &node.entries {
        match entry {
            Entry::Attestation(a) => out.push(outcome(a, value, untrusted, header)),
            Entry::Op(op, next) => {
                // Parsing worked this value out already, so it cannot fail
                // here; a path that somehow did would show nothing.
                let Ok(v) = op.apply(value) else { continue };
                let u = match op {
                    Op::Hash(h) if *h != HashOp::Sha256 => untrusted.or(Some(*h)),
                    _ => untrusted,
                };
                walk(next, &v, u, header, out);
            }
        }
    }
}

fn outcome(a: &Attestation, value: &[u8], untrusted: Option<HashOp>, header: &Header) -> Outcome {
    match a {
        Attestation::Bitcoin { height } => {
            let height = *height;
            let reaches_header = value == header.merkle_root().as_slice();
            if let Some(op) = untrusted {
                Outcome::BitcoinNotSha256 {
                    height,
                    op,
                    reaches_header,
                }
            } else if reaches_header {
                Outcome::BitcoinMatched {
                    block_hash: header.block_hash(),
                    height,
                    time: header.time(),
                }
            } else {
                Outcome::BitcoinMismatch { height }
            }
        }
        Attestation::Pending { uri } => Outcome::Pending { uri: uri.clone() },
        Attestation::Litecoin { height } => Outcome::Litecoin { height: *height },
        Attestation::Ethereum { height } => Outcome::Ethereum { height: *height },
        Attestation::Unknown { tag, .. } => Outcome::Unknown { tag: *tag },
    }
}

/// What one attestation shows against the header given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The path is SHA-256 throughout and reaches the header's merkle root.
    BitcoinMatched {
        /// The header's hash, in the order block explorers show it.
        block_hash: [u8; 32],
        /// The height the attestation names, as written and not checked: a
        /// header holds no height, so only `block_hash` is to be compared
        /// with a block the person knows.
        height: u32,
        /// The header's time, in seconds since 1970.
        time: u32,
    },
    /// A SHA-256 path that does not reach this header's merkle root.
    BitcoinMismatch {
        /// The height the attestation names.
        height: u32,
    },
    /// A Bitcoin path through a hash other than SHA-256: not counted.
    BitcoinNotSha256 {
        /// The height the attestation names.
        height: u32,
        /// The first such hash on the path (the file's own hash counts).
        op: HashOp,
        /// The path's value is the header's merkle root. It is shown for
        /// the person to weigh and is not a match.
        reaches_header: bool,
    },
    /// A calendar's promise, not yet in a block.
    Pending {
        /// The calendar's address.
        uri: String,
    },
    /// A Litecoin attestation: listed, not checked.
    Litecoin {
        /// The height it names.
        height: u32,
    },
    /// An Ethereum attestation: listed, not checked.
    Ethereum {
        /// The height it names.
        height: u32,
    },
    /// An attestation of a kind Faraday does not know.
    Unknown {
        /// Its tag.
        tag: [u8; 8],
    },
}

/// Everything [`Proof::verify`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Report {
    /// The digest given is the one the proof states for the file. When it
    /// is not, nothing below is listed and nothing is shown.
    pub file_matches: bool,
    /// One outcome per attestation, in file order.
    pub paths: Vec<Outcome>,
}

impl Report {
    /// At least one path is SHA-256 throughout and reaches the header.
    pub fn committed(&self) -> bool {
        self.paths
            .iter()
            .any(|o| matches!(o, Outcome::BitcoinMatched { .. }))
    }
}

/// An 80-byte Bitcoin block header whose hash meets its own target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    raw: [u8; 80],
}

impl Header {
    /// Reads a header: exactly 80 bytes, a target that is a positive number
    /// that fits 256 bits and is at most `network`'s easiest, and a double
    /// SHA-256 at or below it. Whether the header is in the chain is not
    /// checked, and cannot be offline.
    pub fn parse(network: Network, bytes: &[u8]) -> Result<Header, Error> {
        let raw: [u8; 80] = bytes.try_into().map_err(|_| Error::HeaderLength)?;
        let h = Header { raw };
        let target = compact_target(h.bits())?;
        let limit = compact_target(network.limit_bits())?;
        if target > limit {
            return Err(Error::HeaderTargetTooEasy);
        }
        let mut hash = h.hash();
        hash.reverse();
        if hash > target {
            return Err(Error::HeaderWork);
        }
        Ok(h)
    }

    fn hash(&self) -> [u8; 32] {
        faraday_crypto::sha256(&faraday_crypto::sha256(&self.raw))
    }

    /// The block hash, in the order block explorers show it.
    pub fn block_hash(&self) -> [u8; 32] {
        let mut h = self.hash();
        h.reverse();
        h
    }

    /// The merkle root, as it is stored in the header.
    pub fn merkle_root(&self) -> [u8; 32] {
        let mut m = [0u8; 32];
        m.copy_from_slice(&self.raw[36..68]);
        m
    }

    /// The time, in seconds since 1970.
    pub fn time(&self) -> u32 {
        u32::from_le_bytes([self.raw[68], self.raw[69], self.raw[70], self.raw[71]])
    }

    fn bits(&self) -> u32 {
        u32::from_le_bytes([self.raw[72], self.raw[73], self.raw[74], self.raw[75]])
    }
}

/// The target a compact `nBits` states, as 32 big-endian bytes, read as
/// Bitcoin Core reads it; a zero, negative or oversized one is refused.
fn compact_target(bits: u32) -> Result<[u8; 32], Error> {
    let size = (bits >> 24) as i64;
    let word = bits & 0x007f_ffff;
    if word != 0 && bits & 0x0080_0000 != 0 {
        return Err(Error::HeaderTarget);
    }
    let mut target = [0u8; 32];
    if size <= 3 {
        let v = word >> (8 * (3 - size));
        target[28..].copy_from_slice(&v.to_be_bytes());
    } else {
        for i in 0..3i64 {
            let b = (word >> (8 * i)) as u8;
            if b != 0 {
                let idx = 31 - (size - 3) - i;
                if idx < 0 {
                    return Err(Error::HeaderTarget);
                }
                target[idx as usize] = b;
            }
        }
    }
    if target == [0u8; 32] {
        return Err(Error::HeaderTarget);
    }
    Ok(target)
}
