//! Explore: a loaded key's public side at any path. The path is read by
//! the derivation-path parser and its purpose names the script; the
//! extended public key, the public key and the address at the path are
//! derived when drawn. Nothing private is shown or kept.

use crate::strings::EN;
use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use faraday_bip::bitcoin::{Address, CompressedPublicKey, NetworkKind};
use faraday_bip::keys::ScriptType;
use faraday_crypto::work::Compute;

use crate::{Faraday, Screen};

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

impl Faraday {
    /// Opens Explore.
    pub(crate) fn explore_open(&mut self) {
        if self.session.keys.is_empty() {
            return self.toast(EN.explore_load_a_key);
        }
        let coin = if self.session.network().is_mainnet() {
            "0"
        } else {
            "1"
        };
        self.explore = Some(ExploreState {
            key: Some(self.session.keys[0].fingerprint().0),
            path: PRESETS[0].1.replace("{c}", coin),
            script: ScriptType::NativeSegwit,
            back: self.screen,
            reading: Default::default(),
        });
        self.screen = Screen::Explore;
    }

    /// The path read, or why it is not one.
    pub fn explore_path(&self) -> Result<DerivationPath, &'static str> {
        let e = self.explore.as_ref().ok_or("")?;
        let t = e.path.trim().trim_start_matches("m/").replace('\'', "h");
        parse_path(&t).map_err(|p| p.message(&EN))
    }

    /// Costs a key derivation the first time. What the path gives for the
    /// chosen key: worked out the first time it is asked with these key,
    /// path and script, by whoever asks, a frame included, and kept on
    /// Explore's state, a lookup after.
    pub fn explore_reading(&self, c: &Compute) -> Option<std::rc::Rc<Reading>> {
        let e = self.explore.as_ref()?;
        let of: ReadingOf = (
            e.key,
            e.path.clone(),
            e.script,
            self.session.network(),
            self.session
                .keys
                .iter()
                .any(|k| Some(k.fingerprint().0) == e.key),
        );
        if let Some((was, r)) = e.reading.borrow().as_ref()
            && *was == of
        {
            return r.clone();
        }
        let r = self.explore_reading_now(c).map(std::rc::Rc::new);
        *e.reading.borrow_mut() = Some((of, r.clone()));
        r
    }

    /// Costs a key derivation. Works out [`Faraday::explore_reading`].
    fn explore_reading_now(&self, c: &Compute) -> Option<Reading> {
        let e = self.explore.as_ref()?;
        let path = self.explore_path().ok()?;
        let master = &self
            .session
            .keys
            .iter()
            .find(|k| Some(k.fingerprint().0) == e.key)?
            .master();
        let xpub = master.derive(c, &path).to_xpub(c);
        let pk = CompressedPublicKey(xpub.public_key);
        let net: faraday_bip::bitcoin::Network = self.session.network().into();
        let kind = NetworkKind::from(net);
        let script = implied_script(&path).unwrap_or(e.script);
        let secp = faraday_bip::bitcoin::secp256k1::Secp256k1::verification_only();
        let address = match script {
            ScriptType::NativeSegwit => Address::p2wpkh(&pk, net),
            ScriptType::NestedSegwit => Address::p2shwpkh(&pk, kind),
            ScriptType::Legacy => Address::p2pkh(pk, kind),
            ScriptType::Taproot => Address::p2tr(&secp, pk.0.x_only_public_key().0, None, net),
        };
        Some(Reading {
            xpub: xpub.to_string(),
            public_key: pk.to_string(),
            script,
            address: address.to_string(),
        })
    }

    /// One press inside Explore.
    pub(crate) fn explore_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        if action == A::Explore {
            return self.explore_open();
        }
        let coin = if self.session.network().is_mainnet() {
            "0"
        } else {
            "1"
        };
        let Some(e) = self.explore.as_mut() else {
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

    /// Typing into the path. Returns whether the key was taken.
    pub(crate) fn explore_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        if self.screen != Screen::Explore {
            return false;
        }
        let Some(e) = self.explore.as_mut() else {
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
            K::Escape => {
                self.screen = e.back;
                self.explore = None;
            }
            K::Down => self.explore_act(crate::Action::XIndex(1)),
            K::Up => self.explore_act(crate::Action::XIndex(-1)),
            _ => return false,
        }
        true
    }
}

/// Deepest path Explore derives.
pub const MAX_PATH_LEVELS: usize = 10;

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
