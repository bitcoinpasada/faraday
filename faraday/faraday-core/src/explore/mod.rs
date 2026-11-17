//! Explore: a loaded key at any path. The path is read by the
//! derivation-path parser and its purpose names the script; the extended
//! public key, the public key and the address at the path are derived
//! when drawn. The key's secret, its extended private keys and an
//! address's private key are each behind an eye, hidden until it is
//! pressed ([`shown`]). The path's account's addresses are listed, and
//! searched for one typed, in [`addresses`] (F76).

pub mod addresses;
pub(crate) mod made;
mod make;
mod pages;
pub(crate) mod screen;
pub mod shown;

pub use addresses::{Page, XAction};
pub use shown::{Eye, Shown};

use crate::strings::EN;
use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use faraday_bip::bitcoin::{Address, CompressedPublicKey, NetworkKind};
use faraday_bip::keys::ScriptType;
use faraday_crypto::work::Compute;

use crate::wallet::{self, Session};
use crate::{Action, Faraday, Screen, Sheet};
use faraday_bip::policy::WalletPolicy;
use faraday_text::{Text, text};
use make::{Offer, kind_of};

/// The presets, by purpose: BIP-84, 86, 49 and 44 receive address 0,
/// and BIP-48's multisig account.
pub const PRESETS: [(&str, &str); 5] = [
    ("BIP-84", "84h/{c}h/0h/0/0"),
    ("BIP-86", "86h/{c}h/0h/0/0"),
    ("BIP-49", "49h/{c}h/0h/0/0"),
    ("BIP-44", "44h/{c}h/0h/0/0"),
    ("BIP-48", "48h/{c}h/0h/2h"),
];

/// What Explore holds: a key, the path as typed, and the script asked
/// for when the path names none.
pub struct ExploreState {
    /// The key, by fingerprint.
    pub key: Option<[u8; 4]>,
    /// The path as typed, without `m/`.
    pub path: String,
    /// The script for a path whose purpose names none.
    pub script: ScriptType,
    /// Where it returns to.
    pub back: Screen,
    /// The page on screen.
    pub page: Page,
    /// The address list's chain: `false` receive, `true` change.
    pub change: bool,
    /// The address list's first index.
    pub from: u32,
    /// Find an address: the text looked for, as typed or pasted.
    pub query: String,
    /// Find an address: how many indices of each chain it looks at.
    pub depth: u32,
    /// Find an address: the search under way or done.
    pub search: Option<addresses::Search>,
    /// Find an address: what became of a paste that was not taken.
    pub note: Option<&'static str>,
    /// Use a sample in the address field (`docs/MODES.md` §6).
    pub(crate) sample: crate::samples::put::Use,
    /// What the eyes show, hidden when Explore opens ([`Shown`]).
    shown: Shown,
    /// The wallet last made at a path here, whose dialog is open
    /// ([`make`]).
    pub(crate) made: Option<usize>,
    /// What a press of **Make a wallet** refused, under that button
    /// until the next press ([`make`]).
    pub(crate) make_error: Option<faraday_text::Text>,
    /// The account's addresses worked out, kept by the account they are
    /// of ([`addresses::Kept`]). Public.
    kept: std::cell::RefCell<addresses::Kept>,
    /// The last reading, with what it was worked out from: the key, the
    /// path, the script, the network and whether the key is loaded. Asked
    /// again with the same, it is a lookup ([`Faraday::explore_reading`]);
    /// anything else works it out afresh, so it is never stale. Public.
    reading: std::cell::RefCell<Option<(ReadingOf, Option<std::rc::Rc<Reading>>)>>,
}

/// What a [`Reading`] is worked out from.
type ReadingOf = (
    Option<[u8; 4]>,
    String,
    ScriptType,
    faraday_bip::keys::Network,
    bool,
);

