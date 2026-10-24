//! Which scheme the screens are drawn in (`docs/THEMES.md` §3): a theme,
//! a mode and a contrast level, and the fourteen themes Settings offered
//! before them, each now one of those triples (§13).

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
        let row = (self.name as usize * Mode::ALL.len() + self.mode as usize) * Contrast::ALL.len()
            + self.contrast as usize;
        &pal::SCHEMES[row]
    }

    /// `role`'s colour in this scheme.
    pub fn role(self, role: Role) -> Color {
        self.scheme()[role as usize]
    }

    /// `paint` as this scheme draws it: a role looked up, a fixed colour
    /// as it is.
    pub fn color(self, paint: impl Into<Paint>) -> Color {
        paint.into().in_scheme(self.scheme())
    }

    /// Dark text on a light page: shadows are lighter.
    pub fn is_light(self) -> bool {
        self.mode == Mode::Light
    }

    /// Text 7:1 or more on every surface: Faraday's own high contrast
    /// rules apply (`docs/THEMES.md` §9).
    pub fn is_high_contrast(self) -> bool {
        self.contrast == Contrast::High
    }
}

/// The themes Settings offers until it offers Theme, Mode and Contrast
/// (`docs/THEMES.md` §12): the fourteen it offered before, each drawn as
/// the scheme a stored setting of it opens in (§13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Preset {
    /// Cobalt, Dark.
    Dark,
    /// Cobalt, Light.
    Light,
    /// Cobalt, Light: the one a first start is in.
    #[default]
    Cobalt,
    /// Frost, Dark.
    Nord,
    /// Lavender, Dark.
    Catppuccin,
    /// Dusk, Dark.
    TokyoNight,
    /// Lamplight, Dark.
    Gruvbox,
    /// Rose, Light.
    RosePine,
    /// Bitcoin Orange, Dark.
    BitcoinOrange,
    /// Bitcoin Orange, Light.
    BitcoinOrangeLight,
    /// Lamplight, Light.
    Lamplight,
    /// Monochrome, Dark, High contrast.
    HighContrastDark,
    /// Cobalt, Light, High contrast.
    HighContrastLight,
    /// Mars, Dark.
    Mars,
}

