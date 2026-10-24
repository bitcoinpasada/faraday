//! The Learn pages, edited as
//! `docs/learn/NN-name.md` (`tools/learn/sync.py`): what a person needs to know to hold and use a key,
//! as titled sections of paragraphs.
//!
//! The text is plain data with no screen in it: a page is a title and
//! its sections, and [`Learn::pages`] lists them in reading order. Which
//! screen opens which page, and which flow a page offers to start, is
//! the app's business. One static per language; [`EN`] is English.

mod en;

pub use en::EN;

/// One page: its title and the sections under it.
pub struct Page {
    /// Title of the page, and the label of the row that opens it.
    pub title: &'static str,
    /// The sections, in the order the page shows them.
    pub sections: &'static [Section],
}

/// One section of a [`Page`]: a heading and its paragraphs.
pub struct Section {
    /// The heading.
    pub heading: &'static str,
    /// The paragraphs under it.
    pub paragraphs: &'static [&'static str],
}

/// How many pages there are.
#[cfg(feature = "unreleased")]
pub const PAGES: usize = 21;
/// How many pages there are: the FROST page is compiled out of the app
/// (F109).
#[cfg(not(feature = "unreleased"))]
pub const PAGES: usize = 20;

/// Every page, in one language.
pub struct Learn {
    /// "Start here": the four things a
    /// person with only this app needs before they hold a key. It is
    /// the screen a device with nothing loaded opens on until it has
    /// been left once, and the first page of Learn afterwards.
    pub start_here: Page,
    /// "Words": what the words are, why 12 or 24,
    /// and that anyone holding them holds the coins.
    pub words: Page,
    /// "Randomness": one section per entropy source the Create
    /// wizard offers, stating what each one asks you to trust.
    pub randomness: Page,
    /// "Backups": paper and steel, what destroys
    /// them, the never list, and the quiz.
    pub backups: Page,
    /// "Passphrases": what one adds, that it makes a
    /// different key, that forgetting it is loss.
    pub passphrases: Page,
    /// "Verifying": why to check addresses and
    /// change, what the Sign screens list, and what the danger cards mean.
    pub verifying: Page,
    /// "The air gap": what moves in and out, and
    /// what the gap does and does not protect against.
    pub air_gap: Page,
    /// "Mistakes and scams": the five a person
    /// meets first, one paragraph each.
    pub scams: Page,
    /// "Multisig": what m-of-n is, what it buys,
    /// what it costs, when it is the wrong answer, and how this device
    /// joins one.
    pub multisig: Page,
    /// "Spend paths and timelocks": what a spend path is, what
    /// `older` and `after` mean, when a timelock starts counting, and
    /// who chooses which path a transaction uses.
    pub spend_paths: Page,
    /// "Xpubs and privacy": what an account key
    /// shows, what it cannot do, and who should have one.
    pub xpubs: Page,
    /// "Inheritance": what an heir needs, what to
    /// leave, what not to leave, and where the passphrase fits.
    pub inheritance: Page,
    /// "Nonces": what the one-time secret in a signature is,
    /// what a leaked or biased one costs, how this device derives every
    /// one of them, and how to check that against another signer.
    pub nonces: Page,
    /// "Signing a message": what a message signature is, what
    /// it proves, what it does not, and how one is checked here.
    pub message: Page,
    /// "Glossary": the terms the screens use, one
    /// line each, in alphabetical order.
    pub glossary: Page,
    /// "Tools": what each of the standalone calculators is for.
    pub tools: Page,
    /// "Kinds of wallets": what a wallet is here, one section
    /// per kind the device understands with what it is for and what it
    /// trades off, and the questions that decide between them.
    pub wallet_kinds: Page,
    /// "FROST": shares as 24-word keys, the group record, the
    /// two-location route and its carry file, what a lost share means,
    /// and the decoy property. Compiled out of the app (F109).
    #[cfg(feature = "unreleased")]
    pub frost: Page,
    /// "Backups in other forms": SLIP-39, Codex32, Seed XOR and
    /// a vault, and what each trades away.
    pub other_backups: Page,
    /// "Coordinator files": what a coordinator is, what crosses
    /// between it and the device in each direction, BSMS, and why the
    /// device verifies everything it is sent.
    pub coordinators: Page,
    /// "Silent payments": what the address is, the two keys
    /// behind it, what the scan key gives away, labels, how a payment
    /// is checked, and why this device does not send one yet.
    pub silent_payments: Page,
}

impl Learn {
    /// The pages, in reading order.
    pub fn pages(&self) -> [&Page; PAGES] {
        [
            &self.start_here,
            &self.words,
            &self.randomness,
            &self.backups,
            &self.passphrases,
            &self.verifying,
            &self.air_gap,
            &self.scams,
            &self.multisig,
            &self.spend_paths,
            &self.xpubs,
            &self.inheritance,
            &self.nonces,
            &self.message,
            &self.glossary,
            &self.tools,
            &self.wallet_kinds,
            #[cfg(feature = "unreleased")]
            &self.frost,
            &self.other_backups,
            &self.coordinators,
            &self.silent_payments,
        ]
    }
}
