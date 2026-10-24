//! The desktop app's preview size (`docs/PREVIEW-SIZE.md`): the app laid
//! out as a screen of another size, in dp, drawn as a framed area centred
//! in the window and scaled down only where the window is smaller, with
//! the pointer's positions carried into it.

use faraday_shell_api::{Event, TouchPhase};

/// The space kept between the window's edges and the frame, in pixels at
/// scale 1.
pub const MARGIN: f64 = 24.0;

/// The frame's edge, a pixel wide around the area.
const EDGE: u32 = 0x003a_4652;

/// The display event for a screen `dp` in size, drawn at the window's
/// `scale`: its pixels are the window's, so the area is crisp where it
/// fits, and its density the window's, so the app lays it out at exactly
/// `dp` (before the person's Display scale). A lookup.
pub fn display((w, h): (u16, u16), scale: f64) -> Event {
    let mut e = crate::view::display((1, 1), scale, None);
    if let Event::Display(d) = &mut e {
        d.width = (f64::from(w) * scale)
            .round()
            .clamp(1.0, f64::from(u16::MAX)) as u16;
        d.height = (f64::from(h) * scale)
            .round()
            .clamp(1.0, f64::from(u16::MAX)) as u16;
    }
    e
}

/// Where a frame goes in the window: centred, at its own size where it
/// fits inside the margin, else scaled down to fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// The area's left edge in the window, pixels.
    pub x: i32,
    /// The area's top edge in the window, pixels.
    pub y: i32,
    /// The area's width in the window, pixels.
    pub w: u32,
    /// The area's height in the window, pixels.
    pub h: u32,
    /// The frame's own width, pixels.
    pub fw: u32,
    /// The frame's own height, pixels.
    pub fh: u32,
}

impl Fit {
    /// Where a frame `frame` pixels in size goes in a window `window`
    /// pixels in size at `scale`. A lookup.
    pub fn of(window: (u32, u32), frame: (u32, u32), scale: f64) -> Fit {
        let (ww, wh) = (f64::from(window.0), f64::from(window.1));
        let (fw, fh) = (f64::from(frame.0.max(1)), f64::from(frame.1.max(1)));
        let m = MARGIN * scale;
        let s = ((ww - 2.0 * m) / fw)
            .min((wh - 2.0 * m) / fh)
            .clamp(0.05, 1.0);
        let (w, h) = ((fw * s).round().max(1.0), (fh * s).round().max(1.0));
        Fit {
            x: ((ww - w) / 2.0).floor() as i32,
            y: ((wh - h) / 2.0).floor() as i32,
            w: w as u32,
            h: h as u32,
            fw: frame.0.max(1),
            fh: frame.1.max(1),
        }
    }

    /// The frame's pixel under the window's pixel (x, y), or `None`
    /// outside the area. A lookup.
    pub fn to_frame(&self, x: f64, y: f64) -> Option<(u16, u16)> {
        let (dx, dy) = (x - f64::from(self.x), y - f64::from(self.y));
        if dx < 0.0 || dy < 0.0 || dx >= f64::from(self.w) || dy >= f64::from(self.h) {
            return None;
        }
        Some(self.scaled(dx, dy))
    }

    /// The frame's pixel nearest the window's pixel (x, y): inside the
    /// area as [`Fit::to_frame`], else on its nearest edge. A lookup.
    pub fn to_frame_clamped(&self, x: f64, y: f64) -> (u16, u16) {
        let dx = (x - f64::from(self.x)).clamp(0.0, f64::from(self.w) - 1.0);
        let dy = (y - f64::from(self.y)).clamp(0.0, f64::from(self.h) - 1.0);
        self.scaled(dx, dy)
    }

    fn scaled(&self, dx: f64, dy: f64) -> (u16, u16) {
        let fx = dx * f64::from(self.fw) / f64::from(self.w);
        let fy = dy * f64::from(self.fh) / f64::from(self.h);
        (
            fx.min(f64::from(self.fw) - 1.0).clamp(0.0, 65535.0) as u16,
            fy.min(f64::from(self.fh) - 1.0).clamp(0.0, 65535.0) as u16,
        )
    }

    /// Costs a pass over the window's pixels and the frame's: copies the
    /// frame's premultiplied RGBA (`fw` × `fh`) into the window's 0RGB
    /// rows (`sw` wide), each window pixel the average of the frame's
    /// pixels it covers, the rest `beyond`, with a pixel-wide edge round
    /// the area.
    pub fn blit(&self, dst: &mut [u32], sw: usize, rgba: &[u8], beyond: u32) {
        let (fw, fh) = (self.fw as usize, self.fh as usize);
        if rgba.len() < fw * fh * 4 || sw == 0 {
            return;
        }
        let (ax, ay) = (i64::from(self.x), i64::from(self.y));
        let (aw, ah) = (i64::from(self.w), i64::from(self.h));
        // The frame's columns each window column covers, worked out once.
        let cols: Vec<(usize, usize)> = (0..self.w as usize)
            .map(|c| span(c, self.w as usize, fw))
            .collect();
        for (row_y, row) in dst.chunks_exact_mut(sw).enumerate() {
            let ry = row_y as i64 - ay;
            if ry < -1 || ry > ah {
                row.fill(beyond);
                continue;
            }
            if ry == -1 || ry == ah {
                row.fill(beyond);
                let from = (ax - 1).clamp(0, sw as i64) as usize;
                let to = (ax + aw + 1).clamp(0, sw as i64) as usize;
                row[from..to].fill(EDGE);
                continue;
            }
            let (r0, r1) = span(ry as usize, self.h as usize, fh);
            for (col_x, px) in row.iter_mut().enumerate() {
                let rx = col_x as i64 - ax;
                *px = if rx == -1 || rx == aw {
                    EDGE
                } else if rx < 0 || rx > aw {
                    beyond
                } else {
                    let (c0, c1) = cols[rx as usize];
                    average(rgba, fw, (c0, c1), (r0, r1))
                };
            }
        }
    }
}

