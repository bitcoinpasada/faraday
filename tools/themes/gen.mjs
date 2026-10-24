// Writes faraday/faraday-ui/src/pal/schemes.rs, Faraday's colour tables,
// from Google's material-color-utilities vendored under
// tools/reference/material-color-utilities/ (docs/THEMES.md §10).
//
//   node tools/themes/gen.mjs [OUT]
//
// OUT defaults to the shipped table; tools/lint-themes.sh writes a
// temporary one and compares. Offline: nothing is fetched. Needs Node 22
// or later (`module.registerHooks`, for the resolve shim).
import './resolve.mjs';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const ref = join(root, 'tools', 'reference', 'material-color-utilities');
const load = (p) => import(join(ref, p));

const { Hct } = await load('hct/hct.js');
const { Blend } = await load('blend/blend.js');
const { TonalPalette } = await load('palettes/tonal_palette.js');
const { DynamicColor } = await load('dynamiccolor/dynamic_color.js');
const { ContrastCurve } = await load('dynamiccolor/contrast_curve.js');
const { ToneDeltaPair } = await load('dynamiccolor/tone_delta_pair.js');
const { MaterialDynamicColors } = await load('dynamiccolor/material_dynamic_colors.js');
const { Variant } = await load('dynamiccolor/variant.js');
const { SchemeTonalSpot } = await load('scheme/scheme_tonal_spot.js');
const { SchemeFidelity } = await load('scheme/scheme_fidelity.js');
const { SchemeNeutral } = await load('scheme/scheme_neutral.js');
const { SchemeMonochrome } = await load('scheme/scheme_monochrome.js');

// The reference's commit, named in the table's header (THEMES §10.1).
const COMMIT = 'eeaf82b8e11bf20f6d8da7c76336575b69e79e01';

// The themes (THEMES §2): Rust name, stored id, seed, scheme style, and
// the custom colours left unharmonised (§5).
const THEMES = [
  ['Cobalt', 'cobalt', '0047AB', 'fidelity', []],
  ['BitcoinOrange', 'bitcoin-orange', 'F7931A', 'fidelity', ['warning']],
  ['Lamplight', 'lamplight', 'B8860B', 'tonal spot', ['warning']],
  ['Mars', 'mars', 'B7410E', 'tonal spot', []],
  ['Forest', 'forest', '2E6B3F', 'tonal spot', ['ok']],
  ['Frost', 'frost', '6E8CA0', 'neutral', []],
  ['Dusk', 'dusk', '3D3A94', 'tonal spot', []],
  ['Lavender', 'lavender', 'A47CC4', 'tonal spot', []],
  ['Rose', 'rose', 'B76E79', 'tonal spot', []],
  ['Monochrome', 'monochrome', '808080', 'monochrome', []],
];
const STYLES = {
  'tonal spot': SchemeTonalSpot,
  fidelity: SchemeFidelity,
  neutral: SchemeNeutral,
  monochrome: SchemeMonochrome,
};

// The custom colours' source colours (THEMES §5): OK and Warning,
// harmonised with the seed unless a theme leaves them raw; the codex32
// booklet's checksum pink, never harmonised, so the checksum squares are
// the booklet's pink in every theme.
const CUSTOM = [
  ['ok', 0xff2e7d32, true],
  ['warning', 0xffffb300, true],
  ['checksum', 0xffc57d81, false],
];

// Mode, then contrast, in the order of each theme's six rows.
const MODES = [['light', false], ['dark', true]];
const CONTRASTS = [['standard', 0], ['medium', 0.5], ['high', 1]];

const m = new MaterialDynamicColors();

