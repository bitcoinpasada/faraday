//! The guide's pages, the Cryptography blocks and the maker table, read
//! when Faraday is built, so the desktop app, the stick and the Starter
//! carry the same text. Nothing here names a screen of any app: each app maps its
//! own screens to pages.
//!
//! The website's guides to starting a computer from the stick
//! (`docs/STARTER.md` §7) are not here: they are `faraday-start`' (read by
//! the Starter, and published by the website). The
//! desktop app and the stick carry only the address of the guide for
//! this computer ([`guide_address`]), not its text.

use faraday_strings::EN;

pub mod block;
pub mod construction;
pub mod guide;
pub mod how;

pub use block::Block;
pub use guide::{
    GuideGroup, GuidePage, GuideSection, Span, guide_groups, guide_page, guide_places,
    guide_section,
};

/// One page of a start guide: its title and the sections under it.
pub struct Page {
    /// Title of the page.
    pub title: &'static str,
    /// The sections, in the order the page shows them.
    pub sections: &'static [Section],
}

/// One section of a [`Page`]: a heading and its paragraphs.
pub struct Section {
    /// The heading.
    pub heading: &'static str,
    /// The paragraphs under it.
    pub paragraphs: &'static [&'static str],
}

/// A maker family's guide (`docs/STARTER.md` §7.6): its page's id, and the
/// computers it is for, by what the firmware reports (§7.1).
#[derive(Debug)]
pub struct Maker {
    /// The page's id in `faraday-start`' table, `maker-<family>`.
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

/// A page from its Markdown: `# ` the
/// title, `## ` a section, paragraphs separated by blank lines. Public
/// so `faraday-start` reads its own pages the same way.
pub fn page_of(md: &'static str) -> Page {
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
    Page {
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
