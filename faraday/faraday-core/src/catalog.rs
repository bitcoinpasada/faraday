//! Tools: every flow Faraday has, on one page, by what it works on, each
//! with the standards it follows. A tile opens its flow, or says what it
//! needs first. "Find a tool" narrows the page to the tiles whose name,
//! line, group or standards hold what is typed.

use crate::create::NewKind;
use crate::forms::Form;
use crate::vaults::VaultAction;
use crate::wallet::FileKind;
use crate::{Action, Faraday, Screen};

/// Where a tile leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Go {
    /// New key, words.
    NewKey,
    /// New key, SLIP-39 shares.
    NewShares,
    /// Add a key, in this form.
    AddKey(Form),
    /// Scan a SeedQR into Add a key.
    SeedQr,
    /// Explore a key.
    Explore,
    /// BIP-85.
    Bip85,
    /// A vanity address.
    Vanity,
    /// Silent payments.
    Silent,
    /// The Lightning node key.
    Lightning,
    /// Create a wallet of this kind.
    Create(NewKind),
    /// Restore a wallet.
    Restore,
    /// Back up the wallet open.
    Backup,
    /// Sign a transaction: the PSBT in Files, or a scan.
    Sign,
    /// The nonce check, at the end of signing.
    NonceCheck,
    /// Decode a transaction.
    Decode,
    /// Scan an address to check against the loaded wallets.
    CheckAddress,
    /// Sign a message.
    SignMessage,
    /// Check a signed message.
    CheckMessage,
    /// The vault list.
    Vaults,
    /// Create a vault (its passphrases are rolled with dice there).
    CreateVault,
    /// An open vault's category, by place in `vaults::CATEGORIES`.
    VaultCategory(usize),
    /// Open an OpenSigner backup in the Inbox.
    OpenBackup,
    /// One of OpenSigner's calculators, by place in `Tool::ALL`.
    Calculator(u8),
    /// Scan a QR code into Files.
    Scan,
    /// A word list, by place in `wordlist::LISTS`.
    WordList(u8),
}

/// One tile.
pub struct Tile {
    /// The group it is listed in.
    pub group: &'static str,
    /// Its name.
    pub name: &'static str,
    /// One line on what it does.
    pub line: &'static str,
    /// The standards it follows.
    pub tags: &'static [&'static str],
    /// Where it leads.
    pub go: Go,
}

/// The groups, in order.
pub const GROUPS: [&str; 8] = [
    "Keys and seeds",
    "Wallets",
    "Transactions",
    "Messages",
    "Vaults and other keys",
    "Calculators",
    "Word lists",
    "QR",
];