/// The address of `pk` as `script` pays to it on `net`.
pub(crate) fn address_of(
    pk: &CompressedPublicKey,
    script: ScriptType,
    net: faraday_bip::bitcoin::Network,
) -> Address {
    let kind = NetworkKind::from(net);
    let secp = faraday_bip::bitcoin::secp256k1::Secp256k1::verification_only();
    match script {
        ScriptType::NativeSegwit => Address::p2wpkh(pk, net),
        ScriptType::NestedSegwit => Address::p2shwpkh(pk, kind),
        ScriptType::Legacy => Address::p2pkh(*pk, kind),
        ScriptType::Taproot => Address::p2tr(&secp, pk.0.x_only_public_key().0, None, net),
    }
}

/// What a path gives: the extended public key, the public key and, at a
/// leaf, the address.
pub struct Reading {
    /// `xpub` or `tpub` at the path.
    pub xpub: String,
    /// The compressed public key, hex.
    pub public_key: String,
    /// The script the address is for.
    pub script: ScriptType,
    /// The address.
    pub address: String,
}

/// Explore's area: its state while it is open.
#[derive(Default)]
pub(crate) struct ExploreArea {
    state: Option<ExploreState>,
    /// The key of a wallet just made that is recorded as a BIP-85 child
    /// seed, until its notice has been shown (F166).
    child: Option<[u8; 4]>,
    /// The place of a wallet opened from the made wallet's dialog: the
    /// page stays while that wallet's card is on show, so its Back
    /// returns here.
    held_for: Option<usize>,
}

impl ExploreArea {
    /// Keeps the page while wallet `i`'s card is on show.
    pub(crate) fn hold_for(&mut self, i: usize) {
        self.held_for = Some(i);
    }

    /// Lets the page go with the card.
    pub(crate) fn release(&mut self) {
        self.held_for = None;
    }

    /// The wallet whose card keeps the page, if any.
    pub(crate) fn held_for(&self) -> Option<usize> {
        self.held_for
    }

    /// Explore's state, while it is open.
    pub(crate) fn state(&self) -> Option<&ExploreState> {
        self.state.as_ref()
    }

    /// Sets the address list's first index, for the test kit.
    #[cfg(feature = "testkit")]
    pub(crate) fn set_from(&mut self, from: u32) {
        if let Some(e) = self.state.as_mut() {
            e.from = from;
        }
    }

    /// Replaces the path as typed and returns the one it replaced, for
    /// the test kit.
    #[cfg(feature = "testkit")]
    pub(crate) fn swap_path(&mut self, path: String) -> Option<String> {
        self.state
            .as_mut()
            .map(|e| std::mem::replace(&mut e.path, path))
    }

    /// Forgets Explore's state.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// Opens Explore on key `fp`, at the first preset on coin type
    /// `coin`, to return to `back`.
    pub(crate) fn open(&mut self, fp: [u8; 4], coin: &str, back: Screen) {
        self.state = Some(ExploreState {
            key: Some(fp),
            path: PRESETS[0].1.replace("{c}", coin),
            script: ScriptType::NativeSegwit,
            back,
            reading: Default::default(),
            page: Page::Path,
            change: false,
            from: 0,
            query: String::new(),
            depth: addresses::DEPTHS[0],
            search: None,
            note: None,
            sample: Default::default(),
            made: None,
            make_error: None,
            kept: Default::default(),
            shown: Shown::default(),
        });
    }

