//! A frame from the core to the panel: the rows that changed converted to
//! the panel's format and written, nothing more (`docs/SPEED.md` §11.1).
//!
//! The core paints its whole canvas whenever it draws, so the shell
//! cannot tell from a `Draw` what changed. It keeps the core's last frame
//! and compares: a frame drawn again with nothing changed converts and
//! writes nothing, and one that moves a caret converts and writes the
//! rows between the first and the last that differ. Comparing is a
//! `memcmp`; converting is a loop over every pixel, several times slower
//! on the Pi.
//!
//! A frame the core says moved a region ([`Presenter::present_moved`],
//! `docs/FRAMES.md` §3.7) is compared after the kept frame and its
//! converted copy have moved the same rows, so only the strip and the
//! overlays are converted; the panel still writes every row of the view.
//!
//! A frame turned onto the panel (`docs/ORIENTATION.md` §3) is compared
//! the same way, by the core's rows; they are the panel's rows at 0 and
//! 180, and a band of its columns at 90 and 270, written row by row
//! where the band changed.
//!
//! A panel that takes the core's own bytes ([`Depth::Rgba8888`], the
//! Pi's, `docs/FRAMES.md` §5) at no turn needs none of that: the frame
//! goes to the panel, which compares it with its kept copy and writes
//! what differs, and nothing is converted or kept here.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use faraday_shell_api::Moved;

use crate::fb::{self, Depth, Geometry};
use crate::orient::Turn;
use crate::panel::{CHUNK_ROWS, Panel, Sink, lock};
use crate::refresh::{Beat, Vblank};
use crate::timings::{Bucket, Timings};
use crate::writer::{Done, Writer};

/// What putting frames on the panel has cost, in rows: the counts the
/// budget test reads.
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Rows of the core's frame converted to the panel's format.
    pub converted: u64,
    /// Rows written to the framebuffer, the arrow's included.
    pub written: u64,
}

/// The core's last frame and its converted copy, kept between frames.
#[derive(Default)]
pub struct Presenter {
    /// The core's frame as last put out (premultiplied RGBA), to compare
    /// the next with. Empty before the first.
    last: Vec<u8>,
    /// `last` converted to the panel's format, turned onto the panel. The
    /// cursor is drawn into the panel's band, never here.
    converted: Vec<u8>,
    /// The turn `converted` was made at.
    turn: Option<Turn>,
    /// Rows converted, ever.
    rows_converted: u64,
    /// The thread that writes the panel, when the frame goes out as it
    /// is ([`Presenter::write_on_thread`]).
    writer: Option<Writer>,
}

impl Presenter {
    /// Costs a comparison of `rgba` with the last frame, then a
    /// conversion and a write of the rows that differ (all of them when
    /// the panel is stale or the size changed).
    ///
    /// `rgba` is the core's whole frame, `g`'s size turned by `turn`
    /// ([`Turn::frame_size`]). The rows go out in bands of
    /// [`CHUNK_ROWS`], each under its own hold of the panel's lock, so a
    /// mouse move from an input thread slips in between two bands and
    /// the arrow never waits for a frame.
    #[cfg(test)]
    pub fn present<S: Sink>(
        &mut self,
        rgba: &[u8],
        g: Geometry,
        turn: Turn,
        panel: &Mutex<Panel<S>>,
        timings: Option<&mut Timings>,
    ) -> std::io::Result<()> {
        self.present_moved(rgba, None, g, turn, panel, timings)
    }

