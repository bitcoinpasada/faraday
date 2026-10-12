//! The ? button: OpenSigner's Learn pages, read from `osk_learn::EN`
//! (edited as `docs/learn/*.md` upstream), so Faraday shows the same text
//! and never a copy that
//! drifts. Each screen names the pages that explain it; a wallet's screens
//! name the pages for its kind.

use std::sync::OnceLock;

use osk_learn::{EN, Page as LearnPage, Section};

use crate::create::NewKind;
use crate::wallet::Kind;
use crate::{Faraday, Screen};

/// The Learn sheet's state: which pages, which one is open, and how far
/// it is scrolled.
#[derive(Default)]
pub struct LearnState {
    /// The pages the screen named.
    pub pages: Vec<&'static LearnPage>,
    /// The one on show.
    pub page: usize,
    /// Design units scrolled.
    pub scroll: f32,
    /// The furthest it can scroll, as last drawn.
    pub max: f32,
}

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
const OWN: [(&str, &str); 20] = [
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
/// record (§3.1a), when the stick does not start, and replacing Faraday
/// on a stick.
pub const BOOT: [&str; 7] = [
    "windows",
    "linux",
    "bitlocker",
    "secure-boot-bitlocker",
    "screens-that-record",
    "not-starting",
    "replace-faraday",
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

/// Faraday's own pages, read once, in [`OWN`]'s order.
fn own() -> &'static [LearnPage] {
    static PAGES: OnceLock<Vec<LearnPage>> = OnceLock::new();
    PAGES.get_or_init(|| OWN.iter().map(|(_, md)| page_of(md)).collect())
}

/// Faraday's own page with this id, or `None` for an id it does not have.
pub fn page(id: &str) -> Option<&'static LearnPage> {
    OWN.iter().position(|(i, _)| *i == id).map(|n| &own()[n])
}

/// Faraday's own page with an id from [`OWN`].
fn known(id: &str) -> &'static LearnPage {
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

fn for_kind(kind: Kind) -> Vec<&'static LearnPage> {
    match kind {
        Kind::Single(_) => vec![&EN.wallet_kinds, &EN.words],
        Kind::Multi(_) => vec![&EN.multisig, &EN.xpubs, &EN.coordinators],
        Kind::TapMulti => vec![&EN.multisig, &EN.spend_paths],
        Kind::Miniscript | Kind::Tree => {
            vec![&EN.spend_paths, &EN.inheritance, &EN.wallet_kinds]
        }
        Kind::MuSig => vec![&EN.nonces, &EN.multisig],
        Kind::Threshold => vec![&EN.frost, &EN.nonces],
        Kind::Silent => vec![&EN.silent_payments, &EN.wallet_kinds],
        Kind::Other => vec![&EN.wallet_kinds],
    }
}

fn for_new(kind: NewKind) -> Vec<&'static LearnPage> {
    match kind {
        NewKind::NativeSegwit | NewKind::Taproot | NewKind::NestedSegwit | NewKind::Legacy => {
            vec![&EN.wallet_kinds, &EN.words, &EN.backups]
        }
        NewKind::Multi | NewKind::MultiNested | NewKind::MultiLegacy => {
            vec![&EN.multisig, &EN.xpubs, &EN.coordinators]
        }
        NewKind::TapMulti => vec![&EN.multisig, &EN.spend_paths],
        NewKind::Threshold => vec![&EN.frost, &EN.nonces],
        NewKind::MuSig => vec![&EN.nonces, &EN.wallet_kinds],
    }
}

