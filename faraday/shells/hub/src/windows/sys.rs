//! Every Win32 call the hub app makes that std does not, through
//! `windows-sys` (`docs/HUB.md` §2). Each `unsafe` block is one call or
//! one buffer's use, with its `SAFETY` note; everything around them is
//! safe Rust, and what the answers mean is the pure modules' (`disks`,
//! `registry`).
//!
//! Disks and volumes are opened through std's `OpenOptions` (with no
//! access for the queries); only the IOCTLs on the handles are calls of
//! their own here.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_CANCELLED, ERROR_INSUFFICIENT_BUFFER, ERROR_MORE_DATA, ERROR_NO_DATA,
    ERROR_PIPE_CONNECTED, ERROR_SUCCESS, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
    WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, FindFirstVolumeW, FindNextVolumeW, FindVolumeClose,
    GetVolumeInformationW, GetVolumePathNamesForVolumeNameW, PIPE_ACCESS_INBOUND, QueryDosDeviceW,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Diagnostics::Debug::{SEM_FAILCRITICALERRORS, SetThreadErrorMode};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, PIPE_NOWAIT,
    PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    SetNamedPipeHandleState,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
    RRF_RT_ANY, RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegOpenKeyExW,
    RegSetValueExW,
};
use windows_sys::Win32::System::SystemInformation::{
    GetFirmwareType, GetSystemDirectoryW, GetSystemWindowsDirectoryW, IMAGE_FILE_MACHINE_UNKNOWN,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, GetProcessId, IsWow64Process2, OpenProcessToken,
    WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::{
    SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    SHGetKnownFolderPath, ShellExecuteExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows_sys::core::GUID;

use super::install::Data;

/// `s` as a NUL-terminated UTF-16 string.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// A UTF-16 buffer up to its first NUL.
fn narrow(b: &[u16]) -> String {
    let end = b.iter().position(|&u| u == 0).unwrap_or(b.len());
    String::from_utf16_lossy(&b[..end])
}

fn raw(f: &impl AsRawHandle) -> HANDLE {
    f.as_raw_handle() as HANDLE
}

/// Stops Windows from asking the person to insert a disk when an empty
/// card reader is queried, on this thread.
pub fn quiet_errors() {
    // SAFETY: sets this thread's error mode; the old mode's pointer may
    // be null.
    unsafe {
        SetThreadErrorMode(SEM_FAILCRITICALERRORS, ptr::null_mut());
    }
}

/// An IOCTL on `f`, whose answer is at most `max` bytes; the buffer is
/// doubled from 512 bytes while Windows says it is too small.
pub fn ioctl(f: &File, code: u32, input: &[u8], max: usize) -> io::Result<Vec<u8>> {
    let mut len = 512usize.min(max);
    loop {
        let mut out = vec![0u8; len];
        let mut got = 0u32;
        // SAFETY: `input` and `out` are live buffers of the lengths
        // given; `got` is written before the call returns; no OVERLAPPED,
        // so the call completes before it returns.
        let ok = unsafe {
            DeviceIoControl(
                raw(f),
                code,
                input.as_ptr().cast(),
                input.len() as u32,
                out.as_mut_ptr().cast(),
                out.len() as u32,
                &mut got,
                ptr::null_mut(),
            )
        };
        if ok != 0 {
            out.truncate(got as usize);
            return Ok(out);
        }
        let e = io::Error::last_os_error();
        let small = matches!(
            e.raw_os_error().map(|c| c as u32),
            Some(ERROR_INSUFFICIENT_BUFFER | ERROR_MORE_DATA)
        );
        if !small || len >= max {
            return Err(e);
        }
        len = (len * 2).min(max);
    }
}

/// A value under `HKEY_LOCAL_MACHINE` or `HKEY_CURRENT_USER`, its bytes.
pub fn reg_get(root: HKEY, key: &str, value: &str) -> Option<Vec<u8>> {
    let (k, v) = (wide(key), wide(value));
    let mut len = 0u32;
    // SAFETY: the strings are NUL-terminated; a null data pointer asks
    // only for the length, written to `len`.
    let e = unsafe {
        RegGetValueW(
            root,
            k.as_ptr(),
            v.as_ptr(),
            RRF_RT_ANY,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut len,
        )
    };
    if e != ERROR_SUCCESS {
        return None;
    }
    let mut buf = vec![0u8; len as usize + 2];
    let mut len = buf.len() as u32;
    // SAFETY: `buf` holds `len` bytes, which is what the call may write.
    let e = unsafe {
        RegGetValueW(
            root,
            k.as_ptr(),
            v.as_ptr(),
            RRF_RT_ANY,
            ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut len,
        )
    };
    (e == ERROR_SUCCESS).then(|| {
        buf.truncate(len as usize);
        buf
    })
}

/// When Windows started, in seconds since 1970: the clock minus the
/// tick count. Stable within a boot to a second or two.
pub fn boot_time() -> Option<u64> {
    use std::time::{SystemTime, UNIX_EPOCH};
    // SAFETY: no arguments, no pointers.
    let up = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() } / 1000;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    now.checked_sub(up)
}

/// `GetFirmwareType`.
pub fn firmware_type() -> Option<u32> {
    let mut t = 0i32;
    // SAFETY: writes one FIRMWARE_TYPE to `t`.
    let ok = unsafe { GetFirmwareType(&mut t) };
    (ok != 0).then_some(t as u32)
}

/// The computer's own machine type, from `IsWow64Process2`.
pub fn native_machine() -> Option<u16> {
    let (mut process, mut native) = (IMAGE_FILE_MACHINE_UNKNOWN, IMAGE_FILE_MACHINE_UNKNOWN);
    // SAFETY: the pseudo-handle of this process needs no closing; both
    // outputs are written by the call.
    let ok = unsafe { IsWow64Process2(GetCurrentProcess(), &mut process, &mut native) };
    (ok != 0).then_some(native)
}

/// Every volume's name (`\\?\Volume{…}\`).
pub fn volumes() -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = [0u16; 64];
    // SAFETY: `buf` holds the length given.
    let find = unsafe { FindFirstVolumeW(buf.as_mut_ptr(), buf.len() as u32) };
    if find == INVALID_HANDLE_VALUE {
        return out;
    }
    loop {
        out.push(narrow(&buf));
        buf = [0u16; 64];
        // SAFETY: `find` is the open search; `buf` holds the length given.
        if unsafe { FindNextVolumeW(find, buf.as_mut_ptr(), buf.len() as u32) } == 0 {
            break;
        }
    }
    // SAFETY: closes the search opened above, once.
    unsafe { FindVolumeClose(find) };
    out
}

