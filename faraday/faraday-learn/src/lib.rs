//! Faraday's own Learn pages, which OpenSigner does not have, and the
//! website's guides to starting a computer from the stick
//! (`docs/HUB.md` §7). Read from `docs/learn/faraday/` when Faraday is
//! built, so the desktop app, the stick, the hub app and the Companion
//! carry the same text. Nothing here names a screen of any app: each app
//! maps its own screens to pages.

use std::sync::OnceLock;

use osk_learn::{Page as LearnPage, Section};

/// Faraday's own pages, which OpenSigner does not have, each by its id.
/// They are edited as `docs/learn/faraday/<id>.md` (the guides to
/// starting a computer from the stick as `docs/learn/faraday/boot/<id>.md`,
/// `docs/HUB.md` §7.2), beside OpenSigner's pages but outside the set
/// `tools/learn/sync.py` keeps in step with `osk_learn::EN`, and read from
/// those files when Faraday is built, so the desktop app, the stick and
/// the hub app carry the same text. The id is the file's name and stays
/// as it is: the hub app's screens open a page by it, and the website
/// publishes each guide at `https://faradayoffline.app/start/<id>`
/// (§7.4).
const OWN: [(&str, &str); 22] = [
    // Upgrading a stick from another stick (`PLAN.md` §5.5).
    (
        "upgrade-a-stick",
        include_str!("../../../docs/learn/faraday/upgrade-a-stick.md"),
    ),
    // A multisig backup's shares (`docs/NEW-WALLET.md` §4.2).
    (
        "shares",
        include_str!("../../../docs/learn/faraday/shares.md"),
    ),
    // The clock and TOTP codes (`docs/TOTP.md`).
    ("time", include_str!("../../../docs/learn/faraday/time.md")),
    // Faraday Companion, the phone app (`docs/COMPANION.md` §4.7).
    (
        "companion",
        include_str!("../../../docs/learn/faraday/companion.md"),
    ),
    (
        "windows",
        include_str!("../../../docs/learn/faraday/boot/windows.md"),
    ),
    (
        "linux",
        include_str!("../../../docs/learn/faraday/boot/linux.md"),
    ),
    (
        "bitlocker",
        include_str!("../../../docs/learn/faraday/boot/bitlocker.md"),
    ),
    (
        "secure-boot-bitlocker",
        include_str!("../../../docs/learn/faraday/boot/secure-boot-bitlocker.md"),
    ),
    (
        "screens-that-record",
        include_str!("../../../docs/learn/faraday/boot/screens-that-record.md"),
    ),
    // The Faraday stick boot entry on Linux (`docs/HUB.md` §3.1c).
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
        "replace-faraday",
        include_str!("../../../docs/learn/faraday/boot/replace-faraday.md"),
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
];

/// The guides to starting a computer from the stick (`docs/HUB.md` §7.2),
/// by id, in the order they are read: Windows, Linux, BitLocker, what
/// Secure Boot and BitLocker are (§3.1b), screens and devices that can
/// record (§3.1a), when the stick does not start, replacing Faraday on
/// a stick, and starting an Intel Mac (`docs/MAC.md` §3.1–§3.2).
pub const BOOT: [&str; 8] = [
    "windows",
    "linux",
    "bitlocker",
    "secure-boot-bitlocker",
    "screens-that-record",
    "not-starting",
    "replace-faraday",
    "mac-intel",
];

/// A maker family's guide (`docs/HUB.md` §7.6): its page's id, and the
/// computers it is for, by what the firmware reports (§7.1).
#[derive(Debug)]
pub struct Maker {
    /// The page's id among [`OWN`], `maker-<family>`.
    pub id: &'static str,
    /// The maker (`sys_vendor`, `SystemManufacturer`) begins with one of
    /// these, ignoring case.
    pub vendors: &'static [&'static str],
    /// The family (`product_family`, `SystemFamily`) or the model
    /// (`product_name`, `SystemProductName`) begins with one of these,
    /// ignoring case; empty for every computer of the maker.
    pub families: &'static [&'static str],
}

/// The maker pages, each checked against the maker's own documentation
/// (§7.6). The first that matches a computer is its guide; a computer
/// none matches gets the general page for its operating system.
pub const MAKERS: [Maker; 9] = [
    Maker {
        id: "maker-dell",
        vendors: &["Dell"],
        families: &[],
    },
    Maker {
        id: "maker-hp",
        vendors: &["HP", "Hewlett-Packard"],
        families: &[],
    },
    Maker {
        id: "maker-lenovo-thinkpad",
        vendors: &["LENOVO"],
        families: &["ThinkPad"],
    },
    Maker {
        id: "maker-lenovo-ideapad",
        vendors: &["LENOVO"],
        families: &["IdeaPad", "Yoga", "Legion"],
    },
    Maker {
        id: "maker-asus",
        vendors: &["ASUSTeK", "ASUS"],
        families: &[],
    },
    Maker {
        id: "maker-acer",
        vendors: &["Acer"],
        families: &[],
    },
    Maker {
        id: "maker-msi",
        vendors: &["Micro-Star", "MSI"],
        families: &[],
    },
    Maker {
        id: "maker-framework",
        vendors: &["Framework"],
        families: &[],
    },
    Maker {
        id: "maker-surface",
        vendors: &["Microsoft"],
        families: &["Surface"],
    },
];

