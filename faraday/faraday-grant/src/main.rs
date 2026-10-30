//! The grant helper, as `rcS` starts it: as root, once, in the
//! background (`docs/history/PLAN.md` §4.3).
//!
//! ```text
//! faraday-grant
//! faraday-grant --find-boot
//! ```
//!
//! Before it reads anything it gives up everything but `CAP_CHOWN`: the
//! bounding set and the ambient set emptied while it is still root, its
//! own user and group (`ofgrant`, 202) with no supplementary groups,
//! `CAP_CHOWN` the only capability it keeps, and no-new-privileges set.
//! It has no seccomp filter (`docs/history/PLAN.md` §12, decision 8): its only input
//! is `/sys`, and it reads nothing from a stick.
//!
//! Then it looks at `/sys` twice a second for as long as the machine is
//! up: partitions for the disk process (§4.3), never a boot partition
//! (F128), USB devices and their
//! interfaces to authorise (§4.6), and the owners of input and video
//! nodes that appeared after boot.
//!
//! `--find-boot`, which `rcS` starts beside it, finds the stick the PC
//! started from (`docs/STICK-IDENTITY.md` §5.2) when the medium's record,
//! `/etc/faraday/exchange`, says `layout`, and writes what it found there
//! in its place; on the Pi, whose record names the card, it ends at once.
//! It stays root, the owner of the partition nodes and of the record, and
//! gives up every capability and the means to gain one before it reads
//! anything. It reads `/sys`, and of a USB disk with a Faraday stick's
//! layout its first partition, up to 64 MiB, for the running kernel's
//! release string; nothing it reads decides what is kept back, which the
//! helper decides from `/sys` and the record alone.
//!
//! # Why this crate may use `unsafe`
//!
//! The workspace forbids `unsafe`; this binary is the second exception,
//! after `faraday-v4l2`, and its `Cargo.toml` says so. Changing a
//! node's owner and mode, changing user and giving up capabilities are
//! system calls `std` has no wrapper for. Every `unsafe` block is one call
//! through `libc`, with plain integers or a buffer that outlives it.

/// A line on stderr that is never fatal: once `rcS` has ended, the
/// console this process was started on may refuse writes, and
/// `eprintln!` panics when a write fails.
macro_rules! say {
    ($($t:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($t)*);
    }};
}

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use faraday_boot::medium::{LAYOUT, Record};
use faraday_grant::{Finder, Grant, Owner};

/// The helper's own account (users.table).
const GRANT_ID: u32 = 202;
/// The disk process's account.
const DISK_ID: u32 = 201;
/// Where the app publishes that it is clean.
const MARKER: &str = "/run/faraday-clean/clean";
/// The medium's record (`faraday_boot::medium`).
const RECORD: &str = "/etc/faraday/exchange";
/// The running kernel's release string.
const OSRELEASE: &str = "/proc/sys/kernel/osrelease";

const CAP_CHOWN: u32 = 0;

fn c_path(p: &Path) -> std::io::Result<CString> {
    CString::new(p.as_os_str().as_bytes()).map_err(|_| std::io::Error::other("a path with a NUL"))
}

fn check(r: libc::c_int) -> std::io::Result<()> {
    if r == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// The real changes of ownership.
struct Chown;

impl Owner for Chown {
    fn take(&mut self, node: &Path) -> std::io::Result<()> {
        let p = c_path(node)?;
        // SAFETY: `p` is a NUL-terminated path that outlives both calls.
        check(unsafe { libc::chown(p.as_ptr(), GRANT_ID, GRANT_ID) })?;
        // SAFETY: as above.
        check(unsafe { libc::chmod(p.as_ptr(), 0o600) })
    }

    fn give(&mut self, node: &Path) -> std::io::Result<()> {
        let p = c_path(node)?;
        // SAFETY: as in `take`.
        check(unsafe { libc::chown(p.as_ptr(), DISK_ID, DISK_ID) })
    }

    fn authorize(&mut self, attr: &Path) -> std::io::Result<()> {
        let p = c_path(attr)?;
        // SAFETY: as in `take`.
        check(unsafe { libc::chown(p.as_ptr(), GRANT_ID, GRANT_ID) })?;
        std::fs::write(attr, b"1")
    }

    fn assign(&mut self, node: &Path, uid: u32, gid: u32, mode: u32) -> std::io::Result<()> {
        let p = c_path(node)?;
        // SAFETY: as in `take`: the helper's own first, so it may set the
        // mode, then the owners `/etc/mdev.conf` names.
        check(unsafe { libc::chown(p.as_ptr(), GRANT_ID, GRANT_ID) })?;
        // SAFETY: as above.
        check(unsafe { libc::chmod(p.as_ptr(), mode as libc::mode_t) })?;
        // SAFETY: as above.
        check(unsafe { libc::chown(p.as_ptr(), uid, gid) })
    }

    fn owner_of(&mut self, node: &Path) -> std::io::Result<(u32, u32)> {
        use std::os::unix::fs::MetadataExt;
        let m = std::fs::metadata(node)?;
        Ok((m.uid(), m.gid()))
    }

    fn probe(&mut self, drivers_probe: &Path, interface: &str) -> std::io::Result<()> {
        let p = c_path(drivers_probe)?;
        // SAFETY: as in `take`.
        check(unsafe { libc::chown(p.as_ptr(), GRANT_ID, GRANT_ID) })?;
        std::fs::write(drivers_probe, interface.as_bytes())
    }

    fn back(&mut self, node: &Path) -> std::io::Result<()> {
        let p = c_path(node)?;
        // SAFETY: as in `take`.
        check(unsafe { libc::chown(p.as_ptr(), 0, 0) })
    }
}

#[repr(C)]
struct CapHeader {
    version: u32,
    pid: libc::c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CapData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

const LINUX_CAPABILITY_VERSION_3: u32 = 0x2008_0522;

/// Every capability out of the bounding and ambient sets, while still
/// root: none can ever be gained back, by this process or anything it
/// could start.
fn drop_bounding() -> std::io::Result<()> {
    for cap in 0..64 {
        // SAFETY: prctl with integer arguments only.
        let r = unsafe { libc::prctl(libc::PR_CAPBSET_DROP, cap, 0, 0, 0) };
        if r != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EINVAL) {
            return Err(std::io::Error::last_os_error());
        }
    }
    // SAFETY: as above.
    check(unsafe {
        libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL as libc::c_ulong,
            0,
            0,
            0,
        )
    })
}

