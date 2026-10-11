//! Flexbox-lite layout (`docs/PLANNING.md` §4.3).
//!
//! An application builds a [`Node`] tree each frame: `Column`, `Row`,
//! `Stack` and vertical `Scroll` containers with leaf [`Widget`]s. Every
//! node may carry a flex `weight`, `min`/`max`/`fixed` sizes, `padding`;
//! containers add `gap`, cross-axis `align` and main-axis `justify`. All of
//! those are in dp. [`solve`] converts to pixels for one display and
//! produces a [`Layout`]: a pixel [`Rect`] and clip per node, in drawing
//! order, plus the hit target of every interactive widget.
//!
//! The algorithm is two-pass. `measure` asks each node for its intrinsic
//! size under a maximum (text wraps to the width it is given, buttons
//! report their minimum size); `place` then distributes leftover main-axis
//! space by weight, shrinks weighted children when the sum overflows, and
//! aligns on the cross axis. A node that shrinks but never grows says so
//! with `shrink` instead of `weight`. There is no line wrapping of
//! children and no shrinking of nodes that carry neither: a screen that
//! does not fit scrolls or clips, it never silently reflows.

use alloc::vec::Vec;

use crate::geom::{Dp, Edges, PxEdges, Rect, Scale, Size, SizeClass};
use crate::widgets::{HitTarget, Widget};

/// An application-chosen identifier for an interactive widget or a node
/// whose rectangle the application wants back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub u32);

/// Cross-axis alignment of children (and of a `Stack`'s children on both
/// axes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Top / left.
    Start,
    /// Centred.
    Center,
    /// Bottom / right.
    End,
    /// Fill the cross axis (default).
    #[default]
    Stretch,
}

/// Main-axis distribution of leftover space when no child has a weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    /// Pack at the start (default).
    #[default]
    Start,
    /// Centre the run of children.
    Center,
    /// Pack at the end.
    End,
    /// Equal space between children.
    SpaceBetween,
}

/// What a node is.
///
/// Deliberately not `Debug`: widgets carry display text, which may be a
/// masked or briefly revealed secret.
pub enum Kind {
    /// Children top to bottom.
    Column,
    /// Children left to right.
    Row,
    /// Children on top of one another, each given the whole area (or
    /// aligned inside it).
    Stack,
    /// One child laid out at its natural height inside a clipped,
    /// vertically scrollable viewport.
    Scroll,
    /// A leaf.
    Widget(Widget),
}

/// One node of the tree.
pub struct Node {
    /// Optional identifier; required to scroll a `Scroll` node and to
    /// fetch a rectangle back from the [`Layout`].
    pub id: Option<Id>,
    /// What this node is.
    pub kind: Kind,
    /// Flex grow factor along the parent's main axis. Zero means "natural
    /// size".
    pub weight: f32,
    /// Flex shrink factor, for a node that gives height back when its
    /// parent overflows but never takes more than its natural size.
    /// Zero falls back to `weight`, which does both.
    pub shrink: f32,
    /// Minimum size in dp (per axis, `None` = no minimum).
    pub min: (Option<f32>, Option<f32>),
    /// Maximum size in dp (per axis, `None` = no maximum).
    pub max: (Option<f32>, Option<f32>),
    /// Fixed size in dp (per axis); overrides measurement and flex.
    pub fixed: (Option<f32>, Option<f32>),
    /// Inner padding in dp.
    pub padding: Edges,
    /// Space between children in dp (containers only).
    pub gap: f32,
    /// Cross-axis alignment applied to children (containers only).
    pub align: Align,
    /// Overrides the parent's `align` for this node.
    pub align_self: Option<Align>,
    /// Main-axis distribution (containers only).
    pub justify: Justify,
    /// A `Scroll` region whose content is at least as tall as the
    /// viewport, so that a body which fits is placed in the whole space
    /// rather than packed at its top (`docs/DESIGN.md` §2.6).
    pub fill: bool,
    /// Children (containers only; `Scroll` uses the first).
    pub children: Vec<Node>,
}

