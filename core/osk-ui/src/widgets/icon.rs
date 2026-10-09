//! The icon set (UX.md §6).
//!
//! Every icon the application draws is a variant of [`Icon`]. Each maps to
//! one code point in the baked icon face, which `tools/fontbake`
//! rasterises from the SeedSigner icon font (`U+E9xx`) and Font Awesome 6
//! Free Solid (everything else). Nothing pulls a symbol out of a text
//! face: text glyphs are the wrong shape and the wrong weight at icon
//! sizes.
//!
//! One meaning per glyph (`docs/DESIGN.md` §3): a glyph is either a fact
//! marker — [`Icon::Keys`] and [`Icon::Eye`], which say whether this
//! device holds the private key (§4.4) — or the identity of one row, and
//! never both. Within one menu no two rows share a glyph, and a label
//! that appears in two menus carries the same glyph in both.

/// A named icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// The scanner: a wallet or a key that arrives by camera.
    Scan,
    /// A key: §4.4's fact marker for a wallet this device can sign for.
    Keys,
    /// Signing a transaction.
    Sign,
    /// Verification: check an address, a config, a signature.
    Verify,
    /// The key explorer.
    Explore,
    /// The Learn section.
    Learn,
    /// Settings.
    Settings,
    /// Tools.
    Tools,
    /// The app bar's back control.
    Back,
    /// The choice marker: the current item of a choice list.
    Check,
    /// The keyboard's ✓ key, which ends an entry.
    Done,
    /// Chevron pointing left: the previous page.
    ChevronLeft,
    /// Chevron pointing right: a row that opens a screen, the next page.
    ChevronRight,
    /// Chevron pointing up: a collapsible section, the jamo shift key.
    ChevronUp,
    /// Home, on the `wide` sidebar.
    House,
    /// Power off, wipe and exit.
    Power,
    /// Swap between two forms of the same thing: converting a key.
    Swap,
    /// An informational note.
    Info,
    /// A completed action.
    Success,
    /// A caution.
    Warning,
    /// An error or a danger.
    Error,
    /// A derivation path: the BIP-85 child a key opens.
    Derivation,
    /// A key fingerprint: the key or keys a wallet is made of.
    Fingerprint,
    /// A passphrase.
    Passphrase,
    /// Bitcoin: the units tool.
    Bitcoin,
    /// A QR code.
    Qr,
    /// Backspace.
    Delete,
    /// The space bar.
    Space,
    /// Locked: the lock screen, the session PIN.
    Lock,
    /// Dice.
    Dice,
    /// A file.
    File,
    /// Reveal: hold to show a secret. §4.4's fact marker for a wallet
    /// this device can only watch.
    Eye,
    /// Hidden: the one mark a masked secret panel carries
    /// (`docs/DESIGN.md` §4.10).
    EyeOff,
    /// Forget, wipe.
    Trash,
    /// A list of words.
    List,
    /// A numbered list: the numbers a seed is stamped into steel as.
    Numbers,
    /// A wallet that a tool can load.
    Wallet,
    /// A shield: the backup of a key.
    Shield,
    /// Paste: the clipboard read as a way into a scanner.
    Paste,
    /// Copy: the clipboard written from a public string.
    Copy,
    /// A keyboard: typing something in — a scanner's "Type" row, a
    /// wallet's "Set a name".
    Keyboard,
    /// Load a key: a key that arrives from a file, a card or a scan.
    Download,
    /// Layers: a multisig wallet built here.
    LayerGroup,
    /// An envelope: signing a message.
    Envelope,
    /// An opened envelope: checking a signed message.
    EnvelopeOpen,
    /// A receipt: decoding a transaction.
    Receipt,
    /// Export: the wallet configuration a coordinator reads.
    Export,
    /// A drive: a key kept on this device.
    Drive,
    /// An SD card: the removable medium on a device whose only one is a
    /// card, where another shows `Drive`.
    SdCard,
    /// A flag: "Start here".
    Flag,
    /// Compression: the CompactSeedQR.
    Compress,
    /// A grid: the SeedQR drawn by hand.
    Grid,
    /// A vault: the encrypted backup.
    Vault,
    /// Scissors: a seed split into parts.
    Scissors,
    /// A checked list: verifying a backup.
    Checklist,
    /// A hash.
    Hashtag,
    /// Encodings.
    Code,
    /// Two checks: a descriptor checksum.
    CheckDouble,
    /// A graph of nodes: miniscript spend paths.
    Diagram,
    /// A raised digit: the bits behind the words.
    Superscript,
    /// More of a list that has no end.
    Ellipsis,
    /// A masked figure: the duress PIN.
    UserSecret,
}