/// Every tile, by group.
pub const TILES: &[Tile] = &[
    Tile {
        group: "Keys and seeds",
        name: "New key",
        line: "Dice, coins, cards, hex, camera or a mix",
        tags: &["BIP-39"],
        go: Go::NewKey,
    },
    Tile {
        group: "Keys and seeds",
        name: "Add a key",
        line: "Type its words, any BIP-39 list, with a passphrase",
        tags: &["BIP-39"],
        go: Go::AddKey(Form::Words),
    },
    Tile {
        group: "Keys and seeds",
        name: "Scan a SeedQR",
        line: "Standard or compact",
        tags: &["SeedQR"],
        go: Go::SeedQr,
    },
    Tile {
        group: "Keys and seeds",
        name: "SLIP-39 shares",
        line: "A new key dealt as m of n shares",
        tags: &["SLIP-39"],
        go: Go::NewShares,
    },
    Tile {
        group: "Keys and seeds",
        name: "Restore from SLIP-39",
        line: "Shares typed or scanned back into a key",
        tags: &["SLIP-39"],
        go: Go::AddKey(Form::Slip39),
    },
    Tile {
        group: "Keys and seeds",
        name: "codex32",
        line: "A secret or its shares typed into a key",
        tags: &["BIP-93"],
        go: Go::AddKey(Form::Codex32),
    },
    Tile {
        group: "Keys and seeds",
        name: "Seed XOR",
        line: "Parts combined into a key",
        tags: &["Seed XOR", "BIP-39"],
        go: Go::AddKey(Form::Xor),
    },
    Tile {
        group: "Keys and seeds",
        name: "Explore a key",
        line: "Its xpubs and addresses at any path",
        tags: &["BIP-32", "BIP-44", "BIP-84", "BIP-86"],
        go: Go::Explore,
    },
    Tile {
        group: "Keys and seeds",
        name: "Child seeds and passwords",
        line: "Derived from a key here",
        tags: &["BIP-85"],
        go: Go::Bip85,
    },
    Tile {
        group: "Keys and seeds",
        name: "Vanity address",
        line: "A first address that begins your way",
        tags: &["BIP-32", "BIP-84", "BIP-86"],
        go: Go::Vanity,
    },
    Tile {
        group: "Keys and seeds",
        name: "Silent payments",
        line: "One address to give out",
        tags: &["BIP-352"],
        go: Go::Silent,
    },
    Tile {
        group: "Keys and seeds",
        name: "Lightning node key",
        line: "From a key here, or an LND aezeed",
        tags: &["aezeed", "ldk-node"],
        go: Go::Lightning,
    },
    Tile {
        group: "Wallets",
        name: "Single-key wallet",
        line: "Native SegWit, Taproot, nested or legacy",
        tags: &["BIP-44", "BIP-49", "BIP-84", "BIP-86"],
        go: Go::Create(NewKind::NativeSegwit),
    },
    Tile {
        group: "Wallets",
        name: "Multisig",
        line: "k of n keys, a quorum signs",
        tags: &["BIP-48", "BIP-67", "BIP-383"],
        go: Go::Create(NewKind::Multi),
    },
    Tile {
        group: "Wallets",
        name: "Taproot multisig",
        line: "k of n in one script leaf",
        tags: &["BIP-341", "BIP-342", "BIP-386", "BIP-387"],
        go: Go::Create(NewKind::TapMulti),
    },
    Tile {
        group: "Wallets",
        name: "MuSig2",
        line: "All keys, one signature on chain",
        tags: &["BIP-327", "BIP-328", "BIP-373", "BIP-390"],
        go: Go::Create(NewKind::MuSig),
    },
    Tile {
        group: "Wallets",
        name: "FROST threshold",
        line: "Any m of n shares, one signature on chain",
        tags: &["BIP-445", "BIP-340"],
        go: Go::Create(NewKind::Threshold),
    },
    Tile {
        group: "Wallets",
        name: "Miniscript and inheritance",
        line: "Load one from its descriptor",
        tags: &["BIP-379", "BIP-382"],
        go: Go::Restore,
    },
    Tile {
        group: "Wallets",
        name: "Restore a wallet",
        line: "Descriptor, wallet file, BIP 129 record or split sheets",
        tags: &["BIP-380", "BIP-388", "BIP-389", "BIP-129"],
        go: Go::Restore,
    },
    Tile {
        group: "Wallets",
        name: "Back up a wallet",
        line: "Seeds by hand, sheets, envelopes, codex32 shares",
        tags: &["SeedQR", "BIP-93", "Seed XOR"],
        go: Go::Backup,
    },
    Tile {
        group: "Transactions",
        name: "Sign a transaction",
        line: "A PSBT from Files or a QR code",
        tags: &["BIP-174", "BIP-370", "BIP-143", "BIP-341"],
        go: Go::Sign,
    },
    Tile {
        group: "Transactions",
        name: "Nonce check",
        line: "Every signature recomputed and checked",
        tags: &["RFC 6979", "BIP-340"],
        go: Go::NonceCheck,
    },
    Tile {
        group: "Transactions",
        name: "Decode a transaction",
        line: "What it spends and pays, field by field",
        tags: &["BIP-141", "BIP-144"],
        go: Go::Decode,
    },
    Tile {
        group: "Transactions",
        name: "Check an address",
        line: "Scan it; the loaded wallets are searched",
        tags: &["BIP-173", "BIP-350"],
        go: Go::CheckAddress,
    },
    Tile {
        group: "Messages",
        name: "Sign a message",
        line: "With a single-key wallet's key here",
        tags: &["BIP-322", "BIP-137"],
        go: Go::SignMessage,
    },
    Tile {
        group: "Messages",
        name: "Check a signed message",
        line: "Against its address",
        tags: &["BIP-322", "BIP-137"],
        go: Go::CheckMessage,
    },
    Tile {
        group: "Vaults and other keys",
        name: "Vaults",
        line: "Keys, wallets, entries and notes under passphrases",
        tags: &["Argon2id", "XChaCha20-Poly1305"],
        go: Go::Vaults,
    },
    Tile {
        group: "Vaults and other keys",
        name: "Diceware passphrase",
        line: "Rolled with dice in Create a vault",
        tags: &["EFF lists"],
        go: Go::CreateVault,
    },
    Tile {
        group: "Vaults and other keys",
        name: "GPG key",
        line: "Make, sign, revoke, paperkey",
        tags: &["OpenPGP", "Ed25519"],
        go: Go::VaultCategory(4),
    },
    Tile {
        group: "Vaults and other keys",
        name: "Secure Boot keys",
        line: "PK, KEK and db; sign an EFI image",
        tags: &["UEFI", "Authenticode"],
        go: Go::VaultCategory(5),
    },
    Tile {
        group: "Vaults and other keys",
        name: "KeePass export",
        line: "A vault entry as a sealed database",
        tags: &["KDBX 4"],
        go: Go::VaultCategory(2),
    },
    Tile {
        group: "Vaults and other keys",
        name: "Open an OpenSigner backup",
        line: "An .oskb with its passphrase",
        tags: &[".oskb"],
        go: Go::OpenBackup,
    },
    Tile {
        group: "Calculators",
        name: "Hashes",
        line: "SHA-256, SHA-256d and HASH160",
        tags: &["SHA-256"],
        go: Go::Calculator(0),
    },
    Tile {
        group: "Calculators",
        name: "Encodings",
        line: "What a string is encoded in, and its value",
        tags: &["Base58", "Bech32"],
        go: Go::Calculator(1),
    },
    Tile {
        group: "Calculators",
        name: "Descriptor checksum",
        line: "A descriptor's checksum, and whether it holds",
        tags: &["BIP-380"],
        go: Go::Calculator(2),
    },
    Tile {
        group: "Calculators",
        name: "Convert an xpub",
        line: "Every spelling of one extended key",
        tags: &["SLIP-132"],
        go: Go::Calculator(3),
    },
    Tile {
        group: "Calculators",
        name: "Units",
        line: "One amount in all four units",
        tags: &["BTC", "sat"],
        go: Go::Calculator(4),
    },
    Tile {
        group: "Calculators",
        name: "Miniscript compiler",
        line: "A policy compiled into a descriptor",
        tags: &["BIP-379"],
        go: Go::Calculator(5),
    },
    Tile {
        group: "Word lists",
        name: "BIP-39 words",
        line: "2048 words, each with its number and its 11 bits",
        tags: &["BIP-39"],
        go: Go::WordList(0),
    },
    Tile {
        group: "Word lists",
        name: "EFF large list",
        line: "7776 words, each with its number and its five dice",
        tags: &["EFF lists", "Diceware"],
        go: Go::WordList(1),
    },
    Tile {
        group: "Word lists",
        name: "EFF short list 1",
        line: "1296 words, each with its number and its four dice",
        tags: &["EFF lists", "Diceware"],
        go: Go::WordList(2),
    },
    Tile {
        group: "Word lists",
        name: "EFF short list 2",
        line: "1296 words with unique three-letter starts, each with its number and its four dice",
        tags: &["EFF lists", "Diceware"],
        go: Go::WordList(3),
    },
    Tile {
        group: "QR",
        name: "Scan a QR code",
        line: "Single or animated, into Files",
        tags: &["BC-UR", "BBQr"],
        go: Go::Scan,
    },
];

