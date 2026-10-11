//! The Learn pages: what a person needs to know to hold and use a key,
//! as titled sections of paragraphs (`docs/PLANNING.md` §16.139).
//!
//! The text is plain data with no screen in it: a page is a title and
//! its sections, and [`Learn::pages`] lists them in reading order. Which
//! screen opens which page, and which flow a page offers to start, is
//! the app's business. One static per language; [`EN`] is English.

#![no_std]

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
pub const PAGES: usize = 27;

/// Every page, in one language.
pub struct Learn {
    /// "Start here" (UX.md §7.1, job A4): the four things a
    /// person with only this app needs before they hold a key. It is
    /// the screen a device with nothing loaded opens on until it has
    /// been left once, and the first page of Learn afterwards.
    pub start_here: Page,
    /// "Words" (UX.md L1): what the words are, why 12 or 24,
    /// and that anyone holding them holds the coins.
    pub words: Page,
    /// "Signer, wallet, node" (UX.md L2): the three jobs, what
    /// this device does, and why it shows no balance.
    pub devices: Page,
    /// "Transactions" (UX.md L3): inputs, outputs, change, fees
    /// and what the Sign screens list.
    pub transactions: Page,
    /// "Randomness" (UX.md L4): why dice or coins, what a
    /// computer's randomness is, and what can be checked.
    pub randomness: Page,
    /// "Where randomness comes from": one section per entropy
    /// source the Create wizard offers, stating what each one asks you
    /// to trust.
    pub where_randomness: Page,
    /// "Backups" (UX.md L5, L6): paper and steel, what destroys
    /// them, where to keep them, the never list, and the quiz.
    pub backups: Page,
    /// "Encrypted backups": what an encrypted backup is, what
    /// its passphrase costs, and where it belongs beside a written one.
    pub encrypted_backups: Page,
    /// "Seed XOR": what a part is, that every part is needed,
    /// that a part is itself a real key, and that XOR is not Shamir.
    pub seed_xor: Page,
    /// "Passphrases" (UX.md L7): what one adds, that it makes a
    /// different key, that forgetting it is loss.
    pub passphrases: Page,
    /// "Verifying" (UX.md L10): why to check addresses and
    /// change, and what the danger cards mean.
    pub verifying: Page,
    /// "The air gap" (UX.md L12): what moves in and out, and
    /// what the gap does and does not protect against.
    pub air_gap: Page,
    /// "Mistakes and scams" (UX.md L14): the five a person
    /// meets first, one paragraph each.
    pub scams: Page,
    /// "Multisig" (UX.md L9): what m-of-n is, what it buys,
    /// what it costs, when it is the wrong answer, and how this device
    /// joins one.
    pub multisig: Page,
    /// "Spend paths and timelocks": what a spend path is, what
    /// `older` and `after` mean, when a timelock starts counting, and
    /// who chooses which path a transaction uses.
    pub spend_paths: Page,
    /// "Xpubs and privacy" (UX.md L11): what an account key
    /// shows, what it cannot do, and who should have one.
    pub xpubs: Page,
    /// "The secure element" (UX.md L13): what the chip does for
    /// a key kept on a phone, what it does not, the PIN and the duress
    /// PIN, and why a lost phone means the words.
    pub secure_element: Page,
    /// "Inheritance" (UX.md L15): what an heir needs, what to
    /// leave, what not to leave, and where the passphrase fits.
    pub inheritance: Page,
    /// "Nonces": what the one-time secret in a signature is,
    /// what a leaked or biased one costs, how this device derives every
    /// one of them, and how to check that against another signer.
    pub nonces: Page,
    /// "Signing a message": what a message signature is, what
    /// it proves, what it does not, and how one is checked here.
    pub message: Page,
    /// "Glossary" (UX.md L17): the terms the screens use, one
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
    /// and the decoy property.
    pub frost: Page,
    /// "Backups in other forms": SLIP-39, Codex32, Seed XOR and
    /// the encrypted backup, what each trades away, and what this
    /// device reads today.
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
            &self.devices,
            &self.transactions,
            &self.randomness,
            &self.where_randomness,
            &self.backups,
            &self.encrypted_backups,
            &self.seed_xor,
            &self.passphrases,
            &self.verifying,
            &self.air_gap,
            &self.scams,
            &self.multisig,
            &self.spend_paths,
            &self.xpubs,
            &self.secure_element,
            &self.inheritance,
            &self.nonces,
            &self.message,
            &self.glossary,
            &self.tools,
            &self.wallet_kinds,
            &self.frost,
            &self.other_backups,
            &self.coordinators,
            &self.silent_payments,
        ]
    }
}