impl Node {
    fn with_kind(kind: Kind) -> Self {
        Node {
            id: None,
            kind,
            weight: 0.0,
            min: (None, None),
            max: (None, None),
            fixed: (None, None),
            padding: Edges::default(),
            gap: 0.0,
            align: Align::Stretch,
            align_self: None,
            justify: Justify::Start,
            shrink: 0.0,
            fill: false,
            children: Vec::new(),
        }
    }

    /// A column.
    pub fn column() -> Self {
        Node::with_kind(Kind::Column)
    }

    /// A row.
    pub fn row() -> Self {
        Node::with_kind(Kind::Row)
    }

    /// A stack.
    pub fn stack() -> Self {
        Node::with_kind(Kind::Stack)
    }

    /// A vertical scroll region around `content`, scrolled by `id`.
    pub fn scroll(id: Id, content: Node) -> Self {
        let mut n = Node::with_kind(Kind::Scroll);
        n.id = Some(id);
        n.children.push(content);
        n
    }

    /// A leaf widget.
    pub fn widget(widget: Widget) -> Self {
        Node::with_kind(Kind::Widget(widget))
    }

    /// An empty node that takes leftover space.
    pub fn spacer() -> Self {
        Node::column().weight(1.0)
    }

    /// Gives a scroll region's content the whole viewport when it fits
    /// in it, so a body placed by `docs/DESIGN.md` §2.6 is centred in
    /// the space it has and only a body that outgrows it starts at the
    /// top and scrolls.
    pub fn filling(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Sets the identifier.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the flex weight.
    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Sets the flex shrink factor: the node gives height back when its
    /// parent overflows, and never grows past its natural size.
    pub fn shrink(mut self, shrink: f32) -> Self {
        self.shrink = shrink;
        self
    }

    /// Sets the minimum width in dp.
    pub fn min_width(mut self, dp: f32) -> Self {
        self.min.0 = Some(dp);
        self
    }

    /// Sets the minimum height in dp.
    pub fn min_height(mut self, dp: f32) -> Self {
        self.min.1 = Some(dp);
        self
    }

    /// Sets the maximum width in dp.
    pub fn max_width(mut self, dp: f32) -> Self {
        self.max.0 = Some(dp);
        self
    }

    /// Sets the maximum height in dp.
    pub fn max_height(mut self, dp: f32) -> Self {
        self.max.1 = Some(dp);
        self
    }

    /// Sets a fixed width in dp.
    pub fn width(mut self, dp: f32) -> Self {
        self.fixed.0 = Some(dp);
        self
    }

    /// Sets a fixed height in dp.
    pub fn height(mut self, dp: f32) -> Self {
        self.fixed.1 = Some(dp);
        self
    }

    /// Sets the padding.
    pub fn padding(mut self, edges: Edges) -> Self {
        self.padding = edges;
        self
    }

    /// Sets the same padding on all sides, in dp.
    pub fn pad(self, dp: f32) -> Self {
        self.padding(Edges::all(dp))
    }

    /// Sets the gap between children, in dp.
    pub fn gap(mut self, dp: f32) -> Self {
        self.gap = dp;
        self
    }

    /// Sets the cross-axis alignment of children.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Overrides the parent's cross-axis alignment for this node.
    pub fn align_self(mut self, align: Align) -> Self {
        self.align_self = Some(align);
        self
    }

    /// Sets the main-axis distribution.
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// Appends a child.
    pub fn child(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }

    /// Appends children.
    pub fn children(mut self, children: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(children);
        self
    }

    /// The widget, if this is a leaf.
    pub fn as_widget(&self) -> Option<&Widget> {
        match &self.kind {
            Kind::Widget(w) => Some(w),
            _ => None,
        }
    }
}

/// Everything measurement needs to know about the display.
#[derive(Debug, Clone, Copy)]
pub struct LayoutCtx {
    /// dp scale.
    pub scale: Scale,
    /// Size class.
    pub class: SizeClass,
    /// The shell's top inset in dp (`DisplayInfo::inset_top`).
    pub inset_top_dp: f32,
    /// The shell's bottom inset in dp (`DisplayInfo::inset_bottom`):
    /// what a keyboard's bottom row and a bottom action stand clear of.
    pub inset_bottom_dp: f32,
}

impl LayoutCtx {
    /// A context with no insets. The scale takes the class's type ramp,
    /// so measurement and drawing always agree about how large a font
    /// is.
    pub fn new(scale: Scale, class: SizeClass) -> Self {
        LayoutCtx {
            scale: scale.with_class(class),
            class,
            inset_top_dp: 0.0,
            inset_bottom_dp: 0.0,
        }
    }