// The roles a scheme gives (THEMES §4), in row order: Rust name, the
// role's Material name, and how it is made.
const ROLES = [
  ['primary', 'primary'],
  ['onPrimary', 'on-primary'],
  ['primaryContainer', 'primary-container'],
  ['onPrimaryContainer', 'on-primary-container'],
  ['inversePrimary', 'inverse-primary'],
  ['secondary', 'secondary'],
  ['onSecondary', 'on-secondary'],
  ['secondaryContainer', 'secondary-container'],
  ['onSecondaryContainer', 'on-secondary-container'],
  ['tertiary', 'tertiary'],
  ['onTertiary', 'on-tertiary'],
  ['tertiaryContainer', 'tertiary-container'],
  ['onTertiaryContainer', 'on-tertiary-container'],
  ['error', 'error'],
  ['onError', 'on-error'],
  ['errorContainer', 'error-container'],
  ['onErrorContainer', 'on-error-container'],
  ['surface', 'surface'],
  ['surfaceDim', 'surface-dim'],
  ['surfaceBright', 'surface-bright'],
  ['surfaceContainerLowest', 'surface-container-lowest'],
  ['surfaceContainerLow', 'surface-container-low'],
  ['surfaceContainer', 'surface-container'],
  ['surfaceContainerHigh', 'surface-container-high'],
  ['surfaceContainerHighest', 'surface-container-highest'],
  ['onSurface', 'on-surface'],
  ['onSurfaceVariant', 'on-surface-variant'],
  ['outline', 'outline'],
  ['outlineVariant', 'outline-variant'],
  ['inverseSurface', 'inverse-surface'],
  ['inverseOnSurface', 'inverse-on-surface'],
  ['scrim', 'scrim'],
  ['shadow', 'shadow'],
  ['ok', 'ok', 'custom'],
  ['onOk', 'on-ok', 'custom'],
  ['okContainer', 'ok-container', 'custom'],
  ['onOkContainer', 'on-ok-container', 'custom'],
  ['warning', 'warning', 'custom'],
  ['onWarning', 'on-warning', 'custom'],
  ['warningContainer', 'warning-container', 'custom'],
  ['onWarningContainer', 'on-warning-container', 'custom'],
  ['checksum', 'checksum', 'custom'],
  ['onChecksum', 'on-checksum', 'custom'],
  ['checksumContainer', 'checksum-container', 'custom'],
  ['onChecksumContainer', 'on-checksum-container', 'custom'],
];

// A custom colour's four roles, built as ColorSpec2021 builds the error
// roles, with the custom palette in place of the error palette: the same
// tones, contrast curves, backgrounds and tone-delta pair (THEMES §5).
function custom(name, palette) {
  const highest = (s) => m.highestSurface(s);
  const main = () =>
    DynamicColor.fromPalette({
      name,
      palette: () => palette,
      tone: (s) => (s.isDark ? 80 : 40),
      isBackground: true,
      background: highest,
      contrastCurve: () => new ContrastCurve(3, 4.5, 7, 7),
      toneDeltaPair: () => new ToneDeltaPair(container(), main(), 10, 'nearer', false),
    });
  const on = () =>
    DynamicColor.fromPalette({
      name: `on_${name}`,
      palette: () => palette,
      tone: (s) => (s.isDark ? 20 : 100),
      background: () => main(),
      contrastCurve: () => new ContrastCurve(4.5, 7, 11, 21),
    });
  const container = () =>
    DynamicColor.fromPalette({
      name: `${name}_container`,
      palette: () => palette,
      tone: (s) => (s.isDark ? 30 : 90),
      isBackground: true,
      background: highest,
      contrastCurve: () => new ContrastCurve(1, 1, 3, 4.5),
      toneDeltaPair: () => new ToneDeltaPair(container(), main(), 10, 'nearer', false),
    });
  const onContainer = () =>
    DynamicColor.fromPalette({
      name: `on_${name}_container`,
      palette: () => palette,
      tone: (s) => {
        if (s.variant === Variant.MONOCHROME) {
          return s.isDark ? 90 : 10;
        }
        return s.isDark ? 90 : 30;
      },
      background: () => container(),
      contrastCurve: () => new ContrastCurve(3, 4.5, 7, 11),
    });
  return { main: main(), on: on(), container: container(), onContainer: onContainer() };
}

const hex = (argb) => '0x' + (argb & 0xffffff).toString(16).padStart(6, '0').toUpperCase();

