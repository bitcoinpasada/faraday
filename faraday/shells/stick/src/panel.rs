//! The panel as the shell writes it: the framebuffer, a copy of what it
//! shows without the arrow, and where the arrow is (`docs/MOTION.md`
//! §3.2).
//!
//! One of these sits behind a lock shared by the main loop, which writes
//! frames, and the input threads, which move the arrow the moment a
//! mouse reports. A frame goes out in bands of [`CHUNK_ROWS`] rows, each
//! under its own hold of the lock, so a move slips in between two bands
//! and the arrow never waits for a frame. Whoever writes a band draws
//! the arrow into it where it falls, and whoever moves the arrow puts
//! back the rows it left from the kept copy, so the panel always shows
//! the frame with the arrow once, wherever the two threads are.

use std::fs::File;
use std::os::unix::fs::FileExt;
use std::sync::{Mutex, MutexGuard};

use crate::cursor;
use crate::fb::{self, Geometry};
use crate::orient::Turn;

/// Rows a frame is written in per hold of the lock: about half a
/// millisecond of writing on a 1080p panel.
pub const CHUNK_ROWS: usize = 64;

/// Takes the lock; a thread that panicked holding it does not make the
/// panel unusable for the others.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Where a panel's bytes go: the framebuffer device, or, in a test, a
/// buffer in memory.
pub trait Sink {
    /// Writes all of `buf` at byte `offset`.
    fn write_at(&mut self, buf: &[u8], offset: u64) -> std::io::Result<()>;
}

impl Sink for File {
    fn write_at(&mut self, buf: &[u8], offset: u64) -> std::io::Result<()> {
        self.write_all_at(buf, offset)
    }
}

/// A panel in memory, grown to whatever is written: what a test draws
/// on.
impl Sink for Vec<u8> {
    fn write_at(&mut self, buf: &[u8], offset: u64) -> std::io::Result<()> {
        let at = usize::try_from(offset).map_err(std::io::Error::other)?;
        if self.len() < at + buf.len() {
            self.resize(at + buf.len(), 0);
        }
        self[at..at + buf.len()].copy_from_slice(buf);
        Ok(())
    }
}

/// The framebuffer and what is on it.
pub struct Panel<S: Sink = File> {
    fb: S,
    geometry: Geometry,
    /// The converted frame the panel shows, without the arrow. Rows are
    /// kept in step with the panel as they are written.
    shown: Vec<u8>,
    /// `shown` does not yet describe the whole panel: before the first
    /// frame, after a blanking, and after a write that failed. Every row
    /// of the next frame is written, and the arrow waits for it.
    stale: bool,
    /// Where the arrow is, in the frame's pixels, if it is drawn.
    arrow: Option<(u16, u16)>,
    /// How the frame is turned onto the panel, which the arrow follows.
    turn: Turn,
    /// The band of rows being written, kept so a write allocates
    /// nothing.
    band: Vec<u8>,
    /// Rows written to the framebuffer, ever, the arrow's included: what
    /// the budget test counts (`present.rs`).
    written: u64,
}

impl<S: Sink> Panel<S> {
    /// A panel nothing has been drawn on yet.
    pub fn new(fb: S, geometry: Geometry) -> Panel<S> {
        Panel {
            fb,
            geometry,
            shown: Vec::new(),
            stale: true,
            arrow: None,
            turn: Turn::R0,
            band: Vec::new(),
            written: 0,
        }
    }

    /// Whether the next frame must be written whole: nothing is known to
    /// be on the panel (before the first frame, after a blanking or a
    /// failed write).
    pub fn stale(&self) -> bool {
        self.stale
    }

    /// The frame is turned `turn` from now on (`docs/ORIENTATION.md`
    /// §7): nothing on the panel is the next frame's, so it is written
    /// whole.
    pub fn set_turn(&mut self, turn: Turn) {
        if turn != self.turn {
            self.turn = turn;
            self.stale = true;
        }
    }

    /// Rows written to the framebuffer so far.
    #[cfg(test)]
    pub fn rows_written(&self) -> u64 {
        self.written
    }

    /// What the panel's framebuffer holds, for a test.
    #[cfg(test)]
    pub fn sink(&self) -> &S {
        &self.fb
    }

