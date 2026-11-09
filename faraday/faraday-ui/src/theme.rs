//! Which scheme the screens are drawn in (`docs/THEMES.md` §3): a theme,
//! a mode and a contrast level, and the words a settings file writes them
//! as, each of the fourteen themes of before mapped to one (§13).

use crate::pal::{self, Paint, Role, Scheme, ThemeName};
use faraday_draw::Color;
use faraday_strings::EN;

/// Light or Dark: Material's `isDark`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// Dark text on a light page.
    #[default]
    Light,
    /// Light text on a dark page.
    Dark,
}

/// Material's contrast level: 0, 0.5 and 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Contrast {
    /// Material's standard contrast.
    #[default]
    Standard,
    /// Material's medium contrast.
    Medium,
    /// Material's high contrast: text 7:1 or more on every surface.
    High,
}

impl Mode {
    /// Both modes, in the order of each theme's schemes.
    pub const ALL: [Mode; 2] = [Mode::Light, Mode::Dark];
}

impl Contrast {
    /// Every level, in the order of each theme's schemes.
    pub const ALL: [Contrast; 3] = [Contrast::Standard, Contrast::Medium, Contrast::High];
}

/// The scheme the screens are drawn in: a theme in a mode at a contrast
/// level. Cobalt, Light, Standard is the one a first start is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Theme {
    /// The seed and style.
    pub name: ThemeName,
    /// Light or Dark.
    pub mode: Mode,
    /// Standard, Medium or High.
    pub contrast: Contrast,
}

impl Theme {
    /// `name` in `mode` at `contrast`.
    pub const fn new(name: ThemeName, mode: Mode, contrast: Contrast) -> Theme {
        Theme {
            name,
            mode,
            contrast,
        }
    }

    /// Every scheme: each theme in Light and Dark, each at every level.
    pub fn all() -> impl Iterator<Item = Theme> {
        ThemeName::ALL.into_iter().flat_map(|name| {
            Mode::ALL.into_iter().flat_map(move |mode| {
                Contrast::ALL
                    .into_iter()
                    .map(move |contrast| Theme::new(name, mode, contrast))
            })
        })
    }

    /// Its colours, one per [`Role`]: a row of the generated table.
    pub fn scheme(self) -> &'static Scheme {
        &pal::SCHEMES[self.row()]
    }

    /// Its row of the generated table, 0 to 59: also its number where a
    /// press is carried as a number (the Starter's actions).
    pub fn row(self) -> usize {
        (self.name as usize * Mode::ALL.len() + self.mode as usize) * Contrast::ALL.len()
            + self.contrast as usize
    }

    /// The scheme of row `row` ([`Theme::row`]), if there is one.
    pub fn from_row(row: usize) -> Option<Theme> {
        let per_name = Mode::ALL.len() * Contrast::ALL.len();
        let name = *ThemeName::ALL.get(row / per_name)?;
        let mode = Mode::ALL[row % per_name / Contrast::ALL.len()];
        let contrast = Contrast::ALL[row % Contrast::ALL.len()];
        Some(Theme::new(name, mode, contrast))
    }

    /// `role`'s colour in this scheme.
    pub fn role(self, role: Role) -> Color {
        self.scheme()[role as usize]
    }

    /// `paint` as this scheme draws it: a role looked up, a stick's tone
    /// by mode and contrast, a fixed colour as it is.
    pub fn color(self, paint: impl Into<Paint>) -> Color {
        paint.into().in_theme(self)
    }

    /// Dark text on a light page: shadows are lighter.
    pub(crate) fn is_light(self) -> bool {
        self.mode == Mode::Light
    }

    /// Text 7:1 or more on every surface: Faraday's own high contrast
    /// rules apply (`docs/THEMES.md` §9).
    pub fn is_high_contrast(self) -> bool {
        self.contrast == Contrast::High
    }
}

