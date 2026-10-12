//! Restart from the stick on Linux (`docs/HUB.md` §3.1): the attached
//! Faraday stick starts once, at the next start, through one firmware
//! boot entry labelled `Faraday stick`.
//!
//! Every Faraday stick's boot partition carries the same partition GUID
//! and is the first partition (`image/overlay/boards/x86_64-uefi/
//! genimage.cfg`), so one entry starts any of them. The hub app stays
//! unprivileged (D13): it reads the entries with `efibootmgr` as itself
//! (the firmware's variables are readable by everyone), then asks polkit
//! once, through `pkexec`, for the one `efibootmgr` that writes: setting
//! `BootNext` to the entry, or creating it (`--create-only`, so it is
//! never put in `BootOrder`) at a number chosen from the listing and
//! setting `BootNext` to it in the same run. Then `systemctl reboot`,
//! which logind allows the person at the computer.
//!
//! The listing is read by [`entries`], a pure function the tests feed
//! recorded outputs; [`Tools`] holds the programs' absolute paths, found
//! on a `PATH` given to it, so the tests run fakes and never a real
//! `efibootmgr`.

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use faraday_hub::api::Reason;

/// The entry's label.
pub const LABEL: &str = "Faraday stick";
/// What the entry starts, on the stick's boot partition.
pub const LOADER: &str = r"\EFI\BOOT\BOOTX64.EFI";
/// The boot partition's number on every Faraday stick.
pub const BOOT_PART: u32 = 1;

/// One `Boot####` entry of `efibootmgr`'s listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its number.
    pub num: u16,
    /// Its label.
    pub label: String,
}

/// The `Boot####` entries of `efibootmgr`'s or `efibootmgr -v`'s
/// output, in order. `BootCurrent`, `BootNext`, `BootOrder`, `Timeout`
/// and anything else are not entries. The label ends at a tab (where
/// the device path follows) or at the end of the line.
pub fn entries(listing: &str) -> Vec<Entry> {
    listing
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("Boot")?;
            let hex = rest.get(..4)?;
            if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let num = u16::from_str_radix(hex, 16).ok()?;
            // `*` for active, or a space.
            let after = rest.get(4..)?;
            let after = after
                .strip_prefix('*')
                .or_else(|| after.strip_prefix(' '))?;
            let label = after.split('\t').next().unwrap_or("").trim();
            Some(Entry {
                num,
                label: label.to_string(),
            })
        })
        .collect()
}

/// The numbers of the entries labelled `Faraday stick`.
pub fn faraday_entries(listing: &str) -> Vec<u16> {
    entries(listing)
        .into_iter()
        .filter(|e| e.label == LABEL)
        .map(|e| e.num)
        .collect()
}

/// The lowest entry number the listing does not use.
pub fn free_number(listing: &str) -> Option<u16> {
    let used: Vec<u16> = entries(listing).iter().map(|e| e.num).collect();
    (0..=u16::MAX).find(|n| !used.contains(n))
}

/// What `pkexec efibootmgr` is given to start the stick once: `BootNext`
/// set to the existing `Faraday stick` entry, or the entry created at a
/// free number and `BootNext` set to it, in one run.
pub fn next_args(listing: &str, device: &str) -> Result<Vec<String>, Reason> {
    let hex = |n: u16| format!("{n:04X}");
    if let Some(n) = faraday_entries(listing).first() {
        return Ok(vec!["--bootnext".into(), hex(*n)]);
    }
    let n = free_number(listing).ok_or_else(|| Reason::BootEntry("no free entry".into()))?;
    Ok(vec![
        "--create-only".into(),
        "--bootnum".into(),
        hex(n),
        "--disk".into(),
        device.into(),
        "--part".into(),
        BOOT_PART.to_string(),
        "--loader".into(),
        LOADER.into(),
        "--label".into(),
        LABEL.into(),
        "--bootnext".into(),
        hex(n),
    ])
}

