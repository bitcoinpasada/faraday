//! The sheet's messages (`main`'s `secret_act`, `secret_to_vault`, `secret_hold` and
//! `secret_cancel`).

use faraday_kernel::session::{PutOutbox, RemoveFile};
use faraday_kernel::vaults::Put;
use faraday_kernel::{Ctx, Effects, Message, Never, Public};
use faraday_port::bytes::Bytes;
use faraday_text::Text;
use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};

use crate::sheet::{Keep, Plain, SecretOut, Then};
use crate::state::{Answer, Finished, Pending, State};

/// What a press on the sheet sends; its owner's `Press` holds it.
#[derive(Public, PartialEq, Eq)]
pub enum Press {
    /// Into the open vault.
    Vault,
    /// Hold to save without encryption: the sheet that asks for the hold.
    Unprotected,
    /// The hold was held to its end.
    Hold,
    /// A file form picked.
    Form(usize),
    /// A field of the key's record unticked, or ticked again.
    Field(u8),
    /// Nothing chosen.
    Cancel,
}

#[derive(Message)]
pub enum Msg {
    Press(Press),
    /// A secret waits for the person to say where. Withheld: it holds the secret.
    #[withheld]
    Offer(SecretOut),
    /// A plain save waits for its hold; `Plain` holds no secret.
    AskPlain(Plain),
    /// `Unprotected`, with where it goes named by the owner (the stick's noun).
    UnprotectedOn(Text),
    #[withheld]
    Put(Result<(), faraday_keys::Refusal>),
    #[withheld]
    Removed(Result<(), faraday_keys::Refusal>),
    #[withheld]
    Outbox(Result<faraday_kernel::FileId, faraday_keys::Refusal>),
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

pub fn update(state: &mut State, msg: Msg, cx: &Ctx<'_>) -> Effects<Msg, Never> {
    match msg {
        Msg::Offer(out) => {
            state.out = Some(out);
            state.plain = None;
            state.answer = None;
            state.finished = None;
            Effects::none()
        }
        Msg::AskPlain(plain) => {
            state.plain = Some(plain);
            Effects::none()
        }
        Msg::UnprotectedOn(on) => {
            if let Some(out) = state.out.as_ref() {
                let names: Vec<&str> = out.names().iter().map(|n| n.as_str()).collect();
                state.plain = Some(Plain {
                    name: Text::joined(&names, ", "),
                    on: on.clone(),
                    keep: on,
                    then: Then::Secret,
                });
            }
            Effects::none()
        }
        Msg::Press(Press::Form(i)) => {
            state.pick_form(i);
            Effects::none()
        }
        Msg::Press(Press::Field(n)) => {
            state.toggle_left(n);
            Effects::none()
        }
        Msg::Press(Press::Vault) => to_vault(state, cx),
        // The owner words the ask for the hold with `UnprotectedOn`.
        Msg::Press(Press::Unprotected) => Effects::none(),
        Msg::Press(Press::Hold) => hold(state, cx),
        Msg::Press(Press::Cancel) => {
            let then = state.plain.take().map(|p| p.then);
            drop(state.out.take());
            state.finished = Some(Finished::Cancelled(
                then.filter(|t| !matches!(t, Then::Secret | Then::Lock)),
            ));
            Effects::none()
        }
        Msg::Put(done) => {
            match done {
                Ok(()) => {
                    if let Some(out) = state.out.take() {
                        let what = match &out.keep {
                            Keep::Key { key, .. } => cx
                                .view
                                .session
                                .key(*key)
                                .map_or_else(|| out.name.clone(), |k| k.name.clone()),
                            Keep::Note | Keep::None => out.name.clone(),
                        };
                        state.finished = Some(Finished::SavedToVault { what });
                    }
                }
                // The vault refused it: the sheet stays as it was, with the secret on it.
                Err(_) => state.answer = Some(Answer::VaultRefused),
            }
            Effects::none()
        }
        Msg::Outbox(done) => outbox_answered(state, done.ok()),
        Msg::Removed(done) => removed_answered(state, done.is_ok()),
    }
}

/// The secret into the open vault.
fn to_vault(state: &mut State, cx: &Ctx<'_>) -> Effects<Msg, Never> {
    if cx.view.vaults.current().is_none() {
        state.answer = Some(Answer::NoVaultOpen);
        return Effects::none();
    }
    state.plain = None;
    state.answer = None;
    let Some(out) = state.out.as_ref() else {
        return Effects::none();
    };
    let record = match &out.keep {
        Keep::Note => Some(Record::new(kind::NOTE).with(field::NOTE, out.bytes.expose())),
        Keep::Key { key, fp, left } => key_record(cx, *key, *fp, left),
        Keep::None => None,
    };
    let Some(record) = record else {
        // A key whose fingerprint changed, or whose record cannot be made, or a secret no vault
        // takes: the sheet closes with no message.
        drop(state.out.take());
        state.finished = Some(Finished::Closed);
        return Effects::none();
    };
    Effects::none().change(
        Put {
            vault: None,
            record,
        },
        false,
        |_, done| Msg::Put(done),
    )
}

/// Session key `key` as the vault's key record, when it is still the key whose fingerprint was
/// offered. The record says its backup is not done while the key is held, and that it was written
/// on an online computer when the key was made online or the app runs online (`main`'s
/// `written_online`).
#[expect(
    clippy::disallowed_methods,
    reason = "a key's vault record leaves its guard here, for the open vault: the reveal named VaultEntry (docs/KEYS.md)"
)]
fn key_record(
    cx: &Ctx<'_>,
    key: faraday_kernel::KeyId,
    fp: faraday_bip::keys::Fingerprint,
    left: &[u8],
) -> Option<Record> {
    let view = cx.view.session.key(key).filter(|k| k.fingerprint == fp)?;
    let keys = cx.view.session.keys();
    let at = keys.position(fp)?;
    let flags = key_flags(view.held, view.made_online, cx.view.mode);
    keys.key_record(
        cx.compute,
        at,
        flags,
        false,
        left,
        faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Words),
    )
    .map(faraday_keys::Held::release)
}

