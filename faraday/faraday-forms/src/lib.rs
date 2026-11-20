//! The registry of the forms a key comes in (`docs/KEY-FORMS.md`, F299).
//!
//! A form's facts are stated here once. It holds the identifier and plain
//! facts only: no secret, no parser and no type of `faraday-core`. Every
//! method is a `match` that names every form, so a form added to the table
//! fails to compile wherever a fact about it is not said. The table's order
//! is its codes: a form goes at the end and none is moved or removed.

use faraday_strings::EN;

/// A form a key comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Form {
    /// BIP-39 words.
    #[default]
    Words,
    /// SLIP-39 shares.
    Slip39,
    /// codex32 strings: the secret, or shares of it.
    Codex32,
    /// Seed XOR parts, each BIP-39 words.
    Xor,
    /// A Krux encrypted code, scanned or taken from Files and opened under
    /// its passphrase (`docs/KEF.md` §15). It collects no parts.
    Krux,
}

/// How a form is typed, which picks the entry Add a key draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typing {
    /// The word grid of a BIP-39 list.
    Words,
    /// Shares typed one at a time, each a list of words.
    ShareWords,
    /// Strings on the codex32 keyboard.
    Codex32,
    /// Parts, each BIP-39 words, every one needed.
    PartWords,
    /// Not typed: read from a code or a file only.
    NotTyped,
}

/// Whether a form is one piece or a set put together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Collects {
    /// One piece.
    One,
    /// A set of shares or parts.
    Set,
}

/// What passphrase goes with a form once it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Passphrase {
    /// A BIP-39 passphrase.
    Bip39,
    /// SLIP-39's own passphrase.
    Slip39,
    /// None.
    None,
}

/// What a form gives the key boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Makes {
    /// BIP-39 words.
    Words,
    /// SLIP-39 shares.
    Slip39Shares,
    /// A master seed.
    MasterSeed,
    /// A master key and no seed: what an extended private key at depth 0
    /// holds.
    MasterKey,
}

/// What a key of a form is written as for a backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Writing {
    /// The words.
    Words,
    /// A SeedQR.
    SeedQr,
    /// Each codex32 string.
    Codex32String,
    /// Each codex32 string's Codex32QR.
    Codex32Qr,
    /// Each Seed XOR part's words.
    PartWords,
    /// Each SLIP-39 share's words.
    ShareWords,
    /// A Krux code.
    KruxCode,
}

/// Where an output is put.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Medium {
    /// On paper, by hand.
    Paper,
    /// A file on a stick.
    File,
}

/// One output of a form: what it is written as, and on which medium.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Output {
    /// What is written.
    pub writing: Writing,
    /// Where.
    pub medium: Medium,
}

const fn out(writing: Writing, medium: Medium) -> Output {
    Output { writing, medium }
}

const WORDS: [Output; 4] = [
    out(Writing::Words, Medium::Paper),
    out(Writing::SeedQr, Medium::Paper),
    out(Writing::Words, Medium::File),
    out(Writing::SeedQr, Medium::File),
];
const SHARES: [Output; 1] = [out(Writing::ShareWords, Medium::Paper)];
const CODEX32: [Output; 4] = [
    out(Writing::Codex32String, Medium::Paper),
    out(Writing::Codex32Qr, Medium::Paper),
    out(Writing::Codex32String, Medium::File),
    out(Writing::Codex32Qr, Medium::File),
];
const PARTS: [Output; 1] = [out(Writing::PartWords, Medium::Paper)];
const KRUX: [Output; 2] = [
    out(Writing::KruxCode, Medium::Paper),
    out(Writing::KruxCode, Medium::File),
];

impl Form {
    /// Every form, in the table's order.
    pub const ALL: [Form; 5] = [
        Form::Words,
        Form::Slip39,
        Form::Codex32,
        Form::Xor,
        Form::Krux,
    ];

    /// Its name on screen.
    pub fn name(self) -> &'static str {
        match self {
            Form::Words => EN.forms_bip_39_words,
            Form::Slip39 => EN.forms_slip_39_shares,
            Form::Codex32 => EN.forms_codex32,
            Form::Xor => EN.forms_seed_xor_parts,
            Form::Krux => EN.forms_krux_code,
        }
    }

    /// Its stable code: its place in the table, carried by actions, the
    /// guide's keys and the plan's spellings.
    pub fn code(self) -> u8 {
        match self {
            Form::Words => 0,
            Form::Slip39 => 1,
            Form::Codex32 => 2,
            Form::Xor => 3,
            Form::Krux => 4,
        }
    }

    /// The form of a code, if it is one.
    pub fn from_code(code: u8) -> Option<Form> {
        Form::ALL.into_iter().find(|f| f.code() == code)
    }

    /// How it is typed.
    pub fn typing(self) -> Typing {
        match self {
            Form::Words => Typing::Words,
            Form::Slip39 => Typing::ShareWords,
            Form::Codex32 => Typing::Codex32,
            Form::Xor => Typing::PartWords,
            Form::Krux => Typing::NotTyped,
        }
    }

    /// One piece, or a set put together.
    pub fn collects(self) -> Collects {
        match self {
            Form::Words | Form::Krux => Collects::One,
            Form::Slip39 | Form::Codex32 | Form::Xor => Collects::Set,
        }
    }

    /// What passphrase goes with it once read.
    pub fn passphrase(self) -> Passphrase {
        match self {
            Form::Words | Form::Xor | Form::Krux => Passphrase::Bip39,
            Form::Slip39 => Passphrase::Slip39,
            Form::Codex32 => Passphrase::None,
        }
    }

    /// What it gives the key boundary.
    pub fn makes(self) -> Makes {
        match self {
            Form::Words | Form::Xor | Form::Krux => Makes::Words,
            Form::Slip39 => Makes::Slip39Shares,
            Form::Codex32 => Makes::MasterSeed,
        }
    }

    /// What a key of this form is written as for a backup.
    pub fn outputs(self) -> &'static [Output] {
        match self {
            Form::Words => &WORDS,
            Form::Slip39 => &SHARES,
            Form::Codex32 => &CODEX32,
            Form::Xor => &PARTS,
            Form::Krux => &KRUX,
        }
    }

    /// Whether Faraday makes a new key in it from randomness.
    pub fn made_here(self) -> bool {
        match self {
            Form::Words | Form::Slip39 | Form::Codex32 => true,
            Form::Xor | Form::Krux => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_form_has_a_name() {
        for f in Form::ALL {
            assert!(!f.name().is_empty());
        }
    }

    #[test]
    fn a_code_is_its_place() {
        for (i, f) in Form::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(f.code()), i);
            assert_eq!(Form::from_code(f.code()), Some(f));
        }
    }
}
