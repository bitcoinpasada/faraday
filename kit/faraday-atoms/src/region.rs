//! Scrolled regions (04 §7.1, §8.1): what a wheel or a finger moves. A region is keyed by an
//! `Anchor` the page gives it (K10); its offset and stretch are the motion store's, which the
//! runtime moves, so a region sends no message and keeps no state of its own.

use faraday_draw::geom::{Rect, Size};
use faraday_port::values::Anchor;
use faraday_tokens::palette::{Color, Role};

use crate::Pressed;
use crate::motion::{ComponentId, Motion};
use crate::ui::Ui;

/// What a wheel or a finger moves.
pub struct Scrolled {
    /// The region's anchor: its entry in the motion store.
    pub id: Anchor,
    /// The part of the window it shows, in pixels.
    pub view: Rect,
    /// The size of all it holds, in pixels.
    pub extent: Size,
    /// The layer it was drawn on: 0 for the page, then the sheets (04 §7.1). Only the top
    /// layer's regions take a wheel or a finger.
    pub layer: u8,
}

/// The overlay scrollbar's thumb, units wide.
const BAR_W: f32 = 6.0;

/// How far in from a scrolled region's right edge a press takes the
/// scrollbar rather than the content, in units.
pub const BAR_GRAB: f32 = 16.0;

/// Where the overlay scrollbar of a region seen through `view` (pixels)
/// runs: the track's top and length and the thumb's length, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarGeometry {
    /// The track's top.
    pub top: i32,
    /// The track's length.
    pub track: i32,
    /// The thumb's length.
    pub thumb: i32,
}

impl BarGeometry {
    /// The bar of a region whose content scrolls `max` units, at `f`
    /// pixels a unit.
    pub fn of(view: Rect, max: f32, f: f32) -> BarGeometry {
        let inset = (4.0 * f).round() as i32;
        let track = view.h - 2 * inset;
        let vh = view.h as f32;
        let thumb = ((track as f32 * vh / (vh + max * f)) as i32)
            .max((30.0 * f) as i32)
            .min(track);
        BarGeometry {
            top: view.y + inset,
            track,
            thumb,
        }
    }

    /// Where the thumb starts along the track, the region scrolled
    /// `offset` of `max` units.
    pub fn at(&self, offset: f32, max: f32) -> i32 {
        ((self.track - self.thumb) as f32 * (offset / max).clamp(0.0, 1.0)).round() as i32
    }
}

/// An overlay scrollbar's thickness, in pixels at `f` to the unit.
fn bar_thickness(f: f32) -> i32 {
    (BAR_W * f).round().max(3.0) as i32
}

/// A region about to be drawn: where it is seen, what it sits on and which anchor keys its
/// motion ([`Ui::region`]).
#[derive(Debug, Clone, Copy)]
pub struct Region {
    id: Anchor,
    view: Rect,
    ground: Role,
    fade: Option<Role>,
    own_bar: bool,
    shown_bar: bool,
    pan: f32,
    moves: bool,
}

impl Region {
    /// The region `id` seen through `view`, on the page, moved by its entry's offset. Its
    /// content is laid out from zero and moved by the whole pixels of the offset after each
    /// place is rounded ([`Ui::oy_px`]), so a row's place is `round(y * f) - offset_px`, never
    /// `round((y - offset) * f)`.
    pub fn new(id: Anchor, view: Rect) -> Self {
        Region {
            id,
            view,
            ground: Role::Surface,
            fade: None,
            own_bar: false,
            shown_bar: false,
            pan: 0.0,
            moves: false,
        }
    }

    /// On the container `ground` rather than on the page (a dialog, a sheet, a menu): its edges
    /// fade into that container, and that container shows in the gap a stretch opens.
    pub fn on(self, ground: Role) -> Self {
        Region { ground, ..self }
    }

    /// Its edges fade into `fill` rather than into what it sits on.
    pub fn fades_into(self, fill: Role) -> Self {
        Region {
            fade: Some(fill),
            ..self
        }
    }

    /// The region draws its own scrollbar.
    pub fn own_bar(self) -> Self {
        Region {
            own_bar: true,
            ..self
        }
    }

    /// The overlay scrollbar is always shown, at full strength, rather than fading in while the
    /// region scrolls and out after it (the guide).
    pub fn bar_shown(self) -> Self {
        Region {
            shown_bar: true,
            ..self
        }
    }

