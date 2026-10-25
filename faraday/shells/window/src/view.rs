//! The window and its drawing surface: an app's frame copied in and
//! presented, and what the screen says about itself.

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use faraday_shell_api::{
    App, BootState, Command, DisplayInfo, Event, Frame, Parts, SecureHardware,
};
use softbuffer::{Context, Surface};
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowAttributes};

use crate::pace::{FRAME, Pacer};

/// What the frame does not cover, for the moment after a resize: the
/// page colour.
const BEYOND: u32 = 0x0011_161c;

/// A window with a surface to draw into.
pub struct View {
    /// The window.
    pub window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
}

impl View {
    /// Opens a window with `attributes`, or says why not.
    pub fn open(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> Result<View, String> {
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Rc::new(w),
            Err(e) => return Err(format!("cannot open a window: {e}")),
        };
        let Ok(context) = Context::new(window.clone()) else {
            return Err(String::from("cannot create a drawing context"));
        };
        let Ok(surface) = Surface::new(&context, window.clone()) else {
            return Err(String::from("cannot create a drawing surface"));
        };
        Ok(View { window, surface })
    }

    /// Whether the compositor paces the frames. Wayland delivers a
    /// redraw with the screen's refresh once a frame has been presented;
    /// X11 delivers it at once, so there the timer keeps the pace.
    pub fn paced(&self) -> bool {
        matches!(
            self.window.window_handle().map(|h| h.as_raw()),
            Ok(RawWindowHandle::Wayland(_))
        )
    }

    /// The screen's refresh period, from the monitor the window is on;
    /// 60 Hz when it does not say.
    pub fn refresh(&self) -> Duration {
        refresh(Some(&self.window))
    }

    /// Draws `app`'s frame into the window and presents it. A frame that
    /// asks for the next one (a fade part of the way) gets a redraw from
    /// here: no event is coming to ask on its behalf.
    pub fn draw<A: App + ?Sized>(&mut self, app: &mut A, pacer: &mut Pacer) {
        let window = &self.window;
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if self.surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = self.surface.buffer_mut() else {
            return;
        };
        let begun = Instant::now();
        let timed = pacer.timed();
        let start = timed.then_some(begun);
        let parts = app.parts();
        let rendered = timed.then(Instant::now);
        copy(&parts, &mut buffer, size.width as usize);
        // Tells winit a frame is about to be shown: on Wayland it asks
        // the compositor to say when the frame is on the screen, and
        // holds the next redraw until then, which is what paces the
        // frames to the screen's refresh.
        window.pre_present_notify();
        let _ = buffer.present();
        let mut again = false;
        while let Some(c) = app.poll_command() {
            again |= matches!(c, Command::Draw);
        }
        if again {
            window.request_redraw();
        }
        if !pacer.paced {
            pacer.drew(begun.elapsed(), refresh(Some(window)));
        }
        if let (Some(start), Some(rendered)) = (start, rendered) {
            let now = Instant::now();
            pacer.presented(rendered - start, now - rendered, now);
        }
    }
}

/// Costs a pass over the window's pixels. Copies `parts` into `buffer`,
/// a window `sw` pixels wide, each pixel once: the premultiplied RGBA to
/// the window's 0RGB, a row at a time; the inner frame where it stands,
/// the base everywhere else; what neither covers (the moment after a
/// resize) the page colour.
pub fn copy(parts: &Parts<'_>, buffer: &mut [u32], sw: usize) {
    if sw == 0 {
        return;
    }
    let base = parts.base;
    let inner = parts
        .inner
        .map(|(f, x, y)| (f, usize::from(x), usize::from(y)));
    for (y, row) in buffer.chunks_exact_mut(sw).enumerate() {
        match inner {
            Some((f, ix, iy)) if y >= iy && y < iy + usize::from(f.height) && ix < sw => {
                let iw = usize::from(f.width).min(sw - ix);
                put(&base, y, &mut row[..ix], 0);
                put(&f, y - iy, &mut row[ix..ix + iw], 0);
                put(&base, y, &mut row[ix + iw..], ix + iw);
            }
            _ => put(&base, y, row, 0),
        }
    }
}

/// Row `y` of `frame` from its column `from` into `dst`, the page colour
/// where the frame ends.
fn put(frame: &Frame<'_>, y: usize, dst: &mut [u32], from: usize) {
    let (fw, fh) = (usize::from(frame.width), usize::from(frame.height));
    let shown = if y < fh {
        fw.saturating_sub(from).min(dst.len())
    } else {
        0
    };
    if shown > 0 {
        let at = (y * fw + from) * 4;
        let src = &frame.rgba[at..at + shown * 4];
        for (d, px) in dst[..shown].iter_mut().zip(src.chunks_exact(4)) {
            *d = (u32::from(px[0]) << 16) | (u32::from(px[1]) << 8) | u32::from(px[2]);
        }
    }
    dst[shown..].fill(BEYOND);
}

/// The refresh period of the monitor `window` is on; 60 Hz when there is
/// no window or it does not say.
pub fn refresh(window: Option<&Window>) -> Duration {
    window
        .and_then(|w| w.current_monitor())
        .and_then(|m| m.refresh_rate_millihertz())
        .filter(|&mhz| mhz > 0)
        .map_or(FRAME, |mhz| {
            Duration::from_secs_f64(1000.0 / f64::from(mhz))
        })
}

/// The display event for a window `shown` pixels in size at the
/// compositor's `scale`, or standing for a device's screen at `ppi`
/// pixels per inch.
pub fn display((w, h): (u32, u32), scale: f64, ppi: Option<f64>) -> Event {
    Event::Display(DisplayInfo {
        width: w.clamp(320, u32::from(u16::MAX)) as u16,
        height: h.clamp(240, u32::from(u16::MAX)) as u16,
        // The compositor's scale for this window, as a density: the
        // app grows no larger than a quarter beyond it. Standing for
        // a device's screen, this screen's own density, so the app
        // is laid out at that screen's true size.
        dpi: match ppi {
            Some(p) => p.round().clamp(80.0, 640.0) as u16,
            None => (160.0 * scale).round().clamp(80.0, 640.0) as u16,
        },
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame `w` × `h` of one colour.
    fn solid(w: u16, h: u16, rgb: [u8; 3]) -> Vec<u8> {
        let px = [rgb[0], rgb[1], rgb[2], 255];
        px.repeat(usize::from(w) * usize::from(h))
    }

    #[test]
    fn the_window_shows_the_inner_frame_where_it_stands_and_the_base_round_it() {
        let base = solid(6, 4, [1, 2, 3]);
        let inner = solid(3, 2, [9, 8, 7]);
        let parts = Parts {
            base: Frame {
                width: 6,
                height: 4,
                rgba: &base,
            },
            inner: Some((
                Frame {
                    width: 3,
                    height: 2,
                    rgba: &inner,
                },
                2,
                1,
            )),
        };
        // A window a column wider and a row taller than the frame: the
        // moment after a resize.
        let mut buffer = vec![0u32; 7 * 5];
        copy(&parts, &mut buffer, 7);
        for y in 0..5 {
            for x in 0..7 {
                let want = if x >= 6 || y >= 4 {
                    BEYOND
                } else if (2..5).contains(&x) && (1..3).contains(&y) {
                    0x0009_0807
                } else {
                    0x0001_0203
                };
                assert_eq!(buffer[y * 7 + x], want, "at {x}, {y}");
            }
        }
    }
}
