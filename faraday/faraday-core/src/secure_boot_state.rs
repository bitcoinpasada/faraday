//! Whether the stick started with Secure Boot on. The stick shell reads
//! the kernel's boot parameters once at start and hands the app the
//! answer; the power-off sheet reminds when it was off (`docs/HUB.md` §7.5).

use crate::{Faraday, Medium};

/// The Secure Boot state the machine started in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SecureBootAtStart {
    /// Not read, unreadable, or not a value this knows.
    #[default]
    Unknown,
    /// Secure Boot was off.
    Off,
    /// Secure Boot was on.
    On,
}

/// Where the byte is in the boot parameters (`struct boot_params`,
/// `secure_boot` in the x86 boot protocol).
const SECURE_BOOT_OFFSET: usize = 0x1ec;

impl SecureBootAtStart {
    /// The state from the contents of `/sys/kernel/boot_params/data`: byte
    /// `0x1ec` is 2 for off and 3 for on; any other value, or a file too
    /// short to hold the byte, is unknown.
    pub fn from_boot_params(data: &[u8]) -> Self {
        match data.get(SECURE_BOOT_OFFSET) {
            Some(2) => Self::Off,
            Some(3) => Self::On,
            _ => Self::Unknown,
        }
    }
}

/// The maker's key for the firmware settings, from its `sys_vendor`
/// string. No maker page exists yet (`docs/HUB.md` D6), so none has a key;
/// the sheet shows one beside the reminder once a page does.
pub fn maker_key(_maker: &str) -> Option<&'static str> {
    None
}

impl Faraday {
    /// Whether the power-off sheet carries the Secure Boot reminder: on the
    /// stick only, and only when it started with Secure Boot off.
    pub fn secure_boot_reminder(&self) -> bool {
        self.medium == Medium::Stick && self.secure_boot == SecureBootAtStart::Off
    }
}
