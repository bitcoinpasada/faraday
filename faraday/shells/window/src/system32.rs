//! Libraries loaded from System32 only, on Windows.
//!
//! The `.exe`'s load configuration (`faraday/build/windows/build.sh`) covers
//! the libraries the program itself imports. The libraries those load in
//! turn by name (`version.dll`, `userenv.dll`, `cryptbase.dll`) would still
//! be searched for in the program's own folder first, such as Downloads,
//! which holds files from anywhere; this call makes every later load of the
//! process look in System32 only.

/// Makes every library the process loads from now on come from System32.
/// Called as the first statement of a Windows program's `main`, before
/// anything else loads. `Err` carries the Windows error code: the caller
/// stops, because the program must not go on with the search path open.
/// Every Windows the programs run on (8 and later) has the call, so a
/// failure is not an old system to tolerate. Off Windows it does nothing.
pub fn system32_only() -> Result<(), u32> {
    #[cfg(windows)]
    {
        windows::set()
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::LibraryLoader::{
        LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
    };

    pub fn set() -> Result<(), u32> {
        // SAFETY: one call with a constant flag and no pointer; it only sets
        // the process's library search path.
        if unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32) } == 0 {
            // SAFETY: reads the calling thread's last error, nothing else.
            Err(unsafe { GetLastError() })
        } else {
            Ok(())
        }
    }
}
