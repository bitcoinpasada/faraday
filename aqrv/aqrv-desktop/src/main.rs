//! The Argon QR Vault's desktop test app (`https://github.com/bitcoinpasada/aqrv`, `AQRV.md`): `aqrv-app` in
//! a window, run by the frame pacing, input and copy `faraday-window`
//! shares with the other desktop shells. Nothing here links faraday-core.
//!
//! It answers the app's commands: entropy from the system, a file read
//! by path or dropped on the window, and a file saved into the Downloads
//! folder.

mod files;

use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use aqrv_app::ArgonQrVault;
use faraday_shell_api::{App, Command, Event, FileKind};
use faraday_window::entropy;
use faraday_window::input::Pointer;
use faraday_window::pace::{Pacer, TICK};
use faraday_window::view::{self, View};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

/// The window's title, as the app names itself.
const TITLE: &str = faraday_strings::EN.aqrv_window_title;
/// The window's identity: the name its taskbar entry and icon go by.
const ID: &str = "app.faradayoffline.ArgonQrVault";

struct Shell {
    app: ArgonQrVault,
    view: Option<View>,
    shown: (u32, u32),
    scale: f64,
    pace: Pacer,
    pointer: Pointer,
}

impl Shell {
    fn redraw(&self) {
        if let Some(view) = self.view.as_ref() {
            view.window.request_redraw();
        }
    }

    fn display(&self) -> Event {
        view::display(self.shown, self.scale, None)
    }

    /// Delivers `event`, then answers what the app asks for.
    fn send(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        let ticked = matches!(event, Event::Tick { .. });
        self.pace.before(&event);
        let mut draw = false;
        let mut events = vec![event];
        while let Some(e) = events.pop() {
            self.app.event(e);
            while let Some(c) = self.app.poll_command() {
                match c {
                    Command::Draw => draw = true,
                    Command::Exit => event_loop.exit(),
                    other => events.extend(answer(other)),
                }
            }
        }
        if ticked {
            self.pace.animating = draw;
        }
        if draw {
            self.redraw();
        }
    }

    /// A file dropped on the window: its bytes, or that it was not read.
    fn dropped(&mut self, event_loop: &ActiveEventLoop, path: &Path) {
        let kind = FileKind::Any;
        let event = match files::read(path) {
            Some(bytes) => Event::File { kind, bytes },
            None => Event::FileUnavailable { kind },
        };
        self.send(event_loop, event);
    }
}

/// What the system answers to one command of the app.
fn answer(command: Command) -> Vec<Event> {
    match command {
        Command::RequestEntropy => entropy(),
        // A name is a path: the typed one.
        Command::ReadFile { kind, name } => vec![match files::read(Path::new(&name)) {
            Some(bytes) => Event::File { kind, bytes },
            None => Event::FileUnavailable { kind },
        }],
        // The app has no picker to open.
        Command::RequestFile { kind } => vec![Event::FileUnavailable { kind }],
        Command::WriteFile {
            kind,
            name_hint,
            bytes,
        } => {
            let saved = files::downloads().and_then(|dir| files::save(&dir, &name_hint, &bytes));
            vec![match saved {
                Some(_) => Event::FileWritten { kind },
                None => Event::FileNotWritten { kind },
            }]
        }
        Command::RequestClipboard { kind } => vec![Event::ClipboardUnavailable { kind }],
        Command::WriteClipboard { kind, .. } => vec![Event::ClipboardNotWritten { kind }],
        _ => Vec::new(),
    }
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(LogicalSize::new(1000.0, 760.0))
            .with_min_inner_size(LogicalSize::new(360.0, 480.0));
        let attributes = view::identify(
            attributes,
            ID,
            include_bytes!("../../../faraday/shells/starter/assets/icon.png"),
        );
        let view = match View::open(event_loop, attributes) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("argon-qr-vault: {e}");
                event_loop.exit();
                return;
            }
        };
        let size = view.window.inner_size();
        self.shown = (size.width, size.height);
        self.scale = view.window.scale_factor();
        self.pace.paced = view.paced();
        self.view = Some(view);
        let display = self.display();
        self.send(event_loop, display);
        self.pace.next_tick = Instant::now() + TICK;
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pace.next_tick));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::DroppedFile(path) => self.dropped(event_loop, &path),
            WindowEvent::RedrawRequested => {
                if self.pace.paced {
                    let tick = self.pace.tick();
                    self.send(event_loop, tick);
                    self.pace.next_tick = Instant::now() + TICK;
                }
                if let Some(view) = self.view.as_mut() {
                    view.draw(&mut self.app, &mut self.pace);
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor;
                let display = self.display();
                self.send(event_loop, display);
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 && (size.width, size.height) != self.shown {
                    self.shown = (size.width, size.height);
                    let display = self.display();
                    self.send(event_loop, display);
                }
                self.redraw();
            }
            other => {
                for e in self.pointer.translate(&other) {
                    self.send(event_loop, e);
                }
            }
        }
    }

    /// The worker has an answer.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _work: ()) {
        self.pace.woken();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_none() {
            return;
        }
        let now = Instant::now();
        if now >= self.pace.next_tick {
            let tick = self.pace.tick();
            self.send(event_loop, tick);
            let refresh = self
                .view
                .as_ref()
                .map_or(view::refresh(None), View::refresh);
            self.pace.ticked_at(now, refresh);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pace.next_tick));
    }
}

fn main() -> ExitCode {
    let event_loop = match EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("argon-qr-vault: no display to open a window on: {e}");
            return ExitCode::FAILURE;
        }
    };
    let proxy: EventLoopProxy<()> = event_loop.create_proxy();
    let wake = move || {
        // A loop that has ended has no use for the note.
        let _ = proxy.send_event(());
    };
    let mut shell = Shell {
        app: ArgonQrVault::new(wake),
        view: None,
        shown: (1000, 760),
        scale: 1.0,
        pace: Pacer::new(false),
        pointer: Pointer::new(),
    };
    match event_loop.run_app(&mut shell) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("argon-qr-vault: {e}");
            ExitCode::FAILURE
        }
    }
}