/// A volume's drive letters and folders (`E:\`).
pub fn volume_paths(name: &str) -> Vec<String> {
    let n = wide(name);
    let mut buf = vec![0u16; 1024];
    let mut len = 0u32;
    // SAFETY: `n` is NUL-terminated; `buf` holds the length given.
    let ok = unsafe {
        GetVolumePathNamesForVolumeNameW(n.as_ptr(), buf.as_mut_ptr(), buf.len() as u32, &mut len)
    };
    if ok == 0 {
        return Vec::new();
    }
    buf.split(|&u| u == 0)
        .take_while(|s| !s.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

/// A volume's label and file system's name, each empty when Windows has
/// none (no file system mounted).
pub fn volume_info(name: &str) -> (String, String) {
    let n = wide(name);
    let (mut label, mut fs) = ([0u16; 261], [0u16; 261]);
    // SAFETY: `n` is NUL-terminated; both buffers hold the lengths given;
    // the other outputs may be null.
    let ok = unsafe {
        GetVolumeInformationW(
            n.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            fs.as_mut_ptr(),
            fs.len() as u32,
        )
    };
    if ok == 0 {
        return (String::new(), String::new());
    }
    (narrow(&label), narrow(&fs))
}

/// A volume's NT device (`\Device\HarddiskVolume3`).
pub fn dos_device(name: &str) -> String {
    let short = name.trim_start_matches(r"\\?\").trim_end_matches('\\');
    let n = wide(short);
    let mut buf = [0u16; 512];
    // SAFETY: `n` is NUL-terminated; `buf` holds the length given.
    let len = unsafe { QueryDosDeviceW(n.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 {
        String::new()
    } else {
        narrow(&buf)
    }
}

/// `GetSystemWindowsDirectoryW`.
pub fn windows_dir() -> String {
    let mut buf = [0u16; 512];
    // SAFETY: `buf` holds the length given.
    let len = unsafe { GetSystemWindowsDirectoryW(buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 || len as usize >= buf.len() {
        String::new()
    } else {
        narrow(&buf)
    }
}

/// `GetSystemDirectoryW`: where Windows' own programs are.
pub fn system_dir() -> Option<PathBuf> {
    let mut buf = [0u16; 512];
    // SAFETY: `buf` holds the length given.
    let len = unsafe { GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) };
    (len != 0 && (len as usize) < buf.len()).then(|| PathBuf::from(narrow(&buf)))
}

/// A known folder's path (`FOLDERID_*`).
pub fn known_folder(id: &GUID) -> Option<PathBuf> {
    let mut path: *mut u16 = ptr::null_mut();
    // SAFETY: writes a CoTaskMem string's pointer to `path` (or null).
    let hr = unsafe { SHGetKnownFolderPath(id, 0, 0, &mut path) };
    if path.is_null() {
        return None;
    }
    let mut len = 0usize;
    // SAFETY: on success `path` is a NUL-terminated string the shell
    // allocated; it is read up to its NUL and freed once.
    let text = unsafe {
        while *path.add(len) != 0 {
            len += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(path, len));
        CoTaskMemFree(path as *const c_void);
        s
    };
    (hr >= 0 && !text.is_empty()).then(|| PathBuf::from(text))
}

/// The person's account's SID, as text (`S-1-5-21-…`).
pub fn user_sid() -> Option<String> {
    let mut token: HANDLE = 0;
    // SAFETY: opens this process's token for query; closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }
    // A TOKEN_USER and its SID fit in this many bytes, aligned as the
    // structure needs.
    let mut buf = [0u64; 64];
    let mut len = 0u32;
    // SAFETY: `buf` holds the length given, aligned for TOKEN_USER.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buf.as_mut_ptr().cast(),
            (buf.len() * 8) as u32,
            &mut len,
        )
    };
    // SAFETY: closes the token opened above, once.
    unsafe { CloseHandle(token) };
    if ok == 0 {
        return None;
    }
    // SAFETY: the call filled `buf` with a TOKEN_USER whose SID pointer
    // points inside `buf`, which lives until the end of this function.
    let sid = unsafe { (*(buf.as_ptr() as *const TOKEN_USER)).User.Sid };
    let mut text: *mut u16 = ptr::null_mut();
    // SAFETY: `sid` is valid (above); the string is LocalAlloc'd.
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 || text.is_null() {
        return None;
    }
    let mut n = 0usize;
    // SAFETY: `text` is NUL-terminated; read to its NUL and freed once.
    let s = unsafe {
        while *text.add(n) != 0 {
            n += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(text, n));
        LocalFree(text.cast());
        s
    };
    Some(s)
}

/// Makes the named pipe `name` for the elevated writer to report on:
/// inbound, one instance, local clients only, open to the person's own
/// account and to an elevated administrator (who could write any disk
/// anyway), and in no-wait mode until a client connects.
pub fn pipe(name: &str) -> io::Result<File> {
    let sid = user_sid().ok_or_else(|| io::Error::other("the account's SID"))?;
    let sddl = wide(&format!("D:P(A;;GA;;;{sid})(A;;GA;;;BA)"));
    let mut sd: *mut c_void = ptr::null_mut();
    // SAFETY: `sddl` is NUL-terminated; the descriptor is LocalAlloc'd
    // and freed below, after the pipe is made.
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd,
        bInheritHandle: 0,
    };
    let n = wide(name);
    // SAFETY: `n` is NUL-terminated and `attrs` points at a live
    // descriptor for the length of the call.
    let h = unsafe {
        CreateNamedPipeW(
            n.as_ptr(),
            PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            0,
            64 << 10,
            0,
            &attrs,
        )
    };
    let e = io::Error::last_os_error();
    // SAFETY: frees the descriptor allocated above, once.
    unsafe { LocalFree(sd) };
    if h == INVALID_HANDLE_VALUE {
        return Err(e);
    }
    // SAFETY: `h` is a new, valid handle nothing else owns.
    Ok(File::from(unsafe { OwnedHandle::from_raw_handle(h as _) }))
}

/// Whether a client has connected to `pipe` (in no-wait mode); once one
/// has, the pipe is put in wait mode for reading.
pub fn pipe_connected(pipe: &File) -> io::Result<bool> {
    // SAFETY: `pipe` is a live pipe handle; no OVERLAPPED.
    let ok = unsafe { ConnectNamedPipe(raw(pipe), ptr::null_mut()) };
    // SAFETY: reads this thread's last error.
    let e = unsafe { GetLastError() };
    if ok == 0 && !matches!(e, ERROR_PIPE_CONNECTED | ERROR_NO_DATA) {
        return Ok(false);
    }
    if ok != 0 {
        return Ok(false);
    }
    let mode = PIPE_READMODE_BYTE | PIPE_WAIT;
    // SAFETY: `pipe` is live; `mode` is read during the call.
    if unsafe { SetNamedPipeHandleState(raw(pipe), &mode, ptr::null(), ptr::null()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(true)
}

/// The process at the other end of `pipe`.
pub fn pipe_client(pipe: &File) -> Option<u32> {
    let mut pid = 0u32;
    // SAFETY: `pipe` is live; writes one u32.
    let ok = unsafe { GetNamedPipeClientProcessId(raw(pipe), &mut pid) };
    (ok != 0).then_some(pid)
}

/// A process this one started.
pub struct Process(OwnedHandle);

impl Process {
    /// Waits up to `wait` for it to end; whether it has.
    pub fn ended(&self, wait: Duration) -> bool {
        let ms = u32::try_from(wait.as_millis()).unwrap_or(u32::MAX);
        // SAFETY: the handle is live while `self` is.
        unsafe { WaitForSingleObject(raw(&self.0), ms) == WAIT_OBJECT_0 }
    }

    /// Its exit code, once it has ended.
    pub fn code(&self) -> Option<u32> {
        let mut code = 0u32;
        // SAFETY: the handle is live; writes one u32.
        let ok = unsafe { GetExitCodeProcess(raw(&self.0), &mut code) };
        (ok != 0).then_some(code)
    }

    /// Its process ID.
    pub fn id(&self) -> u32 {
        // SAFETY: the handle is live.
        unsafe { GetProcessId(raw(&self.0)) }
    }
}

/// Why an elevated start did not happen.
pub enum Elevation {
    /// The person declined the prompt.
    Declined,
    /// Windows' words.
    Failed(io::Error),
}

/// Starts `file` with `params` through the elevation prompt (`runas`),
/// with no window of its own.
pub fn run_elevated(file: &Path, params: &str) -> Result<Process, Elevation> {
    let (verb, f, p) = (
        wide("runas"),
        wide(&file.display().to_string()),
        wide(params),
    );
    // SAFETY: an all-zero SHELLEXECUTEINFOW is a valid empty one.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
    info.lpVerb = verb.as_ptr();
    info.lpFile = f.as_ptr();
    info.lpParameters = p.as_ptr();
    info.nShow = SW_HIDE;
    // SAFETY: the strings `info` points at live until the call returns.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let e = io::Error::last_os_error();
        return Err(if e.raw_os_error() == Some(ERROR_CANCELLED as i32) {
            Elevation::Declined
        } else {
            Elevation::Failed(e)
        });
    }
    if info.hProcess == 0 {
        return Err(Elevation::Failed(io::Error::other("no process")));
    }
    // SAFETY: SEE_MASK_NOCLOSEPROCESS hands this process the handle.
    Ok(Process(unsafe {
        OwnedHandle::from_raw_handle(info.hProcess as _)
    }))
}

/// The person's registry under `HKEY_CURRENT_USER`.
pub struct CurrentUser;

impl super::install::Registry for CurrentUser {
    fn set(&mut self, key: &str, name: &str, data: &Data) -> Result<(), String> {
        let (k, n) = (wide(key), wide(name));
        let mut h: HKEY = 0;
        // SAFETY: `k` is NUL-terminated; the key handle is closed below.
        let e = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                k.as_ptr(),
                0,
                ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                ptr::null(),
                &mut h,
                ptr::null_mut(),
            )
        };
        if e != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(e as i32).to_string());
        }
        let (kind, bytes): (u32, Vec<u8>) = match data {
            Data::Sz(s) => (
                REG_SZ,
                wide(s).iter().flat_map(|u| u.to_le_bytes()).collect(),
            ),
            Data::Dword(v) => (REG_DWORD, v.to_le_bytes().to_vec()),
        };
        // SAFETY: `h` is open; `bytes` holds the length given.
        let e =
            unsafe { RegSetValueExW(h, n.as_ptr(), 0, kind, bytes.as_ptr(), bytes.len() as u32) };
        // SAFETY: closes the key opened above, once.
        unsafe { RegCloseKey(h) };
        if e != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(e as i32).to_string());
        }
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), String> {
        let k = wide(key);
        // SAFETY: `k` is NUL-terminated.
        let e = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, k.as_ptr()) };
        match e {
            ERROR_SUCCESS | windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND => Ok(()),
            e => Err(io::Error::from_raw_os_error(e as i32).to_string()),
        }
    }

    fn get(&mut self, key: &str, name: &str) -> Option<Data> {
        let b = reg_get(HKEY_CURRENT_USER, key, name)?;
        super::registry::sz(&b).map(Data::Sz)
    }

    fn delete_value(&mut self, key: &str, name: &str) -> Result<(), String> {
        use windows_sys::Win32::System::Registry::RegDeleteKeyValueW;
        let (k, n) = (wide(key), wide(name));
        // SAFETY: both strings are NUL-terminated; only the value goes.
        let e = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr()) };
        match e {
            ERROR_SUCCESS | windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND => Ok(()),
            e => Err(io::Error::from_raw_os_error(e as i32).to_string()),
        }
    }

    fn exists(&mut self, key: &str) -> bool {
        let k = wide(key);
        let mut h: HKEY = 0;
        // SAFETY: `k` is NUL-terminated; a key opened is closed below.
        let e = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, k.as_ptr(), 0, KEY_READ, &mut h) };
        if e == ERROR_SUCCESS {
            // SAFETY: closes the key opened above, once.
            unsafe { RegCloseKey(h) };
        }
        e == ERROR_SUCCESS
    }
}

