//! The type ramp: Material's roles in the faces Faraday has (09 §3.2).

use crate::dims::SizeClass;

// ---------------------------------------------------------------------
// Type ramp
// ---------------------------------------------------------------------

/// The face a type role is set in: Material's weight drawn in the faces
/// Faraday has (`docs/LAYOUT.md` §8). Material's regular, 400, is Noto
/// Sans; its medium, 500, is Noto Sans SemiBold, the one heavier text
/// face Faraday carries. `Mono` is Noto Sans Mono at the role's size
/// and line, for what is read character by character (DESIGN §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeFace {
    /// Material's regular: Noto Sans.
    Regular,
    /// Material's medium: Noto Sans SemiBold.
    Medium,
    /// Noto Sans Mono.
    Mono,
}

/// One of Material 3's type roles (`docs/LAYOUT.md` §8): its size, its
/// line height and its face. `faraday-ui` draws text by role
/// (`Ui::text_as`, `Ui::wrap_as` and the rest), its lines spaced by
/// `line`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeRole {
    /// The size in dp (Material's sp).
    pub size: f32,
    /// The line height in dp: how far apart the lines of a paragraph in
    /// this role stand.
    pub line: f32,
    /// The face.
    pub face: TypeFace,
}

impl TypeRole {
    /// This role in the monospace face, at its size and line: data in a
    /// role's place (a fingerprint in a body line).
    pub const fn mono(self) -> TypeRole {
        TypeRole {
            face: TypeFace::Mono,
            ..self
        }
    }

    /// This role in the medium face: Faraday's emphasis inside a line of
    /// a regular role, where Material would set the same size heavier.
    pub const fn medium(self) -> TypeRole {
        TypeRole {
            face: TypeFace::Medium,
            ..self
        }
    }

    /// This role in the regular face.
    pub const fn regular(self) -> TypeRole {
        TypeRole {
            face: TypeFace::Regular,
            ..self
        }
    }
}

pub const fn role(size: f32, line: f32, face: TypeFace) -> TypeRole {
    TypeRole { size, line, face }
}

/// Material's headline medium, 28 on 36, regular: a page's title.
pub const HEADLINE_M: TypeRole = role(28.0, 36.0, TypeFace::Regular);
/// Material's headline small, 24 on 32, regular: a sheet's head and a
/// step flow's heading.
pub const HEADLINE_S: TypeRole = role(24.0, 32.0, TypeFace::Regular);
/// Material's title large, 22 on 28, regular: the top app bar's title
/// and a sheet's head on the small panel.
pub const TITLE_L: TypeRole = role(22.0, 28.0, TypeFace::Regular);
/// Material's title medium, 16 on 24, medium.
pub const TITLE_M: TypeRole = role(16.0, 24.0, TypeFace::Medium);
/// Material's title small, 14 on 20, medium: a section's label.
pub const TITLE_S: TypeRole = role(14.0, 20.0, TypeFace::Medium);
/// Material's body large, 16 on 24, regular: a list item's headline.
pub const BODY_L: TypeRole = role(16.0, 24.0, TypeFace::Regular);
/// Material's body medium, 14 on 20, regular: running text, a list
/// item's supporting line, the snackbar.
pub const BODY_M: TypeRole = role(14.0, 20.0, TypeFace::Regular);
/// Material's body small, 12 on 16, regular: a field's label and its
/// supporting or error line, a plain tooltip.
pub const BODY_S: TypeRole = role(12.0, 16.0, TypeFace::Regular);
/// Material's label large, 14 on 20, medium: a button's label.
pub const LABEL_L: TypeRole = role(14.0, 20.0, TypeFace::Medium);
/// Material's label medium, 12 on 16, medium: a navigation label.
pub const LABEL_M: TypeRole = role(12.0, 16.0, TypeFace::Medium);
/// Material's label small, 11 on 16, medium: a badge, a list item's
/// trailing text.
pub const LABEL_S: TypeRole = role(11.0, 16.0, TypeFace::Medium);

/// A sheet's head: [`HEADLINE_S`], and [`TITLE_L`] on the small panel
/// (`docs/LAYOUT.md` §8).
pub const fn sheet_head(compact: bool) -> TypeRole {
    if compact { TITLE_L } else { HEADLINE_S }
}

/// Title size: app bar titles and headings. Retired by the type roles
/// (`docs/LAYOUT.md` §8): kept only until the screens that still write
/// it move to [`TITLE_L`] or [`HEADLINE_S`]; nothing new reads it.
pub const TITLE: f32 = 20.0;
/// Body text size.
pub const BODY: f32 = 16.0;
/// Label size: row labels and secondary lines.
pub const LABEL: f32 = 14.0;
/// Caption size: notes, hints and badges.
pub const CAPTION: f32 = 12.0;
/// The smallest the wordmark's capitals are drawn in a bar too narrow for
/// the name beside its word: under it the bar shows the mark alone.
pub const WORDMARK_CAP_MIN: f32 = 8.0;
/// The word of an app's name set under FARADAY in such a bar: its
/// capitals' height as a share of FARADAY's.
pub const WORDMARK_STACK_WORD: f32 = 0.7;
/// The gap between FARADAY's foot and the top of the word under it, as a
/// share of FARADAY's capitals' height.
pub const WORDMARK_STACK_GAP: f32 = 0.3;

/// The type ramp's scale for a size class, in percent
/// (`docs/DESIGN.md` §3).
///
/// One ramp, three densities. `Small` is the size the panel was designed
/// at; a phone reads at the same distance from a screen twice as tall,
/// and a desktop window is read from twice as far away as a phone at the
/// same dp size. Only type scales: touch targets, padding and the 8 dp
/// grid are the same everywhere. [`crate::geom::Scale::for_class`]
/// applies it.
pub const fn type_scale_pct(class: SizeClass) -> u16 {
    match class {
        SizeClass::Small => 100,
        SizeClass::Mobile => 110,
        SizeClass::Wide => 130,
    }
}

/// Whether `class` sets its text in Noto Sans SemiCondensed rather than
/// Noto Sans. A menu row on the 268 dp panel leaves its label 144 dp
/// between the icon and the chevron, and a label such as "Create Codex32
/// shares" is wider than that in the ordinary width; the narrow cut is
/// the same design with the same vertical metrics, so only widths
/// change.
pub const fn narrow_text(class: SizeClass) -> bool {
    matches!(class, SizeClass::Small)
}
