//! The test harness shared by the flow tests: an [`OpenSigner`] on a
//! display, driven by ids.
#![allow(dead_code)]

use opensigner_core::ids;
use opensigner_core::load::Step;
use opensigner_core::{AssuranceTier, BuildInfo, OpenSigner, ScreenKind};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{Fingerprint, MasterKey, Network, ScriptType};
use osk_codec::qr::Ecc;
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, Key, SecureHardware, TouchPhase,
};
use osk_ui::Id;
use osk_ui::widgets::keyboard::KeyInput;
use osk_ui::widgets::tokens::HOLD_MS;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// The 2.8" reference panel, portrait (UX.md §2).
pub const PANEL: DisplayInfo = DisplayInfo {
    width: 480,
    height: 640,
    dpi: 286,
    inset_bottom: 0,
    inset_top: 0,
    buttons: 1,
    camera_fixed: false,
    secure: SecureHardware::None,
    boot: BootState::Unknown,
    memory_mib: None,
};

/// The smallest supported panel, portrait.
pub const TINY: DisplayInfo = DisplayInfo {
    width: 240,
    height: 320,
    dpi: 143,
    inset_bottom: 0,
    inset_top: 0,
    buttons: 1,
    camera_fixed: false,
    secure: SecureHardware::None,
    boot: BootState::Unknown,
    memory_mib: None,
};

pub const PHONE: DisplayInfo = DisplayInfo {
    width: 1080,
    height: 2340,
    dpi: 420,
    inset_bottom: 0,
    inset_top: 0,
    buttons: 0,
    camera_fixed: false,
    secure: SecureHardware::None,
    boot: BootState::Unknown,
    memory_mib: None,
};

/// A phone with a secure element, which is the only place a key can be
/// kept on the device (`keep.rs`).
pub const SECURE_PHONE: DisplayInfo = DisplayInfo {
    secure: SecureHardware::StrongBox,
    ..PHONE
};

/// The same phone, reporting the boot the platform vouched for.
pub const VERIFIED_PHONE: DisplayInfo = DisplayInfo {
    boot: BootState::Verified,
    memory_mib: None,
    ..SECURE_PHONE
};

/// The same phone with its bootloader unlocked: the element still holds
/// a key, and the system that asks it for one is nobody's.
pub const UNVERIFIED_PHONE: DisplayInfo = DisplayInfo {
    boot: BootState::Unverified,
    memory_mib: None,
    ..SECURE_PHONE
};

/// The desktop window: one fixed size, the `wide` size class (UX.md §2).
pub const DESKTOP: DisplayInfo = DisplayInfo {
    width: 960,
    height: 640,
    dpi: 160,
    inset_bottom: 0,
    inset_top: 0,
    buttons: 0,
    camera_fixed: false,
    secure: SecureHardware::None,
    boot: BootState::Unknown,
    memory_mib: None,
};

pub const ABANDON: [&str; 12] = [
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
    "abandon", "abandon", "abandon", "about",
];

/// The session PIN the harness sets for the first key.
pub const PIN: &str = "2580";

/// The entropy the harness answers requests with; each answer differs
/// from the last in its first byte, as a real RNG would, so that a
/// rotation actually changes the key.
pub const ENTROPY: [u8; 32] = [0x5a; 32];

/// A shell's secure element and the one blob it keeps, as the Android
/// shell will hold them: a key that never leaves it, and bytes wrapped
/// under a key of its own. Copying the blob to another device is
/// copying it to another `key`.
#[derive(Clone)]
pub struct Element {
    /// The key the element authenticates under. A different one is a
    /// different device.
    pub key: [u8; 32],
    /// What the shell is keeping.
    pub blob: Option<Vec<u8>>,
    /// Whether the element answers at all: `false` is a cancelled
    /// authentication or a key the chip has deleted.
    pub available: bool,
    /// Whether the shell can keep bytes at all.
    pub storable: bool,
}