// ---------------------------------------------------------------------
// The BitLocker guard (docs/HUB.md §6.2): the account, the shell's
// property store, and WMI's Win32_EncryptableVolume.
//
// windows-sys declares WMI's and the property store's interfaces only as
// opaque pointers, with none of their methods: the few methods used are
// called here through their vtables, each slot by its place in the
// interface's declaration (wbemcli.h, propsys.h), with the signature the
// header gives it.
// ---------------------------------------------------------------------

/// Whether this account is an administrator's: its token is split (the
/// elevation prompt asks it only to confirm) or already elevated. A
/// declined prompt on another account is one that asked for an
/// administrator's password (§6.4).
pub fn administrator() -> bool {
    use windows_sys::Win32::Security::{
        TokenElevationType, TokenElevationTypeFull, TokenElevationTypeLimited,
    };
    let mut token: HANDLE = 0;
    // SAFETY: opens this process's token for query; closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return false;
    }
    let mut kind = 0i32;
    let mut len = 0u32;
    // SAFETY: TokenElevationType writes one TOKEN_ELEVATION_TYPE (an
    // i32) to `kind`, whose size is given.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevationType,
            (&mut kind as *mut i32).cast(),
            std::mem::size_of::<i32>() as u32,
            &mut len,
        )
    };
    // SAFETY: closes the token opened above, once.
    unsafe { CloseHandle(token) };
    ok != 0 && (kind == TokenElevationTypeFull || kind == TokenElevationTypeLimited)
}

