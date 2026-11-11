//! Finding the release's images (`docs/IMAGES.md` §5): the Downloads
//! folder Transfer uses, and the files dropped on the window, looked at
//! for names on the release's image list ([`faraday_payload::ImageList`]).
//!
//! A file is opened and hashed only when its name is on the list; every
//! other file in the folder is left as it is. A file is hashed once for
//! its path, size and modification time ([`Memo`]); one that is replaced
//! is hashed again.
//!
//! What this finds is an offer. The sum it carries is the list's, and the
//! writer checks the file against it again as it writes, so a file
//! changed after it was found is refused there.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use faraday_crypto::work::{Compute, Work};
use faraday_payload::{ImageList, NameClass, Sha256Sum, sha256_of};
use faraday_starter::api::{FoundImage, ImageFinding};

use crate::writer::Job;

mod built {
    include!(concat!(env!("OUT_DIR"), "/images.rs"));
}

/// Costs a parse of the release's image list: the one built into this
/// Starter, or `None` for a Starter built without one (or, which the
/// build prevents, with a list that does not parse). The list is not
/// read from Downloads or from beside a file: it is what decides which
/// image may be written to a stick.
pub fn built_in_list() -> Option<ImageList> {
    ImageList::parse(built::IMAGES?).ok()
}

/// The file names of the release's images, for the hardware whose image
/// was not found: `list`'s own, else the names a release of this
/// Starter's version publishes (`faraday/builds.toml`'s `release` entries
/// of the images).
pub fn image_names(list: Option<&ImageList>) -> Vec<String> {
    if let Some(list) = list {
        return list.images().iter().map(|i| i.file.clone()).collect();
    }
    let built = faraday_core::BUILD.unwrap_or(env!("CARGO_PKG_VERSION"));
    let v = built.split('+').next().unwrap_or(built);
    vec![
        format!("faraday-stick-{v}-x86_64-uefi.img.xz"),
        format!("faraday-pi-{v}.img.xz"),
    ]
}

/// What the writer takes for the release image at `path`, when the list
/// built in has a line with that file name and these two sums: the size,
/// build and boot partition's sum are the line's, never the order's. A
/// lookup.
pub fn found_job(path: &str, xz_sha256: &str, img_sha256: &str) -> Option<Job> {
    found_job_in(built_in_list().as_ref()?, path, xz_sha256, img_sha256)
}

/// [`found_job`] against `list`.
pub fn found_job_in(
    list: &ImageList,
    path: &str,
    xz_sha256: &str,
    img_sha256: &str,
) -> Option<Job> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let image = list.images().iter().find(|i| {
        i.file == name
            && i.xz_sha256.to_string() == xz_sha256
            && i.img_sha256.to_string() == img_sha256
    })?;
    Some(Job::Found {
        file: image.payload_file(),
        path: PathBuf::from(path),
    })
}

/// What a file was hashed as: its size and modification time, and the
/// sum.
type Stamp = (u64, SystemTime, Sha256Sum);

/// The sums worked out so far, by path.
#[derive(Default)]
pub struct Memo(HashMap<PathBuf, Stamp>);

/// The image list this Starter was built with, and the sums it has
/// worked out.
pub struct Finder {
    list: Option<ImageList>,
    memo: Mutex<Memo>,
}

impl Finder {
    /// A finder for `list`; with none, nothing is found.
    pub fn new(list: Option<ImageList>) -> Self {
        Self {
            list,
            memo: Mutex::new(Memo::default()),
        }
    }

    /// What is found in `folder` and among the `dropped` files, for the
    /// shell's thread of its own: a token cannot cross threads, so this
    /// one mints its own.
    pub fn answer(&self, folder: &Path, dropped: &[PathBuf]) -> Vec<ImageFinding> {
        let Some(list) = &self.list else {
            return Vec::new();
        };
        #[expect(
            clippy::disallowed_methods,
            reason = "a mint: the finder's thread draws nothing (docs/COMPUTE.md §3)"
        )]
        let c = Compute::new(|_| {});
        let mut memo = self
            .memo
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        find(&c, list, folder, dropped, &mut memo)
    }
}

/// Costs a read of every file in `folder` and among `dropped` whose name
/// is on `list`, and its SHA-256, unless `memo` holds it for the file's
/// size and modification time: the images found, those whose sum is not
/// the list's, and those of another release. Other files are not opened.
/// Folders in `folder` are not entered.
pub fn find(
    c: &Compute,
    list: &ImageList,
    folder: &Path,
    dropped: &[PathBuf],
    memo: &mut Memo,
) -> Vec<ImageFinding> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|entries| entries.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    paths.sort();
    for d in dropped {
        if !paths.contains(d) {
            paths.push(d.clone());
        }
    }
    paths
        .iter()
        .filter_map(|path| one(c, list, path, memo))
        .collect()
}

/// Costs a read of the file at `path` when its name is on `list`: its
/// finding, or `None` when the name is not the list's or it is not a plain
/// file.
fn one(c: &Compute, list: &ImageList, path: &Path, memo: &mut Memo) -> Option<ImageFinding> {
    let name = path.file_name()?.to_str()?;
    let class = list.classify(name);
    if class == NameClass::Unrelated {
        return None;
    }
    let meta = std::fs::metadata(path)
        .ok()
        .filter(std::fs::Metadata::is_file)?;
    let shown = path.display().to_string();
    let name = name.to_owned();
    match class {
        NameClass::Unrelated => None,
        NameClass::OtherRelease { .. } => Some(ImageFinding::OtherRelease {
            path: shown,
            name,
            needs: list.version().to_owned(),
        }),
        NameClass::Listed(image) => {
            let sum = hashed(c, path, &meta, memo)?;
            Some(if sum == image.xz_sha256 {
                ImageFinding::Found(FoundImage {
                    path: shown,
                    name,
                    xz_sha256: image.xz_sha256.to_string(),
                    img_sha256: image.img_sha256.to_string(),
                    inner_size: image.inner_size,
                    build_id: image.build_id.clone(),
                    board: image.board.clone(),
                    panel: image.panel.clone(),
                    starts: image.starts.clone(),
                    boot_sha256: image.boot_sha256.map(|s| s.to_string()),
                })
            } else {
                ImageFinding::Mismatch { path: shown, name }
            })
        }
    }
}

/// Costs a read of the file at `path` and its SHA-256, unless `memo` has
/// it for this size and modification time.
fn hashed(
    c: &Compute,
    path: &Path,
    meta: &std::fs::Metadata,
    memo: &mut Memo,
) -> Option<Sha256Sum> {
    let (size, when) = (meta.len(), meta.modified().ok()?);
    if let Some((s, w, sum)) = memo.0.get(path)
        && (*s, *w) == (size, when)
    {
        return Some(*sum);
    }
    c.count(Work::FileHash);
    let file = File::open(path).ok()?;
    let sum = sha256_of(BufReader::new(file)).ok()?;
    memo.0.insert(path.to_path_buf(), (size, when, sum));
    Some(sum)
}