impl Default for Element {
    fn default() -> Self {
        Element {
            key: [0x3c; 32],
            blob: None,
            available: true,
            storable: true,
        }
    }
}

impl Element {
    /// HMAC-SHA256 of `salt` under the element's key, which is all the
    /// core ever learns from it.
    pub fn mac(&self, salt: &[u8; 32]) -> [u8; 32] {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(salt);
        mac.finalize().into_bytes().into()
    }
}

pub struct Harness {
    pub app: OpenSigner,
    pub now_ms: u64,
    /// Answer every `RequestEntropy` with [`ENTROPY`] (a shell with an
    /// RNG); off for the weak-session tests.
    pub answer_entropy: bool,
    /// Commands seen by [`answer`](Self::answer) and kept for the next
    /// [`drain`](Self::drain).
    pending: Vec<Command>,
    /// Entropy answers given so far.
    answers: u8,
    /// The shell's secure element, on a shell that has one.
    pub element: Option<Element>,
    /// Every command the core has produced, in order. The harness
    /// answers most of them as it goes, so this is how a test sees what
    /// a tap asked the shell for. Clear it to start a fresh window.
    pub seen: Vec<Command>,
}

impl Harness {
    pub fn new(display: DisplayInfo) -> Self {
        Self::with_tier(display, AssuranceTier::C, true)
    }

    /// A shell that never answers entropy requests.
    pub fn without_entropy(display: DisplayInfo) -> Self {
        Self::with_tier(display, AssuranceTier::C, false)
    }

    /// A Tier B shell with a secure element and whatever `element` is
    /// already keeping, which is what a restart looks like.
    pub fn kept(display: DisplayInfo, element: Element) -> Self {
        let kept = element.blob.is_some();
        let mut h = Self::with_tier(display, AssuranceTier::B, true);
        h.element = Some(element);
        h.send(Event::SecretKept { kept });
        h
    }

    /// A device that has never run: no settings file at all, so the
    /// first run's document is the screen it opens on.
    pub fn first_run(display: DisplayInfo) -> Self {
        Self::fresh(display, AssuranceTier::C, true)
    }

    /// The same, on any tier.
    pub fn fresh(display: DisplayInfo, tier: AssuranceTier, answer_entropy: bool) -> Self {
        Self::build(display, tier, answer_entropy, false, osk_backup::MIN_PARAMS)
    }

    pub fn with_tier(display: DisplayInfo, tier: AssuranceTier, answer_entropy: bool) -> Self {
        Self::build(display, tier, answer_entropy, true, osk_backup::MIN_PARAMS)
    }

    /// A device that stretches at the cost a device really stretches
    /// at, for the tests whose subject is that cost: what a written
    /// file's header states, and the memory a reader is asked for.
    pub fn at_device_cost(display: DisplayInfo) -> Self {
        Self::build(
            display,
            AssuranceTier::C,
            true,
            true,
            osk_backup::DEVICE_PARAMS,
        )
    }

    /// `first_run_done` is what a shell that has kept settings hands
    /// back; every harness but [`fresh`](Self::fresh) is a device whose
    /// first run is behind it, which is what the rest of the tests are
    /// about.
    ///
    /// `cost` is the Argon2id cost this device writes the kept-key blob
    /// and its encrypted exports at. Every harness but
    /// [`at_device_cost`](Self::at_device_cost) writes at Argon2id's
    /// minimum: the cost is in every header, so a file written cheap
    /// opens cheap, and a suite that stretches a key for a second per
    /// attempt is a suite nobody runs (`docs/PLANNING.md` §16.119
    /// rule 3).
    fn build(
        display: DisplayInfo,
        tier: AssuranceTier,
        answer_entropy: bool,
        first_run_done: bool,
        cost: osk_backup::Cost,
    ) -> Self {
        let mut app = OpenSigner::new(
            tier,
            BuildInfo {
                version: "test",
                core_hash: None,
            },
        );
        app.set_kdf_cost(cost);
        let mut h = Harness {
            app,
            now_ms: 0,
            answer_entropy,
            pending: Vec::new(),
            answers: 0,
            element: None,
            seen: Vec::new(),
        };
        // A shell hands back what it kept before the display arrives,
        // so nothing is drawn on settings the device is about to have.
        if first_run_done {
            h.app.event(Event::Settings {
                bytes: b"opensigner-settings 1\nfirst_run_done=on\n".to_vec(),
            });
        }
        h.app.event(Event::Display(display));
        if answer_entropy {
            h.answer();
        }
        h
    }

