//! The steps every key wizard ends with (UX.md §7.2): the passphrase
//! offer, passphrase entry, the with/without fingerprint comparison, the
//! hold-to-add confirmation, and, for the first key of a session, the
//! session PIN typed twice (UX.md I1, `docs/PLANNING.md` §16.21). The
//! Load and Create wizards share this state, its input handling and its
//! views (`views/finish.rs`); each maps its own step enum onto
//! [`FinishStep`].
//!
//! The passphrase and the PIN entries are fixed byte arrays zeroized on
//! drop; the built [`MasterKey`] and seed erase themselves. This file is
//! listed in `tools/lint-secrets.sh` and may hold no heap text.

use osk_bip::bip39::{MAX_PASSPHRASE_BYTES, Mnemonic};
use osk_bip::keys::{Fingerprint, MasterKey, Network};
use osk_bip::slip39::{self, Share};
use osk_crypto::{Secret, SeedBytes, Zeroize, ZeroizeOnDrop};
use osk_ui::widgets::keyboard::KeyInput;

use crate::ids::{self, Id};
use crate::load::LoadedKey;
use crate::session::PinEntry;
use crate::shares;

/// Milliseconds a typed secret character stays visible before it is
/// masked (`docs/PLANNING.md` §4.6).
pub const MASK_MS: u64 = 500;

/// The finishing steps, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishStep {
    /// "Add a passphrase?"
    PassphraseOffer,
    /// Typing the passphrase.
    Passphrase,
    /// Fingerprints without and with the passphrase.
    PassphraseConfirm,
    /// The final fingerprint and the hold to add.
    Confirm,
    /// Setting the session PIN; entered from the hold when the session
    /// has none yet.
    Pin,
    /// Repeating the session PIN.
    PinConfirm,
}

