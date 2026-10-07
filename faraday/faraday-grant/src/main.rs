//! The grant helper, as `rcS` starts it: as root, once, in the
//! background (`PLAN.md` §4.3).
//!
//! ```text
//! faraday-grant
//! ```
//!
//! Before it reads anything it gives up everything but `CAP_CHOWN`: the
//! bounding set and the ambient set emptied while it is still root, its
//! own user and group (`ofgrant`, 202) with no supplementary groups,
//! `CAP_CHOWN` the only capability it keeps, and no-new-privileges set.
//! It has no seccomp filter (`PLAN.md` §12, decision 8): its only input
//! is `/sys`, and it reads nothing from a stick.
//!
//! Then it looks at `/sys` twice a second for as long as the machine is
//! up: partitions for the disk process (§4.3), USB devices and their
//! interfaces to authorise (§4.6), and the owners of input and video
//! nodes that appeared after boot.
//!
//! # Why this crate may use `unsafe`
//!
//! The workspace forbids `unsafe`; this binary is the second exception,
//! after `opensigner-v4l2`, and its `Cargo.toml` says so. Changing a
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

use faraday_grant::{Grant, Owner};

/// The helper's own account (users.table).
const GRANT_ID: u32 = 202;
/// The disk process's account.
const DISK_ID: u32 = 201;
/// Where the app publishes that it is clean.
const MARKER: &str = "/run/faraday-clean/clean";

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

/// Everything but `CAP_CHOWN` given up, for good.
fn drop_privilege() -> std::io::Result<()> {
    // While still root: no capability can ever be gained back, by this
    // process or anything it could start.
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
    })?;
    // SAFETY: as above.
    check(unsafe { libc::prctl(libc::PR_SET_KEEPCAPS, 1, 0, 0, 0) })?;
    // SAFETY: an empty group list, then plain integer ids.
    check(unsafe { libc::setgroups(0, std::ptr::null()) })?;
    // SAFETY: as above.
    check(unsafe { libc::setgid(GRANT_ID) })?;
    // SAFETY: as above.
    check(unsafe { libc::setuid(GRANT_ID) })?;
    let mut header = CapHeader {
        version: LINUX_CAPABILITY_VERSION_3,
        pid: 0,
    };
    let keep = CapData {
        effective: 1 << CAP_CHOWN,
        permitted: 1 << CAP_CHOWN,
        inheritable: 0,
    };
    let mut data = [
        keep,
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
    })?;
    // SAFETY: prctl with integer arguments only.
    check(unsafe { libc::prctl(libc::PR_SET_KEEPCAPS, 0, 0, 0, 0) })?;
    // SAFETY: as above.
    check(unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) })
}

fn main() -> ExitCode {
    if let Err(e) = drop_privilege() {
        say!("faraday-grant: cannot give up privilege: {e}");
        return ExitCode::FAILURE;
    }
    let mut grant = Grant::new(
        Path::new("/sys"),
        Path::new("/dev"),
        Path::new(MARKER),
        Chown,
    );
    loop {
        grant.tick();
        std::thread::sleep(Duration::from_millis(500));
    }
}
