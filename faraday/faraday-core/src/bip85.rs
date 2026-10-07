//! BIP-85: child seeds, keys, bytes and passwords derived from a loaded
//! key (`docs/OPENSIGNER-PARITY.md` plan B4). The applications and their
//! names are OpenSigner's (`opensigner_core::Bip85App`, its strings), and
//! every derivation is `osk-bip`'s. Each result is a secret: it goes into
//! the open vault by default (a password as an entry, child words as a
//! key, anything else as a note), loads as a key, or leaves unprotected
//! only through the secret sheet (`docs/FLOWS.md` decision 6). The value
//! is derived where it is used and kept nowhere.

use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};
use opensigner_core::Bip85App;
use opensigner_core::strings::EN;
use osk_bip::bip39::Language;
use osk_bip::bip85;
use zeroize::Zeroizing;

use crate::wallet::fp_text;
use crate::{Faraday, Screen, flow};

/// The applications, in OpenSigner's order.
pub const APPS: [Bip85App; 6] = Bip85App::ALL;

/// An application's name, as OpenSigner writes it.
pub fn app_name(app: Bip85App) -> &'static str {
    match app {
        Bip85App::Words => EN.bip85_app_words,
        Bip85App::Wif => EN.bip85_app_wif,
        Bip85App::Xprv => EN.bip85_app_xprv,
        Bip85App::Hex => EN.bip85_app_hex,
        Bip85App::Base64 => EN.bip85_app_base64,
        Bip85App::Base85 => EN.bip85_app_base85,
    }
}

/// The cards of the flow.
pub mod pstep {
    /// Which loaded key.
    pub const KEY: u8 = 0;
    /// Which application, and its length.
    pub const APP: u8 = 1;
    /// The index.
    pub const INDEX: u8 = 2;
    /// The value, and where it goes.
    pub const RESULT: u8 = 3;
    /// How many cards.
    pub const COUNT: usize = 4;
}

/// The flow's state: a key, an application, two numbers. None of it is a
/// secret; the value is derived from it when needed.
pub struct Bip85State {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; pstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The key, by its fingerprint.
    pub key: Option<[u8; 4]>,
    /// The application.
    pub app: Bip85App,
    /// Words for the child seed, bytes for hex, characters for a password.
    pub length: usize,
    /// The index, as typed.
    pub index: String,
    /// The value is on screen.
    pub shown: bool,
    /// Where the flow returns to.
    pub back: Screen,
}

/// The lengths an application takes, and the one it starts with (BIP-85's
/// own vectors: 21 characters of base64, 12 of base85).
pub fn lengths(app: Bip85App) -> (usize, usize, usize) {
    match app {
        Bip85App::Words => (12, 24, 24),
        Bip85App::Hex => (bip85::HEX_BYTES.0, bip85::HEX_BYTES.1, 32),
        Bip85App::Base64 => (bip85::BASE64_LENGTHS.0, bip85::BASE64_LENGTHS.1, 21),
        Bip85App::Base85 => (bip85::BASE85_LENGTHS.0, bip85::BASE85_LENGTHS.1, 12),
        Bip85App::Wif | Bip85App::Xprv => (0, 0, 0),
    }
}

impl Bip85State {
    /// The index as a number, when it is one BIP-85 takes.
    pub fn index(&self) -> Option<u32> {
        self.index
            .parse::<u32>()
            .ok()
            .filter(|i| *i <= bip85::MAX_INDEX)
    }

    /// The derivation path, as BIP-85 writes it.
    pub fn path(&self) -> String {
        let i = self.index().unwrap_or(0);
        match self.app {
            Bip85App::Words => format!("m/83696968'/39'/0'/{}'/{i}'", self.length),
            Bip85App::Wif => format!("m/83696968'/2'/{i}'"),
            Bip85App::Xprv => format!("m/83696968'/32'/{i}'"),
            Bip85App::Hex => format!("m/83696968'/128169'/{}'/{i}'", self.length),
            Bip85App::Base64 => format!("m/83696968'/707764'/{}'/{i}'", self.length),
            Bip85App::Base85 => format!("m/83696968'/707785'/{}'/{i}'", self.length),
        }
    }
}

