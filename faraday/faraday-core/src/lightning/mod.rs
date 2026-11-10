//! A Lightning node's key: from a
//! loaded key, read the way ldk-node reads a BIP-39 seed, or from an LND
//! cipher seed (aezeed) typed here. The arithmetic is `faraday-bip::aezeed`'s,
//! as the Lightning tool uses it. The node's public key is
//! public; its private key is a secret, into the vault or out only
//! through the secret sheet.

pub(crate) mod screen;

use crate::strings::fill;
use crate::strings::{EN, fill_secret};
use faraday_bip::bip39::Language;
use faraday_vault::Record;
use faraday_vault::records::{field, kind};
use zeroize::Zeroizing;

use crate::{Faraday, Screen};
use faraday_crypto::work::Compute;
use faraday_keys::Keys;
use faraday_keys::hsm_node::NodeRefusal;

/// Where the node key comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeSource {
    /// A loaded key, by fingerprint, read as ldk-node reads its seed.
    Key([u8; 4]),
    /// A loaded key, by fingerprint, read as Core Lightning's
    /// `lightning-hsmtool generatehsm` reads its words.
    KeyCln([u8; 4]),
    /// An LND cipher seed typed here.
    Aezeed,
    /// Core Lightning's node: an `hsm_secret` file from Files, or English
    /// BIP-39 words typed here.
    CoreLightning,
}

/// The node key, once found.
pub use faraday_keys::lightning::Node;

/// A read to run on the worker: faraday-keys' job, holding the aezeed's
/// words and passphrase, or an `hsm_secret` file's bytes and passphrase.
pub(crate) enum ReadWork {
    /// An aezeed.
    Aezeed(faraday_keys::lightning::AezeedJob),
    /// Core Lightning's `hsm_secret`.
    Hsm(faraday_keys::hsm_secret::HsmJob),
}

/// A read's answer: the node key or why there is none, inside
/// faraday-keys' type.
pub(crate) enum ReadDone {
    /// An aezeed's.
    Aezeed(faraday_keys::lightning::AezeedRead),
    /// An `hsm_secret`'s.
    Hsm(faraday_keys::hsm_secret::HsmRead),
}

impl ReadWork {
    /// Costs an aezeed's scrypt or an `hsm_secret`'s Argon2id, and a key
    /// derivation.
    pub(crate) fn run(self, c: &Compute) -> ReadDone {
        match self {
            ReadWork::Aezeed(job) => ReadDone::Aezeed(job.run(c)),
            ReadWork::Hsm(job) => ReadDone::Hsm(job.run(c)),
        }
    }
}

/// The line for a refusal of Core Lightning's file or words.
fn refusal_line(r: NodeRefusal) -> String {
    use NodeRefusal as N;
    use faraday_keys::hsm_node::WordsRefusal as W;
    use faraday_keys::hsm_secret::Refusal as R;
    match r {
        N::File(R::Words) => EN.lightning_hsm_words.to_string(),
        N::File(R::Passphrase) => EN.lightning_hsm_passphrase.to_string(),
        N::File(R::PassphraseNotAscii) => crate::state::ascii_refused(),
        N::File(R::PassphraseTooLong) => EN.lightning_hsm_passphrase_long.to_string(),
        // A file changed since it was listed reads as a changed file.
        N::File(R::Length | R::PassphraseNotNeeded | R::Opening) => {
            EN.lightning_hsm_opening.to_string()
        }
        N::File(R::Memory) => EN.lightning_hsm_memory.to_string(),
        N::Words(W::NotEnglish) => EN.lightning_hsm_not_english.to_string(),
        N::Words(W::Words(line)) | N::Demo(line) => line,
    }
}