    /// The same context with the insets the shell reported, in dp.
    pub fn with_insets(self, top_dp: f32, bottom_dp: f32) -> Self {
        LayoutCtx {
            inset_top_dp: top_dp,
            inset_bottom_dp: bottom_dp,
            ..self
        }
    }

    /// dp to whole pixels.
    pub fn px(&self, dp: f32) -> i32 {
        self.scale.px(Dp(dp))
    }
}

/// Scroll offsets by scroll-node id, in pixels. Kept by [`crate::UiState`]
/// across frames and read by [`solve`].
#[derive(Debug, Clone, Default)]
pub struct ScrollOffsets {
    entries: Vec<(Id, i32)>,
}

impl ScrollOffsets {
    /// Offset of `id`, zero when unknown.
    pub fn get(&self, id: Id) -> i32 {
        self.find(id).unwrap_or(0)
    }

    /// Offset of `id`, `None` when the region has not been scrolled.
    pub fn find(&self, id: Id) -> Option<i32> {
        self.entries.iter().find(|(i, _)| *i == id).map(|(_, o)| *o)
    }

    /// Sets the offset of `id`.
    pub fn set(&mut self, id: Id, offset: i32) {
        match self.entries.iter_mut().find(|(i, _)| *i == id) {
            Some(e) => e.1 = offset,
            None => self.entries.push((id, offset)),
        }
    }
}

/// One solved node.
#[derive(Debug, Clone, Copy)]
pub struct Placed {
    /// The node's id, if any.
    pub id: Option<Id>,
    /// Pixel rectangle.
    pub rect: Rect,
    /// Visible region: the intersection of enclosing scroll viewports.
    pub clip: Rect,
    /// The interactive target, if the node is a widget that reacts to
    /// input.
    pub hit: Option<HitTarget>,
    /// The application id a key can move focus to
    /// (`docs/DESIGN.md` §4.15): everything a tap could act on, and a
    /// dimmed control, which takes focus and does nothing. A keyboard's
    /// keys and a candidate strip's cells are typed, never focused.
    pub focus: Option<Id>,
    /// For `Scroll` nodes: the content height and the applied offset.
    pub scroll: Option<ScrollInfo>,
    /// Depth in the tree (root is 0).
    pub depth: u16,
    /// Whether the node paints anything itself: true for a widget, false
    /// for a row, column, stack or scroll region, whose rectangle is
    /// only the space its children were given.
    pub draws: bool,
}

/// Scroll geometry recorded for a `Scroll` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollInfo {
    /// Height of the content in pixels.
    pub content_h: i32,
    /// Offset that was applied (already clamped).
    pub offset: i32,
    /// The visible part of the region: its rectangle less its padding,
    /// inside any enclosing clip. The content is drawn in it and moves
    /// inside it.
    pub viewport: Rect,
}

impl ScrollInfo {
    /// Largest valid offset.
    pub fn max_offset(&self) -> i32 {
        (self.content_h - self.viewport.h).max(0)
    }
}

/// The solved tree, in pre-order (which is also drawing order).
#[derive(Debug, Clone)]
pub struct Layout {
    /// Solved nodes.
    pub items: Vec<Placed>,
    /// The context it was solved with.
    pub ctx: LayoutCtx,
}

impl Layout {
    /// An empty layout for `ctx`.
    pub fn empty(ctx: LayoutCtx) -> Self {
        Layout {
            items: Vec::new(),
            ctx,
        }
    }

    /// The rectangle of the node with `id` (a node id, or the application
    /// id of a widget whose node has none).
    pub fn rect(&self, id: Id) -> Option<Rect> {
        self.items.iter().find(|p| p.id == Some(id)).map(|p| p.rect)
    }

