//! Values kept on the state that drawing reads, on [`View`]: each worked
//! out the first time anyone asks, a frame included,
//! kept with what it describes, by the inputs it is worked out from, and a
//! lookup after (`docs/SPEED.md` §2 rules 2 and 4, §3.3; `docs/COMPUTE.md`
//! §4). Each says what it reads. Decisions ask the same values on the
//! state; only drawing asks them here. Each is handed the view's token
//! (`View::kept`).

use std::cell::RefCell;
use std::rc::Rc;

use faraday_bip::bip39::Mnemonic;
use faraday_bip::bitcoin::ScriptBuf;
use faraday_bip::keys::{Fingerprint, Network};
use faraday_bip::policy::WalletPolicy;
use faraday_codec::qr::QrMatrix;
use faraday_crypto::work::Compute;
use faraday_keys::vault::{RecordView, VaultId};

use super::View;
use crate::WalletKey;
use crate::create::NewKind;
use crate::explore::Reading;
use crate::glance::Glance;
use crate::gpg::GpgFile;
use crate::plan;
use crate::seeds::SeedsState;
use crate::vault::VaultWallet;
use crate::wallet::{KeyTexts, MessageCheck, NonceCheck, Session, Wallet};

/// The most QR codes of public text kept ([`Kept::public_qr`]); the next
/// one empties the list.
const PUBLIC_QRS_KEPT: usize = 8;
/// The most wallet descriptions kept read by their text
/// ([`Kept::read_kept`]); the next one drops the oldest.
const TEXTS_READ_KEPT: usize = 32;
/// The most multisig configs' names kept ([`Kept::config_name_kept`]);
/// the next one drops the oldest.
const CONFIG_NAMES_KEPT: usize = 16;
/// The most descriptions kept parsed as Create builds them
/// ([`Kept::parsed_kept`]); the next one drops the oldest.
const TEXTS_PARSED_KEPT: usize = 32;
/// The most plans kept mapped ([`Kept::plan_mapped`]): the backup's, a
/// flow's, a chart's; the next one drops the oldest.
const PLANS_MAPPED_KEPT: usize = 8;
/// The most seeds' fingerprints kept by a hash of the secret
/// ([`Kept::fingerprint_memo`]); the next one drops the oldest.
const FP_MEMO_KEPT: usize = 64;
/// The most first addresses of the Seeds tool's descriptions kept
/// ([`Kept::seeds_address`]); the next one empties the list.
const SEEDS_ADDRESSES_KEPT: usize = 16;

/// The most silent payments records kept checked against the loaded keys
/// ([`Kept::silent_check`]); the next one drops the oldest.
const SILENT_CHECKS_KEPT: usize = 8;

/// A silent payments record checked, by the record and the ids of the
/// loaded keys with its fingerprint ([`Kept::silent_check`]).
type KeptSilentCheck = (
    faraday_bip::silent_wallet::SilentWallet,
    Vec<[u8; 32]>,
    crate::silent::RecordCheck,
);

/// A description's first receive address on a network, or why there is
/// none ([`Kept::seeds_address`]).
type KeptAddress = (String, Network, Result<String, String>);

/// A signed message checked, by what its check is worked out from
/// ([`Kept::message_check`]).
struct KeptMessage {
    text: String,
    network: Network,
    wallets: Vec<String>,
    check: Option<Rc<MessageCheck>>,
}

/// What is worked out once and kept by what it is worked out from:
/// public, and never stale, since a change to what a value is worked out
/// from is another key (`docs/SPEED.md` §2). Each list is written through
/// a shared reference, so `Faraday` holds it and drawing reads it. Its
/// fields are private; the lookups below are the only way in.
#[derive(Default)]
pub(crate) struct Kept {
    /// QR codes of public text a screen shows (a guide's address), each
    /// made once, by its text.
    public_qrs: RefCell<Vec<(String, Option<Rc<QrMatrix>>)>>,
    /// Wallet descriptions read that no file or record keeps read, each
    /// by its text.
    texts_read: RefCell<Vec<(String, Option<WalletPolicy>)>>,
    /// The `Name:` line of each multisig config put together from split
    /// sheets, by its text.
    config_names: RefCell<Vec<(String, Option<String>)>>,
    /// Descriptions the Seeds tool puts together, each parsed as Create
    /// builds it, by its text.
    texts_parsed: RefCell<Vec<(String, Result<WalletPolicy, String>)>>,
    /// The signed message last checked, by its text, the network and the
    /// loaded wallets' checksums.
    message_checked: RefCell<Option<KeptMessage>>,
    /// Plans mapped, each by the shape and answers it is the map of.
    plans_mapped: RefCell<Vec<(plan::Shape, plan::Answers, Rc<plan::Mapped>)>>,
    /// First receive addresses of descriptions Seeds puts together, by
    /// the description and network.
    seeds_addresses: RefCell<Vec<KeptAddress>>,
    /// Seeds' fingerprints with no passphrase, by a hash of the secret:
    /// the Inbox's wallets and seeds are put together on every frame.
    fp_memo: RefCell<Vec<([u8; 32], Fingerprint)>>,
    /// Silent payments records checked against the loaded keys, each by
    /// the record and the ids of the loaded keys with its fingerprint.
    silent_checks: RefCell<Vec<KeptSilentCheck>>,
}

