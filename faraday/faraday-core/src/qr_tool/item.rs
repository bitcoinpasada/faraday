//! The QR vault tool opened from a vault item's **Save as a QR vault**
//! (`docs/VAULT.md` §13): the tool names the vault and the item's place
//! and nothing of the item, which is read once when the seal's job is
//! made and stays in the vault, unchanged.

use super::{Pick, QrToolState, go_to_passphrase, st};
use crate::Faraday;
use crate::strings::EN;
use faraday_crypto::work::Compute;
use faraday_keys::vault::VaultId;
use faraday_keys::{QR_RANDOM_LEN, QrError, QrSealJob};
use faraday_text::Text;
use faraday_vault::records::{field, kind};
use zeroize::Zeroizing;

/// An item of an open vault, as the tool was opened on it. Set when the
/// tool opens and kept as it is; the version is the vault's then.
pub(crate) struct Source {
    id: VaultId,
    at: usize,
    version: u64,
    kind: u8,
    /// A seed that keeps a passphrase: it is not in the code.
    pass_kept: bool,
    vault: Text,
}

impl Source {
    /// The Contents line: what the code holds and where it is from.
    pub(crate) fn line(&self) -> Text {
        let what = match self.kind {
            kind::KEY => EN.qr_tool_item_seed,
            kind::CODEX32_SHARE => EN.qr_tool_item_share,
            kind::PASSPHRASE => EN.qr_tool_item_passphrase,
            _ => EN.qr_tool_item_note,
        };
        Text::fill(what, &[&self.vault])
    }

    /// Whether the item is a seed that keeps a passphrase.
    pub(crate) fn pass_kept(&self) -> bool {
        self.pass_kept
    }
}

/// Opens the tool at its Passphrase step on item `at` of open vault `id`,
/// which fits (checked by the press).
pub(crate) fn open(app: &mut Faraday, id: VaultId, at: usize) {
    let store = app.vaults.store();
    let Some(r) = store.records(id).get(at) else {
        return;
    };
    let source = Source {
        id,
        at,
        version: store.version(id),
        kind: r.kind(),
        pass_kept: r.kind() == kind::KEY && r.has(field::KEY_PASSPHRASE),
        vault: app
            .vaults
            .open
            .iter()
            .find(|o| o.id == id)
            .map_or_else(Text::new, |o| Text::of(&o.label(store))),
    };
    super::open(app);
    if let Some(s) = st(app) {
        s.pick = Some(Pick::Item);
        s.item = Some(source);
        go_to_passphrase(s);
    }
}

/// Whether the tool is on an item whose vault is no longer open as it
/// was: locked, closed or changed.
pub(super) fn gone(app: &Faraday) -> bool {
    let Some(src) = super::state(app).and_then(QrToolState::item) else {
        return false;
    };
    !app.vaults.open.iter().any(|o| o.id == src.id)
        || app.vaults.store().version(src.id) != src.version
}

/// Costs what `qr_fit` costs. The seal of the item as a job; the item is
/// read here, once. [`QrError::NoItem`] when the vault is locked, closed
/// or changed since the press.
pub(super) fn job(
    app: &Faraday,
    c: &Compute,
    passphrase: &str,
    exponent: u8,
    random: Zeroizing<[u8; QR_RANDOM_LEN]>,
) -> Result<QrSealJob, QrError> {
    let src = super::state(app)
        .and_then(QrToolState::item)
        .ok_or(QrError::NoItem)?;
    if gone(app) {
        return Err(QrError::NoItem);
    }
    app.vaults
        .store()
        .qr_seal_item_job(c, src.id, src.at, passphrase, exponent, random)
}
