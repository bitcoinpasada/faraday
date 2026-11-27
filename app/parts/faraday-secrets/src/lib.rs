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

/// What the text check found in a text (Transfer's Send text, My seed's Compare, a wallet's name).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextCheck {
    /// Nothing is refused: empty, or fit to show.
    Fine,
    /// Over the most text sent.
    TooLong,
    /// The whole text is this secret.
    Secret(SecretKind),
    /// This secret is in the text.
    Inside(SecretKind),
    /// Part of a code that can hold a secret, and the rest is missing.
    Partial,
    /// Bare hex of the length of BIP-39 entropy or a BIP-32 seed.
    SeedHex,
}

/// The secret a text is or holds: `main`'s secret scan kinds, and a wallet file holding a secret.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SecretKind {
    SeedWords,
    SeedQr,
    CompactSeedQr,
    Codex32Secret,
    Codex32Share,
    Slip39Share,
    BadChecksum,
    Aezeed,
    Wif,
    Xprv,
    UrSeed,
    Nostr,
    SpendKey,
    Paperkey,
    Totp,
    Entries,
    Wallet,
}
