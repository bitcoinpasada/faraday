//! One frame's drawing context, generic over the press type (04 §7.1).

use core::marker::PhantomData;
use std::cell::RefCell;

use faraday_draw::canvas::{Canvas, Ink};
use faraday_draw::fonts::{Family, SizedFace};
use faraday_draw::geom::Scale;
use faraday_draw::icons::Icon;
use faraday_draw::text::{self, TextAlign};
use faraday_draw::{Font, Rect};
use faraday_port::values::{Anchor, Pace, Point};
use faraday_text::Text;
use faraday_tokens::Theme;
use faraday_tokens::palette::{Color, Paint, Role};

use crate::Pressed;
use crate::field::{Edit, FieldAt};
use crate::frame_out::{FrameOut, Laid};
use crate::hits::Hit;
use crate::motion::MotionStore;
use crate::region::Scrolled;

/// The pointer and the focus as the last frame left them, kept by the runtime (04 §7.1).
pub struct Pointer<R> {
    /// Where the pointer is, while it is over the window.
    pub at: Option<Point>,
    /// A button or a finger is down.
    pub down: bool,
    /// The press the keyboard is on: ringed, and Enter or Space presses it.
    pub focused: Option<R>,
}

/// A face: text, emphasis, or data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W {
    /// Noto Sans.
    R,
    /// Noto Sans SemiBold.
    S,
    /// Noto Sans Mono.
    M,
}
/// A glyph: one of the icon face's, or one Faraday draws itself where the
/// face has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// One of the icon face's.
    Icon(Icon),
    /// A USB stick (`docs/BACKUP-FLOW.md` §17.17): Faraday's own drawing.
    Stick,
}

impl From<Icon> for Glyph {
    fn from(icon: Icon) -> Glyph {
        Glyph::Icon(icon)
    }
}

/// One frame's drawing context: canvas, scale, theme, pointer state, motion store, lift.
pub struct Ui<'f, A: Pressed> {
    /// The canvas.
    pub c: &'f mut Canvas,
    /// Pixels per design unit.
    pub f: f32,
    /// The canvas's type ramp over its length scale.
    pub(crate) tr: f32,
    /// The person's text size factor: every type role's size and line height times it, 1
    /// inside [`Ui::fixed_text`].
    pub(crate) text: f32,
    /// The palette.
    pub theme: Theme,
    pub(crate) motion: &'f mut MotionStore,
    /// Where each press was drawn, lifted to the root's type (04 §7.1).
    pub(crate) hits: Vec<Hit<A>>,
    /// The Tab order: indices into `hits`, in drawing order.
    pub(crate) tab: Vec<usize>,
    /// Where the pointer is, and whether a button or a finger is down.
    pub(crate) at: Option<Point>,
    pub(crate) down: bool,
    /// Whether the press given is the one the keyboard is on, as the root's press, through
    /// every lift between here and the root.
    pub(crate) focus: Box<dyn Fn(&A) -> bool + 'f>,
    /// While a press draws its state layer: whether the pointer is over its target.
    pub(crate) over: Option<bool>,
    /// 0 for the page, then 1 for the lowest sheet over it and up (04 §7.1).
    pub layer: u8,
    /// Where design (0, 0) is, in design units.
    pub ox: f32,
    /// See [`Ui::ox`].
    pub oy: f32,
    /// Whole device pixels every place is moved down by after it is rounded.
    pub oy_px: i32,
    /// The faces looked up this frame, by font and scale.
    pub(crate) faces: RefCell<Vec<(Font, Scale, SizedFace)>>,
    /// Where the caret and the selection are in the focused field.
    pub edit: Edit<A>,
    /// Where each line of typing drew its characters this frame.
    pub fields: Vec<FieldAt<A>>,
    /// The regions this frame scrolls, as the page or sheet that drew them reported them.
    pub(crate) regions: Vec<Scrolled>,
    pub(crate) anchors: Vec<(Anchor, Rect)>,
    pub(crate) laid: Vec<(Anchor, Laid, A)>,
    /// The page drawn and the sheets over it, set by the templates.
    pub(crate) page: Anchor,
    pub(crate) sheets: Vec<Anchor>,
    /// A secret was drawn (04 §11).
    pub(crate) secret: bool,
    /// Motion wants another frame.
    pub(crate) moving: bool,
    /// The record of what was drawn is on (04 §9).
    pub(crate) ink: bool,
    /// How far the region being closed is shown stretched past an end, in pixels.
    pub(crate) stretch: i32,
    /// The display is a small panel: one column, a page per step.
    pub compact: bool,
    /// The side of a press's touch target, units.
    pub touch: f32,
    /// Notes are set one type step up (Body medium for Body small).
    pub note_up: bool,
    /// The text caret shows this frame: it blinks.
    pub caret_on: bool,
    /// A caret was drawn this frame, shown or in the off half of its blink.
    pub caret_drawn: bool,
    /// The foot of the caret drawn this frame, units down the window.
    pub caret_foot: Option<f32>,
    /// The field typing goes to, as drawn this frame: its box, line and caret, in pixels.
    pub focus_box: Option<Rect>,
    /// The press that field is pressed with: tells one focused field from another.
    pub focus_action: Option<A>,
    /// The last label a field drew: its top and the foot of the gap under it, units.
    pub(crate) label_at: Option<(f32, f32)>,
    /// Where this frame's floating action button stands, in the window's units (x, y, width,
    /// height): a snackbar drawn after it stands above it.
    pub fab_at: Option<(f32, f32, f32, f32)>,
    /// How many filled (primary) buttons this frame has drawn.
    pub primaries: usize,
    /// The tooltip anchors this frame drew.
    pub(crate) tips: Vec<crate::icon_button::TipAnchor<A>>,
    /// Hits are clipped to this rather than to the drawing's clip while a card's body is
    /// revealed.
    pub(crate) hit_clip: Option<Rect>,
    /// The hit being taken draws its own hover look.
    pub(crate) quiet_hover: bool,
    /// The hit being taken shows what it holds as drawn while pressed: a held secret.
    pub(crate) held: bool,
    /// The hit being taken draws its own focus ring too.
    pub(crate) own_ring: bool,
    /// The hit being taken is the press beside a sheet: no target, no place in the Tab order.
    pub(crate) around: bool,
    /// The control being drawn is refused and drawn dimmed, and still takes a press the app
    /// answers (`Offered::Refused` with `Drawn::Dimmed`): no layer, no ring, no Tab place.
    pub(crate) answering: bool,
    /// The hits taken this frame whose box ran past the side of their clip.
    pub cut: Vec<Rect>,
    /// The screen's back link and where in `hits` its own press is ([`Ui::back`]).
    pub(crate) back: Option<(A, usize)>,
    pub(crate) press: PhantomData<fn(A)>,
}

