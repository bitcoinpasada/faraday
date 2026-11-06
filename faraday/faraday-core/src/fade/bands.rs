//! The frame painted in bands across the cores (`docs/FRAMES.md` §4).
//!
//! A frame is drawn once, on the main thread, into the canvas's list;
//! [`Bands::paint`] paints the list in bands of rows, one band on the main
//! thread and the others on threads that start with the frame and are
//! joined before it returns. A band panicking is the frame panicking:
//! every band is joined first, then the panic goes on (release builds
//! abort, as a panic on the main thread does; `docs/WORKERS.md` §2.1).

use faraday_draw::Canvas;

/// The most bands a frame is painted in.
const MOST: usize = 4;

/// The fewest rows of a band.
const ROWS: usize = 64;

/// How many bands a frame is painted in, and what a test asks of it.
#[derive(Debug)]
pub(crate) struct Bands {
    /// Bands for the machine; 1 paints as the calls are made.
    n: usize,
    /// A test asks for the calls recorded and painted from the list even
    /// at one band.
    #[cfg(test)]
    recorded: bool,
    /// The calls and barriers of the last frame painted from the list.
    #[cfg(test)]
    seen: (usize, usize),
}

impl Default for Bands {
    /// The bands the machine's cores allow.
    fn default() -> Self {
        let cores = std::thread::available_parallelism().map_or(1, usize::from);
        Self::with(cores.min(MOST))
    }
}

impl Bands {
    fn with(n: usize) -> Self {
        Bands {
            n: n.max(1),
            #[cfg(test)]
            recorded: false,
            #[cfg(test)]
            seen: (0, 0),
        }
    }

    /// Whether a whole frame is recorded into the list and painted from
    /// it.
    pub(crate) fn records(&self) -> bool {
        #[cfg(test)]
        if self.recorded {
            return true;
        }
        self.n > 1
    }

    /// How many bands a canvas of `rows` visible rows is painted in.
    fn bands_for(&self, rows: i32) -> usize {
        self.n.min(usize::try_from(rows).unwrap_or(0) / ROWS).max(1)
    }

    /// Costs the painting of every call waiting on `canvas`. Paints them
    /// in bands, the first on this thread.
    pub(crate) fn paint(&mut self, canvas: &mut Canvas) {
        let n = self.bands_for(canvas.height());
        #[cfg(test)]
        {
            let (calls, barriers) = canvas.waiting();
            self.seen = (
                calls,
                if n > 1 {
                    canvas.barriers_in(n)
                } else {
                    barriers
                },
            );
        }
        if n < 2 {
            canvas.replay();
            return;
        }
        canvas.replay_banded(n, &mut |bands| {
            let Some((mine, others)) = bands.split_first_mut() else {
                return;
            };
            std::thread::scope(|s| {
                for band in others {
                    s.spawn(|| band.paint());
                }
                mine.paint();
            });
        });
    }
}

#[cfg(test)]
impl Bands {
    /// `n` bands whatever the machine, `0` for none and no recording.
    pub(crate) fn set_for_test(&mut self, n: usize) {
        *self = Self::with(n);
        self.recorded = n > 0;
    }

    /// The calls, and the barriers for the bands used, of the last frame
    /// painted.
    pub(crate) fn seen(&self) -> (usize, usize) {
        self.seen
    }
}