impl Preset {
    /// Every preset. Settings offers them by [`Preset::group`], each
    /// group in order of its names ([`ThemeGroup::themes`]).
    pub const ALL: [Preset; 14] = [
        Preset::Dark,
        Preset::Light,
        Preset::Cobalt,
        Preset::Nord,
        Preset::Catppuccin,
        Preset::TokyoNight,
        Preset::Gruvbox,
        Preset::RosePine,
        Preset::BitcoinOrange,
        Preset::BitcoinOrangeLight,
        Preset::Lamplight,
        Preset::HighContrastDark,
        Preset::HighContrastLight,
        Preset::Mars,
    ];

    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            Preset::Dark => EN.ui_dark,
            Preset::Light => EN.ui_light,
            Preset::Cobalt => EN.ui_cobalt,
            Preset::Nord => EN.ui_nord,
            Preset::Catppuccin => EN.ui_catppuccin,
            Preset::TokyoNight => EN.ui_tokyo_night,
            Preset::Gruvbox => EN.ui_gruvbox,
            Preset::RosePine => EN.ui_rose_pine,
            Preset::BitcoinOrange => EN.ui_bitcoin_orange,
            Preset::BitcoinOrangeLight => EN.ui_bitcoin_orange_light,
            Preset::Lamplight => EN.ui_lamplight,
            Preset::HighContrastDark => EN.ui_high_contrast_dark,
            Preset::HighContrastLight => EN.ui_high_contrast_light,
            Preset::Mars => EN.ui_mars,
        }
    }

    /// The word the kept settings write it as (`memory.rs`).
    pub fn id(self) -> &'static str {
        match self {
            Preset::Dark => "dark",
            Preset::Light => "light",
            Preset::Cobalt => "cobalt",
            Preset::Nord => "nord",
            Preset::Catppuccin => "catppuccin",
            Preset::TokyoNight => "tokyo-night",
            Preset::Gruvbox => "gruvbox",
            Preset::RosePine => "rose-pine",
            Preset::BitcoinOrange => "bitcoin-orange",
            Preset::BitcoinOrangeLight => "bitcoin-orange-light",
            Preset::Lamplight => "lamplight",
            Preset::HighContrastDark => "high-contrast-dark",
            Preset::HighContrastLight => "high-contrast-light",
            Preset::Mars => "mars",
        }
    }

    /// The preset a kept `id` names.
    pub fn from_id(id: &str) -> Option<Preset> {
        Preset::ALL.into_iter().find(|t| t.id() == id)
    }

    /// The scheme it is drawn in (`docs/THEMES.md` §13).
    pub fn theme(self) -> Theme {
        use Contrast::{High, Standard};
        use Mode::{Dark, Light};
        let (name, mode, contrast) = match self {
            Preset::Dark => (ThemeName::Cobalt, Dark, Standard),
            Preset::Light | Preset::Cobalt => (ThemeName::Cobalt, Light, Standard),
            Preset::Nord => (ThemeName::Frost, Dark, Standard),
            Preset::Catppuccin => (ThemeName::Lavender, Dark, Standard),
            Preset::TokyoNight => (ThemeName::Dusk, Dark, Standard),
            Preset::Gruvbox => (ThemeName::Lamplight, Dark, Standard),
            Preset::RosePine => (ThemeName::Rose, Light, Standard),
            Preset::BitcoinOrange => (ThemeName::BitcoinOrange, Dark, Standard),
            Preset::BitcoinOrangeLight => (ThemeName::BitcoinOrange, Light, Standard),
            Preset::Lamplight => (ThemeName::Lamplight, Light, Standard),
            Preset::HighContrastDark => (ThemeName::Monochrome, Dark, High),
            Preset::HighContrastLight => (ThemeName::Cobalt, Light, High),
            Preset::Mars => (ThemeName::Mars, Dark, Standard),
        };
        Theme::new(name, mode, contrast)
    }

    /// Dark text on a light page.
    pub fn is_light(self) -> bool {
        self.theme().is_light()
    }

    /// Drawn at High contrast.
    pub fn is_high_contrast(self) -> bool {
        self.theme().is_high_contrast()
    }

    /// The group Settings offers it under.
    pub fn group(self) -> ThemeGroup {
        if self.is_high_contrast() {
            ThemeGroup::HighContrast
        } else if self.is_light() {
            ThemeGroup::Light
        } else {
            ThemeGroup::Dark
        }
    }
}

/// The groups Settings offers the presets in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeGroup {
    /// Dark text on a light page.
    Light,
    /// Light text on a dark page.
    Dark,
    /// Text 7:1 or more, light or dark.
    HighContrast,
}

impl ThemeGroup {
    /// Every group, in the order Settings offers them.
    pub const ALL: [ThemeGroup; 3] = [
        ThemeGroup::Light,
        ThemeGroup::Dark,
        ThemeGroup::HighContrast,
    ];

    /// The label Settings puts over the group.
    pub fn name(self) -> &'static str {
        match self {
            ThemeGroup::Light => EN.ui_themes_light,
            ThemeGroup::Dark => EN.ui_themes_dark,
            ThemeGroup::HighContrast => EN.ui_themes_high_contrast,
        }
    }

    /// The presets in this group ([`Preset::group`]), in order of their
    /// names.
    pub fn themes(self) -> impl Iterator<Item = Preset> {
        let mut all = Preset::ALL;
        all.sort_unstable_by_key(|t| t.name());
        all.into_iter().filter(move |t| t.group() == self)
    }
}

#[cfg(test)]
mod tests {
    // The shipped schemes read as Material's colour system says they must
    // (`docs/THEMES.md` §11.1), and the pairs Faraday's screens draw read
    // on every surface (§11.2).

    use super::*;
    use crate::pal::Role::{self, *};

    /// WCAG relative luminance.
    fn luminance(c: Color) -> f64 {
        let lin = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
    }