impl ThemeName {
    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            ThemeName::Cobalt => EN.ui_cobalt,
            ThemeName::BitcoinOrange => EN.ui_bitcoin_orange,
            ThemeName::Lamplight => EN.ui_lamplight,
            ThemeName::Mars => EN.ui_mars,
            ThemeName::Forest => EN.ui_forest,
            ThemeName::Frost => EN.ui_frost,
            ThemeName::Dusk => EN.ui_dusk,
            ThemeName::Lavender => EN.ui_lavender,
            ThemeName::Rose => EN.ui_rose,
            ThemeName::Monochrome => EN.ui_monochrome,
        }
    }

    /// The theme a settings file's `theme=` word names.
    pub fn from_id(id: &str) -> Option<ThemeName> {
        ThemeName::ALL.into_iter().find(|t| t.id() == id)
    }
}

impl Mode {
    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Light => EN.ui_light,
            Mode::Dark => EN.ui_dark,
        }
    }

    /// The word a settings file writes it as (`mode=`).
    pub fn id(self) -> &'static str {
        match self {
            Mode::Light => "light",
            Mode::Dark => "dark",
        }
    }

    /// The mode a settings file's `mode=` word names.
    pub fn from_id(id: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.id() == id)
    }
}

impl Contrast {
    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            Contrast::Standard => EN.ui_contrast_standard,
            Contrast::Medium => EN.ui_contrast_medium,
            Contrast::High => EN.ui_contrast_high,
        }
    }

    /// The word a settings file writes it as (`contrast=`).
    pub fn id(self) -> &'static str {
        match self {
            Contrast::Standard => "standard",
            Contrast::Medium => "medium",
            Contrast::High => "high",
        }
    }

    /// The level a settings file's `contrast=` word names.
    pub fn from_id(id: &str) -> Option<Contrast> {
        Contrast::ALL.into_iter().find(|c| c.id() == id)
    }
}

/// How large text is drawn (Settings › Appearance, F85,
/// `docs/TEXT-SIZE.md` §2): every type role's size and line height times
/// its factor, for Faraday and the Starter alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextSize {
    /// Text at its own size.
    #[default]
    Standard,
    /// Text 1.2 times its size (`TEXT_LARGE_PCT`).
    Large,
    /// Text 1.4 times its size (`TEXT_LARGER_PCT`).
    Larger,
}

impl TextSize {
    /// Every choice, in the order Settings offers them.
    pub const ALL: [TextSize; 3] = [TextSize::Standard, TextSize::Large, TextSize::Larger];

    /// The choices Settings offers (`docs/TEXT-SIZE.md` §2): all three.
    pub const OFFERED: [TextSize; 3] = TextSize::ALL;

    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            TextSize::Standard => EN.settings_text_standard,
            TextSize::Large => EN.settings_text_large,
            TextSize::Larger => EN.settings_text_larger,
        }
    }

    /// The word a settings file writes it as (`text=`).
    pub fn id(self) -> &'static str {
        match self {
            TextSize::Standard => "standard",
            TextSize::Large => "large",
            TextSize::Larger => "larger",
        }
    }

    /// The choice a settings file's `text=` word names.
    pub fn from_id(id: &str) -> Option<TextSize> {
        TextSize::ALL.into_iter().find(|t| t.id() == id)
    }

    /// Its factor in percent: 100, or the two tokens.
    pub fn pct(self) -> u16 {
        match self {
            TextSize::Standard => 100,
            TextSize::Large => faraday_draw::tokens::TEXT_LARGE_PCT,
            TextSize::Larger => faraday_draw::tokens::TEXT_LARGER_PCT,
        }
    }

    /// Its factor: what every text size and line height is multiplied by.
    pub fn factor(self) -> f32 {
        faraday_draw::tokens::percent(self.pct())
    }
}

