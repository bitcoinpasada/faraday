//! The Mac table (`docs/MAC.md` §1, §7), held in `macs.tsv`: one row
//! per model identifier, with its marketing name, year, line, chip,
//! whether it has the T2 chip or 32-bit EFI firmware, and how its
//! built-in keyboard is attached.
//! What Home shows and offers on a Mac comes from here, on any system
//! running on it (`docs/HUB.md` §3.1, §7.1).
//!
//! Sources, all read 2026-10-13:
//! - Apple, "Identify your MacBook Pro model" (support.apple.com/108052),
//!   "Identify your MacBook Air model" (102869), "Identify your MacBook
//!   model" (103257), "Identify your iMac model" (108054), "Identify your
//!   Mac mini model" (102852), "Identify your Mac Pro model" (102887),
//!   "Identify your Mac Studio model" (102231): the identifiers and
//!   names. Where another source disagrees, Apple's identifier is kept.
//! - Apple, "Mac computers that have the Apple T2 Security Chip"
//!   (support.apple.com/103265): the T2 column.
//! - Apple's per-model "Manuals and Downloads" pages
//!   (support.apple.com/docs/mac) and EveryMac's pages by model
//!   identifier (everymac.com/systems/by-identifier/), for the 2006 and
//!   2007 Macs the identification pages leave out, and the name of
//!   `iMac5,2`.
//! - "Operating system options for 32-bit EFI Macs" (ctrl-alt-rees.com,
//!   2024-08-13): the 32-bit EFI column; `MacPro2,1`, which that list
//!   leaves out, has the same 32-bit EFI ROM as `MacPro1,1` ("The
//!   Definitive Classic Mac Pro (2006-2012) Upgrade Guide",
//!   blog.greggant.com, 2018-05-07).
//! - The Linux kernel's `drivers/input/keyboard/applespi.c` (its header
//!   comment and `applespi_tp_models[]`): the SPI keyboards.
//! - Asahi Linux, "Device List" (asahilinux.org/docs/hw/devices/
//!   device-list/) and the M1 to M6 feature-support pages: the Apple
//!   silicon chips, and the name of `Mac17,5`, which Apple's pages do
//!   not list yet.

use std::sync::OnceLock;

use crate::api::Computer;

/// A Mac line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line {
    /// MacBook Pro.
    MacBookPro,
    /// MacBook Air.
    MacBookAir,
    /// MacBook (and MacBook Neo).
    MacBook,
    /// iMac.
    IMac,
    /// iMac Pro.
    IMacPro,
    /// Mac mini.
    MacMini,
    /// Mac Pro.
    MacPro,
    /// Mac Studio.
    MacStudio,
    /// Xserve.
    Xserve,
}

/// How a Mac's built-in keyboard is attached (`docs/MAC.md` §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyboard {
    /// USB, inside the case: a generic HID keyboard.
    Usb,
    /// SPI (`applespi`), the 2015 to 2017 laptops.
    Spi,
    /// Through the T2 chip, which Faraday does not carry a driver for.
    T2,
    /// An Apple silicon laptop's.
    AppleSilicon,
    /// None built in: a desktop.
    None,
}

/// One Mac model identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mac {
    /// The model identifier (`MacBookPro14,1`, `Mac14,2`): `sysctl
    /// hw.model` on macOS, DMI's `product_name` on an Intel Mac.
    pub id: &'static str,
    /// Its marketing names, as Apple's pages give them, with " / "
    /// between them when models of several years share the identifier.
    pub name: &'static str,
    /// The year of its first model.
    pub year: u16,
    /// Its line.
    pub line: Line,
    /// Its Apple chip ("M1 Pro", "A18 Pro"), or `None` for an Intel Mac.
    pub chip: Option<&'static str>,
    /// It has the Apple T2 Security Chip.
    pub t2: bool,
    /// Its firmware is 32-bit EFI, which cannot start a 64-bit EFI file.
    pub efi32: bool,
    /// Its built-in keyboard.
    pub keyboard: Keyboard,
}

impl Mac {
    /// It has an Apple chip, not an Intel processor.
    pub fn apple_silicon(&self) -> bool {
        self.chip.is_some()
    }

    /// Its Apple chip's generation ("M1" for an M1 Pro), or `None` for
    /// an Intel Mac.
    pub fn generation(&self) -> Option<&'static str> {
        self.chip.and_then(|c| c.split(' ').next())
    }
}

