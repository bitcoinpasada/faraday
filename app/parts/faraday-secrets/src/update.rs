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
use crate::state::{Answer, Finished, State};

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
        Msg::Outbox(_) | Msg::Removed(_) => Effects::none(),
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
/// offered. The record says the key was made here and its backup not done, and that it was
/// written on an online computer, as `main` does.
fn key_record(
    cx: &Ctx<'_>,
    key: faraday_kernel::KeyId,
    fp: faraday_bip::keys::Fingerprint,
    left: &[u8],
) -> Option<Record> {
    let view = cx.view.session.key(key).filter(|k| k.fingerprint == fp)?;
    let keys = cx.view.session.keys();
    let at = keys.position(fp)?;
    let pending = view.held.then_some(records::BACKUP_PENDING);
    let flags = match (pending, view.made_online) {
        (p, true) => Some(p.unwrap_or(0) | records::MADE_ONLINE),
        (p, false) => p,
    };
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

/// The secret to the Outbox unprotected, past its hold. A file with a name the Outbox holds
/// replaces it: a share saved before is not there twice.
fn unprotected_now(state: &mut State, cx: &Ctx<'_>) -> Effects<Msg, Never> {
    let Some(out) = state.out.take() else {
        return Effects::none();
    };
    // From the Inbox to another stick: ticked on the visit, written from the Inbox when the
    // person presses Write.
    if out.to_visit {
        state.finished = Some(Finished::ToVisit { name: out.name });
        return Effects::none();
    }
    let mut fx = Effects::none();
    let mut names = Vec::new();
    let files = core::iter::once((&out.name, &out.bytes)).chain(out.also.iter().map(|(n, b)| (n, b)));
    for (name, bytes) in files {
        for held in cx.view.session.outbox().iter().filter(|f| f.name == *name) {
            fx = fx.change(RemoveFile(held.id), false, |_, done| Msg::Removed(done));
        }
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
    state.finished = Some(Finished::Unprotected { names });
    fx
}
