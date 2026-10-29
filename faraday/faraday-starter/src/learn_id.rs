//! The Starter's guides, each by its Learn page.

use crate::api::{Computer, Os};
use crate::macs;

/// A guide, by its Learn page (§7.2, `docs/learn/faraday/boot/`). A page
/// that does not exist shows its title as a line and opens nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnId {
    /// Starting a Windows computer from the stick.
    StartWindows,
    /// Starting a Linux computer from the stick.
    StartLinux,
    /// When the stick does not start.
    NoStart,
    /// Replacing Faraday on a stick.
    KeepData,
    /// A maker family's page, by its id in `faraday_learn::MAKERS`
    /// (§7.6).
    Maker(&'static str),
    /// What Secure Boot and BitLocker are (§3.1b).
    SecureBootBitLocker,
    /// Screens and devices that can record (§3.1a).
    ScreensThatRecord,
    /// What else runs on this computer: the management processors
    /// under the operating system.
    WhatElseRuns,
    /// What the Starter checks of what it writes (CRYPTO-LEARN §8.4).
    CheckingWritten,
    /// About the boot entry (§3.1c).
    BootEntry,
    /// Starting an Intel Mac from the stick (`docs/MAC.md` §3.1–§3.2).
    MacIntel,
}

impl LearnId {
    /// The page's id among the guides (`faraday_guides::BOOT`), which
    /// is also its name on the website.
    pub fn id(self) -> &'static str {
        match self {
            LearnId::StartWindows => "windows",
            LearnId::StartLinux => "linux",
            LearnId::NoStart => "not-starting",
            LearnId::KeepData => "replace-faraday",
            LearnId::Maker(id) => id,
            LearnId::SecureBootBitLocker => "secure-boot-bitlocker",
            LearnId::ScreensThatRecord => "screens-that-record",
            LearnId::WhatElseRuns => "what-else-runs",
            LearnId::CheckingWritten => "checking-written",
            LearnId::BootEntry => "boot-entry",
            LearnId::MacIntel => "mac-intel",
        }
    }

    /// The page.
    pub fn page(self) -> Option<&'static faraday_learn::Page> {
        faraday_guides::page(self.id())
    }

    /// Its address on the website (§7.4).
    pub fn address(self) -> String {
        faraday_learn::guide_address(self.id())
    }

    /// The guide for this computer (§3.4): on a Mac, whatever system it
    /// runs, the Mac guide; otherwise its maker's page where there is one
    /// (§7.6), and the general page for its operating system where there
    /// is not.
    pub fn for_computer(c: &Computer) -> LearnId {
        if macs::is_apple(&c.maker) {
            return LearnId::MacIntel;
        }
        let general = match c.os {
            Os::Windows => LearnId::StartWindows,
            // macOS on a computer whose maker is not Apple: none is
            // expected, and Linux's page is the nearer of the two.
            Os::Linux | Os::MacOs => LearnId::StartLinux,
        };
        match faraday_learn::guide_id(&c.maker, &c.family, &c.model, general.id()) {
            id if id == general.id() => general,
            id => LearnId::Maker(id),
        }
    }

    pub(crate) const ALL: [LearnId; 10] = [
        LearnId::StartWindows,
        LearnId::StartLinux,
        LearnId::NoStart,
        LearnId::KeepData,
        LearnId::CheckingWritten,
        LearnId::SecureBootBitLocker,
        LearnId::ScreensThatRecord,
        LearnId::BootEntry,
        LearnId::MacIntel,
        LearnId::WhatElseRuns,
    ];
}
