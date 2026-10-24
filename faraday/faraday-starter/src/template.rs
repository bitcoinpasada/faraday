//! Blank templates (`docs/STARTER.md` §3.8): the choices, the sheet they
//! make and the file's name. The sheet is `faraday-core`'s own blank sheet
//! with no wallet; nothing here holds or asks for a key or a seed.

use faraday_bip::keys::Network;
use faraday_core::pdf;
pub use faraday_core::wallet_backup::BlankCopy as Copy;
use faraday_core::wallet_backup::sheet_blank;
use faraday_crypto::work::Compute;

use crate::api::Reason;
use crate::sheets;

/// The part of the file's name for a choice of Copy.
fn file_part(c: Copy) -> &'static str {
    match c {
        Copy::Words => "words",
        Copy::SeedQr => "seedqr",
        Copy::Both => "words-seedqr",
    }
}

/// Which form a blank template is for (`docs/CODEX32.md` §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// BIP-39 words.
    Words,
    /// codex32 shares.
    Codex32,
}

/// What a codex32 template is copied onto: the string's boxes, the
/// Codex32QR grid, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexCopy {
    /// The boxes of the string.
    String,
    /// The Codex32QR grid, at 48 characters.
    Codex32Qr,
    /// Both on one sheet, at 48 characters.
    Both,
}

impl CodexCopy {
    /// Every choice, in the order a Choice lists them.
    pub const ALL: [CodexCopy; 3] = [CodexCopy::String, CodexCopy::Codex32Qr, CodexCopy::Both];

    /// The sheet file's `copy:` value.
    fn line(self) -> String {
        match self {
            CodexCopy::String => "string".into(),
            CodexCopy::Codex32Qr => "codex32qr".into(),
            CodexCopy::Both => [CodexCopy::String.line(), CodexCopy::Codex32Qr.line()].join(" "),
        }
    }

    /// The part of the file's name.
    fn file_part(self) -> &'static str {
        match self {
            CodexCopy::String => "string",
            CodexCopy::Codex32Qr => "codex32qr",
            CodexCopy::Both => "string-codex32qr",
        }
    }
}

/// The lengths of a codex32 template, in characters.
pub const CODEX_LENGTHS: [usize; 3] = [48, 74, 127];

/// The choices and what became of the last save or print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// The form: words (§3.8) or codex32 (`docs/CODEX32.md` §11).
    pub form: Form,
    /// 12 or 24 words.
    pub words: usize,
    /// Codex32: 48, 74 or 127 characters.
    pub chars: usize,
    /// Codex32: what is copied onto the card.
    pub codex_copy: CodexCopy,
    /// Codex32: the booklet's worksheets are printed too (none at 127).
    pub worksheets: bool,
    /// What is copied onto the sheet.
    pub copy: Copy,
    /// Places to fill in by hand: one or more.
    pub places: usize,
    /// A save is under way.
    pub saving: bool,
    /// Where the last save put the PDF, or why not.
    pub saved: Option<Result<String, Reason>>,
    /// Why the last open did not happen.
    pub open_failed: Option<Reason>,
}

impl Default for Template {
    fn default() -> Self {
        Template {
            form: Form::Words,
            words: 24,
            chars: 48,
            codex_copy: CodexCopy::String,
            worksheets: false,
            copy: Copy::Words,
            places: 2,
            saving: false,
            saved: None,
            open_failed: None,
        }
    }
}

impl Template {
    /// The sheet file: `faraday-core`'s blank sheet with no wallet, the
    /// network "mainnet", the places and what is copied.
    pub fn sheet(&self) -> String {
        if self.form == Form::Codex32 {
            return format!(
                "faraday-sheet 1\nkind: codex32\nchars: {}\ncopy: {}\nworksheets: {}\nplaces: {}\n",
                self.chars,
                self.codex_copy.line(),
                if self.worksheets { "yes" } else { "no" },
                self.places
            );
        }
        sheet_blank(self.words, self.copy, None, Network::Mainnet, self.places)
    }

    /// Costs what `sheet` costs. The PDF of [`Template::sheet`].
    pub fn pdf(&self, c: &Compute) -> Result<Vec<u8>, String> {
        if self.form == Form::Codex32 {
            return sheets::sheet(c, &self.sheet());
        }
        pdf::sheet(c, &self.sheet())
    }

    /// The file's name.
    pub fn file_name(&self) -> String {
        if self.form == Form::Codex32 {
            return format!(
                "faraday-blank-template-codex32-{}-chars-{}{}.pdf",
                self.chars,
                self.codex_copy.file_part(),
                if self.worksheets { "-worksheets" } else { "" }
            );
        }
        format!(
            "faraday-blank-template-{}-words-{}.pdf",
            self.words,
            file_part(self.copy)
        )
    }
}

impl Template {
    /// Chooses a codex32 length. The Codex32QR grid is for 48 characters
    /// only and the worksheets are published for 48 and 74: a choice
    /// that no longer holds goes back to the string and to none.
    pub fn set_chars(&mut self, chars: usize) {
        if !CODEX_LENGTHS.contains(&chars) {
            return;
        }
        self.chars = chars;
        if chars != 48 {
            self.codex_copy = CodexCopy::String;
        }
        if chars == 127 {
            self.worksheets = false;
        }
    }

    /// Chooses what a codex32 card is copied onto; the grid only at 48.
    pub fn set_codex_copy(&mut self, copy: CodexCopy) {
        if copy == CodexCopy::String || self.chars == 48 {
            self.codex_copy = copy;
        }
    }

    /// Chooses the worksheets; none at 127 characters.
    pub fn set_worksheets(&mut self, on: bool) {
        self.worksheets = on && self.chars != 127;
    }
}