    /// The region may be drawn as a moved frame (04 §7.1): when the runtime lends the rows it
    /// kept of it and only its offset changed, they are put back moved and only the strip the
    /// move uncovered is painted. Its content must depend on nothing but its state and offset:
    /// no caret, no animation, no hover of its own.
    pub fn moves(self) -> Self {
        Region {
            moves: true,
            ..self
        }
    }

    /// The region's content also pans sideways, `x` units now.
    pub fn panned(self, x: f32) -> Self {
        Region { pan: x, ..self }
    }
}

/// How far a region's content goes past its view, units: down, and across where it pans.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Extent {
    /// How far it scrolls down.
    pub down: f32,
    /// How far it pans across.
    pub across: f32,
}

impl From<f32> for Extent {
    fn from(down: f32) -> Self {
        Extent { down, across: 0.0 }
    }
}

/// A region opened with [`Ui::region_open`], closed with [`Ui::region_close`].
#[derive(Debug)]
#[must_use = "a region opened is closed with Ui::region_close"]
pub struct Open {
    region: Region,
    oy_px: i32,
    offset: f32,
    /// The rows the offset moved the content by, and where the rows put back are lent, on a
    /// moved frame.
    moved: Option<(i32, usize)>,
    /// What [`Ui::seen_clip`] read before the region.
    seen: Option<Rect>,
}

impl<A: Pressed> Ui<'_, A> {
    /// The offset (units) and stretch (pixels) the motion store holds for the region `id` of the
    /// page drawn, and whether it moves on its own (a coast or a spring): none while no entry is
    /// there.
    fn region_motion(&self, id: Anchor) -> (f32, i32, bool) {
        match self.motion.get(&ComponentId(self.page, id)) {
            Some(Motion::Region {
                offset,
                stretch,
                velocity,
                ..
            }) => (
                *offset,
                (stretch * self.f).round() as i32,
                *stretch != 0.0 || *velocity != 0.0,
            ),
            _ => (0.0, 0, false),
        }
    }

    /// How far the region `id` of the page drawn is scrolled, units: what a bar over it reads to
    /// tint itself by. None while no entry is there.
    pub fn scroll_of(&self, id: Anchor) -> f32 {
        self.region_motion(id).0
    }

    /// Draws a scrolled region: the content `content` draws is clipped to the region's view, and
    /// the region is reported once it is drawn. The content returns how far it goes past the
    /// view, units.
    pub fn region<R: Into<Extent>>(
        &mut self,
        region: Region,
        content: impl FnOnce(&mut Self) -> R,
    ) {
        let open = self.region_open(region);
        let reach = content(self).into();
        self.region_close(open, reach);
    }

    /// The first half of [`Ui::region`], for a page whose content does not fit one closure:
    /// pushes the view's clip and the pixels the offset moves the content by.
    pub fn region_open(&mut self, region: Region) -> Open {
        let oy_px = self.oy_px;
        let seen = self.seen;
        let (offset, _, _) = self.region_motion(region.id);
        let px = self.px(offset);
        self.oy_px = oy_px - px;
        let mut clip = region.view;
        let mut moved = None;
        if let Some((d, at)) = self.moves_by(&region, px) {
            // The whole view is seen for hits and for what a row skips; only the strip the move
            // uncovered is painted, with the rows beside it a shadow reaches into.
            self.seen = Some(self.c.clip().intersect(&region.view));
            let view = region.view;
            let m = self.strip_margin(&view, d);
            clip = match d {
                0 => Rect::new(view.x, view.y, view.w, 0),
                d if d > 0 => Rect::new(view.x, view.bottom() - d - m, view.w, d + m),
                d => Rect::new(view.x, view.y, view.w, -d + m),
            };
            moved = Some((d, at));
        }
        self.c.push_clip(clip);
        Open {
            region,
            oy_px,
            offset,
            moved,
            seen,
        }
    }

    /// The rows beside a moved frame's strip that are painted again, `d` rows moved in `view`:
    /// the most a shadow reaches past its shape, as far as the view has rows left.
    fn strip_margin(&self, view: &Rect, d: i32) -> i32 {
        if d == 0 {
            return 0;
        }
        self.px(faraday_tokens::dims::SHADOW_REACH)
            .min(view.h - d.abs())
            .max(0)
    }

    /// How many rows this frame's region moves the rows the runtime lent by, and which of them
    /// they are: the region opted in ([`Region::moves`]), is on the page, and was drawn in the
    /// same view; the view is whole in the canvas and in the clip; and the move leaves rows of
    /// the lent ones in view.
    fn moves_by(&self, region: &Region, px: i32) -> Option<(i32, usize)> {
        let view = region.view;
        if !region.moves
            || self.layer != 0
            || self.c.bounds().intersect(&view) != view
            || self.c.clip().intersect(&view) != view
        {
            return None;
        }
        let at = self.keeps.rows.iter().position(|k| {
            k.page == self.page
                && k.region == region.id
                && k.view == view
                && k.rows.len() == (view.w * view.h * 4) as usize
        })?;
        let d = px - self.keeps.rows[at].px;
        (d.abs() < view.h - 1).then_some((d, at))
    }

    /// The second half of [`Ui::region`]: pops the clip, stretches the region past an end, fades
    /// its edges and draws its scrollbar, and reports it.
    pub fn region_close(&mut self, open: Open, reach: impl Into<Extent>) {
        let Open {
            region,
            oy_px,
            offset,
            moved,
            seen,
        } = open;
        let reach = reach.into();
        self.oy_px = oy_px;
        self.seen = seen;
        self.c.pop_clip();
        let (view, ground) = (region.view, region.ground);
        self.region_rows(&region, offset, moved);
        let max = reach.down.max(0.0);
        let across = reach.across.max(0.0);
        self.regions.push(Scrolled {
            id: region.id,
            view,
            extent: Size {
                w: view.w + (across * self.f).round() as i32,
                h: view.h + (max * self.f).round() as i32,
            },
            layer: self.layer,
        });
        let (_, stretch, going) = self.region_motion(region.id);
        self.stretch = stretch;
        if going {
            self.moving = true;
        }
        self.stretch_view(view, ground);
        let into = region.fade.unwrap_or(ground);
        let fade_top = self.layer > 0 || view.y > 0;
        self.edge_fades(view, (max, offset), into, fade_top);
        // The pinned bar is at full strength; the overlay one stays after the content moves and
        // fades out (04 §8.1), and is not drawn while no entry says it moved.
        let alpha = if region.shown_bar {
            255
        } else {
            self.bar_alpha_of(region.id)
        };
        if !region.own_bar && max > 0.0 && alpha > 0 {
            self.scroll_bar(view, max, offset, alpha);
        }
        if region.shown_bar && across > 0.0 {
            self.scroll_bar_across(view, across, region.pan, 255);
        }
        self.stretch = 0;
    }
}

