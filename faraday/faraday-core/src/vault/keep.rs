//! **Keep its codex32 set** and **Keep its BIP-85 origin** where a session
//! seed is saved into a vault (`docs/VAULT.md` §7). A choice holds for the
//! save it was made for and lives where that save does: on the secret
//! sheet's secret (`Keep::Key`), on the Keep sheet (a Save on Vaults), or
//! in the backup or wallet flow whose save asked (`Left`). Another save
//! starts ticked and asks for itself. The seed's own set and origin are
//! never changed by a choice.

use faraday_vault::records::field;

use crate::strings::{EN, fill};
use crate::vault::VaultAction as V;
use crate::{Faraday, Sheet};

/// What happens when the sheet's Save is pressed: the press that asked,
/// done again with the choices made.
#[derive(Clone, Copy, Debug)]
pub enum Then {
    /// Save session key `key` into the open vault, with its BIP-39
    /// passphrase when `with`.
    Key {
        /// The session key.
        key: usize,
        /// Saved with its passphrase.
        with: bool,
    },
    /// A backup's Save into vault `v` of its plan (`backup_vault_save`).
    Vault(usize),
    /// A wallet flow's copy into its added vaults (`flow_wallet_copy`).
    Wallet(usize),
}

impl Then {
    /// The flow whose choices the press uses; none for a save outside a
    /// flow.
    fn scope(self) -> Option<Scope> {
        match self {
            Self::Key { .. } => None,
            Self::Vault(_) => Some(Scope::Backup),
            Self::Wallet(i) => Some(Scope::Flow(i)),
        }
    }
}

/// Whose choices a flow's save reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The wallet backup under way.
    Backup,
    /// Wallet flow `i`.
    Flow(usize),
}

/// What the person unticked in one flow, kept with it and dropped with it.
#[derive(Clone, Debug, Default)]
pub struct Left {
    /// The seeds' fields left out, by fingerprint and the field's number.
    left: Vec<([u8; 4], u8)>,
    /// The seeds the flow's sheet asked about, so it asks once.
    asked: Vec<[u8; 4]>,
}

/// A seed the sheet asks about: its fingerprint, its label and the
/// fields of it that an entry may leave out.
#[derive(Clone, Debug)]
pub struct Ask {
    /// The seed's fingerprint.
    pub fp: [u8; 4],
    /// Its label.
    pub label: String,
    /// The fields it has: codex32 set and BIP-85 origin, by number.
    pub fields: Vec<u8>,
}

/// The save the sheet waits to make: the seeds it asks about, what the
/// person has unticked so far, and what Save goes on with.
#[derive(Clone, Debug)]
pub struct KeepSave {
    /// The seeds, each once.
    pub asks: Vec<Ask>,
    /// The fields unticked, by the seed's fingerprint and the field's
    /// number: they last as long as the sheet.
    pub left: Vec<([u8; 4], u8)>,
    /// What Save does.
    pub then: Then,
    /// The names of the vaults the seeds go into, as they were named when
    /// the sheet opened.
    into: Vec<String>,
}

