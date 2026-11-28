//! The key store, the handles over it and the keys held for their backup.

use faraday_bip::codex32::Codex32;
use faraday_bip::keys::Network;
use faraday_bip::slip39::Share;
use faraday_crypto::work::Compute;
use faraday_kernel::session::{KeyInput, KeyView, WaitingItem};
use faraday_kernel::{AtLock, Holds, KeyId};
use faraday_kernel::{Facts, PracticeSamples};
use faraday_keys::{Keys, Refusal, codex32_seed, slip39_seed};
use faraday_port::values::Medium;
use faraday_text::{Text, secret::SecretText};
use faraday_vault::codex32::SetFacts;

/// The name of the storage that is in the way of keys, when some is: the keys refuse to come in
/// while it is attached.
pub(crate) fn stick_in_the_way(facts: &Facts) -> Option<&'static str> {
    (facts.sticks.attached || !facts.sticks.listed.is_empty()).then_some(match facts.medium {
        Medium::SdCard => faraday_strings::EN.medium_card_or_stick_inline,
        Medium::Stick | Medium::Desktop => faraday_strings::EN.medium_stick_inline,
    })
}

/// A handle and the key it stands for: all 32 bytes of what tells the key from another, so a
/// handle never reaches another key, and never an index that a removal moves.
struct Slot {
    id: KeyId,
    record: faraday_keys::KeyId,
}

/// The keys the session holds. A key made in the session is held until its backup is done:
/// `held` is what `main`'s `held_keys_now` worked out, by the handle and never by a secret.
pub(crate) struct Store {
    keys: Keys,
    slots: Vec<Slot>,
    /// The next handle's number; never goes back within a build of the scope.
    next: u32,
    /// The keys held for their backup, by the key's identity and not by its handle: removing a
    /// key does not release it, and the key typed in again is held again. Only `release` and a
    /// lock end a hold (`main`'s `made_here`, by fingerprint).
    held: Vec<faraday_keys::KeyId>,
    /// What waits in the session: the held keys that are here, in the order they were added.
    waiting: Vec<WaitingItem>,
}

impl Store {
    pub(crate) fn new(network: Network) -> Store {
        Store {
            keys: Keys::on(network),
            slots: Vec::new(),
            next: 0,
            held: Vec::new(),
            waiting: Vec::new(),
        }
    }

    pub(crate) fn keys(&self) -> &Keys {
        &self.keys
    }

    pub(crate) fn keys_mut(&mut self) -> &mut Keys {
        &mut self.keys
    }

    pub(crate) fn waiting(&self) -> &[WaitingItem] {
        &self.waiting
    }

    /// The keys follow the sticks: no key is added while one is attached
    /// (`docs/SECURITY-MODEL.md`).
    pub(crate) fn set_stick(&mut self, noun: Option<&'static str>) {
        self.keys.set_stick_attached(noun);
    }

    /// Whether key `id` is held for its backup. A held key signs nothing and receives nothing:
    /// the signing and the receiving of the wallets ask this before they go on.
    pub(crate) fn is_held(&self, id: KeyId) -> bool {
        self.waiting
            .iter()
            .any(|w| matches!(w, WaitingItem::Held(held) if *held == id))
    }

    /// Whether any key is held.
    pub(crate) fn any_held(&self) -> bool {
        !self.held.is_empty()
    }

    /// `waiting` worked out again from the slots and the holds.
    fn sync(&mut self) {
        self.waiting = self
            .slots
            .iter()
            .filter(|s| self.held.contains(&s.record))
            .map(|s| WaitingItem::Held(s.id))
            .collect();
    }

    /// Practice: the store takes only the published samples the product gave (03 §11, A33). A
    /// Practice given none takes no key at all: the boundary fails closed.
    pub(crate) fn practice(&mut self, samples: Option<&PracticeSamples>) {
        let Some(samples) = samples else {
            self.keys.set_demo(&[], &[], &[]);
            return;
        };
        let handouts: Vec<faraday_keys::Handout> = samples
            .handouts
            .iter()
            .map(|h| faraday_keys::Handout {
                name: h.name,
                words: h.words,
                plain: faraday_bip::keys::Fingerprint(h.plain),
                with_passphrase: faraday_bip::keys::Fingerprint(h.with_passphrase),
            })
            .collect();
        let public: Vec<faraday_bip::keys::Fingerprint> = samples
            .public
            .iter()
            .map(|f| faraday_bip::keys::Fingerprint(*f))
            .collect();
        self.keys.set_demo(&handouts, &public, samples.nodes);
    }

