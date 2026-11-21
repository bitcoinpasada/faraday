//! The choices on New key's Start card (`docs/FLOWS.md`, New key).

use crate::forms::Form;
use faraday_forms::Typing;

/// The choices on the Start card: a form made here, a form typed, or a
/// SeedQR scanned. What a form is lives in the table (`faraday-forms`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// Make a new key from randomness in a form (`Form::made_here`): BIP-39
    /// words, SLIP-39 shares, or codex32 shares by hand
    /// (`docs/CODEX32.md` §6).
    Make(Form),
    /// Type a key already held in a form; Krux code is read, not typed:
    /// Add a key's Krux code form opens.
    Type(Form),
    /// Scan a SeedQR.
    Scan,
}

/// The choices, in the order the card lists them. Actions carry a
/// choice by its place here.
pub const STARTS: [Start; 9] = [
    Start::Make(Form::Words),
    Start::Make(Form::Codex32),
    Start::Type(Form::Words),
    Start::Scan,
    Start::Type(Form::Krux),
    Start::Type(Form::Codex32),
    Start::Make(Form::Slip39),
    Start::Type(Form::Slip39),
    Start::Type(Form::Xor),
];

/// The choices shown first; every other is under Other forms
/// (`docs/KEY-FORMS.md` §4.2).
const UP_FRONT: [Start; 6] = [
    Start::Make(Form::Words),
    Start::Make(Form::Codex32),
    Start::Type(Form::Words),
    Start::Scan,
    Start::Type(Form::Krux),
    Start::Type(Form::Codex32),
];

impl Start {
    /// Its place in [`STARTS`].
    pub fn index(self) -> u8 {
        STARTS.iter().position(|s| *s == self).unwrap_or(0) as u8
    }

    /// Listed under Other forms, closed at first: every choice not shown
    /// first.
    pub fn other(self) -> bool {
        !UP_FRONT.contains(&self)
    }

    /// Listed under Make a new key, rather than Add a key you have
    /// (`docs/CODEX32.md` §6).
    pub fn new_key(self) -> bool {
        match self {
            Start::Make(f) => match f {
                Form::Words | Form::Codex32 => true,
                Form::Slip39 | Form::Xor | Form::Krux => false,
            },
            Start::Type(_) | Start::Scan => false,
        }
    }

    /// The key is made here from randomness.
    pub fn makes(self) -> bool {
        match self {
            Start::Make(f) => f.made_here(),
            Start::Type(_) | Start::Scan => false,
        }
    }

    /// The form typed on the Words card, for a key typed: none for a form
    /// that is not typed.
    pub fn form(self) -> Option<Form> {
        match self {
            Start::Type(f) => match f.typing() {
                Typing::NotTyped => None,
                Typing::Words | Typing::ShareWords | Typing::Codex32 | Typing::PartWords => Some(f),
            },
            Start::Make(_) | Start::Scan => None,
        }
    }
}
