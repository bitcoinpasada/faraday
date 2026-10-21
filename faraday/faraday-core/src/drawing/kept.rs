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
        self.kept(|c| w.address_on(c, network, change, index))
    }

    /// The Inbox PSBT loaded wallet `w` spends from with the fewest
    /// signatures ([`crate::Faraday::psbt_for`]), from each wallet's kept
    /// scripts. Costs a script derivation per wallet and place the first
    /// time.
    pub fn psbt_for(&self, w: usize) -> Option<usize> {
        self.kept(|c| self.app.psbt_for(c, w))
    }

    /// The loaded wallet a PSBT read before spends from
    /// ([`crate::wallet::Session::spends_from`]), from each wallet's kept
    /// scripts. Costs a script derivation per wallet and place the first
    /// time.
    pub fn spends_from(&self, seen: &crate::wallet::PsbtSeen) -> Option<usize> {
        self.kept(|c| self.session.spends_from(c, seen))
    }

    /// An address of wallet `w` as the screen shows it
    /// ([`crate::wallet::Session::address_shown`]), kept on the wallet.
    /// Costs an address derivation the first time.
    pub fn address_shown(&self, w: &Wallet, change: bool, index: u32) -> String {
        self.kept(|c| self.session.address_shown(c, w, change, index))
    }

    /// Wallet `w`'s script at `change`/`index` ([`Wallet::script_at`]),
    /// kept on the wallet. Costs a script derivation the first time.
    pub fn script_at(&self, w: &Wallet, change: bool, index: u32) -> Option<ScriptBuf> {
        self.kept(|c| w.script_at(c, change, index))
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
        self.kept(|c| self.app.public_qr(c, text))
    }

    /// Wallet description `text` read ([`crate::Faraday::read_kept`]),
    /// kept by its text. Costs a parse the first time.
    pub fn read_kept(&self, text: &str) -> Option<WalletPolicy> {
        self.kept(|c| self.app.read_kept(c, text))
    }

    /// A plan's map and its check ([`crate::Faraday::plan_mapped`]), kept
    /// by the shape and answers. Costs a map and a check the first time.
    pub fn plan_mapped(&self, shape: &plan::Shape, a: &plan::Answers) -> Rc<plan::Mapped> {
        self.kept(|c| self.app.plan_mapped(c, shape, a))
    }

    /// A plan's check ([`plan::Mapped::check`]) on its kept map
    /// ([`crate::Faraday::plan_mapped`]), kept with it by the shape and
    /// answers. Costs a map and a check the first time.
    pub(crate) fn plan_check(&self, shape: &plan::Shape, a: &plan::Answers) -> plan::Check {
        self.kept(|c| self.app.plan_mapped(c, shape, a).check(c, shape, a))
    }

    /// The forms of the copies of seed `i` the backup's map keeps
    /// ([`crate::Faraday::copy_forms`]), from the memo's shape and the
    /// kept map. Costs a map the first time.
    pub(crate) fn copy_forms(&self, b: &crate::BackupState, i: usize) -> plan::CopyForms {
        let shape = self.plan_shape(b.wallet);
        plan::copy_forms_on(&self.plan_mapped(&shape, &b.answers), i)
    }

    /// What checking seed `i`'s copies asks for with the cameras listed
    /// now ([`crate::Faraday::copy_checks`]), from
    /// [`View::copy_forms`]. Costs a map the first time.
    pub(crate) fn copy_checks(
        &self,
        b: &crate::BackupState,
        i: usize,
        key: &Key,
    ) -> plan::CopyChecks {
        self.copy_forms(b, i).checks(
            !self.cameras.is_empty(),
            key.language == osk_bip::bip39::Language::English,
        )
    }

    /// The cards of the backup's plan, in order
    /// ([`crate::Faraday::backup_questions`]), from the memo's shape and
    /// the kept map. Costs a map the first time.
    pub(crate) fn backup_questions(&self) -> Vec<u8> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let shape = self.plan_shape(b.wallet);
        crate::plan_cards_on(&self.plan_mapped(&shape, &b.answers))
    }

    /// Signed message `text` checked ([`crate::Faraday::message_check`]),
    /// kept by its text, the network and the loaded wallets. Costs an
    /// address derivation per loaded wallet and address tried and a
    /// signature check the first time.
    pub(crate) fn message_check(&self, text: &str) -> Option<Rc<crate::wallet::MessageCheck>> {
        self.kept(|c| self.app.message_check(c, text))
    }

    /// The wallet description record `r` holds
    /// ([`crate::Faraday::record_described`]), as its open vault keeps it
    /// read, or as a description no record keeps, kept by its text.
    /// Costs a parse the first time.
    pub(crate) fn record_described(
        &self,
        r: &Record,
    ) -> Option<std::borrow::Cow<'_, crate::vaults::Described>> {
        self.kept(|c| self.app.record_described(c, r))
    }

    /// The texts of the shares the restore screen puts together
    /// ([`crate::Faraday::restore_share_texts`]), each vault share's
    /// wallet read once and kept by its text. Costs a parse per vault
    /// share's wallet the first time.
    pub fn restore_share_texts(&self) -> Vec<String> {
        self.kept(|c| self.app.restore_share_texts(c))
    }

    /// Whether the checklist's writes are done
    /// ([`crate::Faraday::wallet_sticks_done`]), from the memo's shape and
    /// the kept map. Costs a map the first time.
    pub(crate) fn wallet_sticks_done(&self) -> bool {
        self.kept(|c| self.app.wallet_sticks_done(c))
    }

    /// Each record of kind `k` among open vault records `o` with the
    /// wallet description it holds ([`crate::vaults::VaultRecords::described_of`]),
    /// as the vault keeps them read. Costs a parse of each description
    /// the first time after a change.
    pub(crate) fn described_of<'r>(
        &self,
        o: &'r crate::vaults::VaultRecords,
        k: u8,
    ) -> Vec<(usize, &'r Record, &'r crate::vaults::Described)> {
        self.kept(|c| o.described_of(c, k).collect())
    }

    /// Whether open vault `v` holds a wallet with `w`'s descriptor
    /// ([`crate::vault_screens::vault_has_wallet`]), from its records as
    /// the vault keeps them read. Costs a parse of each description the
    /// first time after a change.
    pub(crate) fn vault_has_wallet(&self, v: usize, w: &Wallet) -> bool {
        self.kept(|c| crate::vault_screens::vault_has_wallet(c, self.app, v, w))
    }

    /// How many words key record `r`'s seed has
    /// ([`crate::Faraday::record_word_count`]), as its open vault keeps it
    /// read. Costs a parse of the words the first time.
    pub(crate) fn record_word_count(&self, r: &Record) -> Option<usize> {
        self.kept(|c| self.app.record_word_count(c, r))
    }

    /// The files GPG's sheet signs ([`crate::Faraday::gpg_chosen`]), each
    /// file's hash kept by the file. Costs a hash of each file the first
    /// time.
    pub(crate) fn gpg_chosen(&self) -> Vec<GpgFile> {
        self.kept(|c| self.app.gpg_chosen(c))
    }

    /// File `item`'s SHA-256 ([`crate::Item::hash`]): the one taken when
    /// it came in. Costs a hash of the file when none was taken.
    pub(crate) fn item_hash(&self, item: &crate::Item) -> [u8; 32] {
        self.kept(|c| item.hash(c))
    }

    /// The session strip's stage ([`crate::Faraday::session_stage`]),
    /// from each vault file's hash taken when it came in. Costs a hash of
    /// a file whose hash was not taken.
    pub fn session_stage(&self) -> &'static str {
        self.kept(|c| self.app.session_stage(c))
    }

    /// The first vault changed since it was written
    /// ([`crate::Faraday::vault_changed`]), from each vault file's hash
    /// taken when it came in. Costs a hash of a file whose hash was not
    /// taken.
    pub fn vault_changed(&self) -> Option<String> {
        self.kept(|c| self.app.vault_changed(c))
    }

    /// Where vault file `f` stands against the last write
    /// ([`crate::Faraday::currency`]), from its hash taken when it came
    /// in. Costs a hash of a file whose hash was not taken.
    pub fn currency(&self, f: &crate::vaults::VaultFile) -> Option<crate::vaults::Currency> {
        self.kept(|c| self.app.currency(c, f))
    }

    /// What APK `name` in Files reads as ([`crate::Faraday::apk_info`]),
    /// kept by the hash taken when it came in. Costs a read of the APK
    /// the first time.
    pub fn apk_info(&self, name: &str) -> Option<faraday_apk::Info> {
        self.kept(|c| self.app.apk_info(c, name))
    }

    /// The APKs in Files of package `package`
    /// ([`crate::Faraday::apk_listed`]), each read as it is kept. Costs a
    /// read of each APK the first time.
    pub fn apk_listed(&self, package: &str) -> Vec<crate::apk::Listed> {
        self.kept(|c| self.app.apk_listed(c, package))
    }

    /// The row Files shows for APK `k` ([`crate::Faraday::apk_files_row`]),
    /// from the APK as it is kept. Costs a read of the APK the first time.
    pub fn apk_files_row(&self, k: usize) -> crate::apk::Row {
        self.kept(|c| self.app.apk_files_row(c, k))
    }

    /// The label of the flow's copy of vault `salt`'s files
    /// ([`crate::Faraday::flow_files_copy_label`]), from the hashes kept
    /// on the flow. Costs a hash of a file not hashed yet.
    pub(crate) fn flow_files_copy_label(&self, salt: &[u8; 32]) -> String {
        self.kept(|c| self.app.flow_files_copy_label(c, salt))
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
        self.kept(|c| self.app.gpg_files(c))
    }

    /// Where the seed of wallet slot `slot` is in the open vaults
    /// ([`crate::Faraday::vault_key_for`]), from each key record's kept
    /// fingerprint and each candidate's account key kept by record and
    /// path. Costs a PBKDF2 and two elliptic-curve multiplications per
    /// key record, and a PBKDF2 and a key derivation per candidate, the
    /// first time.
    pub fn vault_key_for(&self, slot: &crate::wallet::Slot) -> Option<(usize, usize)> {
        self.kept(|c| self.app.vault_key_for(c, slot))
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
        crate::glance::of_vault(self, v, r)
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
        crate::glance::of(self, w)
    }

    /// The sheet for `target` on the chart of `press`
    /// ([`crate::glance_sheet::view`]), from the memo and the values kept
    /// on the state. Costs what those cost the first time.
    pub(crate) fn glance_sheet(
        &self,
        press: crate::glance::Press,
        target: crate::glance_sheet::Target,
    ) -> Option<crate::glance_sheet::View> {
        crate::glance_sheet::view(self, press, target)
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
        crate::screens::item_title(self, n)
    }

    /// The keys the key scanner for wallet `w` takes
    /// ([`crate::Faraday::wallet_key_lines`]), from the wallet's chart.
    /// Costs what the chart costs the first time.
    pub(crate) fn wallet_key_lines(&self, w: usize) -> Vec<String> {
        self.kept(|c| self.app.wallet_key_lines(c, w))
    }

    /// What the open calculator worked out of what is typed
    /// ([`crate::Faraday::tools_reading`]), kept on its state by what it
    /// is worked out from. Costs a parse, a compile or the hashes the
    /// first time.
    pub(crate) fn tools_reading(&self) -> Option<Rc<crate::tools::Reading>> {
        self.kept(|c| self.app.tools_reading(c))
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

/// What the chart, its sheet and the checklist's titles read, as a frame
/// reads it: the memo's answers and the values kept on the state, each
/// through the accessor of the same name ([`crate::glance::Reads`]).
impl crate::glance::Reads for View<'_> {
    fn app(&self) -> &crate::Faraday {
        self.app
    }
    fn plan_shape(&self, w: usize) -> plan::Shape {
        View::plan_shape(self, w)
    }
    fn held_keys(&self) -> Vec<Fingerprint> {
        View::held_keys(self)
    }
    fn backups(&self) -> Vec<crate::backups::Entry> {
        View::backups(self)
    }
    fn passphrase_seeds(&self) -> bool {
        !View::vault_passphrase_seeds(self).is_empty()
    }
    fn vault_key_for(&self, slot: &crate::wallet::Slot) -> Option<(usize, usize)> {
        View::vault_key_for(self, slot)
    }
    fn plan_load_of(
        &self,
        wallet: &Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        self.kept(|c| self.app.plan_load_of(c, wallet, shape))
    }
    fn plan_in_vault(
        &self,
        v: usize,
        wallet: &Wallet,
        shape: &plan::Shape,
    ) -> Option<(plan::Answers, Vec<String>)> {
        self.kept(|c| self.app.plan_in_vault(c, v, wallet, shape))
    }
    fn plan_notes(&self, wallet: &Wallet, v: Option<usize>) -> Vec<plan::Note> {
        self.kept(|c| self.app.plan_notes(c, wallet, v))
    }
    fn key_fingerprint(&self, r: &Record) -> Option<String> {
        View::vault_key_fingerprint(self, r)
    }
    fn described<'o>(
        &self,
        o: &'o crate::vaults::VaultRecords,
        r: usize,
    ) -> Option<&'o crate::vaults::Described> {
        self.kept(|c| o.described(c, r))
    }
    fn plan_mapped(&self, shape: &plan::Shape, a: &plan::Answers) -> Rc<plan::Mapped> {
        View::plan_mapped(self, shape, a)
    }
    fn plan_check(&self, shape: &plan::Shape, a: &plan::Answers) -> plan::Check {
        View::plan_check(self, shape, a)
    }
    fn backup_item_done(&self, item: plan::Item) -> bool {
        View::backup_item_done(self, item)
    }
    fn backup_public_names_for(&self, wi: usize, a: &plan::Answers) -> Vec<String> {
        View::backup_public_names_for(self, wi, a)
    }
    fn backup_vault_fits(&self, v: usize) -> Option<String> {
        self.kept(|c| self.app.backup_vault_fits(c, v))
    }
    fn wallet_stick_rows(&self) -> Vec<super::StickRow> {
        View::wallet_stick_rows(self)
    }
    fn backup_items(&self) -> Vec<u8> {
        View::backup_items(self)
    }
    fn backup_seed_list(&self, w: usize) -> Vec<(String, Option<usize>)> {
        View::backup_seed_list(self, w)
    }
    fn address_shown(&self, wl: &Wallet, change: bool, index: u32) -> String {
        View::address_shown(self, wl, change, index)
    }
    fn psbt_for(&self, w: usize) -> Option<usize> {
        View::psbt_for(self, w)
    }
}