function scheme(seed, style, raw, dark, contrast) {
  const s = new STYLES[style](Hct.fromInt(0xff000000 | parseInt(seed, 16)), dark, contrast);
  const own = {};
  for (const [name, source, harmonised] of CUSTOM) {
    const keep = !harmonised || raw.includes(name) || s.variant === Variant.MONOCHROME;
    const palette = TonalPalette.fromInt(keep ? source : Blend.harmonize(source, s.sourceColorArgb));
    const c = custom(name, palette);
    const cap = name[0].toUpperCase() + name.slice(1);
    own[name] = c.main;
    own[`on${cap}`] = c.on;
    own[`${name}Container`] = c.container;
    own[`on${cap}Container`] = c.onContainer;
  }
  return ROLES.map(([key, , kind]) => (kind === 'custom' ? own[key] : m[key]()).getArgb(s));
}

const pascal = (k) => k[0].toUpperCase() + k.slice(1);

let out = `// Generated by tools/themes/gen.mjs from Google's material-color-utilities
// 0.4.0, commit ${COMMIT},
// vendored under tools/reference/material-color-utilities/. Not edited by
// hand: \`just themes\` writes it, and tools/lint-themes.sh fails when it
// differs from what the reference makes.

//! Faraday's colour schemes (\`docs/THEMES.md\`): each theme's seed and
//! style made by Material 3's colour system, in Light and Dark at
//! Standard, Medium and High contrast. One scheme a row, its roles'
//! colours in [\`Role\`]'s order.

/// Material 3's colour roles (\`docs/THEMES.md\` §4), in the order a row
/// of the schemes gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
`;
for (const [key, material, kind] of ROLES) {
  const what = kind === 'custom' ? `Faraday's custom colour role ${material} (\`docs/THEMES.md\` §5).` : `Material's ${material}.`;
  out += `    /// ${what}\n    ${pascal(key)},\n`;
}
out += `}

impl Role {
    /// Every role, in row order.
    pub const ALL: [Role; ${ROLES.length}] = [
`;
for (const [key] of ROLES) out += `        Role::${pascal(key)},\n`;
out += `    ];
}

/// How many roles a scheme gives.
pub const ROLE_COUNT: usize = ${ROLES.length};

/// The named themes (\`docs/THEMES.md\` §2), each a seed colour and a
/// Material scheme style, in the order of the schemes' rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemeName {
`;
THEMES.forEach(([rust, , seed, style], i) => {
  if (i === 0) out += `    /// Seed #${seed}, ${style}. The default.\n    #[default]\n`;
  else out += `    /// Seed #${seed}, ${style}.\n`;
  out += `    ${rust},\n`;
});
out += `}

impl ThemeName {
    /// Every theme, in row order.
    pub const ALL: [ThemeName; ${THEMES.length}] = [
`;
for (const [rust] of THEMES) out += `        ThemeName::${rust},\n`;
out += `    ];

    /// The word a settings file writes it as.
    pub fn id(self) -> &'static str {
        match self {
`;
for (const [rust, id] of THEMES) out += `            ThemeName::${rust} => "${id}",\n`;
out += `        }
    }
}

/// Each scheme's colours as 0xRRGGBB, in [\`Role\`]'s order: each theme in
/// [\`ThemeName::ALL\`]'s order, Light then Dark, each at Standard, Medium
/// and High contrast.
#[rustfmt::skip]
pub(super) const SCHEMES: [[u32; ROLE_COUNT]; ${THEMES.length * MODES.length * CONTRASTS.length}] = [
`;
for (const [, id, seed, style, raw] of THEMES) {
  for (const [mode, dark] of MODES) {
    for (const [contrast, level] of CONTRASTS) {
      const row = scheme(seed, style, raw, dark, level).map(hex).join(', ');
      out += `    [${row}], // ${id} ${mode} ${contrast}\n`;
    }
  }
}
out += '];\n';

writeFileSync(process.argv[2] ?? join(root, 'faraday', 'faraday-ui', 'src', 'pal', 'schemes.rs'), out);
