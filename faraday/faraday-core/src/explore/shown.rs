//! Explore's eyes (`docs/KEYS.md` §3, `docs/DESIGN.md` §5): the key's
//! own secret (its words and passphrase, or its master seed and its
//! codex32 strings), its extended private keys (the root's and the
//! account's) and an address's private key, each behind its own eye. One
//! press shows, the next hides; everything is hidden when Explore opens.
//!
//! What an eye shows is made once, at the press that opens it, by the
//! reveal that names it (`Reveal::SeedShown`, `ExtendedPrivateKey`,
//! `AddressKey`), and kept here while it is on screen; drawing reads it
//! and derives nothing. Words and a passphrase are lent from the store
//! each frame (`View::reveal`, a borrow that costs nothing), so only the
//! eye's state is kept for them.

use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use faraday_bip::keys::{Network, WifAscii, XprivAscii};
use faraday_crypto::work::Compute;
use faraday_keys::{Held, Keys, Reveal};
use zeroize::Zeroizing;

use super::addresses::{PAGE, account_of};
use super::{ExploreArea, Page};

/// An eye on Explore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eye {
    /// The key's own secret: its words and passphrase, or its seed.
    Secret,
    /// The root's extended private key and the account's.
    Xprv,
    /// The private key of the address at the path.
    Wif,
    /// The private key of an address on the Addresses page: its chain
    /// (`true` change) and index.
    Row(bool, u32),
}

/// What the eyes show.
///
/// Invariant: everything held is of the key and the network `of` names
/// and is on screen now: the secret and the extended private keys on the
/// path page, these for the account of the path explored; an address's
/// private key for the path explored on the path page, or for an address
/// on the Addresses page as it stands. [`ExploreArea::prune`] drops what
/// is not after every press and key on Explore; closing an eye, leaving
/// Explore, the idle warning and a lock drop it too. Every value wipes
/// itself when dropped.
#[derive(Default)]
pub struct Shown {
    /// The key and the network what is held is of.
    of: Option<([u8; 4], Network)>,
    /// Whether the key's own secret is shown.
    secret: bool,
    /// A key with no words: its seed's forms, made when its eye opened.
    seed: Option<Held<Vec<Zeroizing<String>>>>,
    /// The account path, the root's extended private key and the
    /// account's.
    xprv: Option<(DerivationPath, Held<XprivAscii>, Held<XprivAscii>)>,
    /// Each address's private key shown, by its whole path.
    wifs: Vec<(DerivationPath, Held<WifAscii>)>,
}

impl Shown {
    /// Whether the key's own secret is shown.
    pub fn secret(&self) -> bool {
        self.secret
    }

    /// A key with no words: its seed's forms as shown, the seed in hex
    /// first ([`faraday_keys::Keys::seed_shown`]).
    pub fn seed(&self) -> Option<&[Zeroizing<String>]> {
        self.seed.as_deref().map(Vec::as_slice)
    }

    /// The account path and the root's and the account's extended private
    /// keys, while their eye is open.
    pub fn xprvs(&self) -> Option<(&DerivationPath, &str, &str)> {
        self.xprv
            .as_ref()
            .map(|(p, root, account)| (p, root.as_str(), account.as_str()))
    }

    /// The private key of the address at `path`, while its eye is open.
    pub fn wif(&self, path: &DerivationPath) -> Option<&str> {
        self.wifs
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, w)| w.as_str())
    }

    /// Whether anything is shown.
    pub fn any(&self) -> bool {
        self.secret || self.xprv.is_some() || !self.wifs.is_empty()
    }
}

/// The path whose extended private key the Xprv eye shows beside the
/// root's: the account the path is in, or the path itself where it is in
/// none (a multisig account, a path of another shape).
pub fn xprv_path(path: &DerivationPath) -> DerivationPath {
    account_of(path).unwrap_or_else(|| path.clone())
}

/// The whole path of the address at `change`/`index` of `account`.
pub fn address_path(account: &DerivationPath, change: bool, index: u32) -> DerivationPath {
    let level = |i: u32| ChildNumber::Normal { index: i };
    account.child(level(u32::from(change))).child(level(index))
}