impl<'f, A: Pressed> Ui<'f, A> {
    /// The runtime's only.
    pub fn new(
        canvas: &'f mut Canvas,
        theme: Theme,
        scale: f32,
        motion: &'f mut MotionStore,
        pointer: &'f Pointer<A>,
        ink: bool,
    ) -> Self {
        let focused = pointer.focused.as_ref();
        let focus: Box<dyn Fn(&A) -> bool + 'f> = Box::new(move |a| focused == Some(a));
        let s = canvas.scale();
        let tr = s.type_factor() / s.factor();
        canvas.record_ink(ink);
        Ui {
            c: canvas,
            f: scale,
            tr,
            text: 1.0,
            theme,
            motion,
            hits: Vec::new(),
            tab: Vec::new(),
            at: pointer.at,
            down: pointer.down,
            focus,
            over: None,
            layer: 0,
            ox: 0.0,
            oy: 0.0,
            oy_px: 0,
            faces: RefCell::new(Vec::new()),
            edit: Edit::default(),
            fields: Vec::new(),
            regions: Vec::new(),
            anchors: Vec::new(),
            laid: Vec::new(),
            page: Anchor { scope: "", n: 0 },
            sheets: Vec::new(),
            secret: false,
            moving: false,
            ink,
            stretch: 0,
            compact: false,
            touch: faraday_tokens::dims::TOUCH,
            note_up: false,
            caret_on: true,
            caret_drawn: false,
            caret_foot: None,
            focus_box: None,
            focus_action: None,
            label_at: None,
            primaries: 0,
            fab_at: None,
            tips: Vec::new(),
            hit_clip: None,
            quiet_hover: false,
            held: false,
            own_ring: false,
            around: false,
            answering: false,
            cut: Vec::new(),
            back: None,
            press: PhantomData,
        }
    }

    /// The runtime's only.
    pub fn finish(self) -> FrameOut<A> {
        let ink = self.ink.then(|| Ink(self.c.ink().to_vec()));
        FrameOut {
            hits: self.hits,
            reach: self.tab,
            page: self.page,
            sheets: self.sheets,
            back: self.back,
            anchors: self.anchors,
            regions: self.regions,
            laid: self.laid,
            secret: self.secret,
            pace: if self.moving {
                Pace::Moving
            } else {
                Pace::Rest
            },
            ink,
        }
    }

    /// Takes the window's class from the canvas: a window under `WINDOW_MEDIUM` dp wide is
    /// compact, and its touch floor is `dims::touch`'s for it. The root's frame calls it once
    /// before it draws.
    pub fn size_class(&mut self) {
        let factor = self.c.scale().factor().max(f32::EPSILON);
        let window = faraday_tokens::dims::Window {
            width: self.c.width() as f32 / factor,
            height: self.c.height() as f32 / factor,
        };
        self.compact = window.class() == faraday_tokens::dims::WindowClass::Compact;
        self.touch = faraday_tokens::dims::touch(window);
    }

    /// A child page draws here; its presses are lifted to this one's.
    pub fn within<B: Pressed>(&mut self, lift: fn(B) -> A, f: impl FnOnce(&mut Ui<'_, B>)) {
        let child_edit = self.edit_for_child();
        let parent_focus = &self.focus;
        let focus: Box<dyn Fn(&B) -> bool + '_> = Box::new(move |b| parent_focus(&lift(b.clone())));
        let mut child: Ui<'_, B> = Ui {
            c: &mut *self.c,
            f: self.f,
            tr: self.tr,
            text: self.text,
            theme: self.theme,
            motion: &mut *self.motion,
            hits: Vec::new(),
            tab: Vec::new(),
            at: self.at,
            down: self.down,
            focus,
            over: None,
            layer: self.layer,
            ox: self.ox,
            oy: self.oy,
            oy_px: self.oy_px,
            faces: RefCell::new(Vec::new()),
            edit: child_edit,
            fields: Vec::new(),
            regions: Vec::new(),
            anchors: Vec::new(),
            laid: Vec::new(),
            page: self.page,
            sheets: Vec::new(),
            secret: false,
            moving: false,
            ink: self.ink,
            stretch: 0,
            compact: self.compact,
            touch: self.touch,
            note_up: self.note_up,
            caret_on: self.caret_on,
            caret_drawn: false,
            caret_foot: None,
            focus_box: None,
            focus_action: None,
            label_at: None,
            primaries: 0,
            fab_at: None,
            tips: Vec::new(),
            hit_clip: self.hit_clip,
            quiet_hover: false,
            held: false,
            own_ring: false,
            around: false,
            answering: false,
            cut: Vec::new(),
            back: None,
            press: PhantomData,
        };
        f(&mut child);
        let Ui {
            hits,
            tab,
            fields,
            regions,
            anchors,
            laid,
            sheets,
            secret,
            moving,
            caret_drawn,
            caret_foot,
            focus_box,
            focus_action,
            primaries,
            fab_at,
            tips,
            cut,
            back,
            ..
        } = child;
        let base = self.hits.len();
        self.hits.extend(hits.into_iter().map(|h| Hit {
            rect: h.rect,
            target: h.target,
            press: lift(h.press),
            layer: h.layer,
        }));
        self.tab.extend(tab.into_iter().map(|i| base + i));
        self.regions.extend(regions);
        self.fields.extend(fields.into_iter().map(|f| FieldAt {
            action: lift(f.action),
            first: f.first,
            edges: f.edges,
            band: f.band,
        }));
        self.anchors.extend(anchors);
        self.laid
            .extend(laid.into_iter().map(|(id, fact, p)| (id, fact, lift(p))));
        self.sheets.extend(sheets);
        self.secret |= secret;
        self.moving |= moving;
        self.caret_drawn |= caret_drawn;
        self.caret_foot = caret_foot.or(self.caret_foot);
        self.focus_box = focus_box.or(self.focus_box);
        self.focus_action = focus_action.map(lift).or_else(|| self.focus_action.take());
        self.primaries += primaries;
        self.fab_at = fab_at.or(self.fab_at);
        self.tips
            .extend(tips.into_iter().map(|t| crate::icon_button::TipAnchor {
                rect: t.rect,
                press: lift(t.press),
                text: t.text,
            }));
        self.cut.extend(cut);
        if self.back.is_none() {
            self.back = back.map(|(a, i)| (lift(a), base + i));
        }
    }

