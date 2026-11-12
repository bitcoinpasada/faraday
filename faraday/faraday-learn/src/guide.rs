//! The guide's pages: `docs/guide/<place>.md`, read when Faraday is built
//! and parsed once at first use (`docs/GUIDE-IN-APP.md` §3). Nothing here
//! names a screen: [`PAGES`] lists the files, and the app maps what is on
//! screen to a key.

use std::collections::HashMap;
use std::sync::OnceLock;

use faraday_strings::EN;

/// The included pages, by place: the one list of files. A page is added
/// by a line here.
const PAGES: [(&str, &str); 51] = [
    ("backup", include_str!("../../../docs/guide/backup.md")),
    (
        "backup-flow",
        include_str!("../../../docs/guide/backup-flow.md"),
    ),
    ("backups", include_str!("../../../docs/guide/backups.md")),
    ("bip85", include_str!("../../../docs/guide/bip85.md")),
    ("catalog", include_str!("../../../docs/guide/catalog.md")),
    (
        "check-message",
        include_str!("../../../docs/guide/check-message.md"),
    ),
    ("codes", include_str!("../../../docs/guide/codes.md")),
    ("compare", include_str!("../../../docs/guide/compare.md")),
    ("convert", include_str!("../../../docs/guide/convert.md")),
    (
        "core-wallet",
        include_str!("../../../docs/guide/core-wallet.md"),
    ),
    ("create", include_str!("../../../docs/guide/create.md")),
    (
        "create-vault",
        include_str!("../../../docs/guide/create-vault.md"),
    ),
    ("decode", include_str!("../../../docs/guide/decode.md")),
    ("entry", include_str!("../../../docs/guide/entry.md")),
    ("explore", include_str!("../../../docs/guide/explore.md")),
    ("files", include_str!("../../../docs/guide/files.md")),
    (
        "gpg-restore",
        include_str!("../../../docs/guide/gpg-restore.md"),
    ),
    ("home", include_str!("../../../docs/guide/home.md")),
    ("invoice", include_str!("../../../docs/guide/invoice.md")),
    ("kef-tool", include_str!("../../../docs/guide/kef-tool.md")),
    ("keygen", include_str!("../../../docs/guide/keygen.md")),
    (
        "lightning",
        include_str!("../../../docs/guide/lightning.md"),
    ),
    ("message", include_str!("../../../docs/guide/message.md")),
    ("mode", include_str!("../../../docs/guide/mode.md")),
    (
        "nonce-check",
        include_str!("../../../docs/guide/nonce-check.md"),
    ),
    (
        "ots-check",
        include_str!("../../../docs/guide/ots-check.md"),
    ),
    ("qr-tool", include_str!("../../../docs/guide/qr-tool.md")),
    (
        "randomness",
        include_str!("../../../docs/guide/randomness.md"),
    ),
    ("restore", include_str!("../../../docs/guide/restore.md")),
    ("set-time", include_str!("../../../docs/guide/set-time.md")),
    ("settings", include_str!("../../../docs/guide/settings.md")),
    (
        "signer-check",
        include_str!("../../../docs/guide/signer-check.md"),
    ),
    ("silent", include_str!("../../../docs/guide/silent.md")),
    ("spend", include_str!("../../../docs/guide/spend.md")),
    (
        "spend-from-backup",
        include_str!("../../../docs/guide/spend-from-backup.md"),
    ),
    (
        "starter-appearance",
        include_str!("../../../docs/guide/starter-appearance.md"),
    ),
    (
        "starter-home",
        include_str!("../../../docs/guide/starter-home.md"),
    ),
    (
        "starter-install",
        include_str!("../../../docs/guide/starter-install.md"),
    ),
    (
        "starter-make",
        include_str!("../../../docs/guide/starter-make.md"),
    ),
    (
        "starter-start",
        include_str!("../../../docs/guide/starter-start.md"),
    ),
    (
        "starter-stick",
        include_str!("../../../docs/guide/starter-stick.md"),
    ),
    (
        "starter-templates",
        include_str!("../../../docs/guide/starter-templates.md"),
    ),
    (
        "starter-tools",
        include_str!("../../../docs/guide/starter-tools.md"),
    ),
    ("tools", include_str!("../../../docs/guide/tools.md")),
    ("transfer", include_str!("../../../docs/guide/transfer.md")),
    ("unlock", include_str!("../../../docs/guide/unlock.md")),
    ("vanity", include_str!("../../../docs/guide/vanity.md")),
    (
        "vault-contents",
        include_str!("../../../docs/guide/vault-contents.md"),
    ),
    ("vaults", include_str!("../../../docs/guide/vaults.md")),
    ("wallets", include_str!("../../../docs/guide/wallets.md")),
    (
        "wallets-start",
        include_str!("../../../docs/guide/wallets-start.md"),
    ),
];

