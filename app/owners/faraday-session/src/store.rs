//! The key store, the handles over it and the keys held for their backup.

use faraday_bip::codex32::{Codex32, INDEX_ORDER};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_kernel::Facts;
use faraday_kernel::session::{KeyInput, KeyView, WaitingItem};
use faraday_kernel::{AtLock, Holds, KeyId, Loss};
use faraday_keys::{Keys, Making, Refusal, WorkSet};
use faraday_port::values::Medium;
use faraday_text::{Text, secret::SecretText};

/// The name of the storage that is in the way of keys, when some is: the keys refuse to come in
/// while it is attached.
pub(crate) fn stick_in_the_way(facts: &Facts) -> Option<&'static str> {
    (facts.sticks.attached || !facts.sticks.listed.is_empty()).then_some(match facts.medium {
        Medium::SdCard => faraday_strings::EN.medium_card_or_stick_inline,
        Medium::Stick | Medium::Desktop => faraday_strings::EN.medium_stick_inline,
    })
}

/// What the Lock sheet names when a lock would lose a key held for its backup.
const HELD_LOSS: Loss = Loss("session.held");

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
    /// What waits in the session: the keys held for their backup, in the order they were held.
    waiting: Vec<WaitingItem>,
}

impl Store {
    pub(crate) fn new(network: Network) -> Store {
        Store {
            keys: Keys::on(network),
            slots: Vec::new(),
            next: 0,
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
        !self.waiting.is_empty()
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
                only_with_words(&passphrase)?;
                self.keys.add_seed(c, seed.expose(), "")?
            }
            KeyInput::Xprv(text) => {
                only_with_words(&passphrase)?;
                match self.keys.add_xprv(c, text.as_str(), "") {
                    Ok(fp) => fp,
                    Err(faraday_keys::xprv::XprvError::Session(refusal)) => return Err(refusal),
                    // A text that is no master key to load. `faraday-keys` words no refusal
                    // for it yet: it stays refused, with no text of its own.
                    Err(faraday_keys::xprv::XprvError::Read(_)) => {
                        return Err(Refusal::Words(Text::new()));
                    }
                }
            }
            KeyInput::Shares(shares) => {
                only_with_words(&passphrase)?;
                let mut parsed = Vec::with_capacity(shares.len());
                for share in &shares {
                    parsed
                        .push(Codex32::parse(share.as_str()).map_err(|e| {
                            Refusal::Words(faraday_strings::bip::codex32_error(&e))
                        })?);
                }
                let codex = |e: faraday_bip::codex32::Error| {
                    Refusal::Words(faraday_strings::bip::codex32_error(&e))
                };
                match parsed.as_slice() {
                    // The secret string itself: its payload is the master seed.
                    [one] if one.is_secret() => self.keys.add_seed(c, one.payload(), "")?,
                    // `k` shares of one set: `faraday-keys` rebuilds the seed and keeps the set.
                    _ => {
                        let taken: Vec<u8> = parsed.iter().map(Codex32::share_index).collect();
                        let spare = INDEX_ORDER
                            .iter()
                            .copied()
                            .find(|i| !taken.contains(i))
                            .ok_or_else(|| Refusal::Words(Text::new()))?;
                        let work = WorkSet::given(c, &parsed, Making::Add(spare)).map_err(codex)?;
                        self.keys.add_codex32(c, &work, "")?
                    }
                }
            }
        };
        let record = self
            .keys
            .find(fp)
            .map(|info| info.id)
            .ok_or_else(|| Refusal::Words(Text::new()))?;
        let id = KeyId::minted(self.next);
        self.next = self.next.saturating_add(1);
        self.slots.push(Slot { id, record });
        Ok(id)
    }

    /// Key `id` goes, and its hold with it. Whether it was here.
    pub(crate) fn remove(&mut self, id: KeyId) -> bool {
        let Some(at) = self.slots.iter().position(|s| s.id == id) else {
            return false;
        };
        let slot = self.slots.remove(at);
        if let Some(place) = self.keys.place_of(slot.record) {
            self.keys.remove(place);
        }
        self.waiting
            .retain(|w| !matches!(w, WaitingItem::Held(held) if *held == id));
        true
    }

    /// Every key goes, and every hold; the handles that were.
    pub(crate) fn remove_all(&mut self) -> Vec<KeyId> {
        self.keys.remove_all();
        self.waiting.clear();
        self.slots.drain(..).map(|s| s.id).collect()
    }

    /// Key `id` is held for its backup, if it is here and not held already.
    pub(crate) fn hold(&mut self, id: KeyId) {
        if self.slots.iter().any(|s| s.id == id) && !self.is_held(id) {
            self.waiting.push(WaitingItem::Held(id));
        }
    }

    /// Key `id`'s backup is done: it is not held any more. Whether it was.
    pub(crate) fn release(&mut self, id: KeyId) -> bool {
        let before = self.waiting.len();
        self.waiting
            .retain(|w| !matches!(w, WaitingItem::Held(held) if *held == id));
        self.waiting.len() != before
    }
}

/// A passphrase belongs to words (BIP 39): a master seed, an extended private key and a set of
/// shares take none, and a key is never added with it left out unasked.
fn only_with_words(passphrase: &Option<SecretText>) -> Result<(), Refusal> {
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
            asks_first: self.any_held().then_some(HELD_LOSS),
        }
    }

    /// Nothing is on show here: the screens that show a key are the features'.
    fn hide(&mut self) {}

    fn at_lock(&mut self) {
        self.keys.clear();
        self.slots.clear();
        self.waiting.clear();
    }
}
