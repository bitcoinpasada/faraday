//! Material 3's elevation shadows (09 §3.2).

/// One layer of a Material 3 elevation shadow, in dp: how far it falls
/// below what floats, its blur radius and how far it grows past it.
#[derive(Clone, Copy)]
pub struct ElevationLayer {
    /// Dp below what floats.
    pub y: f32,
    /// Blur radius in dp.
    pub blur: f32,
    /// Dp grown past it on every side.
    pub spread: f32,
}

const fn layer(y: f32, blur: f32, spread: f32) -> ElevationLayer {
    ElevationLayer { y, blur, spread }
}

/// The key layer of elevation levels 1 to 5 (Material Web's
/// `elevation/internal/_elevation.scss`).
pub const ELEVATION_KEY: [ElevationLayer; 5] = [
    layer(1.0, 2.0, 0.0),
    layer(1.0, 2.0, 0.0),
    layer(1.0, 3.0, 0.0),
    layer(2.0, 3.0, 0.0),
    layer(4.0, 4.0, 0.0),
];
/// The ambient layer of elevation levels 1 to 5.
pub const ELEVATION_AMBIENT: [ElevationLayer; 5] = [
    layer(1.0, 3.0, 1.0),
    layer(2.0, 6.0, 2.0),
    layer(4.0, 8.0, 3.0),
    layer(6.0, 10.0, 4.0),
    layer(8.0, 12.0, 6.0),
];
/// The most rows a shadow's ink reaches past its shape, in units: the
/// ambient layer of the highest level, blurred and moved down.
pub const SHADOW_REACH: f32 = 24.0;
/// Opacity (of 255) of the key layer: 30% black.
pub const ELEVATION_KEY_OPACITY: u8 = 77;
/// Opacity (of 255) of the ambient layer: 15% black.
pub const ELEVATION_AMBIENT_OPACITY: u8 = 38;
/// Elevation level of a sheet, a raised card and a board chip.
pub const ELEVATION_SHEET: u8 = 1;
/// Elevation level of a menu.
pub const ELEVATION_MENU: u8 = 2;
/// Elevation level of a floating action button, a toast and a snackbar.
pub const ELEVATION_FAB: u8 = 3;

/// A dialog's elevation level: Material 3 raises a dialog to level 3.
pub const ELEVATION_DIALOG: u8 = 3;
/// Elevation level of a floating action button under the pointer, and
/// of a dragged item.
pub const ELEVATION_RAISED: u8 = 4;
