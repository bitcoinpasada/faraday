//! What the hub app writes from: the payload built into it (§1.3), or a
//! developer's own image given with `--image` (§1.1).
//!
//! The payload's files are the release's `.xz` files as they are; each
//! is checked against its manifest line when it is used
//! ([`faraday_payload::write_payload`] refuses one whose sum differs
//! before a byte is written), never at start.

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use faraday_hub::api::{self, DevImage, Reason};
use faraday_payload::{ImageKind, ImageSum, Manifest, PayloadFile};

mod built {
    include!(concat!(env!("OUT_DIR"), "/payload.rs"));
}

/// The payload built into this hub app, when there is one: its manifest
/// and the files it lists.
#[derive(Clone)]
pub struct Payload {
    /// The manifest.
    pub manifest: Manifest,
    files: Vec<(String, Cow<'static, [u8]>)>,
}

impl Payload {
    /// The payload built in, or `None` for a hub app built without one
    /// (or, which the build prevents, with a manifest that does not
    /// parse).
    pub fn built_in() -> Option<Payload> {
        let manifest = Manifest::parse(built::MANIFEST?).ok()?;
        let files = built::FILES
            .iter()
            .map(|(name, bytes)| ((*name).to_string(), Cow::Borrowed(*bytes)))
            .collect();
        Some(Payload { manifest, files })
    }

    /// A payload from a manifest's bytes and the files' bytes, as the
    /// tests make one.
    pub fn from_parts(manifest: &[u8], files: Vec<(String, Vec<u8>)>) -> Result<Payload, String> {
        let manifest = Manifest::parse(manifest).map_err(|e| format!("MANIFEST: {e:?}"))?;
        let files = files.into_iter().map(|(n, b)| (n, Cow::Owned(b))).collect();
        Ok(Payload { manifest, files })
    }

    /// The file named `name` and its manifest line.
    pub fn file(&self, name: &str) -> Option<(&PayloadFile, &[u8])> {
        let line = self.manifest.file(name)?;
        let bytes = self.files.iter().find(|(n, _)| n == name)?.1.as_ref();
        Some((line, bytes))
    }

    /// What the writer takes for the file named `name`, when the
    /// payload has it and its manifest line says `inner_sha256` is what
    /// is inside it.
    pub fn job(&self, name: &str, inner_sha256: &str) -> Option<crate::writer::Job> {
        let line = self.manifest.file(name)?;
        if line.inner_sha256.to_string() != inner_sha256 {
            return None;
        }
        let bytes = self.files.iter().find(|(n, _)| n == name)?.1.clone();
        Some(crate::writer::Job::Payload {
            file: line.clone(),
            bytes,
        })
    }

    /// The desktop app's manifest line, when the payload carries it.
    pub fn desktop_line(&self) -> Option<&PayloadFile> {
        self.manifest.files().iter().find(|f| {
            matches!(
                api::PayloadKind::of(&f.name),
                Some(api::PayloadKind::Desktop)
            )
        })
    }

    /// The desktop app, decompressed. [`faraday_payload::write_payload`]
    /// checks the `.xz` against the manifest before it decodes and the
    /// decoded bytes against its size and SHA-256 as it goes, so what
    /// comes back is what the manifest names.
    pub fn desktop_app(&self) -> Result<(PayloadFile, Vec<u8>), Reason> {
        let line = self.desktop_line().ok_or(Reason::NoDesktopApp)?;
        let bytes = self
            .files
            .iter()
            .find(|(n, _)| *n == line.name)
            .ok_or(Reason::NoDesktopApp)?
            .1
            .as_ref();
        let mut out = Vec::with_capacity(line.inner_size as usize);
        faraday_payload::write_payload(Cursor::new(bytes), line, &mut out, |_, _| {})
            .map_err(crate::writer::payload_reason)?;
        Ok((line.clone(), out))
    }

    /// The manifest as the hub shows it (Confirm, About).
    pub fn event(&self) -> api::Payload {
        api::Payload {
            version: self.manifest.version().to_string(),
            commit: self.manifest.commit().to_string(),
            files: self
                .manifest
                .files()
                .iter()
                .map(|f| api::PayloadFile {
                    name: f.name.clone(),
                    sha256: f.sha256.to_string(),
                    inner: f.inner_name.clone(),
                    inner_sha256: f.inner_sha256.to_string(),
                    inner_size: f.inner_size,
                    build_id: f.build_id.clone(),
                })
                .collect(),
        }
    }
}

/// A developer's image (`--image`), as hashed when the hub app started:
/// what Confirm shows and what the writer holds the file to.
#[derive(Debug, Clone)]
pub struct Image {
    /// The file.
    pub path: PathBuf,
    /// Plain or `.xz`.
    pub kind: ImageKind,
    /// The SHA-256 and size of what is written.
    pub sum: ImageSum,
    /// It is a boot partition alone (`-boot.vfat`).
    pub boot_only: bool,
    /// The build ID found in it.
    pub build_id: Option<String>,
}

impl Image {
    /// Reads `path` once, decompressing a `.xz`, for its SHA-256 and
    /// size: the value `sha256sum` gives for the image itself.
    pub fn hash(path: &Path) -> Result<Image, String> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let kind = ImageKind::of_name(&name);
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let sum = faraday_payload::hash_image(BufReader::with_capacity(1 << 20, file), kind)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let boot_only = name.ends_with("-boot.vfat");
        Ok(Image {
            path: path.to_path_buf(),
            kind,
            sum,
            boot_only,
            build_id: build_id(path, kind),
        })
    }

    /// What the hub is told.
    pub fn event(&self) -> DevImage {
        DevImage {
            path: self.path.display().to_string(),
            sha256: self.sum.sha256.to_string(),
            size: self.sum.size,
            boot_only: self.boot_only,
            build_id: self.build_id.clone(),
        }
    }
}

/// The build ID in a developer's image: the release string a plain
/// image or boot partition carries, read as the stick's upgrade reads a
/// stick's. A `.xz` is not decompressed a second time for it; the
/// Confirm page shows its SHA-256 instead.
fn build_id(path: &Path, kind: ImageKind) -> Option<String> {
    if kind != ImageKind::Plain {
        return None;
    }
    let release = crate::stick::release_in_file(path)?;
    faraday_core::upgrade::build_id(&release).map(str::to_string)
}
