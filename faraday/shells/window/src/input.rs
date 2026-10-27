//! The window's input as the shell API's events: the mouse as a finger,
//! a touchscreen's first finger, the wheel and two-finger scrolling, the
//! keyboard, and the mouse's back and forward buttons.
//!
//! A finger on a touchscreen is one press, as on the device's panel.
//! Some systems also move the pointer with it and make mouse presses of
//! it (Windows makes a left press and release of a touch, X11 moves the
//! pointer to the first finger), which would press a second time: while
//! a finger is down, and for [`FINGER_ECHO`] after it lifts, the left
//! button and the pointer's moves are the finger's echo and make nothing.

use std::time::{Duration, Instant};

use faraday_shell_api::{Event, Key, TouchPhase};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key as WinitKey, NamedKey};

/// The pointer and keyboard state between window events.
#[derive(Debug, Default)]
pub struct Pointer {
    /// Where the pointer is, in the window's pixels.
    pub cursor: (u16, u16),
    /// The left button, or a finger, is down.
    pub pressed: bool,
    /// The finger pressing, on a touchscreen.
    finger: Option<u64>,
    /// When the last finger lifted: the mouse presses the system makes
    /// of it come up to [`FINGER_ECHO`] later.
    finger_up: Option<Instant>,
    /// Scrolling not yet sent: less than a pixel.
    wheel: f32,
    /// Shift is held.
    shift: bool,
    /// Ctrl is held: Ctrl+A is [`Key::SelectAll`], as Shift+Tab is
    /// [`Key::BackTab`].
    ctrl: bool,
    /// Ctrl is held, or Command on macOS: with V it is [`Key::Paste`].
    command: bool,
}

/// How long after a finger lifts a left press is the system's copy of
/// the touch rather than the mouse's.
pub const FINGER_ECHO: Duration = Duration::from_millis(500);

impl Pointer {
    /// A pointer at the window's corner, nothing pressed.
    pub fn new() -> Pointer {
        Pointer::default()
    }

    /// Whether the left button and the pointer's moves at `now` are a
    /// finger's echo: a finger is down, or lifted less than
    /// [`FINGER_ECHO`] ago. A lookup.
    pub fn finger_echo(&self, now: Instant) -> bool {
        self.finger.is_some()
            || self
                .finger_up
                .is_some_and(|t| now.saturating_duration_since(t) < FINGER_ECHO)
    }