impl Faraday {
    /// The Learn pages that explain what is on screen.
    pub fn learn_pages(&self) -> Vec<&'static LearnPage> {
        let wallet = || {
            self.session
                .wallets
                .get(self.wallet)
                .map(|w| Kind::of(&w.policy))
        };
        match self.screen {
            Screen::Home => vec![&EN.start_here, &EN.air_gap],
            Screen::Family => vec![&EN.inheritance, &EN.transactions, &EN.air_gap, &EN.scams],
            Screen::Start => vec![&EN.wallet_kinds, &EN.start_here],
            Screen::Wallets => wallet().map_or_else(|| vec![&EN.wallet_kinds], for_kind),
            Screen::Spend => {
                let mut p = vec![&EN.transactions, &EN.verifying];
                if let Some(k) = wallet()
                    && matches!(k, Kind::MuSig | Kind::Threshold)
                {
                    p.push(&EN.nonces);
                }
                p.push(&EN.scams);
                p
            }
            Screen::Create => self
                .create
                .as_ref()
                .map_or_else(|| vec![&EN.wallet_kinds], |c| for_new(c.kind)),
            Screen::KeyGen => vec![&EN.randomness, &EN.where_randomness, &EN.words],
            Screen::Entry => vec![&EN.words, &EN.passphrases],
            Screen::Silent => vec![&EN.silent_payments, &EN.xpubs],
            Screen::Explore => vec![&EN.xpubs, &EN.glossary],
            Screen::Vanity => vec![&EN.passphrases, &EN.wallet_kinds],
            Screen::Lightning => vec![&EN.glossary],
            Screen::Tools => vec![&EN.tools, &EN.glossary],
            Screen::Bip85 => vec![&EN.passphrases, &EN.glossary],
            Screen::Backup => {
                // A multisig's backup can keep shares of its description.
                let splits = self
                    .backup
                    .as_ref()
                    .and_then(|b| self.session.wallets.get(b.wallet))
                    .is_some_and(crate::backup::splits);
                if splits {
                    vec![&EN.backups, shares_page(), &EN.other_backups, &EN.seed_xor]
                } else {
                    vec![&EN.backups, &EN.other_backups, &EN.seed_xor]
                }
            }
            Screen::Backups => vec![&EN.backups, &EN.encrypted_backups],
            Screen::Restore => vec![&EN.backups, &EN.inheritance],
            Screen::Message | Screen::CheckMessage => vec![&EN.message],
            Screen::Files | Screen::Visit => vec![&EN.air_gap, &EN.coordinators],
            Screen::Vaults | Screen::CreateVault | Screen::Unlock | Screen::VaultContents => {
                vec![&EN.encrypted_backups, &EN.passphrases]
            }
            Screen::Settings if self.online => vec![&EN.secure_element, &EN.glossary],
            Screen::Settings => vec![&EN.secure_element, &EN.glossary, upgrade_page()],
            Screen::SetTime | Screen::Codes => vec![time_page()],
            // A stick can also be replaced from a computer, keeping its
            // data (`docs/HUB.md` §4.4); a Pi card cannot yet (§3.3).
            Screen::Upgrade if self.medium == crate::medium::Medium::Stick => {
                vec![upgrade_page(), known("replace-faraday")]
            }
            Screen::Upgrade => vec![upgrade_page()],
            Screen::Decode => vec![&EN.transactions, &EN.verifying],
            Screen::Catalog => vec![&EN.tools, &EN.glossary],
            Screen::Transfer => vec![&EN.air_gap, &EN.coordinators],
        }
    }

    /// Opens the Learn sheet on the screen's first page.
    pub(crate) fn learn_open(&mut self) {
        let pages = self.learn_pages();
        if pages.is_empty() {
            return;
        }
        self.learn = LearnState {
            pages,
            ..LearnState::default()
        };
        self.sheet = Some(crate::Sheet::Learn);
    }

    /// Opens the Learn sheet on the page about shares; closing it leaves
    /// the screen as it was.
    pub(crate) fn learn_shares(&mut self) {
        self.learn_open();
        if let Some(i) = self
            .learn
            .pages
            .iter()
            .position(|p| std::ptr::eq(*p, shares_page()))
        {
            self.learn.page = i;
        } else {
            self.learn = LearnState {
                pages: vec![shares_page()],
                ..LearnState::default()
            };
            self.sheet = Some(crate::Sheet::Learn);
        }
    }

    /// Opens the Learn sheet on `pages` over the screen on show: for the
    /// app holding this one ([`Faraday::held`]), whose own screens name
    /// the pages. Closing the sheet leaves the screen as it was.
    pub fn learn_with(&mut self, pages: Vec<&'static LearnPage>) {
        if pages.is_empty() {
            return;
        }
        self.learn = LearnState {
            pages,
            ..LearnState::default()
        };
        self.sheet = Some(crate::Sheet::Learn);
        self.dirty = true;
        self.commands.push_back(osk_shell_api::Command::Draw);
    }

    /// Shows the sheet's page `i`.
    pub(crate) fn learn_page(&mut self, i: u8) {
        if usize::from(i) < self.learn.pages.len() {
            self.learn.page = usize::from(i);
            self.learn.scroll = 0.0;
        }
    }
}