    /// From now on a frame that goes out as it is ([`Depth::Rgba8888`],
    /// no turn) is written by a thread of its own while the loop goes on,
    /// on a machine with more than one core (`docs/FRAMES.md` §6). The
    /// thread writes `panel`, and only a call for `panel` uses it.
    pub fn write_on_thread<S: Sink + Send + 'static>(
        &mut self,
        panel: &Arc<Mutex<Panel<S>>>,
        vblank: Option<Box<dyn Vblank>>,
    ) {
        self.writer = Writer::start(panel, vblank);
    }

    /// The refresh the writer thread waits for, when it does.
    pub fn beat(&self) -> Option<Arc<Beat>> {
        self.writer.as_ref().and_then(Writer::beat)
    }

    /// Costs the wait for the write that is out: the panel shows the last
    /// frame when it returns. How that write came to, for the caller to
    /// say. Called before the panel is blanked or let go.
    pub fn settle(&mut self, timings: Option<&mut Timings>) -> std::io::Result<()> {
        self.collect(timings)
    }

    /// Waits for the write that is out and tallies it: the wait is the
    /// loop's (`wait`), the write's own time is beside it (`write`).
    fn collect(&mut self, timings: Option<&mut Timings>) -> std::io::Result<()> {
        let Some(writer) = self.writer.as_mut().filter(|w| w.busy()) else {
            return Ok(());
        };
        let start = timings.is_some().then(Instant::now);
        let Done { took, result } = writer.wait().expect("a write was out");
        if let (Some(start), Some(t)) = (start, timings) {
            t.add(Bucket::Wait, start.elapsed());
            t.add_beside(Bucket::Write, took);
        }
        result
    }

    /// Forgets the kept frame: the core gave a frame the shell did not
    /// put out, so the next `moved` is relative to a frame the presenter
    /// never saw, and the next frame is taken whole.
    pub fn skipped(&mut self) {
        self.last.clear();
    }

    /// Costs what [`Presenter::present`] costs, for a frame the core says
    /// moved a region (`docs/FRAMES.md` §3.7). At no turn or a half turn on the
    /// converting path the rows of the kept frame and of the converted
    /// copy are moved inside the region's columns first, in ordinary
    /// memory, so the comparison finds only the strip, the overlays and
    /// what is outside the region, and only those are converted. A
    /// `moved` that does not fit the kept frame (another size, a quarter
    /// turn, nothing kept) is ignored.
    pub fn present_moved<S: Sink>(
        &mut self,
        rgba: &[u8],
        moved: Option<Moved>,
        g: Geometry,
        turn: Turn,
        panel: &Mutex<Panel<S>>,
        mut timings: Option<&mut Timings>,
    ) -> std::io::Result<()> {
        let panel_size = (g.width, g.height);
        let (fw, fh) = turn.frame_size(panel_size);
        let (rows, frame_rows) = (usize::from(g.height), usize::from(fh));
        if rows == 0 || frame_rows == 0 {
            self.skipped();
            return Ok(());
        }
        if g.depth == Depth::Rgba8888 && turn == Turn::R0 {
            return self.present_unconverted(rgba, g, panel, timings);
        }
        let src_row = usize::from(fw) * 4;
        let out_row = g.row_bytes();
        let start = timings.is_some().then(Instant::now);
        let kept = self.last.len() == rgba.len()
            && self.converted.len() == rows * out_row
            && self.turn == Some(turn);
        let view = match (kept, moved) {
            (true, Some(m)) => self.move_kept(m, (fw, fh), g, turn),
            _ => None,
        };
        let changed = if !kept {
            self.last.clear();
            self.last.resize(rgba.len(), 0);
            self.converted.clear();
            self.converted.resize(rows * out_row, 0);
            self.turn = Some(turn);
            vec![(0, frame_rows - 1)]
        } else {
            fb::changed_rows(&self.last, rgba, src_row)
        };
        for &(a, b) in &changed {
            fb::convert_turned(rgba, (a, b), g.depth, turn, panel_size, &mut self.converted);
            let src = a * src_row..(b + 1) * src_row;
            self.last[src.clone()].copy_from_slice(&rgba[src]);
            self.rows_converted += (b + 1 - a) as u64;
        }
        if let (Some(start), Some(t)) = (start, timings.as_deref_mut()) {
            t.add(Bucket::Convert, start.elapsed());
        }

        // The panel's own rows of the moved view all changed, though
        // none of them was converted again: each shows another row.
        let changed = match view {
            Some(v) => fb::merge_bands([changed, vec![v]].concat()),
            None => changed,
        };

        // Where the frame's changed bands are on the panel: its rows, or
        // (turned a quarter) one band of its columns down every row,
        // from the first changed column to the last. A stale panel shows
        // nothing known: the whole frame goes out, changed or not.
        let (w, h) = (usize::from(g.width), rows);
        let (bands, columns): (Vec<(usize, usize)>, _) = if lock(panel).stale() {
            (vec![(0, h - 1)], None)
        } else if let (Some(&(first, _)), Some(&(_, last))) = (changed.first(), changed.last()) {
            match turn {
                Turn::R0 => (changed.clone(), None),
                Turn::R180 => (
                    fb::merge_bands(
                        changed
                            .iter()
                            .map(|&(a, b)| (h - 1 - b, h - 1 - a))
                            .collect(),
                    ),
                    None,
                ),
                Turn::R90 => (vec![(0, h - 1)], Some((w - 1 - last, w - 1 - first))),
                Turn::R270 => (vec![(0, h - 1)], Some((first, last))),
            }
        } else {
            return Ok(());
        };
        let start = timings.is_some().then(Instant::now);
        let mut result = Ok(());
        'bands: for (a, b) in bands {
            let mut first = a;
            while first <= b {
                let last = (first + CHUNK_ROWS - 1).min(b);
                let mut p = lock(panel);
                result = match columns {
                    None => p.write_known(&self.converted, first, last),
                    Some(cols) => p.write_columns(&self.converted, cols, first, last),
                };
                drop(p);
                if result.is_err() {
                    break 'bands;
                }
                first = last + 1;
            }
        }
        if result.is_ok() {
            lock(panel).complete();
        }
        if let (Some(start), Some(t)) = (start, timings) {
            t.add(Bucket::Write, start.elapsed());
        }
        result
    }

    /// Moves the rows of `last` and of `converted` as the core moved the
    /// region `m` of a frame `(fw, fh)` pixels. A quarter turn moves
    /// columns of the panel instead: nothing moves and the frame is
    /// taken whole, as without `moved`. A region not inside the frame
    /// moves nothing. Gives the frame's rows of the region that moved.
    fn move_kept(
        &mut self,
        m: Moved,
        (fw, fh): (u16, u16),
        g: Geometry,
        turn: Turn,
    ) -> Option<(usize, usize)> {
        let (x, y, w, h) = (
            usize::from(m.x),
            usize::from(m.y),
            usize::from(m.width),
            usize::from(m.height),
        );
        let dy = isize::from(m.dy);
        let (fw, fh) = (usize::from(fw), usize::from(fh));
        if dy == 0 || w == 0 || x + w > fw || y + h > fh || dy.unsigned_abs() >= h {
            return None;
        }
        let (stride, bpp) = (g.row_bytes(), g.depth.bytes_per_pixel());
        match turn {
            Turn::R0 => move_rows(&mut self.converted, stride, bpp, (x, w), (y, h), dy),
            // The panel is the frame turned half over.
            Turn::R180 => move_rows(
                &mut self.converted,
                stride,
                bpp,
                (fw - x - w, w),
                (fh - y - h, h),
                -dy,
            ),
            Turn::R90 | Turn::R270 => return None,
        }
        move_rows(&mut self.last, fw * 4, 4, (x, w), (y, h), dy);
        Some((y, y + h - 1))
    }

    /// Costs a comparison of `rgba` with what the panel shows and a write
    /// of the rows that differ: a panel that takes the core's own bytes
    /// ([`Depth::Rgba8888`]) at no turn needs no converted copy and no
    /// last frame, so `rgba` goes to [`Panel::write_rows`] in bands of
    /// [`CHUNK_ROWS`], each under its own hold of the lock, and the
    /// panel's kept copy is the only one. The time counts as the write.
    ///
    /// What the presenter kept for a turned frame is dropped, so a turn
    /// back starts from a whole frame, as the panel (marked stale by
    /// [`Panel::set_turn`]) expects.
    fn present_unconverted<S: Sink>(
        &mut self,
        rgba: &[u8],
        g: Geometry,
        panel: &Mutex<Panel<S>>,
        timings: Option<&mut Timings>,
    ) -> std::io::Result<()> {
        self.last = Vec::new();
        self.converted = Vec::new();
        self.turn = None;
        if self.writer.as_ref().is_some_and(|w| w.is_for(panel)) {
            return self.present_staged(rgba, g, panel, timings);
        }
        let start = timings.is_some().then(Instant::now);
        let rows = usize::from(g.height);
        let mut result = Ok(());
        let mut first = 0;
        while first < rows {
            let last = (first + CHUNK_ROWS - 1).min(rows - 1);
            result = lock(panel).write_rows(rgba, first, last);
            if result.is_err() {
                break;
            }
            first = last + 1;
        }
        if result.is_ok() {
            lock(panel).complete();
        }
        if let (Some(start), Some(t)) = (start, timings) {
            t.add(Bucket::Write, start.elapsed());
        }
        result
    }

    /// Costs a comparison of `rgba` with what the panel shows, a copy of
    /// the rows that differ into the panel's kept copy, and the wait for
    /// the write before; the write of those rows is the writer thread's,
    /// while the loop goes on. A frame with no row changed waits for
    /// nothing. The error of the write before, if it failed, is this
    /// call's: it comes a frame late, and the failure has left the panel
    /// stale, so this frame goes out whole.
    fn present_staged<S: Sink>(
        &mut self,
        rgba: &[u8],
        g: Geometry,
        panel: &Mutex<Panel<S>>,
        mut timings: Option<&mut Timings>,
    ) -> std::io::Result<()> {
        let start = timings.is_some().then(Instant::now);
        let rows = usize::from(g.height);
        let whole = vec![(0, rows - 1)];
        // Compared while the write before may still be out: both only
        // read the panel's kept copy.
        let mut bands = Vec::new();
        if lock(panel).takes_whole(rgba.len()) {
            bands.clone_from(&whole);
        } else {
            let mut first = 0;
            while first < rows {
                let last = (first + CHUNK_ROWS - 1).min(rows - 1);
                lock(panel).changed_in(rgba, first, last, &mut bands);
                first = last + 1;
            }
        }
        if bands.is_empty() {
            if let (Some(start), Some(t)) = (start, timings) {
                t.add(Bucket::Stage, start.elapsed());
            }
            return Ok(());
        }
        let compared = start.map(|s| s.elapsed());
        let before = self.collect(timings.as_deref_mut());
        // A write that failed has left the panel stale.
        if lock(panel).takes_whole(rgba.len()) {
            bands.clone_from(&whole);
        }
        let start = timings.is_some().then(Instant::now);
        for &(a, b) in &bands {
            let mut first = a;
            while first <= b {
                let last = (first + CHUNK_ROWS - 1).min(b);
                lock(panel).keep_rows(rgba, first, last);
                first = last + 1;
            }
        }
        if let (Some(compared), Some(start), Some(t)) = (compared, start, timings) {
            t.add(Bucket::Stage, compared + start.elapsed());
        }
        if let Some(w) = self.writer.as_mut() {
            w.send(bands, true);
        }
        before
    }

    /// What has been converted, and what `panel` has written.
    #[cfg(test)]
    pub fn counts<S: Sink>(&self, panel: &Mutex<Panel<S>>) -> Counts {
        Counts {
            converted: self.rows_converted,
            written: lock(panel).rows_written(),
        }
    }
}

