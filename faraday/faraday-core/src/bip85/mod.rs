//! BIP-85: child seeds, keys, bytes and passwords derived from a loaded
//! key. The applications are [`Bip85App`], and every derivation is `faraday-bip`'s. Each result is a secret: it goes into
//! the open vault by default (a password as an entry, child words as a
//! key, anything else as a note), loads as a key, or leaves unprotected
//! only through the secret sheet (`docs/FLOWS.md` decision 6). The value
//! is derived where it is used and kept nowhere.

pub(crate) mod screen;

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::bip39::Language;
use faraday_bip::bip85;
use faraday_crypto::work::Compute;
use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};
use zeroize::Zeroizing;

use crate::wallet::fp_text;
use crate::{Faraday, Screen, flow};
use faraday_keys::Keys;

/// Which BIP-85 application is being derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bip85App {
    /// Application 39': a child key, which goes to the child flow.
    Words,
    /// Application 2': an HD-seed WIF.
    Wif,
    /// Application 32': an extended private key.
    Xprv,
    /// Application 128169': raw bytes as hex.
    Hex,
    /// Application 707764': a base64 password.
    Base64,
    /// Application 707785': a base85 password.
    Base85,
}

impl Bip85App {
    /// Every application, in the order "Which application?" lists them.
    pub const ALL: [Bip85App; 6] = [
        Bip85App::Words,
        Bip85App::Wif,
        Bip85App::Xprv,
        Bip85App::Hex,
        Bip85App::Base64,
        Bip85App::Base85,
    ];
}

/// The applications, in the order they are listed.
pub const APPS: [Bip85App; 6] = Bip85App::ALL;

/// An application's name.
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

/// The BIP-85 flow's area: its state while it is open.
#[derive(Default)]
pub(crate) struct Bip85Area {
    state: Option<Bip85State>,
}

impl Bip85Area {
    /// The flow's state, while it is open.
    pub(crate) fn state(&self) -> Option<&Bip85State> {
        self.state.as_ref()
    }

    /// Forgets the flow's state.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// The column's scroll, while the flow is open.
    pub(crate) fn scroll_mut(&mut self) -> Option<&mut flow::Scroll> {
        self.state.as_mut().map(|b| &mut b.scroll)
    }