impl<A: Pressed> Ui<'_, A> {
    /// A region that moves: the rows lent are put back moved but for the strip painted, and the
    /// rows are taken again, before the stretch, the fades and the bar are drawn over them. None
    /// are taken once the frame has shown a secret (V19).
    fn region_rows(&mut self, region: &Region, offset: f32, moved: Option<(i32, usize)>) {
        let view = region.view;
        if let Some((d, at)) = moved {
            let rows = self.keeps.rows[at].rows;
            let m = self.strip_margin(&view, d);
            self.c.paste_rows(rows, view, -d, m);
            self.keeps.moved += 1;
        }
        if region.moves
            && self.layer == 0
            && !self.secret
            && self.c.bounds().intersect(&view) == view
        {
            let mut rows = Vec::new();
            self.c.copy_region(view, &mut rows);
            self.keeps.made_rows.push(crate::keeps::MadeRows {
                page: self.page,
                region: region.id,
                view,
                px: self.px(offset),
                rows,
            });
        }
    }

    /// The overlay scrollbar along a region's foot: a thumb as long as the
    /// share of the content's width in view, clear of the bar at its right
    /// edge, drawn as [`Ui::scroll_bar`] draws its own.
    fn scroll_bar_across(&mut self, view: Rect, max: f32, offset: f32, alpha: u8) {
        let h = bar_thickness(self.f);
        // The vertical bar's geometry along the width, its end kept clear
        // of the bar at the right edge.
        let along = Rect::new(view.y, view.x, view.h, view.w - h);
        let g = BarGeometry::of(along, max, self.f);
        if g.track <= 0 || g.thumb <= 0 {
            return;
        }
        let inset = g.top - view.x;
        let x = g.top + g.at(offset, max);
        let rect = Rect::new(x, view.bottom() - inset - h, g.thumb, h);
        let color = self.bar_color(alpha);
        self.c.fill_rounded_rect(rect, h as f32 / 2.0, color);
    }

    /// The overlay scrollbar's colour at `alpha`: faint, but at full
    /// strength at High contrast, where it is the one sign the page goes
    /// on.
    fn bar_color(&self, alpha: u8) -> Color {
        let most = if self.theme.is_high_contrast() {
            255
        } else {
            150
        };
        self.col(Role::OnSurfaceVariant.with_alpha((u16::from(alpha) * most / 255) as u8))
    }

    /// The overlay scrollbar: a thumb at the region's right edge, as long
    /// as the share of the content in view. It can be held and dragged
    /// ([`BarGeometry`]).
    fn scroll_bar(&mut self, view: Rect, max: f32, offset: f32, alpha: u8) {
        let f = self.f;
        let BarGeometry {
            top: track_top,
            track,
            thumb,
        } = BarGeometry::of(view, max, f);
        let inset = track_top - view.y;
        if track <= 0 || thumb <= 0 {
            return;
        }
        let at = BarGeometry::of(view, max, f).at(offset, max);
        // Shortened by what the stretch pulls past the end, as a native
        // bar is.
        let squeeze = self.stretch.abs().min(thumb / 2);
        let (y, h) = if self.stretch < 0 {
            (view.y + inset, thumb - squeeze)
        } else {
            (
                view.y + inset + at + squeeze.min(track - thumb - at).max(0),
                thumb - squeeze,
            )
        };
        let w = bar_thickness(f);
        let rect = Rect::new(view.right() - inset - w, y, w, h);
        let color = self.bar_color(alpha);
        self.c.fill_rounded_rect(rect, w as f32 / 2.0, color);
    }
    /// The region's content fades into `ground`, the surface it sits on,
    /// at an edge it continues past: under the top once scrolled, above
    /// the bottom while there is more. Each fade comes in over the first
    /// units scrolled. The top one is drawn only when `fade_top`, into
    /// `top_into` when given (the bar's fill over the page). At High contrast a scheme draws none: a fade
    /// lowers the contrast of what is under it, and the scrollbar says
    /// the page goes on.
    fn edge_fades(&mut self, view: Rect, (max, offset): (f32, f32), ground: Role, fade_top: bool) {
        use faraday_tokens::dims::{
            EDGE_FADE, EDGE_FADE_ALPHA, EDGE_FADE_COMPACT, EDGE_FADE_LOW_COMPACT,
        };
        if max <= 0.0 || view.h <= 0 || self.theme.is_high_contrast() {
            return;
        }
        // Deeper on a small panel, where it is the sign there is more.
        let depth = if self.compact {
            EDGE_FADE_COMPACT
        } else {
            EDGE_FADE
        };
        let band = ((depth * self.f).round() as i32).clamp(1, view.h / 4 + 1);
        // The bottom fade is shallower, so a control resting at the fold
        // (a text button last on a page) still reads at its own emphasis.
        let low = ((if self.compact {
            EDGE_FADE_LOW_COMPACT
        } else {
            EDGE_FADE
        } * self.f)
            .round() as i32)
            .clamp(1, view.h / 4 + 1);
        let page = self.col(ground);
        let reach = band as f32 / self.f;
        let top = if fade_top {
            (offset / reach).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let above = page;
        let bottom = ((max - offset) / (low as f32 / self.f)).clamp(0.0, 1.0);
        for i in 0..band {
            let k = 1.0 - (i as f32 + 0.5) / band as f32;
            let a = k * k * EDGE_FADE_ALPHA;
            if top > 0.0 {
                let row = Rect::new(view.x, view.y + i, view.w, 1);
                self.c.fill_rect(row, above.with_alpha((a * top) as u8));
            }
        }
        for i in 0..low {
            let k = 1.0 - (i as f32 + 0.5) / low as f32;
            let a = k * k * EDGE_FADE_ALPHA;
            if bottom > 0.0 {
                let row = Rect::new(view.x, view.bottom() - 1 - i, view.w, 1);
                self.c.fill_rect(row, page.with_alpha((a * bottom) as u8));
            }
        }
    }

    /// Moves what was drawn in `view` by the stretch, and what is
    /// pressable there with it; `ground`, the surface the region sits on,
    /// shows in the gap.
    fn stretch_view(&mut self, view: Rect, ground: Role) {
        let n = self.stretch.abs().min(view.h);
        if n == 0 {
            return;
        }
        self.c.shift(view, -self.stretch);
        let gap = if self.stretch < 0 {
            Rect::new(view.x, view.y, view.w, n)
        } else {
            Rect::new(view.x, view.bottom() - n, view.w, n)
        };
        let page = self.col(ground);
        self.c.fill_rect(gap, page);
        let by = self.stretch;
        for hit in self.hits.iter_mut() {
            if hit.rect.intersect(&view) == hit.rect {
                let (d, g) = (hit.rect, hit.target);
                hit.rect = Rect::new(d.x, d.y - by, d.w, d.h).intersect(&view);
                hit.target = Rect::new(g.x, g.y - by, g.w, g.h).intersect(&view);
            }
        }
    }
}
