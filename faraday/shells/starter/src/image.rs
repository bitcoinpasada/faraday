//! A developer's own image (`--image`, `docs/STARTER.md` §1.1), as the
//! Starter hashes it at start.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use faraday_payload::{ImageKind, ImageSum};
use faraday_starter::api::DevImage;

/// A developer's image (`--image`), as hashed when the Starter started:
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
    /// Costs a read of the whole file, decompressing a `.xz`, and its
    /// SHA-256: the value `sha256sum` gives for the image itself, and its
    /// size. Done once, at start, before the window opens.
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

    /// What the Starter is told.
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
    faraday_core::build::build_id(&release).map(str::to_string)
}