/// A device as udisks2 names a whole disk: `/dev/` and letters and
/// digits only, so nothing else reaches the privileged command line.
pub fn plain_device(device: &str) -> bool {
    device
        .strip_prefix("/dev/")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// The absolute path of `name` in the first absolute directory of
/// `path` that holds it as an executable file.
pub fn which(name: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|d| d.is_absolute())
        .map(|d| d.join(name))
        .find(|p| {
            std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
}

/// The programs Restart from the stick runs, by absolute path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tools {
    /// `pkexec`.
    pub pkexec: PathBuf,
    /// `efibootmgr`.
    pub efibootmgr: PathBuf,
    /// `systemctl`.
    pub systemctl: PathBuf,
}

impl Tools {
    /// The three found on `path`, or the first that is missing.
    pub fn find(path: &OsStr) -> Result<Tools, Reason> {
        let need = |name: &str| which(name, path).ok_or_else(|| Reason::Missing(name.into()));
        Ok(Tools {
            efibootmgr: need("efibootmgr")?,
            pkexec: need("pkexec")?,
            systemctl: need("systemctl")?,
        })
    }
}

fn run(program: &Path, args: &[String]) -> std::io::Result<Output> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
}

/// What failed, in the program's words, or its exit status.
fn said(out: &Output) -> String {
    let words = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if words.is_empty() {
        out.status.to_string()
    } else {
        words
    }
}

/// `efibootmgr`'s listing, read as the person.
pub fn listing(efibootmgr: &Path) -> Result<String, Reason> {
    let out = run(efibootmgr, &[]).map_err(|e| Reason::BootEntry(e.to_string()))?;
    if !out.status.success() {
        return Err(Reason::BootEntry(said(&out)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `pkexec efibootmgr ARGS`: polkit's password declined or dismissed
/// (`pkexec` exits 126 or 127) is [`Reason::Declined`].
fn privileged(pkexec: &Path, efibootmgr: &Path, args: Vec<String>) -> Result<(), Reason> {
    let mut all = vec![efibootmgr.display().to_string()];
    all.extend(args);
    let out = run(pkexec, &all).map_err(|e| Reason::BootEntry(e.to_string()))?;
    match out.status.code() {
        Some(0) => Ok(()),
        Some(126 | 127) => Err(Reason::Declined),
        _ => Err(Reason::BootEntry(said(&out))),
    }
}

/// Restart from the stick (§3.1): the stick at `device` once at the
/// next start, then the restart. Nothing restarts unless `BootNext` was
/// set.
pub fn restart_from_stick(tools: &Tools, device: &str) -> Result<(), Reason> {
    if !plain_device(device) {
        return Err(Reason::NoStick);
    }
    let list = listing(&tools.efibootmgr)?;
    privileged(&tools.pkexec, &tools.efibootmgr, next_args(&list, device)?)?;
    let out = run(&tools.systemctl, &["reboot".to_string()])
        .map_err(|e| Reason::Restart(format!("systemctl: {e}")))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(Reason::Restart(said(&out)))
    }
}

/// Whether the firmware has a `Faraday stick` entry (§3.1c), from the
/// listing read as the person. Without `efibootmgr`, or when it cannot
/// list, none is known.
pub fn entry_exists(path: &OsStr) -> bool {
    which("efibootmgr", path)
        .and_then(|e| listing(&e).ok())
        .is_some_and(|l| !faraday_entries(&l).is_empty())
}

/// Uninstall's part (§3.1, §5.4), and Home's Remove the Faraday boot
/// entry (§3.1c): removes the `Faraday stick` entries,
/// when there are any, and nothing else. Without `efibootmgr` there can
/// be none.
pub fn remove_entry(path: &OsStr) -> Result<(), Reason> {
    let Some(efibootmgr) = which("efibootmgr", path) else {
        return Ok(());
    };
    let found = faraday_entries(&listing(&efibootmgr)?);
    if found.is_empty() {
        return Ok(());
    }
    let pkexec = which("pkexec", path).ok_or_else(|| Reason::Missing("pkexec".into()))?;
    for n in found {
        privileged(
            &pkexec,
            &efibootmgr,
            vec![
                "--delete-bootnum".into(),
                "--bootnum".into(),
                format!("{n:04X}"),
            ],
        )?;
    }
    Ok(())
}
