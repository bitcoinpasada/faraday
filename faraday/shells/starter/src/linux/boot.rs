//! Restart from the stick on Linux (`docs/STARTER.md` §3.1): the attached
//! Faraday stick starts once, at the next start, through one firmware
//! boot entry labelled `Faraday stick`.
//!
//! Every Faraday stick has its own disk and partition GUIDs
//! (`docs/STICK-IDENTITY.md`), and a firmware entry made by `efibootmgr
//! --disk --part` names the boot partition by its GUID, so an entry
//! starts only the stick it was made for. The Starter stays unprivileged
//! (D13): it reads the entries with `efibootmgr -v` as itself (the
//! firmware's variables are readable by everyone) and takes each
//! `Faraday stick` entry's partition GUID. When one names the attached
//! stick's boot partition (udisks2's `Partition.UUID`), polkit is asked
//! once, through `pkexec`, for the one `efibootmgr` that sets `BootNext`
//! to it. When none does, a new entry is created (`--create-only`, so it
//! is never put in `BootOrder`) at a number chosen from the listing and
//! `BootNext` set to it in the same run. Entries naming other sticks are
//! left (they start nothing; Uninstall removes every one), so a restart
//! asks for the password once. Then `systemctl reboot`, which logind
//! allows the person at the computer.
//!
//! The listing is read by [`entries`], a pure function the tests feed
//! recorded outputs. The programs run, `pkexec` as root included, are
//! only ever the ones in the system's own folders, owned by root and not
//! writable by anyone else ([`Trusted`]); the person's `PATH` is never
//! read, so a file in a folder the person can write is never run as
//! root. The tests give [`Trusted`] a folder and an owner of their own.

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use faraday_core::strings::EN;
use faraday_starter::api::Reason;

/// The entry's label.
pub const LABEL: &str = EN.starter_boot_entry_label;
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
    /// The GUID of the GPT partition its device path names (`-v`'s
    /// `HD(1,GPT,<guid>,…)`), lower case, when it names one.
    pub guid: Option<String>,
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
            let mut parts = after.splitn(2, '\t');
            let label = parts.next().unwrap_or("").trim();
            let guid = parts.next().and_then(partition_guid);
            Some(Entry {
                num,
                label: label.to_string(),
                guid,
            })
        })
        .collect()
}

/// The GUID in the first `HD(<n>,GPT,<guid>,…)` of a device path, lower
/// case.
fn partition_guid(path: &str) -> Option<String> {
    let rest = &path[path.find("HD(")? + 3..];
    let mut fields = rest.split(',');
    let _number = fields.next()?;
    if fields.next()? != "GPT" {
        return None;
    }
    let guid = fields.next()?.trim();
    (!guid.is_empty()).then(|| guid.to_ascii_lowercase())
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
/// set to the `Faraday stick` entry that names the boot partition `guid`
/// (any case), or, when none does, a new entry created at a free number
/// and `BootNext` set to it in one run. Entries naming other partitions
/// are left.
pub fn plan(listing: &str, device: &str, guid: &str) -> Result<Vec<String>, Reason> {
    let hex = |n: u16| format!("{n:04X}");
    let guid = guid.trim().to_ascii_lowercase();
    if guid.is_empty() {
        return Err(Reason::NoStick);
    }
    let faraday: Vec<Entry> = entries(listing)
        .into_iter()
        .filter(|e| e.label == LABEL)
        .collect();
    if let Some(e) = faraday
        .iter()
        .find(|e| e.guid.as_deref() == Some(guid.as_str()))
    {
        return Ok(vec!["--bootnext".into(), hex(e.num)]);
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

/// The system's own program folders, the only ones looked in.
pub const SYSTEM_DIRS: [&str; 4] = ["/usr/sbin", "/usr/bin", "/sbin", "/bin"];

/// Where programs run as root may come from: folders, and the user that
/// must own the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trusted {
    /// The folders, in order.
    pub dirs: Vec<PathBuf>,
    /// The owner the file must have: root (0) on a real system.
    pub uid: u32,
}

impl Trusted {
    /// The system's folders and root.
    pub fn system() -> Trusted {
        Trusted {
            dirs: SYSTEM_DIRS.iter().map(PathBuf::from).collect(),
            uid: 0,
        }
    }

    /// The absolute path of `name` in the first folder that holds it as
    /// an executable file owned by [`Trusted::uid`] and writable by no
    /// one else. The person's `PATH` is never consulted.
    pub fn which(&self, name: &str) -> Option<PathBuf> {
        self.dirs.iter().map(|d| d.join(name)).find(|p| {
            std::fs::metadata(p).is_ok_and(|m| {
                let mode = m.permissions().mode();
                m.is_file() && mode & 0o111 != 0 && m.uid() == self.uid && mode & 0o022 == 0
            })
        })
    }
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
    /// The three found in `from`, or the first that is missing.
    pub fn find(from: &Trusted) -> Result<Tools, Reason> {
        let need = |name: &str| from.which(name).ok_or_else(|| Reason::Missing(name.into()));
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

/// `efibootmgr -v`'s listing (older versions print the device paths only
/// with `-v`), read as the person.
pub fn listing(efibootmgr: &Path) -> Result<String, Reason> {
    let out = run(efibootmgr, &["-v".to_string()]).map_err(|e| Reason::BootEntry(e.to_string()))?;
    if !out.status.success() {
        return Err(Reason::BootEntry(said(&out)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// What deletes the entry `n`.
fn delete_args(n: u16) -> Vec<String> {
    vec![
        "--delete-bootnum".into(),
        "--bootnum".into(),
        format!("{n:04X}"),
    ]
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

/// Restart from the stick (§3.1): the stick at `device`, whose boot
/// partition has the GUID `guid`, once at the next start, then the
/// restart. Nothing restarts unless `BootNext` was set. One password
/// prompt; entries naming other sticks are left.
pub fn restart_from_stick(tools: &Tools, device: &str, guid: &str) -> Result<(), Reason> {
    if !plain_device(device) {
        return Err(Reason::NoStick);
    }
    let list = listing(&tools.efibootmgr)?;
    let plan = plan(&list, device, guid)?;
    privileged(&tools.pkexec, &tools.efibootmgr, plan)?;
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
pub fn entry_exists(from: &Trusted) -> bool {
    from.which("efibootmgr")
        .and_then(|e| listing(&e).ok())
        .is_some_and(|l| !faraday_entries(&l).is_empty())
}

/// Uninstall's part (§3.1, §5.4), and Home's Remove the Faraday boot
/// entry (§3.1c): removes the `Faraday stick` entries,
/// when there are any, and nothing else. Without `efibootmgr` there can
/// be none.
pub fn remove_entry(from: &Trusted) -> Result<(), Reason> {
    let Some(efibootmgr) = from.which("efibootmgr") else {
        return Ok(());
    };
    let found = faraday_entries(&listing(&efibootmgr)?);
    if found.is_empty() {
        return Ok(());
    }
    let pkexec = from
        .which("pkexec")
        .ok_or_else(|| Reason::Missing("pkexec".into()))?;
    for n in found {
        privileged(&pkexec, &efibootmgr, delete_args(n))?;
    }
    Ok(())
}
