//! The choices on New key's Start card (`docs/FLOWS.md`, New key).

use crate::forms::Form;

/// The choices on the Start card (`docs/history/NEW-WALLET.md` §15.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// Make a new key from randomness, as BIP-39 words.
    Make,
    /// Type the words of a key already held.
    Words,
    /// Scan a SeedQR.
    Scan,
    /// Make a new key as SLIP-39 shares.
    MakeShares,
    /// Type SLIP-39 shares.
    Shares,
    /// Type a codex32 secret, or its shares.
    Codex32,
    /// Type Seed XOR parts.
    Xor,
    /// Make a new key as codex32 shares, by hand (`docs/CODEX32.md` §6).
    MakeCodex32,
}

/// The choices, in the order the card lists them. Actions carry a
/// choice by its place here.
pub const STARTS: [Start; 8] = [
    Start::Make,
    Start::MakeCodex32,
    Start::Words,
    Start::Scan,
    Start::Codex32,
    Start::MakeShares,
    Start::Shares,
    Start::Xor,
];

impl Start {
    /// Its place in [`STARTS`].
    pub fn index(self) -> u8 {
        STARTS.iter().position(|s| *s == self).unwrap_or(0) as u8
    }

    /// Listed under Other forms, closed at first.
    pub fn other(self) -> bool {
        matches!(self, Start::MakeShares | Start::Shares | Start::Xor)
    }

    /// Listed under Make a new key, rather than Add a key you have
    /// (`docs/CODEX32.md` §6).
    pub fn new_key(self) -> bool {
        matches!(self, Start::Make | Start::MakeCodex32)
    }

    /// The key is made here from randomness.
    pub fn makes(self) -> bool {
        matches!(self, Start::Make | Start::MakeShares | Start::MakeCodex32)
    }

    /// The form typed on the Words card, for a key typed.
    pub fn form(self) -> Option<Form> {
        match self {
            Start::Words => Some(Form::Words),
            Start::Shares => Some(Form::Slip39),
            Start::Codex32 => Some(Form::Codex32),
            Start::Xor => Some(Form::Xor),
            _ => None,
        }
    }

    /// The choice that types `form`.
    pub fn typing(form: Form) -> Start {
        match form {
            Form::Words => Start::Words,
            Form::Slip39 => Start::Shares,
            Form::Codex32 => Start::Codex32,
            Form::Xor => Start::Xor,
        }
    }
}
