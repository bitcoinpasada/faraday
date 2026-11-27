//! `Role`, the scheme table, the fixed colours and the stick tones (09 §3.2).

/// One of the 45 palette roles.
// Placeholder until the domain copy: `main`'s `faraday/faraday-ui/src/pal/mod.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Primary,
    OnPrimary,
    Surface,
    OnSurface,
}

/// The theme's seed and style: Cobalt is the default.
// Placeholder until the domain copy: `main`'s `faraday/faraday-ui/src/pal/mod.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemeName {
    #[default]
    Cobalt,
}
