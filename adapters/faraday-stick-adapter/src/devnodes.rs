//! The boot report's device-nodes block (`--boot-report`): every entry
//! of `/dev/dri` with its kind, its mode as `ls -l` prints it, its owner
//! and group as numbers and, for a device, its major and minor, so a dev
//! card shows without a serial console that the screen's nodes belong to
//! root alone (mode `crw-------`, owner 0, group 0).

use std::fs::Metadata;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;

/// The most entries listed: `/dev/dri` holds a few nodes.
const MAX_ENTRIES: usize = 256;

/// The block for the machine whose files are under `root`. Costs a read
/// of `/dev/dri` and one `lstat` per entry.
pub fn block(root: &Path) -> String {
    let mut out = String::from("\n== device nodes ==\n");
    let dir = root.join("dev/dri");
    let Ok(read) = std::fs::read_dir(&dir) else {
        out.push_str("/dev/dri absent\n");
        return out;
    };
    let mut names: Vec<String> = read
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    if names.is_empty() {
        out.push_str("/dev/dri empty\n");
    }
    for name in names.iter().take(MAX_ENTRIES) {
        match std::fs::symlink_metadata(dir.join(name)) {
            Ok(m) => out.push_str(&line(name, &m)),
            Err(_) => out.push_str(&format!("/dev/dri/{name} unreadable\n")),
        }
    }
    out
}

/// One entry: `/dev/dri/card0 char crw------- uid 0 gid 0 226:0`.
fn line(name: &str, m: &Metadata) -> String {
    let t = m.file_type();
    let (kind, c) = if t.is_char_device() {
        ("char", 'c')
    } else if t.is_block_device() {
        ("block", 'b')
    } else if t.is_dir() {
        ("other", 'd')
    } else if t.is_symlink() {
        ("other", 'l')
    } else if t.is_file() {
        ("other", '-')
    } else {
        ("other", '?')
    };
    let mut s = format!(
        "/dev/dri/{name} {kind} {c}{} uid {} gid {}",
        mode(m.mode()),
        m.uid(),
        m.gid()
    );
    if t.is_char_device() || t.is_block_device() {
        let (major, minor) = split(m.rdev());
        s.push_str(&format!(" {major}:{minor}"));
    }
    s.push('\n');
    s
}

/// The nine permission characters of `mode`, with `s`, `S`, `t` and `T`
/// for setuid, setgid and sticky as `ls -l` prints them.
fn mode(m: u32) -> String {
    let mut s = String::new();
    for (i, who) in [(6, 0o4000), (3, 0o2000), (0, 0o1000)] {
        let bits = (m >> i) & 7;
        let special = m & who != 0;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(match (bits & 1 != 0, special, i) {
            (true, true, 0) => 't',
            (false, true, 0) => 'T',
            (true, true, _) => 's',
            (false, true, _) => 'S',
            (true, false, _) => 'x',
            (false, false, _) => '-',
        });
    }
    s
}

/// A device number's major and minor (Linux's `major()` and `minor()`).
fn split(dev: u64) -> (u64, u64) {
    let major = ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0xfff);
    let minor = ((dev >> 12) & 0xffff_ff00) | (dev & 0xff);
    (major, minor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn tree(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("faraday-devnodes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn a_machine_with_no_dri_folder_says_absent() {
        let root = tree("absent");
        assert_eq!(block(&root), "\n== device nodes ==\n/dev/dri absent\n");
    }

    #[test]
    fn an_empty_dri_folder_says_empty() {
        let root = tree("empty");
        std::fs::create_dir_all(root.join("dev/dri")).unwrap();
        assert_eq!(block(&root), "\n== device nodes ==\n/dev/dri empty\n");
    }

    #[test]
    fn each_entry_shows_its_mode_owner_and_group() {
        let root = tree("entries");
        let dri = root.join("dev/dri");
        std::fs::create_dir_all(dri.join("by-path")).unwrap();
        std::fs::write(dri.join("card0"), b"").unwrap();
        std::fs::set_permissions(dri.join("card0"), std::fs::Permissions::from_mode(0o600))
            .unwrap();
        std::fs::set_permissions(dri.join("by-path"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        let (uid, gid) = {
            let m = std::fs::metadata(dri.join("card0")).unwrap();
            (m.uid(), m.gid())
        };
        assert_eq!(
            block(&root),
            format!(
                "\n== device nodes ==\n\
                 /dev/dri/by-path other drwxr-xr-x uid {uid} gid {gid}\n\
                 /dev/dri/card0 other -rw------- uid {uid} gid {gid}\n"
            )
        );
    }

    #[test]
    fn a_device_number_splits_into_major_and_minor() {
        // card0 on a Pi is 226:0, renderD128 is 226:128.
        assert_eq!(split(0xe200), (226, 0));
        assert_eq!(split(0xe280), (226, 128));
    }
}