/// A VARIANT or a PROPVARIANT as x86_64 lays it out: the type, three
/// reserved words, and 16 bytes of value.
#[repr(C)]
struct Variant {
    vt: u16,
    reserved: [u16; 3],
    data: [u64; 2],
}

const _: () = assert!(std::mem::size_of::<Variant>() == 24);

const VT_EMPTY: u16 = 0;
const VT_NULL: u16 = 1;
const VT_I2: u16 = 2;
const VT_I4: u16 = 3;
const VT_BSTR: u16 = 8;
const VT_BOOL: u16 = 11;
const VT_UI1: u16 = 17;
const VT_UI2: u16 = 18;
const VT_UI4: u16 = 19;
const VT_INT: u16 = 22;
const VT_UINT: u16 = 23;
const VT_ARRAY: u16 = 0x2000;

impl Variant {
    fn empty() -> Variant {
        Variant {
            vt: VT_EMPTY,
            reserved: [0; 3],
            data: [0; 2],
        }
    }

    /// An integer value, as WMI hands `uint32` and the shell `Int32`.
    fn int(&self) -> Option<u32> {
        let low = self.data[0];
        match self.vt {
            VT_I4 | VT_UI4 | VT_INT | VT_UINT => Some(low as u32),
            VT_I2 | VT_UI2 => Some(u32::from(low as u16)),
            VT_UI1 => Some(u32::from(low as u8)),
            VT_BOOL => Some(u32::from(low as u16 != 0)),
            _ => None,
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        if self.vt == VT_BSTR && self.data[0] != 0 {
            let b = self.data[0] as *mut u16;
            // SAFETY: a VT_BSTR variant this module received owns a BSTR
            // of SysStringLen units; it is wiped before VariantClear
            // frees it, because a recovery password arrives in one.
            unsafe {
                let n = SysStringLen(b) as usize;
                ptr::write_bytes(b, 0, n);
            }
        }
        // SAFETY: `self` is a VARIANT this module set or received; the
        // call frees what it holds and sets it empty.
        unsafe {
            VariantClear((self as *mut Variant).cast());
        }
    }
}

use windows_sys::Win32::Foundation::{SysAllocString, SysFreeString, SysStringLen};
use windows_sys::Win32::System::Variant::VariantClear;

/// A BSTR this module made, freed when dropped.
struct Bstr(*const u16);

impl Bstr {
    fn new(s: &str) -> Bstr {
        let w = wide(s);
        // SAFETY: `w` is NUL-terminated; the copy is freed on drop.
        Bstr(unsafe { SysAllocString(w.as_ptr()) })
    }
}

impl Drop for Bstr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: made by SysAllocString above, freed once.
            unsafe { SysFreeString(self.0) };
        }
    }
}

/// UTF-16 `units` as a string wiped when dropped, built in a buffer of
/// its final size, so no copy of it is left behind as it grows.
fn wiped_text(units: &[u16]) -> Zeroizing<String> {
    let mut s = Zeroizing::new(String::with_capacity(units.len() * 3));
    for c in char::decode_utf16(units.iter().copied()) {
        s.push(c.unwrap_or(char::REPLACEMENT_CHARACTER));
    }
    s
}

/// A BSTR's text.
///
/// # Safety
/// `b` is null or a live BSTR.
unsafe fn bstr_text(b: *const u16) -> Zeroizing<String> {
    if b.is_null() {
        return Zeroizing::new(String::new());
    }
    // SAFETY: a live BSTR holds SysStringLen units (the caller's
    // promise).
    unsafe {
        let n = SysStringLen(b) as usize;
        wiped_text(std::slice::from_raw_parts(b, n))
    }
}

use zeroize::Zeroizing;

/// A COM interface pointer this module holds one reference to, released
/// when dropped.
struct Com(*mut c_void);

impl Com {
    /// The method at `slot` of its vtable.
    ///
    /// # Safety
    /// The pointer is a live interface whose vtable has at least
    /// `slot + 1` entries.
    unsafe fn slot(&self, slot: usize) -> *const c_void {
        // SAFETY: a COM object starts with a pointer to its vtable, an
        // array of function pointers (the caller's promise).
        unsafe {
            let vtbl = *(self.0 as *const *const *const c_void);
            *vtbl.add(slot)
        }
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }
        type Release = unsafe extern "system" fn(*mut c_void) -> u32;
        // SAFETY: IUnknown::Release is slot 2 of every interface; the
        // reference this holds is given back once.
        unsafe {
            let release: Release = std::mem::transmute(self.slot(2));
            release(self.0);
        }
    }
}