    /// Answers the pending entropy request, if any. Every command seen,
    /// the request included, is kept for the next [`drain`](Self::drain).
    pub fn answer(&mut self) {
        let mut asked = false;
        while let Some(c) = self.app.poll_command() {
            asked |= c == Command::RequestEntropy;
            self.pending.push(c);
        }
        if asked {
            self.give_entropy();
        }
    }

    /// Answers one entropy request.
    fn give_entropy(&mut self) {
        let mut bytes = ENTROPY;
        bytes[0] ^= self.answers;
        self.answers = self.answers.wrapping_add(1);
        self.app.event(Event::Entropy(EntropyBytes::new(bytes)));
    }

    pub fn drain(&mut self) -> Vec<Command> {
        let mut out = std::mem::take(&mut self.pending);
        while let Some(c) = self.app.poll_command() {
            if c == Command::RequestEntropy && self.answer_entropy {
                self.give_entropy();
            }
            self.answer_keep(&c);
            out.push(c);
        }
        self.seen.extend(out.iter().cloned());
        out
    }

    /// The salts the core has asked the secure element to authenticate,
    /// in order.
    pub fn mac_salts(&self) -> Vec<[u8; 32]> {
        self.seen
            .iter()
            .filter_map(|c| match c {
                Command::SecureMac { salt } => Some(*salt),
                _ => None,
            })
            .collect()
    }

    /// Whether the shell has been asked to forget the stored key.
    pub fn asked_to_forget(&self) -> bool {
        self.seen.contains(&Command::ForgetSecret)
    }

    /// Answers the kept-secret channel as the shell would: one event per
    /// command, and "unavailable" everywhere on a shell with no element.
    fn answer_keep(&mut self, command: &Command) {
        let answer = match (command, self.element.as_mut()) {
            (Command::SecureMac { salt }, Some(e)) if e.available => {
                Some(Event::SecureMac { mac: e.mac(salt) })
            }
            (Command::SecureMac { .. }, _) => Some(Event::SecureUnavailable),
            (Command::StoreSecret { blob }, Some(e)) if e.storable => {
                e.blob = Some(blob.clone());
                Some(Event::SecretStored)
            }
            (Command::StoreSecret { .. }, _) => Some(Event::SecretNotStored),
            (Command::LoadSecret, Some(e)) => match &e.blob {
                Some(blob) => Some(Event::Secret { blob: blob.clone() }),
                None => Some(Event::SecretUnavailable),
            },
            (Command::LoadSecret, None) => Some(Event::SecretUnavailable),
            (Command::ForgetSecret, e) => {
                if let Some(e) = e {
                    e.blob = None;
                    // The element deletes its own key with the blob, so
                    // no copy of the bytes can be tried again.
                    e.key = [0; 32];
                }
                Some(Event::SecretForgotten)
            }
            _ => None,
        };
        if let Some(event) = answer {
            self.app.event(event);
        }
    }

    pub fn send(&mut self, e: Event) {
        self.app.event(e);
        self.drain();
    }

    /// Reads the frame as a shell does when it blits after a `Draw`, so
    /// that the canvas holds the frame of the state the app is in. The
    /// pixel-moving scroll (§16.36) is taken only then; a test that
    /// never reads the frame never paints one (§16.119 rule 5).
    pub fn blit(&mut self) {
        let _ = self.app.frame();
    }

