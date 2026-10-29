//! The block that says how Faraday does one cryptographic action
//! (`docs/CRYPTO-LEARN.md` §1): the type `how.rs` is generated into and
//! the apps draw (§8.5). The values are the manifest's, in plain
//! statements; the field labels are `faraday-strings`'.

/// One entry of a field: a line of text and how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    /// The statement.
    pub text: &'static str,
    /// How it is drawn.
    pub kind: Kind,
}

/// How a [`Line`] is drawn (`docs/CRYPTO-LEARN.md` §8.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Body M, on-surface-variant.
    Plain,
    /// Read character by character (a path, a function, a library's
    /// version, a fingerprint): Body M in the mono face.
    Mono,
    /// What is not checked ("not checked against vectors", "no fuzz
    /// target: ...", "no decision", "not stated by the library"): the
    /// Warning role with `Icon::Warning` before it.
    Warn,
}

/// One Code entry: the file and the function. The release is the build's
/// own and is put in front when the block is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code {
    /// The file, from the repository's root.
    pub file: &'static str,
    /// The function.
    pub function: &'static str,
}

/// What the cross-check says of the action (`docs/CRYPTO-LEARN.md` §10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossCheck {
    /// "not cross-checked: <reason>", whole, drawn as a Warn line.
    Not(&'static str),
    /// The action's area, written once and referred to by every block
    /// of the area.
    Area(&'static Area),
}

/// An area of the cross-check, its parts in the order the block draws
/// them. A part with nothing to say holds the line that says why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    /// The area.
    pub area: &'static str,
    /// The Faraday code it drives.
    pub code: &'static [Line],
    /// What it is compared with, and how that is itself checked first.
    pub second: &'static [Line],
    /// The published vectors it runs.
    pub vectors: &'static [Line],
    /// What its random cases vary.
    pub random: &'static [Line],
    /// Each known difference.
    pub differences: &'static [Line],
    /// When it runs.
    pub when: &'static [Line],
    /// The last passing run's seed, for this release.
    pub seed: &'static str,
    /// The last passing run's case count for the area.
    pub cases: u32,
}

/// One action's block: the nine fields of `docs/CRYPTO-LEARN.md` §1,
/// where it is offered, and its Learn page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// The action's id in the manifest.
    pub id: &'static str,
    /// The action's name, the block's heading.
    pub name: &'static str,
    /// Action: one sentence.
    pub action: &'static str,
    /// Standards, each by number, name and status.
    pub standards: &'static [Line],
    /// Implementation: the crate and module, or the library with its
    /// version, and what Faraday adds.
    pub implementation: &'static [Line],
    /// Checked against: vectors, tests, the self-test, the fuzz target.
    pub checked: &'static [Line],
    /// Cross-check.
    pub cross_check: CrossCheck,
    /// Code: the entries; the repository's address follows them.
    pub code: &'static [Code],
    /// Secret handling.
    pub secret: &'static [Line],
    /// Limits.
    pub limits: &'static [Line],
    /// Faraday's own calls.
    pub own_calls: &'static [Line],
    /// The screens that offer the action, by their `Screen` variant's name
    /// (`Spend`, `Family`), or the Starter's pages by `starter:<page>`
    /// (`starter:write`, `starter:confirm`, `starter:writing`,
    /// `starter:done`, `starter:start`).
    pub screens: &'static [&'static str],
    /// The id of the Learn page that carries the block after its sections
    /// (a page of `pages::EN` by its field name, or `faraday:<id>` for one
    /// of [`crate::ids`]); empty for none.
    pub learn: &'static str,
}