type Hresult = windows_sys::core::HRESULT;

/// COM, started on this thread, and stopped when dropped.
struct ComThread(bool);

impl ComThread {
    fn start(model: i32) -> ComThread {
        use windows_sys::Win32::System::Com::CoInitializeEx;
        // SAFETY: starts COM on this thread; the reserved pointer is null.
        let hr = unsafe { CoInitializeEx(ptr::null(), model as _) };
        ComThread(hr >= 0)
    }
}

impl Drop for ComThread {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: matches the CoInitializeEx that succeeded above.
            unsafe { windows_sys::Win32::System::Com::CoUninitialize() };
        }
    }
}

/// `IID_IWbemLocator`.
const IID_IWBEM_LOCATOR: GUID = GUID::from_u128(0xdc12a687_737f_11cf_884d_00aa004b2e24);
/// `IID_IPropertyStore`.
const IID_IPROPERTY_STORE: GUID = GUID::from_u128(0x886d8eeb_8cf2_4446_8d02_cdba1dbdcf99);

/// The shell's `System.Volume.BitLockerProtection` for `drive` (`C:`),
/// read without elevation (§6.2); `None` when it cannot be read. Read
/// on a thread of its own, so COM there never meets the window's.
pub fn bitlocker_protection(drive: &str) -> Option<i32> {
    let path = format!("{drive}\\");
    std::thread::spawn(move || {
        use windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED;
        use windows_sys::Win32::UI::Shell::PropertiesSystem::{
            GPS_DEFAULT, PROPERTYKEY, PSGetPropertyKeyFromName, SHGetPropertyStoreFromParsingName,
        };
        let _com = ComThread::start(COINIT_APARTMENTTHREADED);
        let name = wide("System.Volume.BitLockerProtection");
        // SAFETY: an all-zero PROPERTYKEY is a valid empty one.
        let mut key: PROPERTYKEY = unsafe { std::mem::zeroed() };
        // SAFETY: `name` is NUL-terminated; the key is written.
        if unsafe { PSGetPropertyKeyFromName(name.as_ptr(), &mut key) } < 0 {
            return None;
        }
        let p = wide(&path);
        let mut store: *mut c_void = ptr::null_mut();
        // SAFETY: `p` is NUL-terminated; no bind context; the store's
        // pointer is written on success and released by `Com`.
        let hr = unsafe {
            SHGetPropertyStoreFromParsingName(
                p.as_ptr(),
                ptr::null_mut(),
                GPS_DEFAULT,
                &IID_IPROPERTY_STORE,
                &mut store,
            )
        };
        if hr < 0 || store.is_null() {
            return None;
        }
        let store = Com(store);
        type GetValue =
            unsafe extern "system" fn(*mut c_void, *const PROPERTYKEY, *mut Variant) -> Hresult;
        let mut value = Variant::empty();
        // SAFETY: IPropertyStore::GetValue is slot 5 (IUnknown's three,
        // GetCount, GetAt); `key` and `value` are live; the PROPVARIANT
        // is cleared by its drop.
        let hr = unsafe {
            let get: GetValue = std::mem::transmute(store.slot(5));
            get(store.0, &key, &mut value)
        };
        if hr < 0 {
            return None;
        }
        value.int().map(|v| v as i32)
    })
    .join()
    .ok()
    .flatten()
}

/// A connection to WMI's BitLocker namespace, as the elevated copy makes
/// it: COM on this thread, packet privacy (the namespace requires it),
/// the class `Win32_EncryptableVolume`, and each volume's path once it
/// is listed.
pub struct WmiSession {
    class: Com,
    services: Com,
    paths: Vec<(String, String)>,
    _com: ComThread,
}

use super::bitlocker::{self, Failure, Outputs, Value, Volume};

fn failed(call: &str, hr: Hresult) -> Failure {
    Failure {
        call: call.to_string(),
        code: hr as u32,
    }
}

/// Sets packet privacy and impersonation on a WMI proxy.
fn blanket(proxy: &Com) {
    use windows_sys::Win32::System::Com::{
        CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL_PKT_PRIVACY, RPC_C_IMP_LEVEL_IMPERSONATE,
    };
    // RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE.
    const WINNT: u32 = 10;
    const NONE: u32 = 0;
    // SAFETY: `proxy` is a live interface; the other pointers may be
    // null.
    unsafe {
        CoSetProxyBlanket(
            proxy.0,
            WINNT,
            NONE,
            ptr::null(),
            RPC_C_AUTHN_LEVEL_PKT_PRIVACY,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            ptr::null(),
            EOAC_NONE as u32,
        );
    }
}

/// `IWbemClassObject::Get`: slot 4.
type ObjGet = unsafe extern "system" fn(
    *mut c_void,
    *const u16,
    i32,
    *mut Variant,
    *mut i32,
    *mut i32,
) -> Hresult;
/// `IWbemClassObject::Put`: slot 5.
type ObjPut =
    unsafe extern "system" fn(*mut c_void, *const u16, i32, *const Variant, i32) -> Hresult;
/// `IWbemClassObject::SpawnInstance`: slot 15.
type SpawnInstance = unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> Hresult;
/// `IWbemClassObject::GetMethod`: slot 19.
type GetMethod = unsafe extern "system" fn(
    *mut c_void,
    *const u16,
    i32,
    *mut *mut c_void,
    *mut *mut c_void,
) -> Hresult;