    /// Costs a comparison of the rows with what the panel shows and a
    /// write of those that differ: rows `first..=last` of `frame`, a
    /// whole converted frame, written where they differ from the panel,
    /// with the arrow drawn where it falls in them. The caller walks a
    /// frame in chunks of [`CHUNK_ROWS`], one call per hold of the lock,
    /// and calls [`Panel::complete`] after the last.
    pub fn write_rows(&mut self, frame: &[u8], first: usize, last: usize) -> std::io::Result<()> {
        let row = self.geometry.row_bytes();
        if self.shown.len() != frame.len() {
            self.shown.clear();
            self.shown.resize(frame.len(), 0);
            self.stale = true;
        }
        let span = if self.stale {
            Some((first, last))
        } else {
            let rows = first * row..(last + 1) * row;
            fb::changed_rows(&self.shown[rows.clone()], &frame[rows], row)
                .map(|(a, b)| (first + a, first + b))
        };
        let Some((a, b)) = span else {
            return Ok(());
        };
        let shown = std::mem::take(&mut self.shown);
        let result = self.write_band(frame, a, b);
        self.shown = shown;
        match result {
            Ok(()) => {
                self.shown[a * row..(b + 1) * row].copy_from_slice(&frame[a * row..(b + 1) * row]);
            }
            Err(_) => self.stale = true,
        }
        result
    }

    /// Costs a comparison of each row's band of columns with what the
    /// panel shows and a write of those that differ: columns
    /// `x0..=x1` of rows `first..=last` of `frame`, a whole converted
    /// frame, one `pwrite` a row whose band changed, with the arrow
    /// drawn where it falls in it. A frame turned a quarter changes a
    /// band of the panel's columns (`docs/ORIENTATION.md` §3). A stale
    /// panel takes whole rows instead, through [`Panel::write_rows`].
    pub fn write_columns(
        &mut self,
        frame: &[u8],
        (x0, x1): (usize, usize),
        first: usize,
        last: usize,
    ) -> std::io::Result<()> {
        if self.stale || self.shown.len() != frame.len() {
            return self.write_rows(frame, first, last);
        }
        let g = self.geometry;
        let row = g.row_bytes();
        let bytes = g.depth.bytes_per_pixel();
        let cols = x0 * bytes..(x1 + 1) * bytes;
        let arrow = self.arrow.map(|at| (at, cursor::rows(g, self.turn, at)));
        for y in first..=last {
            let band = y * row + cols.start..y * row + cols.end;
            if self.shown[band.clone()] == frame[band.clone()] {
                continue;
            }
            self.band.clear();
            match arrow {
                Some((at, (a, b))) if (a..=b).contains(&y) => {
                    self.band.extend_from_slice(&frame[y * row..(y + 1) * row]);
                    cursor::draw_rows(&mut self.band, g, self.turn, at, y);
                    self.band.drain(..cols.start);
                    self.band.truncate(cols.len());
                }
                _ => self.band.extend_from_slice(&frame[band.clone()]),
            }
            self.written += 1;
            if let Err(e) = self
                .fb
                .write_at(&self.band, (y * g.stride + cols.start) as u64)
            {
                self.stale = true;
                return Err(e);
            }
            self.shown[band.clone()].copy_from_slice(&frame[band]);
        }
        Ok(())
    }

    /// Every row of a frame has been written: the kept copy is the
    /// panel, and the arrow can move on its own again.
    pub fn complete(&mut self) {
        self.stale = false;
    }

    /// The arrow is at `at`, or gone: the rows it leaves and the rows it
    /// lands on are written again from the kept frame, now. Before a
    /// whole frame is on the panel only the place is noted; that frame
    /// draws it.
    pub fn move_arrow(&mut self, at: Option<(u16, u16)>) -> std::io::Result<()> {
        if at == self.arrow {
            return Ok(());
        }
        let was = self.arrow;
        self.arrow = at;
        if self.stale {
            return Ok(());
        }
        let g = self.geometry;
        let bands = fb::merge_bands(
            [was, at]
                .into_iter()
                .flatten()
                .map(|a| cursor::rows(g, self.turn, a))
                .collect(),
        );
        let shown = std::mem::take(&mut self.shown);
        let result = bands
            .into_iter()
            .try_for_each(|(first, last)| self.write_band(&shown, first, last));
        self.shown = shown;
        if result.is_err() {
            self.stale = true;
        }
        result
    }