/// What a wizard's finishing steps build a key from: BIP-39 words,
/// whose seed is PBKDF2 of the words and the passphrase, or a set of
/// SLIP-39 shares, whose seed is the master secret they give under the
/// passphrase (`docs/PLANNING.md` §16.107 rule 2). The two fingerprints
/// "Which key?" compares come from the same source, read twice.
pub enum Material<'a> {
    /// The words typed or computed.
    Words(&'a Mnemonic),
    /// Exactly the shares [`slip39::recover`] needs.
    Shares(&'a [Share]),
    /// The BIP-32 seed itself, which is what a codex32 string carries.
    /// BIP 93 has no passphrase, so the seed is the same read under any
    /// (`docs/PLANNING.md` §16.109 rule 2).
    Seed(&'a [u8]),
    /// A master secret on its way into shares that have not been made
    /// yet. Reading them back gives this secret under the passphrase
    /// they are written with, and some other secret under any other, so
    /// the two fingerprints are encryption under the typed passphrase
    /// and decryption under each (`docs/PLANNING.md` §16.107 rule 4).
    Secret(&'a [u8]),
}

impl Material<'_> {
    /// The BIP-32 seed this material gives when it is read under
    /// `passphrase`. `typed` is the passphrase the shares are being
    /// written under, which only a secret on its way into shares uses.
    fn seed(&self, passphrase: &[u8], typed: &[u8]) -> Option<Secret<SeedBytes>> {
        match self {
            Material::Words(m) => {
                let seed = m.to_seed(passphrase).ok()?;
                Some(Secret::new(SeedBytes::new(seed.expose())?))
            }
            Material::Seed(seed) => Some(Secret::new(SeedBytes::new(seed)?)),
            Material::Shares(shares) => {
                let secret = slip39::recover(shares, passphrase).ok()?;
                Some(Secret::new(SeedBytes::new(secret.expose().as_bytes())?))
            }
            Material::Secret(ms) => {
                let encrypted = slip39::encrypt(
                    ms,
                    typed,
                    IDENTIFIER_UNSALTED,
                    shares::EXTENDABLE,
                    shares::ITERATION_EXPONENT,
                )
                .ok()?;
                let secret = slip39::decrypt(
                    encrypted.expose().as_bytes(),
                    passphrase,
                    IDENTIFIER_UNSALTED,
                    shares::EXTENDABLE,
                    shares::ITERATION_EXPONENT,
                )
                .ok()?;
                Some(Secret::new(SeedBytes::new(secret.expose().as_bytes())?))
            }
        }
    }
}

/// The identifier the encryption is salted with. An extendable backup
/// does not salt with it at all, so the identifier a split is given
/// later cannot change the key these two fingerprints name.
const IDENTIFIER_UNSALTED: u16 = 0;

/// What [`Finish::build`] produced.
struct Built {
    plain: Fingerprint,
    with_passphrase: Option<Fingerprint>,
    seed: Secret<SeedBytes>,
    master: MasterKey,
}

/// Passphrase, built key and session PIN of a wizard's finishing steps.
pub struct Finish {
    passphrase: [u8; MAX_PASSPHRASE_BYTES],
    passphrase_len: u16,
    typed_at: u64,
    built: Option<Built>,
    pin: PinEntry,
    pin_first: PinEntry,
    pin_mismatch: bool,
    pin_done: bool,
    /// The passphrase offer's checked row: "Add a passphrase" rather
    /// than "No passphrase" (`docs/DESIGN.md` §2.7, a choice is checked
    /// and then confirmed).
    offer_add: bool,
    /// "Which key?": the fingerprint with the passphrase is the checked
    /// row. Checking the other one and confirming drops the passphrase.
    which_passphrase: bool,
}

impl Zeroize for Finish {
    fn zeroize(&mut self) {
        self.passphrase.zeroize();
        self.passphrase_len = 0;
        self.typed_at = 0;
        self.built = None;
        self.pin.zeroize();
        self.pin_first.zeroize();
        self.pin_mismatch = false;
        self.pin_done = false;
        self.offer_add = false;
        self.which_passphrase = true;
    }
}

impl Drop for Finish {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Finish {}

impl Default for Finish {
    fn default() -> Self {
        Self::new()
    }
}

impl Finish {
    /// No passphrase, nothing built.
    pub fn new() -> Self {
        Finish {
            passphrase: [0; MAX_PASSPHRASE_BYTES],
            passphrase_len: 0,
            typed_at: 0,
            built: None,
            pin: PinEntry::new(),
            pin_first: PinEntry::new(),
            pin_mismatch: false,
            pin_done: false,
            offer_add: false,
            which_passphrase: true,
        }
    }

    /// Goes one step back from `step`, zeroizing what that step owned.
    /// `None` when leaving the finishing steps altogether.
    pub fn back(&mut self, step: FinishStep) -> Option<FinishStep> {
        Some(match step {
            FinishStep::PassphraseOffer => return None,
            FinishStep::Passphrase => {
                self.clear_passphrase();
                self.offer_add = false;
                FinishStep::PassphraseOffer
            }
            FinishStep::PassphraseConfirm => {
                self.built = None;
                FinishStep::Passphrase
            }
            FinishStep::Confirm => {
                self.built = None;
                if self.passphrase_len > 0 {
                    FinishStep::PassphraseConfirm
                } else {
                    FinishStep::PassphraseOffer
                }
            }
            FinishStep::Pin => {
                self.clear_pins();
                FinishStep::Confirm
            }
            FinishStep::PinConfirm => {
                self.clear_pins();
                FinishStep::Pin
            }
        })
    }

    /// Handles a tap on `step`. Returns the step to show next, if it
    /// changes. `material` is what the key is built from.
    pub fn tap(
        &mut self,
        step: FinishStep,
        id: Id,
        material: &Material<'_>,
        network: Network,
    ) -> Option<FinishStep> {
        match step {
            // A choice is checked, then confirmed (`docs/DESIGN.md`
            // §2.7): the row moves the check, Continue moves the step.
            FinishStep::PassphraseOffer => {
                if id == ids::LOAD_SKIP {
                    self.offer_add = false;
                    None
                } else if id == ids::LOAD_ADD_PASSPHRASE {
                    self.offer_add = true;
                    None
                } else if id != ids::LOAD_PASS_CONTINUE {
                    None
                } else if self.offer_add {
                    Some(FinishStep::Passphrase)
                } else {
                    self.build(material, network).then_some(FinishStep::Confirm)
                }
            }
            FinishStep::PassphraseConfirm => {
                if id == ids::LOAD_WHICH_PLAIN {
                    self.which_passphrase = false;
                    None
                } else if id == ids::LOAD_WHICH_PASSPHRASE {
                    self.which_passphrase = true;
                    None
                } else if id != ids::LOAD_WHICH_CONTINUE {
                    None
                } else {
                    // The key the check is on is the key that is added,
                    // so choosing the plain fingerprint drops what was
                    // typed and builds the key again without it.
                    if !self.which_passphrase {
                        self.clear_passphrase();
                        self.offer_add = false;
                        if !self.build(material, network) {
                            return None;
                        }
                    }
                    Some(FinishStep::Confirm)
                }
            }
            _ => None,
        }
    }

    /// Whether the passphrase offer's checked row is "Add a passphrase".
    pub fn offer_add(&self) -> bool {
        self.offer_add
    }

    /// Whether "Which key?" has the fingerprint with the passphrase
    /// checked.
    pub fn which_passphrase(&self) -> bool {
        self.which_passphrase
    }

    /// Handles keyboard input on the passphrase step at time `now_ms`.
    /// Returns the step to show next, if it changes.
    pub fn key(
        &mut self,
        step: FinishStep,
        input: KeyInput,
        material: &Material<'_>,
        network: Network,
        now_ms: u64,
    ) -> Option<FinishStep> {
        if step != FinishStep::Passphrase {
            return None;
        }
        match input {
            KeyInput::Char(c) => {
                self.passphrase_push(c, now_ms);
                None
            }
            KeyInput::Backspace => {
                self.passphrase_pop();
                None
            }
            KeyInput::Done => {
                if !self.build(material, network) {
                    return None;
                }
                Some(if self.has_passphrase() {
                    self.which_passphrase = true;
                    FinishStep::PassphraseConfirm
                } else {
                    FinishStep::Confirm
                })
            }
            KeyInput::Shift | KeyInput::Symbols => None,
        }
    }

    // ----- session PIN -----

    /// Handles PIN pad input on the two PIN steps. Returns the step to
    /// show next, if it changes; after the second entry matches the
    /// first, [`take_pin`](Self::take_pin) yields the PIN.
    pub fn pin_key(&mut self, step: FinishStep, input: KeyInput) -> Option<FinishStep> {
        match (step, input) {
            (FinishStep::Pin | FinishStep::PinConfirm, KeyInput::Char(c)) => {
                self.pin.push(c);
                None
            }
            (FinishStep::Pin | FinishStep::PinConfirm, KeyInput::Backspace) => {
                self.pin.pop();
                None
            }
            (FinishStep::Pin, KeyInput::Done) => {
                if !self.pin.is_complete() {
                    return None;
                }
                self.pin_mismatch = false;
                core::mem::swap(&mut self.pin, &mut self.pin_first);
                self.pin.clear();
                Some(FinishStep::PinConfirm)
            }
            (FinishStep::PinConfirm, KeyInput::Done) => {
                if !self.pin.is_complete() {
                    return None;
                }
                if self.pin.same_as(&self.pin_first) {
                    self.pin_done = true;
                    None
                } else {
                    self.clear_pins();
                    self.pin_mismatch = true;
                    Some(FinishStep::Pin)
                }
            }
            _ => None,
        }
    }

    /// Digits typed on the current PIN step.
    pub fn pin_len(&self) -> usize {
        self.pin.len()
    }

    /// Whether the current PIN entry is long enough.
    pub fn pin_complete(&self) -> bool {
        self.pin.is_complete()
    }

    /// Whether the last confirmation did not match and the user starts
    /// over.
    pub fn pin_mismatch(&self) -> bool {
        self.pin_mismatch
    }

    /// The PIN, once typed twice the same. The entries are wiped.
    pub fn take_pin(&mut self) -> Option<PinEntry> {
        if !self.pin_done {
            return None;
        }
        let pin = core::mem::take(&mut self.pin);
        self.clear_pins();
        Some(pin)
    }

    fn clear_pins(&mut self) {
        self.pin.clear();
        self.pin_first.clear();
        self.pin_done = false;
    }

    // ----- passphrase -----

    /// Appends a printable ASCII character at time `now_ms`.
    pub fn passphrase_push(&mut self, c: char, now_ms: u64) -> bool {
        let n = usize::from(self.passphrase_len);
        if !c.is_ascii() || !(0x20..=0x7E).contains(&(c as u32)) || n >= MAX_PASSPHRASE_BYTES {
            return false;
        }
        self.passphrase[n] = c as u8;
        self.passphrase_len += 1;
        self.typed_at = now_ms;
        true
    }

    /// Deletes the last passphrase character.
    pub fn passphrase_pop(&mut self) {
        if self.passphrase_len > 0 {
            self.passphrase_len -= 1;
            self.passphrase[usize::from(self.passphrase_len)] = 0;
        }
    }

    /// Passphrase length in characters.
    pub fn passphrase_len(&self) -> usize {
        usize::from(self.passphrase_len)
    }

    /// The last typed character while it is still unmasked at `now_ms`.
    pub fn passphrase_visible_char(&self, now_ms: u64) -> Option<char> {
        if self.passphrase_len == 0 || now_ms >= self.mask_deadline()? {
            return None;
        }
        Some(self.passphrase[usize::from(self.passphrase_len) - 1] as char)
    }

    /// When the last typed character masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        (self.passphrase_len > 0).then_some(self.typed_at + MASK_MS)
    }

    fn clear_passphrase(&mut self) {
        self.passphrase.zeroize();
        self.passphrase_len = 0;
        self.typed_at = 0;
    }

    /// Runs `f` on the passphrase typed, which the SLIP-39 split needs
    /// to write the shares under it.
    pub fn passphrase<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        f(&self.passphrase[..self.passphrase_len()])
    }

    /// Whether a passphrase has been typed.
    pub fn has_passphrase(&self) -> bool {
        self.passphrase_len > 0
    }

    // ----- key -----

    /// Derives the seed(s) and master key for `network`: without the
    /// passphrase (always, for the "recognise, don't re-type" comparison)
    /// and with it when one was typed.
    pub fn build(&mut self, material: &Material<'_>, network: Network) -> bool {
        let typed_len = self.passphrase_len();
        // One encryption, one decryption per fingerprint, on the step
        // that asks for them and not on every frame that draws them.
        let Some(plain_seed) = material.seed(b"", &self.passphrase[..typed_len]) else {
            return false;
        };
        let plain_master = MasterKey::from_seed_bytes(&plain_seed, network);
        let plain = plain_master.fingerprint();
        let built = if self.passphrase_len > 0 {
            let typed = &self.passphrase[..typed_len];
            let Some(seed) = material.seed(typed, typed) else {
                return false;
            };
            let master = MasterKey::from_seed_bytes(&seed, network);
            Built {
                plain,
                with_passphrase: Some(master.fingerprint()),
                seed,
                master,
            }
        } else {
            Built {
                plain,
                with_passphrase: None,
                seed: plain_seed,
                master: plain_master,
            }
        };
        self.built = Some(built);
        true
    }

    /// The fingerprint without a passphrase, once built.
    pub fn fingerprint_plain(&self) -> Option<Fingerprint> {
        self.built.as_ref().map(|b| b.plain)
    }

    /// The fingerprint with the passphrase, once built with one.
    pub fn fingerprint_with_passphrase(&self) -> Option<Fingerprint> {
        self.built.as_ref().and_then(|b| b.with_passphrase)
    }

    /// The fingerprint of the key that will be added.
    pub fn fingerprint(&self) -> Option<Fingerprint> {
        self.built
            .as_ref()
            .map(|b| b.with_passphrase.unwrap_or(b.plain))
    }

    /// Takes the built key out as a [`LoadedKey`] carrying `mnemonic`
    /// where the key has words (`docs/PLANNING.md` §16.19) and the quiz
    /// outcome. A SLIP-39 key passes `None`: it is its master secret and
    /// has no words. The passphrase is not carried: the seed already
    /// includes it.
    pub fn take_key(
        &mut self,
        mnemonic: Option<Mnemonic>,
        backup_verified: bool,
    ) -> Option<LoadedKey> {
        let built = self.built.take()?;
        Some(LoadedKey::new(
            built.seed,
            built.master,
            mnemonic,
            built.with_passphrase.is_some(),
            backup_verified,
        ))
    }
}

/// The key `m` states on `network`, with no passphrase: what a FROST
/// deal's computed member becomes on its way into Keys.
pub fn key_from_words(m: &Mnemonic, network: Network) -> Option<LoadedKey> {
    let seed = Material::Words(m).seed(b"", b"")?;
    let master = MasterKey::from_seed_bytes(&seed, network);
    let words = Mnemonic::from_indices(m.language(), m.indices()).ok()?;
    Some(LoadedKey::new(seed, master, Some(words), false, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use osk_bip::bip39::Language;

    fn abandon() -> Mnemonic {
        Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap()
    }

    #[test]
    fn passphrase_masks_after_half_a_second() {
        let mut f = Finish::new();
        assert!(f.passphrase_push('T', 1000));
        assert!(!f.passphrase_push('\u{e9}', 1000));
        assert_eq!(f.passphrase_visible_char(1200), Some('T'));
        assert_eq!(f.passphrase_visible_char(1500), None);
        assert_eq!(f.mask_deadline(), Some(1500));
        f.passphrase_pop();
        assert_eq!(f.passphrase_len(), 0);
        assert_eq!(f.mask_deadline(), None);
    }

    #[test]
    fn skip_builds_and_confirms_the_plain_key() {
        let mut f = Finish::new();
        let m = abandon();
        assert_eq!(
            f.tap(
                FinishStep::PassphraseOffer,
                ids::LOAD_SKIP,
                &Material::Words(&m),
                Network::Mainnet
            ),
            None,
            "the tap only checks"
        );
        assert_eq!(
            f.tap(
                FinishStep::PassphraseOffer,
                ids::LOAD_PASS_CONTINUE,
                &Material::Words(&m),
                Network::Mainnet
            ),
            Some(FinishStep::Confirm)
        );
        assert_eq!(f.fingerprint().unwrap().to_hex(), *b"73c5da0a");
        assert_eq!(f.fingerprint_with_passphrase(), None);
        let key = f.take_key(Some(m), false).unwrap();
        assert!(!key.has_passphrase && !key.backup_verified);
        assert!(key.has_mnemonic());
        assert!(f.take_key(Some(abandon()), false).is_none(), "taken once");
    }

    #[test]
    fn the_pin_is_typed_twice_and_must_match() {
        let mut f = Finish::new();
        let d = |f: &mut Finish, step, s: &str| {
            let mut next = None;
            for c in s.chars() {
                next = f.pin_key(step, KeyInput::Char(c));
            }
            next
        };
        assert_eq!(d(&mut f, FinishStep::Pin, "12"), None);
        assert!(!f.pin_complete());
        assert_eq!(
            f.pin_key(FinishStep::Pin, KeyInput::Done),
            None,
            "too short"
        );
        d(&mut f, FinishStep::Pin, "34");
        assert!(f.pin_complete());
        assert_eq!(
            f.pin_key(FinishStep::Pin, KeyInput::Done),
            Some(FinishStep::PinConfirm)
        );
        assert_eq!(f.pin_len(), 0, "the second entry starts empty");
        d(&mut f, FinishStep::PinConfirm, "1235");
        assert_eq!(
            f.pin_key(FinishStep::PinConfirm, KeyInput::Done),
            Some(FinishStep::Pin),
            "mismatch starts over"
        );
        assert!(f.pin_mismatch());
        assert!(f.take_pin().is_none());
        d(&mut f, FinishStep::Pin, "1234");
        f.pin_key(FinishStep::Pin, KeyInput::Done);
        assert!(!f.pin_mismatch());
        d(&mut f, FinishStep::PinConfirm, "1234");
        assert_eq!(f.pin_key(FinishStep::PinConfirm, KeyInput::Done), None);
        let pin = f.take_pin().expect("matched");
        assert_eq!(pin.len(), 4);
        assert!(f.take_pin().is_none(), "taken once");
        // Back from the PIN steps clears the entries.
        d(&mut f, FinishStep::Pin, "9999");
        assert_eq!(f.back(FinishStep::Pin), Some(FinishStep::Confirm));
        assert_eq!(f.pin_len(), 0);
        assert_eq!(f.back(FinishStep::PinConfirm), Some(FinishStep::Pin));
    }

    #[test]
    fn a_passphrase_goes_through_the_comparison() {
        let mut f = Finish::new();
        let m = abandon();
        assert_eq!(
            f.tap(
                FinishStep::PassphraseOffer,
                ids::LOAD_ADD_PASSPHRASE,
                &Material::Words(&m),
                Network::Mainnet
            ),
            None,
            "the tap only checks"
        );
        assert!(f.offer_add());
        assert_eq!(
            f.tap(
                FinishStep::PassphraseOffer,
                ids::LOAD_PASS_CONTINUE,
                &Material::Words(&m),
                Network::Mainnet
            ),
            Some(FinishStep::Passphrase)
        );
        for c in "TREZOR".chars() {
            assert_eq!(
                f.key(
                    FinishStep::Passphrase,
                    KeyInput::Char(c),
                    &Material::Words(&m),
                    Network::Mainnet,
                    0
                ),
                None
            );
        }
        assert_eq!(
            f.key(
                FinishStep::Passphrase,
                KeyInput::Done,
                &Material::Words(&m),
                Network::Mainnet,
                0
            ),
            Some(FinishStep::PassphraseConfirm)
        );
        assert_eq!(f.fingerprint_plain().unwrap().to_hex(), *b"73c5da0a");
        assert_ne!(f.fingerprint().unwrap().to_hex(), *b"73c5da0a");
        assert!(f.which_passphrase(), "the passphrase key is the default");
        assert_eq!(
            f.tap(
                FinishStep::PassphraseConfirm,
                ids::LOAD_WHICH_CONTINUE,
                &Material::Words(&m),
                Network::Mainnet
            ),
            Some(FinishStep::Confirm)
        );
        // Back unwinds through the comparison and clears the passphrase.
        assert_eq!(
            f.back(FinishStep::Confirm),
            Some(FinishStep::PassphraseConfirm)
        );
        assert_eq!(
            f.back(FinishStep::PassphraseConfirm),
            Some(FinishStep::Passphrase)
        );
        assert!(f.has_passphrase());
        assert_eq!(
            f.back(FinishStep::Passphrase),
            Some(FinishStep::PassphraseOffer)
        );
        assert!(!f.has_passphrase());
        assert_eq!(f.back(FinishStep::PassphraseOffer), None);
    }

    /// "Which key?" is a choice, so the row the check is on is the key
    /// that is added: the plain fingerprint drops the passphrase.
    #[test]
    fn choosing_the_plain_key_drops_the_passphrase() {
        let mut f = Finish::new();
        let m = abandon();
        f.tap(
            FinishStep::PassphraseOffer,
            ids::LOAD_ADD_PASSPHRASE,
            &Material::Words(&m),
            Network::Mainnet,
        );
        f.tap(
            FinishStep::PassphraseOffer,
            ids::LOAD_PASS_CONTINUE,
            &Material::Words(&m),
            Network::Mainnet,
        );
        for c in "TREZOR".chars() {
            f.key(
                FinishStep::Passphrase,
                KeyInput::Char(c),
                &Material::Words(&m),
                Network::Mainnet,
                0,
            );
        }
        f.key(
            FinishStep::Passphrase,
            KeyInput::Done,
            &Material::Words(&m),
            Network::Mainnet,
            0,
        );
        assert_ne!(f.fingerprint().unwrap().to_hex(), *b"73c5da0a");
        assert_eq!(
            f.tap(
                FinishStep::PassphraseConfirm,
                ids::LOAD_WHICH_PLAIN,
                &Material::Words(&m),
                Network::Mainnet
            ),
            None
        );
        assert!(!f.which_passphrase());
        assert_eq!(
            f.tap(
                FinishStep::PassphraseConfirm,
                ids::LOAD_WHICH_CONTINUE,
                &Material::Words(&m),
                Network::Mainnet
            ),
            Some(FinishStep::Confirm)
        );
        assert!(!f.has_passphrase());
        assert_eq!(f.fingerprint().unwrap().to_hex(), *b"73c5da0a");
    }
}
