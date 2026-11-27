//! An application part: the Secret out sheet, the text checks (TextCheck), file classification.
//! Section 09 §4.3: `State` (fields private), `Msg`, `Press`, `update`, a `Read<'s>` view for its
//! owner's `present`, and `AtLock` for `State`.

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use core::marker::PhantomData;

use faraday_kernel::{AtLock, Ctx, Effects, Message, Never};

#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    skeleton: (),
}

#[derive(Message)]
pub enum Msg {}

/// What a press on this part sends; its owner's `Press` holds it.
#[derive(Clone, PartialEq, Debug)]
pub enum Press {}

pub fn update(_state: &mut State, _msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Never> {
    todo!()
}

/// The read view the owner's `present` maps to a kit model.
pub struct Read<'s>(PhantomData<&'s State>);

/// What a text is checked as (the Secret out sheet's text checks).
// Spec: 09 §4.3 names `TextCheck` without a shape.
pub struct TextCheck(());
