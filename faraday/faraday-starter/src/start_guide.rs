//! The Starter's start guides.

use crate::api::{Computer, Os};
use crate::macs;

/// A start guide, by its page (`docs/guide/start/`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartGuide {
    /// Starting a Windows computer from the stick.
    StartWindows,
    /// Starting a Linux computer from the stick.
    StartLinux,
    /// When the stick does not start.
    NoStart,
    /// Replacing Faraday on a stick.
    KeepData,
    /// A maker family's page, by its id in `faraday_guide::MAKERS`
    /// (§7.6).
    Maker(&'static str),
    /// What Secure Boot and BitLocker are (§3.1b).
    SecureBootBitLocker,
    /// Screens and devices that can record (§3.1a).
    ScreensThatRecord,
    /// What else runs on this computer: the management processors
    /// under the operating system.
    WhatElseRuns,
    /// About the boot entry (§3.1c).
    BootEntry,
    /// Starting an Intel Mac from the stick (`docs/MAC.md` §3.1–§3.2).
    MacIntel,
}

impl StartGuide {
    /// The page's id among the guides (`faraday_start::BOOT`), which
    /// is also its name on the website.
    pub fn id(self) -> &'static str {
        match self {
            StartGuide::StartWindows => "windows",
            StartGuide::StartLinux => "linux",
            StartGuide::NoStart => "not-starting",
            StartGuide::KeepData => "replace-faraday",
            StartGuide::Maker(id) => id,
            StartGuide::SecureBootBitLocker => "secure-boot-bitlocker",
            StartGuide::ScreensThatRecord => "screens-that-record",
            StartGuide::WhatElseRuns => "what-else-runs",
            StartGuide::BootEntry => "boot-entry",
            StartGuide::MacIntel => "mac-intel",
        }
    }

    /// The page.
    pub fn page(self) -> Option<&'static faraday_guide::Page> {
        faraday_start::page(self.id())
    }

    /// Its address on the website (§7.4).
    pub fn address(self) -> String {
        faraday_guide::guide_address(self.id())
    }

    /// The guide for this computer (§3.4): on a Mac, whatever system it
    /// runs, the Mac guide; otherwise its maker's page where there is one
    /// (§7.6), and the general page for its operating system where there
    /// is not.
    pub fn for_computer(c: &Computer) -> StartGuide {
        if macs::is_apple(&c.maker) {
            return StartGuide::MacIntel;
        }
        let general = match c.os {
            Os::Windows => StartGuide::StartWindows,
            // macOS on a computer whose maker is not Apple: none is
            // expected, and Linux's page is the nearer of the two.
            Os::Linux | Os::MacOs => StartGuide::StartLinux,
        };
        match faraday_guide::guide_id(&c.maker, &c.family, &c.model, general.id()) {
            id if id == general.id() => general,
            id => StartGuide::Maker(id),
        }
    }

    pub(crate) const ALL: [StartGuide; 9] = [
        StartGuide::StartWindows,
        StartGuide::StartLinux,
        StartGuide::NoStart,
        StartGuide::KeepData,
        StartGuide::SecureBootBitLocker,
        StartGuide::ScreensThatRecord,
        StartGuide::BootEntry,
        StartGuide::MacIntel,
        StartGuide::WhatElseRuns,
    ];
}