    pub fn tick(&mut self, ms: u64) {
        self.now_ms += ms;
        self.send(Event::Tick {
            now_ms: self.now_ms,
        });
    }

    /// Scrolls `id` into view and returns its centre.
    pub fn center(&mut self, id: Id) -> (u16, u16) {
        let r = self
            .app
            .reveal(id)
            .unwrap_or_else(|| panic!("widget {} is not on screen", id.0));
        self.drain();
        let c = r.center();
        (c.x as u16, c.y as u16)
    }

    /// Sends a camera frame picturing `bytes` as a QR code, at 4 px per
    /// module, then the code a shell reads from it, and drains what
    /// they produced. A shell decodes its own frames and hands the core
    /// the payload (`Event::Scanned`); this stands in for the worker a
    /// real shell runs.
    pub fn scan_bytes(&mut self, bytes: &[u8]) -> Vec<Command> {
        let matrix = osk_codec::qr::encode(osk_codec::qr::Payload::Bytes(bytes), Ecc::Low)
            .expect("a code that fits");
        let (w, h, luma) = matrix.to_luma(4, osk_codec::qr::QUIET_ZONE);
        self.app.event(Event::CameraFrame {
            width: w as u16,
            height: h as u16,
            luma: luma.clone(),
            chroma: None,
        });
        if let Some(bytes) = opensigner_core::scan::decode_frame(w, h, &luma) {
            self.app.event(Event::Scanned { bytes });
        }
        self.drain()
    }

    pub fn tap(&mut self, id: Id) {
        let (x, y) = self.center(id);
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
    }

    /// Taps a point rather than a widget, for asserting that a screen
    /// has nothing to press.
    pub fn tap_at(&mut self, x: u16, y: u16) {
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
    }

    /// Presses `id` and leaves the finger down: a secret panel reveals
    /// while it is held. Returns the point, for [`release`](Self::release).
    pub fn press(&mut self, id: Id) -> (u16, u16) {
        let (x, y) = self.center(id);
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        (x, y)
    }

    /// Lifts the finger pressed at `point`.
    pub fn release(&mut self, point: (u16, u16)) {
        self.send(Event::Touch {
            x: point.0,
            y: point.1,
            phase: TouchPhase::Up,
        });
    }

    pub fn hold(&mut self, id: Id) {
        let (x, y) = self.center(id);
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        self.tick(HOLD_MS / 2);
        self.tick(HOLD_MS / 2 + 10);
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
    }

    pub fn type_text(&mut self, s: &str) {
        for c in s.chars() {
            self.send(Event::Key(Key::Char(c)));
        }
    }

    pub fn key(&mut self, k: Key) {
        self.send(Event::Key(k));
    }

    /// A key coming up again, which is what ends a hold held with Enter.
    pub fn key_up(&mut self, k: Key) {
        self.send(Event::KeyUp(k));
    }

    /// Tabs until the focus ring is on `id` (`docs/DESIGN.md` §4.15).
    pub fn focus_to(&mut self, id: Id) {
        for _ in 0..64 {
            if self.app.focused() == Some(id) {
                return;
            }
            self.key(Key::Tab);
        }
        panic!("Tab never reached widget {}", id.0);
    }

    /// Tabs to `id` and presses Enter on it.
    pub fn enter_on(&mut self, id: Id) {
        self.focus_to(id);
        self.key(Key::Enter);
    }

    /// Holds `id` with Enter: the key down, the ticks, the key up.
    pub fn hold_key(&mut self, id: Id) {
        self.focus_to(id);
        self.key(Key::Enter);
        self.tick(HOLD_MS / 2);
        self.tick(HOLD_MS / 2 + 10);
        self.key_up(Key::Enter);
    }

