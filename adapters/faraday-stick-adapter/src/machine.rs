//! What the machine says of itself at start: the panel's geometry, the density, the maker's
//! names, the medium, the boot parameters and the clean marker (`main`'s stick shell, `run`).

use std::path::{Path, PathBuf};

use faraday_port::Failure;
use faraday_port::devices::Edid;
use faraday_port::event::{Machine, SecureBootAtStart};
use faraday_port::values::Medium;
use faraday_text::Text;

use crate::fb::{Depth, Geometry};
use crate::orient;

pub const DEFAULT_WIDTH: u16 = 480;
pub const DEFAULT_HEIGHT: u16 = 640;
pub const DEFAULT_DPI: u16 = 160;
const CMDLINE_DPI: &str = "faraday.dpi=";
const CMDLINE_WINDOW: &str = "faraday.window=";
pub const CMDLINE: &str = "/proc/cmdline";
/// The RAM directory the carry and the start window's list wait in.
pub const BOXES: &str = "/run/faraday";
/// The disk process's pipes.
pub const DISK_DIR: &str = "/run/faraday-disk";
/// The record the stick's start leaves: the data partition of the medium the machine started from.
pub const EXCHANGE: &str = "/etc/faraday/exchange";
pub const BOOT_PARAMS: &str = "/sys/kernel/boot_params/data";
pub const DRM_CLASS: &str = "/sys/class/drm";
pub const SYS_VENDOR: &str = "/sys/class/dmi/id/sys_vendor";
pub const PRODUCT_FAMILY: &str = "/sys/class/dmi/id/product_family";
pub const PRODUCT_NAME: &str = "/sys/class/dmi/id/product_name";
pub const CHASSIS_TYPE: &str = "/sys/class/dmi/id/chassis_type";
pub const CPUINFO: &str = "/proc/cpuinfo";
pub const USB_DEVICES: &str = "/sys/bus/usb/devices";
pub const SYS_BLOCK: &str = "/sys/class/block";
/// The clean marker (`docs/SECURITY-MODEL.md` "Stateless"): present exactly while the process is
/// in the clean state. `faraday-grant` hands a partition out only while it is there.
pub const CLEAN_MARKER: &str = "/run/faraday-clean/clean";
/// Where the kernel's entropy is read from.
pub const ENTROPY_SOURCE: &str = "/dev/random";
const SECURE_BOOT_OFFSET: usize = 0x1ec;

/// What the removable medium is called on this machine: a card or stick on the Pi, and a stick
/// everywhere else.
pub fn medium() -> Medium {
    let card =
        std::fs::read_to_string(EXCHANGE).is_ok_and(|e| e.trim().starts_with("/dev/mmcblk0"));
    if cfg!(target_arch = "arm") || card {
        Medium::SdCard
    } else {
        Medium::Stick
    }
}

/// `/dev/fb0` to `/sys/class/graphics/fb0`.
fn sysfs_dir(fb: &Path) -> Option<PathBuf> {
    let name = fb.file_name()?.to_str()?;
    name.starts_with("fb")
        .then(|| PathBuf::from("/sys/class/graphics").join(name))
}

/// What sysfs says about the framebuffer. `Ok(None)` is "nothing was asked": off the device, or no
/// such framebuffer directory. An `Err` is one that is there and cannot be described.
pub fn probe(fb: &Path, off_device: bool) -> Result<Option<Geometry>, Failure> {
    if off_device {
        return Ok(None);
    }
    match sysfs_dir(fb) {
        Some(dir) if dir.is_dir() => Geometry::from_sysfs(&dir)
            .map(Some)
            .map_err(|_| Failure::Unavailable),
        _ => Ok(None),
    }
}

/// The geometry to draw with: sysfs on the device, the options off it, and the options win
/// wherever both have an answer.
pub fn geometry(
    size: Option<(u16, u16)>,
    depth: Option<Depth>,
    probed: Option<Geometry>,
) -> Result<Geometry, Failure> {
    let (width, height) = size
        .or(probed.map(|g| (g.width, g.height)))
        .unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));
    let depth = depth.or(probed.map(|g| g.depth)).unwrap_or(Depth::Rgb565);
    if width == 0 || height == 0 {
        return Err(Failure::Unavailable);
    }
    // A probed stride is only meaningful for the probed size and depth.
    let packed = usize::from(width) * depth.bytes_per_pixel();
    let stride = match probed {
        Some(g) if (g.width, g.height, g.depth) == (width, height, depth) => g.stride,
        _ => packed,
    };
    Ok(Geometry {
        width,
        height,
        depth,
        stride,
    })
}