    /// The path read, or why it is not one.
    pub(crate) fn path(&self) -> Result<DerivationPath, &'static str> {
        let e = self.state.as_ref().ok_or("")?;
        let t = e.path.trim().trim_start_matches("m/").replace('\'', "h");
        parse_path(&t).map_err(|p| p.message(&EN))
    }

    /// Costs a key derivation the first time. What the path gives for the
    /// chosen key in `session`: worked out the first time it is asked with
    /// these key, path and script, by whoever asks, a frame included, and
    /// kept on Explore's state, a lookup after.
    pub(crate) fn reading(&self, c: &Compute, session: &Session) -> Option<std::rc::Rc<Reading>> {
        let e = self.state.as_ref()?;
        let of: ReadingOf = (
            e.key,
            e.path.clone(),
            e.script,
            session.network(),
            session.keys.all().any(|k| Some(k.fingerprint.0) == e.key),
        );
        if let Some((was, r)) = e.reading.borrow().as_ref()
            && *was == of
        {
            return r.clone();
        }
        let r = self.reading_now(c, session).map(std::rc::Rc::new);
        *e.reading.borrow_mut() = Some((of, r.clone()));
        r
    }

    /// Costs a key derivation. Works out [`ExploreArea::reading`].
    fn reading_now(&self, c: &Compute, session: &Session) -> Option<Reading> {
        let e = self.state.as_ref()?;
        let path = self.path().ok()?;
        let place = session.keys.position_of(e.key?)?;
        let xpub = session.keys.xpub_at(c, place, &path)?;
        let pk = CompressedPublicKey(xpub.public_key);
        let net: faraday_bip::bitcoin::Network = session.network().into();
        let script = implied_script(&path).unwrap_or(e.script);
        let address = address_of(&pk, script, net);
        Some(Reading {
            xpub: xpub.to_string(),
            public_key: pk.to_string(),
            script,
            address: address.to_string(),
        })
    }

    /// A press that changes only the key, the path or the script, on
    /// coin type `coin`.
    pub(crate) fn act(&mut self, action: crate::Action, coin: &str) {
        use crate::Action as A;
        let Some(e) = self.state.as_mut() else {
            return;
        };
        match action {
            A::XKey(fp) => e.key = Some(fp),
            A::XPreset(i) => {
                if let Some((_, p)) = PRESETS.get(usize::from(i)) {
                    e.path = p.replace("{c}", coin);
                }
            }
            A::XScript(i) => {
                if let Some(&s) = ScriptType::ALL.get(usize::from(i)) {
                    e.script = s;
                }
            }
            A::XIndex(d) => {
                // The last level, up or down, kept as it was typed.
                let t = e.path.trim().trim_start_matches("m/").replace('\'', "h");
                if let Ok(path) = parse_path(&t) {
                    let mut levels: Vec<ChildNumber> = path.into_iter().copied().collect();
                    if let Some(last) = levels.last_mut() {
                        let (i, hard) = match *last {
                            ChildNumber::Normal { index } => (index, false),
                            ChildNumber::Hardened { index } => (index, true),
                        };
                        let n = (i64::from(i) + i64::from(d)).clamp(0, (1 << 31) - 1) as u32;
                        *last = if hard {
                            ChildNumber::from_hardened_idx(n).unwrap_or(*last)
                        } else {
                            ChildNumber::from_normal_idx(n).unwrap_or(*last)
                        };
                    }
                    e.path = DerivationPath::from(levels).to_string().replace('\'', "h");
                }
            }
            _ => {}
        }
    }

    /// Typing into the path. Returns whether the key was
    /// taken.
    pub(crate) fn path_key(&mut self, key: faraday_shell_api::Key, coin: &str) -> bool {
        use faraday_shell_api::Key as K;
        let Some(e) = self.state.as_mut() else {
            return false;
        };
        match key {
            K::Char(c) if c.is_ascii_digit() || matches!(c, '/' | 'h' | '\'' | 'm') => {
                if e.path.len() < 120 {
                    e.path.push(c);
                }
            }
            K::Backspace => {
                e.path.pop();
            }
            K::Down => self.act(crate::Action::XIndex(1), coin),
            K::Up => self.act(crate::Action::XIndex(-1), coin),
            _ => return false,
        }
        true
    }
}

impl Faraday {
    /// Explore's state, while it is open.
    pub fn explore(&self) -> Option<&ExploreState> {
        self.explore.state()
    }

