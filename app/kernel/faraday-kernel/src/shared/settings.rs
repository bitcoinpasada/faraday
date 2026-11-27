//! The settings owner's state and changes (03 §7.2, §7.3).

use faraday_port::values::Orientation;

use super::change;
use crate::cx::Edits;
use crate::message::Public;

/// The settings owner's pages (06 §4.2). The kernel's, as `vaults::Page` is.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Settings,
    SetTime,
}

/// The person's choices: the settings owner's state, lent to `View`.
pub struct Settings {
    /// Minutes without input before the session locks; 0 for never.
    pub idle_lock_min: u16,
    /// Minutes without input before the machine powers off; 0 for never. Offline only.
    pub idle_off_min: u16,
    /// How long each frame of an animated code stays up.
    pub qr_frame_ms: u32,
    /// The time zone, minutes east of UTC; `None` until the person chooses one.
    pub zone_min: Option<i16>,
    /// Set the time's offset against the adapter's clock.
    pub offset_secs: i64,
    /// The clock was set or kept by the person.
    pub checked: bool,
    pub appearance: Appearance,
    pub scale_pct: u16,
    /// Which way a panel that can turn shows the app.
    pub rotation: Orientation,
    /// Remembered: the amounts kept with a seal.
    pub seal_amounts: bool,
    /// Remembered: the nonce check's history.
    pub nonce_history: bool,
}

/// 05 §4.1: the values of a first start, which the settings owner starts from and the runtime
/// lends where that owner is not yet built.
impl Default for Settings {
    fn default() -> Self {
        Settings {
            idle_lock_min: 30,
            idle_off_min: 60,
            qr_frame_ms: 500,
            zone_min: None,
            offset_secs: 0,
            checked: false,
            appearance: Appearance {
                theme: ThemeId(0),
                dark: false,
                contrast: Contrast::Standard,
                text: TextSize::Standard,
                motion: None,
                speed: MotionSpeed::Relaxed,
            },
            scale_pct: 100,
            rotation: Orientation::Landscape,
            seal_amounts: false,
            nonce_history: false,
        }
    }
}

impl Settings {
    pub fn edits(&self) -> Edits {
        todo!()
    }
}

/// The kernel's own appearance values, which the runtime maps to the kit's `Theme` and
/// `TextSize` when it builds the `Ui` (04 §12.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub struct Appearance {
    pub theme: ThemeId,
    pub dark: bool,
    pub contrast: Contrast,
    pub text: TextSize,
    /// `None` until the person chooses.
    pub motion: Option<MotionStyle>,
    pub speed: MotionSpeed,
}

/// A theme by its place in the kit's scheme table.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct ThemeId(pub u8);

#[derive(Public, Copy, PartialEq, Eq)]
pub enum Contrast {
    Standard,
    Medium,
    High,
}

#[derive(Public, Copy, PartialEq, Eq)]
pub enum TextSize {
    Standard,
    Large,
    Larger,
}

/// Off is Reduce motion: nothing slides, glides, coasts or stretches.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum MotionStyle {
    Smooth,
    Crossfade,
    Off,
}

#[derive(Public, Copy, PartialEq, Eq)]
pub enum MotionSpeed {
    Quick,
    Medium,
    Relaxed,
}

/// One choice of `Settings`.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Choice {
    IdleLock(u16),
    IdleOff(u16),
    QrFrame(u32),
    Zone(i16),
    Appearance(Appearance),
    Scale(u16),
    Rotation(Orientation),
    SealAmounts(bool),
    NonceHistory(bool),
}

/// A choice set from outside the settings owner (the Starter's Appearance).
pub struct Set(pub Choice);

pub enum Change {
    Set(Set),
}

pub enum Done {
    Set(()),
}

/// What the settings owner says changed: the clock set or kept, or any other choice.
pub enum Changed {
    Clock,
    Choices,
}

change!(Set => ());