impl ExploreArea {
    /// What the eyes show, while Explore is open.
    pub(crate) fn shown(&self) -> Option<&Shown> {
        self.state.as_ref().map(|e| &e.shown)
    }

    /// Costs a key derivation for an eye that opens on a private key (two
    /// for the extended private keys). A press on an eye: it opens,
    /// making what it shows by its reveal, or closes, dropping it.
    pub(crate) fn eye(&mut self, c: &Compute, keys: &Keys, eye: Eye) {
        let path = self.path().ok();
        let Some(e) = self.state.as_mut() else {
            return;
        };
        let Some(fp) = e.key else {
            return;
        };
        let Some(place) = keys.position_of(fp) else {
            return;
        };
        let s = &mut e.shown;
        if s.of != Some((fp, keys.network())) {
            *s = Shown {
                of: Some((fp, keys.network())),
                ..Shown::default()
            };
        }
        match eye {
            Eye::Secret if s.secret => {
                s.secret = false;
                s.seed = None;
            }
            Eye::Secret => {
                s.secret = true;
                s.seed = keys.seed_shown(place, Reveal::SeedShown);
            }
            Eye::Xprv if s.xprv.is_some() => s.xprv = None,
            Eye::Xprv => {
                let Some(path) = path else {
                    return;
                };
                let at = xprv_path(&path);
                let why = Reveal::ExtendedPrivateKey;
                let root = keys.xprv_at(c, place, &DerivationPath::master(), why);
                let account = keys.xprv_at(c, place, &at, why);
                if let (Some(root), Some(account)) = (root, account) {
                    s.xprv = Some((at, root, account));
                }
            }
            Eye::Wif | Eye::Row(..) => {
                let at = match (eye, path) {
                    (Eye::Row(change, index), Some(p)) => match account_of(&p) {
                        Some(account) => address_path(&account, change, index),
                        None => return,
                    },
                    (_, Some(p)) => p,
                    (_, None) => return,
                };
                if let Some(k) = s.wifs.iter().position(|(p, _)| *p == at) {
                    s.wifs.remove(k);
                } else if let Some(w) = keys.wif_at(c, place, &at, Reveal::AddressKey) {
                    s.wifs.push((at, w));
                }
            }
        }
    }

    /// Drops what the eyes hold that is not on screen now ([`Shown`]'s
    /// invariant): all of it for another key, network or a key no longer
    /// loaded; on the path page what is not of the path explored; on the
    /// Addresses page all but the private keys of the addresses listed;
    /// on Find an address all of it. Run after every press and key on
    /// Explore.
    pub(crate) fn prune(&mut self, keys: &Keys) {
        let path = self.path().ok();
        let Some(e) = self.state.as_mut() else {
            return;
        };
        let s = &mut e.shown;
        let loaded = e.key.filter(|fp| keys.position_of(*fp).is_some());
        if s.of.is_some() && s.of != loaded.map(|fp| (fp, keys.network())) {
            *s = Shown::default();
            return;
        }
        match (e.page, path) {
            (Page::Path, Some(path)) => {
                let at = xprv_path(&path);
                if s.xprv.as_ref().is_some_and(|(p, ..)| *p != at) {
                    s.xprv = None;
                }
                s.wifs.retain(|(p, _)| *p == path);
            }
            (Page::Addresses, Some(path)) => {
                s.secret = false;
                s.seed = None;
                s.xprv = None;
                let Some(account) = account_of(&path) else {
                    s.wifs.clear();
                    return;
                };
                let (change, from) = (e.change, e.from);
                let listed: Vec<DerivationPath> = (from..from.saturating_add(PAGE))
                    .map(|i| address_path(&account, change, i))
                    .collect();
                s.wifs.retain(|(p, _)| listed.contains(p));
            }
            (Page::Path, None) => {
                s.xprv = None;
                s.wifs.clear();
            }
            _ => *s = Shown::default(),
        }
    }

    /// Hides everything the eyes show.
    pub(crate) fn hide(&mut self) {
        if let Some(e) = self.state.as_mut() {
            e.shown = Shown::default();
        }
    }
}