    /// The coin type of the session's network, as a path level.
    fn explore_coin(&self) -> &'static str {
        if self.session.network().is_mainnet() {
            "0"
        } else {
            "1"
        }
    }

    /// Opens Explore.
    pub(crate) fn explore_open(&mut self) {
        if self.session.keys.is_empty() {
            return self.toast(EN.explore_load_a_key);
        }
        let fp = self.session.keys[0].fingerprint.0;
        self.explore.open(fp, self.explore_coin(), self.screen);
        self.screen = Screen::Explore;
    }

    /// The path read, or why it is not one.
    pub fn explore_path(&self) -> Result<DerivationPath, &'static str> {
        self.explore.path()
    }

    /// Costs a key derivation the first time. What the path gives for the
    /// chosen key, kept on Explore's state.
    pub fn explore_reading(&self, c: &Compute) -> Option<std::rc::Rc<Reading>> {
        self.explore.reading(c, &self.session)
    }

    /// What Explore's eyes show, while it is open.
    pub fn explore_shown(&self) -> Option<&Shown> {
        self.explore.shown()
    }

    /// One press inside Explore. What the eyes hold that the press took
    /// off screen is dropped after it ([`ExploreArea::prune`]).
    pub(crate) fn explore_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        self.explore.clear_make_error();
        match action {
            A::Explore => return self.explore_open(),
            A::X(XAction::Eye(eye)) => {
                let c = self.compute();
                self.explore.eye(&c, &self.session.keys, eye);
            }
            A::X(x) => self.explore_x(x),
            _ => self.explore.act(action, self.explore_coin()),
        }
        self.explore.prune(&self.session.keys);
    }

    /// Hides everything Explore's eyes show: the idle warning, so that
    /// nothing secret stays on screen while no one is there.
    pub(crate) fn explore_hide(&mut self) {
        self.explore.hide();
    }

    /// Forgets Explore's state, what its eyes show with it, once it is
    /// not on screen: leaving it, however it is left, but for Convert a
    /// seed opened from it, and Add a key or the aezeed's entry opened from
    /// that, which come back.
    pub(crate) fn explore_left(&mut self) {
        let comes_back = crate::convert::state(self).is_some_and(|s| s.back == Screen::Explore);
        let away = comes_back
            && (self.screen == Screen::Convert
                || (matches!(self.screen, Screen::KeyGen | Screen::Randomness)
                    && self
                        .keygen
                        .state()
                        .is_some_and(|k| k.back == Screen::Convert))
                || (self.screen == Screen::Lightning && self.lightning.for_convert()));
        let held = self.explore.held_for().is_some_and(|i| self.card_open(i));
        if !held {
            self.explore.release();
        }
        if self.screen != Screen::Explore && !away && !held {
            self.explore.clear();
        }
    }

    /// Typing into the path. Returns whether the key was taken.
    pub(crate) fn explore_key(&mut self, key: faraday_shell_api::Key) -> bool {
        if self.screen != Screen::Explore {
            return false;
        }
        let taken = match self.explore.state().map(|e| e.page) {
            Some(Page::Path) => {
                let coin = self.explore_coin();
                self.explore.path_key(key, coin)
            }
            Some(_) => self.explore_page_key(key),
            None => return false,
        };
        self.explore.prune(&self.session.keys);
        taken
    }
}

/// Deepest path Explore derives.
pub(crate) const MAX_PATH_LEVELS: usize = 10;

/// Why the path text does not parse. The last valid path stays applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// Two slashes in a row, or a slash first: `m//0`.
    EmptyLevel,
    /// A level ends with a slash and nothing after it.
    Unfinished,
    /// A level is not digits with an optional `h`.
    NotANumber,
    /// An index of 2³¹ or more.
    IndexTooLarge,
    /// More than [`MAX_PATH_LEVELS`] levels.
    TooDeep,
}

impl PathError {
    /// The inline message under the path field.
    pub fn message(self, s: &crate::strings::Strings) -> &'static str {
        match self {
            PathError::EmptyLevel => s.explore_path_empty,
            PathError::Unfinished => s.explore_path_unfinished,
            PathError::NotANumber => s.explore_path_not_a_number,
            PathError::IndexTooLarge => s.explore_path_too_large,
            PathError::TooDeep => s.explore_path_too_deep,
        }
    }
}