/// Letters, digits and spaces, lower-cased: punctuation like the hyphen
/// in "BIP-85" is dropped, so it does not have to be typed to find a
/// tile. "bip85" and "bip-85" then search alike.
fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether a tile matches what is typed in Find a tool: every word of it
/// in the name, line, group or a standard; "85" finds BIP-85.
pub fn matches(tile: &Tile, find: &str) -> bool {
    let hay = fold(&format!(
        "{} {} {} {}",
        tile.name,
        tile.line,
        tile.group,
        tile.tags.join(" ")
    ));
    find.split_whitespace().all(|w| hay.contains(&fold(w)))
}

impl Faraday {
    /// What a tile needs first, when it cannot open now.
    pub fn tile_need(&self, go: Go) -> Option<&'static str> {
        let keys = !self.session.keys.is_empty();
        let may = self.may_load_keys();
        let need = match go {
            Go::NewKey | Go::NewShares | Go::AddKey(_) | Go::SeedQr if !may => {
                "Remove the stick first"
            }
            Go::Explore | Go::Bip85 | Go::Vanity | Go::Silent if !keys => "Load a key first",
            Go::Backup if self.session.wallets.get(self.wallet).is_none() => "Load a wallet first",
            Go::NonceCheck
                if !self
                    .spend
                    .as_ref()
                    .is_some_and(|s| s.spend.finished.is_some()) =>
            {
                "At the end of signing"
            }
            Go::CheckAddress if self.session.wallets.is_empty() => "Load a wallet first",
            Go::SignMessage if self.session.message_wallets().is_empty() => {
                "Needs a single-key wallet with its key here"
            }
            Go::CheckMessage if !self.inbox.iter().any(|i| i.kind == FileKind::Message) => {
                "Copy a signed message in first"
            }
            Go::VaultCategory(_) if self.vaults.open.is_empty() => "Unlock a vault first",
            Go::OpenBackup if !self.inbox.iter().any(|i| i.kind == FileKind::Backup) => {
                "Copy an .oskb in first"
            }
            _ => return None,
        };
        Some(need)
    }

    /// Opens tile `i`'s flow.
    pub(crate) fn catalog_go(&mut self, i: usize) {
        let Some(tile) = TILES.get(i) else {
            return;
        };
        if self.tile_need(tile.go).is_some() {
            return;
        }
        match tile.go {
            Go::NewKey => self.act(Action::KeyGen(None)),
            Go::NewShares => self.act(Action::KeyGenSlip39),
            Go::AddKey(form) => {
                self.act(Action::Entry(None));
                if let Some(k) = Form::ALL.iter().position(|f| *f == form) {
                    self.act(Action::EntryForm(k as u8));
                }
            }
            Go::SeedQr => {
                self.act(Action::Entry(None));
                self.act(Action::ScanSeed);
            }
            Go::Explore => self.act(Action::Explore),
            Go::Bip85 => self.act(Action::Bip85),
            Go::Vanity => self.act(Action::Vanity(crate::vanity::VanityAction::Open)),
            Go::Silent => self.act(Action::Silent),
            Go::Lightning => self.act(Action::Lightning),
            Go::Create(kind) => {
                self.act(Action::CreateWallet);
                if let Some(k) = NewKind::ALL.iter().position(|x| *x == kind) {
                    self.act(Action::CKind(k as u8));
                }
            }
            Go::Restore => self.act(Action::RestoreWallet),
            Go::Backup => self.act(Action::Backup(self.wallet)),
            Go::Sign => match self.lead_psbt() {
                Some(k) => self.act(Action::StartSpend(k)),
                None => self.act(Action::Scan),
            },
            Go::NonceCheck => {
                if let Some(s) = self.spend.as_mut() {
                    s.open = Some(crate::wallet::step::FINISH);
                    s.follow = true;
                }
                self.act(Action::Nav(Screen::Spend));
            }
            Go::Decode => match self
                .inbox
                .iter()
                .position(|i| i.kind == FileKind::Transaction)
            {
                Some(k) => self.act(Action::DecodeInbox(k)),
                None => self.act(Action::Scan),
            },
            Go::CheckAddress | Go::Scan => self.act(Action::Scan),
            Go::SignMessage => self.act(Action::SignMessage),
            Go::CheckMessage => {
                if let Some(k) = self.inbox.iter().position(|i| i.kind == FileKind::Message) {
                    self.act(Action::CheckMessage(k));
                }
            }
            Go::Vaults => self.act(Action::Nav(Screen::Vaults)),
            Go::CreateVault => self.act(Action::Vault(VaultAction::Create)),
            Go::VaultCategory(c) => {
                self.vaults.current = 0;
                self.act(Action::Nav(Screen::VaultContents));
                self.act(Action::Vault(VaultAction::Category(c)));
            }
            Go::OpenBackup => {
                if let Some(k) = self.inbox.iter().position(|i| i.kind == FileKind::Backup) {
                    self.act(Action::BackupOpen(k));
                }
            }
            Go::WordList(k) => self.act(Action::WordList(crate::wordlist::WordListAction::Open(
                k, None,
            ))),
            Go::Calculator(t) => {
                self.act(Action::Tools);
                self.act(Action::TTool(t));
            }
        }
    }

    /// Typing on the Tools page goes to Find a tool. Returns whether the
    /// key was taken.
    pub(crate) fn catalog_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
        if self.screen != Screen::Catalog {
            return false;
        }
        match key {
            K::Char(c) if !c.is_control() && self.catalog_find.chars().count() < 40 => {
                self.catalog_find.push(c);
            }
            K::Backspace => {
                self.catalog_find.pop();
            }
            K::Escape => self.catalog_find.clear(),
            _ => return false,
        }
        self.list_offset = 0.0;
        true
    }
}
