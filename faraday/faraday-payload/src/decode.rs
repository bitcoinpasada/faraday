//! The verifying, streaming decoder (`docs/STARTER.md` §1.3, §4.1).

use core::fmt;
use std::io::{self, Read, Seek, SeekFrom, Write};

use lzma_rust2::{Action, Status, XzStream};
use sha2::{Digest, Sha256};

use crate::{PayloadFile, Sha256Sum};

/// The most memory the xz decoder may take for one block, in KiB.
/// `xz -9e` streams need 65 MiB (a 64 MiB dictionary); a stream asking
/// for more than this is refused before anything is allocated for it.
pub const MEMORY_LIMIT_KIB: u32 = 128 * 1024;

/// What is read from the source at a time.
const IN_CHUNK: usize = 64 * 1024;
/// What is handed to the writer at a time, at most.
const OUT_CHUNK: usize = 1024 * 1024;

/// What a developer's image file is (`--image`, `docs/STARTER.md` §1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    /// The image's own bytes: an `.img`, or a `-boot.vfat`.
    Plain,
    /// An `.xz` holding the image.
    Xz,
}

impl ImageKind {
    /// `Xz` for a name ending in `.xz`, otherwise `Plain`.
    pub fn of_name(name: &str) -> Self {
        if name.ends_with(".xz") {
            Self::Xz
        } else {
            Self::Plain
        }
    }
}

/// The SHA-256 and size of an image, as decompressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageSum {
    /// SHA-256 of the image's bytes.
    pub sha256: Sha256Sum,
    /// The image's size in bytes.
    pub size: u64,
}

/// Why a payload file or an image was not written, or not written
/// correctly.
#[derive(Debug)]
pub enum PayloadError {
    /// Reading the source failed.
    Read(io::Error),
    /// Writing the destination failed.
    Write(io::Error),
    /// The payload file's SHA-256 is not the manifest's. Nothing was
    /// written.
    PayloadSum {
        /// The manifest's sum.
        expected: Sha256Sum,
        /// The file's.
        actual: Sha256Sum,
    },
    /// The payload file matched its manifest line when it was checked,
    /// and read differently while it was decompressed.
    PayloadChanged,
    /// The `.xz` is not a whole, intact xz stream: it is truncated, it is
    /// malformed, one of its integrity checks failed, or it needs more
    /// memory than [`MEMORY_LIMIT_KIB`]. The decoder's own words.
    Damaged(String),
    /// The decompressed bytes run past the expected size. Nothing past
    /// that size was written.
    Longer {
        /// The expected size.
        expected: u64,
    },
    /// The decompressed bytes stop short of the expected size.
    Shorter {
        /// The expected size.
        expected: u64,
        /// What was decompressed and written.
        actual: u64,
    },
    /// The decompressed bytes' SHA-256 is not the expected one.
    Sum {
        /// The sum the manifest or the Confirm page gave.
        expected: Sha256Sum,
        /// The sum of what was written.
        actual: Sha256Sum,
    },
}

impl fmt::Display for PayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(e) => write!(f, "reading the source failed: {e}"),
            Self::Write(e) => write!(f, "writing failed: {e}"),
            Self::PayloadSum { expected, actual } => write!(
                f,
                "the payload file's SHA-256 is {actual}, the manifest's is {expected}; nothing was written"
            ),
            Self::PayloadChanged => write!(f, "the payload file changed while it was read"),
            Self::Damaged(e) => write!(f, "the .xz file is damaged: {e}"),
            Self::Longer { expected } => {
                write!(f, "the decompressed image is longer than {expected} bytes")
            }
            Self::Shorter { expected, actual } => write!(
                f,
                "the decompressed image is {actual} bytes, not {expected}"
            ),
            Self::Sum { expected, actual } => write!(
                f,
                "the decompressed image's SHA-256 is {actual}, not {expected}"
            ),
        }
    }
}

impl std::error::Error for PayloadError {}

/// Writes the image or binary inside a payload file to `out`.
///
/// `src` is the payload file, the `.xz`, positioned at its start, and
/// `file` its manifest line. First the whole of `src` is read and its
/// SHA-256 compared with the manifest's; on a mismatch nothing is
/// written. Then `src` is read again from the same position and
/// decompressed into `out` as it is read, and `progress` is called with
/// the bytes written so far and the manifest's size after each write.
/// Success means `out` was flushed and received exactly the manifest's
/// size in bytes, with the manifest's SHA-256.
pub fn write_payload<R, W, P>(
    mut src: R,
    file: &PayloadFile,
    out: W,
    progress: P,
) -> Result<(), PayloadError>
where
    R: Read + Seek,
    W: Write,
    P: FnMut(u64, u64),
{
    let start = src.stream_position().map_err(PayloadError::Read)?;
    let actual = sum_of(&mut src)?;
    if actual != file.sha256 {
        return Err(PayloadError::PayloadSum {
            expected: file.sha256,
            actual,
        });
    }
    src.seek(SeekFrom::Start(start))
        .map_err(PayloadError::Read)?;

    let expected = ImageSum {
        sha256: file.inner_sha256,
        size: file.inner_size,
    };
    let mut writer = Verified::new(out, expected.size, progress);
    let read_again = stream(&mut src, ImageKind::Xz, |chunk| writer.take(chunk))?;
    if read_again != file.sha256 {
        return Err(PayloadError::PayloadChanged);
    }
    writer.finish(expected.sha256)
}

/// The SHA-256 and size of a developer's image, decompressed first if
/// it is an `.xz`. Nothing is written; this is the sum the Confirm page
/// shows.
pub fn hash_image<R: Read>(src: R, kind: ImageKind) -> Result<ImageSum, PayloadError> {
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    stream(src, kind, |chunk| {
        hasher.update(chunk);
        size += chunk.len() as u64;
        Ok(())
    })?;
    Ok(ImageSum {
        sha256: Sha256Sum(hasher.finalize().into()),
        size,
    })
}