    /// Taps key `input` of the on-screen keyboard `keyboard` (PIN pads
    /// are touch-only).
    pub fn pad(&mut self, keyboard: Id, input: KeyInput) {
        let r = self
            .app
            .key_rect(keyboard, input)
            .unwrap_or_else(|| panic!("no enabled key {input:?} on keyboard {}", keyboard.0));
        let c = r.center();
        let (x, y) = (c.x as u16, c.y as u16);
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
    }

    /// Types `digits` on keyboard `keyboard` and presses ✓.
    pub fn type_pin(&mut self, keyboard: Id, digits: &str) {
        for c in digits.chars() {
            self.pad(keyboard, KeyInput::Char(c));
        }
        self.pad(keyboard, KeyInput::Done);
    }

    /// The session PIN steps, when a wizard is on them: [`PIN`] twice.
    pub fn set_pin_if_asked(&mut self) {
        let asked = self.app.load_step() == Some(Step::Pin)
            || self.app.create_step() == Some(opensigner_core::create::Step::Pin);
        if asked {
            self.type_pin(ids::LOAD_PIN_KEYBOARD, PIN);
            self.type_pin(ids::LOAD_PIN_KEYBOARD, PIN);
        }
    }

    /// "Add key" on a wizard's confirm step, then the PIN steps when the
    /// session has no PIN yet. Adding a key is reversible, so it is a
    /// tap (`docs/PLANNING.md` §16.27).
    pub fn add_key(&mut self) {
        self.tap(ids::LOAD_HOLD);
        self.set_pin_if_asked();
    }

    /// Unlocks with `digits` on the lock screen.
    pub fn unlock(&mut self, digits: &str) {
        assert_eq!(self.app.screen(), ScreenKind::Lock);
        self.type_pin(ids::LOCK_KEYBOARD, digits);
    }

    /// Home → Keys → the empty-state row or Add a key → "Load a key".
    pub fn open_load(&mut self) {
        self.open_add(ids::KEYS_LOAD);
    }

    /// Home → Keys → the empty-state row or Add a key → "Create a key".
    pub fn open_create(&mut self) {
        self.open_add(ids::KEYS_CREATE);
    }

    /// Add a key → "Load a key" → 24 words → the language step → the
    /// typed words → the checksum screen. A FROST member is a key like
    /// any other (`docs/PLANNING.md` §16.104 rule 3).
    pub fn start_load_24(&mut self, words: &[&str]) {
        self.open_add(ids::KEYS_LOAD);
        assert_eq!(self.app.screen(), ScreenKind::Load);
        self.choose(ids::LOAD_SOURCE_TYPE, ids::LOAD_SOURCE_CONTINUE);
        self.choose(ids::at(ids::LOAD_COUNT_BASE, 1), ids::LOAD_COUNT_CONTINUE);
        self.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
        for w in words {
            self.type_word(w);
        }
        assert_eq!(self.app.load_step(), Some(Step::Checksum));
    }

    /// Types one word and taps its candidate, which is the only thing
    /// that takes a word (`docs/PLANNING.md` §16.118).
    pub fn type_word(&mut self, word: &str) {
        self.type_text(word);
        let n = self
            .app
            .candidates()
            .iter()
            .position(|c| c == word)
            .unwrap_or_else(|| panic!("{word} is not among {:?}", self.app.candidates()));
        self.tap_candidate(n, word);
    }

    /// Taps candidate cell `n` of the word entry and expects the word
    /// to be taken: one tap where a tap accepts, and two on the panel,
    /// where the first selects the cell (`docs/PLANNING.md` §16.50).
    pub fn tap_candidate(&mut self, n: usize, word: &str) {
        let before = self.app.load_words_accepted();
        let per_row = (0..)
            .take_while(|i| self.app.candidate_rect(ids::LOAD_CANDIDATES, *i).is_some())
            .count();
        let (id, cell) = if per_row == 0 || n < per_row {
            (ids::LOAD_CANDIDATES, n)
        } else {
            (ids::LOAD_CANDIDATES_2, n - per_row)
        };
        let r = self
            .app
            .candidate_rect(id, cell)
            .unwrap_or_else(|| panic!("no candidate cell {cell} for {word}"));
        let c = r.center();
        self.tap_at(c.x as u16, c.y as u16);
        if self.app.load_words_accepted() == before {
            self.tap_at(c.x as u16, c.y as u16);
        }
        assert!(
            self.app.load_words_accepted() > before,
            "{word} was not accepted"
        );
    }

