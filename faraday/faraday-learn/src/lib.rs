//! Every Learn page: those in [`pages`] and
//! Faraday's own: read from
//! `docs/learn/faraday/` when Faraday is built, so the desktop app, the
//! stick and the Starter carry the same text. Nothing
//! here names a screen of any app: each app maps its own screens to
//! pages.
//!
//! The website's guides to starting a computer from the stick
//! (`docs/STARTER.md` §7) are not here: they are `faraday-guides`' (read by
//! the Starter, and published by the website). The
//! desktop app and the stick carry only the address of the guide for
//! this computer ([`guide_address`]), not its text.

use std::sync::OnceLock;

use faraday_strings::EN;
use pages::Page as LearnPage;

pub mod pages;
pub mod sketch;

pub use pages::{Page, Section};

pub use sketch::{Sketch, is_marker};

/// Faraday's own pages, each by its id.
/// They are edited as `docs/learn/faraday/<id>.md`, beside the other
/// pages but outside the set `tools/learn/sync.py` keeps in step with
/// `pages::EN`, and read from those files when Faraday is built, so
/// the desktop app, the stick and the Starter carry the same text. The
/// guides to starting a computer from the stick
/// (`docs/learn/faraday/boot/*.md`) are `faraday-guides`' own table, not
/// this one, except `replace-faraday`, which the stick's Upgrade screen
/// shows directly ([`known`]).
const OWN: [(&str, &str); 11] = [
    // Upgrading a stick from another stick (`docs/history/PLAN.md` §5.5).
    (
        "upgrade-a-stick",
        include_str!("../../../docs/learn/faraday/upgrade-a-stick.md"),
    ),
    // A multisig backup's shares (`docs/history/NEW-WALLET.md` §4.2).
    (
        "shares",
        include_str!("../../../docs/learn/faraday/shares.md"),
    ),
    // The clock and TOTP codes (`docs/TOTP.md`).
    ("time", include_str!("../../../docs/learn/faraday/time.md")),
    (
        "replace-faraday",
        include_str!("../../../docs/learn/faraday/boot/replace-faraday.md"),
    ),
    // The demo's public seeds and the test networks (F20).
    ("demo", include_str!("../../../docs/learn/faraday/demo.md")),
    // The APK signing key (`docs/APK-SIGNING.md` §10).
    (
        "apk-key",
        include_str!("../../../docs/learn/faraday/apk-key.md"),
    ),
    // GPG keys, their revocation certificate and paperkey (F32).
    ("gpg", include_str!("../../../docs/learn/faraday/gpg.md")),
    // What a backup plan's three checks mean (`docs/BACKUP-FLOW.md`).
    (
        "backup-plan-check",
        include_str!("../../../docs/learn/faraday/backup-plan-check.md"),
    ),
    // Copies made outside the backup plan (`docs/BACKUP-FLOW.md` §19.19
    // item 7).
    (
        "extra-copies",
        include_str!("../../../docs/learn/faraday/extra-copies.md"),
    ),
    // Where keys are kept: memory, vaults, the idle lock (F58).
    (
        "keys-kept",
        include_str!("../../../docs/learn/faraday/keys-kept.md"),
    ),
    // Vaults (`docs/VAULT.md`, F62).
    (
        "vaults",
        include_str!("../../../docs/learn/faraday/vaults.md"),
    ),
];

/// A maker family's guide (`docs/STARTER.md` §7.6): its page's id, and the
/// computers it is for, by what the firmware reports (§7.1).
#[derive(Debug)]
pub struct Maker {
    /// The page's id in `faraday-guides`' table, `maker-<family>`.
    pub id: &'static str,
    /// The maker (`sys_vendor`, `SystemManufacturer`) begins with one of
    /// these, ignoring case.
    pub vendors: &'static [&'static str],
    /// The family (`product_family`, `SystemFamily`) or the model
    /// (`product_name`, `SystemProductName`) begins with one of these,
    /// ignoring case; empty for every computer of the maker.
    pub families: &'static [&'static str],
    /// The key that opens this computer's BIOS, as the page says it
    /// (`docs/FLOWS.md`, Restart; F42): its text is `faraday-strings`'.
    pub bios_key: &'static str,
    /// The key that opens this computer's boot menu, to start the stick
    /// (`docs/STARTER.md` §3.4, F47), as the page says it.
    pub boot_key: &'static str,
    /// Where the Secure Boot setting is in this maker's BIOS, as the
    /// page names it: a menu path, parts joined by " → "; a second path
    /// for other models after "; ", with what they are before ": "
    /// ("business: Security → Secure Boot Configuration").
    pub secure_boot: &'static str,
}