/// What the Lightning screen holds.
pub struct LightningState {
    /// The source chosen.
    pub source: Option<NodeSource>,
    /// The aezeed's words, as typed.
    pub typed: crate::secret_text::SecretText,
    /// The aezeed's passphrase; empty is LND's default.
    pub passphrase: crate::secret_text::SecretText,
    /// Typing goes to the passphrase.
    pub on_passphrase: bool,
    /// The node key, once found.
    pub node: Option<Node>,
    /// Why it was not.
    pub error: Option<String>,
    /// The private key is on screen.
    pub shown: bool,
    /// Where it returns to.
    pub back: Screen,
    /// Use a sample on the aezeed's words (`docs/MODES.md` §6).
    pub(crate) sample: crate::samples::put::Use,
    /// Core Lightning's `hsm_secret` chosen in Files: its name there.
    /// Only the name is kept here; the bytes stay in Files until a read
    /// looks the file up by name and moves a copy into its job.
    pub(crate) file: Option<String>,
    /// The chosen file asks for a passphrase.
    pub(crate) file_asks: bool,
    /// Opened from Convert: only the entry shows, and what it reads is
    /// held there as a seed, not shown as a node key.
    pub for_convert: bool,
}

/// The Lightning screen's area: its state while it is open.
#[derive(Default)]
pub(crate) struct LightningArea {
    state: Option<LightningState>,
}

impl LightningState {
    /// The file chosen in Files asks for a passphrase.
    pub fn file_asks_passphrase(&self) -> bool {
        self.file_asks
    }
}

impl LightningArea {
    /// The screen's state, while it is open.
    pub(crate) fn state(&self) -> Option<&LightningState> {
        self.state.as_ref()
    }

    /// Opens the screen, to return to `back`.
    pub(crate) fn open(&mut self, back: Screen) {
        self.state = Some(LightningState {
            source: None,
            typed: crate::secret_text::SecretText::new(),
            passphrase: crate::secret_text::SecretText::new(),
            on_passphrase: false,
            node: None,
            error: None,
            shown: false,
            back,
            sample: Default::default(),
            file: None,
            file_asks: false,
            for_convert: false,
        });
    }

    /// Opens the aezeed's entry for Convert, to return to it.
    pub(crate) fn open_for_convert(&mut self) {
        self.open(Screen::Convert);
        if let Some(l) = self.state.as_mut() {
            l.source = Some(NodeSource::Aezeed);
            l.for_convert = true;
        }
    }

    /// Whether the screen is open for Convert.
    pub(crate) fn for_convert(&self) -> bool {
        self.state.as_ref().is_some_and(|l| l.for_convert)
    }

    /// Forgets the screen's state: the words, the passphrase and the key.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// Costs a key derivation. Finds the node key of the loaded key chosen.
    pub(crate) fn resolve(&mut self, c: &Compute, keys: &Keys) {
        let Some(l) = self.state.as_ref() else {
            return;
        };
        let found: Result<Node, String> = match l.source {
            Some(NodeSource::Key(fp)) => {
                match keys.position_of(fp).and_then(|k| keys.lightning_node(c, k)) {
                    None => Err(EN.lightning_this_key.to_string()),
                    Some(node) => node,
                }
            }
            Some(NodeSource::KeyCln(fp)) => {
                match keys.position_of(fp).and_then(|k| keys.hsm_key_node(c, k)) {
                    None => Err(EN.lightning_this_key.to_string()),
                    Some(node) => node.map_err(|e| refusal_line(NodeRefusal::Words(e))),
                }
            }
            // A cipher seed and an hsm_secret are read on the worker
            // ([`LightningArea::read_job`]).
            Some(NodeSource::Aezeed | NodeSource::CoreLightning) | None => return,
        };
        if let Some(l) = self.state.as_mut() {
            match found {
                Ok(n) => {
                    l.node = Some(n);
                    l.error = None;
                }
                Err(e) => l.error = Some(e),
            }
        }
    }

