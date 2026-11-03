//! `IMAGES`, the release's image list (`docs/IMAGES.md` §4), and what the
//! Starter does with it to find images a person downloaded (§5): a first
//! line `# Faraday X.Y.Z images, commit HASH`, then one line per image,
//! six fields separated by one tab: the release file (`….img.xz`), its
//! SHA-256, the SHA-256 of the `.img` inside it, the platform, the
//! screen, and the boards it starts joined by `, `.
//!
//! Nothing here touches a disk: [`ImageList::classify`] reads a file's
//! name alone, and [`sha256_of`] reads a `Read`, so every refusal is
//! tested without a disk.
//!
//! A sum found here is an offer only. What is written later is checked
//! again as it is written ([`crate::write_payload`]'s rule), so a file
//! that changed after it was hashed is refused there.

use core::fmt;
use std::io::{self, Read};

use sha2::{Digest, Sha256};

use crate::Sha256Sum;

const HEADER_PREFIX: &str = "# Faraday ";
const HEADER_MIDDLE: &str = " images, commit ";
const IMAGE_SUFFIX: &str = ".img.xz";

/// A parsed `IMAGES`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageList {
    version: String,
    commit: String,
    images: Vec<ListedImage>,
}

/// One image's line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedImage {
    /// The release file's name, ending in `.img.xz`.
    pub file: String,
    /// The SHA-256 of the release file, the `.xz` itself.
    pub xz_sha256: Sha256Sum,
    /// The SHA-256 of the `.img` inside it.
    pub img_sha256: Sha256Sum,
    /// The platform (`x86_64-uefi`, `pi3`).
    pub board: String,
    /// The screen (`efi-framebuffer`, `any`).
    pub panel: String,
    /// The boards it starts.
    pub starts: Vec<String>,
}

/// Why an `IMAGES` was refused, and on which line (1 is the first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageListError {
    /// The line, counted from 1.
    pub line: usize,
    /// What is wrong with it.
    pub fault: ImageListFault,
}

/// What is wrong with an `IMAGES` line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageListFault {
    /// The file is not UTF-8 text.
    NotText,
    /// The first line is not `# Faraday VERSION images, commit HASH`.
    Header,
    /// An empty line where an image's line belongs.
    EmptyLine,
    /// An image's line does not have exactly six tab-separated fields.
    FieldCount(usize),
    /// A file name is empty, holds a path separator or a control
    /// character, starts with a dot, does not end in `.img.xz`, or does
    /// not carry the list's version.
    Name,
    /// A sum is not 64 lowercase hexadecimal digits.
    Sum,
    /// The platform, the screen or a board is empty or holds a control
    /// character.
    Field,
    /// An image is named twice.
    Duplicate,
    /// The list names no images.
    NoImages,
}

impl fmt::Display for ImageListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.fault {
            ImageListFault::NotText => "not UTF-8 text",
            ImageListFault::Header => "not an image list header",
            ImageListFault::EmptyLine => "empty line",
            ImageListFault::FieldCount(_) => "not six tab-separated fields",
            ImageListFault::Name => "bad file name",
            ImageListFault::Sum => "bad SHA-256",
            ImageListFault::Field => "bad platform, screen or board",
            ImageListFault::Duplicate => "file named twice",
            ImageListFault::NoImages => "no images",
        };
        write!(f, "IMAGES line {}: {what}", self.line)
    }
}

impl std::error::Error for ImageListError {}

/// What a file's name says about the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameClass<'a> {
    /// The name is on the list.
    Listed(&'a ListedImage),
    /// The name is a listed one with another version in it.
    OtherRelease {
        /// The listed image the name stands for.
        image: &'a ListedImage,
        /// The version in the file's name.
        found: &'a str,
    },
    /// The name is not one of the list's.
    Unrelated,
}

