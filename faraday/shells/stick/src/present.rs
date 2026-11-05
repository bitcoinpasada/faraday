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
//! A frame turned onto the panel (`docs/ORIENTATION.md` §3) is compared
//! the same way, by the core's rows; they are the panel's rows at 0 and
//! 180, and a band of its columns at 90 and 270, written row by row
//! where the band changed.
//!
//! A panel that takes the core's own bytes ([`Depth::Rgba8888`], the
//! Pi's, `docs/FRAMES.md` §5) at no turn needs none of that: the frame
//! goes to the panel, which compares it with its kept copy and writes
//! what differs, and nothing is converted or kept here.

use std::sync::Mutex;
use std::time::Instant;

use crate::fb::{self, Depth, Geometry};
use crate::orient::Turn;
use crate::panel::{CHUNK_ROWS, Panel, Sink, lock};
use crate::timings::{Bucket, Timings};

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
    pub fn present<S: Sink>(
        &mut self,
        rgba: &[u8],
        g: Geometry,
        turn: Turn,
        panel: &Mutex<Panel<S>>,
        mut timings: Option<&mut Timings>,
    ) -> std::io::Result<()> {
        let panel_size = (g.width, g.height);
        let (fw, fh) = turn.frame_size(panel_size);
        let (rows, frame_rows) = (usize::from(g.height), usize::from(fh));
        if rows == 0 || frame_rows == 0 {
            return Ok(());
        }
        if g.depth == Depth::Rgba8888 && turn == Turn::R0 {
            return self.present_unconverted(rgba, g, panel, timings);
        }
        let src_row = usize::from(fw) * 4;
        let out_row = g.row_bytes();
        let start = timings.is_some().then(Instant::now);
        let changed = if self.last.len() != rgba.len()
            || self.converted.len() != rows * out_row
            || self.turn != Some(turn)
        {
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

    /// What has been converted, and what `panel` has written.
    #[cfg(test)]
    pub fn counts<S: Sink>(&self, panel: &Mutex<Panel<S>>) -> Counts {
        Counts {
            converted: self.rows_converted,
            written: lock(panel).rows_written(),
        }
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
}
