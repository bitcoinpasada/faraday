//! Modes (F81, `docs/MODES.md`): what the pill names and the Mode sheet
//! chooses. One of Practice, Mainnet, Testnet, Signet, Regtest. Practice is not a
//! network: it is the key store's public set ([`faraday_keys::Keys::demo`])
//! on Testnet, so nothing here stores the mode beside the session.
//!
//! A mode change is a lock: the fresh session the shell starts afterwards
//! is in the mode chosen ([`Faraday::mode_after_restart`]).

pub(crate) mod screen;

use crate::Faraday;
use crate::strings::EN;
use crate::wallet::network_name;
use faraday_bip::keys::Network;

/// What a session is in: a network, or Practice (the public set on Testnet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// The public samples on Testnet.
    Demo,
    /// Bitcoin itself.
    Mainnet,
    /// Testnet.
    Testnet,
    /// Signet.
    Signet,
    /// Regtest.
    Regtest,
}

/// The modes of the stick and the Pi, and of a `Faraday::new()`.
pub const STICK_MODES: &[Mode] = &[Mode::Mainnet, Mode::Testnet, Mode::Signet, Mode::Regtest];

/// The modes of Faraday Demo and of the website.
pub const DESKTOP_MODES: &[Mode] = &[Mode::Demo, Mode::Testnet, Mode::Signet, Mode::Regtest];

impl Mode {
    /// The network the mode runs on: Practice is on Testnet.
    pub fn network(self) -> Network {
        match self {
            Mode::Demo | Mode::Testnet => Network::Testnet,
            Mode::Mainnet => Network::Mainnet,
            Mode::Signet => Network::Signet,
            Mode::Regtest => Network::Regtest,
        }
    }

    /// The mode of a session on `network` that is not Practice.
    pub fn of(network: Network) -> Mode {
        match network {
            Network::Mainnet => Mode::Mainnet,
            Network::Testnet => Mode::Testnet,
            Network::Signet => Mode::Signet,
            Network::Regtest => Mode::Regtest,
        }
    }

    /// What the pill and the sheet call it.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Demo => EN.mode_demo,
            m => network_name(m.network()),
        }
    }
}

/// The mode a lock in progress changes to, if the lock is a mode change.
#[derive(Debug, Default)]
pub struct ModesArea {
    next_mode: Option<Mode>,
}

impl ModesArea {
    /// The mode chosen, once the lock for it is asked.
    pub(crate) fn choose(&mut self, m: Mode) {
        self.next_mode = Some(m);
    }

    /// No mode change is asked for.
    pub(crate) fn clear(&mut self) {
        self.next_mode = None;
    }

    /// The mode the fresh session starts in: the one chosen, or `in_use`
    /// (a lock keeps the mode).
    pub(crate) fn after_restart(&self, in_use: Mode) -> Mode {
        self.next_mode.unwrap_or(in_use)
    }

    /// Whether the lock now asked for is a mode change.
    pub(crate) fn changing(&self) -> bool {
        self.next_mode.is_some()
    }
}

impl Faraday {
    /// The modes this app offers, in the order the sheet lists them.
    pub fn modes(&self) -> &'static [Mode] {
        self.session.modes
    }

    /// Sets the modes this app offers: `STICK_MODES` (the default) or
    /// `DESKTOP_MODES`. They are the session's too: a wallet or a PSBT
    /// moves an empty session only to one of them
    /// ([`crate::wallet::Session::follow`]).
    pub fn set_modes(&mut self, modes: &'static [Mode]) {
        self.session.modes = modes;
    }

    /// The mode in use: Practice while the key store's public set is on,
    /// otherwise the session's network. Read from the state, never
    /// stored.
    pub fn mode(&self) -> Mode {
        if self.session.keys.demo() {
            Mode::Demo
        } else {
            Mode::of(self.session.network())
        }
    }

    /// The mode the fresh session starts in once the app is dropped: the
    /// one chosen, or the one in use (a lock keeps the mode). The shell
    /// reads it before it drops the app.
    pub fn mode_after_restart(&self) -> Mode {
        self.next_mode.after_restart(self.mode())
    }

    /// What a lock hands the shell to keep in memory: the boxes as they
    /// are, or, for a mode change, nothing at all, the Inbox and the
    /// Outbox emptied here too (`docs/MODES.md` §2, item 6).
    pub(crate) fn save_boxes_for_lock(&mut self) {
        if self.mode_changing() {
            self.inbox.clear();
            self.outbox.clear();
            self.out
                .storage_out
                .push_back(crate::StorageCommand::SaveBoxes {
                    inbox: Vec::new(),
                    outbox: Vec::new(),
                    kept: Vec::new(),
                });
        } else {
            self.save_boxes();
        }
    }

    /// Whether the lock now asked for is a mode change: it keeps nothing
    /// but the mode. A lookup.
    pub fn mode_changing(&self) -> bool {
        self.next_mode.changing()
    }

    /// Handles `Action::ModeSheet`: the pill or Settings' Mode row opens
    /// the sheet, over the page it was pressed on.
    pub(crate) fn mode_sheet_open(&mut self) {
        self.sheet = Some(crate::Sheet::Mode);
    }

    /// Handles `Action::Mode`: a mode change is a lock. The mode in use,
    /// one this app does not offer, or any while the network is fixed
    /// closes the sheet and does nothing else. Otherwise the same ask as
    /// Lock, and a fresh session in the mode; no warning comes first.
    pub(crate) fn mode_act(&mut self, m: Mode) {
        if m == self.mode() || !self.modes().contains(&m) || self.session.keys.network_fixed() {
            self.next_mode.clear();
            self.sheet = None;
            return;
        }
        self.next_mode.choose(m);
        self.sheet = None;
        self.lock_ask(false);
    }
}