/// Writes a developer's image to `out`, decompressed first if it is an
/// `.xz`. `expected` is what [`hash_image`] gave for the same file when
/// the person confirmed; `progress` is called with the bytes written so
/// far and `expected.size` after each write. Success means `out` was
/// flushed and received exactly `expected.size` bytes with
/// `expected.sha256`, so a file that changed after it was confirmed is
/// reported, never written as a success.
pub fn write_image<R, W, P>(
    src: R,
    kind: ImageKind,
    expected: &ImageSum,
    out: W,
    progress: P,
) -> Result<(), PayloadError>
where
    R: Read,
    W: Write,
    P: FnMut(u64, u64),
{
    let mut writer = Verified::new(out, expected.size, progress);
    stream(src, kind, |chunk| writer.take(chunk))?;
    writer.finish(expected.sha256)
}

/// A writer that counts, hashes and bounds what it is given.
struct Verified<W, P> {
    out: W,
    expected_size: u64,
    written: u64,
    hasher: Sha256,
    progress: P,
}

impl<W: Write, P: FnMut(u64, u64)> Verified<W, P> {
    fn new(out: W, expected_size: u64, progress: P) -> Self {
        Self {
            out,
            expected_size,
            written: 0,
            hasher: Sha256::new(),
            progress,
        }
    }

    fn take(&mut self, chunk: &[u8]) -> Result<(), PayloadError> {
        let after = self.written + chunk.len() as u64;
        if after > self.expected_size {
            return Err(PayloadError::Longer {
                expected: self.expected_size,
            });
        }
        self.out.write_all(chunk).map_err(PayloadError::Write)?;
        self.hasher.update(chunk);
        self.written = after;
        (self.progress)(self.written, self.expected_size);
        Ok(())
    }

    fn finish(mut self, expected: Sha256Sum) -> Result<(), PayloadError> {
        self.out.flush().map_err(PayloadError::Write)?;
        if self.written != self.expected_size {
            return Err(PayloadError::Shorter {
                expected: self.expected_size,
                actual: self.written,
            });
        }
        let actual = Sha256Sum(self.hasher.finalize().into());
        if actual != expected {
            return Err(PayloadError::Sum { expected, actual });
        }
        Ok(())
    }
}

/// One read from `src`, an interrupted one retried.
fn read_some<R: Read>(src: &mut R, buf: &mut [u8]) -> Result<usize, PayloadError> {
    loop {
        match src.read(buf) {
            Ok(n) => return Ok(n),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(PayloadError::Read(e)),
        }
    }
}

/// The SHA-256 of everything `src` holds from where it is.
fn sum_of<R: Read>(mut src: R) -> Result<Sha256Sum, PayloadError> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; IN_CHUNK];
    loop {
        let n = read_some(&mut src, &mut buf)?;
        if n == 0 {
            return Ok(Sha256Sum(hasher.finalize().into()));
        }
        hasher.update(&buf[..n]);
    }
}

/// Reads `src` to its end and hands its contents to `emit`, decompressed
/// if `kind` is `Xz`, a chunk at a time. Returns the SHA-256 of the bytes
/// read from `src`.
fn stream<R, E>(mut src: R, kind: ImageKind, mut emit: E) -> Result<Sha256Sum, PayloadError>
where
    R: Read,
    E: FnMut(&[u8]) -> Result<(), PayloadError>,
{
    let mut src_hash = Sha256::new();
    match kind {
        ImageKind::Plain => {
            let mut buf = vec![0u8; OUT_CHUNK];
            loop {
                let n = read_some(&mut src, &mut buf)?;
                if n == 0 {
                    break;
                }
                src_hash.update(&buf[..n]);
                emit(&buf[..n])?;
            }
        }
        ImageKind::Xz => {
            // Concatenated streams and stream padding are accepted, as
            // `xz -d` accepts them; anything else after a stream is not.
            let mut decoder = XzStream::new_mem_limit(true, MEMORY_LIMIT_KIB);
            let mut input = vec![0u8; IN_CHUNK];
            let mut output = vec![0u8; OUT_CHUNK];
            let (mut in_pos, mut in_len, mut eof) = (0usize, 0usize, false);
            let mut stalls = 0u32;
            loop {
                if in_pos == in_len && !eof {
                    in_len = read_some(&mut src, &mut input)?;
                    in_pos = 0;
                    if in_len == 0 {
                        eof = true;
                    } else {
                        src_hash.update(&input[..in_len]);
                    }
                }
                let action = if eof { Action::Finish } else { Action::Run };
                let step = decoder
                    .process(&input[in_pos..in_len], &mut output, action)
                    .map_err(|e| PayloadError::Damaged(e.to_string()))?;
                in_pos += step.bytes_consumed;
                if step.bytes_produced > 0 {
                    emit(&output[..step.bytes_produced])?;
                }
                if step.status == Status::StreamEnd {
                    break;
                }
                // A decoder that neither reads nor writes while it has
                // input, or after the end of it, would spin here forever.
                if step.bytes_consumed == 0 && step.bytes_produced == 0 && (in_pos < in_len || eof)
                {
                    stalls += 1;
                    if stalls > 3 {
                        return Err(PayloadError::Damaged(
                            "the decoder stopped before the end of the stream".into(),
                        ));
                    }
                } else {
                    stalls = 0;
                }
            }
            if in_pos < in_len || (!eof && read_some(&mut src, &mut input)? != 0) {
                return Err(PayloadError::Damaged("data after the xz stream".into()));
            }
        }
    }
    Ok(Sha256Sum(src_hash.finalize().into()))
}
