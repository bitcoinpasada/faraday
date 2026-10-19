//! Whether the stick started with Secure Boot on. The stick shell reads
//! the kernel's boot parameters once at start and hands the app the
//! answer; the power-off sheet reminds when it was off (`docs/STARTER.md` §7.5).

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

impl Faraday {
    /// Whether the power-off sheet carries the Secure Boot reminder: on the
    /// PC stick only (not the Pi, not Faraday Desktop, which never reads the
    /// state and shows its stick pills, not a Mac, `docs/MAC.md` §3.2), and
    /// unless Secure Boot was read as on.
    pub fn secure_boot_reminder(&self) -> bool {
        self.medium == Medium::Stick
            && !self.shell_sticks
            && !self.maker.starts_with("Apple")
            && self.secure_boot != SecureBootAtStart::On
    }

    /// This computer's BIOS key from its maker's page, where the sheet
    /// shows the Secure Boot reminder and a maker page matches it
    /// (`docs/FLOWS.md`, Restart).
    pub fn bios_key(&self) -> Option<&'static str> {
        if !self.secure_boot_reminder() {
            return None;
        }
        faraday_learn::maker(&self.maker, &self.maker_family, &self.maker_model).map(|m| m.bios_key)
    }

    /// The address of the guide for this computer, when the power-off
    /// sheet shows its code: the same the Starter shows for the same
    /// computer (`docs/STARTER.md` §7.5).
    pub fn secure_boot_guide(&self) -> Option<String> {
        self.secure_boot_reminder().then(|| {
            let id = crate::learn::guide_id(
                &self.maker,
                &self.maker_family,
                &self.maker_model,
                "windows",
            );
            crate::learn::guide_address(id)
        })
    }
}