impl Kept {
    /// Each kept list with its name, its length and its bound.
    #[cfg(test)]
    pub(crate) fn sizes(&self) -> Vec<(&'static str, usize, usize)> {
        vec![
            (
                "public QR codes (public_qr)",
                self.public_qrs.borrow().len(),
                PUBLIC_QRS_KEPT,
            ),
            (
                "descriptions read (read_kept)",
                self.texts_read.borrow().len(),
                TEXTS_READ_KEPT,
            ),
            (
                "configs' names (config_name_kept)",
                self.config_names.borrow().len(),
                CONFIG_NAMES_KEPT,
            ),
            (
                "descriptions parsed (parsed_kept)",
                self.texts_parsed.borrow().len(),
                TEXTS_PARSED_KEPT,
            ),
            (
                "plans mapped (plan_mapped)",
                self.plans_mapped.borrow().len(),
                PLANS_MAPPED_KEPT,
            ),
            (
                "Seeds' addresses (seeds_address)",
                self.seeds_addresses.borrow().len(),
                SEEDS_ADDRESSES_KEPT,
            ),
            (
                "seeds' fingerprints (fp_memo)",
                self.fp_memo.borrow().len(),
                FP_MEMO_KEPT,
            ),
            (
                "silent payments records checked (silent_check)",
                self.silent_checks.borrow().len(),
                SILENT_CHECKS_KEPT,
            ),
        ]
    }

    /// Costs a QR encoding the first time. The QR code of `text`, public
    /// text a screen shows (a guide's address): made the first time it is
    /// asked, by whoever asks, a frame included, and kept by its text, a
    /// lookup after (`docs/SPEED.md` §2 rule 1: what is kept is public).
    pub(crate) fn public_qr(&self, c: &Compute, text: &str) -> Option<Rc<QrMatrix>> {
        if let Some((_, m)) = self.public_qrs.borrow().iter().find(|(t, _)| t == text) {
            return m.clone();
        }
        let m = faraday_codec::qr::encode(
            c,
            faraday_codec::qr::Payload::Bytes(text.as_bytes()),
            faraday_codec::qr::Ecc::Medium,
        )
        .ok()
        .map(Rc::new);
        let mut kept = self.public_qrs.borrow_mut();
        if kept.len() >= PUBLIC_QRS_KEPT {
            kept.clear();
        }
        kept.push((text.to_string(), m.clone()));
        m
    }

    /// Costs a parse the first time. Wallet description `text` read
    /// ([`crate::wallet::read_wallet`]): the first time it is asked, by
    /// whoever asks, a frame included, and kept by its text, a lookup
    /// after. For a description no Inbox file or vault record keeps read:
    /// split sheets put together.
    pub(crate) fn read_kept(&self, c: &Compute, text: &str) -> Option<WalletPolicy> {
        if let Some((_, p)) = self.texts_read.borrow().iter().find(|(t, _)| t == text) {
            return p.clone();
        }
        let p = crate::wallet::read_wallet(c, text).ok();
        let mut kept = self.texts_read.borrow_mut();
        if kept.len() >= TEXTS_READ_KEPT {
            kept.remove(0);
        }
        kept.push((text.to_string(), p.clone()));
        p
    }

