//! The settings Appearance keeps beside the scheme: the Motion style
//! and the Speed (`docs/MOTION.md` §3.5).
//! Each reads from the settings file by its word and reads anything else
//! as nothing, so an unknown value leaves the default.

use crate::strings::EN;
use faraday_draw::tokens::{MOTION_LONG4, MOTION_MEDIUM1, MOTION_MEDIUM4};

/// How screens change (`docs/MOTION.md` §3.5): Off is Reduce motion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionStyle {
    /// The session pane glides, the rest fades through.
    #[default]
    Smooth,
    /// Two kept screens blend.
    Crossfade,
    /// Screens change at once.
    Off,
}

impl MotionStyle {
    /// Every choice, in the order Settings offers them.
    pub const ALL: [MotionStyle; 3] = [
        MotionStyle::Smooth,
        MotionStyle::Crossfade,
        MotionStyle::Off,
    ];

    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            MotionStyle::Smooth => EN.settings_motion_smooth,
            MotionStyle::Crossfade => EN.settings_motion_crossfade,
            MotionStyle::Off => EN.settings_motion_off,
        }
    }

    /// The line saying what it does.
    pub fn line(self) -> &'static str {
        match self {
            MotionStyle::Smooth => EN.settings_motion_smooth_line,
            MotionStyle::Crossfade => EN.settings_motion_crossfade_line,
            MotionStyle::Off => EN.settings_motion_off_line,
        }
    }

    /// The word a settings file writes it as (`motion=`).
    pub fn id(self) -> &'static str {
        match self {
            MotionStyle::Smooth => "smooth",
            MotionStyle::Crossfade => "crossfade",
            MotionStyle::Off => "off",
        }
    }

    /// The choice a `motion=` word names; `full` and `reduced` are what
    /// files wrote while Reduce motion was a switch.
    pub fn from_id(id: &str) -> Option<MotionStyle> {
        match id {
            "full" => Some(MotionStyle::Smooth),
            "reduced" => Some(MotionStyle::Off),
            _ => Self::ALL.into_iter().find(|m| m.id() == id),
        }
    }
}

/// How long a change takes, for Smooth motion and Crossfade.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionSpeed {
    /// 250 ms.
    Quick,
    /// 400 ms.
    Medium,
    /// 600 ms.
    #[default]
    Relaxed,
}

impl MotionSpeed {
    /// Every choice, in the order Settings offers them.
    pub const ALL: [MotionSpeed; 3] = [
        MotionSpeed::Quick,
        MotionSpeed::Medium,
        MotionSpeed::Relaxed,
    ];

    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            MotionSpeed::Quick => EN.settings_speed_quick,
            MotionSpeed::Medium => EN.settings_speed_medium,
            MotionSpeed::Relaxed => EN.settings_speed_relaxed,
        }
    }

    /// Its duration, in ms.
    pub fn ms(self) -> u64 {
        match self {
            MotionSpeed::Quick => MOTION_MEDIUM1,
            MotionSpeed::Medium => MOTION_MEDIUM4,
            MotionSpeed::Relaxed => MOTION_LONG4,
        }
    }

    /// The word a settings file writes it as (`speed=`).
    pub fn id(self) -> &'static str {
        match self {
            MotionSpeed::Quick => "quick",
            MotionSpeed::Medium => "medium",
            MotionSpeed::Relaxed => "relaxed",
        }
    }

    /// The choice a `speed=` word names.
    pub fn from_id(id: &str) -> Option<MotionSpeed> {
        Self::ALL.into_iter().find(|s| s.id() == id)
    }
}
