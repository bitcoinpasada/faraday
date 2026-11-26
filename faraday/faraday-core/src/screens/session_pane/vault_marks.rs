//! Vaults' pane marks (`docs/WORKSPACE.md` §2): while exactly one vault is
//! open, each key and wallet the session holds says whether that vault holds
//! it, and the ones it does not hold carry Save, one press. The press is the
//! vault's own Save (`VaultAction::SaveKey`, `SaveWallet`), so every check
//! that Save makes stays; nothing is written from here.

use super::SessionRow;
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::vault::VaultAction as V;

/// The open vault the marks are about: the one open vault, when it can take
/// a Save. `None` for no vault, two or more (no mark). The flag is whether
/// it can be written.
fn the_vault(app: &View) -> Option<bool> {
    let open = &app.vaults.open;
    (open.len() == 1 && app.vaults.current_id().is_some()).then(|| open[0].read_only().is_none())
}

/// Marks the key rows and the wallet rows of Vaults' pane. A row not held
/// carries Save where the vault can be written, its own press kept as the
/// row's `open`.
pub(super) fn mark(app: &View, keys: &mut [SessionRow], wallets: &mut [SessionRow]) {
    let Some(writable) = the_vault(app) else {
        return;
    };
    let cur = app.vaults.current;
    for (k, (key, row)) in app.session.keys.all().zip(keys.iter_mut()).enumerate() {
        // A key is in a vault only where the vault holds it with its
        // passphrase, or with none where it has none; its words alone are
        // a mark of their own, and Save is for a key the vault holds
        // nowhere.
        let words = app.vault_holds_seed_kept(cur, k, false);
        let held = app.vault_holds_seed_kept(cur, k, true);
        let savable = key.has_words() || app.session.keys.holds_seed_bytes(k);
        let mark = match (held, words) {
            (true, _) => EN.session_mark_in_a_vault,
            (false, true) => EN.session_mark_words_in_a_vault,
            _ => EN.session_mark_not_saved,
        };
        put(
            row,
            mark,
            !words && writable && savable,
            Action::Vault(V::SaveKey(k)),
        );
    }
    for row in wallets.iter_mut() {
        let Action::OpenWallet(i) = row.action else {
            continue;
        };
        let Some(w) = app.session.wallets.get(i) else {
            continue;
        };
        let held = app.vault_has_wallet(cur, w);
        let mark = if held {
            EN.session_mark_in_a_vault
        } else {
            EN.session_mark_not_saved
        };
        put(
            row,
            mark,
            !held && writable,
            Action::Vault(V::SaveWallet(i)),
        );
    }
}

/// `row` marked `mark`; with `save`, the Save button, the row's old press
/// moved to `open`.
fn put(row: &mut SessionRow, mark: &str, save: bool, press: Action) {
    row.trailing = mark.to_string();
    if save {
        row.open = Some(row.action);
        row.action = press;
        row.button = Some((EN.vault_save, true));
    }
}
