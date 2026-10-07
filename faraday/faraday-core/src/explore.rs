//! Explore: a loaded key's public side at any path (`docs/OPENSIGNER-
//! PARITY.md` plan C1). The path is read by OpenSigner's own parser and
//! its purpose names the script, as OpenSigner's Explore does; the
//! extended public key, the public key and the address at the path are
//! derived when drawn. Nothing private is shown or kept.

use opensigner_core::explore::{implied_script, parse_path};
use opensigner_core::strings::EN;
use osk_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use osk_bip::bitcoin::{Address, CompressedPublicKey, NetworkKind};
use osk_bip::keys::ScriptType;

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

impl Faraday {
    /// Opens Explore.
    pub(crate) fn explore_open(&mut self) {
        if self.session.keys.is_empty() {
            return self.toast("Load a key first");
        }
        let coin = if self.session.network().is_mainnet() {
            "0"
        } else {
            "1"
        };
        self.explore = Some(ExploreState {
            key: Some(self.session.keys[0].master.fingerprint().0),
            path: PRESETS[0].1.replace("{c}", coin),
            script: ScriptType::NativeSegwit,
            back: self.screen,
        });
        self.screen = Screen::Explore;
    }

    /// The path read, or why it is not one.
    pub fn explore_path(&self) -> Result<DerivationPath, &'static str> {
        let e = self.explore.as_ref().ok_or("")?;
        let t = e.path.trim().trim_start_matches("m/").replace('\'', "h");
        parse_path(&t).map_err(|p| p.message(&EN))
    }

    /// What the path gives for the chosen key.
    pub fn explore_reading(&self) -> Option<Reading> {
        let e = self.explore.as_ref()?;
        let path = self.explore_path().ok()?;
        let master = &self
            .session
            .keys
            .iter()
            .find(|k| Some(k.master.fingerprint().0) == e.key)?
            .master;
        let xpub = master.derive(&path).to_xpub();
        let pk = CompressedPublicKey(xpub.public_key);
        let net: osk_bip::bitcoin::Network = self.session.network().into();
        let kind = NetworkKind::from(net);
        let script = implied_script(&path).unwrap_or(e.script);
        let secp = osk_bip::bitcoin::secp256k1::Secp256k1::verification_only();
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
    pub(crate) fn explore_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
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