/// The record's marks: its backup is pending while the key is held, and it was written on an online
/// computer when the key was made online or the app runs online (`main`'s `written_online`).
pub(crate) fn key_flags(held: bool, made_online: bool, mode: faraday_kernel::Mode) -> Option<u8> {
    let pending = held.then_some(records::BACKUP_PENDING);
    match (pending, made_online || mode.online) {
        (p, true) => Some(p.unwrap_or(0) | records::MADE_ONLINE),
        (p, false) => p,
    }
}

/// The hold, held to its end: what the sheet asked for is done.
fn hold(state: &mut State, cx: &Ctx<'_>) -> Effects<Msg, Never> {
    let Some(plain) = state.plain.take() else {
        return Effects::none();
    };
    match plain.then {
        Then::Secret => unprotected_now(state, cx),
        other => {
            state.finished = Some(Finished::Held(other));
            Effects::none()
        }
    }
}

/// The secret to the Outbox unprotected, past its hold. The secret stays on the sheet until every
/// write is answered `Ok`; a refusal keeps it there and says so. A file with a name the Outbox
/// holds is replaced once the new one is in: a share saved before is not there twice.
fn unprotected_now(state: &mut State, cx: &Ctx<'_>) -> Effects<Msg, Never> {
    // From the Inbox to another stick: ticked on the visit, written from the Inbox when the
    // person presses Write.
    if state.out.as_ref().is_some_and(|o| o.to_visit) {
        if let Some(out) = state.out.take() {
            state.finished = Some(Finished::ToVisit { name: out.name });
        }
        return Effects::none();
    }
    let Some(out) = state.out.as_ref() else {
        return Effects::none();
    };
    let mut fx = Effects::none();
    let mut names = Vec::new();
    let mut old = Vec::new();
    let files =
        core::iter::once((&out.name, &out.bytes)).chain(out.also.iter().map(|(n, b)| (n, b)));
    for (name, bytes) in files {
        old.extend(
            cx.view
                .session
                .outbox()
                .iter()
                .filter(|f| f.name == *name)
                .map(|f| f.id),
        );
        fx = fx.change(
            PutOutbox {
                name: name.clone(),
                bytes: Bytes::new(bytes.expose().clone()),
            },
            false,
            |_, done| Msg::Outbox(done),
        );
        names.push(name.clone());
    }
    state.answer = None;
    state.pending = Some(Pending {
        waiting: names.len(),
        names,
        refused: false,
        old,
        written: Vec::new(),
        removing: None,
        old_stays: false,
    });
    fx
}

/// The removals as changes.
fn removing(ids: Vec<faraday_kernel::FileId>) -> Effects<Msg, Never> {
    let mut fx = Effects::none();
    for id in ids {
        fx = fx.change(RemoveFile(id), false, |_, done| Msg::Removed(done));
    }
    fx
}

/// One Outbox write answered, with the file it made.
fn outbox_answered(state: &mut State, made: Option<faraday_kernel::FileId>) -> Effects<Msg, Never> {
    removing(outbox_step(state, made))
}

/// The state's side of [`outbox_answered`]: the files to remove now. When the last write is in and
/// every one was accepted, those are the earlier files of the same names, and the sheet finishes
/// once those answers are in. When any was refused the save is all or nothing: those are the
/// files this save wrote, the secret stays on the sheet and the refusal is said.
pub(crate) fn outbox_step(
    state: &mut State,
    made: Option<faraday_kernel::FileId>,
) -> Vec<faraday_kernel::FileId> {
    let Some(pending) = state.pending.as_mut() else {
        return Vec::new();
    };
    pending.waiting = pending.waiting.saturating_sub(1);
    match made {
        Some(id) => pending.written.push(id),
        None => {
            pending.refused = true;
            state.answer = Some(Answer::OutboxRefused);
        }
    }
    if pending.waiting > 0 {
        return Vec::new();
    }
    if pending.refused {
        let written = core::mem::take(&mut pending.written);
        state.pending = None;
        return written;
    }
    let old = core::mem::take(&mut pending.old);
    if old.is_empty() {
        finish_unprotected(state);
    } else {
        pending.removing = Some(old.len());
    }
    old
}

/// One removal answered. Only the removals of earlier files, after a save, count; those of a
/// refused save's own files are not waited for.
fn removed_answered(state: &mut State, ok: bool) -> Effects<Msg, Never> {
    removed_step(state, ok);
    Effects::none()
}

/// The state's side of [`removed_answered`].
pub(crate) fn removed_step(state: &mut State, ok: bool) {
    let Some(pending) = state.pending.as_mut() else {
        return;
    };
    let Some(left) = pending.removing.as_mut() else {
        return;
    };
    *left = left.saturating_sub(1);
    pending.old_stays |= !ok;
    if *left == 0 {
        finish_unprotected(state);
    }
}

/// Every write is in and the earlier files are dealt with: the secret leaves the sheet.
fn finish_unprotected(state: &mut State) {
    if let Some(pending) = state.pending.take() {
        drop(state.out.take());
        state.finished = Some(Finished::Unprotected {
            names: pending.names,
            old_stays: pending.old_stays,
        });
    }
}