/// The table's rows, as `macs.tsv` holds them (its header says how).
const TABLE: &str = include_str!("macs.tsv");

/// One row of `macs.tsv`, or `None` when it does not read.
fn row(l: &'static str) -> Option<Mac> {
    let f: Vec<&'static str> = l.split('\t').collect();
    let [id, name, year, line, chip, t2, efi32, keyboard] = f.as_slice() else {
        return None;
    };
    let yes = |v: &str| match v {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    };
    Some(Mac {
        id,
        name,
        year: year.parse().ok()?,
        line: match *line {
            "macbook-pro" => Line::MacBookPro,
            "macbook-air" => Line::MacBookAir,
            "macbook" => Line::MacBook,
            "imac" => Line::IMac,
            "imac-pro" => Line::IMacPro,
            "mac-mini" => Line::MacMini,
            "mac-pro" => Line::MacPro,
            "mac-studio" => Line::MacStudio,
            "xserve" => Line::Xserve,
            _ => return None,
        },
        chip: match *chip {
            "intel" => None,
            "" => return None,
            c => Some(c),
        },
        t2: yes(t2)?,
        efi32: yes(efi32)?,
        keyboard: match *keyboard {
            "usb" => Keyboard::Usb,
            "spi" => Keyboard::Spi,
            "t2" => Keyboard::T2,
            "apple-silicon" => Keyboard::AppleSilicon,
            "none" => Keyboard::None,
            _ => return None,
        },
    })
}

/// Every Mac model identifier, from the sources in this module's
/// comment: by line, then by year.
pub fn macs() -> &'static [Mac] {
    static MACS: OnceLock<Vec<Mac>> = OnceLock::new();
    MACS.get_or_init(|| {
        TABLE
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| row(l).unwrap_or_else(|| panic!("macs.tsv: a row that does not read: {l}")))
            .collect()
    })
}

/// The Mac with model identifier `model`, or `None` for one the table
/// does not know.
pub fn mac(model: &str) -> Option<&'static Mac> {
    let model = model.trim();
    macs().iter().find(|m| m.id == model)
}

/// Whether a computer's maker is Apple (§3.1: "the maker begins
/// Apple"), on any system.
pub fn is_apple(maker: &str) -> bool {
    let m = maker.trim();
    m.len() >= 5 && m.is_char_boundary(5) && m[..5].eq_ignore_ascii_case("apple")
}

/// Whether a Mac can start the PC stick, as Home reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// An Intel Mac with 64-bit EFI: it starts the PC stick. `t2` is
    /// whether it has the T2 chip, `None` for an identifier the table
    /// does not know.
    Intel {
        /// It has the T2 chip; `None` when not known.
        t2: Option<bool>,
    },
    /// 32-bit EFI firmware: it cannot start the stick.
    Firmware32,
    /// Apple silicon: not yet (until `docs/MAC.md` A1 and A2).
    NotYet,
}

/// What Home needs to know about this computer as a Mac, or `None` when
/// it is not one. An ARM computer is Apple silicon whatever the table
/// says (an x86_64 hub app under Rosetta reports `arm` too).
pub fn start(c: &Computer) -> Option<Start> {
    if !is_apple(&c.maker) {
        return None;
    }
    let row = mac(&c.model);
    Some(match row {
        _ if c.arm => Start::NotYet,
        Some(m) if m.apple_silicon() => Start::NotYet,
        Some(m) if m.efi32 => Start::Firmware32,
        Some(m) => Start::Intel { t2: Some(m.t2) },
        None => Start::Intel { t2: None },
    })
}

/// Which stick image fits this computer (`docs/HUB.md` §3.2 step 0):
/// the PC stick for a computer that can start it, the Apple silicon
/// stick for an M1 or M2 Mac, neither otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickFor {
    /// "A PC or an Intel Mac".
    Pc,
    /// "A Mac with M1 or M2".
    AppleSilicon,
}

/// The stick image that fits this computer, when one does.
pub fn stick_for(c: &Computer) -> Option<StickFor> {
    match start(c) {
        None if c.arm => None,
        None | Some(Start::Intel { .. }) => Some(StickFor::Pc),
        Some(Start::Firmware32) => None,
        Some(Start::NotYet) => match mac(&c.model).and_then(Mac::generation) {
            Some("M1" | "M2") => Some(StickFor::AppleSilicon),
            _ => None,
        },
    }
}
