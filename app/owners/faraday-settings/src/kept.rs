//! What the settings owner carries through a lock (05 §7.4): the person's choices, and the mode
//! asked for. Both entries are `any_mode` (a mode change keeps them); both are `Public`.

use faraday_kernel::settings::Settings;
use faraday_kernel::{CarryName, CarryValue, Public};
use faraday_port::bytes::Bytes;
use faraday_text::Text;

use crate::file;

/// The carry's `settings` entry: the choices and the clock, kept across a lock.
pub const SETTINGS: CarryName = CarryName::new("settings.kept", false, true);

/// The carry's `mode` entry: the mode asked for, which takes effect at this lock and which
/// `Root::mode` reads (05 §7.5).
pub const MODE: CarryName = CarryName::new("settings.mode", false, true);

/// What a session is in: a network, or Practice (the public samples on Testnet). `main`'s
/// `modes::Mode`. The kernel's `Mode` is built only by a product crate (S9), which maps this one.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum ModeAsked {
    Practice,
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

impl CarryValue for ModeAsked {
    fn encode(&self) -> Bytes {
        Bytes::new(vec![match self {
            ModeAsked::Practice => 0,
            ModeAsked::Mainnet => 1,
            ModeAsked::Testnet => 2,
            ModeAsked::Signet => 3,
            ModeAsked::Regtest => 4,
        }])
    }

    fn decode(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0] => Some(ModeAsked::Practice),
            [1] => Some(ModeAsked::Mainnet),
            [2] => Some(ModeAsked::Testnet),
            [3] => Some(ModeAsked::Signet),
            [4] => Some(ModeAsked::Regtest),
            _ => None,
        }
    }
}

/// The settings as the carry holds them: the file's body text (read back with the stick's rules
/// off, so an idle time may be never), what the file does not hold, and what the boot stick holds.
#[derive(Public)]
pub struct Kept {
    body: Text,
    offset_secs: i64,
    checked: bool,
    seal_amounts: bool,
    nonce_history: bool,
    on_stick: Option<Text>,
}

const VERSION: u8 = 1;

impl Kept {
    pub(crate) fn of(s: &Settings, on_stick: Option<&str>) -> Kept {
        Kept {
            body: Text::of(&file::body(s)),
            offset_secs: s.offset_secs,
            checked: s.checked,
            seal_amounts: s.seal_amounts,
            nonce_history: s.nonce_history,
            on_stick: on_stick.map(Text::of),
        }
    }

    /// The settings these hold, over the defaults, and what the boot stick held.
    pub(crate) fn settings(&self) -> (Settings, Option<String>) {
        let mut s = Settings::default();
        file::read_kept(&mut s, self.body.as_str());
        s.offset_secs = self.offset_secs;
        s.checked = self.checked;
        s.seal_amounts = self.seal_amounts;
        s.nonce_history = self.nonce_history;
        (s, self.on_stick.as_ref().map(|t| t.as_str().to_owned()))
    }
}

impl CarryValue for Kept {
    /// Version, a flags byte, the offset, the body's length (four bytes) and the body, then what
    /// the boot stick holds, when it is known.
    fn encode(&self) -> Bytes {
        let flags = u8::from(self.checked)
            | u8::from(self.seal_amounts) << 1
            | u8::from(self.nonce_history) << 2
            | u8::from(self.on_stick.is_some()) << 3;
        let body = self.body.as_str().as_bytes();
        let mut out = vec![VERSION, flags];
        out.extend_from_slice(&self.offset_secs.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        if let Some(on) = &self.on_stick {
            out.extend_from_slice(on.as_str().as_bytes());
        }
        Bytes::new(out)
    }

    fn decode(bytes: &[u8]) -> Option<Self> {
        let (&version, rest) = bytes.split_first()?;
        let (&flags, rest) = rest.split_first()?;
        if version != VERSION || flags > 0b1111 {
            return None;
        }
        let (offset, rest) = rest.split_at_checked(8)?;
        let (len, rest) = rest.split_at_checked(4)?;
        let len = u32::from_le_bytes(len.try_into().ok()?) as usize;
        let (body, rest) = rest.split_at_checked(len)?;
        let on_stick = if flags & 0b1000 != 0 {
            Some(Text::of(core::str::from_utf8(rest).ok()?))
        } else if rest.is_empty() {
            None
        } else {
            return None;
        };
        Some(Kept {
            body: Text::of(core::str::from_utf8(body).ok()?),
            offset_secs: i64::from_le_bytes(offset.try_into().ok()?),
            checked: flags & 1 != 0,
            seal_amounts: flags & 0b10 != 0,
            nonce_history: flags & 0b100 != 0,
            on_stick,
        })
    }
}
