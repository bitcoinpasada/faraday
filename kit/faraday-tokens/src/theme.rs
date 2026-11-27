//! `Theme`, `TextSize`, `Contrast` and `Mode` (09 §3.2).

use crate::palette::ThemeName;

/// Light or Dark: Material's `isDark`.
// Placeholder until the domain copy: `main`'s `faraday/faraday-ui/src/theme.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

/// Material's contrast level.
// Placeholder until the domain copy: `main`'s `faraday/faraday-ui/src/theme.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Contrast {
    #[default]
    Standard,
    Medium,
    High,
}

/// The scheme the screens are drawn in: a theme in a mode at a contrast level.
// Placeholder until the domain copy: `main`'s `faraday/faraday-ui/src/theme.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Theme {
    pub name: ThemeName,
    pub mode: Mode,
    pub contrast: Contrast,
}

/// How large text is drawn.
// Placeholder until the domain copy: `main`'s `faraday/faraday-ui/src/theme.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextSize {
    #[default]
    Standard,
    Large,
    Larger,
}