/// A property of a WMI object, or `None` when it is not there.
fn property(obj: &Com, name: &str) -> Option<Variant> {
    let n = wide(name);
    let mut v = Variant::empty();
    // SAFETY: IWbemClassObject::Get is slot 4; `n` is NUL-terminated;
    // the type and flavour outputs may be null; `v` is cleared on drop.
    let hr = unsafe {
        let get: ObjGet = std::mem::transmute(obj.slot(4));
        get(
            obj.0,
            n.as_ptr(),
            0,
            &mut v,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    (hr >= 0 && v.vt != VT_NULL && v.vt != VT_EMPTY).then_some(v)
}

fn property_text(obj: &Com, name: &str) -> Option<Zeroizing<String>> {
    let v = property(obj, name)?;
    // SAFETY: a VT_BSTR variant holds a live BSTR until it is dropped,
    // after the text is copied.
    (v.vt == VT_BSTR).then(|| unsafe { bstr_text(v.data[0] as *const u16) })
}

/// A string array property (`VT_ARRAY | VT_BSTR`).
fn property_texts(obj: &Com, name: &str) -> Option<Vec<String>> {
    use windows_sys::Win32::System::Ole::{
        SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound,
    };
    let v = property(obj, name)?;
    if v.vt != (VT_ARRAY | VT_BSTR) {
        return None;
    }
    let sa = v.data[0] as *const windows_sys::Win32::System::Com::SAFEARRAY;
    let (mut lo, mut hi) = (0i32, -1i32);
    // SAFETY: `sa` is the variant's live one-dimensional SAFEARRAY; the
    // bounds are written.
    unsafe {
        if SafeArrayGetLBound(sa, 1, &mut lo) < 0 || SafeArrayGetUBound(sa, 1, &mut hi) < 0 {
            return None;
        }
    }
    let mut out = Vec::new();
    for i in lo..=hi {
        let mut b: *const u16 = ptr::null();
        // SAFETY: `i` is within the bounds read above; for a BSTR array
        // the call writes a copy of the element, freed below.
        unsafe {
            if SafeArrayGetElement(sa, &i, (&mut b as *mut *const u16).cast()) < 0 {
                return None;
            }
            out.push(bstr_text(b).to_string());
            SysFreeString(b);
        }
    }
    Some(out)
}

impl WmiSession {
    /// Connects, as the elevated copy: COM, its security, the locator,
    /// the namespace, the class.
    pub fn connect() -> Result<WmiSession, Failure> {
        use windows_sys::Win32::System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeSecurity,
            EOAC_NONE, RPC_C_AUTHN_LEVEL_PKT_PRIVACY, RPC_C_IMP_LEVEL_IMPERSONATE,
        };
        use windows_sys::Win32::System::Wmi::WbemLocator;
        let com = ComThread::start(COINIT_MULTITHREADED);
        // SAFETY: process-wide COM security for this copy, which does
        // nothing else; a second call fails harmlessly (RPC_E_TOO_LATE).
        unsafe {
            CoInitializeSecurity(
                ptr::null_mut(),
                -1,
                ptr::null(),
                ptr::null(),
                RPC_C_AUTHN_LEVEL_PKT_PRIVACY,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                ptr::null(),
                EOAC_NONE as u32,
                ptr::null(),
            );
        }
        let mut loc: *mut c_void = ptr::null_mut();
        // SAFETY: creates the in-process locator; its pointer is written
        // on success and released by `Com`.
        let hr = unsafe {
            CoCreateInstance(
                &WbemLocator,
                ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_IWBEM_LOCATOR,
                &mut loc,
            )
        };
        if hr < 0 || loc.is_null() {
            return Err(failed("CoCreateInstance", hr));
        }
        let loc = Com(loc);
        /// `IWbemLocator::ConnectServer`: slot 3.
        type Connect = unsafe extern "system" fn(
            *mut c_void,
            *const u16,
            *const u16,
            *const u16,
            *const u16,
            i32,
            *const u16,
            *mut c_void,
            *mut *mut c_void,
        ) -> Hresult;
        let ns = Bstr::new(bitlocker::NAMESPACE);
        let mut services: *mut c_void = ptr::null_mut();
        // SAFETY: ConnectServer is slot 3 of IWbemLocator; `ns` is a live
        // BSTR; user, password, locale, authority and context are null
        // (this account, the default locale); the services pointer is
        // written on success and released by `Com`.
        let hr = unsafe {
            let connect: Connect = std::mem::transmute(loc.slot(3));
            connect(
                loc.0,
                ns.0,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0,
                ptr::null(),
                ptr::null_mut(),
                &mut services,
            )
        };
        if hr < 0 || services.is_null() {
            return Err(failed("ConnectServer", hr));
        }
        let services = Com(services);
        blanket(&services);
        /// `IWbemServices::GetObject`: slot 6.
        type GetObject = unsafe extern "system" fn(
            *mut c_void,
            *const u16,
            i32,
            *mut c_void,
            *mut *mut c_void,
            *mut *mut c_void,
        ) -> Hresult;
        let name = Bstr::new(bitlocker::CLASS);
        let mut class: *mut c_void = ptr::null_mut();
        // SAFETY: GetObject is slot 6 of IWbemServices; `name` is a live
        // BSTR; no context, no call result; the class pointer is written
        // on success and released by `Com`.
        let hr = unsafe {
            let get: GetObject = std::mem::transmute(services.slot(6));
            get(
                services.0,
                name.0,
                0,
                ptr::null_mut(),
                &mut class,
                ptr::null_mut(),
            )
        };
        if hr < 0 || class.is_null() {
            return Err(failed("GetObject", hr));
        }
        Ok(WmiSession {
            class: Com(class),
            services,
            paths: Vec::new(),
            _com: com,
        })
    }
}

impl bitlocker::Wmi for WmiSession {
    fn volumes(&mut self) -> Result<Vec<Volume>, Failure> {
        use windows_sys::Win32::System::Wmi::{
            WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_INFINITE,
        };
        /// `IWbemServices::ExecQuery`: slot 20.
        type ExecQuery = unsafe extern "system" fn(
            *mut c_void,
            *const u16,
            *const u16,
            i32,
            *mut c_void,
            *mut *mut c_void,
        ) -> Hresult;
        /// `IEnumWbemClassObject::Next`: slot 4.
        type Next =
            unsafe extern "system" fn(*mut c_void, i32, u32, *mut *mut c_void, *mut u32) -> Hresult;
        let (lang, query) = (
            Bstr::new("WQL"),
            Bstr::new(&format!("SELECT * FROM {}", bitlocker::CLASS)),
        );
        let mut en: *mut c_void = ptr::null_mut();
        // SAFETY: ExecQuery is slot 20 of IWbemServices; both BSTRs are
        // live; no context; the enumerator is written on success and
        // released by `Com`.
        let hr = unsafe {
            let exec: ExecQuery = std::mem::transmute(self.services.slot(20));
            exec(
                self.services.0,
                lang.0,
                query.0,
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                ptr::null_mut(),
                &mut en,
            )
        };
        if hr < 0 || en.is_null() {
            return Err(failed("ExecQuery", hr));
        }
        let en = Com(en);
        blanket(&en);
        let mut out = Vec::new();
        self.paths.clear();
        loop {
            let mut obj: *mut c_void = ptr::null_mut();
            let mut got = 0u32;
            // SAFETY: Next is slot 4 of IEnumWbemClassObject; one object
            // is asked for, its pointer written to `obj` when `got` is 1,
            // and released by `Com`.
            let hr = unsafe {
                let next: Next = std::mem::transmute(en.slot(4));
                next(en.0, WBEM_INFINITE, 1, &mut obj, &mut got)
            };
            if hr < 0 {
                return Err(failed("IEnumWbemClassObject::Next", hr));
            }
            if got == 0 || obj.is_null() {
                break;
            }
            let obj = Com(obj);
            let Some(device) = property_text(&obj, "DeviceID") else {
                continue;
            };
            let Some(path) = property_text(&obj, "__RELPATH") else {
                continue;
            };
            let int = |n: &str| property(&obj, n).and_then(|v| v.int());
            out.push(Volume {
                device: device.to_string(),
                letter: property_text(&obj, "DriveLetter")
                    .map(|l| l.to_string())
                    .unwrap_or_default(),
                kind: int("VolumeType"),
                protection: int("ProtectionStatus").unwrap_or(2),
            });
            self.paths.push((device.to_string(), path.to_string()));
        }
        Ok(out)
    }

    fn call(
        &mut self,
        device: &str,
        method: &str,
        args: &[(&str, Value)],
        outs: &[&str],
    ) -> Result<Outputs, Failure> {
        // WBEM_E_NOT_FOUND, for a volume not listed.
        const NOT_FOUND: i32 = 0x8004_1002u32 as i32;
        let path = self
            .paths
            .iter()
            .find(|(d, _)| d == device)
            .map(|(_, p)| p.clone())
            .ok_or_else(|| failed(method, NOT_FOUND))?;
        let m = wide(method);
        let (mut in_sig, mut out_sig): (*mut c_void, *mut c_void) =
            (ptr::null_mut(), ptr::null_mut());
        // SAFETY: GetMethod is slot 19 of IWbemClassObject; `m` is
        // NUL-terminated; both signatures are written (either may be
        // null for a method without one) and released by `Com`.
        let hr = unsafe {
            let get: GetMethod = std::mem::transmute(self.class.slot(19));
            get(self.class.0, m.as_ptr(), 0, &mut in_sig, &mut out_sig)
        };
        let (in_sig, _out_sig) = (Com(in_sig), Com(out_sig));
        if hr < 0 {
            return Err(failed(method, hr));
        }
        let input = if in_sig.0.is_null() {
            if !args.is_empty() {
                return Err(failed(method, NOT_FOUND));
            }
            Com(ptr::null_mut())
        } else {
            let mut inst: *mut c_void = ptr::null_mut();
            // SAFETY: SpawnInstance is slot 15 of IWbemClassObject; the
            // instance is written on success and released by `Com`.
            let hr = unsafe {
                let spawn: SpawnInstance = std::mem::transmute(in_sig.slot(15));
                spawn(in_sig.0, 0, &mut inst)
            };
            if hr < 0 || inst.is_null() {
                return Err(failed(method, hr));
            }
            let inst = Com(inst);
            for (name, value) in args {
                let mut v = Variant::empty();
                match value {
                    Value::U32(n) => {
                        v.vt = VT_I4;
                        v.data[0] = u64::from(*n);
                    }
                    Value::Str(s) => {
                        let w = wide(s);
                        v.vt = VT_BSTR;
                        // SAFETY: `w` is NUL-terminated; the BSTR made is
                        // freed with `v`.
                        v.data[0] = unsafe { SysAllocString(w.as_ptr()) } as u64;
                    }
                    Value::Strs(_) => return Err(failed(method, NOT_FOUND)),
                }
                let n = wide(name);
                // SAFETY: Put is slot 5 of IWbemClassObject; `n` is
                // NUL-terminated; `v` is live for the call, which copies
                // it; type 0 takes the property's own.
                let hr = unsafe {
                    let put: ObjPut = std::mem::transmute(inst.slot(5));
                    put(inst.0, n.as_ptr(), 0, &v, 0)
                };
                if hr < 0 {
                    return Err(failed(method, hr));
                }
            }
            inst
        };
        /// `IWbemServices::ExecMethod`: slot 24.
        type ExecMethod = unsafe extern "system" fn(
            *mut c_void,
            *const u16,
            *const u16,
            i32,
            *mut c_void,
            *mut c_void,
            *mut *mut c_void,
            *mut *mut c_void,
        ) -> Hresult;
        let (p, mb) = (Bstr::new(&path), Bstr::new(method));
        let mut out: *mut c_void = ptr::null_mut();
        // SAFETY: ExecMethod is slot 24 of IWbemServices; both BSTRs are
        // live; the input instance may be null; no context, no call
        // result; the output object is written on success and released
        // by `Com`.
        let hr = unsafe {
            let exec: ExecMethod = std::mem::transmute(self.services.slot(24));
            exec(
                self.services.0,
                p.0,
                mb.0,
                0,
                ptr::null_mut(),
                input.0,
                &mut out,
                ptr::null_mut(),
            )
        };
        if hr < 0 || out.is_null() {
            return Err(failed(method, hr));
        }
        let out = Com(out);
        let ret = property(&out, "ReturnValue")
            .and_then(|v| v.int())
            .ok_or_else(|| failed(method, NOT_FOUND))?;
        let mut values = Vec::new();
        for name in outs {
            let Some(v) = property(&out, name) else {
                continue;
            };
            let value = if v.vt == VT_BSTR {
                drop(v);
                property_text(&out, name).map(Value::Str)
            } else if v.vt == (VT_ARRAY | VT_BSTR) {
                drop(v);
                property_texts(&out, name).map(Value::Strs)
            } else {
                v.int().map(Value::U32)
            };
            if let Some(value) = value {
                values.push((name.to_string(), value));
            }
        }
        Ok(Outputs { ret, values })
    }
}

// ---------------------------------------------------------------------
// What is attached that is of concern (docs/HUB.md §3.1a)
// ---------------------------------------------------------------------

use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA, SPDRP_COMPATIBLEIDS, SPDRP_DEVICEDESC,
    SPDRP_HARDWAREID, SPDRP_MFG, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsW, SetupDiGetDevicePropertyW, SetupDiGetDeviceRegistryPropertyW,
};
use windows_sys::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows_sys::Win32::Devices::Properties::{DEVPKEY_Device_BusReportedDeviceDesc, DEVPROPTYPE};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION};