/// Parses path text without its `m/` prefix into a derivation path.
/// Empty text is the master itself.
pub fn parse_path(text: &str) -> Result<DerivationPath, PathError> {
    let mut levels = [ChildNumber::Normal { index: 0 }; MAX_PATH_LEVELS];
    let mut n = 0usize;
    if text.is_empty() {
        return Ok(DerivationPath::from(&levels[..0]));
    }
    for (i, level) in text.split('/').enumerate() {
        if level.is_empty() {
            return Err(if i > 0 && i + 1 == text.split('/').count() {
                PathError::Unfinished
            } else {
                PathError::EmptyLevel
            });
        }
        if n == MAX_PATH_LEVELS {
            return Err(PathError::TooDeep);
        }
        let (digits, hardened) = match level.strip_suffix('h') {
            Some(d) => (d, true),
            None => (level, false),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(PathError::NotANumber);
        }
        let index: u32 = digits.parse().map_err(|_| PathError::IndexTooLarge)?;
        levels[n] = if hardened {
            ChildNumber::from_hardened_idx(index).map_err(|_| PathError::IndexTooLarge)?
        } else {
            ChildNumber::from_normal_idx(index).map_err(|_| PathError::IndexTooLarge)?
        };
        n += 1;
    }
    Ok(DerivationPath::from(&levels[..n]))
}

/// The script type a path's purpose level implies, when it is one of
/// BIP-44/49/84/86 and hardened.
pub fn implied_script(path: &DerivationPath) -> Option<ScriptType> {
    match path.as_ref().first()? {
        ChildNumber::Hardened { index } => ScriptType::ALL
            .iter()
            .copied()
            .find(|s| s.purpose() == *index),
        ChildNumber::Normal { .. } => None,
    }
}

impl Faraday {
    /// What the explored path's account is offered as (`Offer`), for the
    /// pages to draw.
    pub(crate) fn explore_make_offer(&self) -> Option<Offer> {
        self.explore.make_offer(&self.session)
    }

    /// A press of **Make a wallet** or **Open the wallet** on Explore.
    pub(crate) fn explore_make(&mut self) {
        let Some(offer) = self.explore_make_offer() else {
            return;
        };
        let Some((Some(fp), account, script, ..)) = self.explore_account() else {
            return;
        };
        match offer {
            Offer::Open(i) => self.act(Action::OpenWallet(i)),
            Offer::Make => {
                let c = self.compute();
                let made = self.make_wallet_at(&c, fp, &account, script);
                if made.is_ok() {
                    self.sheet = Some(Sheet::MadeWallet);
                }
                self.explore.set_made(made);
            }
        }
    }

    /// The wallet the Wallet made dialog is about: Explore's, or the
    /// address check's find made into one.
    pub(crate) fn made_place(&self) -> Option<usize> {
        if self.screen == Screen::CheckAddress {
            self.address_check_state().and_then(|s| s.made())
        } else {
            self.explore.state().and_then(|e| e.made)
        }
    }

    /// **Done** on the made wallet's dialog; a wallet made with a key
    /// recorded as a BIP-85 child seed then shows its notice, once.
    pub(crate) fn explore_made_done(&mut self) {
        if self.sheet == Some(Sheet::MadeWallet) {
            self.sheet = self.explore.take_child().map(|_| Sheet::ChildNotice);
        }
    }

    /// Whether wallet `i`'s card is on show.
    pub(crate) fn card_open(&self, i: usize) -> bool {
        self.screen == Screen::Wallets && self.wallets.wallet() == i
    }

    /// **Open the wallet** on the made wallet's dialog.
    pub(crate) fn explore_made_open(&mut self) {
        let made = self.made_place();
        let from = self.screen;
        self.explore_made_done();
        if let Some(i) = made {
            // Back from the card returns to the page it was made on.
            match from {
                Screen::Explore => self.explore.hold_for(i),
                Screen::CheckAddress => self.tools.address_check.hold_for(i),
                _ => {}
            }
            self.trail.set_returning();
            self.act(Action::OpenWallet(i));
        }
    }

