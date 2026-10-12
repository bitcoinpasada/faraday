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
