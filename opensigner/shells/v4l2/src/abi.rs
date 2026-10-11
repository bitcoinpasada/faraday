//! The Video4Linux2 ABI, declared rather than bound.
//!
//! Everything the capture path needs is here: five `#[repr(C)]` structs,
//! the FourCC codes for the four pixel formats we accept, the capability
//! and enumeration constants, and the `ioctl` request numbers derived
//! from the struct sizes with the kernel's own `_IOC` arithmetic.
//!
//! Two of the structs are laid out differently on a 32-bit and a 64-bit
//! target, because they carry a `struct timeval` and a pointer-sized
//! union member. The Pi is 32-bit ARM and the desktop is 64-bit, so both
//! layouts are declared and both sizes are asserted in the tests below.
//! Deriving the request numbers from `size_of` rather than pasting one
//! target's constants is what keeps the two in step: the size is part of
//! the request number, and a wrong one is `EINVAL` at run time on the
//! other target only.

use std::ffi::c_int;

// --- Buffer types, memory models, capabilities ---------------------------

/// `V4L2_BUF_TYPE_VIDEO_CAPTURE`: a single-planar capture queue.
pub const BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
/// `V4L2_FIELD_ANY`: the driver picks the field order.
pub const FIELD_ANY: u32 = 0;
/// `V4L2_MEMORY_MMAP`: buffers owned by the driver, mapped by us.
pub const MEMORY_MMAP: u32 = 1;

/// `V4L2_CAP_VIDEO_CAPTURE`: the node produces video frames.
pub const CAP_VIDEO_CAPTURE: u32 = 0x0000_0001;
/// `V4L2_CAP_STREAMING`: the node supports the buffer queue.
pub const CAP_STREAMING: u32 = 0x0400_0000;
/// `V4L2_CAP_DEVICE_CAPS`: `device_caps` is filled in and is the field
/// to test; without it only the driver-wide `capabilities` is set.
pub const CAP_DEVICE_CAPS: u32 = 0x8000_0000;

// --- Pixel formats -------------------------------------------------------

/// A V4L2 FourCC, little-endian as the kernel packs it.
const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

/// `V4L2_PIX_FMT_GREY`: 8-bit luma, exactly what the core wants.
pub const PIX_FMT_GREY: u32 = fourcc(b'G', b'R', b'E', b'Y');
/// `V4L2_PIX_FMT_YUYV`: packed 4:2:2, luma in every second byte.
pub const PIX_FMT_YUYV: u32 = fourcc(b'Y', b'U', b'Y', b'V');
/// `V4L2_PIX_FMT_NV12`: planar 4:2:0, luma plane first.
pub const PIX_FMT_NV12: u32 = fourcc(b'N', b'V', b'1', b'2');
/// `V4L2_PIX_FMT_YUV420`, whose FourCC is `YU12`: planar 4:2:0, luma
/// plane first.
pub const PIX_FMT_YU12: u32 = fourcc(b'Y', b'U', b'1', b'2');

/// The formats we ask for, in order of preference: colour first, since
/// the core draws the viewfinder in colour where a frame carries it,
/// and `GREY` last as the format with none to give.
pub const FORMATS: [u32; 4] = [PIX_FMT_YUYV, PIX_FMT_NV12, PIX_FMT_YU12, PIX_FMT_GREY];

// --- Structs -------------------------------------------------------------

/// `struct v4l2_capability`: 104 bytes on every target.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Capability {
    /// The driver's name, NUL-padded (`vivid`, `bcm2835-v4l2`, `uvcvideo`).
    pub driver: [u8; 16],
    /// The device's name.
    pub card: [u8; 32],
    /// The bus location.
    pub bus_info: [u8; 32],
    /// Kernel version.
    pub version: u32,
    /// The capabilities of every node this driver owns.
    pub capabilities: u32,
    /// The capabilities of this node alone; valid when
    /// [`CAP_DEVICE_CAPS`] is set in `capabilities`.
    pub device_caps: u32,
    /// Reserved; the kernel requires zeroes.
    pub reserved: [u32; 3],
}

impl Capability {
    /// The capabilities of this node: `device_caps` when the driver
    /// fills it in, the driver-wide set otherwise. The Pi's ISP and
    /// codec nodes and a metadata node are told apart from a camera by
    /// this field, which is why it is read rather than `capabilities`.
    pub fn node_caps(&self) -> u32 {
        if self.capabilities & CAP_DEVICE_CAPS != 0 {
            self.device_caps
        } else {
            self.capabilities
        }
    }
}