    /// Whether `event` is a mouse event the system made of a finger
    /// ([`Pointer::finger_echo`]), which the window does not act on
    /// either (a press on the top strip's room). A lookup.
    pub fn is_echo(&self, event: &WindowEvent) -> bool {
        matches!(
            event,
            WindowEvent::CursorMoved { .. }
                | WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    ..
                }
        ) && self.finger_echo(Instant::now())
    }

    /// Finger `id` on a touchscreen at `p`, in the window's pixels, at
    /// `now`: the first finger down is the press and its moves and lift
    /// follow; other fingers make nothing, as on the device's panel.
    pub fn finger(
        &mut self,
        id: u64,
        phase: TouchPhase,
        p: (u16, u16),
        now: Instant,
    ) -> Vec<Event> {
        let (x, y) = p;
        let ours = self.finger == Some(id);
        match phase {
            TouchPhase::Down if self.finger.is_none() => {
                self.finger = Some(id);
                self.pressed = true;
            }
            TouchPhase::Move if ours && p != self.cursor => {}
            TouchPhase::Up if ours => {
                self.finger = None;
                self.finger_up = Some(now);
                self.pressed = false;
            }
            _ => return Vec::new(),
        }
        self.cursor = p;
        vec![Event::Touch { x, y, phase }]
    }

    /// The events `event` makes, in the order they are sent. Events the
    /// window itself handles (closing, resizing, redrawing, a drop) make
    /// none.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub fn translate(&mut self, event: &WindowEvent) -> Vec<Event> {
        let mut out = Vec::new();
        let touch = |out: &mut Vec<Event>, cursor: (u16, u16), phase| {
            let (x, y) = cursor;
            out.push(Event::Touch { x, y, phase });
        };
        match event {
            // The pointer and the left button the system works with a
            // finger: the finger's own events say what it does.
            WindowEvent::CursorMoved { .. }
            | WindowEvent::MouseInput {
                button: MouseButton::Left,
                ..
            } if self.finger_echo(Instant::now()) => {}
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
                    out.push(Event::Hover { x: p.0, y: p.1 });
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if !self.pressed {
                    out.push(Event::HoverEnd);
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
                // A finger down is lifted with it: its own lift, later,
                // makes nothing more.
                if self.finger.take().is_some() {
                    self.finger_up = Some(Instant::now());
                }
                out.push(Event::Focus(false));
            }
            WindowEvent::Focused(true) => out.push(Event::Focus(true)),
            // A finger on a touchscreen presses as on the device's panel.
            WindowEvent::Touch(t) => {
                use winit::event::TouchPhase as T;
                let p = (
                    t.location.x.clamp(0.0, 65535.0) as u16,
                    t.location.y.clamp(0.0, 65535.0) as u16,
                );
                let phase = match t.phase {
                    T::Started => TouchPhase::Down,
                    T::Moved => TouchPhase::Move,
                    T::Ended | T::Cancelled => TouchPhase::Up,
                };
                return self.finger(t.id, phase, p, Instant::now());
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
                    out.push(if notched {
                        Event::Wheel { x, y, dy }
                    } else {
                        Event::Scroll { x, y, dy }
                    });
                }
                if !notched && *phase == winit::event::TouchPhase::Ended {
                    self.wheel = 0.0;
                    out.push(Event::ScrollEnd { x, y });
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                let shift = m.state().shift_key();
                if shift != self.shift {
                    self.shift = shift;
                    out.push(Event::Shift { held: shift });
                }
                self.ctrl = m.state().control_key();
                self.command = self.ctrl || (cfg!(target_os = "macos") && m.state().super_key());
            }
            // The mouse's back and forward buttons.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Back,
                ..
            } => out.push(Event::Back),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Forward,
                ..
            } => out.push(Event::Forward),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return out;
                }
                let Some(key) = map_key(&event.logical_key) else {
                    return out;
                };
                let key = if key == Key::Tab && self.shift {
                    Key::BackTab
                } else if self.ctrl && matches!(key, Key::Char('a' | 'A')) {
                    Key::SelectAll
                } else if self.command && matches!(key, Key::Char('v' | 'V')) {
                    Key::Paste
                } else {
                    key
                };
                out.push(Event::Key(key));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: u16, y: u16, phase: TouchPhase) -> Vec<Event> {
        vec![Event::Touch { x, y, phase }]
    }

    #[test]
    fn a_finger_presses_once_and_a_second_finger_makes_nothing() {
        let mut p = Pointer::new();
        let t0 = Instant::now();
        assert_eq!(
            p.finger(7, TouchPhase::Down, (10, 20), t0),
            at(10, 20, TouchPhase::Down)
        );
        assert!(p.finger(8, TouchPhase::Down, (90, 90), t0).is_empty());
        assert!(p.finger(8, TouchPhase::Move, (95, 90), t0).is_empty());
        assert_eq!(
            p.finger(7, TouchPhase::Move, (10, 40), t0),
            at(10, 40, TouchPhase::Move)
        );
        // The same place again is no move.
        assert!(p.finger(7, TouchPhase::Move, (10, 40), t0).is_empty());
        assert!(p.finger(8, TouchPhase::Up, (95, 90), t0).is_empty());
        assert_eq!(
            p.finger(7, TouchPhase::Up, (10, 40), t0),
            at(10, 40, TouchPhase::Up)
        );
        assert!(!p.pressed);
    }

    #[test]
    fn the_mouse_press_the_system_makes_of_a_touch_presses_nothing_more() {
        let mut p = Pointer::new();
        let t0 = Instant::now();
        assert!(!p.finger_echo(t0));
        p.finger(1, TouchPhase::Down, (5, 5), t0);
        assert!(p.finger_echo(t0));
        p.finger(1, TouchPhase::Up, (5, 5), t0);
        // Windows' left press and release of the tap come just after it.
        assert!(p.finger_echo(t0 + Duration::from_millis(100)));
        // The mouse, a while later, is the mouse again.
        assert!(!p.finger_echo(t0 + FINGER_ECHO));
    }
}
