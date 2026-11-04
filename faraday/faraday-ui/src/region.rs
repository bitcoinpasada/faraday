//! The scrolled regions a frame draws: which offset each moves
//! ([`Slot`]) and where each was drawn and how far it goes
//! ([`Scrolled`]), for the app to move the one under the pointer.

use crate::Ui;
use crate::pal::Role;
use faraday_draw::Rect;

/// Which offset a scrolled region moves. A screen can scroll more than
/// one region at once, each by its own offset; the wheel, a trackpad and
/// a finger move the one under the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slot {
    /// The screen's region, or the open sheet's when it scrolls.
    #[default]
    Page,
    /// Stick visit's Outbox list (`VisitState::out_offset`).
    VisitOut,
    /// The wallet card's body on a desktop (`Faraday::card_offset`).
    Card,
    /// Vault contents' item on a desktop (`VaultArea::item_offset`, `faraday-core`).
    VaultItem,
    /// Files' For the stick list on a desktop, beside From the stick
    /// (`Faraday::files_out_offset`).
    FilesOut,
    /// The list pane of a list and detail page, beside its detail
    /// (`docs/LAYOUT.md` §5; `Panes::list_offset`, `faraday-core`).
    ListPane,
    /// The backup board's list in the right-hand column on a desktop
    /// (`Faraday::board_list_offset`).
    BoardList,
    /// A codex32 checksum worksheet, which pans both ways
    /// (`SheetPan`, `faraday-core`).
    Worksheet,
    /// A codex32 checksum worksheet drawn whole: it moves no offset of
    /// its own, and says where it stands for the page to follow its work
    /// (`Codex32Make::follow_page`, `faraday-core`).
    WorksheetWhole,
    /// The codex32 entry's "Demo only" card, a supporting pane that
    /// scrolls within itself (`Codex32Make::pane_offset`, `faraday-core`).
    DemoPane,
    /// The codex32 entry's input panel beside the sheets, which scrolls
    /// within itself where it is taller than the window
    /// (`docs/LAYOUT.md` §16.3; `Codex32Make::panel_offset`,
    /// `faraday-core`).
    InputPanel,
    /// Learn's supporting pane on an extra-large window, which scrolls
    /// within itself beside the page (`docs/LAYOUT.md` §6;
    /// `LearnState::scroll`, `faraday-core`).
    LearnPane,
    /// The desktop sidebar's lists of keys and wallets, which scroll
    /// within the sidebar where they are taller than its room
    /// (`Scroll::sidebar_offset`, `faraday-core`).
    Sidebar,
    /// Where the codex32 dice sheet's work is while the step-through
    /// rolls: the marked die's track, or the tree on the character's
    /// step; and, in the tour of the shares made, the marked cells of a
    /// translation worksheet. It moves no offset of its own, and says where it stands for
    /// the page to follow (`Faraday::dice_follow`, `faraday-core`).
    DiceWork,
}

/// A scrolled region as drawn: where it is and how far it goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scrolled {
    /// The offset it moves.
    pub slot: Slot,
    /// Where it is seen on the panel, in pixels.
    pub view: Rect,
    /// How far its content scrolls, in design units.
    pub max: f32,
    /// It draws its own scrollbar.
    pub own_bar: bool,
    /// How far its content pans sideways, in design units: 0 for a
    /// region that only scrolls up and down.
    pub max_x: f32,
}

/// Whether a region's content is a function of its state and offset
/// alone, so a scroll step may move the rows already drawn and paint only
/// the rows it brings into view (`docs/FRAMES.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Moves {
    /// The region is drawn whole each frame.
    #[default]
    No,
    /// The region may be drawn as a moved frame.
    Yes,
}

/// A region about to be drawn: where it is seen, what it sits on and
/// which offset it moves ([`Ui::region`]).
#[derive(Debug, Clone, Copy)]
pub struct Region {
    slot: Slot,
    view: Rect,
    ground: Role,
    own_bar: bool,
    offset: Option<f32>,
    pan: f32,
    whole: Option<f32>,
    moves: Moves,
}

impl Region {
    /// The screen's or the open sheet's region seen through `view`, moved
    /// by [`Ui::offset`], on the page, with the page's overlay scrollbar.
    pub fn page(view: Rect) -> Self {
        Region {
            slot: Slot::Page,
            view,
            ground: Role::Surface,
            own_bar: false,
            offset: None,
            pan: 0.0,
            whole: None,
            moves: Moves::No,
        }
    }

    /// The region `slot` seen through `view`, moved by its own `offset`
    /// (units); the page's scrollbar is shown at its edge while it is the
    /// one scrolled.
    pub fn beside(slot: Slot, view: Rect, offset: f32) -> Self {
        Region {
            slot,
            offset: Some(offset),
            ..Self::page(view)
        }
    }

    /// `slot`'s region seen through `view`, laid out from offset 0 and
    /// moved by its own `offset` ([`Region::beside`] and
    /// [`Region::from_zero`]).
    pub fn scrolled(slot: Slot, view: Rect, offset: f32) -> Self {
        Self::beside(slot, view, offset).from_zero(offset)
    }

