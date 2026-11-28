//! What waits on the sheet: a secret on its way out and the hold that sends one out as a plain
//! file (`main`'s `SecretOut`, `Plain`).

use faraday_kernel::{KeyId, Public};
use faraday_bip::keys::Fingerprint;
use faraday_port::bytes::Bytes;
use faraday_text::Text;

/// What a secret becomes in a vault.
pub enum Keep {
    /// A note holding the file's text.
    Note,
    /// Session key `key`, whose fingerprint is `fp`, as the key record Vaults' Save writes.
    Key {
        key: KeyId,
        /// Checked before it is saved.
        fp: Fingerprint,
        /// The fields (codex32 set, BIP-85 origin) the person unticked on the sheet: they hold
        /// for this secret's save and end with it.
        left: Vec<u8>,
    },
    /// Nothing: a picture or a file Faraday does not read is not offered to a vault.
    None,
}

/// One form a secret's file can take.
pub struct SecretFile {
    /// The form, as the sheet's button names it.
    pub label: &'static str,
    /// The file name.
    pub name: Text,
    /// The file.
    pub bytes: Bytes,
}

/// What kind of secret a sheet offers.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Ack {
    /// Every secret but a seed's.
    Any,
    /// A seed's.
    Seed,
    /// Text or a file Faraday does not read: it cannot tell.
    Unknown,
}

/// A secret on its way out, waiting for the person to say where. It has no `Clone` and no
/// `Debug`; its bytes are wiped when it drops.
pub struct SecretOut {
    /// The file name it would go out under.
    pub name: Text,
    /// The file.
    pub bytes: Bytes,
    /// What it is, as the sheet names it.
    pub what: Text,
    /// What someone holding it can do.
    pub gives: Text,
    /// What it becomes in a vault.
    pub keep: Keep,
    /// The forms the file can take, when it can take more than one: the person picks one, and
    /// `name` and `bytes` are the one picked. Empty for a secret of one form.
    pub forms: Vec<SecretFile>,
    /// The form picked, by index into `forms`.
    pub form: usize,
    /// What it is to Faraday.
    pub ack: Ack,
    /// An Inbox file a stick visit would write to another stick: past the warning it is ticked
    /// on the visit, and goes nowhere else.
    pub to_visit: bool,
    /// Further files that go out with `name` and `bytes`, one file each: the other shares of a
    /// set, which never share a file.
    pub also: Vec<(Text, Bytes)>,
}

impl SecretOut {
    /// A secret of one form that goes into a vault as a note.
    pub fn new(name: &str, bytes: Bytes, what: &str, gives: &str) -> SecretOut {
        SecretOut {
            name: Text::of(name),
            bytes,
            what: Text::of(what),
            gives: Text::of(gives),
            keep: Keep::Note,
            forms: Vec::new(),
            form: 0,
            ack: Ack::Any,
            to_visit: false,
            also: Vec::new(),
        }
    }

    /// The names of every file that goes out, the first first.
    pub fn names(&self) -> Vec<&Text> {
        core::iter::once(&self.name)
            .chain(self.also.iter().map(|(n, _)| n))
            .collect()
    }
}

/// What a held "save without encryption" does. The data each of the others needs (the plan as it
/// was, the files' names) stays with the feature that asked.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Then {
    /// The secret sheet's secret goes out unprotected.
    Secret,
    /// A backup plan keeps these files on a stick at a place.
    Plan,
    /// Lock and write makes this file of the plan on a stick at a place.
    Lock,
    /// Transfer's Receive saves this file into the folder.
    Transfer,
    /// A vault record's Save to a stick writes its ticked files.
    VaultSave,
}

/// A secret about to be saved as a plain file, outside any vault: the sheet confirmed only by its
/// hold. It holds no secret.
#[derive(Public, PartialEq, Eq)]
pub struct Plain {
    /// What goes: a file's name, or several.
    pub name: Text,
    /// Where it goes, as the sheet's first line names it.
    pub on: Text,
    /// What keeps it, as the sheet's second line names it.
    pub keep: Text,
    /// What the hold does.
    pub then: Then,
}