impl ImageList {
    /// Parses `IMAGES`'s bytes. A final newline is allowed and not
    /// required; any other empty line is refused.
    pub fn parse(bytes: &[u8]) -> Result<Self, ImageListError> {
        let text = core::str::from_utf8(bytes).map_err(|_| ImageListError {
            line: 1,
            fault: ImageListFault::NotText,
        })?;
        let text = text.strip_suffix('\n').unwrap_or(text);
        let mut lines = text.split('\n');
        let (version, commit) = parse_header(lines.next().unwrap_or("")).ok_or(ImageListError {
            line: 1,
            fault: ImageListFault::Header,
        })?;
        let mut images: Vec<ListedImage> = Vec::new();
        for (i, line) in lines.enumerate() {
            let number = i + 2;
            let fail = |fault| ImageListError {
                line: number,
                fault,
            };
            let image = ListedImage::parse_line(line, version).map_err(fail)?;
            if images.iter().any(|f| f.file == image.file) {
                return Err(fail(ImageListFault::Duplicate));
            }
            images.push(image);
        }
        if images.is_empty() {
            return Err(ImageListError {
                line: 2,
                fault: ImageListFault::NoImages,
            });
        }
        Ok(Self {
            version: version.to_owned(),
            commit: commit.to_owned(),
            images,
        })
    }

    /// The release version the list was staged for: the one this Starter
    /// needs.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The full commit hash the release was built from.
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// Every image, in the list's order.
    pub fn images(&self) -> &[ListedImage] {
        &self.images
    }

    /// What `name` is to this list, from the name alone.
    pub fn classify<'a>(&'a self, name: &'a str) -> NameClass<'a> {
        if let Some(image) = self.images.iter().find(|i| i.file == name) {
            return NameClass::Listed(image);
        }
        for image in &self.images {
            let Some((pre, post)) = image.file.split_once(self.version.as_str()) else {
                continue;
            };
            let Some(found) = name
                .strip_prefix(pre)
                .and_then(|rest| rest.strip_suffix(post))
            else {
                continue;
            };
            if found != self.version && version_like(found) {
                return NameClass::OtherRelease { image, found };
            }
        }
        NameClass::Unrelated
    }
}

fn parse_header(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix(HEADER_PREFIX)?;
    let (version, commit) = rest.split_once(HEADER_MIDDLE)?;
    let commit_ok = matches!(commit.len(), 40 | 64)
        && commit
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
    (version_like(version) && commit_ok).then_some((version, commit))
}

/// A version as a release file's name carries it: it starts with a
/// digit and holds letters, digits and `. - + _` only.
fn version_like(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_digit())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
}

impl ListedImage {
    fn parse_line(line: &str, version: &str) -> Result<Self, ImageListFault> {
        if line.is_empty() {
            return Err(ImageListFault::EmptyLine);
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [file, xz, img, board, panel, starts] = fields[..] else {
            return Err(ImageListFault::FieldCount(fields.len()));
        };
        let name_ok = !file.starts_with('.')
            && !file
                .chars()
                .any(|c| c == '/' || c == '\\' || c == ':' || c.is_control())
            && file.len() > IMAGE_SUFFIX.len()
            && file.ends_with(IMAGE_SUFFIX)
            && file.contains(version);
        if !name_ok {
            return Err(ImageListFault::Name);
        }
        let xz_sha256 = Sha256Sum::from_hex(xz).ok_or(ImageListFault::Sum)?;
        let img_sha256 = Sha256Sum::from_hex(img).ok_or(ImageListFault::Sum)?;
        let text_ok = |t: &str| !t.is_empty() && !t.chars().any(char::is_control);
        let starts: Vec<String> = starts.split(", ").map(str::to_owned).collect();
        if !text_ok(board) || !text_ok(panel) || !starts.iter().all(|s| text_ok(s)) {
            return Err(ImageListFault::Field);
        }
        Ok(Self {
            file: file.to_owned(),
            xz_sha256,
            img_sha256,
            board: board.to_owned(),
            panel: panel.to_owned(),
            starts,
        })
    }
}

/// Costs a read of the whole source and its SHA-256: the value
/// `sha256sum` gives for the bytes read.
pub fn sha256_of(mut source: impl Read) -> io::Result<Sha256Sum> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = match source.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        hasher.update(&buf[..n]);
    }
    Ok(Sha256Sum(hasher.finalize().into()))
}