    /// Costs a parse of the config the first time: the name multisig
    /// config `text` carries on its `Name:` line
    /// (`faraday_bip::multisig_config::parse_named`), kept by its text, a
    /// lookup after.
    pub(crate) fn config_name_kept(&self, c: &Compute, text: &str) -> Option<String> {
        if let Some((_, n)) = self.config_names.borrow().iter().find(|(t, _)| t == text) {
            return n.clone();
        }
        let n = faraday_bip::multisig_config::parse_named(c, text)
            .ok()
            .and_then(|(_, n)| n);
        let mut kept = self.config_names.borrow_mut();
        if kept.len() >= CONFIG_NAMES_KEPT {
            kept.remove(0);
        }
        kept.push((text.to_string(), n.clone()));
        n
    }

    /// Costs a parse of the description the first time: description
    /// `text` parsed as Create builds a wallet (`WalletPolicy::parse_any`),
    /// with the parser's refusal, kept by its text, a lookup after.
    pub(crate) fn parsed_kept(&self, c: &Compute, text: &str) -> Result<WalletPolicy, String> {
        if let Some((_, p)) = self.texts_parsed.borrow().iter().find(|(t, _)| t == text) {
            return p.clone();
        }
        let p =
            WalletPolicy::parse_any(c, text).map_err(|e| faraday_strings::bip::policy_error(&e));
        let mut kept = self.texts_parsed.borrow_mut();
        if kept.len() >= TEXTS_PARSED_KEPT {
            let _ = kept.remove(0);
        }
        kept.push((text.to_string(), p.clone()));
        p
    }

    /// Costs an address derivation per loaded wallet and address tried,
    /// up to its first hundred on each chain, and a signature check, the
    /// first time: signed message `text` checked
    /// ([`crate::wallet::Session::check_message`]) against `session`,
    /// kept by its text, the network and the loaded wallets, a lookup
    /// after.
    pub(crate) fn message_check(
        &self,
        c: &Compute,
        text: &str,
        session: &Session,
    ) -> Option<Rc<MessageCheck>> {
        let network = session.network();
        let sums: Vec<String> = session
            .wallets
            .iter()
            .map(|w| w.checksum().to_string())
            .collect();
        if let Some(k) = self.message_checked.borrow().as_ref()
            && k.text == text
            && k.network == network
            && k.wallets == sums
        {
            return k.check.clone();
        }
        let check = session.check_message(c, text).map(Rc::new);
        *self.message_checked.borrow_mut() = Some(KeptMessage {
            text: text.to_string(),
            network,
            wallets: sums,
            check: check.clone(),
        });
        check
    }

    /// Costs a map of the plan and its check the first time. The plan of
    /// `shape` under answers `a`, mapped with its check, for every helper
    /// that reads the plan's map (`docs/SPEED.md` §3.3): mapped the first
    /// time these shape and answers are asked, by whoever asks, a frame
    /// included, and kept by them, a lookup after. A frame and a press
    /// read the same map, and it is the map of the state as it is: a
    /// change to the answers or the shape is another key.
    pub(crate) fn plan_mapped(
        &self,
        c: &Compute,
        shape: &plan::Shape,
        a: &plan::Answers,
    ) -> Rc<plan::Mapped> {
        if let Some((_, _, m)) = self
            .plans_mapped
            .borrow()
            .iter()
            .find(|(s, x, _)| s == shape && x == a)
        {
            return m.clone();
        }
        let m = Rc::new({
            let m = plan::Mapped::of(c, shape, a);
            m.check(c, shape, a);
            m
        });
        let mut kept = self.plans_mapped.borrow_mut();
        // The plans a session shows at once: the backup's, a flow's, a
        // chart's; an answer changed makes another, and the oldest goes.
        if kept.len() >= PLANS_MAPPED_KEPT {
            kept.remove(0);
        }
        kept.push((shape.clone(), a.clone(), m.clone()));
        m
    }

    /// Costs a parse of the description and an address the first time.
    /// The first receive address of the wallet description `d` makes on
    /// `net`: worked out the first time it is asked for this description,
    /// by whoever asks, a frame included, and kept by the description and
    /// network it comes from, a lookup after (public).
    pub(crate) fn seeds_address(
        &self,
        c: &Compute,
        d: &str,
        net: Network,
    ) -> Result<String, String> {
        if let Some((.., a)) = self
            .seeds_addresses
            .borrow()
            .iter()
            .find(|(x, n, _)| x == d && *n == net)
        {
            return a.clone();
        }
        let a = crate::wallet::read_wallet(c, d)
            .map_err(|e| e.text())
            .and_then(|p| {
                p.address_at(c, net, false, 0)
                    .map(|a| a.to_string())
                    .map_err(|e| faraday_strings::bip::policy_error(&e))
            });
        let mut kept = self.seeds_addresses.borrow_mut();
        if kept.len() >= SEEDS_ADDRESSES_KEPT {
            kept.clear();
        }
        kept.push((d.to_string(), net, a.clone()));
        a
    }

