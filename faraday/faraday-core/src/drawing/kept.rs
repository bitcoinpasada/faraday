//! Values kept on the state that drawing reads, on [`View`]: each worked
//! out the first time anyone asks, a frame included ([`crate::work::keeping`]),
//! kept with what it describes, by the inputs it is worked out from, and a
//! lookup after (`docs/SPEED.md` §2 rules 2 and 4, §3.3; `docs/COMPUTE.md`
//! §4). Each says what it reads. Decisions ask the same values on the
//! state; only drawing asks them here. Each is handed the view's token
//! (`View::kept`).

use std::rc::Rc;

use faraday_vault::Record;
use osk_bip::bip39::Mnemonic;
use osk_bip::bitcoin::ScriptBuf;
use osk_bip::keys::{Fingerprint, Network};
use osk_bip::policy::WalletPolicy;
use osk_codec::qr::QrMatrix;

use super::View;
use crate::create::NewKind;
use crate::explore::Reading;
use crate::gpg::GpgFile;
use crate::plan;
use crate::seeds::SeedsState;
use crate::wallet::{Key, Wallet};

impl View<'_> {
    /// Wallet `w`'s address on `network` at `change`/`index`
    /// ([`Wallet::address_on`]), kept on the wallet. Costs an address
    /// derivation the first time.
    pub fn address_on(&self, w: &Wallet, network: Network, change: bool, index: u32) -> String {
        self.kept(|_c| w.address_on(network, change, index))
    }

    /// Wallet `w`'s script at `change`/`index` ([`Wallet::script_at`]),
    /// kept on the wallet. Costs a script derivation the first time.
    pub fn script_at(&self, w: &Wallet, change: bool, index: u32) -> Option<ScriptBuf> {
        self.kept(|_c| w.script_at(change, index))
    }

    /// Key `k`'s account key text for `kind` and `account`
    /// ([`Key::account_text`]), kept on the key. Costs a key derivation
    /// the first time.
    pub fn account_text(&self, k: &Key, kind: NewKind, account: u32) -> Result<String, String> {
        self.kept(|_c| k.account_text(kind, account))
    }

    /// Key `k`'s key text at `path` ([`Key::text_at`]), kept on the key.
    /// Costs a key derivation the first time.
    pub fn text_at(&self, k: &Key, path: &str) -> Result<String, String> {
        self.kept(|_c| k.text_at(path))
    }

    /// What tells key `k`'s words apart ([`Key::words_id`]), kept on the
    /// key. Costs a parse of the words the first time.
    pub fn words_id<'k>(&self, k: &'k Key) -> Option<&'k [u8; 32]> {
        self.kept(|_c| k.words_id())
    }

    /// The key the words typed on Load a key make, for its status line
    /// (`EntryState::probed`), kept until an event forgets it. Costs a
    /// PBKDF2 and an elliptic-curve multiplication the first time.
    pub fn entry_probed(&self, m: &Mnemonic) -> Result<Fingerprint, String> {
        self.kept(|_c| self.entry.probed(m))
    }

    /// Explore's reading ([`crate::Faraday::explore_reading`]), kept by
    /// what it is read from. Costs a key derivation and addresses the
    /// first time.
    pub fn explore_reading(&self) -> Option<Rc<Reading>> {
        self.kept(|_c| self.app.explore_reading())
    }

    /// The QR code of public text `text` ([`crate::Faraday::public_qr`]),
    /// kept by its text. Costs a QR encoding the first time.
    pub fn public_qr(&self, text: &str) -> Option<Rc<QrMatrix>> {
        self.kept(|_c| self.app.public_qr(text))
    }

    /// Wallet description `text` read ([`crate::Faraday::read_kept`]),
    /// kept by its text. Costs a parse the first time.
    pub fn read_kept(&self, text: &str) -> Option<WalletPolicy> {
        self.kept(|_c| self.app.read_kept(text))
    }

    /// A plan's map and its check ([`crate::Faraday::plan_mapped`]), kept
    /// by the shape and answers. Costs a map and a check the first time.
    pub fn plan_mapped(&self, shape: &plan::Shape, a: &plan::Answers) -> Rc<plan::Mapped> {
        self.kept(|_c| self.app.plan_mapped(shape, a))
    }

    /// The Seeds tool's address ([`crate::Faraday::seeds_address`]), kept
    /// by what it is derived from. Costs an address derivation the first
    /// time.
    pub fn seeds_address(&self, s: &SeedsState) -> Result<String, String> {
        self.kept(|_c| self.app.seeds_address(s))
    }

    /// The files GPG Sign a file offers ([`crate::Faraday::gpg_files`]),
    /// each file's hash kept by the file. Costs a hash of each file the
    /// first time.
    pub fn gpg_files(&self) -> Vec<GpgFile> {
        self.kept(|_c| self.app.gpg_files())
    }

    /// A vault key record's fingerprint
    /// (`vault_screens::key_fingerprint`), kept by the key's id. Costs a
    /// PBKDF2 and two elliptic-curve multiplications the first time.
    pub fn vault_key_fingerprint(&self, r: &Record) -> Option<String> {
        self.kept(|_c| crate::vault_screens::key_fingerprint(self.app, r))
    }
}