    /// The typed words checked and, when they are an aezeed's twenty-four,
    /// the read to run on the worker: the words as indices and a copy of
    /// the passphrase, both wiped with the job. Says why not otherwise.
    pub(crate) fn read_job(&mut self, keys: &Keys) -> Option<ReadWork> {
        let l = self.state.as_mut()?;
        if l.source != Some(NodeSource::Aezeed) {
            return None;
        }
        let mut idx = [0u16; faraday_keys::lightning::AEZEED_WORDS];
        let words: Vec<&str> = l.typed.split_whitespace().collect();
        let refused = if words.len() != faraday_keys::lightning::AEZEED_WORDS {
            Some(fill(EN.lightning_an_aezeed, &[&words.len().to_string()]))
        } else if let Some(k) = words
            .iter()
            .position(|w| Language::English.index_of(w).is_none())
        {
            Some(fill(EN.lightning_word_not_in_list, &[&(k + 1).to_string()]))
        } else {
            for (slot, w) in idx.iter_mut().zip(&words) {
                *slot = Language::English.index_of(w).unwrap_or(0);
            }
            None
        };
        if refused.is_some() {
            l.error = refused;
            return None;
        }
        let pass = crate::secret_text::SecretText::of(&l.passphrase);
        let job = if l.for_convert {
            keys.aezeed_seed_job(&idx, pass)
        } else {
            keys.aezeed_job(&idx, pass)
        };
        faraday_keys::lightning::wipe_words(&mut idx);
        Some(ReadWork::Aezeed(job))
    }

    /// Core Lightning's file chosen in Files by `name`, `asks` for whether
    /// it wants a passphrase. The typed words, the passphrase and the node
    /// are dropped; typing goes to the passphrase where the file asks.
    pub(crate) fn choose_file(&mut self, name: String, asks: bool) {
        if let Some(l) = self.state.as_mut() {
            l.source = Some(NodeSource::CoreLightning);
            l.file = Some(name);
            l.file_asks = asks;
            l.on_passphrase = asks;
            l.typed.set("");
            l.passphrase.set("");
            l.node = None;
            l.error = None;
        }
    }

    /// The read of the chosen file, its `bytes` moved in: the passphrase
    /// is the field's when the file asks, else empty. An empty one is
    /// taken when the file asks.
    pub(crate) fn hsm_job(&self, keys: &Keys, bytes: Zeroizing<Vec<u8>>) -> Option<ReadWork> {
        let l = self.state.as_ref()?;
        let pass = if faraday_keys::hsm_secret::needs_passphrase(&bytes) {
            crate::secret_text::SecretText::of(&l.passphrase)
        } else {
            crate::secret_text::SecretText::new()
        };
        Some(ReadWork::Hsm(keys.hsm_job(bytes, pass)))
    }

    /// Costs a PBKDF2 and a key derivation. Core Lightning's node from the
    /// typed English BIP-39 words and the passphrase.
    pub(crate) fn read_words(&mut self, c: &Compute, keys: &Keys) {
        let Some(l) = self.state.as_mut() else {
            return;
        };
        let words = Zeroizing::new(l.typed.split_whitespace().collect::<Vec<_>>().join(" "));
        match keys.hsm_words_node(c, &words, &l.passphrase) {
            Ok(n) => {
                l.node = Some(n);
                l.error = None;
            }
            Err(e) => l.error = Some(refusal_line(e)),
        }
    }

    /// An aezeed read's answer, applied on the main thread.
    pub(crate) fn read_done(&mut self, keys: &Keys, done: ReadDone) {
        let found = match done {
            ReadDone::Aezeed(read) => keys
                .aezeed_node(read)
                .map_err(|e| fill(EN.lightning_not_an_aezeed, &[&e])),
            ReadDone::Hsm(read) => keys.hsm_node(read).map_err(refusal_line),
        };
        if let Some(l) = self.state.as_mut() {
            match found {
                Ok(n) => {
                    l.node = Some(n);
                    l.error = None;
                }
                Err(e) => l.error = Some(e),
            }
        }
    }