    /// Paints the panel black: what was on it is gone before the power
    /// is. Writes every row.
    pub fn blank(&mut self) {
        let g = self.geometry;
        let row = vec![0u8; g.row_bytes()];
        for y in 0..usize::from(g.height) {
            let _ = self.fb.write_at(&row, (y * g.stride) as u64);
        }
        self.stale = true;
        self.arrow = None;
    }

    /// Rows `first..=last` of `frame` to the panel, the arrow drawn in
    /// where it falls: one `pwrite` when the rows are packed, one per row
    /// when the kernel pads them. `write_all_at` needs no seek and no
    /// `mmap`, so the whole path stays inside safe `std`.
    fn write_band(&mut self, frame: &[u8], first: usize, last: usize) -> std::io::Result<()> {
        let g = self.geometry;
        let row = g.row_bytes();
        self.band.clear();
        self.band
            .extend_from_slice(&frame[first * row..(last + 1) * row]);
        if let Some(at) = self.arrow {
            cursor::draw_rows(&mut self.band, g, self.turn, at, first);
        }
        self.written += (last + 1 - first) as u64;
        if g.stride == row {
            self.fb.write_at(&self.band, (first * row) as u64)
        } else {
            (first..=last).try_for_each(|y| {
                let r = (y - first) * row;
                self.fb
                    .write_at(&self.band[r..r + row], (y * g.stride) as u64)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fb::Depth;

    /// A panel that is a temporary file, read back to check.
    fn panel(tag: &str) -> (Panel<File>, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("faraday-panel-{}-{tag}", std::process::id()));
        let fb = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("a file for a panel");
        let geometry = Geometry {
            width: 32,
            height: 160,
            depth: Depth::Rgb565,
            stride: 64,
        };
        (Panel::new(fb, geometry), path)
    }

    fn frame(fill: u8) -> Vec<u8> {
        vec![fill; 64 * 160]
    }

    fn whole(panel: &mut Panel, frame: &[u8]) {
        let mut first = 0;
        while first < 160 {
            let last = (first + CHUNK_ROWS - 1).min(159);
            panel.write_rows(frame, first, last).expect("written");
            first = last + 1;
        }
        panel.complete();
    }

    #[test]
    fn the_arrow_moves_over_a_frame_and_leaves_it_as_it_was() {
        let (mut panel, path) = panel("arrow");
        let g = panel.geometry;
        let page = frame(0x33);
        whole(&mut panel, &page);
        let read = || std::fs::read(&path).expect("read back");
        assert_eq!(read(), page, "the frame, no arrow");
        // The arrow lands: its rows carry it, nothing else changes.
        panel.move_arrow(Some((4, 20))).expect("moved");
        let mut want = page.clone();
        cursor::draw_rows(&mut want, g, Turn::R0, (4, 20), 0);
        assert_eq!(read(), want);
        // It moves on: the rows it left are the frame again.
        panel.move_arrow(Some((10, 100))).expect("moved");
        let mut want = page.clone();
        cursor::draw_rows(&mut want, g, Turn::R0, (10, 100), 0);
        assert_eq!(read(), want);
        // A new frame arrives in chunks while the arrow stays: the arrow
        // is drawn into the chunk it falls in, once.
        let next = frame(0x77);
        whole(&mut panel, &next);
        let mut want = next.clone();
        cursor::draw_rows(&mut want, g, Turn::R0, (10, 100), 0);
        assert_eq!(read(), want);
        // And goes away: the frame alone.
        panel.move_arrow(None).expect("moved");
        assert_eq!(read(), next);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_chunk_that_did_not_change_is_not_written() {
        let (mut panel, path) = panel("chunks");
        let page = frame(0x33);
        whole(&mut panel, &page);
        // Scribble on the file behind the panel's back: a chunk written
        // again would put the frame back.
        let scribbled = {
            let mut s = std::fs::read(&path).expect("read back");
            s[0] = 0xff;
            s[64 * 150] = 0xff;
            std::fs::write(&path, &s).expect("scribble");
            s
        };
        let mut changed = page.clone();
        changed[64 * 80] = 0x11;
        whole(&mut panel, &changed);
        let mut want = scribbled.clone();
        want[64 * 80] = 0x11;
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            want,
            "only row 80 went out"
        );
        let _ = std::fs::remove_file(&path);
    }
}