/// Sets this process's capabilities to `keep` alone, effective and
/// permitted.
fn set_caps(keep: u32) -> std::io::Result<()> {
    let mut header = CapHeader {
        version: LINUX_CAPABILITY_VERSION_3,
        pid: 0,
    };
    let mut data = [
        CapData {
            effective: keep,
            permitted: keep,
            inheritable: 0,
        },
        CapData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        },
    ];
    // SAFETY: capset reads one header and two data structs, laid out as
    // the kernel's version 3 ABI.
    check(unsafe {
        libc::syscall(
            libc::SYS_capset,
            &mut header as *mut CapHeader,
            data.as_mut_ptr(),
        ) as libc::c_int
    })
}

/// Everything but `CAP_CHOWN` given up, for good.
fn drop_privilege() -> std::io::Result<()> {
    drop_bounding()?;
    // SAFETY: prctl with integer arguments only.
    check(unsafe { libc::prctl(libc::PR_SET_KEEPCAPS, 1, 0, 0, 0) })?;
    // SAFETY: an empty group list, then plain integer ids.
    check(unsafe { libc::setgroups(0, std::ptr::null()) })?;
    // SAFETY: as above.
    check(unsafe { libc::setgid(GRANT_ID) })?;
    // SAFETY: as above.
    check(unsafe { libc::setuid(GRANT_ID) })?;
    set_caps(1 << CAP_CHOWN)?;
    // SAFETY: prctl with integer arguments only.
    check(unsafe { libc::prctl(libc::PR_SET_KEEPCAPS, 0, 0, 0, 0) })?;
    // SAFETY: as above.
    check(unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) })
}

/// Every capability given up, for good, the user still root: what the
/// finder keeps is root's ownership of the nodes and the record.
fn drop_all() -> std::io::Result<()> {
    drop_bounding()?;
    set_caps(0)?;
    // SAFETY: prctl with integer arguments only.
    check(unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) })
}

/// `--find-boot`: looks four times a second until the finder can say,
/// and writes the record whole, by a rename. A stick not found yet may
/// still come, on a machine slow to bring its USB disks up: the finder
/// goes on looking, a second a time, and writes again once one, or two,
/// of this build are found. A stick found, or two, is final.
fn find_boot() -> ExitCode {
    if !std::fs::read_to_string(RECORD).is_ok_and(|r| r.trim() == LAYOUT) {
        return ExitCode::SUCCESS;
    }
    if let Err(e) = drop_all() {
        say!("faraday-grant: cannot give up privilege: {e}");
        return ExitCode::FAILURE;
    }
    let release = std::fs::read_to_string(OSRELEASE).unwrap_or_default();
    let mut finder = Finder::new(Path::new("/sys"), Path::new("/dev"), &release);
    let start = std::time::Instant::now();
    let mut written: Option<Record> = None;
    loop {
        match finder.look(start.elapsed()) {
            Some(r) if written.as_ref() != Some(&r) => {
                match &r {
                    Record::Found { partition, .. } => {
                        say!("faraday-grant: started from /dev/{partition}");
                    }
                    Record::Two => say!("faraday-grant: two sticks of this build: none taken"),
                    _ => say!("faraday-grant: the stick it started from was not found"),
                }
                let new = format!("{RECORD}.new");
                if let Err(e) =
                    std::fs::write(&new, r.text()).and_then(|()| std::fs::rename(&new, RECORD))
                {
                    say!("faraday-grant: cannot write {RECORD}: {e}");
                    return ExitCode::FAILURE;
                }
                if r != Record::NotFound {
                    return ExitCode::SUCCESS;
                }
                written = Some(r);
            }
            Some(_) => std::thread::sleep(Duration::from_secs(1)),
            None => std::thread::sleep(Duration::from_millis(250)),
        }
    }
}

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--find-boot") {
        return find_boot();
    }
    if let Err(e) = drop_privilege() {
        say!("faraday-grant: cannot give up privilege: {e}");
        return ExitCode::FAILURE;
    }
    let mut grant = Grant::new(
        Path::new("/sys"),
        Path::new("/dev"),
        Path::new(MARKER),
        Path::new(RECORD),
        Chown,
    );
    loop {
        grant.tick();
        std::thread::sleep(Duration::from_millis(500));
    }
}
