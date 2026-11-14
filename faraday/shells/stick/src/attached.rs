//! What the kernel shows of attached storage, read from `/sys` and not
//! from the disk process: a disk process that is slow, never answers or
//! leaves a disk out of its list (no partitions, a boot-named partition)
//! does not make the stick rule fail open (`docs/SECURITY-MODEL.md`).
//!
//! Each look reads a few small files under `/sys/class/block` and
//! `/sys/bus/usb/devices`, which any user can read, and touches no stick.

use std::fs;
use std::path::{Path, PathBuf};

/// The storage attached under `sys_block` (`/sys/class/block`) and
/// `usb_root` (`/sys/bus/usb/devices`), each by name, sorted: a disk or
/// partition of size above 0 on a USB bus or named `mmcblk<n>` or
/// `mmcblk<n>p<m>`, and an authorised mass-storage interface of a USB
/// device. The disk `boot_partition` is on is left out, with the USB port
/// it is behind: the medium the machine started from is not a stick that
/// was attached. An empty card-reader slot (size 0) is not storage.
pub fn attached(sys_block: &Path, usb_root: &Path, boot_partition: Option<&str>) -> Vec<String> {
    let boot_disk = boot_partition.and_then(|p| disk_of(sys_block, p));
    let boot_port = boot_partition.and_then(|p| crate::start_window::medium_port(sys_block, p));
    let mut out = Vec::new();
    for name in names(sys_block) {
        if !is_storage_name(&name) || size(&sys_block.join(&name)) == 0 {
            continue;
        }
        let Ok(real) = fs::canonicalize(sys_block.join(&name)) else {
            continue;
        };
        if boot_disk
            .as_ref()
            .is_some_and(|b| real == *b || real.parent() == Some(b.as_path()))
        {
            continue;
        }
        if name.starts_with("mmcblk") || on_usb(&real) {
            out.push(name);
        }
    }
    for name in names(usb_root) {
        let Some((port, _)) = name.split_once(':') else {
            continue;
        };
        if boot_port.as_deref() == Some(port) {
            continue;
        }
        let dir = usb_root.join(&name);
        let class = fs::read_to_string(dir.join("bInterfaceClass")).unwrap_or_default();
        if u8::from_str_radix(class.trim(), 16) == Ok(8)
            && authorised(&dir)
            && authorised(&usb_root.join(port))
        {
            out.push(name);
        }
    }
    out.sort();
    out
}

fn names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .map(|e| {
            e.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// `mmcblk0`, `mmcblk0p1`, `sda`, `sdb1`: not the card's `boot0` or `rpmb`
/// areas, loop, RAM or mapped devices.
fn is_storage_name(name: &str) -> bool {
    if let Some(rest) = name.strip_prefix("mmcblk") {
        let (disk, part) = rest.split_once('p').unwrap_or((rest, "0"));
        return !disk.is_empty()
            && disk.bytes().all(|b| b.is_ascii_digit())
            && part.bytes().all(|b| b.is_ascii_digit());
    }
    name.strip_prefix("sd").is_some_and(|rest| {
        let letters = rest.bytes().take_while(u8::is_ascii_lowercase).count();
        letters > 0 && rest[letters..].bytes().all(|b| b.is_ascii_digit())
    })
}

/// The size in 512-byte sectors, 0 when it cannot be read.
fn size(dir: &Path) -> u64 {
    fs::read_to_string(dir.join("size"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Whether the device's real path runs through a USB bus (`usb1`).
fn on_usb(real: &Path) -> bool {
    real.components().any(|c| {
        c.as_os_str()
            .to_str()
            .and_then(|c| c.strip_prefix("usb"))
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// The real path of the disk a partition (or a disk) is.
fn disk_of(sys_block: &Path, name: &str) -> Option<PathBuf> {
    let real = fs::canonicalize(sys_block.join(name)).ok()?;
    if real.join("partition").exists() {
        real.parent().map(Path::to_path_buf)
    } else {
        Some(real)
    }
}

/// Whether the device or interface is let through (`authorized` is not 0,
/// or not there).
fn authorised(dir: &Path) -> bool {
    fs::read_to_string(dir.join("authorized")).map_or(true, |s| s.trim() != "0")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn put(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// A fake `/sys`: a disk under `devices/<place>` linked from
    /// `class/block`; a partition is a directory inside its disk.
    fn disk(root: &Path, place: &str, name: &str, sectors: u64) -> PathBuf {
        let real = root.join("devices").join(place).join(name);
        put(&real.join("size"), &format!("{sectors}\n"));
        symlink(&real, root.join("class/block").join(name)).unwrap();
        real
    }

    fn partition(root: &Path, disk: &Path, name: &str) {
        let real = disk.join(name);
        put(&real.join("size"), "10\n");
        put(&real.join("partition"), "1\n");
        symlink(&real, root.join("class/block").join(name)).unwrap();
    }

    fn tree(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("faraday-attached-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("class/block")).unwrap();
        fs::create_dir_all(root.join("usb")).unwrap();
        root
    }

    fn look(root: &Path, boot: Option<&str>) -> Vec<String> {
        attached(&root.join("class/block"), &root.join("usb"), boot)
    }

    #[test]
    fn a_usb_disk_with_no_partitions_is_attached() {
        let root = tree("bare");
        disk(&root, "pci/usb1/1-1/1-1:1.0/host0/block", "sda", 1000);
        assert_eq!(look(&root, None), ["sda"]);
    }

    #[test]
    fn an_empty_reader_slot_and_a_sata_disk_are_not() {
        let root = tree("empty");
        disk(&root, "pci/usb1/1-1/1-1:1.0/host0/block", "sda", 0);
        disk(&root, "pci/ata1/host0/block", "sdb", 1000);
        assert!(look(&root, None).is_empty());
    }

    #[test]
    fn the_boot_disk_and_its_partitions_are_left_out_and_a_card_is_not() {
        let root = tree("boot");
        let sdb = disk(&root, "pci/usb1/1-2/1-2:1.0/host0/block", "sdb", 1000);
        partition(&root, &sdb, "sdb2");
        disk(&root, "soc/mmc/block", "mmcblk0", 1000);
        assert_eq!(look(&root, None), ["mmcblk0", "sdb", "sdb2"]);
        assert_eq!(look(&root, Some("sdb2")), ["mmcblk0"]);
    }

    #[test]
    fn an_authorised_mass_storage_interface_alone_is_attached() {
        let root = tree("iface");
        put(&root.join("usb/1-1:1.0/bInterfaceClass"), "08\n");
        put(&root.join("usb/1-2:1.0/bInterfaceClass"), "03\n");
        put(&root.join("usb/1-3:1.0/bInterfaceClass"), "08\n");
        put(&root.join("usb/1-3:1.0/authorized"), "0\n");
        assert_eq!(look(&root, None), ["1-1:1.0"]);
    }
}
