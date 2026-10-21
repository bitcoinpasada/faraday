//! Blank templates (`docs/STARTER.md` §3.8): the choices, the sheet they
//! make and the file's name. The sheet is `faraday-core`'s own blank sheet
//! with no wallet; nothing here holds or asks for a key or a seed.

pub use faraday_core::backup::BlankCopy as Copy;
use faraday_core::backup::sheet_blank;
use faraday_core::pdf;
use osk_bip::keys::Network;
use osk_crypto::work::Compute;

use crate::api::Reason;

/// The part of the file's name for a choice of Copy.
fn file_part(c: Copy) -> &'static str {
    match c {
        Copy::Words => "words",
        Copy::SeedQr => "seedqr",
        Copy::Both => "words-seedqr",
    }
}

/// The choices and what became of the last save or print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// 12 or 24 words.
    pub words: usize,
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
            words: 24,
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
        sheet_blank(self.words, self.copy, None, Network::Mainnet, self.places)
    }

    /// The PDF of [`Template::sheet`].
    pub fn pdf(&self, c: &Compute) -> Result<Vec<u8>, String> {
        pdf::sheet(c, &self.sheet())
    }

    /// The file's name.
    pub fn file_name(&self) -> String {
        format!(
            "faraday-blank-template-{}-words-{}.pdf",
            self.words,
            file_part(self.copy)
        )
    }
}
