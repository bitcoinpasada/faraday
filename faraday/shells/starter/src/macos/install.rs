//! Install and uninstall on a Mac (`docs/STARTER.md` §5.2, §5.4,
//! `docs/MAC.md` §6), per user and never as an administrator, with no
//! macOS call in it: the bundle is copied by the function the caller
//! hands in (`/usr/bin/ditto` on a Mac, which keeps the bundle's
//! signature; a plain copy in the tests), so the tests install into a
//! temporary home.
//!
//! | | |
//! |---|---|
//! | the Starter | `~/Applications/Faraday Starter.app`, a copy of the running bundle |
//! | the desktop app's cache | `~/Library/Caches/faraday/X.Y.Z/` |
//!
//! A bundle is the Starter's when its `Contents/Info.plist` names
//! [`IDENTIFIER`]; a bundle of the same name that does not (or whose
//! `Info.plist` cannot be read) is another program's, and is neither
//! replaced nor removed. The desktop app has no Mac build until batch
//! D, so it is neither installed nor run here.

use std::fs;
use std::path::{Path, PathBuf};

use faraday_core::names;
use faraday_starter::api::{InstallItem, Installed, Reason};

use super::plist;

/// The installed bundle's name, in `~/Applications`.
pub const BUNDLE: &str = "Faraday Starter.app";
/// The Starter's `CFBundleIdentifier`: its menu identity on Linux and
/// Windows too.
pub const IDENTIFIER: &str = names::MENU_ID;
/// Beside the bundle while it is replaced.
const NEW: &str = ".Faraday Starter.app.new";
const OLD: &str = ".Faraday Starter.app.old";

/// The folders an install works in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// The home folder.
    pub home: PathBuf,
}

impl Dirs {
    /// From `HOME`.
    pub fn from_env() -> Result<Dirs, Reason> {
        match std::env::var_os("HOME") {
            Some(h) if !h.is_empty() => Ok(Dirs {
                home: PathBuf::from(h),
            }),
            _ => Err(Reason::NoHome),
        }
    }

    /// `~/Applications`.
    pub fn applications(&self) -> PathBuf {
        self.home.join("Applications")
    }

    /// `~/Applications/Faraday Starter.app`.
    pub fn bundle(&self) -> PathBuf {
        self.applications().join(BUNDLE)
    }

    /// `~/Library/Caches/faraday`, the desktop app's cache (§5.3).
    pub fn cache(&self) -> PathBuf {
        self.home.join("Library").join("Caches").join("faraday")
    }

    /// `~/Downloads`.
    pub fn downloads(&self) -> PathBuf {
        self.home.join("Downloads")
    }

    /// A path as a person reads it, `~/` for the home folder.
    pub fn shown(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.display().to_string(),
        }
    }

    fn file_error(&self, path: &Path, e: &std::io::Error) -> Reason {
        Reason::File {
            path: self.shown(path),
            detail: e.to_string(),
        }
    }
}

/// What a bundle folder is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bundle {
    /// Nothing is there.
    Absent,
    /// The Starter, at this `CFBundleShortVersionString`.
    Ours(String),
    /// Something else.
    Other,
}

/// What is at `path`, read from its `Contents/Info.plist`.
pub fn bundle_at(path: &Path) -> Bundle {
    if fs::symlink_metadata(path).is_err() {
        return Bundle::Absent;
    }
    if !path.is_dir() || path.is_symlink() {
        return Bundle::Other;
    }
    let Ok(text) = fs::read_to_string(path.join("Contents").join("Info.plist")) else {
        return Bundle::Other;
    };
    let Ok(info) = plist::parse(&text) else {
        return Bundle::Other;
    };
    if info
        .get("CFBundleIdentifier")
        .and_then(plist::Value::as_str)
        != Some(IDENTIFIER)
    {
        return Bundle::Other;
    }
    let version = info
        .get("CFBundleShortVersionString")
        .and_then(plist::Value::as_str)
        .unwrap_or("")
        .to_string();
    Bundle::Ours(version)
}