    /// The placed node with `id`.
    pub fn placed(&self, id: Id) -> Option<&Placed> {
        self.items.iter().find(|p| p.id == Some(id))
    }

    /// The topmost interactive widget under `(x, y)`.
    pub fn hit_test(&self, x: i32, y: i32) -> Option<&Placed> {
        self.items
            .iter()
            .rev()
            .find(|p| p.hit.is_some() && p.clip.contains(x, y) && p.rect.contains(x, y))
    }

    /// Moves scroll region `id` to `offset` without solving the tree
    /// again: every item inside the region moves by the difference
    /// between the old offset and the new one, and the region records
    /// the new offset. What comes out is what [`solve_with`] would have
    /// produced for the same tree at the same offset.
    ///
    /// Returns how far the content moved in pixels, positive when it
    /// moved up. Zero when it did not move, and when the region is
    /// clipped by an enclosing one or contains one, whose viewports move
    /// with the content: those the caller solves again.
    pub fn scroll_to(&mut self, id: Id, offset: i32) -> i32 {
        let Some(index) = self
            .items
            .iter()
            .position(|p| p.id == Some(id) && p.scroll.is_some())
        else {
            return 0;
        };
        let region = self.items[index];
        let info = match region.scroll {
            Some(info) => info,
            None => return 0,
        };
        let clipped = region.clip.intersect(&region.rect) != region.rect;
        let end = self.items[index + 1..]
            .iter()
            .position(|p| p.depth <= region.depth)
            .map_or(self.items.len(), |n| index + 1 + n);
        let nested = self.items[index + 1..end]
            .iter()
            .any(|p| p.scroll.is_some());
        if clipped || nested {
            return 0;
        }
        let dy = offset.clamp(0, info.max_offset()) - info.offset;
        if dy == 0 {
            return 0;
        }
        self.items[index].scroll = Some(ScrollInfo {
            offset: info.offset + dy,
            ..info
        });
        for p in &mut self.items[index + 1..end] {
            p.rect.y -= dy;
        }
        dy
    }

    /// The innermost scroll region under `(x, y)`.
    pub fn scroll_at(&self, x: i32, y: i32) -> Option<&Placed> {
        self.items
            .iter()
            .rev()
            .find(|p| p.scroll.is_some() && p.clip.contains(x, y) && p.rect.contains(x, y))
    }
}

/// Solves `root` into `area` for a display, with no scroll offsets.
pub fn solve(root: &Node, area: Rect, ctx: &LayoutCtx) -> Layout {
    solve_with(root, area, ctx, &ScrollOffsets::default())
}

/// Solves `root` into `area`, applying `scroll` offsets.
pub fn solve_with(root: &Node, area: Rect, ctx: &LayoutCtx, scroll: &ScrollOffsets) -> Layout {
    let mut layout = Layout::empty(*ctx);
    let mut solver = Solver { ctx, scroll };
    solver.place(root, area, area, 0, &mut layout);
    layout
}

struct Solver<'a> {
    ctx: &'a LayoutCtx,
    scroll: &'a ScrollOffsets,
}

/// A node's dp constraints converted to pixels.
struct Px {
    min: Size,
    max: Size,
    fixed: (Option<i32>, Option<i32>),
    padding: PxEdges,
    gap: i32,
}

impl<'a> Solver<'a> {
    fn px(&self, node: &Node) -> Px {
        let p = |dp: Option<f32>| dp.map(|d| self.ctx.px(d));
        Px {
            min: Size::new(p(node.min.0).unwrap_or(0), p(node.min.1).unwrap_or(0)),
            max: Size::new(
                p(node.max.0).unwrap_or(i32::MAX),
                p(node.max.1).unwrap_or(i32::MAX),
            ),
            fixed: (p(node.fixed.0), p(node.fixed.1)),
            padding: node.padding.to_px(self.ctx.scale),
            gap: self.ctx.px(node.gap),
        }
    }