    /// Takes the column's scroll as the pass left it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(b) = self.state.as_mut() {
            b.scroll = to;
        }
    }

    /// Opens the flow on `key` (the only key loaded, if there is one), to
    /// return to `back`.
    pub(crate) fn open(&mut self, key: Option<[u8; 4]>, back: Screen) {
        let (_, _, length) = lengths(Bip85App::Words);
        self.state = Some(Bip85State {
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
            back,
        });
    }

    /// Costs what `child_mnemonic`, `child_wif`, `child_xprv` and the rest
    /// it calls cost. The value the flow derives from `keys`, as text: the
    /// child words, the WIF, the xprv, the hex or the password.
    pub(crate) fn value(
        &self,
        c: &Compute,
        keys: &Keys,
    ) -> Option<faraday_keys::Held<Zeroizing<String>>> {
        let b = self.state.as_ref()?;
        let place = keys.position_of(b.key?)?;
        let i = b.index()?;
        let app = match b.app {
            Bip85App::Words => faraday_keys::bip85::App::Words(b.length),
            Bip85App::Wif => faraday_keys::bip85::App::Wif,
            Bip85App::Xprv => faraday_keys::bip85::App::Xprv,
            Bip85App::Hex => faraday_keys::bip85::App::Hex(b.length),
            Bip85App::Base64 => faraday_keys::bip85::App::Base64(b.length),
            Bip85App::Base85 => faraday_keys::bip85::App::Base85(b.length),
        };
        keys.bip85_child(c, place, app, i, faraday_keys::Reveal::Bip85Child)
    }

    /// What the value is called where it is kept.
    pub(crate) fn title(&self) -> String {
        let Some(b) = self.state.as_ref() else {
            return String::new();
        };
        let fp = b
            .key
            .map(|f| fp_text(faraday_bip::keys::Fingerprint(f)))
            .unwrap_or_default();
        fill(
            EN.bip85_bip_85_index,
            &[
                app_name(b.app),
                &fp.to_string(),
                &b.index().unwrap_or(0).to_string(),
            ],
        )
    }

    /// A press that changes only this flow.
    pub(crate) fn act(&mut self, action: crate::Action) {
        use crate::Action as A;
        let Some(b) = self.state.as_mut() else {
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
                // The value is hidden once its card is left, Back among
                // the ways (F104).
                if b.open != Some(pstep::RESULT) {
                    b.shown = false;
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

    /// Digits typed into the index, on `screen`. Returns whether the key
    /// was taken.
    pub(crate) fn key(&mut self, key: faraday_shell_api::Key, screen: &mut Screen) -> bool {
        use faraday_shell_api::Key as K;
        if *screen != Screen::Bip85 {
            return false;
        }
        let Some(b) = self.state.as_mut() else {
            return false;
        };
        match key {
            K::Enter => self.act(crate::Action::PNext),
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

impl Faraday {
    /// The flow's state, while it is open.
    pub fn bip85(&self) -> Option<&Bip85State> {
        self.bip85.state()
    }

    /// Opens the flow.
    pub(crate) fn bip85_open(&mut self) {
        if self.session.keys.is_empty() {
            self.toast(EN.bip85_load_a_key_first);
            return;
        }
        let key = (self.session.keys.len() == 1).then(|| self.session.keys[0].fingerprint.0);
        self.bip85.open(key, self.screen);
        self.screen = Screen::Bip85;
    }

    /// Costs what `child_mnemonic`, `child_wif`, `child_xprv` and the rest
    /// it calls cost. The value the flow derives, as text.
    pub fn bip85_value(&self, c: &Compute) -> Option<faraday_keys::Held<Zeroizing<String>>> {
        // The demo derives no BIP-85 output (`docs/DEMO.md` §4).
        if !self.makes_secrets() {
            return None;
        }
        self.bip85.value(c, &self.session.keys)
    }

    /// The value into the open vault: a password as an entry, child words
    /// as a key, anything else as a note.
    pub(crate) fn bip85_to_vault(&mut self) {
        let c = self.compute();
        if self.vaults.open.get(self.vaults.current).is_none() {
            self.toast(EN.bip85_no_vault_is_open);
            return;
        }
        let (Some(value), Some(b)) = (self.bip85_value(&c), self.bip85.state()) else {
            return;
        };
        let title = self.bip85.title();
        let path = b.path();
        let record = match b.app {
            Bip85App::Base64 | Bip85App::Base85 => Record::new(kind::ENTRY)
                .with(field::TITLE, title.as_bytes())
                .with(field::PASSWORD, value.as_bytes())
                .with(field::NOTES, path.as_bytes()),
            Bip85App::Words => {
                let Ok(m) = crate::wallet::parse_words(&c, Language::English, &value) else {
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
        self.vault_push(
            record,
            &fill(EN.bip85_is_in_the_vault, &[&title.to_string()]),
        );
    }

    /// Child words load as a key of this session.
    pub(crate) fn bip85_load(&mut self) {
        let c = self.compute();
        if !self.may_load_keys() {
            self.toast(&fill(EN.bip85_remove_the_first, &[self.medium.noun()]));
            return;
        }
        let Some(value) = self.bip85_value(&c) else {
            return;
        };
        let title = self.bip85.title();
        match self.session.keys.add_words(&c, &value, &title, None) {
            Ok(fp) => self.toast(&fill(EN.bip85_key_added, &[&fp_text(fp).to_string()])),
            Err(e) => self.toast(&e.text()),
        }
    }

    /// The value out unprotected, through the secret sheet.
    pub(crate) fn bip85_out(&mut self) {
        let c = self.compute();
        let (Some(value), Some(b)) = (self.bip85_value(&c), self.bip85.state()) else {
            return;
        };
        let fp = b
            .key
            .map(|f| fp_text(faraday_bip::keys::Fingerprint(f)))
            .unwrap_or_default();
        let name = format!(
            "bip85-{fp}-{}-{}.txt",
            app_name(b.app).to_lowercase().replace([' ', '(', ')'], ""),
            b.index().unwrap_or(0)
        );
        let text = Zeroizing::new(format!("{}\n{}\n", b.path(), value.as_str()));
        let what = match b.app {
            Bip85App::Words => EN.bip85_a_child_seed,
            Bip85App::Wif | Bip85App::Xprv => EN.bip85_a_private_key,
            Bip85App::Hex => EN.bip85_secret_bytes,
            Bip85App::Base64 | Bip85App::Base85 => EN.bip85_a_password,
        };
        self.offer_secret(crate::secrets::SecretOut::new(
            name,
            Zeroizing::new(text.as_bytes().to_vec()),
            what,
            EN.bip85_secret_warning,
        ));
    }

    /// One press inside the flow.
    pub(crate) fn bip85_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        match action {
            A::Bip85 => self.bip85_open(),
            A::PVault => self.bip85_to_vault(),
            A::PLoad => self.bip85_load(),
            A::POut => self.bip85_out(),
            _ => self.bip85.act(action),
        }
    }

    /// Digits typed into the index; Escape is Back (F104). Returns
    /// whether the key was taken.
    pub(crate) fn bip85_key(&mut self, key: faraday_shell_api::Key) -> bool {
        if key == faraday_shell_api::Key::Escape
            && self.screen == Screen::Bip85
            && self.bip85.state().is_some()
        {
            self.back();
            return true;
        }
        self.bip85.key(key, &mut self.screen)
    }
}

/// BIP-85's steps, plugged into Back, Forward and the choices that move
/// on (F103, F104).
pub(crate) struct Bip85Steps;

/// The one [`Bip85Steps`].
pub(crate) static BIP85_STEPS: Bip85Steps = Bip85Steps;

impl crate::trail::StepFlow for Bip85Steps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let b = app.bip85.state()?;
        let order = (0..pstep::COUNT as u8).collect();
        Some(crate::trail::Steps::new(order, b.open))
    }

    fn open(&self, _: &Faraday, k: u8) -> crate::Action {
        crate::Action::PStep(k)
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        screen::TITLES[usize::from(k)]
    }

    fn moves_on(&self, app: &Faraday, chosen: crate::Action) -> Option<crate::Action> {
        let b = app.bip85.state()?;
        // An application with no length to set is the card's one choice.
        let one = matches!(chosen, crate::Action::PApp(_))
            && b.open == Some(pstep::APP)
            && lengths(b.app).1 == 0;
        one.then_some(crate::Action::PNext)
    }
}