impl Icon {
    /// Every icon, for the gallery and for the completeness test.
    pub const ALL: [Icon; 62] = [
        Icon::Scan,
        Icon::Keys,
        Icon::Sign,
        Icon::Verify,
        Icon::Explore,
        Icon::Learn,
        Icon::Settings,
        Icon::Tools,
        Icon::Back,
        Icon::Check,
        Icon::Done,
        Icon::ChevronLeft,
        Icon::ChevronRight,
        Icon::ChevronUp,
        Icon::House,
        Icon::Power,
        Icon::Swap,
        Icon::Info,
        Icon::Success,
        Icon::Warning,
        Icon::Error,
        Icon::Derivation,
        Icon::Fingerprint,
        Icon::Passphrase,
        Icon::Bitcoin,
        Icon::Qr,
        Icon::Delete,
        Icon::Space,
        Icon::Lock,
        Icon::Dice,
        Icon::File,
        Icon::Eye,
        Icon::EyeOff,
        Icon::Trash,
        Icon::List,
        Icon::Numbers,
        Icon::Wallet,
        Icon::Shield,
        Icon::Paste,
        Icon::Copy,
        Icon::Keyboard,
        Icon::Download,
        Icon::LayerGroup,
        Icon::Envelope,
        Icon::EnvelopeOpen,
        Icon::Receipt,
        Icon::Export,
        Icon::Drive,
        Icon::SdCard,
        Icon::Flag,
        Icon::Compress,
        Icon::Grid,
        Icon::Vault,
        Icon::Scissors,
        Icon::Checklist,
        Icon::Hashtag,
        Icon::Code,
        Icon::CheckDouble,
        Icon::Diagram,
        Icon::Superscript,
        Icon::Ellipsis,
        Icon::UserSecret,
    ];

    /// The code point in the baked icon face.
    pub const fn code_point(self) -> char {
        match self {
            Icon::Scan => '\u{e900}',
            Icon::Keys => '\u{e901}',
            Icon::Settings => '\u{e902}',
            Icon::Tools => '\u{e903}',
            Icon::Back => '\u{e904}',
            Icon::Check => '\u{e905}',
            Icon::ChevronLeft => '\u{e909}',
            Icon::ChevronRight => '\u{e90a}',
            Icon::ChevronUp => '\u{e90b}',
            Icon::Power => '\u{e910}',
            Icon::Info => '\u{e912}',
            Icon::Success => '\u{e913}',
            Icon::Warning => '\u{e914}',
            Icon::Error => '\u{e915}',
            Icon::Derivation => '\u{e918}',
            Icon::Fingerprint => '\u{e91a}',
            Icon::Passphrase => '\u{e91b}',
            Icon::Bitcoin => '\u{e91d}',
            Icon::Qr => '\u{e920}',
            Icon::Sign => '\u{e921}',
            Icon::Delete => '\u{e922}',
            Icon::Space => '\u{e923}',
            Icon::Vault => '\u{e2c5}',
            Icon::Verify => '\u{f002}',
            Icon::Grid => '\u{f00a}',
            Icon::Done => '\u{f00c}',
            Icon::House => '\u{f015}',
            Icon::Download => '\u{f019}',
            Icon::Lock => '\u{f023}',
            Icon::Flag => '\u{f024}',
            Icon::Learn => '\u{f02d}',
            Icon::List => '\u{f03a}',
            Icon::Compress => '\u{f066}',
            Icon::Eye => '\u{f06e}',
            Icon::EyeOff => '\u{f070}',
            Icon::Drive => '\u{f0a0}',
            Icon::Checklist => '\u{f0ae}',
            Icon::Scissors => '\u{f0c4}',
            Icon::Copy => '\u{f0c5}',
            Icon::Numbers => '\u{f0cb}',
            Icon::Envelope => '\u{f0e0}',
            Icon::Paste => '\u{f0ea}',
            Icon::Keyboard => '\u{f11c}',
            Icon::Code => '\u{f121}',
            Icon::Superscript => '\u{f12b}',
            Icon::Ellipsis => '\u{f141}',
            Icon::Explore => '\u{f14e}',
            Icon::File => '\u{f15b}',
            Icon::Trash => '\u{f1f8}',
            Icon::UserSecret => '\u{f21b}',
            Icon::Hashtag => '\u{f292}',
            Icon::Swap => '\u{f362}',
            Icon::Shield => '\u{f3ed}',
            Icon::Diagram => '\u{f542}',
            Icon::Receipt => '\u{f543}',
            Icon::Dice => '\u{f522}',
            Icon::Wallet => '\u{f555}',
            Icon::CheckDouble => '\u{f560}',
            Icon::Export => '\u{f56e}',
            Icon::LayerGroup => '\u{f5fd}',
            Icon::EnvelopeOpen => '\u{f658}',
            Icon::SdCard => '\u{f7c2}',
        }
    }