use super::devices::{Output, RawUsb};

/// Each active display output: its technology, the monitor's friendly
/// name and its device path, from `QueryDisplayConfig` and
/// `DisplayConfigGetDeviceInfo`. The EDID is the caller's to read.
pub fn display_outputs() -> Vec<Output> {
    let (mut np, mut nm) = (0u32, 0u32);
    // SAFETY: writes two counts.
    if unsafe { GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) }
        != ERROR_SUCCESS
    {
        return Vec::new();
    }
    // SAFETY: both structures are plain data, valid when zeroed.
    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = vec![unsafe { std::mem::zeroed() }; np as usize];
    // SAFETY: as above.
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = vec![unsafe { std::mem::zeroed() }; nm as usize];
    // SAFETY: the arrays hold the counts given; the call writes at most
    // that many and the counts it wrote.
    let e = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut np,
            paths.as_mut_ptr(),
            &mut nm,
            modes.as_mut_ptr(),
            ptr::null_mut(),
        )
    };
    if e != ERROR_SUCCESS {
        return Vec::new();
    }
    paths.truncate(np as usize);
    paths
        .iter()
        .filter_map(|p| {
            // SAFETY: plain data, valid when zeroed.
            let mut name: DISPLAYCONFIG_TARGET_DEVICE_NAME = unsafe { std::mem::zeroed() };
            name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            name.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            name.header.adapterId = p.targetInfo.adapterId;
            name.header.id = p.targetInfo.id;
            // SAFETY: the header says the packet's type and size.
            if unsafe { DisplayConfigGetDeviceInfo(&mut name.header) } != 0 {
                return None;
            }
            Some(Output {
                technology: name.outputTechnology as u32,
                name: narrow(&name.monitorFriendlyDeviceName),
                path: narrow(&name.monitorDevicePath),
                edid: None,
            })
        })
        .collect()
}