/// Whether `s`, trimmed, begins with `prefix`, ignoring ASCII case; a
/// vendor prefix must end at a word's end ("HP", not "HPE").
fn begins(s: &str, prefix: &str, word: bool) -> bool {
    let s = s.trim();
    s.len() >= prefix.len()
        && s.is_char_boundary(prefix.len())
        && s[..prefix.len()].eq_ignore_ascii_case(prefix)
        && (!word
            || s[prefix.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_alphanumeric()))
}

/// The maker page for a computer, by its maker, family and model as the
/// firmware reports them (§7.1), or `None` when no page is for it.
pub fn maker(vendor: &str, family: &str, model: &str) -> Option<&'static Maker> {
    MAKERS.iter().find(|m| {
        m.vendors.iter().any(|v| begins(vendor, v, true))
            && (m.families.is_empty()
                || m.families
                    .iter()
                    .any(|f| begins(family, f, false) || begins(model, f, false)))
    })
}

/// The website's guides (`docs/HUB.md` §7.4): the only address the hub
/// app and the stick contain.
pub const GUIDES: &str = "https://faradayoffline.app/start/";

/// The guide's page for a computer (`docs/HUB.md` §3.4, §7.6): its
/// maker's page where there is one, otherwise `general`.
pub fn guide_id(vendor: &str, family: &str, model: &str, general: &'static str) -> &'static str {
    maker(vendor, family, model).map_or(general, |m| m.id)
}

/// A guide's address on the website.
pub fn guide_address(id: &str) -> String {
    format!("{GUIDES}{id}")
}

/// Faraday's own pages, read once, in [`OWN`]'s order.
fn own() -> &'static [LearnPage] {
    static PAGES: OnceLock<Vec<LearnPage>> = OnceLock::new();
    PAGES.get_or_init(|| OWN.iter().map(|(_, md)| page_of(md)).collect())
}

/// The id of every one of Faraday's own pages, in [`OWN`]'s order: for
/// an app that lists them all (the Companion's Learn).
pub fn ids() -> impl Iterator<Item = &'static str> {
    OWN.iter().map(|(id, _)| *id)
}

/// Faraday's own page with this id, or `None` for an id it does not have.
pub fn page(id: &str) -> Option<&'static LearnPage> {
    OWN.iter().position(|(i, _)| *i == id).map(|n| &own()[n])
}

/// Faraday's own page with an id from [`OWN`].
pub fn known(id: &str) -> &'static LearnPage {
    page(id).expect("an id from OWN")
}

/// A page from its Markdown, as `docs/learn/` writes pages: `# ` the
/// title, `## ` a section, paragraphs separated by blank lines.
fn page_of(md: &'static str) -> LearnPage {
    let mut title = "";
    let mut sections: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    let mut para: Vec<&'static str> = Vec::new();
    let flush = |para: &mut Vec<&'static str>,
                 sections: &mut Vec<(&'static str, Vec<&'static str>)>| {
        if para.is_empty() {
            return;
        }
        let text: &'static str = Box::leak(para.join(" ").into_boxed_str());
        para.clear();
        if let Some(s) = sections.last_mut() {
            s.1.push(text);
        }
    };
    for line in md.lines() {
        if let Some(t) = line.strip_prefix("# ") {
            title = t;
        } else if let Some(h) = line.strip_prefix("## ") {
            flush(&mut para, &mut sections);
            sections.push((h, Vec::new()));
        } else if line.trim().is_empty() {
            flush(&mut para, &mut sections);
        } else {
            para.push(line.trim());
        }
    }
    flush(&mut para, &mut sections);
    let sections: Vec<Section> = sections
        .into_iter()
        .map(|(heading, paragraphs)| Section {
            heading,
            paragraphs: Box::leak(paragraphs.into_boxed_slice()),
        })
        .collect();
    LearnPage {
        title,
        sections: Box::leak(sections.into_boxed_slice()),
    }
}

/// "Upgrading a Faraday stick".
pub fn upgrade_page() -> &'static LearnPage {
    known("upgrade-a-stick")
}

/// "Shares of a wallet description".
pub fn shares_page() -> &'static LearnPage {
    known("shares")
}

/// "Time and TOTP codes".
pub fn time_page() -> &'static LearnPage {
    known("time")
}

/// "Faraday Companion".
pub fn companion_page() -> &'static LearnPage {
    known("companion")
}