/// The frame's cells, `[from, to)`, that the area's cell `i` of `n`
/// covers out of the frame's `m`: at least one.
fn span(i: usize, n: usize, m: usize) -> (usize, usize) {
    let from = (i * m / n).min(m - 1);
    let to = ((i + 1) * m / n).clamp(from + 1, m);
    (from, to)
}

/// The average colour of the frame's pixels in columns `cols` and rows
/// `rows`, as 0RGB.
fn average(rgba: &[u8], fw: usize, cols: (usize, usize), rows: (usize, usize)) -> u32 {
    let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
    for y in rows.0..rows.1 {
        let line = &rgba[(y * fw + cols.0) * 4..(y * fw + cols.1) * 4];
        for p in line.chunks_exact(4) {
            r += u32::from(p[0]);
            g += u32::from(p[1]);
            b += u32::from(p[2]);
            n += 1;
        }
    }
    let n = n.max(1);
    ((r / n) << 16) | ((g / n) << 8) | (b / n)
}

/// The pointer's events carried into the area: what a press begun
/// outside it does is nothing, and a press begun inside it follows the
/// pointer to the area's edge.
#[derive(Debug, Default)]
pub struct Carry {
    /// A press began outside the area and has not ended.
    outside: bool,
}

impl Carry {
    /// `event`, in window pixels, as the frame's: `None` where it falls
    /// outside the area. Events with no position pass as they are. A
    /// lookup.
    pub fn carry(&mut self, fit: &Fit, event: Event) -> Option<Event> {
        let at = |x: u16, y: u16| fit.to_frame(f64::from(x), f64::from(y));
        match event {
            Event::Touch { x, y, phase } => match phase {
                TouchPhase::Down => {
                    let inside = at(x, y);
                    self.outside = inside.is_none();
                    inside.map(|(x, y)| Event::Touch { x, y, phase })
                }
                TouchPhase::Move | TouchPhase::Up => {
                    let outside = self.outside;
                    if phase == TouchPhase::Up {
                        self.outside = false;
                    }
                    if outside {
                        return None;
                    }
                    let (x, y) = fit.to_frame_clamped(f64::from(x), f64::from(y));
                    Some(Event::Touch { x, y, phase })
                }
            },
            Event::Hover { x, y } => Some(match at(x, y) {
                Some((x, y)) => Event::Hover { x, y },
                None => Event::HoverEnd,
            }),
            Event::Scroll { x, y, dy } => at(x, y).map(|(x, y)| Event::Scroll { x, y, dy }),
            Event::Wheel { x, y, dy } => at(x, y).map(|(x, y)| Event::Wheel { x, y, dy }),
            Event::ScrollEnd { x, y } => {
                let (x, y) = fit.to_frame_clamped(f64::from(x), f64::from(y));
                Some(Event::ScrollEnd { x, y })
            }
            other => Some(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Pi panel fits a 1280 × 800 window at its own size, centred;
    /// a full monitor is scaled down to fit inside the margin.
    #[test]
    fn a_screen_is_centred_and_shrinks_only_to_fit() {
        let pi = Fit::of((1280, 800), (268, 358), 1.0);
        assert_eq!((pi.w, pi.h), (268, 358));
        assert_eq!((pi.x, pi.y), (506, 221));
        let monitor = Fit::of((1280, 800), (1920, 1080), 1.0);
        assert!(monitor.w <= 1280 - 48 && monitor.h <= 800 - 48);
        assert_eq!(monitor.w * 1080 / 1920, monitor.h);
    }

    /// A press inside the area lands on the frame's pixel under it; one
    /// outside does nothing, and neither does its release.
    #[test]
    fn a_press_lands_on_the_frame_under_it() {
        let fit = Fit::of((1280, 800), (2560, 1600), 1.0);
        let mut carry = Carry::default();
        let (x, y) = (
            fit.x as u16 + fit.w as u16 / 2,
            fit.y as u16 + fit.h as u16 / 2,
        );
        let down = Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        };
        let Some(Event::Touch { x: fx, y: fy, .. }) = carry.carry(&fit, down) else {
            panic!("the press was not carried");
        };
        assert!((i32::from(fx) - 1280).abs() <= 2 && (i32::from(fy) - 800).abs() <= 2);
        let outside = Event::Touch {
            x: 2,
            y: 2,
            phase: TouchPhase::Down,
        };
        assert_eq!(carry.carry(&fit, outside), None);
        let up = Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        };
        assert_eq!(carry.carry(&fit, up), None);
    }
}