    /// Home → the tile at `i`.
    pub fn open_tile(&mut self, i: usize) {
        self.go_home();
        self.tap(ids::at(ids::HOME_TILE_BASE, i));
    }

    /// Home → Keys.
    pub fn open_keys(&mut self) {
        if self.app.screen() != ScreenKind::Keys {
            self.open_tile(TILE_KEYS);
        }
    }

    /// Home → Wallets.
    pub fn open_wallets(&mut self) {
        if self.app.screen() != ScreenKind::Wallets {
            self.open_tile(TILE_WALLETS);
        }
    }

    /// One of the ways a key arrives: an empty-state row on Keys, a row
    /// of the Add a key menu once a key is loaded.
    pub fn open_add(&mut self, row: Id) {
        if self.app.screen() != ScreenKind::Add {
            self.open_keys();
            // With nothing loaded Keys lists the two start rows itself,
            // and the row is tapped there; every other way in is a row
            // of the Add a key menu.
            if self.app.rect_of(row).is_none() {
                self.tap(ids::KEYS_ADD);
            }
        }
        self.tap(row);
    }

    /// One of the kinds of wallet: a row of the Add a wallet menu, or
    /// the empty state's own row where Wallets lists it.
    pub fn open_add_wallet(&mut self, row: Id) {
        if self.app.screen() != ScreenKind::AddWallet {
            self.open_wallets();
            if self.app.rect_of(row).is_none() {
                self.tap(ids::WALLETS_ADD);
            }
        }
        self.tap(row);
    }

    /// Key `i`'s page → "Open with passphrase".
    pub fn open_passphrase(&mut self, i: usize) {
        self.open_key(i);
        self.tap(ids::DETAIL_OPEN_PASSPHRASE);
        assert_eq!(self.app.screen(), ScreenKind::OpenPassphrase);
    }

    /// Key `i`'s page → BIP-85 → Words, which is the child-key flow.
    pub fn open_child(&mut self, i: usize) {
        self.open_key(i);
        self.tap(ids::DETAIL_BIP85);
        self.choose(ids::at(ids::PICK_BASE, 0), ids::PICK_CONTINUE);
        assert_eq!(self.app.screen(), ScreenKind::OpenChild);
    }

    /// Home → Keys → the row of key `i`, which is its own page.
    pub fn open_key(&mut self, i: usize) {
        self.open_keys();
        self.tap(ids::at(ids::KEYS_ROW_BASE, i));
    }

    /// Add a wallet → the wizard → Single-sig → key `i` → `script` →
    /// "Add this wallet", which lands on the new wallet's page.
    pub fn add_single_sig(&mut self, i: usize, script: usize) {
        self.open_add_wallet(ids::BUILD_NEW);
        self.choose(ids::at(ids::BUILD_KIND_BASE, 0), ids::BUILD_KIND_CONTINUE);
        self.tap(ids::at(ids::BUILD_WHICH_BASE, i));
        self.tap(ids::BUILD_CONTINUE);
        self.choose(
            ids::at(ids::BUILD_SCRIPT_BASE, script),
            ids::BUILD_SCRIPT_CONTINUE,
        );
        assert_eq!(self.app.screen(), ScreenKind::Build);
        self.tap(ids::BUILD_ADD_WALLET);
        assert_eq!(self.app.screen(), ScreenKind::Wallet);
    }