    /// A place a later Reveal or glide names. Stable across frames.
    pub fn anchor(&mut self, id: Anchor) {
        // The spec's `anchor` takes no rect: the place is the clip drawn in.
        let at = self.c.clip();
        self.anchors.push((id, at));
    }

    /// A layout fact an update reads.
    pub fn laid(&mut self, id: Anchor, fact: Laid, to: fn(Laid) -> A) {
        self.laid.push((id, fact, to(fact)));
    }

    /// The page this frame draws, set by the screen template (04 §4).
    pub fn set_page(&mut self, id: Anchor) {
        self.page = id;
    }

    /// A sheet drawn over the page, lowest first, set by the screen template.
    pub fn add_sheet(&mut self, id: Anchor) {
        self.sheets.push(id);
    }

    /// Draws what `f` draws of a text shown: a secret on show sets the frame's `secret` flag and
    /// marks the ink it leaves (04 §11).
    pub fn showing<T>(&mut self, text: crate::Shown<'_>, f: impl FnOnce(&mut Self) -> T) -> T {
        if text.is_secret() {
            self.secret = true;
            self.c.mark_secret(true);
        }
        let drawn = f(self);
        if text.is_secret() {
            self.c.mark_secret(false);
        }
        drawn
    }

    /// The caret and the selection a child page's field reads: only a field the keyboard is on
    /// uses them, and a child's press is told apart by the focus function, so they pass down
    /// as they are, with the field they belong to dropped.
    fn edit_for_child<B: Pressed>(&self) -> Edit<B> {
        Edit {
            field: None,
            caret: self.edit.caret,
            anchor: self.edit.anchor,
        }
    }

    /// Records the screen's back link: drawn, it is what the mouse's back button does. Only the
    /// page itself (layer 0) has one; the press is the last one taken.
    pub fn back(&mut self, action: A) {
        if self.layer == 0 {
            self.back = Some((action, self.hits.len().saturating_sub(1)));
        }
    }

    /// The top app bar's fill with the page `offset` units under it: surface, and
    /// surface-container once the page is a quarter of an edge fade's depth under it, mixed
    /// between the two on the way. The tint is quick, as Material's is, so that a page
    /// scrolled on past it moves rows and does not redraw the bar with each step.
    pub fn bar_tone(&self, offset: f32) -> Color {
        use faraday_tokens::dims::{APP_BAR_TINT_SHARE, EDGE_FADE, EDGE_FADE_COMPACT};
        let depth = if self.compact {
            EDGE_FADE_COMPACT
        } else {
            EDGE_FADE
        };
        self.col(Role::Surface).mix(
            self.col(Role::SurfaceContainer),
            offset / (depth * APP_BAR_TINT_SHARE),
        )
    }

    /// Whether the press given is the one the keyboard is on.
    pub fn is_focused(&self, press: &A) -> bool {
        (self.focus)(press)
    }
}