    /// An aezeed read's answer for Convert: held in `keys` as the tool's
    /// source, and the entry's words and passphrase dropped with the
    /// screen's state. Says why not, on the entry, otherwise.
    pub(crate) fn read_held(
        &mut self,
        keys: &mut Keys,
        done: ReadDone,
    ) -> Option<faraday_bip::keys::Fingerprint> {
        let ReadDone::Aezeed(read) = done else {
            return None;
        };
        match keys.aezeed_hold(read) {
            Ok(fp) => {
                self.state = None;
                Some(fp)
            }
            Err(e) => {
                if let Some(l) = self.state.as_mut() {
                    l.error = Some(e);
                }
                None
            }
        }
    }

    /// The node id and the private key, for `why`.
    pub(crate) fn secret_text(
        &self,
        why: faraday_keys::Reveal,
    ) -> Option<(String, Zeroizing<String>)> {
        let n = self.state.as_ref()?.node.as_ref()?;
        let id: String = faraday_crypto::hex::encode_string(n.public());
        Some((id, n.private_hex(why)?))
    }

    /// Costs a key derivation when a loaded key is chosen. A press that
    /// changes only this screen: a source chosen, the passphrase field, the
    /// private key shown.
    pub(crate) fn act(&mut self, c: &Compute, keys: &Keys, action: crate::Action) {
        use crate::Action as A;
        let Some(l) = self.state.as_mut() else {
            return;
        };
        match action {
            A::LKey(fp) => {
                l.source = Some(NodeSource::Key(fp));
                l.file = None;
                l.file_asks = false;
                l.on_passphrase = false;
                l.node = None;
                l.error = None;
                self.resolve(c, keys);
            }
            A::LCln(Some(fp)) => {
                l.source = Some(NodeSource::KeyCln(fp));
                l.file = None;
                l.file_asks = false;
                l.on_passphrase = false;
                l.node = None;
                l.error = None;
                self.resolve(c, keys);
            }
            A::LAezeed => {
                // What was typed for another source or file is not this one's.
                if l.file.is_some() || l.source != Some(NodeSource::Aezeed) {
                    l.typed.set("");
                    l.passphrase.set("");
                }
                l.source = Some(NodeSource::Aezeed);
                l.file = None;
                l.file_asks = false;
                l.on_passphrase = false;
                l.node = None;
                l.error = None;
            }
            A::LCln(None) => {
                // What was typed for another source or file is not this one's.
                if l.file.is_some() || l.source != Some(NodeSource::CoreLightning) {
                    l.typed.set("");
                    l.passphrase.set("");
                }
                l.source = Some(NodeSource::CoreLightning);
                l.file = None;
                l.file_asks = false;
                l.on_passphrase = false;
                l.node = None;
                l.error = None;
            }
            // A file's passphrase is the only field there is to press.
            A::LPassphrase => l.on_passphrase = l.file.is_some() || !l.on_passphrase,
            A::LShow => l.shown = !l.shown,
            _ => {}
        }
    }

    /// Whether the screen reads a typed aezeed or a chosen `hsm_secret`
    /// file, whose Enter stretches it with scrypt or Argon2id.
    pub(crate) fn reads_aezeed(&self, no_sheet: bool) -> bool {
        no_sheet
            && self.state.as_ref().is_some_and(|l| {
                l.source == Some(NodeSource::Aezeed)
                    || (l.source == Some(NodeSource::CoreLightning) && l.file.is_some())
            })
    }

