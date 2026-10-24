//! Tools: every flow Faraday has, on one page, by what it works on, each
//! with the standards it follows. A tile opens its flow, or says what it
//! needs first. "Find a tool" narrows the page to the tiles whose name,
//! line, group or standards hold what is typed.

use crate::create::NewKind;
use crate::forms::Form;
use crate::strings::EN;
use crate::vault::VaultAction;
use crate::wallet::FileKind;
use crate::{Action, Faraday, Screen};

/// Where a tile leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Go {
    /// Add a key: New key's flow, opening on Start.
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
    /// An open vault's category, by place in `vault::CATEGORIES`.
    VaultCategory(usize),
    /// TOTP codes for the entries in open vaults (`docs/TOTP.md` §5).
    Codes,
    /// One of the calculators, by place in `Tool::ALL`.
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
    EN.catalog_keys_and_seeds,
    EN.catalog_wallets,
    EN.catalog_transactions,
    EN.catalog_messages,
    EN.catalog_vaults_group,
    EN.catalog_calculators,
    EN.catalog_word_lists,
    EN.catalog_qr,
];

/// Every tile, by group.
pub const TILES: &[Tile] = &[
    // One tile for making a key and for typing one: Start offers both
    // (owner, 2026-10-24).
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_add_a_key,
        line: EN.catalog_make_or_type,
        tags: &["BIP-39"],
        go: Go::NewKey,
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_scan_a_seedqr,
        line: EN.catalog_scan_seedqr_line,
        tags: &["SeedQR"],
        go: Go::SeedQr,
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_slip_39_shares,
        line: EN.catalog_a_new_key_dealt,
        tags: &["SLIP-39"],
        go: Go::NewShares,
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_restore_slip39,
        line: EN.catalog_shares_typed,
        tags: &["SLIP-39"],
        go: Go::AddKey(Form::Slip39),
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: "codex32",
        line: EN.catalog_codex32_line,
        tags: &["BIP-93"],
        go: Go::AddKey(Form::Codex32),
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_seed_xor,
        line: EN.catalog_parts_combined,
        tags: &["Seed XOR", "BIP-39"],
        go: Go::AddKey(Form::Xor),
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_explore_a_key,
        line: EN.catalog_explore_key_line,
        tags: &["BIP-32", "BIP-44", "BIP-84", "BIP-86"],
        go: Go::Explore,
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_child_seeds,
        line: EN.catalog_child_seeds_line,
        tags: &["BIP-85"],
        go: Go::Bip85,
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_vanity_address,
        line: EN.catalog_a_first_address,
        tags: &["BIP-32", "BIP-84", "BIP-86"],
        go: Go::Vanity,
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_silent_payments,
        line: EN.catalog_one_address,
        tags: &["BIP-352"],
        go: Go::Silent,
    },
    Tile {
        group: EN.catalog_keys_and_seeds,
        name: EN.catalog_lightning_node,
        line: EN.catalog_from_a_key,
        tags: &["aezeed", "ldk-node"],
        go: Go::Lightning,
    },
    Tile {
        group: EN.catalog_wallets,
        name: EN.catalog_single_key,
        line: EN.catalog_native_segwit,
        tags: &["BIP-44", "BIP-49", "BIP-84", "BIP-86"],
        go: Go::Create(NewKind::NativeSegwit),
    },
    Tile {
        group: EN.catalog_wallets,
        name: EN.catalog_multisig,
        line: EN.catalog_k_of_n_keys,
        tags: &["BIP-48", "BIP-67", "BIP-383"],
        go: Go::Create(NewKind::Multi),
    },
    Tile {
        group: EN.catalog_wallets,
        name: EN.catalog_taproot_multisig,
        line: EN.catalog_k_of_n_in_one,
        tags: &["BIP-341", "BIP-342", "BIP-386", "BIP-387"],
        go: Go::Create(NewKind::TapMulti),
    },
    Tile {
        group: EN.catalog_wallets,
        name: "MuSig2",
        line: EN.catalog_all_keys_one,
        tags: &["BIP-327", "BIP-328", "BIP-373", "BIP-390"],
        go: Go::Create(NewKind::MuSig),
    },
    // FROST is compiled out of the app (F109).
    #[cfg(feature = "unreleased")]
    Tile {
        group: EN.catalog_wallets,
        name: EN.catalog_frost_threshold,
        line: EN.catalog_any_m_of_n,
        tags: &["BIP-445", "BIP-340"],
        go: Go::Create(NewKind::Threshold),
    },
    Tile {
        group: EN.catalog_wallets,
        name: EN.catalog_miniscript_inheritance,
        line: EN.catalog_miniscript_line,
        tags: &["BIP-379", "BIP-382"],
        go: Go::Restore,
    },
    Tile {
        group: EN.catalog_wallets,
        name: EN.catalog_restore_wallet,
        line: EN.catalog_descriptor_wallet,
        tags: &["BIP-380", "BIP-388", "BIP-389", "BIP-129"],
        go: Go::Restore,
    },
    Tile {
        group: EN.catalog_wallets,
        name: EN.catalog_back_up_a_wallet,
        line: EN.catalog_seeds_by_hand,
        tags: &["SeedQR", "BIP-93", "Seed XOR"],
        go: Go::Backup,
    },
    Tile {
        group: EN.catalog_transactions,
        name: EN.catalog_sign_a_transaction,
        line: EN.catalog_sign_transaction_line,
        tags: &["BIP-174", "BIP-370", "BIP-143", "BIP-341"],
        go: Go::Sign,
    },
    Tile {
        group: EN.catalog_transactions,
        name: EN.catalog_nonce_check,
        line: EN.catalog_every_signature,
        tags: &["RFC 6979", "BIP-340"],
        go: Go::NonceCheck,
    },
    Tile {
        group: EN.catalog_transactions,
        name: EN.catalog_decode_a_transaction,
        line: EN.catalog_what_it_spends,
        tags: &["BIP-141", "BIP-144"],
        go: Go::Decode,
    },
    Tile {
        group: EN.catalog_transactions,
        name: EN.catalog_check_an_address,
        line: EN.catalog_check_address_line,
        tags: &["BIP-173", "BIP-350"],
        go: Go::CheckAddress,
    },
    Tile {
        group: EN.catalog_messages,
        name: EN.catalog_sign_a_message,
        line: EN.catalog_with_a_single,
        tags: &["BIP-322", "BIP-137"],
        go: Go::SignMessage,
    },
    Tile {
        group: EN.catalog_messages,
        name: EN.catalog_check_a_signed,
        line: EN.catalog_check_signed_line,
        tags: &["BIP-322", "BIP-137"],
        go: Go::CheckMessage,
    },
    Tile {
        group: EN.catalog_vaults_group,
        name: EN.catalog_vaults,
        line: EN.catalog_keys_wallets,
        tags: &["Argon2id", "XChaCha20-Poly1305"],
        go: Go::Vaults,
    },
    Tile {
        group: EN.catalog_vaults_group,
        name: EN.catalog_totp_codes,
        line: EN.catalog_totp_codes_line,
        tags: &["RFC 6238"],
        go: Go::Codes,
    },
    Tile {
        group: EN.catalog_vaults_group,
        name: EN.catalog_diceware_passphrase,
        line: EN.catalog_diceware_line,
        tags: &["EFF lists"],
        go: Go::CreateVault,
    },
    Tile {
        group: EN.catalog_vaults_group,
        name: EN.catalog_gpg_key,
        line: EN.catalog_make_sign_revoke,
        tags: &["OpenPGP", "Ed25519"],
        go: Go::VaultCategory(4),
    },
    Tile {
        group: EN.catalog_vaults_group,
        name: EN.catalog_secure_boot,
        line: EN.catalog_secure_boot_line,
        tags: &["UEFI", "Authenticode"],
        go: Go::VaultCategory(5),
    },
    Tile {
        group: EN.catalog_vaults_group,
        name: EN.catalog_keepass_export,
        line: EN.catalog_a_vault_entry,
        tags: &["KDBX 4"],
        go: Go::VaultCategory(2),
    },
    Tile {
        group: EN.catalog_calculators,
        name: EN.catalog_hashes,
        line: EN.catalog_sha_256_sha,
        tags: &["SHA-256"],
        go: Go::Calculator(0),
    },
    Tile {
        group: EN.catalog_calculators,
        name: EN.catalog_encodings,
        line: EN.catalog_what_a_string,
        tags: &["Base58", "Bech32"],
        go: Go::Calculator(1),
    },
    Tile {
        group: EN.catalog_calculators,
        name: EN.catalog_descriptor,
        line: EN.catalog_a_descriptor,
        tags: &["BIP-380"],
        go: Go::Calculator(2),
    },
    Tile {
        group: EN.catalog_calculators,
        name: EN.catalog_convert_xpub,
        line: EN.catalog_every_spelling,
        tags: &["SLIP-132"],
        go: Go::Calculator(3),
    },
    Tile {
        group: EN.catalog_calculators,
        name: EN.catalog_units,
        line: EN.catalog_one_amount,
        tags: &["BTC", "sat"],
        go: Go::Calculator(4),
    },
    Tile {
        group: EN.catalog_calculators,
        name: EN.catalog_miniscript,
        line: EN.catalog_a_policy_compiled,
        tags: &["BIP-379"],
        go: Go::Calculator(5),
    },
    Tile {
        group: EN.catalog_word_lists,
        name: EN.catalog_bip_39_words,
        line: EN.catalog_2048_words,
        tags: &["BIP-39"],
        go: Go::WordList(0),
    },
    Tile {
        group: EN.catalog_word_lists,
        name: EN.catalog_eff_large_list,
        line: EN.catalog_7776_words,
        tags: &["EFF lists", "Diceware"],
        go: Go::WordList(1),
    },
    Tile {
        group: EN.catalog_word_lists,
        name: EN.catalog_eff_short_list_1,
        line: EN.catalog_1296_words_each,
        tags: &["EFF lists", "Diceware"],
        go: Go::WordList(2),
    },
    Tile {
        group: EN.catalog_word_lists,
        name: EN.catalog_eff_short_list_2,
        line: EN.catalog_1296_words,
        tags: &["EFF lists", "Diceware"],
        go: Go::WordList(3),
    },
    Tile {
        group: EN.catalog_qr,
        name: EN.catalog_scan_a_qr_code,
        line: EN.catalog_single_or_animated,
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
        // A tile that loads a key or scans opens with a stick attached too:
        // it waits for the stick to be pulled ([`crate::Sheet::Pull`]).
        let need = match go {
            Go::Explore | Go::Bip85 | Go::Vanity | Go::Silent if !keys => EN.catalog_load_a_key,
            Go::Backup if self.session.wallets.get(self.wallets.wallet()).is_none() => {
                EN.catalog_load_a_wallet
            }
            Go::NonceCheck
                if !self
                    .wallets
                    .spend()
                    .is_some_and(|s| s.spend.finished.is_some()) =>
            {
                EN.catalog_at_the_end
            }
            Go::CheckAddress if self.session.wallets.is_empty() => EN.catalog_load_a_wallet,
            Go::SignMessage if self.session.message_wallets().is_empty() => {
                EN.catalog_needs_a_single
            }
            Go::CheckMessage
                if !self.inbox.iter().any(|i| i.kind == FileKind::Message)
                    && self.put_on().is_none() =>
            {
                EN.catalog_copy_a_signed
            }
            // The keys the backup flow makes: the tile starts it, or resumes
            // it, and asks for no vault first (`docs/BACKUP-FLOW.md` §3).
            Go::VaultCategory(c)
                if crate::backup_flow::Kind::of_category(c)
                    .is_some_and(|k| self.flows.resumable(k).is_some()) =>
            {
                EN.backup_flow_continue_make
            }
            Go::VaultCategory(c) if crate::backup_flow::Kind::of_category(c).is_some() => {
                return None;
            }
            // Pressable: these two lead into making or unlocking one.
            Go::VaultCategory(_) | Go::Codes if self.vaults.open.is_empty() => {
                if self.vault_files().is_empty() {
                    EN.catalog_make_a_vault
                } else {
                    EN.catalog_unlock_a_vault
                }
            }
            _ => return None,
        };
        Some(need)
    }

    /// Whether a tile opens when pressed: it needs nothing first, or what
    /// it needs is a vault, which the press leads into making or
    /// unlocking.
    pub fn tile_opens(&self, go: Go) -> bool {
        matches!(go, Go::VaultCategory(_) | Go::Codes) || self.tile_need(go).is_none()
    }

    /// Opens tile `i`'s flow.
    pub(crate) fn catalog_go(&mut self, i: usize) {
        let Some(tile) = TILES.get(i) else {
            return;
        };
        if !self.tile_opens(tile.go) {
            return;
        }
        match tile.go {
            Go::NewKey => self.act(Action::KeyGen(None)),
            Go::NewShares => self.act(Action::KeyGenSlip39),
            // Add a key is New key opening on Start; a tile for one form
            // opens it with that choice made (`docs/history/NEW-WALLET.md` §15.3).
            Go::AddKey(form) => self.keygen_open_on(crate::keygen::Start::typing(form)),
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
            Go::Backup => self.act(Action::Backup(self.wallets.wallet())),
            Go::Sign => match self.lead_psbt() {
                Some(k) => self.act(Action::StartSpend(k)),
                None => self.act(Action::Scan),
            },
            Go::NonceCheck => {
                if let Some(s) = self.wallets.spend_mut() {
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
                } else {
                    // Demo: the sample message stands in for the one to copy in.
                    self.check_use_sample();
                }
            }
            Go::Vaults => self.act(Action::Nav(Screen::Vaults)),
            Go::CreateVault => self.act(Action::Vault(VaultAction::Create)),
            // No vault open: Create a vault or Unlock, which come back to
            // this category of the vault made or unlocked.
            // The keys the backup flow makes: the flow, when none is open
            // or one is waiting to be resumed.
            Go::VaultCategory(c)
                if crate::backup_flow::Kind::of_category(c).is_some_and(|k| {
                    self.vaults.open.is_empty() || self.flows.resumable(k).is_some()
                }) =>
            {
                if let Some(k) = crate::backup_flow::Kind::of_category(c) {
                    self.flow_begin(k);
                }
            }
            Go::VaultCategory(c) if self.vaults.open.is_empty() => {
                self.vaults.set_category(c);
                if let Some((_, a)) = self.vault_way(Screen::VaultContents) {
                    self.act(a);
                }
            }
            Go::VaultCategory(c) => {
                self.vaults.set_current(0);
                self.act(Action::Nav(Screen::VaultContents));
                self.act(Action::Vault(VaultAction::Category(c)));
            }
            // No vault open: Create a vault or Unlock, which come back to
            // the codes screen.
            Go::Codes if self.vaults.open.is_empty() => {
                if let Some((_, a)) = self.vault_way(Screen::Codes) {
                    self.act(a);
                }
            }
            Go::Codes => self.act(Action::OpenCodes),
            Go::WordList(k) => self.act(Action::WordList(crate::wordlist::WordListAction::Open(
                k, None,
            ))),
            Go::Calculator(t) => {
                self.act(Action::Tools);
                self.act(Action::TTool(t));
            }
        }
    }
}
