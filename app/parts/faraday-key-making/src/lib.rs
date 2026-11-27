//! An application part: making a key: its entries, the words made, the quiz (03 §9.3).
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
    #[lock(kept)]
    word_grid: faraday_word_grid::State,
    #[lock(kept)]
    pass_field: faraday_pass_field::State,
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
