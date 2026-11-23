//! What each kind of held item offers (`docs/WORKSPACE.md` §2, "Row
//! buttons and what a row opens"): its row button, at most one, and its
//! "What you can do" list, likeliest first, both as existing [`Action`]s.
//! One list per item, read by Files' row menu now and by the session
//! pane's details later. Built where a menu opens, never per draw.

use crate::strings::EN;
use crate::{Action, Faraday, FileKind, Item, Screen};

/// One thing to do: its label and its action.
pub(crate) type Pick = (String, Action);

/// An item's list and whether its first entry is also its row button.
pub(crate) struct Offer {
    /// "What you can do", likeliest first.
    pub list: Vec<Pick>,
    /// The first entry of `list` is the row's button.
    #[expect(dead_code, reason = "the session pane's rows read it, still to build")]
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
    #[expect(dead_code, reason = "the session pane's rows read it, still to build")]
    pub(crate) fn button(&self) -> Option<&Pick> {
        self.button.then(|| self.list.first()).flatten()
    }
}

/// Costs a lookup of the loaded wallets by a PSBT's first input. The
/// offer of Inbox file `k`, by its kind.
pub(crate) fn offer(app: &mut Faraday, k: usize) -> Offer {
    let c = app.compute();
    let app = &*app;
    let Some(item) = app.inbox.get(k) else {
        return Offer::none();
    };
    match item.kind {
        FileKind::Psbt => psbt(app, &c, k, item),
        FileKind::Transaction => Offer::with_button(vec![decode(k), check_signer(k)]),
        FileKind::Wallet | FileKind::SecretWallet => wallet(app, k, item),
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
fn psbt(app: &Faraday, c: &faraday_crypto::work::Compute, k: usize, item: &Item) -> Offer {
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
            && item
                .psbt()
                .is_some_and(|seen| app.session.spends_from(c, seen).is_some());
        signable.then(|| (EN.inbox_actions_sign.to_string(), Action::StartSpend(k)))
    };
    Offer::with_button(first.into_iter().chain(checks).collect())
}

/// A wallet file: Load, Add to vault and the setup check.
fn wallet(app: &Faraday, k: usize, item: &Item) -> Offer {
    let mut list = vec![(
        EN.inbox_actions_load_wallet.to_string(),
        Action::LoadWallet(k),
    )];
    list.extend(into_vault(app, k));
    list.extend(crate::multisig_check::menu_line(item.wallet(), false, k));
    Offer::with_button(list)
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

/// Text: Sign this message (a message wallet loaded), Check an address.
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
