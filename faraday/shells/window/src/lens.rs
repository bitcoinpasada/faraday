//! The camera and the scanner that reads its frames, as the app asks for
//! them: on when the app's scan sheet opens, the newest frame to the app
//! to draw and to the scanner to read, what the scanner read back to the
//! app.

use std::path::PathBuf;

use faraday_core::{Faraday, StorageEvent};
use faraday_scanner::{ReadMode, Scanner, reduce, reduce_chroma, reduce_factor};
use faraday_shell_api::Event;

use crate::camera::{self, Camera};

/// The camera and its scanner.
pub struct Lens {
    /// The webcam, when the app asks for it.
    camera: Camera,
    /// Reads the codes in the camera's frames on its own thread.
    scanner: Option<Scanner>,
}

/// The shell a [`Lens`] delivers to.
pub trait LensHost {
    /// The lens.
    fn lens(&mut self) -> &mut Lens;
    /// Delivers one event to the app, and answers what it asks.
    fn send(&mut self, event: Event);
    /// The Faraday that asked for the camera, for what the scanner saw.
    fn faraday(&mut self) -> Option<&mut Faraday>;
}

impl Lens {
    /// A lens on `camera`, off.
    pub fn new(camera: Camera) -> Lens {
        Lens {
            camera,
            scanner: None,
        }
    }

    /// The app asked for the camera (`Command::CameraOn`). Which cameras
    /// there are goes to the app first, for the scan sheet's choice, as
    /// of the moment it opens. The event to send back when the camera
    /// cannot start.
    pub fn on(&mut self, app: &mut Faraday) -> Option<Event> {
        app.storage(StorageEvent::Cameras(camera::list()));
        if let Err(reason) = self.camera.on() {
            eprintln!("faraday: camera unavailable: {reason}");
            return Some(reason.event());
        }
        if self.scanner.is_none() {
            self.scanner = Some(Scanner::new(ReadMode::Full));
        }
        None
    }

    /// The app is done with the camera (`Command::CameraOff`), or the
    /// window is closing.
    pub fn off(&mut self) {
        self.camera.off();
        self.scanner = None;
    }

    /// A camera the person chose on the scan sheet.
    pub fn chosen(&mut self, app: &mut Faraday) {
        if let Some(id) = app.take_camera() {
            self.camera.choose(PathBuf::from(id));
        }
    }

    /// Delivers what the camera and the scanner have for the app: the
    /// camera's answers, the newest frame (to the scanner as well), each
    /// code read, and what the scanner saw of a code, for the outline
    /// and for "too fine for this camera".
    pub fn pump<H: LensHost>(host: &mut H) {
        let mut newest = None;
        let mut answers = Vec::new();
        while let Some(event) = host.lens().camera.poll() {
            match event {
                Event::CameraFrame { .. } => newest = Some(event),
                other => answers.push(other),
            }
        }
        for answer in answers {
            host.lens().scanner = None;
            host.send(answer);
        }
        if let Some(Event::CameraFrame {
            width,
            height,
            luma,
            chroma,
        }) = newest
        {
            // The preview is the reduced frame, as on the stick, and the
            // scanner gets the camera's own pixels: a 1280 × 720 frame
            // is reduced once here and not again by the core's drawing.
            let (w, h) = (usize::from(width), usize::from(height));
            let factor = reduce_factor(w, h);
            let (rw, rh, small) = reduce(w, h, &luma);
            let small_chroma = chroma
                .as_deref()
                .map(|uv| reduce_chroma(w, h, uv, factor))
                .filter(|uv| !uv.is_empty());
            if let Some(scanner) = host.lens().scanner.as_ref() {
                scanner.offer(width, height, luma);
            }
            // A frame shorter than its size says reduces to nothing and
            // is not drawn.
            if (rw, rh) != (0, 0) {
                host.send(Event::CameraFrame {
                    width: rw,
                    height: rh,
                    luma: small,
                    chroma: small_chroma,
                });
            }
        }
        while let Some(bytes) = host.lens().scanner.as_ref().and_then(Scanner::poll) {
            host.send(Event::Scanned { bytes });
        }
        if let Some(seen) = host.lens().scanner.as_ref().and_then(Scanner::poll_seen)
            && let Some(app) = host.faraday()
        {
            app.storage(StorageEvent::QrSeen {
                width: seen.width,
                height: seen.height,
                corners: seen.corners,
                module_tenths: (seen.module_px * 10.0).clamp(0.0, 65535.0) as u16,
                read: seen.read,
            });
        }
    }
}
