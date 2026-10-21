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
use crate::WalletKey;
use crate::create::NewKind;
use crate::explore::Reading;
use crate::glance::Glance;
use crate::gpg::GpgFile;
use crate::plan;
use crate::seeds::SeedsState;
use crate::vaults::VaultWallet;
use crate::wallet::{Key, NonceCheck, Wallet};
use crate::work;

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
        self.kept(|c| k.account_text(c, kind, account))
    }

    /// Key `k`'s key text at `path` ([`Key::text_at`]), kept on the key.
    /// Costs a key derivation the first time.
    pub fn text_at(&self, k: &Key, path: &str) -> Result<String, String> {
        self.kept(|c| k.text_at(c, path))
    }

    /// What tells key `k`'s words apart ([`Key::words_id`]), kept on the
    /// key. Costs a parse of the words the first time.
    pub fn words_id<'k>(&self, k: &'k Key) -> Option<&'k [u8; 32]> {
        self.kept(|c| k.words_id(c))
    }

    /// The key the words typed on Load a key make, for its status line
    /// (`EntryState::probed`), kept until an event forgets it. Costs a
    /// PBKDF2 and an elliptic-curve multiplication the first time.
    pub fn entry_probed(&self, m: &Mnemonic) -> Result<Fingerprint, String> {
        self.kept(|c| self.entry.probed(c, m))
    }

    /// Explore's reading ([`crate::Faraday::explore_reading`]), kept by
    /// what it is read from. Costs a key derivation and addresses the
    /// first time.
    pub fn explore_reading(&self) -> Option<Rc<Reading>> {
        self.kept(|c| self.app.explore_reading(c))
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
        self.kept(|c| self.app.seeds_address(c, s))
    }

    /// The files GPG Sign a file offers ([`crate::Faraday::gpg_files`]),
    /// each file's hash kept by the file. Costs a hash of each file the
    /// first time.
    pub fn gpg_files(&self) -> Vec<GpgFile> {
        self.kept(|_c| self.app.gpg_files())
    }

    /// Where the seed with master fingerprint `fp` is in the open vaults
    /// ([`crate::Faraday::vault_key_for`]), from each key record's kept
    /// fingerprint. Costs a PBKDF2 and two elliptic-curve multiplications
    /// per key record the first time.
    pub fn vault_key_for(&self, fp: Fingerprint) -> Option<(usize, usize)> {
        self.kept(|c| self.app.vault_key_for(c, fp))
    }

    /// The wallets open vault `v` holds with some of their seeds
    /// ([`crate::Faraday::vault_wallets_with_keys`]), from each key
    /// record's kept fingerprint. Costs a PBKDF2 and two elliptic-curve
    /// multiplications per key record the first time.
    pub fn vault_wallets_with_keys(&self, v: usize) -> Vec<VaultWallet> {
        self.kept(|c| self.app.vault_wallets_with_keys(c, v))
    }

    /// The wallet open vault `v`'s key record `r` loads with
    /// ([`crate::Faraday::vault_key_wallet`]), from the record's kept
    /// fingerprint. Costs a PBKDF2 and two elliptic-curve multiplications
    /// the first time.
    pub(crate) fn vault_key_wallet(&self, v: usize, r: usize) -> Option<String> {
        self.kept(|c| self.app.vault_key_wallet(c, v, r))
    }

    /// The keys open vaults hold that are not loaded
    /// ([`crate::seeds::vault_seeds`]), from each key record's kept
    /// fingerprint. Costs a PBKDF2 and two elliptic-curve multiplications
    /// per key record the first time.
    pub(crate) fn vault_seeds(&self) -> Vec<(usize, usize, String)> {
        self.kept(|c| crate::seeds::vault_seeds(c, self.app))
    }

    /// Open vault `v`'s wallet record `r` at a glance
    /// ([`crate::glance::of_vault`]), from each key record's kept
    /// fingerprint. Costs a PBKDF2 and two elliptic-curve multiplications
    /// per key record the first time.
    pub(crate) fn glance_of_vault(&self, v: usize, r: usize) -> Option<Glance> {
        self.kept(|c| crate::glance::of_vault(c, self.app, v, r))
    }

    /// The account key in slot `slot` of wallet `i`
    /// ([`crate::Faraday::wallet_key`]), from the key's kept account
    /// text. Costs a key derivation the first time.
    pub fn wallet_key(&self, i: usize, slot: u8) -> Result<WalletKey<'_>, String> {
        self.kept(|c| self.app.wallet_key(c, i, slot))
    }

    /// The descriptor the Seeds tool's piece describes
    /// ([`crate::Faraday::seeds_descriptor`]), from each key's kept text
    /// at the path. Costs a key derivation per key the first time.
    pub fn seeds_descriptor(&self, s: &SeedsState) -> Result<String, String> {
        self.kept(|c| self.app.seeds_descriptor(c, s))
    }

    /// Whether open vault `o` holds session key `k`'s seed, with its
    /// passphrase too when `with_pass` ([`crate::Faraday::vault_holds_seed`]),
    /// from the kept ids of the words. Costs a parse of the words per key
    /// record the first time.
    pub(crate) fn vault_holds_seed(&self, o: usize, k: usize, with_pass: bool) -> bool {
        self.kept(|c| self.app.vault_holds_seed(c, o, k, with_pass))
    }

    /// Whether open vault `v` holds session key `key`
    /// ([`crate::vault_screens::vault_has_key`]), from each key record's
    /// kept fingerprint and id. Costs a PBKDF2 and two elliptic-curve
    /// multiplications per key record the first time.
    pub(crate) fn vault_has_key(&self, v: usize, key: &Key) -> bool {
        self.kept(|c| crate::vault_screens::vault_has_key(c, self.app, v, key))
    }

    /// Where the wallet being backed up and its keys are kept
    /// ([`crate::Faraday::backup_kept`]), from each key record's kept
    /// fingerprint and id. Costs a PBKDF2 and two elliptic-curve
    /// multiplications per key record the first time.
    pub fn backup_kept(&self) -> Option<crate::backup::Kept> {
        self.kept(|c| self.app.backup_kept(c))
    }

    /// Loaded wallet `w` at a glance ([`crate::glance::of`]), from the
    /// memo and the values kept on the state. Costs what those cost the
    /// first time.
    pub(crate) fn glance_of(&self, w: usize) -> Option<Glance> {
        self.kept(|c| crate::glance::of(c, self.app, w))
    }

    /// The sheet for `target` on the chart of `press`
    /// ([`crate::glance_sheet::view`]), from the memo and the values kept
    /// on the state. Costs what those cost the first time.
    pub(crate) fn glance_sheet(
        &self,
        press: crate::glance::Press,
        target: crate::glance_sheet::Target,
    ) -> Option<crate::glance_sheet::View> {
        self.kept(|c| crate::glance_sheet::view(c, self.app, press, target))
    }

    /// Whether open vault `o` holds a seed the plan puts into another
    /// vault and not vault `v` ([`crate::Faraday::backup_vault_taken`]),
    /// from the kept ids of the words. Costs a parse of the words per key
    /// record the first time.
    pub fn backup_vault_taken(&self, v: usize, o: usize) -> bool {
        self.kept(|c| self.app.backup_vault_taken(c, v, o))
    }

    /// Checklist item `n`'s title and closed line
    /// ([`crate::screens::item_title`]), from the memo. Costs what the
    /// memo's answers cost the first time in a frame.
    pub(crate) fn item_title(&self, n: u8) -> (String, String) {
        self.kept(|c| crate::screens::item_title(c, self.app, n))
    }

    /// The keys the key scanner for wallet `w` takes
    /// ([`crate::Faraday::wallet_key_lines`]), from the wallet's chart.
    /// Costs what the chart costs the first time.
    pub(crate) fn wallet_key_lines(&self, w: usize) -> Vec<String> {
        self.kept(|c| self.app.wallet_key_lines(c, w))
    }

    /// The loaded keys a policy may name ([`crate::Faraday::tools_keys`]),
    /// from each key's kept account text. Costs a key derivation per key
    /// the first time.
    pub fn tools_keys(&self) -> Vec<(String, String)> {
        self.kept(|c| self.app.tools_keys(c))
    }

    /// What the Silent page shows of its key
    /// ([`crate::Faraday::silent_shown`]), kept on the flow's state.
    /// Costs two key derivations and an address the first time.
    pub(crate) fn silent_shown(&self) -> crate::silent::SilentShown {
        self.kept(|c| self.app.silent_shown(c))
    }

    /// The nonce check of the spend's PSBT `psbt`
    /// ([`crate::wallet::Session::nonce_check`]), kept on the spend by the
    /// PSBT's bytes; `None` with no spend open. Costs the session's
    /// account keys, a derivation per signature made here and the checks
    /// the first time.
    pub(crate) fn nonce_check(&self, psbt: &osk_psbt::Psbt) -> Option<Rc<NonceCheck>> {
        let s = self.spend.as_ref()?;
        let bytes = psbt.to_bytes();
        let mut cache = s.nonce_check.borrow_mut();
        if let Some((b, check)) = cache.as_ref()
            && *b == bytes
        {
            return Some(check.clone());
        }
        let check = Rc::new(self.kept(|c| work::keeping(|| self.session.nonce_check(c, psbt))));
        *cache = Some((bytes, check.clone()));
        Some(check)
    }

    /// A vault key record's fingerprint
    /// (`vault_screens::key_fingerprint`), kept by the key's id. Costs a
    /// PBKDF2 and two elliptic-curve multiplications the first time.
    pub fn vault_key_fingerprint(&self, r: &Record) -> Option<String> {
        self.kept(|c| crate::vault_screens::key_fingerprint(c, self.app, r))
    }
}