    /// WCAG contrast ratio.
    fn ratio(a: Color, b: Color) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    fn level(theme: Theme) -> usize {
        theme.contrast as usize
    }

    /// Black or white: as far as a tone can go.
    fn extreme(c: Color) -> bool {
        matches!((c.r, c.g, c.b), (0, 0, 0) | (255, 255, 255))
    }

    /// The surfaces nearest any text in a scheme, as Material's test takes
    /// them.
    const LIMITING: [Role; 2] = [SurfaceDim, SurfaceBright];

    /// Every surface role.
    const SURFACES: [Role; 8] = [
        Surface,
        SurfaceDim,
        SurfaceBright,
        SurfaceContainerLowest,
        SurfaceContainerLow,
        SurfaceContainer,
        SurfaceContainerHigh,
        SurfaceContainerHighest,
    ];

    /// Material's constraint conformance test (material-color-utilities,
    /// `dynamiccolor/dynamic_color_test.ts`, at the pinned commit), with
    /// ColorSpec2021's targets at Standard, Medium and High, and Faraday's
    /// custom colours under the error roles' targets: a target of 4.5 or
    /// less is a floor, met to within 0.05; one above 4.5 is met, or one
    /// of the two colours is already black or white.
    #[test]
    fn every_scheme_meets_materials_contrast_constraints() {
        // (foregrounds, backgrounds, each against each, targets)
        let every: [(&[Role], &[Role], [f64; 3]); 7] = [
            (&[OnSurface], &LIMITING, [7.0, 11.0, 21.0]),
            (&[OnSurfaceVariant], &LIMITING, [4.5, 7.0, 11.0]),
            (
                &[Primary, Secondary, Tertiary, Error, Ok, Warning, Checksum],
                &LIMITING,
                [4.5, 7.0, 7.0],
            ),
            (&[Outline], &LIMITING, [3.0, 4.5, 7.0]),
            (
                &[
                    PrimaryContainer,
                    SecondaryContainer,
                    TertiaryContainer,
                    ErrorContainer,
                    OutlineVariant,
                    OkContainer,
                    WarningContainer,
                    ChecksumContainer,
                ],
                &LIMITING,
                [0.0, 3.0, 4.5],
            ),
            (&[InverseOnSurface], &[InverseSurface], [7.0, 11.0, 21.0]),
            (&[InversePrimary], &[InverseSurface], [4.5, 7.0, 7.0]),
        ];
        // (foreground, the background it is on, targets)
        let on = [
            (OnPrimary, Primary, [7.0, 11.0, 21.0]),
            (OnSecondary, Secondary, [7.0, 11.0, 21.0]),
            (OnTertiary, Tertiary, [7.0, 11.0, 21.0]),
            (OnError, Error, [7.0, 11.0, 21.0]),
            (OnOk, Ok, [7.0, 11.0, 21.0]),
            (OnWarning, Warning, [7.0, 11.0, 21.0]),
            (OnChecksum, Checksum, [7.0, 11.0, 21.0]),
            (OnPrimaryContainer, PrimaryContainer, [4.5, 7.0, 11.0]),
            (OnSecondaryContainer, SecondaryContainer, [4.5, 7.0, 11.0]),
            (OnTertiaryContainer, TertiaryContainer, [4.5, 7.0, 11.0]),
            (OnErrorContainer, ErrorContainer, [4.5, 7.0, 11.0]),
            (OnOkContainer, OkContainer, [4.5, 7.0, 11.0]),
            (OnWarningContainer, WarningContainer, [4.5, 7.0, 11.0]),
            (OnChecksumContainer, ChecksumContainer, [4.5, 7.0, 11.0]),
        ];
        let mut short = Vec::new();
        for theme in Theme::all() {
            let mut check = |fg: Role, bg: Role, need: f64| {
                let (f, b) = (theme.role(fg), theme.role(bg));
                let got = ratio(f, b);
                let fails = if need <= 4.5 {
                    got < need - 0.05
                } else {
                    !extreme(f) && !extreme(b) && got < need - 0.05
                };
                if fails {
                    short.push(format!(
                        "{theme:?}: {fg:?} on {bg:?} is {got:.2}:1, needs {need}"
                    ));
                }
            };
            for (fgs, bgs, need) in every {
                for &fg in fgs {
                    for &bg in bgs {
                        check(fg, bg, need[level(theme)]);
                    }
                }
            }
            for (fg, bg, need) in on {
                check(fg, bg, need[level(theme)]);
            }
        }
        assert!(short.is_empty(), "{short:#?}");
    }

