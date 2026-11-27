//! An application part: the ways in to a key (Scan, From Files, Type) and the key-form pick.
//! Section 09 §4.3: `State` (fields private), `Msg`, `Press`, `update`, a `Read<'s>` view for its
//! owner's `present`, and `AtLock` for `State`.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use core::marker::PhantomData;

use faraday_kernel::{AtLock, Ctx, Effects, Message, Never};

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    text_field: faraday_text_field::State,
    #[lock(kept)]
    word_grid: faraday_word_grid::State,
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