/// `struct v4l2_pix_format`: twelve `u32`s, 48 bytes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PixFormat {
    /// Frame width in pixels; the driver may adjust it.
    pub width: u32,
    /// Frame height in pixels; the driver may adjust it.
    pub height: u32,
    /// The FourCC; the driver may substitute one it supports.
    pub pixelformat: u32,
    /// Field order; [`FIELD_ANY`] on the way in.
    pub field: u32,
    /// Row stride in bytes, which may exceed the packed row.
    pub bytesperline: u32,
    /// Bytes in a whole frame.
    pub sizeimage: u32,
    /// Colour space; the driver's answer is not read.
    pub colorspace: u32,
    /// Driver-private data.
    pub private: u32,
    /// Format flags.
    pub flags: u32,
    /// The `ycbcr_enc`/`hsv_enc` union.
    pub enc: u32,
    /// Quantisation range.
    pub quantization: u32,
    /// Transfer function.
    pub xfer_func: u32,
}

/// The `fmt` union of `struct v4l2_format`: 200 bytes (`raw_data`).
const FORMAT_UNION: usize = 200;

/// `struct v4l2_format`: a type tag and a 200-byte union.
///
/// The union holds `struct v4l2_window`, which contains a pointer, so it
/// is 8-aligned on a 64-bit target and the tag is followed by four bytes
/// of padding there: 208 bytes against 204 on 32-bit. Only the
/// `v4l2_pix_format` member is used; the rest of the union is carried as
/// zeroed bytes, which is what the kernel expects of a field it does not
/// read.
#[repr(C)]
#[cfg_attr(target_pointer_width = "64", repr(align(8)))]
#[derive(Clone, Copy)]
pub struct Format {
    /// One of the `V4L2_BUF_TYPE_*` values.
    pub type_: u32,
    /// The alignment padding the 64-bit union forces.
    #[cfg(target_pointer_width = "64")]
    _pad: u32,
    /// The `pix` member of the union.
    pub pix: PixFormat,
    /// The rest of the union, never read.
    _rest: [u8; FORMAT_UNION - size_of::<PixFormat>()],
}

impl Format {
    /// A `v4l2_format` for the video capture queue carrying `pix`.
    pub fn capture(pix: PixFormat) -> Format {
        Format {
            type_: BUF_TYPE_VIDEO_CAPTURE,
            pix,
            ..Format::default()
        }
    }
}

impl Default for Format {
    fn default() -> Format {
        Format {
            type_: 0,
            #[cfg(target_pointer_width = "64")]
            _pad: 0,
            pix: PixFormat::default(),
            _rest: [0; FORMAT_UNION - size_of::<PixFormat>()],
        }
    }
}

/// `struct v4l2_requestbuffers`: 20 bytes on every target.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RequestBuffers {
    /// Buffers asked for on the way in, granted on the way out.
    pub count: u32,
    /// One of the `V4L2_BUF_TYPE_*` values.
    pub type_: u32,
    /// One of the `V4L2_MEMORY_*` values.
    pub memory: u32,
    /// Queue capabilities the driver reports back.
    pub capabilities: u32,
    /// Request flags.
    pub flags: u8,
    /// Reserved; the kernel requires zeroes.
    pub reserved: [u8; 3],
}

/// `struct v4l2_timecode`: 16 bytes on every target.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Timecode {
    /// Timecode type.
    pub type_: u32,
    /// Timecode flags.
    pub flags: u32,
    /// Frame number.
    pub frames: u8,
    /// Seconds.
    pub seconds: u8,
    /// Minutes.
    pub minutes: u8,
    /// Hours.
    pub hours: u8,
    /// User bits.
    pub userbits: [u8; 4],
}

/// `time_t` inside `struct timeval`, which is the whole reason
/// `v4l2_buffer` has two sizes.
#[cfg(target_pointer_width = "64")]
type Time = i64;
/// `time_t` inside `struct timeval` on a 32-bit target.
#[cfg(not(target_pointer_width = "64"))]
type Time = i32;

/// The pointer-sized `m` union of `struct v4l2_buffer`.
#[cfg(target_pointer_width = "64")]
type UnionWord = u64;
/// The pointer-sized `m` union on a 32-bit target.
#[cfg(not(target_pointer_width = "64"))]
type UnionWord = u32;

