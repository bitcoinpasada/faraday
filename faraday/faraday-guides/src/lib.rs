//! The website's guides to starting a computer from the stick
//! (`docs/STARTER.md` §7): every page under `docs/learn/faraday/boot/`
//! except `replace-faraday`, which stays `faraday-learn`'s (the stick's
//! Upgrade screen shows it directly, without this crate). Read by the
//! Starter, and published by the website at
//! `https://faradayoffline.app/start/<id>` (§7.4); the desktop app and
//! the stick carry only the address of the guide for this computer
//! (`faraday_learn::guide_address`), not its text.

use std::sync::OnceLock;

use faraday_learn::Page as LearnPage;

/// Every guide here, each by its id. Edited as
/// `docs/learn/faraday/boot/<id>.md`, beside `replace-faraday`, which is
/// `faraday-learn`'s. The id is the file's name and stays as it is: the
/// Starter's screens open a page by it, and the website publishes each
/// guide at `https://faradayoffline.app/start/<id>` (`docs/STARTER.md`
/// §7.4).
const OWN: [(&str, &str); 17] = [
    (
        "windows",
        include_str!("../../../docs/learn/faraday/boot/windows.md"),
    ),
    (
        "linux",
        include_str!("../../../docs/learn/faraday/boot/linux.md"),
    ),
    (
        "secure-boot-bitlocker",
        include_str!("../../../docs/learn/faraday/boot/secure-boot-bitlocker.md"),
    ),
    (
        "screens-that-record",
        include_str!("../../../docs/learn/faraday/boot/screens-that-record.md"),
    ),
    // The Faraday stick boot entry on Linux (`docs/STARTER.md` §3.1c).
    (
        "boot-entry",
        include_str!("../../../docs/learn/faraday/boot/boot-entry.md"),
    ),
    (
        "not-starting",
        include_str!("../../../docs/learn/faraday/boot/not-starting.md"),
    ),
    (
        "mac-intel",
        include_str!("../../../docs/learn/faraday/boot/mac-intel.md"),
    ),
    (
        "maker-dell",
        include_str!("../../../docs/learn/faraday/boot/maker-dell.md"),
    ),
    (
        "maker-hp",
        include_str!("../../../docs/learn/faraday/boot/maker-hp.md"),
    ),
    (
        "maker-lenovo-thinkpad",
        include_str!("../../../docs/learn/faraday/boot/maker-lenovo-thinkpad.md"),
    ),
    (
        "maker-lenovo-ideapad",
        include_str!("../../../docs/learn/faraday/boot/maker-lenovo-ideapad.md"),
    ),
    (
        "maker-asus",
        include_str!("../../../docs/learn/faraday/boot/maker-asus.md"),
    ),
    (
        "maker-acer",
        include_str!("../../../docs/learn/faraday/boot/maker-acer.md"),
    ),
    (
        "maker-msi",
        include_str!("../../../docs/learn/faraday/boot/maker-msi.md"),
    ),
    (
        "maker-framework",
        include_str!("../../../docs/learn/faraday/boot/maker-framework.md"),
    ),
    (
        "maker-surface",
        include_str!("../../../docs/learn/faraday/boot/maker-surface.md"),
    ),
    // A maker with no page of its own (`docs/STARTER.md` §7): the
    // common keys, from the pages above, and the same steps. It matches
    // no computer, so it is not in `faraday_learn::MAKERS`.
    (
        "maker-other",
        include_str!("../../../docs/learn/faraday/boot/maker-other.md"),
    ),
];

/// The guides to starting a computer from the stick (`docs/STARTER.md`
/// §7.2), by id, in the order they are read: Windows, Linux, what Secure
/// Boot and BitLocker are (§3.1b), screens and devices that can record
/// (§3.1a), when the stick does not start, replacing Faraday on a stick
/// (`faraday-learn`'s, found through [`faraday_learn::page`]), and
/// starting an Intel Mac (`docs/MAC.md` §3.1–§3.2).
pub const BOOT: [&str; 7] = [
    "windows",
    "linux",
    "secure-boot-bitlocker",
    "screens-that-record",
    "not-starting",
    "replace-faraday",
    "mac-intel",
];

/// This crate's own pages, read once, in [`OWN`]'s order.
fn own() -> &'static [LearnPage] {
    static PAGES: OnceLock<Vec<LearnPage>> = OnceLock::new();
    PAGES.get_or_init(|| {
        OWN.iter()
            .map(|(_, md)| faraday_learn::page_of(md))
            .collect()
    })
}

/// The id of every guide here, in [`OWN`]'s order: for an app that
/// lists them all, chained after
/// `faraday_learn::ids()`.
pub fn ids() -> impl Iterator<Item = &'static str> {
    OWN.iter().map(|(id, _)| *id)
}

/// A guide with this id, or, for an id not among them (`replace-faraday`
/// is `faraday-learn`'s), its page there; `None` for an id neither has.
pub fn page(id: &str) -> Option<&'static LearnPage> {
    OWN.iter()
        .position(|(i, _)| *i == id)
        .map(|n| &own()[n])
        .or_else(|| faraday_learn::page(id))
}