    /// Clamps a measured size to the node's min/max/fixed.
    fn clamp(px: &Px, size: Size) -> Size {
        let w = px.fixed.0.unwrap_or(size.w.max(px.min.w).min(px.max.w));
        let h = px.fixed.1.unwrap_or(size.h.max(px.min.h).min(px.max.h));
        Size::new(w, h)
    }

    /// Intrinsic size of `node` when offered at most `max`.
    fn measure(&self, node: &Node, max: Size) -> Size {
        let px = self.px(node);
        // Fixed sizes short-circuit; a fixed axis is also the offered max.
        let max = Size::new(
            px.fixed.0.unwrap_or(max.w).min(px.max.w),
            px.fixed.1.unwrap_or(max.h).min(px.max.h),
        );
        let inner = Size::new(
            (max.w - px.padding.horizontal()).max(0),
            (max.h - px.padding.vertical()).max(0),
        );
        let content = match &node.kind {
            Kind::Widget(w) => w.measure(self.ctx, inner),
            Kind::Column => {
                let mut w = 0;
                let mut h = 0;
                for (i, c) in node.children.iter().enumerate() {
                    let s = self.measure(c, inner);
                    w = w.max(s.w);
                    h += s.h + if i > 0 { px.gap } else { 0 };
                }
                Size::new(w, h)
            }
            Kind::Row => {
                let mut w = 0;
                let mut h = 0;
                for (i, c) in node.children.iter().enumerate() {
                    let s = self.measure(c, inner);
                    h = h.max(s.h);
                    w += s.w + if i > 0 { px.gap } else { 0 };
                }
                Size::new(w, h)
            }
            Kind::Stack => node
                .children
                .iter()
                .fold(Size::ZERO, |acc, c| acc.max(self.measure(c, inner))),
            Kind::Scroll => {
                let content = node.children.first().map_or(Size::ZERO, |c| {
                    self.measure(c, Size::new(inner.w, i32::MAX))
                });
                Size::new(content.w, content.h.min(inner.h))
            }
        };
        let with_padding = Size::new(
            content.w.saturating_add(px.padding.horizontal()),
            content.h.saturating_add(px.padding.vertical()),
        );
        Self::clamp(&px, with_padding)
    }