    /// Costs two key derivations per loaded key with the record's
    /// fingerprint the first time. Silent payments record `w` checked
    /// against the loaded keys ([`crate::silent::check_record`]): kept by
    /// the record and the ids of the loaded keys with its fingerprint,
    /// which are all the check reads, a lookup after.
    pub(crate) fn silent_check(
        &self,
        c: &Compute,
        keys: &faraday_keys::Keys,
        w: &faraday_bip::silent_wallet::SilentWallet,
    ) -> crate::silent::RecordCheck {
        let ids = crate::silent::ids_of(keys, w.fingerprint.0);
        if let Some((.., check)) = self
            .silent_checks
            .borrow()
            .iter()
            .find(|(r, i, _)| r == w && *i == ids)
        {
            return *check;
        }
        let check = crate::silent::check_record(c, keys, w).0;
        let mut kept = self.silent_checks.borrow_mut();
        if kept.len() >= SILENT_CHECKS_KEPT {
            kept.remove(0);
        }
        kept.push((w.clone(), ids, check));
        check
    }

    /// The fingerprint of a seed with no passphrase, remembered by a hash
    /// of the secret so a frame does not run PBKDF2 again. Costs what `f`
    /// costs, the first time.
    pub(crate) fn fingerprint_memo(
        &self,
        tag: &[u8],
        secret: &[u8],
        f: impl FnOnce() -> Option<Fingerprint>,
    ) -> Option<Fingerprint> {
        let id = faraday_keys::identity::secret_id(tag, secret);
        if let Some((_, fp)) = self.fp_memo.borrow().iter().find(|(k, _)| *k == id) {
            return Some(*fp);
        }
        let fp = f()?;
        let mut kept = self.fp_memo.borrow_mut();
        if kept.len() >= FP_MEMO_KEPT {
            kept.remove(0);
        }
        kept.push((id, fp));
        Some(fp)
    }
}

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

    /// Key `i`'s account key text for `kind` and `account`
    /// ([`crate::wallet::KeyTexts::account_text`]), kept on the key. Costs
    /// a key derivation the first time.
    pub fn account_text(&self, i: usize, kind: NewKind, account: u32) -> Result<String, String> {
        self.kept(|c| self.session.keys.account_text(c, i, kind, account))
    }

    /// Key `i`'s key text at `path` ([`crate::wallet::KeyTexts::text_at`]),
    /// kept on the key. Costs a key derivation the first time.
    pub fn text_at(&self, i: usize, path: &str) -> Result<String, String> {
        self.kept(|c| self.session.keys.text_at(c, i, path))
    }

    /// What tells key `i`'s words apart ([`faraday_keys::Keys::words_id`]),
    /// kept on the key. Costs a parse of the words the first time.
    pub fn words_id(&self, i: usize) -> Option<&[u8; 32]> {
        self.kept(|c| self.session.keys.words_id(c, i))
    }

    /// The key the words typed on Load a key make, for its status line
    /// (`EntryState::probed`), kept until an event forgets it; `None`
    /// while typing goes on, before it is worked out
    /// ([`crate::Faraday::typing_now`]). Costs a PBKDF2 and an
    /// elliptic-curve multiplication the first time after typing pauses.
    pub fn entry_probed(&self, m: &Mnemonic) -> Option<Result<Fingerprint, String>> {
        if self.typing_now() {
            return self.entry.state().probe_kept();
        }
        Some(self.kept(|c| self.entry.state().probed(c, m)))
    }

    /// Explore's reading ([`crate::Faraday::explore_reading`]), kept by
    /// what it is read from. Costs a key derivation and addresses the
    /// first time.
    pub fn explore_reading(&self) -> Option<Rc<Reading>> {
        self.kept(|c| self.app.explore_reading(c))
    }

    /// Explore's address list as it stands
    /// ([`crate::Faraday::explore_list`]), kept by the account it is of.
    /// Costs a key derivation and an address derivation per address the
    /// first time.
    pub fn explore_list(&self) -> Vec<(u32, String)> {
        self.kept(|c| self.app.explore_list(c))
    }

    /// The QR code of public text `text` ([`Kept::public_qr`]),
    /// kept by its text. Costs a QR encoding the first time.
    pub fn public_qr(&self, text: &str) -> Option<Rc<QrMatrix>> {
        self.kept(|c| self.app.kept.public_qr(c, text))
    }

    /// Wallet description `text` read ([`Kept::read_kept`]),
    /// kept by its text. Costs a parse the first time.
    pub fn read_kept(&self, text: &str) -> Option<WalletPolicy> {
        self.kept(|c| self.app.kept.read_kept(c, text))
    }

    /// A plan's map and its check ([`Kept::plan_mapped`]), kept
    /// by the shape and answers. Costs a map and a check the first time.
    pub fn plan_mapped(&self, shape: &plan::Shape, a: &plan::Answers) -> Rc<plan::Mapped> {
        self.kept(|c| self.app.kept.plan_mapped(c, shape, a))
    }

    /// A plan's check ([`plan::Mapped::check`]) on its kept map
    /// ([`Kept::plan_mapped`]), kept with it by the shape and
    /// answers. Costs a map and a check the first time.
    pub(crate) fn plan_check(&self, shape: &plan::Shape, a: &plan::Answers) -> plan::Check {
        self.kept(|c| self.app.kept.plan_mapped(c, shape, a).check(c, shape, a))
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
        key: &faraday_keys::KeyInfo,
    ) -> plan::CopyChecks {
        self.copy_forms(b, i).checks(
            !self.cameras().is_empty(),
            key.language == faraday_bip::bip39::Language::English,
        )
    }

    /// The cards of the backup's plan, in order
    /// ([`crate::Faraday::backup_questions`]), from the memo's shape and
    /// the kept map. Costs a map the first time.
    pub(crate) fn backup_questions(&self) -> Vec<u8> {
        let Some(b) = self.backup.state() else {
            return Vec::new();
        };
        let shape = self.plan_shape(b.wallet);
        crate::plan_cards_on(&self.plan_mapped(&shape, &b.answers))
    }

    /// Signed message `text` checked ([`Kept::message_check`]),
    /// kept by its text, the network and the loaded wallets. Costs an
    /// address derivation per loaded wallet and address tried and a
    /// signature check the first time.
    pub(crate) fn message_check(&self, text: &str) -> Option<Rc<crate::wallet::MessageCheck>> {
        self.kept(|c| self.app.kept.message_check(c, text, &self.app.session))
    }

    /// The wallet description record `r` holds
    /// ([`crate::Faraday::record_described`]), as its open vault keeps it
    /// read, or as a description no record keeps, kept by its text.
    /// Costs a parse the first time.
    pub(crate) fn record_described(
        &self,
        r: RecordView<'_>,
    ) -> Option<std::borrow::Cow<'_, crate::vault::Described>> {
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

    /// Each record of kind `k` of open vault `id` with the wallet
    /// description it holds ([`crate::vault::Vaults::described_of`]), as
    /// the vault keeps them read. Costs a parse of each description the
    /// first time after a change.
    pub(crate) fn described_of(
        &self,
        id: VaultId,
        k: u8,
    ) -> Vec<(usize, RecordView<'_>, &crate::vault::Described)> {
        self.kept(|c| self.app.vaults.described_of(c, id, k).collect())
    }

    /// Whether open vault `v` holds a wallet with `w`'s descriptor
    /// ([`crate::vault::screen::vault_has_wallet`]), from its records as
    /// the vault keeps them read. Costs a parse of each description the
    /// first time after a change.
    pub(crate) fn vault_has_wallet(&self, v: usize, w: &Wallet) -> bool {
        self.kept(|c| crate::vault::screen::vault_has_wallet(c, self.app, v, w))
    }

    /// How many words key record `r`'s seed has
    /// ([`crate::Faraday::record_word_count`]), as its open vault keeps it
    /// read. Costs a parse of the words the first time.
    pub(crate) fn record_word_count(&self, r: RecordView<'_>) -> Option<usize> {
        self.kept(|c| self.app.vaults.record_word_count(c, r))
    }

    /// The files GPG's sheet signs ([`crate::Faraday::gpg_chosen`]), each
    /// file's hash kept by the file. Costs a hash of each file the first
    /// time.
    pub(crate) fn gpg_chosen(&self) -> Vec<GpgFile> {
        self.kept(|c| self.app.gpg_chosen(c))
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
    pub fn currency(&self, f: &crate::vault::VaultFile) -> Option<crate::vault::Currency> {
        self.kept(|c| self.app.currency(c, f))
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
    pub fn wallet_key(&self, i: usize, slot: u8) -> Result<WalletKey, String> {
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
    /// ([`crate::vault::screen::vault_has_key`]), from each key record's
    /// kept fingerprint and id. Costs a PBKDF2 and two elliptic-curve
    /// multiplications per key record the first time.
    pub(crate) fn vault_has_key(&self, v: usize, key: &faraday_keys::KeyInfo) -> bool {
        self.kept(|c| crate::vault::screen::vault_has_key(c, self.app, v, key))
    }

    /// Where the wallet being backed up and its keys are kept
    /// ([`crate::Faraday::backup_kept`]), from each key record's kept
    /// fingerprint and id. Costs a PBKDF2 and two elliptic-curve
    /// multiplications per key record the first time.
    pub fn backup_kept(&self) -> Option<crate::wallet_backup::Kept> {
        self.kept(|c| self.app.backup_kept(c))
    }

    /// Loaded wallet `w` at a glance ([`crate::glance::of`]), from the
    /// memo and the values kept on the state. Costs what those cost the
    /// first time.
    pub(crate) fn glance_of(&self, w: usize) -> Option<Glance> {
        crate::glance::of(self, w)
    }

    /// The sheet for `target` on the chart of `press`
    /// ([`crate::glance::sheet::view`]), from the memo and the values kept
    /// on the state. Costs what those cost the first time.
    pub(crate) fn glance_sheet(
        &self,
        press: crate::glance::Press,
        target: crate::glance::sheet::Target,
    ) -> Option<crate::glance::sheet::View> {
        crate::glance::sheet::view(self, press, target)
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

    /// How session wallet `i`'s silent payments record stands against the
    /// loaded keys ([`crate::Faraday::silent_wallet_check`]), kept by the
    /// record and the keys; `None` for any other wallet. Costs two key
    /// derivations the first time.
    pub(crate) fn silent_wallet_check(&self, i: usize) -> Option<crate::silent::RecordCheck> {
        self.kept(|c| self.app.silent_wallet_check(c, i))
    }

    /// Whether session wallet `i` is a silent payments wallet whose record
    /// a loaded key with its fingerprint contradicts
    /// ([`View::silent_wallet_check`]). Costs two key derivations the
    /// first time.
    pub(crate) fn silent_differs(&self, i: usize) -> bool {
        self.silent_wallet_check(i) == Some(crate::silent::RecordCheck::Differs)
    }

    /// How far the keys here go to sign for session wallet `i`, as
    /// [`crate::screens::sign_readiness`] says, with none for a silent
    /// payments wallet whose record its loaded key contradicts
    /// ([`View::silent_differs`]). Costs what that costs the first time.
    pub(crate) fn readiness(&self, i: usize) -> u8 {
        if self.silent_differs(i) {
            return 0;
        }
        self.session
            .wallets
            .get(i)
            .map_or(0, |w| crate::screens::sign_readiness(self, w))
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
    pub(crate) fn nonce_check(&self, psbt: &faraday_psbt::Psbt) -> Option<Rc<NonceCheck>> {
        let s = self.wallets.spend()?;
        let bytes = psbt.to_bytes();
        let mut cache = s.nonce_check.borrow_mut();
        if let Some((b, check)) = cache.as_ref()
            && *b == bytes
        {
            return Some(check.clone());
        }
        let check = Rc::new(self.kept(|c| self.session.nonce_check(c, psbt)));
        *cache = Some((bytes, check.clone()));
        Some(check)
    }

    /// A vault key record's fingerprint
    /// (`vault::screen::key_fingerprint`), kept by the key's id. Costs a
    /// PBKDF2 and two elliptic-curve multiplications the first time.
    pub fn vault_key_fingerprint(&self, r: RecordView<'_>) -> Option<String> {
        self.kept(|c| crate::vault::screen::key_fingerprint(c, self.app, r))
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
    fn key_fingerprint(&self, r: RecordView<'_>) -> Option<String> {
        View::vault_key_fingerprint(self, r)
    }
    fn described(&self, id: VaultId, r: usize) -> Option<&crate::vault::Described> {
        self.kept(|c| self.app.vaults.described(c, id, r))
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