/// `GetSystemMetrics(SM_REMOTESESSION)`: this session is a remote one.
pub fn remote_session() -> bool {
    // SAFETY: no pointers.
    unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
}

/// A `REG_MULTI_SZ` or `REG_SZ` registry property of a device.
fn device_property(set: isize, data: &SP_DEVINFO_DATA, property: u32) -> Option<Vec<u8>> {
    let mut need = 0u32;
    // SAFETY: a null buffer of size 0 asks only for the size.
    unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            set,
            data,
            property,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            &mut need,
        )
    };
    if need == 0 {
        return None;
    }
    let mut buf = vec![0u8; need as usize];
    // SAFETY: `buf` holds `need` bytes.
    let ok = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            set,
            data,
            property,
            ptr::null_mut(),
            buf.as_mut_ptr(),
            need,
            &mut need,
        )
    };
    (ok != 0).then_some(buf)
}

/// The device's own product string, `DEVPKEY_Device_BusReportedDeviceDesc`.
fn bus_description(set: isize, data: &SP_DEVINFO_DATA) -> String {
    let mut kind: DEVPROPTYPE = 0;
    let mut buf = vec![0u8; 512];
    let mut need = 0u32;
    // SAFETY: `buf` holds the size given; the key is a static.
    let ok = unsafe {
        SetupDiGetDevicePropertyW(
            set,
            data,
            &DEVPKEY_Device_BusReportedDeviceDesc,
            &mut kind,
            buf.as_mut_ptr(),
            buf.len() as u32,
            &mut need,
            0,
        )
    };
    if ok == 0 {
        return String::new();
    }
    buf.truncate(need as usize);
    super::registry::sz(&buf).unwrap_or_default()
}

/// Every present device under the `USB` enumerator, through SetupAPI.
pub fn usb_devices() -> Vec<RawUsb> {
    let usb = wide("USB");
    // SAFETY: the enumerator is NUL-terminated; no class or window.
    let set = unsafe {
        SetupDiGetClassDevsW(
            ptr::null(),
            usb.as_ptr(),
            0,
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )
    };
    if set == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0.. {
        // SAFETY: plain data, valid when zeroed.
        let mut data: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
        // SAFETY: `data` says its size.
        if unsafe { SetupDiEnumDeviceInfo(set, i, &mut data) } == 0 {
            break;
        }
        let multi = |p| {
            device_property(set, &data, p)
                .map(|b| super::registry::multi_sz(&b))
                .unwrap_or_default()
        };
        let single = |p| {
            device_property(set, &data, p)
                .and_then(|b| super::registry::sz(&b))
                .unwrap_or_default()
        };
        out.push(RawUsb {
            hardware_ids: multi(SPDRP_HARDWAREID),
            compatible_ids: multi(SPDRP_COMPATIBLEIDS),
            bus_description: bus_description(set, &data),
            description: single(SPDRP_DEVICEDESC),
            maker: single(SPDRP_MFG),
        });
    }
    // SAFETY: the set came from SetupDiGetClassDevsW and is freed once.
    unsafe { SetupDiDestroyDeviceInfoList(set) };
    out
}

/// Whether Windows runs as a guest of a hypervisor, from `CPUID`
/// (`devices::guest`). Not x86_64: not known, so not a guest.
pub fn hypervisor_guest() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::__cpuid;
        // SAFETY: CPUID exists on every x86_64 processor.
        #[allow(unused_unsafe)]
        let (l1, hv, feat) = unsafe { (__cpuid(1), __cpuid(0x4000_0000), __cpuid(0x4000_0003)) };
        let mut vendor = [0u8; 12];
        vendor[..4].copy_from_slice(&hv.ebx.to_le_bytes());
        vendor[4..8].copy_from_slice(&hv.ecx.to_le_bytes());
        vendor[8..].copy_from_slice(&hv.edx.to_le_bytes());
        super::devices::guest(l1.ecx, &vendor, feat.ebx)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}
