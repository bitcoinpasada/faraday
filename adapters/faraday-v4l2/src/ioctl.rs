//! The two ioctls the stick adapter needs of the kernel: the framebuffer's wait for the vertical
//! blank and an input device's exclusive grab. Their `unsafe` lives here, with the camera's, as on
//! `main` (`docs/deps/libc.md`).

use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;

const IOC_WRITE: u32 = 1;
const DIR_SHIFT: u32 = 30;
const SIZE_SHIFT: u32 = 16;
const TYPE_SHIFT: u32 = 8;

/// `_IOW(ty, nr, size)`.
const fn iow(ty: u8, nr: u8, size: usize) -> u32 {
    (IOC_WRITE << DIR_SHIFT)
        | ((size as u32) << SIZE_SHIFT)
        | ((ty as u32) << TYPE_SHIFT)
        | nr as u32
}

/// `EVIOCGRAB`: `_IOW('E', 0x90, int)`, an exclusive grab of an input device.
const EVIOCGRAB: u32 = iow(b'E', 0x90, size_of::<libc::c_int>());
/// `FBIO_WAITFORVSYNC`: `_IOW('F', 0x20, __u32)`, the framebuffer's wait for the vertical blank.
const FBIO_WAITFORVSYNC: u32 = iow(b'F', 0x20, size_of::<u32>());

/// Waits for the next vertical blank of the screen a framebuffer device shows on, and returns when
/// it comes.
///
/// The Pi's video driver gives up after a tenth of a second and returns at once when the screen is
/// off; a framebuffer without the call (a PC's firmware framebuffer) fails with `ENOTTY`, and one a
/// program holds the display of with `EBUSY`. The caller treats any error as "no vblank to wait
/// for".
pub fn wait_vblank(fb: &File) -> io::Result<()> {
    let mut crtc: u32 = 0;
    // SAFETY: `fb` is a live `File`, so its descriptor is valid; the request number is built from
    // `size_of::<u32>()` and `crtc` is an initialised, uniquely borrowed `u32` that outlives the
    // call, which writes at most that many bytes there.
    #[expect(
        unsafe_code,
        reason = "one ioctl, with a pointer to a local the request sizes"
    )]
    let r = unsafe {
        libc::ioctl(
            fb.as_raw_fd(),
            FBIO_WAITFORVSYNC as _,
            (&raw mut crtc).cast::<libc::c_void>(),
        )
    };
    if r < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Takes an input device exclusively (`EVIOCGRAB`): while this file is open, the kernel's own
/// handlers (the virtual terminal's keyboard among them) are given none of the device's events,
/// and only this reader is. The grab ends when the file is closed.
///
/// A file that is not an input device fails with `ENOTTY`; a device another process holds fails
/// with `EBUSY`.
pub fn grab_input(device: &File) -> io::Result<()> {
    // SAFETY: `device` is a live `File`, so its descriptor is valid; `EVIOCGRAB` takes its
    // argument by value as an `int`, and no pointer crosses the call.
    #[expect(unsafe_code, reason = "one ioctl, no pointer crosses the call")]
    let r = unsafe { libc::ioctl(device.as_raw_fd(), EVIOCGRAB as _, 1 as libc::c_int) };
    if r < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: the request numbers are the kernel's (`linux/input.h`, `linux/fb.h`).
    /// Regression: a mis-built `_IOW` makes the grab fail on a device that has it, and keys reach the console's buffer.
    /// Owner: adapter
    #[test]
    fn the_request_numbers_are_the_kernels() {
        assert_eq!(EVIOCGRAB, 0x4004_4590);
        assert_eq!(FBIO_WAITFORVSYNC, 0x4004_4620);
    }
}