impl Theme {
    /// The scheme a settings file's `theme=` word opens in before any
    /// `mode=` or `contrast=` line (`docs/THEMES.md` §13): each of the
    /// fourteen themes Settings offered before the three choices maps to
    /// a theme, mode and contrast; a theme named since is in Light at
    /// Standard. A word neither names is `None`.
    pub fn from_stored(id: &str) -> Option<Theme> {
        use Contrast::{High, Standard};
        use Mode::{Dark, Light};
        let (name, mode, contrast) = match id {
            "dark" => (ThemeName::Cobalt, Dark, Standard),
            "light" => (ThemeName::Cobalt, Light, Standard),
            "nord" => (ThemeName::Frost, Dark, Standard),
            "catppuccin" => (ThemeName::Lavender, Dark, Standard),
            "tokyo-night" => (ThemeName::Dusk, Dark, Standard),
            "gruvbox" => (ThemeName::Lamplight, Dark, Standard),
            "rose-pine" => (ThemeName::Rose, Light, Standard),
            "bitcoin-orange" => (ThemeName::BitcoinOrange, Dark, Standard),
            "bitcoin-orange-light" => (ThemeName::BitcoinOrange, Light, Standard),
            "mars" => (ThemeName::Mars, Dark, Standard),
            "high-contrast-dark" => (ThemeName::Monochrome, Dark, High),
            "high-contrast-light" => (ThemeName::Cobalt, Light, High),
            other => (ThemeName::from_id(other)?, Light, Standard),
        };
        Some(Theme::new(name, mode, contrast))
    }

    /// A scheme written as a tool's argument: a stored theme id, then
    /// optionally a mode and a contrast, each after a comma
    /// (`frost,dark,high`), read as a settings file's lines are.
    pub fn from_words(words: &str) -> Option<Theme> {
        let mut parts = words.split(',');
        let mut theme = Theme::from_stored(parts.next()?)?;
        if let Some(mode) = parts.next() {
            theme.mode = Mode::from_id(mode)?;
        }
        if let Some(contrast) = parts.next() {
            theme.contrast = Contrast::from_id(contrast)?;
        }
        parts.next().is_none().then_some(theme)
    }

    /// Its words as [`Theme::from_words`] reads them, joined by `-`
    /// for a file name: `frost-dark-high`.
    pub fn slug(self) -> String {
        format!(
            "{}-{}-{}",
            self.name.id(),
            self.mode.id(),
            self.contrast.id()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stored theme id opens in the scheme THEMES §13 maps it to.
    #[test]
    fn each_old_theme_opens_in_its_new_scheme() {
        use Contrast::{High, Standard};
        use Mode::{Dark, Light};
        let cases = [
            ("dark", ThemeName::Cobalt, Dark, Standard),
            ("light", ThemeName::Cobalt, Light, Standard),
            ("cobalt", ThemeName::Cobalt, Light, Standard),
            ("nord", ThemeName::Frost, Dark, Standard),
            ("catppuccin", ThemeName::Lavender, Dark, Standard),
            ("tokyo-night", ThemeName::Dusk, Dark, Standard),
            ("gruvbox", ThemeName::Lamplight, Dark, Standard),
            ("rose-pine", ThemeName::Rose, Light, Standard),
            ("bitcoin-orange", ThemeName::BitcoinOrange, Dark, Standard),
            (
                "bitcoin-orange-light",
                ThemeName::BitcoinOrange,
                Light,
                Standard,
            ),
            ("lamplight", ThemeName::Lamplight, Light, Standard),
            ("mars", ThemeName::Mars, Dark, Standard),
            ("high-contrast-dark", ThemeName::Monochrome, Dark, High),
            ("high-contrast-light", ThemeName::Cobalt, Light, High),
            ("forest", ThemeName::Forest, Light, Standard),
        ];
        for (id, name, mode, contrast) in cases {
            assert_eq!(
                Theme::from_stored(id),
                Some(Theme::new(name, mode, contrast)),
                "{id}"
            );
        }
        assert_eq!(Theme::from_stored("solarized"), None);
    }
}
