//! The window and its drawing surface: an app's frame copied in and
//! presented, and what the screen says about itself.

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware};
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
        let frame = app.frame();
        let rendered = timed.then(Instant::now);
        let (fw, fh) = (usize::from(frame.width), usize::from(frame.height));
        let sw = size.width as usize;
        // The frame's premultiplied RGBA to the window's 0RGB, a row at a
        // time: the window is the frame's size but for the moment after a
        // resize, when what the frame does not cover is the page colour.
        let shown = fw.min(sw);
        for (y, row) in buffer.chunks_exact_mut(sw).enumerate() {
            if y < fh {
                let src = &frame.rgba[y * fw * 4..(y * fw + shown) * 4];
                for (dst, px) in row[..shown].iter_mut().zip(src.chunks_exact(4)) {
                    *dst = (u32::from(px[0]) << 16) | (u32::from(px[1]) << 8) | u32::from(px[2]);
                }
                row[shown..].fill(BEYOND);
            } else {
                row.fill(BEYOND);
            }
        }
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