/// Moves the pixels of columns `(x, w)` of rows `(y, h)` of `buf` (rows
/// of `stride` bytes, `bpp` bytes a pixel) down by `dy` rows, up for a
/// negative `dy`, inside those rows; the rows the move brings into view
/// keep what they held.
fn move_rows(
    buf: &mut [u8],
    stride: usize,
    bpp: usize,
    (x, w): (usize, usize),
    (y, h): (usize, usize),
    dy: isize,
) {
    let by = dy.unsigned_abs();
    let (c0, c1) = (x * bpp, (x + w) * bpp);
    for i in 0..h - by {
        // Read a row before it is written: the far end first.
        let (from, to) = if dy > 0 {
            let from = y + h - by - 1 - i;
            (from, from + by)
        } else {
            let from = y + by + i;
            (from, from - by)
        };
        buf.copy_within(from * stride + c0..from * stride + c1, to * stride + c0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_core::Faraday;
    use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware};

    /// The Pi's panel.
    const G: Geometry = Geometry {
        width: 480,
        height: 640,
        depth: Depth::Rgb565,
        stride: 960,
    };

    /// The Pi's panel at `dpi`.
    fn display(dpi: u16) -> Event {
        Event::Display(DisplayInfo {
            width: G.width,
            height: G.height,
            dpi,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        })
    }

    /// A core on the Pi's panel.
    fn core() -> Faraday {
        let mut app = Faraday::new();
        app.event(display(160));
        app
    }

    /// Delivers `event`, then puts out a frame if the core asked for one,
    /// as the shell's loop does: whether it asked.
    fn pass(
        app: &mut Faraday,
        event: Option<Event>,
        p: &mut Presenter,
        panel: &Mutex<Panel<Vec<u8>>>,
    ) -> bool {
        if let Some(e) = event {
            app.event(e);
        }
        let mut draw = false;
        while let Some(c) = app.poll_command() {
            draw |= matches!(c, Command::Draw);
        }
        if draw {
            let frame = app.frame();
            p.present(frame.rgba, G, Turn::R0, panel, None)
                .expect("written");
        }
        draw
    }

    fn since(now: Counts, before: Counts) -> Counts {
        Counts {
            converted: now.converted - before.converted,
            written: now.written - before.written,
        }
    }

    #[test]
    fn the_panel_is_written_only_where_the_frame_changed() {
        let panel = Mutex::new(Panel::new(Vec::new(), G));
        let mut p = Presenter::default();
        let mut app = core();
        let rows = u64::from(G.height);

        // The first frame: every row converted and written, once.
        assert!(pass(&mut app, None, &mut p, &panel));
        assert_eq!(
            p.counts(&panel),
            Counts {
                converted: rows,
                written: rows
            }
        );
        // Let whatever fades in at the start finish.
        let mut now = 0;
        for _ in 0..240 {
            now += 16;
            pass(&mut app, Some(Event::Tick { now_ms: now }), &mut p, &panel);
        }

        // At rest: ticks put out nothing.
        let rest = p.counts(&panel);
        for _ in 0..120 {
            now += 16;
            pass(&mut app, Some(Event::Tick { now_ms: now }), &mut p, &panel);
        }
        assert_eq!(since(p.counts(&panel), rest), Counts::default(), "idle");

        // The same frame drawn again: the core paints it whole, the
        // panel gets nothing.
        let before = p.counts(&panel);
        for _ in 0..2 {
            let _ = app.drawn_texts();
            let frame = app.frame();
            p.present(frame.rgba, G, Turn::R0, &panel, None)
                .expect("written");
        }
        assert_eq!(
            since(p.counts(&panel), before),
            Counts::default(),
            "a frame drawn again"
        );

        // A frame that changes (the same panel told a denser grain, so
        // every line is laid out again), and whatever it moves after: each frame
        // converts and writes at most one whole frame, and the panel
        // shows the last.
        let start = p.counts(&panel);
        let mut event = Some(display(200));
        for _ in 0..120 {
            let before = p.counts(&panel);
            now += 16;
            let e = event.take().unwrap_or(Event::Tick { now_ms: now });
            pass(&mut app, Some(e), &mut p, &panel);
            let spent = since(p.counts(&panel), before);
            assert!(
                spent.converted <= rows && spent.written <= rows,
                "{spent:?}"
            );
        }
        assert!(
            since(p.counts(&panel), start).converted > 0,
            "the frame changed"
        );
        let frame = app.frame();
        let mut want = vec![0u8; G.row_bytes() * usize::from(G.height)];
        fb::convert_into(frame.rgba, G.depth, &mut want);
        assert_eq!(*lock(&panel).sink(), want, "the panel shows the frame");

        // A blanked panel takes the next frame whole, even an unchanged
        // one.
        lock(&panel).blank();
        let before = p.counts(&panel);
        let frame = app.frame();
        p.present(frame.rgba, G, Turn::R0, &panel, None)
            .expect("written");
        assert_eq!(
            since(p.counts(&panel), before),
            Counts {
                converted: 0,
                written: rows
            }
        );
        assert_eq!(*lock(&panel).sink(), want);
    }

    /// A frame of one grey, `turn`'s size.
    fn grey(turn: Turn, v: u8) -> Vec<u8> {
        let (fw, fh) = turn.frame_size((G.width, G.height));
        vec![v; usize::from(fw) * usize::from(fh) * 4]
    }

    /// Sets the pixel at column `x` of row `y` of `frame` (a frame `turn`
    /// wide) to `v`.
    fn dot(frame: &mut [u8], turn: Turn, x: usize, y: usize, v: u8) {
        let (fw, _) = turn.frame_size((G.width, G.height));
        let at = (y * usize::from(fw) + x) * 4;
        frame[at..at + 3].fill(v);
    }

    /// What the panel should hold for `frame`, with the arrow at `arrow`.
    fn want(frame: &[u8], turn: Turn, arrow: Option<(u16, u16)>) -> Vec<u8> {
        let panel = (G.width, G.height);
        let mut out = vec![0u8; G.row_bytes() * usize::from(G.height)];
        let (_, fh) = turn.frame_size(panel);
        fb::convert_turned(
            frame,
            (0, usize::from(fh) - 1),
            G.depth,
            turn,
            panel,
            &mut out,
        );
        if let Some(at) = arrow {
            crate::cursor::draw_rows(&mut out, G, turn, at, 0);
        }
        out
    }

    fn put(p: &mut Presenter, frame: &[u8], turn: Turn, panel: &Mutex<Panel<Vec<u8>>>) -> Counts {
        let before = p.counts(panel);
        p.present(frame, G, turn, panel, None).expect("written");
        since(p.counts(panel), before)
    }

    #[test]
    fn a_caret_and_a_clock_are_two_writes() {
        let panel = Mutex::new(Panel::new(Vec::new(), G));
        let mut p = Presenter::default();
        let mut frame = grey(Turn::R0, 40);
        put(&mut p, &frame, Turn::R0, &panel);
        dot(&mut frame, Turn::R0, 100, 20, 255);
        dot(&mut frame, Turn::R0, 300, 600, 255);
        let spent = put(&mut p, &frame, Turn::R0, &panel);
        assert_eq!(
            spent,
            Counts {
                converted: 2,
                written: 2
            }
        );
        assert_eq!(*lock(&panel).sink(), want(&frame, Turn::R0, None));
    }

    #[test]
    fn the_panel_shows_the_frame_with_the_arrow_after_every_change() {
        let panel = Mutex::new(Panel::new(Vec::new(), G));
        let mut p = Presenter::default();
        let mut frame = grey(Turn::R0, 40);
        let shows = |frame: &[u8], arrow| {
            assert_eq!(*lock(&panel).sink(), want(frame, Turn::R0, arrow));
        };
        // The whole frame, then the arrow lands in it.
        put(&mut p, &frame, Turn::R0, &panel);
        shows(&frame, None);
        lock(&panel).move_arrow(Some((200, 300))).expect("moved");
        shows(&frame, Some((200, 300)));
        // A small change away from the arrow, and one under it.
        dot(&mut frame, Turn::R0, 10, 50, 255);
        let spent = put(&mut p, &frame, Turn::R0, &panel);
        assert_eq!(spent.written, 1);
        shows(&frame, Some((200, 300)));
        dot(&mut frame, Turn::R0, 205, 305, 200);
        put(&mut p, &frame, Turn::R0, &panel);
        shows(&frame, Some((200, 300)));
        // A change that spans the arrow's band and the rows around it.
        for y in 280..330 {
            dot(&mut frame, Turn::R0, 0, y, 90);
        }
        put(&mut p, &frame, Turn::R0, &panel);
        shows(&frame, Some((200, 300)));
        // The arrow moves away: no stale arrow stays.
        lock(&panel).move_arrow(Some((20, 500))).expect("moved");
        shows(&frame, Some((20, 500)));
        lock(&panel).move_arrow(None).expect("moved");
        shows(&frame, None);
        // The same frame again writes nothing.
        assert_eq!(put(&mut p, &frame, Turn::R0, &panel), Counts::default());
    }

    #[test]
    fn a_turned_frame_shows_as_it_did() {
        for turn in [Turn::R90, Turn::R180, Turn::R270] {
            let panel = Mutex::new(Panel::new(Vec::new(), G));
            lock(&panel).set_turn(turn);
            let mut p = Presenter::default();
            let mut frame = grey(turn, 40);
            put(&mut p, &frame, turn, &panel);
            assert_eq!(*lock(&panel).sink(), want(&frame, turn, None), "{turn:?}");
            dot(&mut frame, turn, 5, 7, 255);
            dot(&mut frame, turn, 100, 200, 255);
            put(&mut p, &frame, turn, &panel);
            assert_eq!(*lock(&panel).sink(), want(&frame, turn, None), "{turn:?}");
            lock(&panel).move_arrow(Some((60, 90))).expect("moved");
            dot(&mut frame, turn, 70, 95, 120);
            put(&mut p, &frame, turn, &panel);
            assert_eq!(
                *lock(&panel).sink(),
                want(&frame, turn, Some((60, 90))),
                "{turn:?}"
            );
        }
    }

    /// The Pi's panel with the framebuffer the kernel patch gives it.
    const G32: Geometry = Geometry {
        width: 480,
        height: 640,
        depth: Depth::Rgba8888,
        stride: 1920,
    };

    /// An opaque frame of one grey, `turn`'s size.
    fn opaque(turn: Turn, v: u8) -> Vec<u8> {
        let mut frame = grey(turn, v);
        frame.chunks_exact_mut(4).for_each(|p| p[3] = 255);
        frame
    }

    #[test]
    fn a_red_first_panel_holds_the_frame_as_it_is_and_nothing_is_converted() {
        let panel = Mutex::new(Panel::new(Vec::new(), G32));
        let mut p = Presenter::default();
        let rows = u64::from(G32.height);
        let present = |p: &mut Presenter, frame: &[u8], turn: Turn| {
            let before = p.counts(&panel);
            p.present(frame, G32, turn, &panel, None).expect("written");
            since(p.counts(&panel), before)
        };
        let shown = || lock(&panel).sink().clone();

        // A whole frame, in colours that tell red from blue.
        let mut frame = opaque(Turn::R0, 40);
        frame
            .chunks_exact_mut(4)
            .for_each(|p| p[..3].copy_from_slice(&[200, 100, 10]));
        let spent = present(&mut p, &frame, Turn::R0);
        assert_eq!(
            spent,
            Counts {
                converted: 0,
                written: rows
            }
        );
        assert_eq!(shown(), frame);

        // The same frame again writes nothing; a caret writes its row.
        assert_eq!(present(&mut p, &frame, Turn::R0), Counts::default());
        dot(&mut frame, Turn::R0, 100, 20, 255);
        let spent = present(&mut p, &frame, Turn::R0);
        assert_eq!(
            spent,
            Counts {
                converted: 0,
                written: 1
            }
        );
        assert_eq!(shown(), frame);

        // The arrow across a band of changes, and moved away.
        lock(&panel).move_arrow(Some((200, 300))).expect("moved");
        for y in 280..330 {
            dot(&mut frame, Turn::R0, 0, y, 90);
        }
        present(&mut p, &frame, Turn::R0);
        let mut with_arrow = frame.clone();
        crate::cursor::draw_rows(&mut with_arrow, G32, Turn::R0, (200, 300), 0);
        assert_eq!(shown(), with_arrow);
        lock(&panel).move_arrow(None).expect("moved");
        assert_eq!(shown(), frame);

        // A turn and back: each shows its frame whole.
        lock(&panel).set_turn(Turn::R90);
        let mut turned = opaque(Turn::R90, 60);
        dot(&mut turned, Turn::R90, 5, 7, 255);
        present(&mut p, &turned, Turn::R90);
        let mut out = vec![0u8; G32.row_bytes() * usize::from(G32.height)];
        let (_, fh) = Turn::R90.frame_size((G32.width, G32.height));
        fb::convert_turned(
            &turned,
            (0, usize::from(fh) - 1),
            G32.depth,
            Turn::R90,
            (G32.width, G32.height),
            &mut out,
        );
        assert_eq!(shown(), out);
        lock(&panel).set_turn(Turn::R0);
        let spent = present(&mut p, &frame, Turn::R0);
        assert_eq!(
            spent,
            Counts {
                converted: 0,
                written: rows
            }
        );
        assert_eq!(shown(), frame);
        lock(&panel).set_turn(Turn::R90);
        present(&mut p, &turned, Turn::R90);
        assert_eq!(shown(), out);
    }

    #[test]
    fn a_red_first_panel_shows_the_frame_after_a_blanking_and_in_a_window() {
        let mut frame = opaque(Turn::R0, 40);
        frame
            .chunks_exact_mut(4)
            .for_each(|p| p[..3].copy_from_slice(&[200, 100, 10]));
        let rows = u64::from(G32.height);

        // A blanked panel takes the same frame whole.
        let panel = Mutex::new(Panel::new(Vec::new(), G32));
        let mut p = Presenter::default();
        p.present(&frame, G32, Turn::R0, &panel, None)
            .expect("written");
        lock(&panel).blank();
        assert!(lock(&panel).sink().iter().all(|&b| b == 0));
        let before = p.counts(&panel);
        p.present(&frame, G32, Turn::R0, &panel, None)
            .expect("written");
        assert_eq!(since(p.counts(&panel), before).written, rows);
        assert_eq!(*lock(&panel).sink(), frame);

        // In a window of a larger screen: the frame is inside, the rest
        // of the screen black.
        let screen = Geometry {
            width: 600,
            height: 700,
            stride: 2400,
            ..G32
        };
        let (inside, at) = screen.window((G32.width, G32.height)).expect("fits");
        let panel = Mutex::new(Panel::new(Vec::new(), inside));
        lock(&panel).window_in(screen, at);
        let mut p = Presenter::default();
        p.present(&frame, inside, Turn::R0, &panel, None)
            .expect("written");
        lock(&panel).move_arrow(Some((200, 300))).expect("moved");
        lock(&panel).move_arrow(None).expect("moved");
        let shown = lock(&panel).sink().clone();
        let (x0, y0) = (usize::from(at.0), usize::from(at.1));
        for y in 0..usize::from(screen.height) {
            for x in 0..usize::from(screen.width) {
                let i = y * screen.stride + x * 4;
                let inside = (x0..x0 + 480).contains(&x) && (y0..y0 + 640).contains(&y);
                let want = if inside {
                    let f = ((y - y0) * 480 + x - x0) * 4;
                    &frame[f..f + 4]
                } else {
                    &[0u8; 4][..]
                };
                assert_eq!(&shown[i..i + 4], want, "pixel {x},{y}");
            }
        }
    }

    const SAID: &str = "faraday-timings.txt written";

    /// What a panel `g` should hold for an opaque grey frame at `turn`
    /// with the line over it (and the arrow on that) if asked.
    fn lined(
        g: Geometry,
        turn: Turn,
        frame: &[u8],
        line: bool,
        arrow: Option<(u16, u16)>,
    ) -> Vec<u8> {
        let mut out = vec![0u8; g.row_bytes() * usize::from(g.height)];
        let (_, fh) = turn.frame_size((g.width, g.height));
        fb::convert_turned(
            frame,
            (0, usize::from(fh) - 1),
            g.depth,
            turn,
            (g.width, g.height),
            &mut out,
        );
        if line {
            crate::notice::draw_rows(&mut out, g, turn, SAID, 0);
        }
        if let Some(at) = arrow {
            crate::cursor::draw_rows(&mut out, g, turn, at, 0);
        }
        out
    }

    #[test]
    fn a_line_over_the_top_stays_through_frames_and_leaves_the_frame_as_it_was() {
        let g565 = Geometry { stride: 960, ..G };
        for (g, turn) in [
            (G32, Turn::R0),
            (g565, Turn::R0),
            (G32, Turn::R90),
            (g565, Turn::R270),
        ] {
            let panel = Mutex::new(Panel::new(Vec::new(), g));
            lock(&panel).set_turn(turn);
            let mut p = Presenter::default();
            let mut frame = opaque(turn, 40);
            let present = |p: &mut Presenter, frame: &[u8]| {
                p.present(frame, g, turn, &panel, None).expect("written");
            };
            let shown = || lock(&panel).sink().clone();
            let tag = format!("{:?} {turn:?}", g.depth);
            present(&mut p, &frame);
            let before = shown();
            assert_eq!(before, lined(g, turn, &frame, false, None), "{tag}");
            // The line goes up over the frame.
            lock(&panel).set_notice(Some(SAID.into())).expect("up");
            assert_eq!(shown(), lined(g, turn, &frame, true, None), "{tag}");
            assert_ne!(shown(), before, "{tag}");
            // A frame written meanwhile, under the line and away from it,
            // keeps it; so does the arrow on the same rows.
            dot(&mut frame, turn, 5, 3, 255);
            dot(&mut frame, turn, 100, 200, 255);
            present(&mut p, &frame);
            assert_eq!(shown(), lined(g, turn, &frame, true, None), "{tag}");
            lock(&panel).move_arrow(Some((10, 2))).expect("moved");
            assert_eq!(
                shown(),
                lined(g, turn, &frame, true, Some((10, 2))),
                "{tag}"
            );
            dot(&mut frame, turn, 50, 4, 90);
            present(&mut p, &frame);
            assert_eq!(
                shown(),
                lined(g, turn, &frame, true, Some((10, 2))),
                "{tag}"
            );
            lock(&panel).move_arrow(None).expect("moved");
            // It ends: the panel holds the frame exactly.
            lock(&panel).set_notice(None).expect("down");
            assert_eq!(shown(), lined(g, turn, &frame, false, None), "{tag}");
        }
    }

    /// A panel of the Pi's size in `depth`.
    fn panel_of(depth: Depth) -> Geometry {
        Geometry {
            depth,
            stride: usize::from(G.width) * depth.bytes_per_pixel(),
            ..G
        }
    }

    /// The scene the moved-frame tests move: a region from row 60 to 60
    /// short of the foot and 10 in from each side, whose content is the
    /// page `offset` rows down, every row of it different; the rest of
    /// the frame is one colour.
    struct Scene {
        turn: Turn,
        fw: usize,
        fh: usize,
    }

    impl Scene {
        fn new(turn: Turn) -> Scene {
            let (fw, fh) = turn.frame_size((G.width, G.height));
            Scene {
                turn,
                fw: usize::from(fw),
                fh: usize::from(fh),
            }
        }

        /// The region as the core reports it moved by `dy`.
        fn region(&self, dy: i16) -> Moved {
            Moved {
                x: 10,
                y: 60,
                width: (self.fw - 20) as u16,
                height: (self.fh - 120) as u16,
                dy,
            }
        }

        /// The frame with the page `offset` rows down, and a dot at
        /// the pixel `over` if given.
        fn frame(&self, offset: i32, over: Option<(usize, usize)>) -> Vec<u8> {
            let r = self.region(0);
            let (x0, x1) = (usize::from(r.x), usize::from(r.x + r.width));
            let (y0, y1) = (usize::from(r.y), usize::from(r.y + r.height));
            let mut out = vec![255u8; self.fw * self.fh * 4];
            for y in 0..self.fh {
                for x in 0..self.fw {
                    let v = if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                        let page = (y as i32 + offset).rem_euclid(100_000) as u32;
                        let h = page.wrapping_mul(2_654_435_761) ^ (x as u32).wrapping_mul(40_503);
                        [h as u8, (h >> 8) as u8, (h >> 16) as u8]
                    } else {
                        [30, 60, 90]
                    };
                    out[(y * self.fw + x) * 4..][..3].copy_from_slice(&v);
                }
            }
            if let Some((x, y)) = over {
                dot(&mut out, self.turn, x, y, 250);
            }
            out
        }
    }

    #[test]
    fn a_panel_after_moved_frames_holds_what_a_whole_redraw_gives() {
        for depth in [
            Depth::Rgb565,
            Depth::Bgr888,
            Depth::Bgra8888,
            Depth::Rgba8888,
        ] {
            for turn in [Turn::R0, Turn::R90, Turn::R180, Turn::R270] {
                let g = panel_of(depth);
                let scene = Scene::new(turn);
                let panel = Mutex::new(Panel::new(Vec::new(), g));
                lock(&panel).set_turn(turn);
                let mut p = Presenter::default();
                let tag = format!("{depth:?} {turn:?}");
                let (mut offset, mut line, mut arrow, mut over) = (0, false, None, None);
                // Each step: how far the page moved (two moves composed
                // are one), whether the take before it was skipped, and
                // what goes over the page.
                let steps: [(i16, bool, u8); 10] = [
                    (0, false, 0),
                    (8, false, 0),
                    (-8, false, 0),
                    (31, false, 0),
                    (-3, false, 0),
                    (5 + 7, false, 0),
                    (9, true, 0),
                    (-6, false, 1),
                    (12, false, 2),
                    (-20, false, 3),
                ];
                for (i, &(dy, skip, extra)) in steps.iter().enumerate() {
                    if skip {
                        p.skipped();
                    }
                    match extra {
                        1 => {
                            arrow = Some((200, 300));
                            lock(&panel).move_arrow(arrow).expect("moved");
                            over = Some((4, 3));
                        }
                        2 => {
                            line = true;
                            lock(&panel).set_notice(Some(SAID.into())).expect("up");
                        }
                        3 => {
                            arrow = None;
                            lock(&panel).move_arrow(None).expect("moved");
                            over = None;
                        }
                        _ => {}
                    }
                    offset -= i32::from(dy);
                    let frame = scene.frame(offset, over);
                    let moved = (i > 0).then(|| scene.region(dy));
                    p.present_moved(&frame, moved, g, turn, &panel, None)
                        .expect("written");
                    assert_eq!(
                        *lock(&panel).sink(),
                        lined(g, turn, &frame, line, arrow),
                        "{tag}, step {i}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_moved_scroll_step_converts_only_the_strip_and_what_changed() {
        let strip = 8u64;
        for depth in [Depth::Rgb565, Depth::Bgra8888] {
            for turn in [Turn::R0, Turn::R180] {
                let g = panel_of(depth);
                let scene = Scene::new(turn);
                let view_rows = u64::from(scene.region(0).height);
                let panel = Mutex::new(Panel::new(Vec::new(), g));
                lock(&panel).set_turn(turn);
                let mut p = Presenter::default();
                let tag = format!("{depth:?} {turn:?}");
                let first = scene.frame(0, None);
                p.present_moved(&first, None, g, turn, &panel, None)
                    .expect("written");

                // A step down, then up: the strip is converted, and the
                // whole view written, each of its rows showing another.
                let mut offset = 0;
                for dy in [8i16, -8] {
                    offset -= i32::from(dy);
                    let before = p.counts(&panel);
                    let frame = scene.frame(offset, None);
                    p.present_moved(&frame, Some(scene.region(dy)), g, turn, &panel, None)
                        .expect("written");
                    let spent = since(p.counts(&panel), before);
                    assert_eq!(spent.converted, strip, "{tag}, dy {dy}");
                    assert_eq!(spent.written, view_rows, "{tag}, dy {dy}");
                }

                // A dot in the head of the frame with the step: the
                // strip and that row, and not the rows between.
                offset -= 8;
                let before = p.counts(&panel);
                let frame = scene.frame(offset, Some((4, 3)));
                p.present_moved(&frame, Some(scene.region(8)), g, turn, &panel, None)
                    .expect("written");
                let spent = since(p.counts(&panel), before);
                assert_eq!(spent.converted, strip + 1, "{tag}: {spent:?}");
            }
        }
    }

    #[test]
    fn the_pis_panel_at_no_turn_converts_nothing_for_a_moved_frame() {
        let g = panel_of(Depth::Rgba8888);
        let scene = Scene::new(Turn::R0);
        let panel = Mutex::new(Panel::new(Vec::new(), g));
        let mut p = Presenter::default();
        let first = scene.frame(0, None);
        p.present_moved(&first, None, g, Turn::R0, &panel, None)
            .expect("written");
        let before = p.counts(&panel);
        let frame = scene.frame(-8, None);
        p.present_moved(&frame, Some(scene.region(8)), g, Turn::R0, &panel, None)
            .expect("written");
        let spent = since(p.counts(&panel), before);
        assert_eq!(spent.converted, 0);
        assert_eq!(spent.written, u64::from(scene.region(0).height));
        assert_eq!(*lock(&panel).sink(), frame);
    }

    #[test]
    fn a_page_scrolled_in_the_app_shows_what_a_whole_redraw_shows() {
        let panel = Mutex::new(Panel::new(Vec::new(), G));
        let mut p = Presenter::default();
        let mut app = core();
        app.press(faraday_core::Action::Nav(faraday_core::Screen::Settings));
        let take = |app: &mut Faraday, p: &mut Presenter, event: Event| {
            app.event(event);
            while app.poll_command().is_some() {}
            let parts = app.parts();
            let moved = parts.moved;
            p.present_moved(parts.base.rgba, moved, G, Turn::R0, &panel, None)
                .expect("written");
            moved
        };
        for now in (1..20).map(|i| i * 16) {
            take(&mut app, &mut p, Event::Tick { now_ms: now });
        }
        let mut moves = 0;
        for dy in [30, 30, 12, -20, -40, 7, 60, 60, -90] {
            let before = p.counts(&panel);
            let event = Event::Scroll { x: 240, y: 320, dy };
            if take(&mut app, &mut p, event).is_some() {
                moves += 1;
                let spent = since(p.counts(&panel), before);
                assert!(
                    spent.converted < u64::from(G.height) / 2,
                    "a moved step converts the strip: {spent:?}"
                );
            }
            let frame = app.frame();
            let mut want = vec![0u8; G.row_bytes() * usize::from(G.height)];
            fb::convert_into(frame.rgba, G.depth, &mut want);
            assert_eq!(*lock(&panel).sink(), want, "dy {dy}");
        }
        assert!(moves > 0, "Settings scrolled as moved frames");
    }
}
