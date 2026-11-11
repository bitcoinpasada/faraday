//! The files the app asks for: a path read, a dropped file read, and a
//! saved file written into the Downloads folder without replacing one.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The most a file read for the app may hold. A code is a few hundred
/// bytes and its picture a few thousand; anything near this is not one.
pub const MOST: u64 = 16 << 20;

/// The file at `path`, or nothing when it cannot be read or is larger
/// than [`MOST`].
pub fn read(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(MOST + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= MOST).then_some(bytes)
}

/// Where a saved file goes: the Downloads folder (the system's own
/// choice on Linux, `user-dirs.dirs`), the home folder when there is
/// none.
pub fn downloads() -> Option<PathBuf> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)?;
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|c| !c.is_empty())
        .map_or_else(|| home.join(".config"), PathBuf::from);
    let listed = std::fs::read_to_string(config.join("user-dirs.dirs")).ok();
    Some(choose(&home, listed.as_deref(), |p| p.is_dir()))
}

/// The Downloads folder given the home folder, the text of
/// `user-dirs.dirs` if there is one, and whether a folder exists.
fn choose(home: &Path, user_dirs: Option<&str>, is_dir: impl Fn(&Path) -> bool) -> PathBuf {
    if let Some(dir) = user_dirs.and_then(|text| user_dir(text, home))
        && is_dir(&dir)
    {
        return dir;
    }
    let plain = home.join("Downloads");
    if is_dir(&plain) {
        plain
    } else {
        home.to_path_buf()
    }
}

/// The `XDG_DOWNLOAD_DIR` line of a `user-dirs.dirs` file, with `$HOME`
/// filled in. A line that is not inside the home folder or is not
/// absolute is not taken.
fn user_dir(text: &str, home: &Path) -> Option<PathBuf> {
    let line = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("XDG_DOWNLOAD_DIR="))?;
    let value = line.trim().trim_matches('"');
    let path = match value.strip_prefix("$HOME") {
        Some("") => home.to_path_buf(),
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(value),
    };
    path.is_absolute().then_some(path)
}

/// Writes `bytes` into `dir` under the plain file name in `hint`, or
/// under `name (2).ext`, `name (3).ext` and so on when a file by that name
/// is there: nothing is replaced. The path it wrote.
pub fn save(dir: &Path, hint: &str, bytes: &[u8]) -> Option<PathBuf> {
    let name = Path::new(hint).file_name()?.to_str()?;
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => name.split_at(i),
        _ => (name, ""),
    };
    for n in 1..1000 {
        let candidate = if n == 1 {
            name.to_owned()
        } else {
            format!("{stem} ({n}){ext}")
        };
        let path = dir.join(candidate);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                f.write_all(bytes).and_then(|()| f.sync_all()).ok()?;
                return Some(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aqrv-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        dir
    }

    #[test]
    fn a_second_save_under_one_name_keeps_the_first() {
        let dir = folder("twice");
        let a = save(&dir, "argon-qr-vault.png", b"one").expect("saved");
        let b = save(&dir, "argon-qr-vault.png", b"two").expect("saved");
        let c = save(&dir, "argon-qr-vault.png", b"three").expect("saved");
        assert_eq!(
            a.file_name().and_then(|n| n.to_str()),
            Some("argon-qr-vault.png")
        );
        assert_eq!(
            b.file_name().and_then(|n| n.to_str()),
            Some("argon-qr-vault (2).png")
        );
        assert_eq!(
            c.file_name().and_then(|n| n.to_str()),
            Some("argon-qr-vault (3).png")
        );
        assert_eq!(std::fs::read(a).expect("read"), b"one");
        assert_eq!(std::fs::read(b).expect("read"), b"two");
    }

    #[test]
    fn a_name_with_a_folder_in_it_is_saved_by_its_file_name_alone() {
        let dir = folder("path");
        let p = save(&dir, "../../elsewhere.aqrv", b"x").expect("saved");
        assert_eq!(p, dir.join("elsewhere.aqrv"));
    }

    #[test]
    fn the_downloads_folder_is_the_one_the_system_names_or_the_home_folder() {
        let home = Path::new("/h");
        let listed = "XDG_DOWNLOAD_DIR=\"$HOME/Fetched\"\n";
        let all = |_: &Path| true;
        assert_eq!(choose(home, Some(listed), all), Path::new("/h/Fetched"));
        assert_eq!(choose(home, None, all), Path::new("/h/Downloads"));
        assert_eq!(choose(home, Some(listed), |_| false), home);
    }

    #[test]
    fn a_file_over_the_limit_is_not_read() {
        let dir = folder("big");
        let path = dir.join("big");
        std::fs::File::create(&path)
            .and_then(|f| f.set_len(MOST + 1))
            .expect("a big file");
        assert!(read(&path).is_none());
        assert_eq!(read(&dir.join("missing")), None);
    }
}