    pub(crate) fn view(&self, id: KeyId) -> Option<KeyView<'_>> {
        let slot = self.slots.iter().find(|s| s.id == id)?;
        let info = self.keys.info(self.keys.place_of(slot.record)?)?;
        Some(KeyView {
            fingerprint: info.fingerprint,
            name: &info.label,
            held: self.is_held(id),
            sample: self.keys.is_sample(info.fingerprint),
            made_online: self.keys.made_online(info.fingerprint),
        })
    }

    /// Costs what the key's own `add_*` costs. The key as it comes in, under a handle. What
    /// `Keys` refuses it refuses (a stick attached, a key the demo does not take, a duplicate),
    /// and a passphrase goes only with words: anything else with one is refused, never added
    /// without it.
    pub(crate) fn add(
        &mut self,
        c: &Compute,
        key: KeyInput,
        passphrase: Option<SecretText>,
    ) -> Result<KeyId, Refusal> {
        let fp = match key {
            KeyInput::Words(words) => {
                let pass = passphrase.as_ref().map_or("", |p| p.as_str());
                self.keys
                    .add_words_with(c, words.as_str(), pass, "", None)?
            }
            KeyInput::Seed(seed) => {
                only_with_words(passphrase.as_ref())?;
                self.keys.add_seed(c, seed.expose(), "")?
            }
            KeyInput::Xprv(text) => {
                only_with_words(passphrase.as_ref())?;
                match self.keys.add_xprv(c, text.as_str(), "") {
                    Ok(fp) => fp,
                    Err(faraday_keys::xprv::XprvError::Session(refusal)) => return Err(refusal),
                    Err(faraday_keys::xprv::XprvError::Read(refusal)) => {
                        return Err(Refusal::Words(crate::xprv_words::xprv_refusal(&refusal)));
                    }
                }
            }
            KeyInput::Shares(shares) => self.add_shares(c, &shares, passphrase.as_ref())?,
        };
        let record = self
            .keys
            .find(fp)
            .map(|info| info.id)
            .ok_or_else(|| Refusal::Words(Text::new()))?;
        let id = KeyId::minted(self.next);
        self.next = self.next.saturating_add(1);
        self.slots.push(Slot { id, record });
        self.sync();
        Ok(id)
    }

    /// Costs what `add_seed` costs and a recovery of the shares. The key the strings or the
    /// words make: SLIP-39 shares with their passphrase, codex32 strings with none. A codex32
    /// key keeps the plain facts of its set and the strings typed, as `main`'s did.
    fn add_shares(
        &mut self,
        c: &Compute,
        shares: &[SecretText],
        passphrase: Option<&SecretText>,
    ) -> Result<faraday_bip::keys::Fingerprint, Refusal> {
        let words = |e: Text| Refusal::Words(e);
        let codex = shares
            .first()
            .is_some_and(|s| s.as_str().trim().to_ascii_lowercase().starts_with("ms1"));
        if !codex {
            let mut parsed = Vec::with_capacity(shares.len());
            for share in shares {
                parsed.push(
                    Share::parse(share.as_str())
                        .map_err(|e| words(faraday_strings::bip::slip39_error(&e)))?,
                );
            }
            let pass = passphrase.map_or("", |p| p.as_str());
            let seed = slip39_seed(c, &parsed, pass.as_bytes())
                .map_err(|e| words(faraday_strings::bip::slip39_error(&e)))?;
            return self.keys.add_seed(c, &seed, "");
        }
        only_with_words(passphrase)?;
        let strings = |e: faraday_bip::codex32::Error| {
            words(Text::fill(
                faraday_strings::EN.forms_the_strings_do,
                &[&faraday_strings::bip::codex32_error(&e)],
            ))
        };
        let mut typed: Vec<Codex32> = Vec::with_capacity(shares.len());
        for share in shares {
            typed.push(Codex32::parse(share.as_str().trim()).map_err(strings)?);
        }
        typed = one_set(typed)?;
        let seed = codex32_seed(&typed).map_err(strings)?;
        let fp = self.keys.add_seed(c, &seed, "")?;
        let texts: Vec<Text> = typed
            .iter()
            .map(|s| Text::of(s.encode().as_str()))
            .collect();
        let refs: Vec<&str> = texts.iter().map(Text::as_str).collect();
        if let Some(i) = self.keys.position(fp) {
            self.keys.set_codex32(i, set_facts(&typed));
            self.keys.keep_codex32(c, i, &refs);
        }
        Ok(fp)
    }

    /// Key `id` goes. Its hold stays with the key's identity: the key typed in again is held
    /// again. Whether it was here.
    pub(crate) fn remove(&mut self, id: KeyId) -> bool {
        let Some(at) = self.slots.iter().position(|s| s.id == id) else {
            return false;
        };
        let slot = self.slots.remove(at);
        if let Some(place) = self.keys.place_of(slot.record) {
            self.keys.remove(place);
        }
        self.sync();
        true
    }

    /// Every key goes, the holds stay; the handles that were.
    pub(crate) fn remove_all(&mut self) -> Vec<KeyId> {
        self.keys.remove_all();
        let gone = self.slots.drain(..).map(|s| s.id).collect();
        self.sync();
        gone
    }

    /// Key `id` is held for its backup, if it is here and not held already.
    pub(crate) fn hold(&mut self, id: KeyId) {
        let Some(record) = self.slots.iter().find(|s| s.id == id).map(|s| s.record) else {
            return;
        };
        if !self.held.contains(&record) {
            self.held.push(record);
        }
        self.sync();
    }

    /// Key `id`'s backup is done: it is not held any more. Whether it was.
    pub(crate) fn release(&mut self, id: KeyId) -> bool {
        let Some(record) = self.slots.iter().find(|s| s.id == id).map(|s| s.record) else {
            return false;
        };
        let before = self.held.len();
        self.held.retain(|h| *h != record);
        self.sync();
        self.held.len() != before
    }
}

