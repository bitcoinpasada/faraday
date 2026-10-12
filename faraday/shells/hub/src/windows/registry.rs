//! This computer's facts on Windows (`docs/HUB.md` §7.1), from the
//! registry's answers and two calls, with no Win32 call in it: the
//! values are the bytes `RegGetValueW` returns, so the tests feed it
//! recorded ones.

use faraday_hub::api::{Computer, Os};

/// `HKLM\…\BIOS`: `SystemManufacturer`, `SystemProductName`,
/// `SystemFamily`.
pub const BIOS: &str = r"HARDWARE\DESCRIPTION\System\BIOS";
/// `HKLM\…\State`: `UEFISecureBootEnabled`.
pub const SECURE_BOOT: &str = r"SYSTEM\CurrentControlSet\Control\SecureBoot\State";
/// `HKLM\…\Setup`: `SystemPartition`.
pub const SETUP: &str = r"SYSTEM\Setup";
/// `HKLM\…\Memory Management`: `ExistingPageFiles`.
pub const MEMORY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management";

/// `GetFirmwareType`'s `FirmwareTypeUefi`.
pub const FIRMWARE_UEFI: u32 = 2;
/// `IMAGE_FILE_MACHINE_*` values that are ARM.
const ARM_MACHINES: [u16; 3] = [0xaa64, 0x01c4, 0x01c0];

/// A `REG_SZ` value: UTF-16LE, up to its NUL.
pub fn sz(b: &[u8]) -> Option<String> {
    let units: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16(&units).ok()
}

/// A `REG_MULTI_SZ` value: UTF-16LE strings, each ended by a NUL, the
/// list by another.
pub fn multi_sz(b: &[u8]) -> Vec<String> {
    let units: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    units
        .split(|&u| u == 0)
        .take_while(|s| !s.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

/// A `REG_DWORD` value.
pub fn dword(b: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(..4)?.try_into().ok()?))
}

/// What was read, each `None` where the call or the value failed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// `SystemManufacturer`.
    pub manufacturer: Option<Vec<u8>>,
    /// `SystemProductName`.
    pub product: Option<Vec<u8>>,
    /// `SystemFamily`.
    pub family: Option<Vec<u8>>,
    /// `UEFISecureBootEnabled`.
    pub secure_boot: Option<Vec<u8>>,
    /// `GetFirmwareType`.
    pub firmware: Option<u32>,
    /// `IsWow64Process2`'s native machine.
    pub machine: Option<u16>,
}

/// This computer's facts. A computer started in legacy BIOS mode has no
/// Secure Boot; on one started in UEFI mode a value that cannot be read
/// is not known.
pub fn computer(f: &Facts) -> Computer {
    let text = |v: &Option<Vec<u8>>| {
        v.as_deref()
            .and_then(sz)
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let uefi = f.firmware == Some(FIRMWARE_UEFI);
    let secure_boot = if uefi {
        f.secure_boot.as_deref().and_then(dword).map(|v| v == 1)
    } else if f.firmware.is_some() {
        Some(false)
    } else {
        None
    };
    Computer {
        os: Os::Windows,
        maker: text(&f.manufacturer),
        model: text(&f.product),
        family: text(&f.family),
        uefi,
        arm: f.machine.is_some_and(|m| ARM_MACHINES.contains(&m)),
        secure_boot,
        tpm_unlock: false,
    }
}