    /// Costs a key derivation for each step of the account's path, a
    /// descriptor parse and two address derivations. Makes the wallet
    /// Create makes for the loaded key `fp` at `account` under `script`
    /// (a standard single-key account), named after the key and the
    /// account, once its first address is the one the key gives at
    /// `account/0/0`; the wallet's place, or why none was made. The
    /// address check's result card calls it for the account it found.
    pub(crate) fn make_wallet_at(
        &mut self,
        c: &Compute,
        fp: [u8; 4],
        account: &DerivationPath,
        script: ScriptType,
    ) -> Result<usize, Text> {
        let not_loaded = || Text::of(EN.explore_load_a_key);
        let place = self.session.keys.position_of(fp).ok_or_else(not_loaded)?;
        let key_name = self
            .session
            .keys
            .info(place)
            .map(|k| match k.label.trim() {
                // A child seed's mark stays in the vault (F166): the
                // name is the label, or the fingerprint.
                "" => wallet::fp_text(k.fingerprint),
                l => Text::of(l),
            })
            .unwrap_or_default();
        let net = self.session.network();
        let key = self
            .session
            .keys
            .key_text(c, place, &faraday_keys::Spec::Path(account.clone()))
            .ok_or_else(not_loaded)??;
        let descriptor = kind_of(script).descriptor(1, &[key.as_str()]);
        let path = Text::replaced(&text!("m/{account}"), "'", "h");
        // The first address is checked before the wallet is kept.
        let policy = WalletPolicy::parse_any(c, descriptor.as_str())
            .map_err(|e| faraday_strings::bip::policy_error(&e))?;
        let secp = faraday_bip::bitcoin::secp256k1::Secp256k1::verification_only();
        let made = crate::address_find::address(c, &secp, &policy, net, false, 0);
        let mut leaf: Vec<ChildNumber> = account.as_ref().to_vec();
        leaf.extend([ChildNumber::Normal { index: 0 }; 2]);
        let gives = self
            .session
            .keys
            .xpub_at(c, place, &DerivationPath::from(leaf))
            .map(|x| address_of(&CompressedPublicKey(x.public_key), script, net.into()));
        if made.is_none() || made != gives {
            return Err(Text::fill(EN.made_wallet_differs, &[&path]));
        }
        let name = Text::fill(EN.made_wallet_name, &[&key_name, &path]);
        let i = self
            .session
            .add_wallet(
                c,
                &name,
                &descriptor,
                EN.create_make_made_here,
                wallet::Origin::Statement,
            )
            .map_err(|e| e.text())?;
        self.glance.moved_to(&self.session, i);
        self.wallets.set_wallet(i);
        let child = self
            .session
            .keys
            .info(place)
            .is_some_and(|k| k.child_of.is_some());
        self.explore.set_child(child.then_some(fp));
        Ok(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_parse_and_reject() {
        let p = parse_path("84h/0h/0h/0/0").unwrap();
        assert_eq!(p.len(), 5);
        assert_eq!(implied_script(&p), Some(ScriptType::NativeSegwit));
        assert_eq!(parse_path("").unwrap().len(), 0);
        assert_eq!(implied_script(&parse_path("").unwrap()), None);
        assert_eq!(implied_script(&parse_path("84/0").unwrap()), None);
        assert_eq!(implied_script(&parse_path("45h").unwrap()), None);
        assert_eq!(parse_path("/0"), Err(PathError::EmptyLevel));
        assert_eq!(parse_path("84h//0"), Err(PathError::EmptyLevel));
        assert_eq!(parse_path("84h/"), Err(PathError::Unfinished));
        assert_eq!(parse_path("2147483648"), Err(PathError::IndexTooLarge));
        assert_eq!(parse_path("2147483647h").unwrap().len(), 1);
        assert_eq!(parse_path("hh"), Err(PathError::NotANumber));
        assert_eq!(parse_path("h"), Err(PathError::NotANumber));
        assert_eq!(parse_path("0/0/0/0/0/0/0/0/0/0/0"), Err(PathError::TooDeep));
    }
}
