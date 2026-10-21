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

use std::sync::Mutex;
use std::time::Instant;

use crate::fb::{self, Geometry};
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
    /// `last` converted to the panel's format. The cursor is drawn into
    /// the panel's band, never here.
    converted: Vec<u8>,
    /// Rows converted, ever.
    rows_converted: u64,
}

impl Presenter {
    /// Costs a comparison of `rgba` with the last frame, then a
    /// conversion and a write of the rows that differ (all of them when
    /// the panel is stale or the size changed).
    ///
    /// `rgba` is the core's whole frame at `g`'s size. The rows go out in
    /// bands of [`CHUNK_ROWS`], each under its own hold of the panel's
    /// lock, so a mouse move from an input thread slips in between two
    /// bands and the arrow never waits for a frame.
    pub fn present<S: Sink>(
        &mut self,
        rgba: &[u8],
        g: Geometry,
        panel: &Mutex<Panel<S>>,
        mut timings: Option<&mut Timings>,
    ) -> std::io::Result<()> {
        let rows = usize::from(g.height);
        if rows == 0 {
            return Ok(());
        }
        let src_row = usize::from(g.width) * 4;
        let out_row = g.row_bytes();
        let start = timings.is_some().then(Instant::now);
        let changed = if self.last.len() != rgba.len() || self.converted.len() != rows * out_row {
            self.last.clear();
            self.last.resize(rgba.len(), 0);
            self.converted.clear();
            self.converted.resize(rows * out_row, 0);
            Some((0, rows - 1))
        } else {
            fb::changed_rows(&self.last, rgba, src_row)
        };
        if let Some((a, b)) = changed {
            let src = a * src_row..(b + 1) * src_row;
            fb::convert_into(
                &rgba[src.clone()],
                g.depth,
                &mut self.converted[a * out_row..(b + 1) * out_row],
            );
            self.last[src.clone()].copy_from_slice(&rgba[src]);
            self.rows_converted += (b + 1 - a) as u64;
        }
        if let (Some(start), Some(t)) = (start, timings.as_deref_mut()) {
            t.add(Bucket::Convert, start.elapsed());
        }

        // A stale panel shows nothing known: the whole frame goes out,
        // changed or not.
        let span = if lock(panel).stale() {
            Some((0, rows - 1))
        } else {
            changed
        };
        let Some((a, b)) = span else {
            return Ok(());
        };
        let start = timings.is_some().then(Instant::now);
        let mut result = Ok(());
        let mut first = a;
        while first <= b && result.is_ok() {
            let last = (first + CHUNK_ROWS - 1).min(b);
            result = lock(panel).write_rows(&self.converted, first, last);
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
    use crate::fb::Depth;
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
            p.present(frame.rgba, G, panel, None).expect("written");
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
            p.present(frame.rgba, G, &panel, None).expect("written");
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
        p.present(frame.rgba, G, &panel, None).expect("written");
        assert_eq!(
            since(p.counts(&panel), before),
            Counts {
                converted: 0,
                written: rows
            }
        );
        assert_eq!(*lock(&panel).sink(), want);
    }
}