    fn place(&mut self, node: &Node, rect: Rect, clip: Rect, depth: u16, out: &mut Layout) {
        let px = self.px(node);
        let index = out.items.len();
        let hit = node.as_widget().and_then(|w| match w.hit_target() {
            // A chunked string is only interactive once it is paged, which
            // depends on the rectangle it was given.
            Some(HitTarget::Pager { id, .. }) => {
                let pages = crate::widgets::chunked_pages(w, self.ctx, rect);
                (pages > 1).then_some(HitTarget::Pager {
                    id,
                    pages: pages.min(255) as u8,
                })
            }
            // A candidate strip shows as many words as fit its width, in
            // as many cells as it divides that width into.
            Some(HitTarget::Candidates { id, .. }) => {
                let count = crate::widgets::candidate_count(w, self.ctx, rect.w);
                let cells = crate::widgets::candidate_cells(w, self.ctx, rect.w);
                (count > 0).then_some(HitTarget::Candidates { id, count, cells })
            }
            other => other,
        });
        let focus = match hit {
            Some(HitTarget::Tap(id) | HitTarget::Hold(id) | HitTarget::Reveal(id)) => Some(id),
            Some(HitTarget::Pager { id, .. }) => Some(id),
            Some(HitTarget::Keyboard { .. } | HitTarget::Candidates { .. }) => None,
            None => node.as_widget().and_then(Widget::dimmed_id),
        };
        out.items.push(Placed {
            id: node.id.or_else(|| node.as_widget().and_then(Widget::id)),
            rect,
            clip,
            hit,
            focus,
            scroll: None,
            depth,
            draws: matches!(node.kind, Kind::Widget(_)),
        });
        let inner = rect.inset(px.padding);
        match &node.kind {
            Kind::Widget(_) => {}
            Kind::Column => self.place_flex(node, &px, inner, clip, depth, true, out),
            Kind::Row => self.place_flex(node, &px, inner, clip, depth, false, out),
            Kind::Stack => {
                for c in &node.children {
                    let align = c.align_self.unwrap_or(node.align);
                    let r = if align == Align::Stretch {
                        inner
                    } else {
                        let s = self.measure(c, inner.size());
                        let x = match align {
                            Align::Start | Align::Stretch => inner.x,
                            Align::Center => inner.x + (inner.w - s.w) / 2,
                            Align::End => inner.right() - s.w,
                        };
                        let y = match align {
                            Align::Start | Align::Stretch => inner.y,
                            Align::Center => inner.y + (inner.h - s.h) / 2,
                            Align::End => inner.bottom() - s.h,
                        };
                        Rect::new(x, y, s.w, s.h)
                    };
                    self.place(c, r, clip, depth + 1, out);
                }
            }
            Kind::Scroll => {
                let Some(content) = node.children.first() else {
                    return;
                };
                let size = self.measure(content, Size::new(inner.w, i32::MAX));
                // A filling region gives its content the whole viewport
                // where the content fits in it, so what is placed inside
                // has the space to be centred in.
                let content_h = if node.fill {
                    size.h.max(inner.h)
                } else {
                    size.h
                };
                let max_offset = (content_h - inner.h).max(0);
                let offset = node
                    .id
                    .and_then(|id| self.scroll.find(id))
                    .unwrap_or(0)
                    .clamp(0, max_offset);
                let viewport = clip.intersect(&inner);
                out.items[index].scroll = Some(ScrollInfo {
                    content_h,
                    offset,
                    viewport,
                });
                let r = Rect::new(inner.x, inner.y - offset, inner.w, content_h);
                self.place(content, r, viewport, depth + 1, out);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn place_flex(
        &mut self,
        node: &Node,
        px: &Px,
        inner: Rect,
        clip: Rect,
        depth: u16,
        vertical: bool,
        out: &mut Layout,
    ) {
        let n = node.children.len();
        if n == 0 {
            return;
        }
        let main_avail = if vertical { inner.h } else { inner.w };
        let cross_avail = if vertical { inner.w } else { inner.h };

        // Pass 1: intrinsic sizes.
        let mut sizes: Vec<Size> = Vec::with_capacity(n);
        let mut mains: Vec<i32> = Vec::with_capacity(n);
        for c in &node.children {
            let s = self.measure(c, inner.size());
            sizes.push(s);
            mains.push(if vertical { s.h } else { s.w });
        }
        let gaps = px.gap * (n as i32 - 1);
        let total: i32 = mains.iter().sum::<i32>() + gaps;
        let free = main_avail - total;
        let weight_sum: f32 = node.children.iter().map(|c| c.weight.max(0.0)).sum();
        let shrink_of = |c: &Node| if c.shrink > 0.0 { c.shrink } else { c.weight };
        let shrink_sum: f32 = node.children.iter().map(|c| shrink_of(c).max(0.0)).sum();

        if free > 0 && weight_sum > 0.0 {
            // Grow weighted children, respecting their maxima; redistribute
            // what a capped child could not take.
            let mut remaining = free;
            let mut pool: Vec<usize> = (0..n).filter(|&i| node.children[i].weight > 0.0).collect();
            while remaining > 0 && !pool.is_empty() {
                let wsum: f32 = pool.iter().map(|&i| node.children[i].weight).sum();
                let mut next_pool = Vec::new();
                let mut given = 0;
                for &i in &pool {
                    let c = &node.children[i];
                    let cpx = self.px(c);
                    let cap = if vertical { cpx.max.h } else { cpx.max.w };
                    let fixed = if vertical { cpx.fixed.1 } else { cpx.fixed.0 };
                    if fixed.is_some() {
                        continue;
                    }
                    let share = ((remaining as f32) * c.weight / wsum) as i32;
                    let room = (cap - mains[i]).max(0);
                    let take = share.min(room);
                    mains[i] += take;
                    given += take;
                    if take < share {
                        // Capped; drop from the pool.
                    } else {
                        next_pool.push(i);
                    }
                }
                remaining -= given;
                if given == 0 {
                    break;
                }
                pool = next_pool;
            }
        } else if free < 0 && shrink_sum > 0.0 {
            // Overflow: shrink the children that carry a shrink factor
            // proportionally, not below their minimum; children that
            // reach it drop out and the rest absorb what is left.
            let mut deficit = -free;
            let mut pool: Vec<usize> = (0..n)
                .filter(|&i| shrink_of(&node.children[i]) > 0.0)
                .collect();
            while deficit > 0 && !pool.is_empty() {
                let wsum: f32 = pool.iter().map(|&i| shrink_of(&node.children[i])).sum();
                let mut next_pool = Vec::new();
                let mut taken = 0;
                for &i in &pool {
                    let c = &node.children[i];
                    let cpx = self.px(c);
                    let min = if vertical { cpx.min.h } else { cpx.min.w };
                    let fixed = if vertical { cpx.fixed.1 } else { cpx.fixed.0 };
                    if fixed.is_some() {
                        continue;
                    }
                    let share = ((deficit as f32) * shrink_of(c) / wsum) as i32 + 1;
                    let room = (mains[i] - min).max(0);
                    let take = share.min(room).min(deficit - taken);
                    mains[i] -= take;
                    taken += take;
                    if take < share && room == take {
                        // At its minimum; drop from the pool.
                    } else {
                        next_pool.push(i);
                    }
                }
                deficit -= taken;
                if taken == 0 {
                    break;
                }
                pool = next_pool;
            }
        }

        // Main-axis justification of leftover space.
        let used: i32 = mains.iter().sum::<i32>() + gaps;
        let leftover = (main_avail - used).max(0);
        let (mut cursor, between) = match node.justify {
            Justify::Start => (0, 0),
            Justify::Center => (leftover / 2, 0),
            Justify::End => (leftover, 0),
            Justify::SpaceBetween if n > 1 => (0, leftover / (n as i32 - 1)),
            Justify::SpaceBetween => (0, 0),
        };

        // Pass 2: cross axis and placement.
        for (i, c) in node.children.iter().enumerate() {
            let align = c.align_self.unwrap_or(node.align);
            let cpx = self.px(c);
            let cross_fixed = if vertical { cpx.fixed.0 } else { cpx.fixed.1 };
            let cross_max = if vertical { cpx.max.w } else { cpx.max.h };
            let cross_nat = if vertical { sizes[i].w } else { sizes[i].h };
            let cross = cross_fixed.unwrap_or(match align {
                Align::Stretch => cross_avail.min(cross_max),
                _ => cross_nat.min(cross_avail),
            });
            let cross_off = match align {
                Align::Start | Align::Stretch => 0,
                Align::Center => (cross_avail - cross) / 2,
                Align::End => cross_avail - cross,
            };
            let main = mains[i];
            let r = if vertical {
                Rect::new(inner.x + cross_off, inner.y + cursor, cross, main)
            } else {
                Rect::new(inner.x + cursor, inner.y + cross_off, main, cross)
            };
            self.place(c, r, clip, depth + 1, out);
            cursor += main + px.gap + between;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Font;
    use crate::widgets::Tone;

    fn ctx() -> LayoutCtx {
        LayoutCtx::new(Scale::IDENTITY, SizeClass::Mobile)
    }

    fn area() -> Rect {
        Rect::new(0, 0, 300, 200)
    }

    fn fixed_box(id: u32, w: f32, h: f32) -> Node {
        Node::column().id(Id(id)).width(w).height(h)
    }

    #[test]
    fn scroll_content_taller_than_viewport_is_offset_and_clipped() {
        let content = Node::column().children((0..10).map(|i| fixed_box(100 + i, 100.0, 50.0)));
        let root = Node::column()
            .child(fixed_box(1, 100.0, 40.0))
            .child(Node::scroll(Id(9), content).weight(1.0));
        let l = solve(&root, area(), &ctx());
        let sc = l.placed(Id(9)).unwrap();
        assert_eq!(sc.rect, Rect::new(0, 40, 300, 160));
        assert_eq!(sc.scroll.unwrap().content_h, 500);
        assert_eq!(sc.scroll.unwrap().offset, 0);
        assert_eq!(l.rect(Id(100)).unwrap().y, 40);
        let last = l.placed(Id(109)).unwrap();
        assert_eq!(last.rect.y, 40 + 450);
        assert_eq!(last.clip, Rect::new(0, 40, 300, 160));

        let mut offsets = ScrollOffsets::default();
        offsets.set(Id(9), 10_000); // clamped to content - viewport
        let l = solve_with(&root, area(), &ctx(), &offsets);
        assert_eq!(l.placed(Id(9)).unwrap().scroll.unwrap().offset, 340);
        assert_eq!(l.rect(Id(109)).unwrap().bottom(), 200);
    }

    /// A header above a long list in a scroll region, the shape of the
    /// Settings screen.
    fn settings_like() -> Node {
        use crate::widgets::ButtonStyle;
        let rows = (0..40).map(|i| {
            Node::row()
                .gap(8.0)
                .child(Node::widget(Widget::text(
                    "A setting",
                    Font::regular(14.0),
                    Tone::Text,
                )))
                .child(Node::spacer())
                .child(
                    Node::widget(Widget::button(Id(100 + i), "Change", ButtonStyle::Primary))
                        .width(80.0),
                )
        });
        Node::column()
            .child(Node::widget(Widget::text(
                "Settings",
                Font::regular(20.0),
                Tone::Text,
            )))
            .child(Node::scroll(Id(9), Node::column().gap(6.0).children(rows)).weight(1.0))
    }

    #[test]
    fn a_scrolled_layout_shows_what_a_fresh_solve_would() {
        use osk_shell_api::{BootState, DisplayInfo, SecureHardware};
        for (w, h, dpi) in [
            (240, 320, 143),
            (480, 640, 286),
            (1080, 2340, 420),
            (960, 640, 160),
        ] {
            let display = DisplayInfo {
                width: w as u16,
                height: h as u16,
                dpi,
                inset_top: 0,
                inset_bottom: 0,
                buttons: 1,
                camera_fixed: false,
                secure: SecureHardware::None,
                boot: BootState::Unknown,
                memory_mib: None,
            };
            let ctx = LayoutCtx::new(Scale::new(dpi), SizeClass::of(&display));
            let area = Rect::new(0, 0, w, h);
            let root = settings_like();
            let mut offsets = ScrollOffsets::default();
            let mut moved = solve_with(&root, area, &ctx, &offsets);
            for offset in [37, 400, 1_000_000, 120, 0] {
                offsets.set(Id(9), offset);
                let fresh = solve_with(&root, area, &ctx, &offsets);
                moved.scroll_to(Id(9), offset);
                assert_eq!(moved.items.len(), fresh.items.len());
                for (a, b) in moved.items.iter().zip(&fresh.items) {
                    assert_eq!(a.id, b.id);
                    assert_eq!(a.rect, b.rect, "{w}x{h} at {offset}");
                    assert_eq!(a.clip, b.clip, "{w}x{h} at {offset}");
                    assert_eq!(a.hit, b.hit);
                    assert_eq!(a.scroll, b.scroll, "{w}x{h} at {offset}");
                    assert_eq!(a.depth, b.depth);
                }
            }
        }
    }

    #[test]
    fn hit_test_finds_topmost_interactive_widget() {
        use crate::widgets::ButtonStyle;
        let root = Node::stack()
            .child(Node::widget(Widget::button(
                Id(1),
                "Under",
                ButtonStyle::Primary,
            )))
            .child(
                Node::column()
                    .child(
                        Node::widget(Widget::button(Id(2), "Over", ButtonStyle::Primary))
                            .height(30.0),
                    )
                    .child(Node::widget(Widget::text(
                        "label",
                        Font::regular(14.0),
                        Tone::Text,
                    ))),
            );
        let l = solve(&root, area(), &ctx());
        assert_eq!(l.hit_test(5, 5).unwrap().hit, Some(HitTarget::Tap(Id(2))));
        assert_eq!(l.hit_test(5, 100).unwrap().hit, Some(HitTarget::Tap(Id(1))));
        assert!(l.hit_test(1000, 1000).is_none());
    }
}