/// The strings of one set, one for each index: a string whose identifier differs from the
/// first one's is of another seed, and a different string at an index already given clashes,
/// both refused as `main` refused them as each was added (`add_codex32`); the same string again
/// is dropped.
fn one_set(typed: Vec<Codex32>) -> Result<Vec<Codex32>, Refusal> {
    let mut kept: Vec<Codex32> = Vec::with_capacity(typed.len());
    for string in typed {
        if let Some(first) = kept.first()
            && first.identifier() != string.identifier()
        {
            return Err(Refusal::Words(Text::of(
                faraday_strings::EN.forms_other_seed_string,
            )));
        }
        match kept
            .iter()
            .find(|k| k.share_index() == string.share_index())
        {
            Some(same) if same.encode().as_str() == string.encode().as_str() => {}
            Some(_) => {
                return Err(Refusal::Words(Text::of(
                    faraday_strings::EN.forms_string_clash,
                )));
            }
            None => kept.push(string),
        }
    }
    Ok(kept)
}

/// The plain facts of the set the codex32 strings `typed` are of, for the key they load: k, the
/// identifier, the length and each index typed (`main`'s `forms::set_facts`).
fn set_facts(typed: &[Codex32]) -> Option<SetFacts> {
    let first = typed.first()?;
    Some(SetFacts {
        threshold: first.threshold(),
        identifier: first.identifier(),
        length: u8::try_from(first.encode().as_str().len()).unwrap_or(0),
        unknown: 0,
        strings: typed
            .iter()
            .map(|s| (s.share_index(), faraday_vault::codex32::State::Made))
            .collect(),
    })
}

/// A passphrase belongs to words (BIP 39) and to SLIP-39 shares: a master seed, an extended
/// private key and codex32 strings take none, and a key is never added with it left out unasked.
fn only_with_words(passphrase: Option<&SecretText>) -> Result<(), Refusal> {
    match passphrase {
        Some(_) => Err(Refusal::Words(Text::new())),
        None => Ok(()),
    }
}

impl AtLock for Store {
    fn holds(&self) -> Holds {
        Holds {
            kept: self.keys.holds_secret(),
            typed: false,
            session: self.any_held(),
            // `main` asks first at a lock only for what a lock loses of the flows and the
            // vaults; a held key alone does not (its hold is the backup flow's).
            asks_first: None,
        }
    }

    /// Nothing is on show here: the screens that show a key are the features'.
    fn hide(&mut self) {}

    fn at_lock(&mut self) {
        self.keys.clear();
        self.slots.clear();
        self.held.clear();
        self.waiting.clear();
    }
}