    /// A short name, for the gallery.
    pub const fn name(self) -> &'static str {
        match self {
            Icon::Scan => "Scan",
            Icon::Keys => "Keys",
            Icon::Sign => "Sign",
            Icon::Verify => "Verify",
            Icon::Explore => "Explore",
            Icon::Learn => "Learn",
            Icon::Settings => "Settings",
            Icon::Tools => "Tools",
            Icon::Back => "Back",
            Icon::Check => "Check",
            Icon::Done => "Done",
            Icon::ChevronLeft => "ChevronLeft",
            Icon::ChevronRight => "ChevronRight",
            Icon::ChevronUp => "ChevronUp",
            Icon::House => "House",
            Icon::Power => "Power",
            Icon::Swap => "Swap",
            Icon::Info => "Info",
            Icon::Success => "Success",
            Icon::Warning => "Warning",
            Icon::Error => "Error",
            Icon::Derivation => "Derivation",
            Icon::Fingerprint => "Fingerprint",
            Icon::Passphrase => "Passphrase",
            Icon::Bitcoin => "Bitcoin",
            Icon::Qr => "Qr",
            Icon::Delete => "Delete",
            Icon::Space => "Space",
            Icon::Lock => "Lock",
            Icon::Dice => "Dice",
            Icon::File => "File",
            Icon::Eye => "Eye",
            Icon::EyeOff => "EyeOff",
            Icon::Trash => "Trash",
            Icon::List => "List",
            Icon::Numbers => "Numbers",
            Icon::Wallet => "Wallet",
            Icon::Shield => "Shield",
            Icon::Paste => "Paste",
            Icon::Copy => "Copy",
            Icon::Keyboard => "Keyboard",
            Icon::Download => "Download",
            Icon::LayerGroup => "LayerGroup",
            Icon::Envelope => "Envelope",
            Icon::EnvelopeOpen => "EnvelopeOpen",
            Icon::Receipt => "Receipt",
            Icon::Export => "Export",
            Icon::Drive => "Drive",
            Icon::SdCard => "SdCard",
            Icon::Flag => "Flag",
            Icon::Compress => "Compress",
            Icon::Grid => "Grid",
            Icon::Vault => "Vault",
            Icon::Scissors => "Scissors",
            Icon::Checklist => "Checklist",
            Icon::Hashtag => "Hashtag",
            Icon::Code => "Code",
            Icon::CheckDouble => "CheckDouble",
            Icon::Diagram => "Diagram",
            Icon::Superscript => "Superscript",
            Icon::Ellipsis => "Ellipsis",
            Icon::UserSecret => "UserSecret",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::Family;

    #[test]
    fn every_icon_has_an_outline() {
        let icons = crate::fonts::outlines(Family::Icon);
        for icon in Icon::ALL {
            let g = icons
                .glyph(icon.code_point())
                .unwrap_or_else(|| panic!("{} is missing from the icon face", icon.name()));
            assert!(
                !g.outline.commands.is_empty(),
                "{} has no outline to fill",
                icon.name()
            );
        }
    }

    #[test]
    fn code_points_are_unique() {
        for (i, a) in Icon::ALL.iter().enumerate() {
            for b in &Icon::ALL[i + 1..] {
                assert_ne!(a.code_point(), b.code_point(), "{a:?} and {b:?}");
            }
        }
    }
}
