//! The window's input as the shell API's events: the mouse as a finger,
//! a touchscreen's first finger, the wheel and two-finger scrolling, the
//! keyboard, and the mouse's back and forward buttons.

use osk_shell_api::{Event, Key, TouchPhase};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key as WinitKey, NamedKey};

/// What a window event comes to.
#[derive(Debug, PartialEq)]
pub enum Input {
    /// An event for the app.
    Event(Event),
    /// A function key the shell itself may use: 2 for F2, up to 5.
    Function(u8),
}

/// The pointer and keyboard state between window events.
#[derive(Debug, Default)]
pub struct Pointer {
    /// Where the pointer is, in the window's pixels.
    pub cursor: (u16, u16),
    /// The left button, or a finger, is down.
    pub pressed: bool,
    /// The finger pressing, on a touchscreen.
    finger: Option<u64>,
    /// Scrolling not yet sent: less than a pixel.
    wheel: f32,
    /// Shift is held.
    shift: bool,
    /// Ctrl is held: Ctrl+A is [`Key::SelectAll`], as Shift+Tab is
    /// [`Key::BackTab`].
    ctrl: bool,
}

impl Pointer {
    /// A pointer at the window's corner, nothing pressed.
    pub fn new() -> Pointer {
        Pointer::default()
    }

    /// The events `event` makes, in the order they are sent. Events the
    /// window itself handles (closing, resizing, redrawing, a drop) make
    /// none.
    pub fn translate(&mut self, event: &WindowEvent) -> Vec<Input> {
        let mut out = Vec::new();
        let touch = |out: &mut Vec<Input>, cursor: (u16, u16), phase| {
            let (x, y) = cursor;
            out.push(Input::Event(Event::Touch { x, y, phase }));
        };
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let p = (
                    position.x.clamp(0.0, 65535.0) as u16,
                    position.y.clamp(0.0, 65535.0) as u16,
                );
                let moved = p != self.cursor;
                self.cursor = p;
                if self.pressed && moved {
                    touch(&mut out, p, TouchPhase::Move);
                } else if moved {
                    out.push(Input::Event(Event::Hover { x: p.0, y: p.1 }));
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if !self.pressed {
                    out.push(Input::Event(Event::HoverEnd));
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    self.pressed = true;
                    touch(&mut out, self.cursor, TouchPhase::Down);
                }
                ElementState::Released => {
                    if self.pressed {
                        self.pressed = false;
                        touch(&mut out, self.cursor, TouchPhase::Up);
                    }
                }
            },
            WindowEvent::Focused(false) => {
                if self.pressed {
                    self.pressed = false;
                    touch(&mut out, self.cursor, TouchPhase::Up);
                }
            }
            // A finger on a touchscreen presses as on the device's panel:
            // the first finger down is the press, others are ignored.
            WindowEvent::Touch(t) => {
                use winit::event::TouchPhase as T;
                let p = (
                    t.location.x.clamp(0.0, 65535.0) as u16,
                    t.location.y.clamp(0.0, 65535.0) as u16,
                );
                match t.phase {
                    T::Started if self.finger.is_none() => {
                        self.finger = Some(t.id);
                        self.cursor = p;
                        self.pressed = true;
                        touch(&mut out, p, TouchPhase::Down);
                    }
                    T::Moved if self.finger == Some(t.id) => {
                        self.cursor = p;
                        touch(&mut out, p, TouchPhase::Move);
                    }
                    T::Ended | T::Cancelled if self.finger == Some(t.id) => {
                        self.cursor = p;
                        self.finger = None;
                        self.pressed = false;
                        touch(&mut out, p, TouchPhase::Up);
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                // The shell API's scroll is pixels the content moves up;
                // winit's positive is a wheel turned away or two fingers
                // moved up, which move the content down. A wheel notch
                // is 48 pixels, as on the stick, and glides; two fingers
                // move the content with them and coast when they lift.
                let (x, y) = self.cursor;
                let (dy, notched) = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => (-lines * 48.0, true),
                    MouseScrollDelta::PixelDelta(p) => (-(p.y as f32), false),
                };
                self.wheel += dy;
                let whole = self.wheel.round();
                self.wheel -= whole;
                if whole != 0.0 {
                    let dy = whole.clamp(-32768.0, 32767.0) as i16;
                    out.push(Input::Event(if notched {
                        Event::Wheel { x, y, dy }
                    } else {
                        Event::Scroll { x, y, dy }
                    }));
                }
                if !notched && *phase == winit::event::TouchPhase::Ended {
                    self.wheel = 0.0;
                    out.push(Input::Event(Event::ScrollEnd { x, y }));
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                let shift = m.state().shift_key();
                if shift != self.shift {
                    self.shift = shift;
                    out.push(Input::Event(Event::Shift { held: shift }));
                }
                self.ctrl = m.state().control_key();
            }
            // The mouse's back and forward buttons.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Back,
                ..
            } => out.push(Input::Event(Event::Back)),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Forward,
                ..
            } => out.push(Input::Event(Event::Forward)),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return out;
                }
                let function = match &event.logical_key {
                    WinitKey::Named(NamedKey::F2) => Some(2),
                    WinitKey::Named(NamedKey::F3) => Some(3),
                    WinitKey::Named(NamedKey::F4) => Some(4),
                    WinitKey::Named(NamedKey::F5) => Some(5),
                    _ => None,
                };
                if let Some(n) = function {
                    out.push(Input::Function(n));
                    return out;
                }
                let Some(key) = map_key(&event.logical_key) else {
                    return out;
                };
                let key = if key == Key::Tab && self.shift {
                    Key::BackTab
                } else if self.ctrl && matches!(key, Key::Char('a' | 'A')) {
                    Key::SelectAll
                } else {
                    key
                };
                out.push(Input::Event(Event::Key(key)));
            }
            _ => {}
        }
        out
    }
}

/// The shell API's key for a key winit names, when it has one.
pub fn map_key(key: &WinitKey) -> Option<Key> {
    match key {
        WinitKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
        WinitKey::Named(NamedKey::Delete) => Some(Key::Delete),
        WinitKey::Named(NamedKey::Enter) => Some(Key::Enter),
        WinitKey::Named(NamedKey::Escape) => Some(Key::Escape),
        WinitKey::Named(NamedKey::ArrowUp) => Some(Key::Up),
        WinitKey::Named(NamedKey::ArrowDown) => Some(Key::Down),
        WinitKey::Named(NamedKey::ArrowLeft) => Some(Key::Left),
        WinitKey::Named(NamedKey::ArrowRight) => Some(Key::Right),
        WinitKey::Named(NamedKey::Home) => Some(Key::Home),
        WinitKey::Named(NamedKey::End) => Some(Key::End),
        WinitKey::Named(NamedKey::Tab) => Some(Key::Tab),
        WinitKey::Named(NamedKey::Space) => Some(Key::Char(' ')),
        WinitKey::Character(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if !c.is_control() => Some(Key::Char(c)),
                _ => None,
            }
        }
        _ => None,
    }
}
