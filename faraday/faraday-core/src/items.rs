//! What each kind of held item offers (`docs/WORKSPACE.md` §2, "Row
//! buttons and what a row opens"): its row button, at most one, and its
//! "What you can do" list, likeliest first, both as existing [`Action`]s.
//! One list per item, read by Files' row menu now and by the session
//! pane's details later. Built where a menu opens, never per draw.

use crate::strings::EN;
use crate::wallet::PsbtSeen;
use crate::{Action, Faraday, FileKind, Item, Screen};

/// One thing to do: its label and its action.
pub(crate) type Pick = (String, Action);

/// A file row's button on the pane: its words and what it does, if its
/// kind has one.
pub(crate) type RowButton = Option<(&'static str, Action)>;

/// An item's list and whether its first entry is also its row button.
pub(crate) struct Offer {
    /// "What you can do", likeliest first.
    pub list: Vec<Pick>,
    /// The first entry of `list` is the row's button.
    button: bool,
}

impl Offer {
    fn none() -> Offer {
        Offer {
            list: Vec::new(),
            button: false,
        }
    }

    fn with_button(list: Vec<Pick>) -> Offer {
        let button = !list.is_empty();
        Offer { list, button }
    }

    fn without_button(list: Vec<Pick>) -> Offer {
        Offer {
            list,
            button: false,
        }
    }

    /// The row's one button, if its kind has one.
    pub(crate) fn button(&self) -> Option<&Pick> {
        self.button.then(|| self.list.first()).flatten()
    }
}

/// The loaded wallet a PSBT spends from; the caller gives its own lookup
/// (drawing its kept one, event code the session's with a Compute), so
/// this works nothing out.
pub(crate) type SpendsFrom<'a> = &'a dyn Fn(&PsbtSeen) -> Option<usize>;

/// The loaded wallet an Inbox file describes, by its place; the caller
/// gives the Inbox's kept answer (as for [`SpendsFrom`]), `None` where it
/// is not there.
pub(crate) type LoadedAs<'a> = &'a dyn Fn(usize) -> Option<usize>;

/// The offer of Inbox file `k`, by its kind.
pub(crate) fn offer(
    app: &Faraday,
    k: usize,
    spends_from: SpendsFrom,
    loaded_as: LoadedAs,
) -> Offer {
    let Some(item) = app.inbox.get(k) else {
        return Offer::none();
    };
    match item.kind {
        FileKind::Psbt => psbt(app, spends_from, k, item),
        FileKind::Transaction => Offer::with_button(vec![decode(k), check_signer(k)]),
        FileKind::Wallet | FileKind::SecretWallet => wallet(app, k, item, loaded_as(k)),
        FileKind::Vault => vault(app, item),
        FileKind::Key | FileKind::KeyWithSecret | FileKind::KeyLine => {
            Offer::with_button(vec![use_key(k, item.kind)])
        }
        FileKind::Words => {
            Offer::with_button([add_key(k)].into_iter().chain(into_vault(app, k)).collect())
        }
        FileKind::SeedPart => Offer::without_button(into_vault(app, k).into_iter().collect()),
        FileKind::Share => Offer::with_button(vec![restore_shares(app)]),
        FileKind::Message => Offer::with_button(vec![(
            EN.inbox_actions_check.to_string(),
            Action::CheckMessage(k),
        )]),
        FileKind::Text => text(app, k, item),
        FileKind::KruxCode => Offer::with_button(vec![(
            EN.inbox_actions_open_code.to_string(),
            Action::KefFile(k),
        )]),
        FileKind::Gpg => Offer::with_button(gpg(k, item)),
        FileKind::CoreWallet => Offer::with_button(vec![(
            EN.inbox_actions_read_wallet.to_string(),
            Action::CoreWallet(crate::core_wallet::tool::CoreWalletAction::OpenFile(k)),
        )]),
        _ => Offer::none(),
    }
}

/// The words of a row button as the text the strings hold them in, for a
/// row that keeps them as such; none for a label made with a count (the
/// shares' Restore), which gives no row button.
pub(crate) fn button_label(label: &str) -> Option<&'static str> {
    [
        EN.inbox_actions_add_signatures,
        EN.inbox_actions_sign,
        EN.catalog_check_a_signer,
        EN.files_decode,
        EN.inbox_actions_load_wallet,
        EN.inbox_actions_load,
        EN.read_go_use_key,
        EN.inbox_actions_unlock,
        EN.inbox_actions_check,
        EN.inbox_actions_open_code,
        EN.inbox_actions_read_wallet,
        EN.gpg_restore_files_action,
    ]
    .into_iter()
    .find(|s| *s == label)
}

fn decode(k: usize) -> Pick {
    (
        EN.files_decode.to_string(),
        Action::File(crate::file_menu::FilePress::Decode(
            crate::file_menu::Row::In(k),
        )),
    )
}

fn check_signer(k: usize) -> Pick {
    (
        EN.catalog_check_a_signer.to_string(),
        Action::SignerCheck(crate::decode::check::SignerPress::File(false, k)),
    )
}