/// The bundle the program at `exe` runs from: `X.app` for
/// `X.app/Contents/MacOS/<program>`; `None` for a program not in one
/// (a developer's `cargo run`).
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    let named = |p: &Path, n: &str| p.file_name().is_some_and(|f| f == n);
    (named(macos, "MacOS")
        && named(contents, "Contents")
        && app.extension().is_some_and(|e| e == "app"))
    .then(|| app.to_path_buf())
}

/// What is installed: `None` when no Starter bundle is.
pub fn installed(dirs: &Dirs) -> Option<Installed> {
    let bundle = dirs.bundle();
    match bundle_at(&bundle) {
        Bundle::Ours(version) => Some(Installed {
            version,
            items: vec![(InstallItem::App, dirs.shown(&bundle))],
        }),
        Bundle::Absent | Bundle::Other => None,
    }
}

/// Removes `path` when it is a folder of the Starter's, or nothing is
/// there.
fn remove_ours(dirs: &Dirs, path: &Path) -> Result<(), Reason> {
    match bundle_at(path) {
        Bundle::Ours(_) => fs::remove_dir_all(path).map_err(|e| dirs.file_error(path, &e)),
        Bundle::Absent | Bundle::Other => Ok(()),
    }
}

/// Installs the bundle the program at `running` runs from as
/// `~/Applications/Faraday Starter.app`, `copy` copying a bundle folder
/// to a place that does not exist yet. A Starter there is replaced
/// whole; another program's bundle of that name stays and Install says
/// it is in the way. Where the Starter went.
pub fn install(
    dirs: &Dirs,
    running: &Path,
    copy: &mut dyn FnMut(&Path, &Path) -> Result<(), String>,
) -> Result<String, Reason> {
    let from = bundle_of(running).ok_or(Reason::Unsupported)?;
    let target = dirs.bundle();
    let shown = dirs.shown(&target);
    if from == target {
        return Ok(shown);
    }
    if bundle_at(&from) == Bundle::Other {
        return Err(Reason::Unsupported);
    }
    if bundle_at(&target) == Bundle::Other {
        return Err(Reason::InTheWay { path: shown });
    }
    let apps = dirs.applications();
    fs::create_dir_all(&apps).map_err(|e| dirs.file_error(&apps, &e))?;
    let (new, old) = (apps.join(NEW), apps.join(OLD));
    for leftover in [&new, &old] {
        if bundle_at(leftover) == Bundle::Other {
            return Err(Reason::InTheWay {
                path: dirs.shown(leftover),
            });
        }
        remove_ours(dirs, leftover)?;
    }
    copy(&from, &new).map_err(|detail| Reason::File {
        path: dirs.shown(&new),
        detail,
    })?;
    if !matches!(bundle_at(&new), Bundle::Ours(_)) {
        remove_ours(dirs, &new)?;
        return Err(Reason::InTheWay {
            path: dirs.shown(&new),
        });
    }
    let replacing = target.exists();
    if replacing {
        fs::rename(&target, &old).map_err(|e| dirs.file_error(&target, &e))?;
    }
    if let Err(e) = fs::rename(&new, &target) {
        if replacing {
            let _ = fs::rename(&old, &target);
        }
        return Err(dirs.file_error(&target, &e));
    }
    if replacing {
        remove_ours(dirs, &old)?;
    }
    Ok(shown)
}

/// Uninstalls: the Starter's bundle, when it is the Starter's, and the
/// desktop app's cache, and nothing else. What is already gone is not
/// an error.
pub fn uninstall(dirs: &Dirs) -> Result<(), Reason> {
    let apps = dirs.applications();
    for path in [dirs.bundle(), apps.join(NEW), apps.join(OLD)] {
        remove_ours(dirs, &path)?;
    }
    clear_cache(dirs)
}

/// Removes the `X.Y.Z/` folders from the cache folder, then the cache
/// folder if nothing else is in it.
fn clear_cache(dirs: &Dirs) -> Result<(), Reason> {
    let root = dirs.cache();
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let release = !name.is_empty() && name.split('.').all(|p| p.parse::<u32>().is_ok());
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if release && is_dir {
            fs::remove_dir_all(entry.path()).map_err(|e| dirs.file_error(&entry.path(), &e))?;
        }
    }
    let _ = fs::remove_dir(&root);
    Ok(())
}