/// A run of a paragraph's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// The text.
    pub text: String,
    /// Whether it is a bold span, drawn in the medium face.
    pub bold: bool,
}

/// A paragraph: its runs of text, in order.
pub type Paragraph = Vec<Span>;

/// One section of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuideSection {
    /// Its key, `<place>` for the lead or `<place>.<part>`.
    pub key: String,
    /// Its heading: the page's title for the lead.
    pub heading: String,
    /// Its paragraphs.
    pub paragraphs: Vec<Paragraph>,
}

/// One place's page: the lead section, then a section per part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuidePage {
    /// The place, as the file is named.
    pub place: String,
    /// The page's title.
    pub title: String,
    /// The lead section first, then the parts' in the page's order.
    pub sections: Vec<GuideSection>,
}

impl GuidePage {
    /// The lead section.
    pub fn lead(&self) -> &GuideSection {
        &self.sections[0]
    }
}

/// Header lines the website and the lint read; the app skips them (but for Key).
const HEADS: [&str; 8] = [
    "Kind",          // not wording: a table head the parser reads
    "Status",        // not wording: a table head the parser reads
    "Screen",        // not wording: a table head the parser reads
    "Starter page",  // not wording: a table head the parser reads
    "Starter sheet", // not wording: a table head the parser reads
    "Group",         // not wording: a table head the parser reads
    "Opened from",   // not wording: a table head the parser reads
    "Key",           // not wording: a table head the parser reads
];

/// The runs of `text`, bold where it is between `**`.
fn spans(text: &str) -> Paragraph {
    let mut out = Vec::new();
    for (i, part) in text.split("**").enumerate() {
        if !part.is_empty() {
            out.push(Span {
                text: part.to_string(),
                bold: i % 2 == 1,
            });
        }
    }
    out
}

/// The head line `**Name:** value`, as (name, value).
fn head(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("**")?;
    let (name, value) = rest.split_once(":**")?;
    HEADS.contains(&name).then(|| (name, value.trim()))
}

/// Costs one pass over the page's text. A page that has no title or no
/// lead key, or a section without a key, is a mistake in the file: it
/// panics, and the page's own test finds it.
pub fn parse(place: &str, text: &str) -> GuidePage {
    let mut title = String::new();
    let mut sections: Vec<GuideSection> = Vec::new();
    let mut fenced = false;
    let mut para: Vec<&str> = Vec::new();
    let mut paras: Vec<Paragraph> = Vec::new();
    // The open section's heading and key, set when its `##` or the title
    // begins it.
    let mut open: Option<(String, Option<String>)> = None;

    fn close(
        open: &mut Option<(String, Option<String>)>,
        paras: &mut Vec<Paragraph>,
        sections: &mut Vec<GuideSection>,
        place: &str,
    ) {
        if let Some((heading, key)) = open.take() {
            let key =
                key.unwrap_or_else(|| panic!("guide page {place}: section {heading:?} has no key"));
            sections.push(GuideSection {
                key,
                heading,
                paragraphs: std::mem::take(paras),
            });
        }
    }
    fn flush(
        para: &mut Vec<&str>,
        open: &mut Option<(String, Option<String>)>,
        paras: &mut Vec<Paragraph>,
    ) {
        if para.is_empty() {
            return;
        }
        let first = para[0];
        if let Some((name, value)) = head(first) {
            // not wording: the table head the parser reads
            if let ("Key", Some((_, key))) = (name, open) {
                *key = Some(value.to_string());
            }
        } else if !first.starts_with("![") && !first.starts_with('|') && !first.starts_with("*![") {
            paras.push(spans(&para.join(" ")));
        }
        para.clear();
    }
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if let Some(t) = line.strip_prefix("# ") {
            flush(&mut para, &mut open, &mut paras);
            title = t.trim().to_string();
            open = Some((title.clone(), None));
        } else if let Some(h) = line.strip_prefix("## ") {
            flush(&mut para, &mut open, &mut paras);
            close(&mut open, &mut paras, &mut sections, place);
            open = Some((h.trim().to_string(), None));
        } else if line.trim().is_empty() {
            flush(&mut para, &mut open, &mut paras);
        } else {
            // A head line is its own paragraph, though the header lines
            // of a page stand one under the other.
            if head(line).is_some() {
                flush(&mut para, &mut open, &mut paras);
                para.push(line);
                flush(&mut para, &mut open, &mut paras);
            } else {
                para.push(line);
            }
        }
    }
    flush(&mut para, &mut open, &mut paras);
    close(&mut open, &mut paras, &mut sections, place);
    assert!(!title.is_empty(), "guide page {place}: no title");
    GuidePage {
        place: place.to_string(),
        title,
        sections,
    }
}