/// The density to report (`docs/ORIENTATION.md` §2.1): the option first; then, on a screen of 11
/// inches or less whose EDID gives its size, the EDID's; then the kernel command line's, then 160,
/// doubled on a framebuffer 2,800 pixels wide or more.
pub fn dpi(
    asked: Option<u16>,
    cmdline: &str,
    edid: Option<faraday_paper::edid::Size>,
    fb: (u16, u16),
) -> u16 {
    if let Some(dpi) = asked {
        return dpi;
    }
    if let Some(dpi) = orient::edid_dpi(edid, fb) {
        return dpi;
    }
    let dpi = cmdline
        .split_whitespace()
        .find_map(|word| word.strip_prefix(CMDLINE_DPI))
        .and_then(|n| n.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_DPI);
    if fb.0 >= 2800 {
        dpi.saturating_mul(2)
    } else {
        dpi
    }
}

/// The window the kernel command line asks for, as `WxH`.
pub fn window_asked(cmdline: &str) -> Option<(u16, u16)> {
    let v = cmdline
        .split_whitespace()
        .find_map(|word| word.strip_prefix(CMDLINE_WINDOW))?;
    let (w, h) = v.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Secure Boot as the firmware left it in the boot parameters: byte `0x1ec` is 2 for off and 3
/// for on; anything else is unknown.
pub fn secure_boot(boot_params: &[u8]) -> SecureBootAtStart {
    match boot_params.get(SECURE_BOOT_OFFSET) {
        Some(2) => SecureBootAtStart::Off,
        Some(3) => SecureBootAtStart::On,
        _ => SecureBootAtStart::Unknown,
    }
}

/// Whether the chassis type is a laptop (8, 9, 10, 14, 31, 32): its keyboard is on the PS/2
/// controller inside it.
pub fn chassis_laptop(chassis_type: &str) -> bool {
    matches!(chassis_type.trim().parse::<u8>(), Ok(8..=10 | 14 | 31 | 32))
}

fn read_text(path: &str) -> Option<Text> {
    let text = std::fs::read_to_string(path).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| Text::of(text))
}

/// What the machine says of itself, once (`Start::machine`).
pub fn machine(boot_params: &[u8], drm_edid: &[u8], cores: u16) -> Machine {
    let cpu = std::fs::read_to_string(CPUINFO).unwrap_or_default();
    let screen = faraday_paper::edid::from_boot_params(boot_params)
        .or_else(|| (!drm_edid.is_empty()).then_some(drm_edid))
        .and_then(|b| <[u8; 128]>::try_from(b.get(..128)?).ok())
        .map(Edid);
    Machine {
        maker: read_text(SYS_VENDOR),
        product: read_text(PRODUCT_NAME),
        family: read_text(PRODUCT_FAMILY),
        chassis: std::fs::read_to_string(CHASSIS_TYPE)
            .ok()
            .and_then(|c| c.trim().parse().ok()),
        hypervisor: cpu
            .lines()
            .any(|l| l.starts_with("flags") && l.split_whitespace().any(|w| w == "hypervisor")),
        screen,
        secure_boot: secure_boot(boot_params),
        memory_total_mib: total_memory_mib(),
        cores,
    }
}

/// This machine's total memory in MiB, from `/proc/meminfo`.
fn total_memory_mib() -> Option<u32> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    u32::try_from(kib / 1024).ok()
}

/// Whether the root filesystem is mounted read-only, as `/proc/mounts` says.
pub fn root_read_only() -> Option<bool> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    let root = mounts
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let (_dev, at, _kind, options) = (f.next()?, f.next()?, f.next()?, f.next()?);
            (at == "/").then_some(options)
        })
        .next_back()?;
    Some(root.split(',').any(|o| o == "ro"))
}

/// Writes a marker file holding `body`, or removes it; removing one that is not there is done.
pub fn put_marker(path: &Path, on: bool, body: &[u8]) -> std::io::Result<()> {
    if on {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(path, body)
    } else {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            r => r,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: the density is the option's, else the command line's, else 160, doubled on a very wide panel.
    /// Regression: text a quarter of its size on a high-density laptop screen.
    /// Owner: adapter
    #[test]
    fn the_density_is_the_option_then_the_command_line_then_the_default() {
        assert_eq!(dpi(Some(200), "faraday.dpi=120", None, (480, 640)), 200);
        assert_eq!(dpi(None, "quiet faraday.dpi=120", None, (480, 640)), 120);
        assert_eq!(dpi(None, "", None, (480, 640)), 160);
        assert_eq!(dpi(None, "", None, (3200, 1800)), 320);
    }

    /// Invariant: a probed stride is used only for the probed size and depth.
    /// Regression: a stride of another mode writes rows sheared across the panel.
    /// Owner: adapter
    #[test]
    fn a_probed_stride_is_kept_only_for_the_probed_mode() {
        let probed = Geometry {
            width: 480,
            height: 640,
            depth: Depth::Rgb565,
            stride: 1024,
        };
        let same = geometry(None, None, Some(probed)).expect("a geometry");
        assert_eq!(same.stride, 1024);
        let other = geometry(Some((320, 240)), None, Some(probed)).expect("a geometry");
        assert_eq!(other.stride, 640);
        assert!(geometry(Some((0, 10)), None, None).is_err());
    }
}
