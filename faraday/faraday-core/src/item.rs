//! A file in the Inbox or the Outbox.

use crate::receipt::sha256_of;
use crate::secrets;
use crate::wallet::{self, FileKind};
use faraday_crypto::work::Compute;

/// A file in the Inbox or the Outbox.
#[derive(Clone)]
pub struct Item {
    /// Its name.
    pub name: String,
    /// Its contents.
    pub bytes: Vec<u8>,
    /// What it holds.
    pub kind: FileKind,
    /// A secret the person let out unprotected, whatever its contents
    /// read as (`docs/FLOWS.md` decision 6).
    pub secret: bool,
    /// A picture copied in from a stick: what its QR codes read as, which
    /// the disk process decoded ([`FileKind::Other`] when none read). Its
    /// bytes are the picture's, kept to be written out as they came.
    pub picture: Option<FileKind>,
    /// SHA-256 of a vault's or an EFI image's bytes, taken once
    /// when the file came in (each is public: a vault's file is sealed);
    /// `None` for any other file. An Item's bytes never change after it
    /// is made.
    pub sha: Option<[u8; 32]>,
    /// What a vault file's header and length state, read once when the
    /// file came in as [`Item::sha`] is taken; `None` for any other file
    /// and for a vault file whose header does not read.
    pub facts: Option<faraday_vault::file::Facts>,
    /// The wallet a wallet file describes, read once when the file came
    /// in, as [`Item::sha`] is taken (`docs/SPEED.md` §3.3): `None` for
    /// any other file. An Item's bytes are never changed after it is
    /// made, so this stays what they say; nothing reads the bytes again for it.
    pub wallet: Option<faraday_bip::policy::WalletPolicy>,
    /// What a PSBT file says (its txid, its signatures, what its first
    /// input spends), read once when the file came in, as
    /// [`Item::wallet`] is: `None` for any other file. Which loaded
    /// wallet it spends from is found from it
    /// ([`wallet::Session::spends_from`]), so it follows the wallets.
    pub psbt: Option<wallet::PsbtSeen>,
    /// The outpoints a PSBT or a finished transaction spends, sorted,
    /// read once when the file came in, as [`Item::psbt`] is; empty for
    /// any other file.
    pub outpoints: Vec<faraday_bip::bitcoin::OutPoint>,
    /// The id of the unsigned transaction a PSBT or a finished
    /// transaction holds, read once when the file came in; `None` for any
    /// other file.
    pub unsigned_txid: Option<faraday_bip::bitcoin::Txid>,
}

/// Prints the name, the byte count and the `secret` flag, never the bytes:
/// a file can be a secret.
impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Item))
            .field("name", &self.name)
            .field("len", &self.bytes.len())
            .field("secret", &self.secret)
            .finish_non_exhaustive()
    }
}

/// An Inbox or Outbox file may be a seed or a share written in the clear:
/// its bytes are wiped when it goes, whatever it holds.
impl Drop for Item {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.bytes);
    }
}

impl Item {
    /// Costs a classification, a parse of a wallet file's description and a
    /// hash of a vault or an EFI image. The file as it came in,
    /// classified, with what it describes read once.
    pub(crate) fn new(c: &Compute, name: &str, bytes: Vec<u8>) -> Item {
        let wallet::Classified {
            kind,
            psbt,
            wallet,
            outpoints,
            unsigned_txid,
        } = wallet::classify_read(c, name, &bytes);
        let sha =
            matches!(kind, FileKind::Vault | FileKind::EfiImage).then(|| sha256_of(c, &bytes));
        let facts = (kind == FileKind::Vault)
            .then(|| faraday_vault::file::inspect(&bytes).ok())
            .flatten();
        Item {
            name: name.to_string(),
            bytes,
            kind,
            secret: false,
            picture: None,
            sha,
            facts,
            wallet,
            psbt,
            outpoints,
            unsigned_txid,
        }
    }

    /// What a PSBT file says, read when it came in ([`Item::psbt`]): a
    /// lookup.
    pub fn psbt(&self) -> Option<&wallet::PsbtSeen> {
        self.psbt.as_ref().filter(|_| self.kind == FileKind::Psbt)
    }

    /// The wallet a wallet file describes, read when it came in. A lookup.
    pub fn wallet(&self) -> Option<&faraday_bip::policy::WalletPolicy> {
        self.wallet.as_ref()
    }

    /// Costs a hash of a whole file (through `sha256_of`). SHA-256 of its
    /// bytes: the one taken when it came in ([`Item::sha`], a
    /// vault's or an EFI image's), a lookup. Any other file is hashed now.
    pub fn hash(&self, c: &Compute) -> [u8; 32] {
        self.sha.unwrap_or_else(|| sha256_of(c, &self.bytes))
    }

    /// What it holds: a picture's codes, or the file itself.
    pub fn holds(&self) -> FileKind {
        self.picture.unwrap_or(self.kind)
    }

    /// A lock drops it from the Inbox: it may hold a secret, which the
    /// next process must not inherit (`docs/history/PLAN.md` §5.3).
    pub(crate) fn dropped_at_lock(&self) -> bool {
        self.holds().may_be_secret() || self.secret
    }

    /// Who may read it: a secret let out is one, whatever its kind; a
    /// picture is what its codes hold.
    pub fn exposure(&self) -> secrets::Exposure {
        if self.secret {
            secrets::Exposure::Secret
        } else {
            self.holds().exposure()
        }
    }

    /// What a stick visit asks the person to say before it writes this
    /// Inbox file, on the secret sheet: `None` for a public or sealed file,
    /// written as it is.
    pub(crate) fn copy_ack(&self) -> Option<secrets::Ack> {
        if self.secret {
            return Some(secrets::Ack::Any);
        }
        match self.holds() {
            FileKind::Words | FileKind::SeedPart => Some(secrets::Ack::Seed),
            FileKind::Entries | FileKind::SecretWallet => Some(secrets::Ack::Any),
            FileKind::Text | FileKind::Other => Some(secrets::Ack::Unknown),
            _ => None,
        }
    }
}