/// The maker pages, each checked against the maker's own documentation
/// (§7.6). The first that matches a computer is its guide; a computer
/// none matches gets the general page for its operating system.
pub const MAKERS: [Maker; 9] = [
    Maker {
        id: "maker-dell",
        vendors: &["Dell"],
        families: &[],
        bios_key: EN.bios_key_dell,
        boot_key: EN.boot_key_dell,
        secure_boot: EN.secure_boot_dell,
    },
    Maker {
        id: "maker-hp",
        vendors: &["HP", "Hewlett-Packard"],
        families: &[],
        bios_key: EN.bios_key_hp,
        boot_key: EN.boot_key_hp,
        secure_boot: EN.secure_boot_hp,
    },
    Maker {
        id: "maker-lenovo-thinkpad",
        vendors: &["LENOVO"],
        families: &["ThinkPad"],
        bios_key: EN.bios_key_lenovo_thinkpad,
        boot_key: EN.boot_key_lenovo_thinkpad,
        secure_boot: EN.secure_boot_lenovo_thinkpad,
    },
    Maker {
        id: "maker-lenovo-ideapad",
        vendors: &["LENOVO"],
        families: &["IdeaPad", "Yoga", "Legion"],
        bios_key: EN.bios_key_lenovo_ideapad,
        boot_key: EN.boot_key_lenovo_ideapad,
        secure_boot: EN.secure_boot_lenovo_ideapad,
    },
    Maker {
        id: "maker-asus",
        vendors: &["ASUSTeK", "ASUS"],
        families: &[],
        bios_key: EN.bios_key_asus,
        boot_key: EN.boot_key_asus,
        secure_boot: EN.secure_boot_asus,
    },
    Maker {
        id: "maker-acer",
        vendors: &["Acer"],
        families: &[],
        bios_key: EN.bios_key_acer,
        boot_key: EN.boot_key_acer,
        secure_boot: EN.secure_boot_acer,
    },
    Maker {
        id: "maker-msi",
        vendors: &["Micro-Star", "MSI"],
        families: &[],
        bios_key: EN.bios_key_msi,
        boot_key: EN.boot_key_msi,
        secure_boot: EN.secure_boot_msi,
    },
    Maker {
        id: "maker-framework",
        vendors: &["Framework"],
        families: &[],
        bios_key: EN.bios_key_framework,
        boot_key: EN.boot_key_framework,
        secure_boot: EN.secure_boot_framework,
    },
    Maker {
        id: "maker-surface",
        vendors: &["Microsoft"],
        families: &["Surface"],
        bios_key: EN.bios_key_surface,
        boot_key: EN.boot_key_surface,
        secure_boot: EN.secure_boot_surface,
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

/// The website's guides (`docs/STARTER.md` §7.4): the only address the Starter
/// app and the stick contain.
pub const GUIDES: &str = "https://faradayoffline.app/start/";

/// The guide's page for a computer (`docs/STARTER.md` §3.4, §7.6): its
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
/// an app that lists them all.
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
/// title, `## ` a section, paragraphs separated by blank lines. Public
/// so `faraday-guides` reads its own pages the same way.
pub fn page_of(md: &'static str) -> LearnPage {
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

/// A guide's step: a paragraph that begins with its number, a full stop and a
/// space, `3. Choose the stick.` Returns the number and the text after
/// it, or `None` for any other paragraph. An app draws the number its
/// own way and never prints the paragraph's raw `n. `.
pub fn step(para: &str) -> Option<(u32, &str)> {
    let digits = para.find(|c: char| !c.is_ascii_digit())?;
    if digits == 0 || digits > 3 {
        return None;
    }
    let text = para[digits..].strip_prefix(". ")?;
    let n = para[..digits].parse().ok()?;
    (n > 0 && !text.trim().is_empty()).then_some((n, text))
}

/// Every step number on `page`, in the order written.
pub fn steps(page: &LearnPage) -> Vec<u32> {
    page.sections
        .iter()
        .flat_map(|s| s.paragraphs.iter())
        .filter_map(|p| step(p).map(|(n, _)| n))
        .collect()
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

/// "GPG keys".
pub fn gpg_page() -> &'static LearnPage {
    known("gpg")
}

/// "A backup plan's check".
pub fn backup_plan_check_page() -> &'static LearnPage {
    known("backup-plan-check")
}

/// "Extra copies".
pub fn extra_copies_page() -> &'static LearnPage {
    known("extra-copies")
}

/// "The demo and its public seeds".
pub fn demo_page() -> &'static LearnPage {
    known("demo")
}

/// "Where keys are kept".
pub fn keys_kept_page() -> &'static LearnPage {
    known("keys-kept")
}

/// "Vaults".
pub fn vaults_page() -> &'static LearnPage {
    known("vaults")
}

/// "The APK signing key".
pub fn apk_key_page() -> &'static LearnPage {
    known("apk-key")
}