impl Faraday {
    /// Opens the flow.
    pub(crate) fn bip85_open(&mut self) {
        if self.session.keys.is_empty() {
            self.toast("Load a key first");
            return;
        }
        let key =
            (self.session.keys.len() == 1).then(|| self.session.keys[0].master.fingerprint().0);
        let (_, _, length) = lengths(Bip85App::Words);
        self.bip85 = Some(Bip85State {
            open: Some(if key.is_some() {
                pstep::APP
            } else {
                pstep::KEY
            }),
            done: [key.is_some(), false, false, false],
            scroll: flow::Scroll::default(),
            key,
            app: Bip85App::Words,
            length,
            index: "0".to_string(),
            shown: false,
            back: self.screen,
        });
        self.screen = Screen::Bip85;
    }

    /// The value the flow derives, as text: the child words, the WIF, the
    /// xprv, the hex or the password.
    pub fn bip85_value(&self) -> Option<Zeroizing<String>> {
        let b = self.bip85.as_ref()?;
        let master = &self
            .session
            .keys
            .iter()
            .find(|k| Some(k.master.fingerprint().0) == b.key)?
            .master;
        let i = b.index()?;
        let text = match b.app {
            Bip85App::Words => {
                let m = bip85::child_mnemonic(master, Language::English, b.length, i).ok()?;
                let words: Vec<&str> = m
                    .indices()
                    .iter()
                    .map(|&w| Language::English.word(w))
                    .collect();
                words.join(" ")
            }
            Bip85App::Wif => bip85::child_wif(master, i).ok()?.as_str().to_string(),
            Bip85App::Xprv => bip85::child_xprv(master, i).ok()?.as_str().to_string(),
            Bip85App::Hex => bip85::child_hex(master, b.length, i)
                .ok()?
                .as_bytes()
                .iter()
                .map(|x| format!("{x:02x}"))
                .collect(),
            Bip85App::Base64 => bip85::child_password_base64(master, b.length, i)
                .ok()?
                .as_str()
                .to_string(),
            Bip85App::Base85 => bip85::child_password_base85(master, b.length, i)
                .ok()?
                .as_str()
                .to_string(),
        };
        Some(Zeroizing::new(text))
    }

    /// What the value is called where it is kept.
    fn bip85_title(&self) -> String {
        let Some(b) = self.bip85.as_ref() else {
            return String::new();
        };
        let fp = b
            .key
            .map(|f| fp_text(osk_bip::keys::Fingerprint(f)))
            .unwrap_or_default();
        format!(
            "BIP-85 {} · {fp} · index {}",
            app_name(b.app),
            b.index().unwrap_or(0)
        )
    }

    /// The value into the open vault: a password as an entry, child words
    /// as a key, anything else as a note.
    pub(crate) fn bip85_to_vault(&mut self) {
        if self.vaults.open.get(self.vaults.current).is_none() {
            self.toast("No vault is open");
            return;
        }
        let (Some(value), Some(b)) = (self.bip85_value(), self.bip85.as_ref()) else {
            return;
        };
        let title = self.bip85_title();
        let path = b.path();
        let record = match b.app {
            Bip85App::Base64 | Bip85App::Base85 => Record::new(kind::ENTRY)
                .with(field::TITLE, title.as_bytes())
                .with(field::PASSWORD, value.as_bytes())
                .with(field::NOTES, path.as_bytes()),
            Bip85App::Words => {
                let Ok(m) = osk_bip::bip39::Mnemonic::parse(Language::English, &value) else {
                    return;
                };
                Record::new(kind::KEY)
                    .with(field::KEY, &records::words_payload(&m))
                    .with(field::KEY_LABEL, title.as_bytes())
            }
            _ => {
                let note = Zeroizing::new(format!("{title}\n{path}\n{}", value.as_str()));
                Record::new(kind::NOTE).with(field::NOTE, note.as_bytes())
            }
        };
        self.vault_push(record, &format!("{title} is in the vault"));
    }

    /// Child words load as a key of this session.
    pub(crate) fn bip85_load(&mut self) {
        if !self.may_load_keys() {
            self.toast("Remove the stick first");
            return;
        }
        let Some(value) = self.bip85_value() else {
            return;
        };
        let title = self.bip85_title();
        match self.session.add_words(&value, &title, None) {
            Ok(fp) => self.toast(&format!("Key {} added", fp_text(fp))),
            Err(e) => self.toast(&e.text()),
        }
    }