/// The text of a fingerprint, four bytes, as bytes.
pub(crate) fn fp_bytes(text: &str) -> Option<[u8; 4]> {
    let b = text.as_bytes();
    if b.len() != 8 {
        return None;
    }
    let mut out = [0u8; 4];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(text.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

/// The fields among `fields` as the sheet lists them, each ticked unless
/// it is in `out`.
pub(crate) fn ticks_of(fields: &[u8], out: &[u8]) -> Vec<(u8, bool)> {
    [field::KEY_CODEX32, field::KEY_CHILD]
        .into_iter()
        .filter(|n| fields.contains(n))
        .map(|n| (n, !out.contains(&n)))
        .collect()
}

/// The fields session key `k` has that a vault entry may leave out.
pub(crate) fn fields_of(app: &Faraday, k: usize) -> Vec<u8> {
    let Some(info) = app.session.keys.info(k) else {
        return Vec::new();
    };
    [
        (field::KEY_CODEX32, info.codex32.is_some()),
        (field::KEY_CHILD, info.child_of.is_some()),
    ]
    .into_iter()
    .filter(|h| h.1)
    .map(|h| h.0)
    .collect()
}

fn left_of(list: &[([u8; 4], u8)], fp: [u8; 4]) -> Vec<u8> {
    list.iter().filter(|u| u.0 == fp).map(|u| u.1).collect()
}

fn flow_left(app: &Faraday, scope: Scope) -> Option<&Left> {
    match scope {
        Scope::Backup => app.backup.keeps(),
        Scope::Flow(i) => app.flows.get(i).map(|f| &f.keeps),
    }
}

/// The fields the person left out of the seed with fingerprint `fp` in
/// the flow `scope`.
pub(crate) fn left_out_fp(app: &Faraday, scope: Scope, fp: [u8; 4]) -> Vec<u8> {
    flow_left(app, scope).map_or_else(Vec::new, |l| left_of(&l.left, fp))
}

/// The fields the person left out of session key `k`'s entry in the flow
/// `scope`.
pub(crate) fn left_out(app: &Faraday, scope: Scope, k: usize) -> Vec<u8> {
    match app.session.keys.info(k) {
        Some(i) => left_out_fp(app, scope, i.fingerprint.0),
        None => Vec::new(),
    }
}

/// The seeds of session keys `keys` that have a set or an origin, as
/// the sheet lists them.
pub(crate) fn asks_for(app: &Faraday, keys: &[usize]) -> Vec<Ask> {
    keys.iter()
        .filter_map(|&k| {
            let info = app.session.keys.info(k)?;
            let fields = fields_of(app, k);
            (!fields.is_empty()).then(|| Ask {
                fp: info.fingerprint.0,
                label: info.label.to_string(),
                fields,
            })
        })
        .collect()
}

/// Choice `number` of seed `fp` flipped, where the sheet that is up keeps
/// it: on the Keep sheet, else on the secret sheet's secret.
fn flip(app: &mut Faraday, fp: [u8; 4], number: u8) {
    if let Some(s) = app.vaults.state.keep_save.as_mut() {
        let shown = s
            .asks
            .iter()
            .any(|a| a.fp == fp && a.fields.contains(&number));
        if !shown {
            return;
        }
        if s.left.contains(&(fp, number)) {
            s.left.retain(|u| *u != (fp, number));
        } else {
            s.left.push((fp, number));
        }
        return;
    }
    if app.sheet != Some(Sheet::SecretOut) {
        return;
    }
    let Some(crate::secrets::Keep::Key { key, .. }) = app.secrets.out().map(|o| &o.keep) else {
        return;
    };
    let key = *key;
    let same = app
        .session
        .keys
        .info(key)
        .is_some_and(|i| i.fingerprint.0 == fp);
    if !same || !fields_of(app, key).contains(&number) {
        return;
    }
    if let Some(crate::secrets::Keep::Key { left, .. }) = app.secrets.out_mut().map(|o| &mut o.keep)
    {
        if left.contains(&number) {
            left.retain(|n| *n != number);
        } else {
            left.push(number);
        }
    }
}

/// The sheet's save, if any.
pub(crate) fn pending(app: &Faraday) -> Option<&KeepSave> {
    app.vaults.state.keep_save.as_ref()
}

/// Save of session key `k` pressed: a seed with a set or an origin
/// asks first, another is saved.
pub(crate) fn asks(app: &mut Faraday, k: usize, with: bool) {
    let then = Then::Key { key: k, with };
    if !gate(app, asks_for(app, &[k]), then, &[app.vaults.current]) {
        app.vault_save_key_with(k, with, &[]);
    }
}

/// A backup's press that would save `asks` into a vault: the sheet
/// opens for the seeds not asked about in this flow, and the press waits
/// for Save. False when nothing is to be asked and the press goes on.
pub(crate) fn gate(app: &mut Faraday, asks: Vec<Ask>, then: Then, into: &[usize]) -> bool {
    let mut list: Vec<Ask> = Vec::new();
    for a in asks {
        let asked = then
            .scope()
            .and_then(|sc| flow_left(app, sc))
            .is_some_and(|l| l.asked.contains(&a.fp));
        if asked {
            continue;
        }
        match list.iter_mut().find(|l| l.fp == a.fp) {
            Some(l) => {
                for n in a.fields {
                    if !l.fields.contains(&n) {
                        l.fields.push(n);
                    }
                }
            }
            None => list.push(a),
        }
    }
    if list.is_empty() {
        return false;
    }
    let into = into
        .iter()
        .filter_map(|&o| app.vaults.open.get(o))
        .map(|o| o.label(app.vaults.store()))
        .collect();
    app.vaults.state.keep_save = Some(KeepSave {
        asks: list,
        left: Vec::new(),
        then,
        into,
    });
    app.sheet = Some(Sheet::SaveKeeps);
    true
}

/// The sheet's Save (`go`) or Cancel. Save goes on with what the press
/// was doing; Cancel saves nothing and leaves a flow where it was.
pub(crate) fn act(app: &mut Faraday, a: V) {
    match a {
        V::KeepTick(fp, number) => flip(app, fp, number),
        V::KeepSave | V::KeepCancel => {
            let Some(s) = app.vaults.state.keep_save.take() else {
                return;
            };
            if app.sheet == Some(Sheet::SaveKeeps) {
                app.sheet = None;
            }
            if matches!(a, V::KeepCancel) {
                return;
            }
            if let Some(l) = s.then.scope().and_then(|sc| flow_left_mut(app, sc)) {
                l.left.extend(s.left.iter().copied());
                l.asked.extend(s.asks.iter().map(|k| k.fp));
            }
            match s.then {
                Then::Key { key, with } => {
                    let left = app
                        .session
                        .keys
                        .info(key)
                        .map(|i| left_of(&s.left, i.fingerprint.0))
                        .unwrap_or_default();
                    app.vault_save_key_with(key, with, &left);
                }
                Then::Vault(v) => app.backup_vault_save(v),
                Then::Wallet(i) => app.flow_wallet_copy(i),
            }
        }
        _ => {}
    }
}

fn flow_left_mut(app: &mut Faraday, scope: Scope) -> Option<&mut Left> {
    match scope {
        Scope::Backup => app.backup.keeps_mut(),
        Scope::Flow(i) => app.flows.keeps_mut(i),
    }
}

/// The Keep sheet goes with the first press after it stops being up (a
/// sheet over it, a lock): that is Cancel. The secret sheet's choices are
/// on its secret and the flows' in the flows, so nothing else is kept
/// here. True when it dropped something.
pub(crate) fn settle(app: &mut Faraday) -> bool {
    if app.vaults.state.keep_save.is_some() && app.sheet != Some(Sheet::SaveKeeps) {
        app.vaults.state.keep_save = None;
        return true;
    }
    false
}

/// The title of the sheet: the vaults the seeds go into, as the sheet was
/// opened for them; **Save** where it names none.
pub(crate) fn title(app: &Faraday) -> String {
    match pending(app).map(|s| s.into.join(", ")) {
        Some(names) if !names.is_empty() => fill(EN.secret_out_save_into, &[&names]),
        _ => EN.vault_save.to_string(),
    }
}