    /// What the screens rely on: text, the accent, status colours and a
    /// control's edge on every surface; at High contrast text 7:1, as the
    /// high contrast themes promised.
    #[test]
    fn text_tones_and_edges_read_on_every_surface() {
        let mut short = Vec::new();
        for theme in Theme::all() {
            let text = if theme.is_high_contrast() { 7.0 } else { 4.5 };
            for bg in SURFACES {
                let pairs = [
                    (OnSurface, text),
                    (OnSurfaceVariant, text),
                    (Primary, text),
                    (Error, text),
                    (Ok, text),
                    (Warning, text),
                    (Outline, 3.0),
                ];
                for (fg, need) in pairs {
                    let got = ratio(theme.role(fg), theme.role(bg));
                    if got < need {
                        short.push(format!("{theme:?}: {fg:?} on {bg:?} is {got:.2}:1"));
                    }
                }
            }
        }
        assert!(short.is_empty(), "{short:#?}");
    }

    /// The backup board's boxes, the codex32 worksheets and the badges
    /// (THEMES §6): each box's words on its fill and band, in the paints
    /// the screens draw them in, 4.5:1 (7:1 at High contrast), and each
    /// box's edge 3:1 against what is round it. What the screens draw is
    /// checked on the frame by faraday-core's `fills_read`.
    #[test]
    fn the_boards_boxes_and_the_worksheets_read_in_every_scheme() {
        use crate::pal::*;
        let mut short = Vec::new();
        for theme in Theme::all() {
            let text = if theme.is_high_contrast() { 7.0 } else { 4.5 };
            let c = |p: Paint| theme.color(p);
            // (what, on what, needs)
            let pairs = [
                ("place name on its fill", PLACE_TEXT, PLACE_FILL, text),
                ("place name on its band", PLACE_TEXT, PLACE_BAND, text),
                ("stick name", STICK_TEXT, STICK_FILL, text),
                ("paper", PAPER_TEXT, PAPER_FILL, text),
                ("files", FILES_TEXT, FILES_FILL, text),
                ("vault band", VAULT_ON, VAULT_EDGE, text),
                (
                    "checksum square",
                    CODEX32_ON_CHECKSUM,
                    CODEX32_CHECKSUM_FILL,
                    text,
                ),
                ("table row", TEXT, CODEX32_LOOKUP_FILL, text),
                ("a badge's number", MUTED, INNER, text),
                ("a pressed box's text", TEXT, BG_PRESSED, text),
                ("place edge", PLACE_EDGE, SURFACE, 3.0),
                ("stick edge", STICK_EDGE, PLACE_FILL, 3.0),
                ("vault edge", VAULT_EDGE, STICK_FILL, 3.0),
                ("files edge", FILES_EDGE, STICK_FILL, 3.0),
                ("paper edge", PAPER_EDGE, PLACE_FILL, 3.0),
                ("chip edge", CHIP_EDGE, SURFACE, 3.0),
            ];
            for (what, fg, bg, need) in pairs {
                let got = ratio(c(fg), c(bg));
                if got < need {
                    short.push(format!("{theme:?}: {what} is {got:.2}:1"));
                }
            }
        }
        assert!(short.is_empty(), "{short:#?}");
    }

    /// A stored preset opens in the scheme THEMES §13 maps it to.
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
        ];
        for (id, name, mode, contrast) in cases {
            let preset = Preset::from_id(id).expect(id);
            assert_eq!(preset.theme(), Theme::new(name, mode, contrast), "{id}");
        }
    }
}