    /// The value out unprotected, through the secret sheet.
    pub(crate) fn bip85_out(&mut self) {
        let (Some(value), Some(b)) = (self.bip85_value(), self.bip85.as_ref()) else {
            return;
        };
        let fp = b
            .key
            .map(|f| fp_text(osk_bip::keys::Fingerprint(f)))
            .unwrap_or_default();
        let name = format!(
            "bip85-{fp}-{}-{}.txt",
            app_name(b.app).to_lowercase().replace([' ', '(', ')'], ""),
            b.index().unwrap_or(0)
        );
        let text = Zeroizing::new(format!("{}\n{}\n", b.path(), value.as_str()));
        let what = match b.app {
            Bip85App::Words => "A child seed's words",
            Bip85App::Wif | Bip85App::Xprv => "A private key",
            Bip85App::Hex => "Secret bytes",
            Bip85App::Base64 | Bip85App::Base85 => "A password",
        };
        self.offer_secret(crate::secrets::SecretOut {
            name,
            bytes: Zeroizing::new(text.as_bytes().to_vec()),
            what,
            gives: "Whoever has it has what it protects or spends",
            round: None,
        });
    }

    /// One press inside the flow.
    pub(crate) fn bip85_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        match action {
            A::Bip85 => return self.bip85_open(),
            A::PVault => return self.bip85_to_vault(),
            A::PLoad => return self.bip85_load(),
            A::POut => return self.bip85_out(),
            _ => {}
        }
        let Some(b) = self.bip85.as_mut() else {
            return;
        };
        let open = |b: &mut Bip85State, s: u8| {
            b.open = Some(s);
            b.scroll.follow = true;
        };
        match action {
            A::PStep(s) => {
                if b.open == Some(s) {
                    b.open = None;
                } else if (0..s).all(|i| b.done[usize::from(i)]) {
                    open(b, s);
                }
            }
            A::PKey(fp) => {
                b.key = Some(fp);
                b.shown = false;
                b.done[usize::from(pstep::KEY)] = true;
                open(b, pstep::APP);
            }
            A::PApp(i) => {
                if let Some(&app) = APPS.get(usize::from(i)) {
                    b.app = app;
                    b.length = lengths(app).2;
                    b.shown = false;
                }
            }
            A::PLength(delta) => {
                let (lo, hi, _) = lengths(b.app);
                let step = if b.app == Bip85App::Words { 3 } else { 1 };
                let next = b.length as i64 + i64::from(delta) * step;
                b.length = (next.max(lo as i64) as usize).min(hi);
                b.shown = false;
            }
            A::PIndex(delta) => {
                let i = i64::from(b.index().unwrap_or(0)) + i64::from(delta);
                b.index = i.clamp(0, i64::from(bip85::MAX_INDEX)).to_string();
                b.shown = false;
            }
            A::PNext => {
                if let Some(s) = b.open {
                    if s == pstep::INDEX && b.index().is_none() {
                        return;
                    }
                    b.done[usize::from(s)] = true;
                    if s < pstep::RESULT {
                        open(b, s + 1);
                    }
                }
            }
            A::PShow => b.shown = !b.shown,
            _ => {}
        }
    }

    /// Digits typed into the index. Returns whether the key was taken.
    pub(crate) fn bip85_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
        if self.screen != Screen::Bip85 {
            return false;
        }
        let Some(b) = self.bip85.as_mut() else {
            return false;
        };
        match key {
            K::Escape => {
                self.screen = b.back;
                self.bip85 = None;
            }
            K::Enter => self.bip85_act(crate::Action::PNext),
            K::Char(c) if c.is_ascii_digit() && b.open == Some(pstep::INDEX) => {
                if b.index == "0" {
                    b.index.clear();
                }
                if b.index.len() < 10 {
                    b.index.push(c);
                }
                b.shown = false;
            }
            K::Backspace if b.open == Some(pstep::INDEX) => {
                b.index.pop();
                b.shown = false;
            }
            _ => return false,
        }
        true
    }
}
