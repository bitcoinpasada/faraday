//! `MANIFEST` (`docs/HUB.md` §1.3, "The manifest and where it is
//! built"): a first line `# Faraday hub payload VERSION, commit <full
//! hash>`, then one line per payload file, six fields separated by one
//! tab each: the payload file's name, its SHA-256, the name of what is
//! inside it, that file's SHA-256, that file's size in bytes, and the
//! build ID it carries.

use core::fmt;

use crate::Sha256Sum;

const HEADER_PREFIX: &str = "# Faraday hub payload ";
const HEADER_COMMIT: &str = ", commit ";

/// A parsed `MANIFEST`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    version: String,
    commit: String,
    files: Vec<PayloadFile>,
}

/// One payload file's line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadFile {
    /// The payload file's name, ending in `.xz`.
    pub name: String,
    /// The SHA-256 of the payload file, the `.xz` itself.
    pub sha256: Sha256Sum,
    /// The name of the image or binary inside it.
    pub inner_name: String,
    /// The SHA-256 of the image or binary inside it.
    pub inner_sha256: Sha256Sum,
    /// The size of the image or binary inside it, in bytes.
    pub inner_size: u64,
    /// The build ID the image or binary carries.
    pub build_id: String,
}

/// Why a `MANIFEST` was refused, and on which line (1 is the first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestError {
    /// The line, counted from 1.
    pub line: usize,
    /// What is wrong with it.
    pub fault: ManifestFault,
}

/// What is wrong with a `MANIFEST` line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestFault {
    /// The file is not UTF-8 text.
    NotText,
    /// The first line is not `# Faraday hub payload VERSION, commit HASH`.
    Header,
    /// An empty line where a file's line belongs.
    EmptyLine,
    /// A file's line does not have exactly six tab-separated fields.
    FieldCount(usize),
    /// A file name is empty, holds a path separator or a control
    /// character, or starts with a dot; or a payload name does not end
    /// in `.xz`.
    Name,
    /// A sum is not 64 lowercase hexadecimal digits.
    Sum,
    /// The size is not a decimal number of bytes above zero.
    Size,
    /// The build ID is empty or holds a control character.
    BuildId,
    /// A payload file is named twice.
    Duplicate,
    /// The manifest names no files.
    NoFiles,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.fault {
            ManifestFault::NotText => "not UTF-8 text",
            ManifestFault::Header => "not a payload manifest header",
            ManifestFault::EmptyLine => "empty line",
            ManifestFault::FieldCount(_) => "not six tab-separated fields",
            ManifestFault::Name => "bad file name",
            ManifestFault::Sum => "bad SHA-256",
            ManifestFault::Size => "bad size",
            ManifestFault::BuildId => "bad build ID",
            ManifestFault::Duplicate => "file named twice",
            ManifestFault::NoFiles => "no files",
        };
        write!(f, "MANIFEST line {}: {what}", self.line)
    }
}

impl std::error::Error for ManifestError {}

impl Manifest {
    /// Parses `MANIFEST`'s bytes. A final newline is allowed and not
    /// required; any other empty line is refused.
    pub fn parse(bytes: &[u8]) -> Result<Self, ManifestError> {
        let text = core::str::from_utf8(bytes).map_err(|_| ManifestError {
            line: 1,
            fault: ManifestFault::NotText,
        })?;
        let text = text.strip_suffix('\n').unwrap_or(text);
        let mut lines = text.split('\n');

        let header = lines.next().unwrap_or("");
        let (version, commit) = parse_header(header).ok_or(ManifestError {
            line: 1,
            fault: ManifestFault::Header,
        })?;

        let mut files: Vec<PayloadFile> = Vec::new();
        for (i, line) in lines.enumerate() {
            let number = i + 2;
            let file = PayloadFile::parse_line(line).map_err(|fault| ManifestError {
                line: number,
                fault,
            })?;
            if files.iter().any(|f| f.name == file.name) {
                return Err(ManifestError {
                    line: number,
                    fault: ManifestFault::Duplicate,
                });
            }
            files.push(file);
        }
        if files.is_empty() {
            return Err(ManifestError {
                line: 2,
                fault: ManifestFault::NoFiles,
            });
        }
        Ok(Self {
            version: version.to_owned(),
            commit: commit.to_owned(),
            files,
        })
    }

    /// The release version the payload was staged for.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The full commit hash the payload was built from.
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// Every payload file, in the manifest's order.
    pub fn files(&self) -> &[PayloadFile] {
        &self.files
    }

    /// The line for the payload file named `name`.
    pub fn file(&self, name: &str) -> Option<&PayloadFile> {
        self.files.iter().find(|f| f.name == name)
    }
}

fn parse_header(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix(HEADER_PREFIX)?;
    let (version, commit) = rest.split_once(HEADER_COMMIT)?;
    let version_ok = !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'));
    let commit_ok = matches!(commit.len(), 40 | 64)
        && commit
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
    (version_ok && commit_ok).then_some((version, commit))
}

impl PayloadFile {
    /// Parses one file's line (without its newline).
    pub fn parse_line(line: &str) -> Result<Self, ManifestFault> {
        if line.is_empty() {
            return Err(ManifestFault::EmptyLine);
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [name, sha256, inner_name, inner_sha256, size, build_id] = fields[..] else {
            return Err(ManifestFault::FieldCount(fields.len()));
        };
        if !name_ok(name) || name.len() <= ".xz".len() || !name.ends_with(".xz") {
            return Err(ManifestFault::Name);
        }
        if !name_ok(inner_name) {
            return Err(ManifestFault::Name);
        }
        let sha256 = Sha256Sum::from_hex(sha256).ok_or(ManifestFault::Sum)?;
        let inner_sha256 = Sha256Sum::from_hex(inner_sha256).ok_or(ManifestFault::Sum)?;
        let inner_size = parse_size(size).ok_or(ManifestFault::Size)?;
        if build_id.is_empty() || build_id.chars().any(char::is_control) {
            return Err(ManifestFault::BuildId);
        }
        Ok(Self {
            name: name.to_owned(),
            sha256,
            inner_name: inner_name.to_owned(),
            inner_sha256,
            inner_size,
            build_id: build_id.to_owned(),
        })
    }
}

/// A plain file name: something the hub app can name a file by without
/// it reaching another folder.
fn name_ok(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name
            .chars()
            .any(|c| c == '/' || c == '\\' || c == ':' || c.is_control())
}

/// Decimal digits, no sign, no leading zero, above zero.
fn parse_size(text: &str) -> Option<u64> {
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