    /// On the container `ground` rather than on the page (a dialog, a
    /// sheet, a menu): its edges fade into that container, and that
    /// container shows in the gap a stretch opens (`docs/THEMES.md` §6, §7).
    pub fn on(self, ground: Role) -> Self {
        Region { ground, ..self }
    }

    /// The region draws its own scrollbar.
    pub fn own_bar(self) -> Self {
        Region {
            own_bar: true,
            ..self
        }
    }

    /// The region's content also pans sideways, `x` units now (its extent
    /// across is the content's [`Extent::across`]).
    pub fn panned(self, x: f32) -> Self {
        Region { pan: x, ..self }
    }

    /// The content is laid out at offset 0 and moved by the whole pixels
    /// of the region's offset after each place is rounded
    /// ([`Ui::oy_px`]), so a row's place is `round(y * f) - offset_px`,
    /// never `round((y - offset) * f)` (`docs/FRAMES.md` §3.2). The
    /// screen must then not take `by`, the offset the content is moved
    /// by (the region's offset, as far as the content allows), off its
    /// own `y`.
    pub fn from_zero(self, by: f32) -> Self {
        Region {
            whole: Some(by),
            ..self
        }
    }

    /// Says whether the region may be drawn as a moved frame. It must be
    /// laid out from zero ([`Region::from_zero`]), and its content must
    /// depend on nothing but its state and offset.
    pub fn moves(self, moves: Moves) -> Self {
        Region { moves, ..self }
    }
}

/// The region the core would move the rows of, for [`Ui`] to draw as a
/// moved frame (`docs/FRAMES.md` §3.1): the region the last frame kept
/// the content layer of, and the whole pixels its offset was at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    /// The region.
    pub slot: Slot,
    /// Where it was seen.
    pub view: Rect,
    /// The whole pixels its offset moved its content by.
    pub px: i32,
}

/// How far a region's content goes past its view, units: down, and
/// across where it pans.
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

/// A region opened with [`Ui::region_open`], closed with
/// [`Ui::region_close`].
#[derive(Debug)]
#[must_use = "a region opened is closed with Ui::region_close"]
pub struct Open {
    region: Region,
    oy_px: i32,
    /// The rows the offset moved the content by, on a moved frame.
    moved: Option<i32>,
    /// What [`Ui::seen_clip`] read before the region.
    seen: Option<Rect>,
}

/// Which region the canvas's content layer ([`Canvas::layer`]) is of: the
/// active region's content as its content left it, before the stretch, the
/// edge fades and the scrollbar (`docs/FRAMES.md` §3.1).
#[derive(Debug, Default)]
pub struct Layer {
    /// The region the pixels are of, with its view.
    pub of: Option<(Slot, Rect)>,
    /// The whole pixels the region's offset moved its content by.
    pub px: i32,
    /// The frame drew the region as a moved frame, by this many rows.
    pub moved: Option<i32>,
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Draws a scrolled region: the content `content` draws is clipped to
    /// the region's view, and the region is reported once it is drawn
    /// (what [`Ui::region_close`] says). The content returns how far it
    /// goes past the view, units.
    pub fn region<R: Into<Extent>>(
        &mut self,
        region: Region,
        content: impl FnOnce(&mut Self) -> R,
    ) {
        let open = self.region_open(region);
        let reach = content(self).into();
        self.region_close(open, reach);
    }

    /// The first half of [`Ui::region`], for a screen whose content does
    /// not fit one closure: pushes the view's clip and, for a region
    /// laid out from zero, the pixels its offset moves it by.
    pub fn region_open(&mut self, region: Region) -> Open {
        let oy_px = self.oy_px;
        let seen = self.seen;
        let mut moved = None;
        let mut clip = region.view;
        if let Some(by) = region.whole {
            let px = self.px(by);
            self.oy_px = oy_px - px;
            if let Some(d) = self.moves_by(&region, px) {
                // The whole view is seen for hits and for what a row
                // skips; only the strip the move uncovered is painted.
                self.seen = Some(self.c.clip().intersect(&region.view));
                clip = match d {
                    0 => Rect::new(clip.x, clip.y, clip.w, 0),
                    d if d > 0 => Rect::new(clip.x, clip.bottom() - d, clip.w, d),
                    d => Rect::new(clip.x, clip.y, clip.w, -d),
                };
                moved = Some(d);
            }
        }
        self.c.push_clip(clip);
        Open {
            region,
            oy_px,
            moved,
            seen,
        }
    }

    /// How many rows this frame's region moves the layer by, when it is
    /// drawn as a moved frame: the region is the one the core planned
    /// to move and is the active one, its screen says it moves, and the
    /// move leaves rows of the layer in view.
    fn moves_by(&self, region: &Region, px: i32) -> Option<i32> {
        let plan = self.moved?;
        let d = px - plan.px;
        (region.moves == Moves::Yes
            && self.keep_content
            && plan.slot == region.slot
            && plan.view == region.view
            && region.slot == self.active
            && self.in_sheet == self.stretch_in_sheet
            && d.abs() < region.view.h - 1)
            .then_some(d)
    }