/// Costs the parse of every page on its first call; a lookup after.
fn all() -> &'static HashMap<&'static str, GuidePage> {
    static ALL: OnceLock<HashMap<&'static str, GuidePage>> = OnceLock::new();
    ALL.get_or_init(|| PAGES.iter().map(|(p, t)| (*p, parse(p, t))).collect())
}

/// The page of `place`, if one is written. A lookup after the first call,
/// which parses every page (`all`).
pub fn guide_page(place: &str) -> Option<&'static GuidePage> {
    all().get(place)
}

/// The places that have a page.
pub fn guide_places() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = all().keys().copied().collect();
    v.sort_unstable();
    v
}

/// The section with this key: the part's, if written. A lookup after the
/// first call, which parses every page (`all`).
pub fn guide_section(key: &str) -> Option<&'static GuideSection> {
    let place = key.split('.').next().unwrap_or(key);
    guide_page(place)?.sections.iter().find(|s| s.key == key)
}

/// A group of the guide's contents: its heading and the places listed
/// under it, in reading order.
pub struct GuideGroup {
    /// The group's heading.
    pub title: &'static str,
    /// Its pages, in reading order.
    pub pages: Vec<&'static GuidePage>,
}

/// The contents' groups and the places under each, in reading order
/// (`docs/GUIDE-IN-APP.md` §4.3). Every place with a page is listed once.
const ORDER: [&[&str]; 10] = [
    &["starter-stick", "starter-start"],
    &["home", "mode", "randomness", "files"],
    &[
        "keygen",
        "entry",
        "explore",
        "bip85",
        "convert",
        "vanity",
        "lightning",
    ],
    &[
        "backups",
        "backup",
        "backup-flow",
        "kef-tool",
        "qr-tool",
        "gpg-restore",
    ],
    &[
        "wallets-start",
        "wallets",
        "create",
        "restore",
        "silent",
        "core-wallet",
    ],
    &[
        "spend",
        "spend-from-backup",
        "message",
        "check-message",
        "decode",
        "compare",
        "signer-check",
        "nonce-check",
        "transfer",
    ],
    &["vaults", "create-vault", "unlock", "vault-contents"],
    &["tools", "catalog", "codes", "ots-check", "invoice"],
    &["settings", "set-time"],
    &[
        "starter-home",
        "starter-make",
        "starter-tools",
        "starter-install",
        "starter-templates",
        "starter-appearance",
    ],
];

/// The heading of the contents' group `i` of [`ORDER`].
fn group_title(i: usize) -> &'static str {
    [
        EN.learn_group_get,
        EN.learn_group_first,
        EN.learn_group_keys,
        EN.learn_group_backups,
        EN.learn_group_wallets,
        EN.learn_group_spending,
        EN.learn_group_vaults,
        EN.learn_group_tools,
        EN.learn_group_settings,
        EN.learn_group_starter,
    ][i]
}

/// The guide's contents: the groups, each with its pages that are
/// written, in reading order. A lookup after the first call, which
/// parses every page (`all`).
pub fn guide_groups() -> &'static [GuideGroup] {
    static GROUPS: OnceLock<Vec<GuideGroup>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        ORDER
            .iter()
            .enumerate()
            .map(|(i, places)| GuideGroup {
                title: group_title(i),
                pages: places.iter().filter_map(|p| guide_page(p)).collect(),
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_read_by_its_sections_and_bold_spans() {
        let p = parse(
            "x",
            "# X\n\n**Kind:** Reference\n**Key:** x\n\nOne **two** three.\n\n## Part\n\n**Key:** x.part\n\n![p](a.png)\n\nBody.\n",
        );
        assert_eq!(p.title, "X");
        assert_eq!(p.sections.len(), 2);
        assert_eq!(p.lead().key, "x");
        assert_eq!(p.lead().paragraphs[0].len(), 3);
        assert!(p.lead().paragraphs[0][1].bold);
        assert_eq!(p.sections[1].key, "x.part");
        assert_eq!(p.sections[1].heading, "Part");
        assert_eq!(p.sections[1].paragraphs.len(), 1);
    }

    #[test]
    fn the_contents_lists_every_page_once() {
        let mut listed: Vec<&str> = guide_groups()
            .iter()
            .flat_map(|g| g.pages.iter().map(|p| p.place.as_str()))
            .collect();
        listed.sort_unstable();
        assert_eq!(listed, guide_places());
    }

    #[test]
    fn home_has_a_lead() {
        let s = guide_section("home").unwrap();
        assert!(!s.paragraphs.is_empty());
        assert!(guide_section("home.nothing").is_none());
    }
}