/// `struct v4l2_buffer`: 68 bytes on 32-bit, 88 on 64-bit.
///
/// The two differences are the `struct timeval` (two `long`s) and the
/// `m` union, whose widest member is a pointer. `#[repr(C)]` inserts the
/// alignment padding the C compiler does, so no padding field is written
/// out here; the sizes are asserted in the tests.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Buffer {
    /// Index into the driver's buffer array.
    pub index: u32,
    /// One of the `V4L2_BUF_TYPE_*` values.
    pub type_: u32,
    /// Bytes the driver actually wrote.
    pub bytesused: u32,
    /// Buffer flags.
    pub flags: u32,
    /// Field order of this frame.
    pub field: u32,
    /// Capture time, seconds.
    pub timestamp_sec: Time,
    /// Capture time, microseconds.
    pub timestamp_usec: Time,
    /// Timecode, when the driver has one.
    pub timecode: Timecode,
    /// Frame sequence number; gaps mean dropped frames.
    pub sequence: u32,
    /// One of the `V4L2_MEMORY_*` values.
    pub memory: u32,
    /// The `m` union: the mmap offset under [`MEMORY_MMAP`], a pointer
    /// under the other memory models, so it is pointer-sized.
    pub m: UnionWord,
    /// Buffer length in bytes.
    pub length: u32,
    /// Reserved; the kernel requires zeroes.
    pub reserved2: u32,
    /// The `request_fd`/`reserved` union.
    pub request_fd: i32,
}

// --- ioctl request numbers ----------------------------------------------

/// `_IOC_WRITE`: userspace writes the argument.
const IOC_WRITE: u32 = 1;
/// `_IOC_READ`: the kernel writes the argument.
const IOC_READ: u32 = 2;
/// `_IOC_TYPESHIFT`.
const TYPE_SHIFT: u32 = 8;
/// `_IOC_SIZESHIFT`.
const SIZE_SHIFT: u32 = 16;
/// `_IOC_DIRSHIFT`.
const DIR_SHIFT: u32 = 30;
/// The `'V'` ioctl type every V4L2 request carries.
const TYPE_V: u32 = b'V' as u32;

/// The kernel's `_IOC(dir, 'V', nr, size)`.
const fn ioc(dir: u32, nr: u32, size: usize) -> u32 {
    (dir << DIR_SHIFT) | ((size as u32) << SIZE_SHIFT) | (TYPE_V << TYPE_SHIFT) | nr
}

/// `VIDIOC_QUERYCAP`.
pub const VIDIOC_QUERYCAP: u32 = ioc(IOC_READ, 0, size_of::<Capability>());
/// `VIDIOC_S_FMT`.
pub const VIDIOC_S_FMT: u32 = ioc(IOC_READ | IOC_WRITE, 5, size_of::<Format>());
/// `VIDIOC_REQBUFS`.
pub const VIDIOC_REQBUFS: u32 = ioc(IOC_READ | IOC_WRITE, 8, size_of::<RequestBuffers>());
/// `VIDIOC_QUERYBUF`.
pub const VIDIOC_QUERYBUF: u32 = ioc(IOC_READ | IOC_WRITE, 9, size_of::<Buffer>());
/// `VIDIOC_QBUF`.
pub const VIDIOC_QBUF: u32 = ioc(IOC_READ | IOC_WRITE, 15, size_of::<Buffer>());
/// `VIDIOC_DQBUF`.
pub const VIDIOC_DQBUF: u32 = ioc(IOC_READ | IOC_WRITE, 17, size_of::<Buffer>());
/// `VIDIOC_STREAMON`.
pub const VIDIOC_STREAMON: u32 = ioc(IOC_WRITE, 18, size_of::<c_int>());
/// `VIDIOC_STREAMOFF`.
pub const VIDIOC_STREAMOFF: u32 = ioc(IOC_WRITE, 19, size_of::<c_int>());
/// `VIDIOC_TRY_FMT`: the same argument as `S_FMT` without setting
/// anything, which is what the device search uses so that probing a node
/// leaves it as it was found.
pub const VIDIOC_TRY_FMT: u32 = ioc(IOC_READ | IOC_WRITE, 64, size_of::<Format>());

// The same sizes as the tests below, checked at compile time, because
// the 32-bit tests cannot run on a 64-bit build box: `just pi-bin`
// cross-compiles for `armv7-unknown-linux-gnueabihf` and nothing on the
// box executes ARM. A wrong layout there is otherwise `EINVAL` on the
// device and nowhere else.
const _: () = assert!(size_of::<Capability>() == 104);
const _: () = assert!(size_of::<RequestBuffers>() == 20);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(size_of::<Format>() == 208 && size_of::<Buffer>() == 88);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(size_of::<Format>() == 204 && size_of::<Buffer>() == 68);

#[cfg(test)]
mod tests {
    use super::*;

    /// The three sizes that are the same on every target.
    #[test]
    fn fixed_struct_sizes() {
        assert_eq!(size_of::<Capability>(), 104);
        assert_eq!(size_of::<RequestBuffers>(), 20);
        assert_eq!(size_of::<PixFormat>(), 48);
        assert_eq!(size_of::<Timecode>(), 16);
    }

