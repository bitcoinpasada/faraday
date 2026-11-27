//! BIP-85 children: a value derived from a master key and an index.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use faraday_bip::bip39::Language;
use faraday_bip::bip85;
use faraday_bip::keys::MasterKey;
use faraday_crypto::work::Compute;
use faraday_text::Text;

/// Which application's value a child is, with the length it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    /// Application 39': English words, 12, 18 or 24 of them.
    Words(usize),
    /// Application 2': an HD-seed WIF.
    Wif,
    /// Application 32': an extended private key.
    Xprv,
    /// Application 128169': this many bytes, as hex.
    Hex(usize),
    /// Application 707764': a base64 password of this length.
    Base64(usize),
    /// Application 707785': a base85 password of this length.
    Base85(usize),
}

/// Costs what `bip85::child_mnemonic`, `child_wif`, `child_xprv` and the
/// rest cost. The value of `app` at `index` below `master`, as text: the
/// child words, the WIF, the xprv, the hex or the password; `None` when
/// BIP-85 refuses the length or the index.
pub fn child(c: &Compute, master: &MasterKey, app: App, index: u32) -> Option<Text> {
    Some(match app {
        App::Words(length) => {
            let m = bip85::child_mnemonic(c, master, Language::English, length, index).ok()?;
            let mut words = Text::new();
            for (n, &w) in m.indices().iter().enumerate() {
                if n > 0 {
                    words.push(' ');
                }
                words.push_str(Language::English.word(w));
            }
            words
        }
        App::Wif => crate::texts::wiped(bip85::child_wif(c, master, index).ok()?.as_str()),
        App::Xprv => crate::texts::wiped(bip85::child_xprv(c, master, index).ok()?.as_str()),
        App::Hex(length) => faraday_crypto::hex::encode_secret(
            bip85::child_hex(c, master, length, index).ok()?.as_bytes(),
        ),
        App::Base64(length) => crate::texts::wiped(
            bip85::child_password_base64(c, master, length, index)
                .ok()?
                .as_str(),
        ),
        App::Base85(length) => crate::texts::wiped(
            bip85::child_password_base85(c, master, length, index)
                .ok()?
                .as_str(),
        ),
    })
}