    /// Whether the screen is on Core Lightning's source.
    pub(crate) fn on_core_lightning(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|l| l.source == Some(NodeSource::CoreLightning))
    }

    /// The name of the file chosen in Files.
    pub(crate) fn chosen_file(&self) -> Option<String> {
        self.state.as_ref()?.file.clone()
    }

    /// Whether the file chosen asks for a passphrase, as last looked at.
    pub(crate) fn file_asks(&self) -> bool {
        self.state.as_ref().is_some_and(|l| l.file_asks)
    }

    /// The file chosen is no longer in Files: the choice is dropped and
    /// the screen says so.
    pub(crate) fn file_gone(&mut self) {
        if let Some(l) = self.state.as_mut() {
            let name = l.file.take().unwrap_or_default();
            l.file_asks = false;
            l.on_passphrase = false;
            l.node = None;
            l.error = Some(fill(EN.lightning_hsm_file_gone, &[&name]));
        }
    }

    /// The file chosen changed in Files and now asks, or does not ask, for
    /// a passphrase: the screen follows it and nothing is read.
    pub(crate) fn file_changed(&mut self, asks: bool) {
        if let Some(l) = self.state.as_mut() {
            l.file_asks = asks;
            l.on_passphrase = asks;
            l.node = None;
            l.error = Some(EN.lightning_hsm_opening.to_string());
        }
    }

    /// Typing the aezeed and its passphrase, on `screen`. Returns whether
    /// the key was taken. Escape leaves the screen for where it came from.
    pub(crate) fn key(&mut self, key: faraday_shell_api::Key, screen: &mut Screen) -> bool {
        use faraday_shell_api::Key as K;
        if *screen != Screen::Lightning {
            return false;
        }
        let Some(l) = self.state.as_mut() else {
            return false;
        };
        if key == K::Escape {
            *screen = l.back;
            self.state = None;
            return true;
        }
        if !matches!(
            l.source,
            Some(NodeSource::Aezeed | NodeSource::CoreLightning)
        ) {
            return false;
        }
        // Nothing is typed into a file's words: only its passphrase, where
        // the file asks for one.
        if l.source == Some(NodeSource::CoreLightning)
            && l.file.is_some()
            && !(l.file_asks && l.on_passphrase)
        {
            if key == K::Enter {
                return true;
            }
            return false;
        }
        l.sample.dismiss();
        let field = if l.on_passphrase {
            &mut l.passphrase
        } else {
            &mut l.typed
        };
        match key {
            K::Char(c) if l.on_passphrase && (' '..='~').contains(&c) => field.push(c),
            // Anything else is refused, and the screen says so.
            K::Char(c) if l.on_passphrase && !c.is_control() => {
                l.error = Some(crate::state::ascii_refused());
                return true;
            }
            K::Char(c) if c.is_ascii_alphabetic() => field.push(c.to_ascii_lowercase()),
            K::Char(' ') => {
                if !field.is_empty() && !field.ends_with(' ') {
                    field.push(' ');
                }
            }
            K::Backspace => {
                field.pop();
            }
            K::Tab if l.file.is_none() => l.on_passphrase = !l.on_passphrase,
            // The read itself is started by `Faraday::lightning_key`.
            K::Enter => {
                if l.file.is_none() {
                    l.on_passphrase = false;
                }
                return true;
            }
            _ => return false,
        }
        if let Some(l) = self.state.as_mut() {
            l.node = None;
        }
        true
    }

    /// Practice's passphrase typed into the aezeed's passphrase field.
    pub(crate) fn set_passphrase(&mut self, text: &str) {
        if let Some(l) = self.state.as_mut() {
            l.passphrase.set(text);
            l.node = None;
            l.error = None;
        }
    }

    /// Whether the aezeed's words field draws Use a sample: the cipher
    /// seed chosen, nothing typed or pressed.
    pub(crate) fn sample_shown(&self) -> bool {
        self.state.as_ref().is_some_and(|l| {
            matches!(
                l.source,
                Some(NodeSource::Aezeed | NodeSource::CoreLightning)
            ) && l.typed.is_empty()
                && l.file.is_none()
                && l.sample.shown()
        })
    }

    /// An aezeed read by a scanner, typed in: the cipher seed chosen as
    /// the source, its words lowercased with one space between, the
    /// passphrase left for the person to type (`docs/SCANNING.md` §4.2).
    pub(crate) fn put_aezeed(&mut self, words: &str) {
        if let Some(l) = self.state.as_mut() {
            let typed = Zeroizing::new(
                words
                    .split_whitespace()
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            l.source = Some(NodeSource::Aezeed);
            l.typed.set(&typed);
            l.node = None;
            l.error = None;
            l.sample.dismiss();
        }
    }

    /// Types the sample cipher seed's `words` in.
    pub(crate) fn use_sample(&mut self, words: &str) {
        if let Some(l) = self.state.as_mut() {
            l.typed.set(words);
            l.node = None;
            l.error = None;
            l.sample.fill();
        }
    }
}