    /// `v4l2_format` and `v4l2_buffer` are wider on a 64-bit target: the
    /// union in the first is pointer-aligned, the second holds a
    /// `struct timeval` and a pointer-sized union member. Both sizes are
    /// asserted so that a change to either declaration fails here rather
    /// than as `EINVAL` on the Pi.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn struct_sizes_64_bit() {
        assert_eq!(size_of::<Format>(), 208);
        assert_eq!(size_of::<Buffer>(), 88);
        assert_eq!(align_of::<Format>(), 8);
        assert_eq!(align_of::<Buffer>(), 8);
    }

    /// The 32-bit layouts, which is what `just pi-bin` builds.
    #[test]
    #[cfg(target_pointer_width = "32")]
    fn struct_sizes_32_bit() {
        assert_eq!(size_of::<Format>(), 204);
        assert_eq!(size_of::<Buffer>(), 68);
        assert_eq!(align_of::<Format>(), 4);
        assert_eq!(align_of::<Buffer>(), 4);
    }

    /// The request numbers carry the struct size, so on a 64-bit target
    /// they must come out as the published constants. Getting these from
    /// `size_of` rather than pasting them is what makes the 32-bit build
    /// right too.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn ioctl_numbers_64_bit() {
        assert_eq!(VIDIOC_QUERYCAP, 0x8068_5600);
        assert_eq!(VIDIOC_S_FMT, 0xC0D0_5605);
        assert_eq!(VIDIOC_REQBUFS, 0xC014_5608);
        assert_eq!(VIDIOC_QUERYBUF, 0xC058_5609);
        assert_eq!(VIDIOC_QBUF, 0xC058_560F);
        assert_eq!(VIDIOC_DQBUF, 0xC058_5611);
        assert_eq!(VIDIOC_STREAMON, 0x4004_5612);
        assert_eq!(VIDIOC_STREAMOFF, 0x4004_5613);
        assert_eq!(VIDIOC_TRY_FMT, 0xC0D0_5640);
    }

    /// The same requests on 32-bit ARM: only the three that carry a
    /// size-varying struct differ, and they differ by the size alone.
    #[test]
    #[cfg(target_pointer_width = "32")]
    fn ioctl_numbers_32_bit() {
        assert_eq!(VIDIOC_QUERYCAP, 0x8068_5600);
        assert_eq!(VIDIOC_S_FMT, 0xC0CC_5605);
        assert_eq!(VIDIOC_REQBUFS, 0xC014_5608);
        assert_eq!(VIDIOC_QUERYBUF, 0xC044_5609);
        assert_eq!(VIDIOC_QBUF, 0xC044_560F);
        assert_eq!(VIDIOC_DQBUF, 0xC044_5611);
        assert_eq!(VIDIOC_STREAMON, 0x4004_5612);
        assert_eq!(VIDIOC_STREAMOFF, 0x4004_5613);
        assert_eq!(VIDIOC_TRY_FMT, 0xC0CC_5640);
    }

    /// The FourCCs are the little-endian packing the kernel headers use.
    #[test]
    fn fourcc_codes() {
        assert_eq!(PIX_FMT_GREY, 0x5945_5247);
        assert_eq!(PIX_FMT_YUYV, 0x5659_5559);
        assert_eq!(PIX_FMT_NV12, 0x3231_564E);
        assert_eq!(PIX_FMT_YU12, 0x3231_5559);
    }

    /// `device_caps` is the field that tells a camera node from the
    /// Pi's ISP and codec nodes, but only when the driver says it is
    /// filled in.
    #[test]
    fn node_caps_prefers_device_caps() {
        let mut cap = Capability {
            capabilities: CAP_VIDEO_CAPTURE | CAP_STREAMING,
            ..Capability::default()
        };
        assert_eq!(cap.node_caps(), CAP_VIDEO_CAPTURE | CAP_STREAMING);
        cap.capabilities |= CAP_DEVICE_CAPS;
        cap.device_caps = CAP_STREAMING;
        assert_eq!(cap.node_caps(), CAP_STREAMING);
    }

    /// The `Format` constructor lays the pixel format into the union
    /// and leaves the rest of it zeroed, which is what the kernel wants
    /// of a member it does not read.
    #[test]
    fn capture_format_carries_the_pix_member() {
        let f = Format::capture(PixFormat {
            width: 640,
            height: 480,
            pixelformat: PIX_FMT_GREY,
            ..PixFormat::default()
        });
        assert_eq!(f.type_, BUF_TYPE_VIDEO_CAPTURE);
        assert_eq!(f.pix.width, 640);
        assert_eq!(f.pix.pixelformat, PIX_FMT_GREY);
    }
}
