//! The window's loop, written once: the desktop app and the Starter each
//! implement [`Program`] for what is theirs (the window's attributes,
//! what an event does to the app, a file dropped) and [`run`] does the
//! rest: opening the window, the frames' pace, the mouse, a finger, the
//! keyboard and a press on the top strip that moves the window, the
//! camera, and the ticks.

use std::path::Path;
use std::time::Instant;

use faraday_core::Faraday;
use faraday_shell_api::Event;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::drag::TitleDrag;
use crate::input::Pointer;
use crate::lens::{Lens, LensHost};
use crate::pace::{Pacer, TICK};
use crate::view::{self, View};

/// What the loop holds for a [`Program`], which is handed it with each
/// call.
pub struct Core {
    /// The size the window opens at.
    pub size: (u32, u32),
    /// The window and its surface.
    pub view: Option<View>,
    /// The window's size in pixels, as last told to the app.
    pub shown: (u32, u32),
    /// The window's scale factor.
    pub scale: f64,
    /// The ticks and the frames' pace.
    pub pace: Pacer,
    /// The mouse, a finger and the keyboard.
    pub pointer: Pointer,
    /// A press on the top strip's empty room that may move the window.
    pub title: TitleDrag,
    /// The webcam, when the app asks for it, and the scanner that reads
    /// its frames on its own thread.
    pub lens: Lens,
}

impl Core {
    /// What a window at `size` starts with; `timings` is `--timings`.
    pub fn new(size: (u32, u32), timings: bool, lens: Lens) -> Core {
        Core {
            size,
            view: None,
            shown: size,
            scale: 1.0,
            pace: Pacer::new(timings),
            pointer: Pointer::new(),
            title: TitleDrag::default(),
            lens,
        }
    }

    /// The window, once it is open.
    pub fn window(&self) -> Option<&Window> {
        self.view.as_ref().map(|v| &*v.window)
    }

    /// Asks for a redraw.
    pub fn redraw(&self) {
        if let Some(w) = self.window() {
            w.request_redraw();
        }
    }
}

/// What differs between the programs that run in the window.
pub trait Program {
    /// What a line printed about the window starts with.
    const NAME: &'static str;

    /// The window's attributes: title, size, identity and icon.
    fn attributes(&self, core: &Core, event_loop: &ActiveEventLoop) -> WindowAttributes;

    /// The screen's pixels per inch the app is laid out for, when it is
    /// not the window's own.
    fn ppi(&self) -> Option<f64> {
        None
    }

    /// The window is open and the app has been told its display.
    fn started(&mut self, core: &mut Core, event_loop: &ActiveEventLoop);

    /// Delivers one event to the app, and answers what it asks.
    fn send(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, event: Event);

    /// The Faraday that asked for the camera, for what the scanner saw.
    fn faraday(&mut self) -> Option<&mut Faraday>;

    /// The window was asked to close.
    fn close_requested(&mut self, core: &mut Core, event_loop: &ActiveEventLoop);

    /// A file dropped on the window.
    fn dropped(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, path: &Path);

    /// A file dragged over the window (`over`), or the drag gone. Whether
    /// the program took it; if not, the pointer's translation gets it.
    fn hovered(&mut self, core: &mut Core, over: bool) -> bool;

    /// The scale factor changed (`core.scale` is the new one).
    fn scale_changed(&mut self, _core: &mut Core) {}

    /// Draws the frame into the window.
    fn draw(&mut self, core: &mut Core);

    /// Whether a press on the top strip's empty room may move the window.
    fn moves_window(&self) -> bool {
        true
    }

    /// Whether the point is empty room on the top strip.
    fn strip_room(&self, x: i32, y: i32) -> bool;

    /// What the program looks at before each pass's camera and tick.
    fn polled(&mut self, _core: &mut Core, _event_loop: &ActiveEventLoop) {}

    /// What the program looks at after each pass's tick, at `now`.
    fn ticked(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, now: Instant);

    /// The loop is ending: the program lets go of what it holds.
    fn exiting(&mut self, core: &mut Core);
}

/// Runs `program` in a window on `event_loop` until it ends, and gives
/// the program back with how the loop ended.
pub fn run<P: Program>(
    event_loop: EventLoop<()>,
    core: Core,
    program: P,
) -> (Result<(), winit::error::EventLoopError>, P) {
    let mut shell = Shell { core, program };
    let ran = event_loop.run_app(&mut shell);
    (ran, shell.program)
}

struct Shell<P> {
    core: Core,
    program: P,
}

/// The program and the loop, for the lens to deliver to.
struct Pass<'a, P> {
    core: &'a mut Core,
    program: &'a mut P,
    event_loop: &'a ActiveEventLoop,
}