    /// [`Ui::region_open`] for a page whose drawing and closing are in
    /// different functions: the region is kept until [`Ui::region_end`]
    /// (or [`Ui::region_abandon`], for a page that returns without
    /// closing it).
    pub fn region_begin(&mut self, region: Region) {
        self.region_abandon();
        self.begun = Some(self.region_open(region));
    }

    /// [`Ui::region_begin`] for a page `(x0, cw, h)` laid out from zero
    /// and moved by `by` ([`Region::from_zero`]).
    pub fn page_begin(&mut self, (x0, cw, h): (f32, f32, f32), by: f32) {
        let view = self.rect(x0, 0.0, cw, h);
        self.region_begin(Region::page(view).from_zero(by));
    }

    /// [`Ui::page_begin`] for a page whose content depends on nothing
    /// but its state and offset ([`Moves::Yes`]).
    pub fn moving_page_begin(&mut self, (x0, cw, h): (f32, f32, f32), by: f32) {
        let view = self.rect(x0, 0.0, cw, h);
        self.region_begin(Region::page(view).from_zero(by).moves(Moves::Yes));
    }

    /// [`Ui::page_begin`] that returns `top`, where the page's content
    /// starts, for a page that begins at once with its first line.
    pub fn page_top(&mut self, page: (f32, f32, f32), by: f32, top: f32) -> f32 {
        self.page_begin(page, by);
        top
    }

    /// Closes the region [`Ui::region_begin`] opened ([`Ui::region_close`]).
    pub fn region_end(&mut self, reach: impl Into<Extent>) {
        if let Some(open) = self.begun.take() {
            self.region_close(open, reach);
        }
    }

    /// Closes the region [`Ui::region_begin`] opened without reporting
    /// it, if it is still open: its clip and its pixels are given back.
    pub fn region_abandon(&mut self) {
        if let Some(Open {
            oy_px, moved, seen, ..
        }) = self.begun.take()
        {
            self.oy_px = oy_px;
            self.seen = seen;
            self.c.pop_clip();
            self.spoiled |= moved.is_some();
        }
    }

    /// The second half of [`Ui::region`]: pops the clip, takes the
    /// region's content layer, stretches the region past an end, fades
    /// its edges and draws its scrollbar.
    pub fn region_close(&mut self, open: Open, reach: impl Into<Extent>) {
        let Open {
            region,
            oy_px,
            moved,
            seen,
        } = open;
        self.oy_px = oy_px;
        self.seen = seen;
        self.c.pop_clip();
        self.region_finish(region, reach.into(), moved);
    }

    /// [`Ui::region_close`] for a region whose clip the screen pops
    /// itself (or never pushed): the region is reported as drawn, and
    /// stretched, faded and given its scrollbar.
    pub fn region_reported(&mut self, region: Region, reach: impl Into<Extent>) {
        self.region_finish(region, reach.into(), None);
    }

    /// [`Ui::region_reported`], for a region drawn as a moved frame by
    /// `moved` rows when it is: the layer of the frame before, moved, is
    /// put back but for the strip painted, and the layer is taken again
    /// before the overlays.
    fn region_finish(&mut self, region: Region, reach: Extent, moved: Option<i32>) {
        let offset = region.offset.unwrap_or(self.offset);
        let (slot, view, ground) = (region.slot, region.view, region.ground);
        let max = reach.down.max(0.0);
        let scrolled = Scrolled {
            slot,
            view,
            max,
            own_bar: region.own_bar,
            max_x: reach.across.max(0.0),
        };
        match self.scrolled.iter_mut().find(|s| s.slot == slot) {
            Some(s) => *s = scrolled,
            None => self.scrolled.push(scrolled),
        }
        if self.in_sheet != self.stretch_in_sheet {
            return;
        }
        if slot == self.active {
            if let Some(d) = moved {
                self.c.paste_layer(view, -d);
                self.content.moved = Some(d);
            }
            if self.keep_content && region.moves == Moves::Yes {
                self.c.keep_layer(view);
                self.content.of = Some((slot, view));
                self.content.px = region.whole.map_or(0, |by| self.px(by));
            }
            self.stretch_view(view, ground);
        }
        self.edge_fades(view, max, offset, ground);
        // A small panel draws no scrollbar: the edge fades say the page
        // goes on.
        if let Some((alpha, bar_offset)) = self.bar
            && slot == self.active
            && !self.compact
            && !region.own_bar
            && max > 0.0
            && alpha > 0
        {
            self.scroll_bar(view, max, bar_offset, alpha);
        }
        if let Some((alpha, _)) = self.bar
            && slot == self.active
            && !self.compact
            && scrolled.max_x > 0.0
            && alpha > 0
        {
            self.scroll_bar_across(view, scrolled.max_x, region.pan, alpha);
        }
    }
}
