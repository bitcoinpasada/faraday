//! The website's guides to starting a computer from the stick
//! (`docs/STARTER.md` §7): every page under `docs/guide/start/`
//! (`docs/guide/start/`). Read by the
//! Starter, and published by the website at
//! `https://faradayoffline.app/start/<id>` (§7.4); the desktop app and
//! the stick carry only the address of the guide for this computer
//! (`faraday_guide::guide_address`), not its text.

use std::sync::OnceLock;

use faraday_guide::Page as GuidePage;

/// Every guide here, each by its id. Edited as
/// `docs/guide/start/<id>.md`. The id is the file's name and stays as it is: the
/// Starter's screens open a page by it, and the website publishes each
/// guide at `https://faradayoffline.app/start/<id>` (`docs/STARTER.md`
/// §7.4).
const OWN: [(&str, &str); 19] = [
    (
        "windows",
        include_str!("../../../docs/guide/start/windows.md"),
    ),
    ("linux", include_str!("../../../docs/guide/start/linux.md")),
    (
        "replace-faraday",
        include_str!("../../../docs/guide/start/replace-faraday.md"),
    ),
    (
        "secure-boot-bitlocker",
        include_str!("../../../docs/guide/start/secure-boot-bitlocker.md"),
    ),
    (
        "screens-that-record",
        include_str!("../../../docs/guide/start/screens-that-record.md"),
    ),
    // The management processors under the operating system (Intel's
    // Management Engine and AMT, AMD's PSP).
    (
        "what-else-runs",
        include_str!("../../../docs/guide/start/what-else-runs.md"),
    ),
    // The Faraday stick boot entry on Linux (`docs/STARTER.md` §3.1c).
    (
        "boot-entry",
        include_str!("../../../docs/guide/start/boot-entry.md"),
    ),
    (
        "not-starting",
        include_str!("../../../docs/guide/start/not-starting.md"),
    ),
    (
        "mac-intel",
        include_str!("../../../docs/guide/start/mac-intel.md"),
    ),
    (
        "maker-dell",
        include_str!("../../../docs/guide/start/maker-dell.md"),
    ),
    (
        "maker-hp",
        include_str!("../../../docs/guide/start/maker-hp.md"),
    ),
    (
        "maker-lenovo-thinkpad",
        include_str!("../../../docs/guide/start/maker-lenovo-thinkpad.md"),
    ),
    (
        "maker-lenovo-ideapad",
        include_str!("../../../docs/guide/start/maker-lenovo-ideapad.md"),
    ),
    (
        "maker-asus",
        include_str!("../../../docs/guide/start/maker-asus.md"),
    ),
    (
        "maker-acer",
        include_str!("../../../docs/guide/start/maker-acer.md"),
    ),
    (
        "maker-msi",
        include_str!("../../../docs/guide/start/maker-msi.md"),
    ),
    (
        "maker-framework",
        include_str!("../../../docs/guide/start/maker-framework.md"),
    ),
    (
        "maker-surface",
        include_str!("../../../docs/guide/start/maker-surface.md"),
    ),
    // A maker with no page of its own (`docs/STARTER.md` §7): the
    // common keys, from the pages above, and the same steps. It matches
    // no computer, so it is not in `faraday_guide::MAKERS`.
    (
        "maker-other",
        include_str!("../../../docs/guide/start/maker-other.md"),
    ),
];

/// The guides to starting a computer from the stick (`docs/STARTER.md`
/// §7.2), by id, in the order they are read: Windows, Linux, what Secure
/// Boot and BitLocker are (§3.1b), screens and devices that can record
/// (§3.1a), when the stick does not start, replacing Faraday on a stick
/// and starting an Intel Mac (`docs/MAC.md` §3.1–§3.2).
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
fn own() -> &'static [GuidePage] {
    static PAGES: OnceLock<Vec<GuidePage>> = OnceLock::new();
    PAGES.get_or_init(|| {
        OWN.iter()
            .map(|(_, md)| faraday_guide::page_of(md))
            .collect()
    })
}

/// The id of every guide here, in [`OWN`]'s order: for an app that
/// lists them all.
pub fn ids() -> impl Iterator<Item = &'static str> {
    OWN.iter().map(|(id, _)| *id)
}

/// The guide with this id; `None` for an id not among them.
pub fn page(id: &str) -> Option<&'static GuidePage> {
    OWN.iter().position(|(i, _)| *i == id).map(|n| &own()[n])
}