/// Add to vault, only while a vault is open (F60).
fn into_vault(app: &Faraday, k: usize) -> Option<Pick> {
    app.vault_way(Screen::Files).is_none().then(|| {
        (
            EN.add_to_vault_button.to_string(),
            Action::Vault(crate::vault::VaultAction::AddFile(k)),
        )
    })
}

/// A PSBT: Collect where it belongs to the spend under way, whoever
/// signed it (as Files' button offers it); Sign while a loaded wallet can
/// sign it and a key here is left to; else only the checks.
fn psbt(app: &Faraday, spends_from: SpendsFrom, k: usize, item: &Item) -> Offer {
    let checks = [check_signer(k), decode(k)];
    let same_spend = app.wallets.spend().is_some_and(|s| {
        item.psbt()
            .is_some_and(|p| p.txid == s.spend.txid && item.name != *s.spend.source)
    });
    let first = if same_spend {
        Some((
            EN.inbox_actions_add_signatures.to_string(),
            Action::Collect(k),
        ))
    } else {
        let signable = !app.checked_not_signed(k)
            && item.psbt().is_some_and(|seen| spends_from(seen).is_some());
        signable.then(|| (EN.inbox_actions_sign.to_string(), Action::StartSpend(k)))
    };
    Offer::with_button(first.into_iter().chain(checks).collect())
}

/// A wallet file, in `docs/WORKSPACE.md`'s order. Load and Add to vault
/// only while its wallet is not loaded (the wallet decides, not the
/// file's name; `loaded` is the session's wallet it describes, from the
/// Inbox's kept answer, and without that answer Load is offered); the
/// setup check; Back up (the loaded wallet's own flow) and Spend from
/// backup (the flow's own way in), for a file the flow takes as a wallet
/// file, a secret one whose wallet was read included.
fn wallet(app: &Faraday, k: usize, item: &Item, loaded: Option<usize>) -> Offer {
    let mut list = Vec::new();
    if loaded.is_none() {
        list.push((
            EN.inbox_actions_load_wallet.to_string(),
            Action::LoadWallet(k),
        ));
        list.extend(into_vault(app, k));
    }
    list.extend(crate::multisig_check::menu_line(item.wallet(), false, k));
    if crate::create::wallet_flow_kind(item) == FileKind::Wallet {
        if let Some(i) = loaded {
            list.push((EN.backups_back_up.to_string(), Action::Backup(i)));
        }
        list.push((
            EN.home_ways_spend_short.to_string(),
            Action::SpendBackup(crate::spend_backup::SpendBackupAction::FromFile(k)),
        ));
    }
    if loaded.is_none() {
        Offer::with_button(list)
    } else {
        Offer::without_button(list)
    }
}

/// A vault file that is locked: Unlock, the press its own row makes.
fn vault(app: &Faraday, item: &Item) -> Offer {
    let files = app.vault_files();
    match crate::vault::vault_at(&files, item) {
        // Never while a stick is in: the row waits until it is pulled.
        Some(at) if files[at].open.is_none() && app.stick.is_empty() => Offer::with_button(vec![(
            EN.inbox_actions_unlock.to_string(),
            Action::Vault(crate::vault::VaultAction::Open(at)),
        )]),
        _ => Offer::none(),
    }
}

/// A key file or key line: Use key.
fn use_key(k: usize, kind: FileKind) -> Pick {
    match kind {
        FileKind::KeyLine => (EN.inbox_actions_load.to_string(), Action::LoadKey(k)),
        _ => (EN.read_go_use_key.to_string(), Action::CreateWallet),
    }
}

fn add_key(k: usize) -> Pick {
    (EN.inbox_actions_load.to_string(), Action::LoadKey(k))
}

fn restore_shares(app: &Faraday) -> Pick {
    let n = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Share)
        .count();
    (
        crate::strings::fill(EN.inbox_actions_restore, &[&n.to_string()]),
        Action::RestoreShares,
    )
}

/// Text: Sign this message (a message wallet loaded), Check an address,
/// Add to vault.
fn text(app: &Faraday, k: usize, item: &Item) -> Offer {
    let mut list = Vec::new();
    if !app.session.message_wallets().is_empty() && std::str::from_utf8(&item.bytes).is_ok() {
        list.push((
            EN.files_sign_this_message.to_string(),
            Action::File(crate::file_menu::FilePress::SignMessage(k)),
        ));
    }
    list.push((
        EN.files_check_an_address.to_string(),
        Action::File(crate::file_menu::FilePress::CheckAddress(k)),
    ));
    // The way a text file goes into a vault (a note), while one is open.
    list.extend(into_vault(app, k));
    Offer::without_button(list)
}

/// A GPG public key that can be restored: Restore.
fn gpg(k: usize, item: &Item) -> Vec<Pick> {
    use crate::backup_flow::steps::{FlowAction, RestorePress};
    if !crate::backup_flow::gpg_restore::restorable(item) {
        return Vec::new();
    }
    vec![(
        EN.gpg_restore_files_action.to_string(),
        Action::Flow(FlowAction::Restore(RestorePress::From(k))),
    )]
}