impl<A: Pressed> Ui<'_, A> {
    /// `c` in this frame's theme.
    pub fn col(&self, c: impl Into<Paint>) -> Color {
        let c: Paint = c.into();
        self.theme.color(c)
    }

    /// Fills the whole frame with `c`.
    pub fn clear(&mut self, c: impl Into<Paint>) {
        let c: Paint = c.into();
        let c = self.col(c);
        self.c.clear(c);
    }

    /// The clip as the screen sees it.
    pub fn seen_clip(&self) -> Rect {
        self.c.clip()
    }

    /// Design units to pixels.
    pub fn px(&self, v: f32) -> i32 {
        (v * self.f).round() as i32
    }

    /// The pixel row of design `y` (the frame's offset and the region's
    /// whole pixels applied).
    pub fn py(&self, y: f32) -> i32 {
        self.px(y + self.oy) + self.oy_px
    }

    /// A rectangle in design units, in pixels.
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        let (x, y) = (x + self.ox, y + self.oy);
        let x0 = self.px(x);
        let y0 = self.px(y);
        Rect::new(
            x0,
            y0 + self.oy_px,
            self.px(x + w) - x0,
            self.px(y + h) - y0,
        )
    }

    /// Whether anything drawn in the band from `y` to `y + h`, design
    /// units, can be seen: the band meets the clip, with the most a shadow
    /// reaches past its shape (`SHADOW_REACH`) to spare, so a row's shadow
    /// in view does not depend on the offset putting the row out of it. A row of a long list out
    /// of it is passed over, its height alone counted.
    pub fn shows(&self, y: f32, h: f32) -> bool {
        let band = self.rect(0.0, y, 1.0, h);
        let clip = self.seen_clip();
        let spare = self.px(faraday_tokens::dims::SHADOW_REACH).max(4);
        band.y - spare < clip.bottom() && band.bottom() + spare > clip.y
    }

    /// A filled circle; at High contrast a tint is its ring
    /// ([`Ui::fill`]).
    pub fn dot(&mut self, cx: f32, cy: f32, r: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        if let Some(edge) = self.tint_edge(color) {
            let rect = self.rect(cx - r, cy - r, 2.0 * r, 2.0 * r);
            let width = self.f.max(1.0);
            self.c.stroke_rounded_rect(rect, r * self.f, width, edge);
            return;
        }
        let color = self.col(self.solid(color));
        self.c.fill_circle(
            (cx + self.ox) * self.f,
            (cy + self.oy) * self.f + self.oy_px as f32,
            r * self.f,
            color,
        );
    }

    /// A filled rectangle with rounded corners. At High contrast
    /// a see-through theme colour is drawn at full strength, or, as a
    /// tint (under [`faraday_tokens::dims::HIGH_CONTRAST_TINT_MAX`]), as
    /// its edge: no see-through layer lowers the contrast of what is on
    /// it. A state layer, a control not available now or something
    /// fading is a [`Ui::wash`].
    pub fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        if let Some(edge) = self.tint_edge(color) {
            let rect = self.rect(x, y, w, h);
            let width = self.f.max(1.0);
            self.c.stroke_rounded_rect(rect, r * self.f, width, edge);
            return;
        }
        let color = self.solid(color);
        self.wash(x, y, w, h, r, color);
    }

    /// An outlined rectangle with rounded corners; at High contrast
    /// a see-through colour is drawn at full strength.
    pub fn stroke(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        let color = self.col(self.solid(color));
        let rect = self.rect(x, y, w, h);
        let width = self.f.max(1.0);
        self.c.stroke_rounded_rect(rect, r * self.f, width, color);
    }

    /// A card: a surface with a one-unit edge. The page is the surface
    /// too (`docs/DECISIONS.md` F39), so the edge is what sets a card
    /// apart; its fill is surface-container-low, Material's elevated
    /// card drawn flat (`docs/THEMES.md` §7).
    pub fn card(&mut self, x: f32, y: f32, w: f32, h: f32, edge: impl Into<Paint>) {
        use faraday_tokens::dims::SHAPE_M;
        let edge: Paint = edge.into();
        self.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
        self.stroke(x, y, w, h, SHAPE_M, edge);
    }

    /// At High contrast, the edge to draw in place of a fill of `c`
    /// when `c` is one of the scheme's roles made see-through as a tint:
    /// a tint under text or an icon would lower its contrast below 7:1,
    /// so what it marks is edged in its colour at full strength instead
    /// (`docs/THEMES.md` §9). `None` elsewhere.
    pub fn tint_edge(&self, c: Paint) -> Option<Color> {
        use faraday_tokens::dims::HIGH_CONTRAST_TINT_MAX;
        (self.theme.is_high_contrast() && c.alpha() < HIGH_CONTRAST_TINT_MAX && c.is_role())
            .then(|| self.col(c.with_alpha(255)))
    }

    /// `c` at full strength at High contrast where it is one of the
    /// scheme's roles made see-through: an edge, a rule or a mark keeps
    /// its 3:1 there. `c` as it is elsewhere.
    pub fn solid(&self, c: Paint) -> Paint {
        if self.theme.is_high_contrast() && c.alpha() < 255 && c.is_role() {
            c.with_alpha(255)
        } else {
            c
        }
    }

    /// A filled rectangle with rounded corners in `color` as it is asked
    /// for in every theme, see-through included: a state layer, a
    /// control not available now, something fading in or out.
    pub fn wash(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: impl Into<Paint>) {
        let color = self.col(color);
        let rect = self.rect(x, y, w, h);
        if r <= 0.0 {
            self.c.fill_rect(rect, color);
        } else {
            self.c.fill_rounded_rect(rect, r * self.f, color);
        }
    }

    pub fn font(&self, size: f32, w: W) -> Font {
        // Nothing is drawn under Material's smallest size.
        let size = match w {
            W::M => size.max(faraday_tokens::dims::TEXT_MIN / 0.94),
            _ => size.max(faraday_tokens::dims::TEXT_MIN),
        };
        let family = match w {
            W::R => Family::Regular,
            W::S => Family::SemiBold,
            W::M => Family::Mono,
        };
        // The text size applies here, once, so every measure, line, wrap
        // and draw built on it sees it (`docs/TEXT-SIZE.md` §3).
        let size = size * self.text;
        let size = if w == W::M {
            size / self.tr * 0.94
        } else {
            size / self.tr
        };
        Font::new(family, size)
    }

    /// Draws text `t` times its size from here on: the person's text size
    /// (`docs/TEXT-SIZE.md` §2), set once a frame before anything is
    /// drawn.
    pub fn set_text_size(&mut self, t: crate::TextSize) {
        self.text = t.factor();
    }

    /// What every text size and line height is multiplied by now: the
    /// person's text size, or 1 inside [`Ui::fixed_text`].
    pub fn text_factor(&self) -> f32 {
        self.text
    }

    /// `draw` with text at its Standard size, whatever the text size: what
    /// keeps its size as the text grows (a keyboard's keys, a pad). The
    /// factor is put back after.
    pub fn fixed_text<R>(&mut self, draw: impl FnOnce(&mut Self) -> R) -> R {
        let text = std::mem::replace(&mut self.text, 1.0);
        let r = draw(self);
        self.text = text;
        r
    }

    /// The face of `font` at the canvas's scale, looked up once a frame.
    pub fn face_of(&self, font: Font) -> faraday_draw::fonts::SizedFace {
        let scale = self.c.scale();
        let mut faces = self.faces.borrow_mut();
        if let Some(e) = faces.iter().find(|e| e.0 == font && e.1 == scale) {
            return e.2;
        }
        let face = font.sized(scale);
        faces.push((font, scale, face));
        face
    }

    /// The face for a size and weight.
    pub fn face(&self, size: f32, w: W) -> faraday_draw::fonts::SizedFace {
        self.face_of(self.font(size, w))
    }

    /// The width of `s` in design units.
    pub fn measure(&self, size: f32, w: W, s: &str) -> f32 {
        let face = self.face(size, w);
        text::width(&face, s) as f32 / self.f
    }

    /// The height of one line in design units.
    pub fn line(&self, size: f32, w: W) -> f32 {
        let face = self.face(size, w);
        face.line_height() as f32 / self.f
    }

    /// `c` as text and icons are drawn in this frame's theme: as
    /// [`Ui::col`], since the scheme's primary reads as text on every
    /// surface (`docs/THEMES.md` §6).
    pub fn ink(&self, c: impl Into<Paint>) -> Color {
        self.col(c)
    }

    /// `s` with its top-left at (x, y). Returns its width.
    pub fn text(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        w: W,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let color: Paint = color.into();
        let color = self.ink(color);
        let font = self.font(size, w);
        let px = self
            .c
            .text(self.px(x + self.ox), self.py(y), s, font, color);
        px as f32 / self.f
    }

    /// `s` centred vertically in a band of height `h` starting at y.
    #[allow(
        clippy::too_many_arguments,
        reason = "centred text's position, height, size, weight, colour and string, as the text helpers take them"
    )]
    pub fn text_mid(
        &mut self,
        x: f32,
        y: f32,
        h: f32,
        size: f32,
        w: W,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let color: Paint = color.into();
        let lh = self.line(size, w);
        self.text(x, y + (h - lh) / 2.0, size, w, color, s)
    }

    /// `s` ending at `right`, centred in a band of height `h`.
    #[allow(
        clippy::too_many_arguments,
        reason = "right-aligned text's edge, height, size, weight, colour and string, as the text helpers take them"
    )]
    pub fn text_right(
        &mut self,
        right: f32,
        y: f32,
        h: f32,
        size: f32,
        w: W,
        color: impl Into<Paint>,
        s: &str,
    ) {
        let color: Paint = color.into();
        let width = self.measure(size, w, s);
        self.text_mid(right - width, y, h, size, w, color, s);
    }

    /// How many lines `s` breaks onto at `max` width: the same
    /// break-at-a-space rule [`Ui::wrap`] draws, counted rather than
    /// drawn, so a caller can grow a row before drawing its background.
    pub fn wrap_lines(&self, size: f32, w: W, s: &str, max: f32) -> usize {
        let face = self.face(size, w);
        let space = face.advance_64(' ');
        let fits = |pen: u32| text::px_from_64(pen) as f32 / self.f <= max;
        let mut lines = 1usize;
        // The pen of the line so far, in 1/64 px: advances only add up, so
        // it is what measuring the whole line would sum.
        let mut cur: Option<u32> = None;
        // A no-break space joins what it stands between, as drawing does.
        for word in s
            .split(|c: char| c.is_whitespace() && c != '\u{a0}')
            .filter(|w| !w.is_empty())
        {
            let adv: u32 = word.chars().map(|c| face.advance_64(c)).sum();
            match cur {
                Some(pen) if fits(pen + space + adv) => cur = Some(pen + space + adv),
                Some(_) => {
                    lines += 1;
                    cur = Some(adv);
                }
                None => cur = Some(adv),
            }
        }
        lines
    }

    /// `s` cut with an ellipsis to fit `max` units: the longest start of
    /// it that fits with "…" after it, or nothing when "…" alone does
    /// not. One pass over its characters' advances, which only add up,
    /// so the first start that does not fit ends the search.
    pub fn fit(&self, size: f32, w: W, s: &str, max: f32) -> Text {
        let face = self.face(size, w);
        let fits = |pen: u32| text::px_from_64(pen) as f32 / self.f <= max;
        let whole: u32 = s.chars().map(|c| face.advance_64(c)).sum();
        if fits(whole) {
            return Text::of(s);
        }
        let ellipsis = face.advance_64('…');
        if !fits(ellipsis) {
            return Text::new();
        }
        let (mut pen, mut end) = (0u32, 0usize);
        for (i, c) in s.char_indices() {
            pen += face.advance_64(c);
            if !fits(pen + ellipsis) {
                break;
            }
            end = i + c.len_utf8();
        }
        let mut out = Text::with_room(end + '…'.len_utf8());
        out.push_str(&s[..end]);
        out.push('…');
        out
    }

    /// `s` cut in the middle to fit `max` units, so that its end (a file
    /// name's extension and what stands before it) is kept: the longest
    /// end that takes no more than half of what "…" leaves, "…" before
    /// it, and before that the longest start that still fits. Nothing
    /// when "…" alone does not fit. Two passes over its characters'
    /// advances, as [`Ui::fit`] counts them.
    pub fn fit_middle(&self, size: f32, w: W, s: &str, max: f32) -> Text {
        let face = self.face(size, w);
        let fits = |pen: u32| text::px_from_64(pen) as f32 / self.f <= max;
        let whole: u32 = s.chars().map(|c| face.advance_64(c)).sum();
        if fits(whole) {
            return Text::of(s);
        }
        let ellipsis = face.advance_64('…');
        if !fits(ellipsis) {
            return Text::new();
        }
        let (mut tail, mut from) = (0u32, s.len());
        for (i, c) in s.char_indices().rev() {
            let wider = tail + face.advance_64(c);
            if !fits(2 * wider + ellipsis) {
                break;
            }
            (tail, from) = (wider, i);
        }
        let (mut pen, mut end) = (0u32, 0usize);
        for (i, c) in s[..from].char_indices() {
            pen += face.advance_64(c);
            if !fits(pen + ellipsis + tail) {
                break;
            }
            end = i + c.len_utf8();
        }
        let mut out = Text::with_room(end + '…'.len_utf8() + s.len() - from);
        out.push_str(&s[..end]);
        out.push('…');
        out.push_str(&s[from..]);
        out
    }

    /// `s` wrapped to `width`. Returns the height used.
    #[allow(
        clippy::too_many_arguments,
        reason = "wrapped text's position, width, size, weight, colour and string, as the text helpers take them"
    )]
    pub fn wrap(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        size: f32,
        w: W,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let color: Paint = color.into();
        let color = self.ink(color);
        let font = self.font(size, w);
        let rect = Rect::new(
            self.px(x + self.ox),
            self.py(y),
            self.px(width),
            self.px(2000.0),
        );
        let px = self.c.text_wrapped(rect, s, font, color, TextAlign::Start);
        px as f32 / self.f
    }

    /// `s` wrapped to `width` with the bytes `mark` of it drawn in
    /// `mark_color` and heavier: the monospace face has one weight, so the
    /// marked run is drawn again [`faraday_tokens::dims::MARK_BOLD`] to the right, which keeps
    /// every character where it would be unmarked. Breaks as [`Ui::wrap`]
    /// does. Returns the height used.
    pub fn wrap_marked(
        &mut self,
        (x, y, width): (f32, f32, f32),
        size: f32,
        w: W,
        colors: (Paint, Paint),
        s: &str,
        mark: std::ops::Range<usize>,
    ) -> f32 {
        let (color, mark_color) = (self.ink(colors.0), self.ink(colors.1));
        let font = self.font(size, w);
        let face = self.face_of(font);
        let lines = text::wrap(&face, s, self.px(width));
        let lh = face.line_height();
        let bold = self.px(faraday_tokens::dims::MARK_BOLD).max(1);
        let (x0, mut y0) = (self.px(x + self.ox), self.py(y));
        for &(start, end) in &lines {
            let mut pen = x0;
            let cuts = [
                (start, mark.start.clamp(start, end), false),
                (
                    mark.start.clamp(start, end),
                    mark.end.clamp(start, end),
                    true,
                ),
                (mark.end.clamp(start, end), end, false),
            ];
            for (a, b, marked) in cuts {
                if a >= b {
                    continue;
                }
                let part = &s[a..b];
                if marked {
                    self.c.text(pen + bold, y0, part, font, mark_color);
                    pen += self.c.text(pen, y0, part, font, mark_color);
                } else {
                    pen += self.c.text(pen, y0, part, font, color);
                }
            }
            y0 += lh;
        }
        (lines.len() as i32 * lh) as f32 / self.f
    }

    /// An icon centred in a square of side `boxw` at (x, y).
    pub fn icon(
        &mut self,
        x: f32,
        y: f32,
        boxw: f32,
        icon: impl Into<Glyph>,
        size: f32,
        color: impl Into<Paint>,
    ) {
        let color: Paint = color.into();
        let color = self.ink(color);
        match icon.into() {
            Glyph::Icon(icon) => {
                let rect = self.rect(x, y, boxw, boxw);
                self.c.icon(rect, icon, size, color);
            }
            Glyph::Stick => self.stick_glyph(x + boxw / 2.0, y + boxw / 2.0, size, color),
        }
    }

    /// The chevron that ends a row which opens a page or its details
    /// (`docs/DESIGN.md` §4.13): `CHEVRON` wide, its right edge at `end`,
    /// centred in the height `h` from `y`. The one place a row's chevron is
    /// drawn.
    pub fn row_chevron(&mut self, end: f32, y: f32, h: f32, color: impl Into<Paint>) {
        use faraday_tokens::dims::CHEVRON;
        self.icon(
            end - CHEVRON,
            y + (h - CHEVRON) / 2.0,
            CHEVRON,
            Icon::ChevronRight,
            CHEVRON,
            color,
        );
    }

    /// The USB stick glyph centred on (cx, cy), `size` units across: a
    /// body with rounded corners and its connector at the right end, two
    /// holes in the connector. Drawn as outlines, at the icon face's size.
    fn stick_glyph(&mut self, cx: f32, cy: f32, size: f32, color: impl Into<Paint>) {
        let color = self.col(color);
        let f = self.f;
        let (ox, oy, oy_px) = (self.ox, self.oy, self.oy_px);
        // In units of the glyph's size, from its centre.
        let px = |u: f32, v: f32| {
            (
                (cx + u * size + ox) * f,
                (cy + v * size + oy) * f + oy_px as f32,
            )
        };
        // The body, with its corners cut round in three steps each.
        let (l, r, t, b, k) = (-0.5, 0.18, -0.27, 0.27, 0.1);
        let mut body = Vec::with_capacity(16);
        let corners = [
            (r - k, t + k, -std::f32::consts::FRAC_PI_2),
            (r - k, b - k, 0.0),
            (l + k, b - k, std::f32::consts::FRAC_PI_2),
            (l + k, t + k, std::f32::consts::PI),
        ];
        for (ccx, ccy, a0) in corners {
            for j in 0..4 {
                let a = a0 + j as f32 * std::f32::consts::FRAC_PI_6;
                body.push(px(ccx + k * a.cos(), ccy + k * a.sin()));
            }
        }
        // The connector, overlapping the body's end, and its two holes
        // wound the other way round.
        let (cl, cr, ct, cb) = (0.12, 0.5, -0.16, 0.16);
        let plug = [px(cl, ct), px(cr, ct), px(cr, cb), px(cl, cb)];
        let hole = |hl: f32, hr: f32| [px(hl, -0.06), px(hl, 0.06), px(hr, 0.06), px(hr, -0.06)];
        let (h1, h2) = (hole(0.24, 0.33), hole(0.38, 0.45));
        // The holes are cut from the connector alone, where it is clear
        // of the body.
        self.c.fill_contours(&[&body, &plug, &h1, &h2], color);
    }
}

