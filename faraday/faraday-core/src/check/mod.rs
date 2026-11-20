//! Check: what to hold up against a hardware wallet, as a grid of tiles
//! each opening the tool that does it, as Tools does (`docs/DESIGN.md` §5,
//! Check; F255). Tools stays the master list: a row is a second way in.

pub mod screen;

use crate::catalog::{Go, TILES};
use crate::forms::Form;
use crate::strings::EN;
use crate::{Action, Faraday, Screen, Sheet};

/// A row of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A seed against the wallet app: My seed makes my wallet.
    SeedCheck,
    /// A multisig wallet's setup: Check a multisig setup.
    Multisig,
    /// An address on its screen: Check an address.
    Address,
    /// A transaction before signing: Decode a transaction.
    Transaction,
    /// What it signed: Check a signer.
    Signed,
    /// Its signatures hide nothing: Nonce check.
    Signatures,
    /// A seed made from dice rolls: New key at its dice source.
    Dice,
    /// A backup in another form: a menu of the forms.
    Backup,
    /// A signed message: Check a signed message.
    Message,
    /// Child seeds: Child seeds and passwords.
    Child,
    /// An xpub's other spellings: Convert an xpub.
    Xpub,
}

/// A choice in the backup menu: a form of the table, or a SeedQR (a
/// writing of BIP-39 words, not a form of its own).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backup {
    /// A form of the table.
    Form(Form),
    /// A SeedQR.
    SeedQr,
}

impl Backup {
    /// Its label in the menu.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Backup::Form(f) => match f {
                Form::Slip39 => EN.check_menu_slip39,
                Form::Xor => EN.check_menu_xor,
                Form::Codex32 => EN.check_menu_codex32,
                Form::Krux => EN.check_menu_krux,
                // Typed on Add a key's own tile, so not in the menu.
                Form::Words => f.name(),
            },
            Backup::SeedQr => EN.check_menu_seedqr,
        }
    }
}

/// The choices in the backup menu, in order.
pub(crate) const FORMS: [Backup; 5] = [
    Backup::Form(Form::Slip39),
    Backup::Form(Form::Xor),
    Backup::SeedQr,
    Backup::Form(Form::Codex32),
    Backup::Form(Form::Krux),
];

/// A press on Check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// A row.
    Row(Row),
    /// A form in the backup menu.
    Form(Backup),
}

/// One row as it is drawn: its label, its line, whether it works from the
/// seed, and what a press does.
pub(crate) struct Item {
    /// The label.
    pub label: &'static str,
    /// What the person needs in hand.
    pub line: &'static str,
    /// Marked **Seed**.
    pub seed: bool,
    /// The row.
    pub row: Row,
}

/// The groups in order, each with its rows. A row whose tool is not built
/// is not here.
pub(crate) const GROUPS: [(&str, &[Item]); 4] = [
    (
        EN.check_group_wallet,
        &[
            Item {
                label: EN.seed_check_title,
                line: EN.check_seed_line,
                seed: true,
                row: Row::SeedCheck,
            },
            Item {
                label: EN.catalog_check_an_address,
                line: EN.check_address_line,
                seed: false,
                row: Row::Address,
            },
        ],
    ),
    (
        EN.check_group_signing,
        &[
            Item {
                label: EN.catalog_decode_a_transaction,
                line: EN.check_transaction_line,
                seed: false,
                row: Row::Transaction,
            },
            Item {
                label: EN.catalog_check_a_signer,
                line: EN.check_signed_line,
                seed: false,
                row: Row::Signed,
            },
            Item {
                label: EN.catalog_nonce_check,
                line: EN.check_signatures_line,
                seed: false,
                row: Row::Signatures,
            },
        ],
    ),
    (
        EN.check_group_setup,
        &[
            Item {
                label: EN.catalog_check_dice_seed,
                line: EN.check_dice_line,
                seed: true,
                row: Row::Dice,
            },
            Item {
                label: EN.mscheck_title,
                line: EN.check_multisig_line,
                seed: false,
                row: Row::Multisig,
            },
            Item {
                label: EN.catalog_check_a_backup,
                line: EN.check_backup_line,
                seed: true,
                row: Row::Backup,
            },
        ],
    ),
    (
        EN.check_group_messages,
        &[
            Item {
                label: EN.catalog_check_a_signed,
                line: EN.check_message_line,
                seed: false,
                row: Row::Message,
            },
            Item {
                label: EN.catalog_child_seeds,
                line: EN.check_child_line,
                seed: true,
                row: Row::Child,
            },
            Item {
                label: EN.catalog_convert_xpub,
                line: EN.check_xpub_line,
                seed: false,
                row: Row::Xpub,
            },
        ],
    ),
];

/// A press on Check.
pub(crate) fn press(app: &mut Faraday, p: Press) {
    match p {
        Press::Row(Row::Dice) => app.keygen_open_dice(),
        Press::Row(Row::Backup) => app.sheet = Some(Sheet::CheckBackups),
        Press::Row(r) => tile(
            app,
            match r {
                Row::SeedCheck => Go::SeedCheck,
                Row::Multisig => Go::MultisigCheck,
                Row::Address => Go::CheckAddress,
                Row::Transaction => Go::Decode,
                Row::Signed => Go::SignerCheck,
                Row::Signatures => Go::NonceCheck,
                Row::Message => Go::CheckMessage,
                Row::Child => Go::Bip85,
                Row::Xpub => Go::Calculator(3),
                Row::Dice | Row::Backup => return,
            },
        ),
        Press::Form(f) => {
            app.sheet = None;
            let go = match f {
                Backup::Form(Form::Krux) => {
                    // No tile and no choice on New key's Start card: the row
                    // opens Add a key on its Krux code form.
                    app.act(Action::Entry(None));
                    if app.screen == Screen::Entry {
                        app.entry.choose_form(Form::Krux);
                    }
                    return;
                }
                Backup::Form(Form::Slip39) => Go::AddKey(Form::Slip39),
                Backup::Form(Form::Xor) => Go::AddKey(Form::Xor),
                Backup::Form(Form::Codex32) => Go::AddKey(Form::Codex32),
                // The words are typed on Add a key and are not a row here.
                Backup::Form(Form::Words) => return,
                Backup::SeedQr => Go::SeedQr,
            };
            tile(app, go);
        }
    }
}

/// Opens the Tools tile that leads to `go`, as a press on it does; when
/// it needs something first, says what.
fn tile(app: &mut Faraday, go: Go) {
    let Some(i) = TILES.iter().position(|t| t.go == go) else {
        return;
    };
    if app.tile_opens(go) {
        app.act(Action::Catalog(i as u8));
    } else if let Some(need) = app.tile_need(go) {
        app.toast(need);
    }
}
