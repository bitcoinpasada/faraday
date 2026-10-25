//! Moving the window by its top strip, as desktop apps that draw their
//! own title area do (`docs/DESIGN.md` §5, the top strip): a press on
//! empty room in the strip followed by movement hands the window to the
//! system to move (`Window::drag_window`); a double-click there toggles
//! maximised. A press that is not followed by movement does nothing.

use std::time::{Duration, Instant};

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::window::Window;

/// How far the pointer moves, in pixels, before a press on the strip is a
/// drag.
const SLOP: f64 = 4.0;

/// The longest gap between two presses that make a double-click.
const DOUBLE: Duration = Duration::from_millis(400);

/// A press on empty strip room, until it lifts or becomes a drag.
#[derive(Debug, Default)]
pub struct TitleDrag {
    /// Where the press went down.
    down: Option<PhysicalPosition<f64>>,
    /// When the last press on the strip room was.
    last: Option<Instant>,
}

impl TitleDrag {
    /// Whether the window takes `event` itself, so the app is not given
    /// it. `cursor` is where the pointer is, in pixels; `room` says
    /// whether a point is empty strip room (asked only for a left press).
    /// Costs a lookup.
    pub fn takes(
        &mut self,
        event: &WindowEvent,
        cursor: (u16, u16),
        window: &Window,
        room: impl FnOnce(i32, i32) -> bool,
    ) -> bool {
        match event {
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !room(i32::from(cursor.0), i32::from(cursor.1)) {
                    self.down = None;
                    return false;
                }
                let now = Instant::now();
                if self.last.is_some_and(|t| now - t <= DOUBLE) {
                    self.last = None;
                    self.down = None;
                    if window.is_resizable() {
                        window.set_maximized(!window.is_maximized());
                    }
                } else {
                    self.last = Some(now);
                    self.down = Some(PhysicalPosition::new(
                        f64::from(cursor.0),
                        f64::from(cursor.1),
                    ));
                }
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(d) = self.down
                    && (position.x - d.x).hypot(position.y - d.y) > SLOP
                {
                    self.down = None;
                    self.last = None;
                    let _ = window.drag_window();
                    return true;
                }
                false
            }
            // The lift of a press the window took: the app never saw the
            // press, so it is not given the lift.
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.down.take().is_some(),
            _ => false,
        }
    }
}