impl<P: Program> LensHost for Pass<'_, P> {
    fn lens(&mut self) -> &mut Lens {
        &mut self.core.lens
    }

    fn send(&mut self, event: Event) {
        self.program.send(self.core, self.event_loop, event);
    }

    fn faraday(&mut self) -> Option<&mut Faraday> {
        self.program.faraday()
    }
}

impl<P: Program> Shell<P> {
    /// The window's own display, for the app to lay itself out in.
    fn display(&self) -> Event {
        view::display(self.core.shown, self.core.scale, self.program.ppi())
    }

    /// Tells the app the display.
    fn send_display(&mut self, event_loop: &ActiveEventLoop) {
        let display = self.display();
        self.program.send(&mut self.core, event_loop, display);
    }

    /// A tick, delivered.
    fn send_tick(&mut self, event_loop: &ActiveEventLoop) {
        let tick = self.core.pace.tick();
        self.program.send(&mut self.core, event_loop, tick);
    }
}

impl<P: Program> ApplicationHandler for Shell<P> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.core.view.is_some() {
            return;
        }
        let attributes = self.program.attributes(&self.core, event_loop);
        let view = match View::open(event_loop, attributes) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}: {e}", P::NAME);
                event_loop.exit();
                return;
            }
        };
        let size = view.window.inner_size();
        self.core.shown = (size.width, size.height);
        self.core.scale = view.window.scale_factor();
        self.core.pace.paced = view.paced();
        self.core.view = Some(view);
        self.send_display(event_loop);
        self.program.started(&mut self.core, event_loop);
        self.core.pace.next_tick = Instant::now() + TICK;
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.core.pace.next_tick));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.program.close_requested(&mut self.core, event_loop),
            // Only X11 reports a drop in winit 0.30; the list is the way
            // on native Wayland.
            WindowEvent::DroppedFile(path) => {
                self.program.dropped(&mut self.core, event_loop, &path);
            }
            // A file dragged over the window edges the page as the place
            // it drops, until it is dropped or the drag leaves.
            WindowEvent::HoveredFile(_) if self.program.hovered(&mut self.core, true) => {}
            WindowEvent::HoveredFileCancelled if self.program.hovered(&mut self.core, false) => {}
            WindowEvent::RedrawRequested => {
                // With the compositor pacing the frames, this is the
                // moment before the screen's next refresh: the tick moves
                // whatever moves to now, and the frame is drawn for the
                // moment it will be seen. A tick that asks for a frame
                // asks for the next redraw, which comes with the refresh
                // after; the timer stays as the idle tick, pushed back so
                // it does not tick between two refreshes.
                if self.core.pace.paced {
                    self.send_tick(event_loop);
                    self.core.pace.next_tick = Instant::now() + TICK;
                }
                self.program.draw(&mut self.core);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.core.scale = scale_factor;
                self.program.scale_changed(&mut self.core);
                self.send_display(event_loop);
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 && (size.width, size.height) != self.core.shown
                {
                    self.core.shown = (size.width, size.height);
                    // The app lays itself out again.
                    self.send_display(event_loop);
                }
                self.core.redraw();
            }
            // The mouse, a finger, the wheel and the keyboard, as the
            // shell API's events.
            other => {
                // A press on the top strip's empty room and a drag move
                // the window, a double-click maximises it; not the press
                // the system makes of a finger's.
                let core = &mut self.core;
                if self.program.moves_window()
                    && !core.pointer.is_echo(&other)
                    && let Some(view) = core.view.as_ref()
                    && core
                        .title
                        .takes(&other, core.pointer.cursor, &view.window, |x, y| {
                            self.program.strip_room(x, y)
                        })
                {
                    return;
                }
                for e in core.pointer.translate(&other) {
                    self.program.send(core, event_loop, e);
                }
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _work: ()) {
        self.core.pace.woken();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.core.view.is_none() {
            return;
        }
        self.program.polled(&mut self.core, event_loop);
        // The newest camera frame, to the app to draw and to the scanner
        // to read; and whatever the scanner has read since.
        Lens::pump(&mut Pass {
            core: &mut self.core,
            program: &mut self.program,
            event_loop,
        });
        let now = Instant::now();
        if now >= self.core.pace.next_tick {
            self.send_tick(event_loop);
            let refresh = view::refresh(self.core.window());
            self.core.pace.ticked_at(now, refresh);
        }
        self.program.ticked(&mut self.core, event_loop, now);
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.core.pace.next_tick));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.core.lens.off();
        self.program.exiting(&mut self.core);
    }
}