    /// The chevron until Home is on screen.
    pub fn go_home(&mut self) {
        for _ in 0..8 {
            if self.app.screen() == ScreenKind::Home {
                return;
            }
            self.tap(ids::BACK);
        }
        assert_eq!(self.app.screen(), ScreenKind::Home, "no way back to Home");
    }

    /// Home → the Settings tile.
    pub fn open_settings(&mut self) {
        if self.app.screen() != ScreenKind::Settings {
            self.open_tile(TILE_SETTINGS);
        }
    }

    /// The SegWit single-sig wallet of key `i`: its row on Wallets where
    /// it is registered already, and the flow that registers it where it
    /// is not. A single-sig wallet is explicit now (`docs/PLANNING.md`
    /// §16.104 rule 2), so a test that wants one adds it.
    pub fn open_single_sig(&mut self, i: usize) {
        self.open_wallets();
        match self.single_sig_row(i) {
            Some(w) => self.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, w)),
            None => self.add_single_sig(i, 2),
        }
    }

    /// Which row of Wallets is the SegWit single-sig wallet of key `i`.
    pub fn single_sig_row(&self, i: usize) -> Option<usize> {
        self.app.single_sig_wallet(i, ScriptType::NativeSegwit)
    }

    /// Home → Wallets → the row of the policy at `i`.
    pub fn open_policy(&mut self, i: usize) {
        self.open_wallets();
        self.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, i));
    }

    /// Settings → a value row → the Choice it opens → the option at
    /// `i`. §4.2's Setting applies on the tap, so the chevron is what
    /// closes the list, and the harness leaves the Settings menu on
    /// screen.
    pub fn set_setting(&mut self, row: Id, base: u32, i: usize) {
        if self.app.screen() != ScreenKind::Settings {
            self.open_settings();
        }
        self.tap(row);
        assert_eq!(self.app.screen(), ScreenKind::Setting);
        self.tap(ids::at(base, i));
        assert_eq!(self.app.screen(), ScreenKind::Setting, "the screen stays");
        self.tap(ids::BACK);
    }

    /// Settings → Network → the chain at `i`, then back to Home.
    pub fn set_network(&mut self, i: usize) {
        self.set_setting(ids::SETTINGS_NETWORK_ROW, ids::SETTINGS_NET_BASE, i);
        self.tap(ids::BACK);
    }

    /// A choice step: the tap that checks `row`, then the Continue that
    /// confirms it (`docs/DESIGN.md` §2.7).
    pub fn choose(&mut self, row: Id, cont: Id) {
        self.tap(row);
        self.tap(cont);
    }

    /// The Load wizard's three choice steps: source, count, language.
    pub fn load_choices(&mut self, count: usize) {
        self.choose(ids::LOAD_SOURCE_TYPE, ids::LOAD_SOURCE_CONTINUE);
        self.choose(
            ids::at(ids::LOAD_COUNT_BASE, count),
            ids::LOAD_COUNT_CONTINUE,
        );
        self.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    }

    /// Home → Load a key → "SLIP-39 shares" → the word count, by its
    /// row on the count Choice: 0 for 20 words, 1 for 33.
    pub fn open_slip39(&mut self, count_row: usize) {
        self.open_load();
        assert_eq!(self.app.screen(), ScreenKind::Load);
        self.choose(ids::LOAD_SOURCE_SLIP39, ids::LOAD_SOURCE_CONTINUE);
        self.choose(
            ids::at(ids::LOAD_COUNT_BASE, count_row),
            ids::LOAD_COUNT_CONTINUE,
        );
        assert_eq!(self.app.load_step(), Some(Step::Words));
    }

    /// Home → Load a key → "Codex32", which opens the string entry.
    pub fn open_codex32(&mut self) {
        self.open_load();
        assert_eq!(self.app.screen(), ScreenKind::Load);
        self.choose(ids::LOAD_SOURCE_CODEX32, ids::LOAD_SOURCE_CONTINUE);
        assert_eq!(self.app.load_step(), Some(Step::Words));
    }

    /// Types one codex32 string on the bech32 keyboard, character by
    /// character after the `ms1` the field already holds, and presses ✓.
    /// A key the mask has dimmed is not there to tap, which is what
    /// `pad` panics on.
    pub fn type_codex32(&mut self, string: &str) {
        let lower = string.to_ascii_lowercase();
        let rest = lower
            .strip_prefix("ms1")
            .unwrap_or_else(|| panic!("{string} is not a codex32 string"));
        for c in rest.chars() {
            self.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
        }
        self.pad(ids::LOAD_KEYBOARD, KeyInput::Done);
    }

    /// Types one codex32 string into the "Type it back" entry, on its
    /// own keyboard, and presses ✓.
    pub fn type_codex32_back(&mut self, string: &str) {
        let lower = string.to_ascii_lowercase();
        let rest = lower
            .strip_prefix("ms1")
            .unwrap_or_else(|| panic!("{string} is not a codex32 string"));
        for c in rest.chars() {
            self.pad(ids::CODEX32_KEYBOARD, KeyInput::Char(c));
        }
        self.pad(ids::CODEX32_KEYBOARD, KeyInput::Done);
    }

    /// Types one SLIP-39 share, word by word, and stops on the result
    /// that says whether it was taken.
    pub fn type_share(&mut self, share: &str) {
        for w in share.split_whitespace() {
            self.type_word(w);
        }
        assert_eq!(self.app.load_step(), Some(Step::Checksum));
    }

    /// Home → wizard → typed words → checksum screen.
    pub fn start_load(&mut self, words: &[&str]) {
        self.open_load();
        assert_eq!(self.app.screen(), ScreenKind::Load);
        assert_eq!(self.app.load_step(), Some(Step::Source));
        self.load_choices(0);
        assert_eq!(self.app.load_step(), Some(Step::Words));
        for w in words {
            self.type_text(w);
            self.key(Key::Enter);
        }
        assert_eq!(self.app.load_step(), Some(Step::Checksum));
    }

    /// Checksum → passphrase → Home.
    pub fn finish_load(&mut self, passphrase: Option<&str>) {
        self.tap(ids::LOAD_CONTINUE);
        self.finish_passphrase(passphrase);
    }

    /// The passphrase steps onwards, from the offer the wizard is
    /// already on.
    pub fn finish_passphrase(&mut self, passphrase: Option<&str>) {
        assert_eq!(self.app.load_step(), Some(Step::PassphraseOffer));
        match passphrase {
            None => self.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE),
            Some(p) => {
                self.choose(ids::LOAD_ADD_PASSPHRASE, ids::LOAD_PASS_CONTINUE);
                assert_eq!(self.app.load_step(), Some(Step::Passphrase));
                self.type_text(p);
                self.key(Key::Enter);
                assert_eq!(self.app.load_step(), Some(Step::PassphraseConfirm));
                self.tap(ids::LOAD_WHICH_CONTINUE);
            }
        }
        assert_eq!(self.app.load_step(), Some(Step::Confirm));
        self.add_key();
        // A device that can keep the key offers to at once; Back is
        // "not now", and the key is added either way.
        if self.app.screen() == ScreenKind::Keep {
            self.tap(ids::BACK);
        }
        assert_eq!(self.app.screen(), ScreenKind::Home);
    }
}

/// The launcher's tiles, in grid order (`docs/PLANNING.md` §16.104).
pub const TILE_WALLETS: usize = 0;
pub const TILE_KEYS: usize = 1;
pub const TILE_SCAN: usize = 2;
pub const TILE_TOOLS: usize = 3;
pub const TILE_LEARN: usize = 4;
pub const TILE_SETTINGS: usize = 5;

pub fn expected_fingerprint(passphrase: &[u8], network: Network) -> Fingerprint {
    let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
    MasterKey::from_seed(&m.to_seed(passphrase).unwrap(), network).fingerprint()
}