impl Faraday {
    /// The aezeed's entry opened from Convert, to return to it.
    pub(crate) fn lightning_for_convert(&mut self) {
        // As Add a key: no secret is read in while a stick is attached.
        if !self.stick.is_empty() {
            return self.toast(&crate::stick::keys_load(self.medium));
        }
        if !self.may_load_keys() {
            return;
        }
        self.lightning.open_for_convert();
        self.screen = Screen::Lightning;
    }

    /// The Lightning screen's state, while it is open.
    pub fn lightning(&self) -> Option<&LightningState> {
        self.lightning.state()
    }

    /// One press on the screen.
    pub(crate) fn lightning_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        match action {
            A::Lightning => {
                self.lightning.open(self.screen);
                self.screen = Screen::Lightning;
            }
            A::LFile(at) => self.lightning_file(at),
            A::LResolve if self.lightning_reads_aezeed() => self.lightning_read(),
            A::LResolve if self.lightning.on_core_lightning() => {
                let c = self.compute();
                self.lightning.read_words(&c, &self.session.keys);
            }
            A::LResolve => {
                let c = self.compute();
                self.lightning.resolve(&c, &self.session.keys);
            }
            A::LVault => {
                if self.vaults.open.get(self.vaults.current).is_none() {
                    return self.toast(EN.lightning_no_vault);
                }
                let Some((id, private)) = self.lightning.secret_text(
                    faraday_keys::Reveal::VaultEntry(faraday_keys::Part::NodeKey),
                ) else {
                    return;
                };
                let note = fill_secret(EN.lightning_lightning, &[&id, private.as_str()]);
                self.vault_push(
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    EN.lightning_the_node,
                );
            }
            A::LOut => {
                let Some((id, private)) = self.lightning.secret_text(
                    faraday_keys::Reveal::OutboxFile(faraday_keys::Part::NodeKey),
                ) else {
                    return;
                };
                self.offer_secret(crate::secrets::SecretOut::new(
                    format!("lightning-{}.txt", &id[..16]),
                    Zeroizing::new(
                        format!("node id {id}\nprivate key {}\n", private.as_str()).into_bytes(),
                    ),
                    EN.lightning_a_lightning,
                    EN.lightning_whoever_has,
                ));
            }
            _ => {
                let c = self.compute();
                self.lightning.act(&c, &self.session.keys, action);
            }
        }
    }

    /// The read card's **Use in Lightning**: the screen opens, to return
    /// to the screen under the scanner, with the aezeed read typed in.
    pub(crate) fn lightning_with_aezeed(&mut self, words: &str) {
        self.lightning.open(self.screen);
        self.screen = Screen::Lightning;
        self.lightning.put_aezeed(words);
    }

    /// Whether the screen reads a typed aezeed, whose Enter stretches it
    /// with scrypt.
    pub(crate) fn lightning_reads_aezeed(&self) -> bool {
        self.lightning.reads_aezeed(self.sheet.is_none())
    }

    /// Whether `key` is left for the tick after a frame: Enter where it
    /// reads an aezeed or Core Lightning's file or words, which scrypt,
    /// Argon2id or a PBKDF2 stretch. Not
    /// while a job runs: then the key is refused as every key is.
    pub(crate) fn key_slow(&self, key: faraday_shell_api::Key) -> bool {
        key == faraday_shell_api::Key::Enter
            && self.worker.busy().is_none()
            && self.screen == Screen::Lightning
            && (self.lightning_reads_aezeed()
                || (self.sheet.is_none() && self.lightning.on_core_lightning()))
    }

    /// [`Faraday::key_slow`] once a key waiting from before is taken, so a
    /// job it starts is seen running.
    pub(crate) fn later_then_slow(&mut self, key: faraday_shell_api::Key) -> bool {
        self.later_now();
        self.key_slow(key)
    }

    /// What the screen says while an aezeed is read.
    pub(crate) fn lightning_working(&self) -> &'static str {
        if self.lightning.for_convert() {
            EN.working_aezeed
        } else {
            EN.working_node_key
        }
    }

    /// An `hsm_secret` chosen in Files: the read starts at once where the
    /// file asks for no passphrase, else when Read it is pressed.
    fn lightning_file(&mut self, at: usize) {
        let Some((name, asks)) = self
            .inbox
            .get(at)
            .filter(|i| could_be_hsm(i.kind, i.bytes.len()))
            .map(|i| {
                (
                    i.name.clone(),
                    faraday_keys::hsm_secret::needs_passphrase(&i.bytes),
                )
            })
        else {
            return;
        };
        self.lightning.choose_file(name, asks);
        if !asks {
            self.lightning_read();
        }
    }

    /// Starts the aezeed or `hsm_secret` read on the worker.
    fn lightning_read(&mut self) {
        let job = if self.lightning.on_core_lightning() {
            let Some(name) = self.lightning.chosen_file() else {
                return;
            };
            // Looked up by name now: the file may have gone or changed
            // since it was chosen.
            let Some(item) = self.inbox.iter().find(|i| i.name == name) else {
                return self.lightning.file_gone();
            };
            let asks = faraday_keys::hsm_secret::needs_passphrase(&item.bytes);
            if asks != self.lightning.file_asks() {
                return self.lightning.file_changed(asks);
            }
            self.lightning
                .hsm_job(&self.session.keys, Zeroizing::new(item.bytes.clone()))
        } else {
            self.lightning.read_job(&self.session.keys)
        };
        let Some(job) = job else {
            return;
        };
        let busy = self.lightning_working();
        self.start(crate::worker::Job::Aezeed(Box::new(job)), busy);
    }

    /// The aezeed read's answer. Dropped, and wiped, where the screen was
    /// left meanwhile.
    pub(crate) fn aezeed_read(&mut self, done: ReadDone) {
        if self.screen != Screen::Lightning {
            return;
        }
        if self.lightning.for_convert() {
            if let Some(fp) = self.lightning.read_held(&mut self.session.keys, done) {
                // Back at the flow, not a step forward: its old spot goes.
                self.trail.drop_screen(Screen::Convert);
                self.screen = Screen::Convert;
                self.tools.convert_mut().choose_aezeed(fp);
            }
        } else {
            self.lightning.read_done(&self.session.keys, done);
        }
    }

    /// Typing the aezeed and its passphrase. Returns whether the key was
    /// taken. Enter reads the aezeed, on the worker.
    pub(crate) fn lightning_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        // Escape leaves the screen: a read under way has nowhere to go.
        if key == K::Escape && self.screen == Screen::Lightning {
            self.taken_while_busy(crate::Action::Back);
        }
        let reads = key == K::Enter && self.lightning_reads_aezeed();
        let words = key == K::Enter && !reads && self.lightning.on_core_lightning();
        let taken = self.lightning.key(key, &mut self.screen);
        if taken && reads {
            self.lightning_read();
        } else if taken && words {
            let c = self.compute();
            self.lightning.read_words(&c, &self.session.keys);
        }
        taken
    }
}

/// Whether a file of this kind and length is listed as a possible
/// `hsm_secret` under the Core Lightning source.
pub(crate) fn could_be_hsm(kind: crate::wallet::FileKind, len: usize) -> bool {
    matches!(
        kind,
        crate::wallet::FileKind::Other | crate::wallet::FileKind::Text
    ) && (faraday_keys::hsm_secret::PLAIN_LEN..=faraday_keys::hsm_secret::MNEMONIC_MAX_LEN)
        .contains(&len)
}