/// A frame to draw in and read back, for the tests of this crate and of the kit's levels above it.
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::*;
    use crate::hold::HoldTone;
    use crate::tag::Status;
    use faraday_port::values::{Display, Orientation};

    /// A press for the tests: a number.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub struct P(pub u8);

    impl Pressed for P {
        fn hides(&self) -> bool {
            false
        }
    }

    /// A canvas of 640 × 480 pixels at `dpi`.
    pub fn canvas(dpi: u16) -> Canvas {
        Canvas::new(&Display {
            width: 640,
            height: 480,
            dpi,
            orientation: Orientation::Portrait,
            rotation_offered: None,
            external: false,
            preview: None,
        })
    }

    /// A pointer resting nowhere, and the keyboard on `focused`.
    pub fn at_rest(focused: Option<P>) -> Pointer<P> {
        Pointer {
            at: None,
            down: false,
            focused,
        }
    }

    /// The frame `draw` makes on `canvas` at a pixel a unit (160 dpi), with the pointer as given.
    pub fn frame(
        canvas: &mut Canvas,
        pointer: &Pointer<P>,
        draw: impl FnOnce(&mut Ui<'_, P>),
    ) -> FrameOut<P> {
        let mut motion = MotionStore::new();
        let mut ui = Ui::new(canvas, Theme::default(), 1.0, &mut motion, pointer, true);
        draw(&mut ui);
        ui.finish()
    }

    // The colours below are the roles the atoms themselves draw with (a tag's, a hold's), read
    // through their own tables, so no test names the tokens crate (07 TS11).

    /// A page fill: the neutral tag's container, any one the tints below lie over.
    pub fn page() -> Paint {
        Status::Neutral.roles().0.paint()
    }

    /// A see-through ok tint: the ok tag's container, see-through.
    pub fn ok_tint() -> Paint {
        Status::Ok.roles().0.with_alpha(30)
    }

    /// A see-through primary edge: the primary hold's edge role, see-through.
    pub fn primary_edge() -> Paint {
        HoldTone::Primary.roles().0.with_alpha(110)
    }

    /// The colour a refused hold's edge and a field's error line are drawn in, in `theme`.
    pub fn error_in(theme: Theme) -> Color {
        theme.role(HoldTone::Error.roles().0)
    }

    /// The pixel at (x, y), red, green and blue.
    pub fn pixel(c: &Canvas, x: usize, y: usize) -> (u8, u8, u8) {
        let f = c.pixels();
        let i = (y * usize::from(f.width) + x) * 4;
        (f.rgba[i], f.rgba[i + 1], f.rgba[i + 2])
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    /// WCAG relative luminance of a colour.
    fn luminance((r, g, b): (u8, u8, u8)) -> f32 {
        let lin = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    }

    /// WCAG contrast ratio of two colours.
    fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// A canvas drawn on at `dpi` and `theme`, and the pixels of a tint over a clear page: (inside
    /// the chip, on its edge, the page, on the translucent edge).
    fn tinted(theme: Theme) -> [(u8, u8, u8); 4] {
        let mut canvas = canvas(160);
        let pointer = at_rest(None);
        let mut motion = MotionStore::new();
        let bare;
        {
            let mut ui = Ui::new(&mut canvas, theme, 1.0, &mut motion, &pointer, false);
            let c: Color = ui.col(page());
            bare = (c.r, c.g, c.b);
            ui.clear(page());
            ui.fill(100.0, 100.0, 120.0, 40.0, 0.0, ok_tint());
            ui.stroke(100.0, 200.0, 120.0, 40.0, 0.0, primary_edge());
        }
        [
            pixel(&canvas, 160, 120),
            pixel(&canvas, 160, 100),
            bare,
            pixel(&canvas, 160, 200),
        ]
    }

    /// Invariant: At High contrast a tint is drawn as its edge with the page left under what it holds, and elsewhere a tint is a tint.
    /// Regression: A see-through fill lowers the contrast of the text drawn on it at High contrast.
    /// Owner: kit
    #[test]
    fn high_contrast_draws_a_tint_as_its_edge_and_edges_at_full_strength() {
        for theme in crate::Theme::all().filter(|t| t.is_high_contrast()) {
            let [inside, edge, page, line] = tinted(theme);
            assert_eq!(inside, page, "{theme:?}");
            assert!(contrast(edge, page) >= 3.0, "{theme:?} tint's edge");
            assert!(contrast(line, page) >= 3.0, "{theme:?} edge");
        }
        let [inside, _, page, _] = tinted(crate::Theme::default());
        assert_ne!(inside, page);
    }

    /// The line count by building each candidate line and measuring it whole: what `wrap_lines`
    /// did before it summed advances.
    fn wrap_lines_whole(ui: &Ui<P>, size: f32, w: W, s: &str, max: f32) -> usize {
        let mut lines = 1usize;
        let mut cur = String::new();
        for word in s.split_whitespace() {
            let candidate = if cur.is_empty() {
                word.to_string()
            } else {
                format!("{cur} {word}")
            };
            if cur.is_empty() || ui.measure(size, w, &candidate) <= max {
                cur = candidate;
            } else {
                lines += 1;
                cur = word.to_string();
            }
        }
        lines
    }

    /// Invariant: A text breaks onto as many lines as measuring each whole line gives, in every face and size.
    /// Regression: Counting a line by summed advances drifts from the drawn line for wide, narrow or combining characters.
    /// Owner: kit
    #[test]
    fn wrap_lines_counts_what_measuring_each_whole_line_counts() {
        let texts = [
            "",
            "   ",
            "one",
            "The quick brown fox jumps over the lazy dog and keeps running",
            "  leading   and\ttrailing \n spaces  ",
            "a supercalifragilisticexpialidocious_word_wider_than_any_line fits not",
            "Zażółć gęślą jaźń — naïve café façade, 日本語のテキスト と 한국어",
            "e\u{301} combining a\u{308}\u{323} marks n\u{303} on letters",
            "iiiiii WWWWWW iiiiii WWWWWW mmm lll . , ; : ! ? 0123456789",
        ];
        for dpi in [120, 160, 240] {
            let mut c = canvas(dpi);
            let pointer = at_rest(None);
            let mut motion = MotionStore::new();
            let ui = Ui::new(
                &mut c,
                crate::Theme::default(),
                1.0,
                &mut motion,
                &pointer,
                false,
            );
            for t in texts {
                for w in [W::R, W::S, W::M] {
                    for size in [11.0, 14.0, 20.0] {
                        for max in [1.0, 20.0, 57.5, 90.0, 140.0, 231.0, 480.0, 5000.0] {
                            assert_eq!(
                                ui.wrap_lines(size, w, t, max),
                                wrap_lines_whole(&ui, size, w, t, max),
                                "{t:?} at {size} {max} {dpi}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Invariant: A name cut in the middle keeps its start and its end, its extension, within the room given.
    /// Regression: Cutting a long file name drops its extension or leaves it wider than its room.
    /// Owner: kit
    #[test]
    fn a_name_cut_in_the_middle_keeps_its_start_and_its_end() {
        let mut canvas = canvas(160);
        let pointer = at_rest(None);
        let mut motion = MotionStore::new();
        let ui = Ui::new(
            &mut canvas,
            crate::Theme::default(),
            1.0,
            &mut motion,
            &pointer,
            false,
        );
        let name = "backup-0123456789abcdef0123456789abcdef-code.png";
        let whole = ui.measure(14.0, W::R, name);
        assert_eq!(ui.fit_middle(14.0, W::R, name, whole).as_str(), name);
        let cut = ui.fit_middle(14.0, W::R, name, whole / 2.0);
        let (start, end) = cut.split_once('…').unwrap();
        assert!(!start.is_empty() && name.starts_with(start), "{}", &*cut);
        assert!(end.ends_with(".png") && name.ends_with(end), "{}", &*cut);
        assert!(ui.measure(14.0, W::R, &cut) <= whole / 2.0, "{}", &*cut);
        assert_eq!(ui.fit_middle(14.0, W::R, name, 0.0).as_str(), "");
    }

    /// `fit` as it was until 2026-10-21: one character off the end at a time, the rest measured
    /// again with "…" after it each time.
    fn fit_by_popping(ui: &Ui<P>, size: f32, w: W, s: &str, max: f32) -> String {
        if ui.measure(size, w, s) <= max {
            return s.to_string();
        }
        let mut chars: Vec<char> = s.chars().collect();
        while !chars.is_empty() {
            chars.pop();
            let t: String = chars.iter().collect::<String>() + "…";
            if ui.measure(size, w, &t) <= max {
                return t;
            }
        }
        String::new()
    }

    /// Invariant: A name too long for its room is cut where it always was: the longest start that fits with an ellipsis after it.
    /// Regression: The one-pass cut of a name lands a character off the cut that measured each start whole.
    /// Owner: kit
    #[test]
    fn a_name_too_long_for_its_room_is_cut_where_it_always_was() {
        let long = "savings-2-of-3-multisig-wallet-backup-".repeat(6);
        let names = [
            "",
            "a",
            "…",
            "savings-unsigned.psbt",
            "Wallet · 2-of-3 · 8f3a91c2 · wsh(sortedmulti)",
            "Grüße – ñandú · 漢字のテスト · Ωmega",
            "WWWWWWWWWWWWWWWWWWWWiiiiiiiiiiiiiiiiiiii",
            long.as_str(),
        ];
        for dpi in [160, 286, 320] {
            let mut canvas = canvas(dpi);
            let pointer = at_rest(None);
            let mut motion = MotionStore::new();
            let f = f32::from(dpi) / 160.0;
            let ui = Ui::new(
                &mut canvas,
                crate::Theme::default(),
                f,
                &mut motion,
                &pointer,
                false,
            );
            for name in names {
                for (size, w) in [(12.0, W::R), (14.0, W::M), (13.0, W::S)] {
                    for max in [0.0, 3.0, 9.5, 20.0, 47.25, 80.0, 133.0, 260.0, 1000.0] {
                        assert_eq!(
                            ui.fit(size, w, name, max).as_str(),
                            fit_by_popping(&ui, size, w, name, max),
                            "{name:?} at {size} {w:?} in {max} at {dpi} dpi"
                        );
                    }
                }
            }
        }
    }

    /// A child page's press: its own type.
    #[derive(Clone, Copy, PartialEq, Debug)]
    struct Child(u8);

    impl Pressed for Child {
        fn hides(&self) -> bool {
            false
        }
    }

    /// Invariant: A press a child page records reaches the frame as the root's press, and the keyboard's focus on the root's press rings the child's control.
    /// Regression: A child page's hits are left in the child's type, or lose the focus ring once lifted.
    /// Owner: kit
    #[test]
    fn a_child_pages_press_reaches_the_frame_lifted_and_keeps_its_focus_ring() {
        let draw = |canvas: &mut Canvas, focused: Option<P>| {
            frame(canvas, &at_rest(focused), |ui| {
                ui.within(
                    |c: Child| P(c.0 + 10),
                    |ui| {
                        ui.button(
                            100.0,
                            100.0,
                            Some(80.0),
                            40.0,
                            "Go",
                            crate::Look::Secondary,
                            Child(1),
                        );
                    },
                );
            })
        };
        let mut plain = canvas(160);
        let out = draw(&mut plain, None);
        assert_eq!(
            out.hits.iter().map(|h| h.press).collect::<Vec<_>>(),
            [P(11)]
        );
        assert_eq!(out.reach, [0]);
        let mut ringed = canvas(160);
        draw(&mut ringed, Some(P(11)));
        assert_ne!(pixel(&ringed, 96, 120), pixel(&plain, 96, 120));
    }

    /// Invariant: A frame that shows a secret says so, and a frame that shows none does not.
    /// Regression: A secret on show is drawn without the frame's secret flag, so the lock check misses it.
    /// Owner: kit
    #[test]
    fn a_frame_that_draws_a_secret_says_so() {
        use crate::offered::Offered;
        use crate::shown::Shown;
        let model = |label| crate::button::ButtonModel {
            label,
            style: crate::Style::Filled,
            offered: Offered::Yes(P(1)),
            chosen: false,
            icon: None,
        };
        let drawn = |label| {
            let mut c = canvas(160);
            frame(&mut c, &at_rest(None), |ui| {
                crate::button::draw(&model(label), ui, (10.0, 10.0, None, 40.0));
            })
        };
        assert!(drawn(Shown::secret("abandon")).secret);
        assert!(!drawn(Shown::plain("Go")).secret);
    }
}
