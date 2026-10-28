//! Faraday Demo inside the Starter (F126, `docs/STARTER.md` §5.3): Open
//! Faraday Demo on Home shows the Demo, held by its host
//! (`faraday_core::Host`, `docs/DESKTOP-HOST.md`), in the Starter's own
//! window under the top app bar, whose way back returns to Home. The
//! shell makes the Demo's app ([`StarterCommand::OpenDemo`]), serves its
//! sticks and storage, and lets go of them when it is left
//! ([`StarterCommand::DemoClosed`]).
//!
//! One theme in a session: the Demo opens in the Starter's theme, and a
//! theme chosen in the Demo is the Starter's from then on, kept when the
//! Demo is left and in memory only (`docs/THEMES.md` §14). Text size is
//! one choice in the same way: the Demo opens at the Starter's, and one
//! chosen in the Demo is the Starter's, kept as About's choice is
//! (`docs/TEXT-SIZE.md` §6).

use faraday_core::{Action, Host};
use faraday_shell_api::{App, Command, DisplayInfo, Event, TouchPhase};

use crate::api::{Os, StarterCommand};
use crate::{Starter, StarterAction};

/// Faraday Demo as the Starter holds it.
pub(crate) struct Held {
    /// The Demo's host, holding its app.
    host: Host,
    /// The size the host was last told it has, in pixels. Invariant:
    /// what [`Starter::demo_area`] gives now, kept in step by
    /// [`Starter::fit_demo`] at every change of the window and when the
    /// Demo is shown.
    size: Option<(u16, u16)>,
    /// The press under way began on the Demo's frame, so its moves and
    /// its release are the Demo's too.
    touch: bool,
}

impl Starter {
    /// Whether Home offers Open Faraday Demo: on Linux and Windows. The
    /// macOS Starter offers none (F83). A lookup.
    pub fn offers_desktop(&self) -> bool {
        self.computer.os != Os::MacOs
    }

    /// Open Faraday Demo pressed: the shell is asked for the Demo.
    pub(crate) fn open_demo_asked(&mut self) {
        if self.offers_desktop() && self.demo.is_none() {
            self.desktop_failed = None;
            self.out.push_back(StarterCommand::OpenDemo);
        }
    }

    /// Shows `host` in the window under the top app bar, drawn in the
    /// Starter's theme: the Demo the shell made for Open Faraday Demo, or
    /// the fresh one a lock in the Demo starts in place of the last.
    pub fn show_demo(&mut self, mut host: Host) {
        host.app.draw_in(self.theme);
        host.app.draw_text_at(self.text);
        // Boxed: a `Faraday` is large, and the Starter holds one already.
        self.demo = Some(Box::new(Held {
            host,
            size: None,
            touch: false,
        }));
        self.menu = false;
        self.pressed = None;
        self.frost = None;
        self.fit_demo();
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// Whether Faraday Demo is on screen.
    pub fn showing_demo(&self) -> bool {
        self.demo.is_some()
    }

    /// The Demo's host while it is on screen, for the shell to serve its
    /// app's sticks and storage.
    pub fn demo(&mut self) -> Option<&mut Host> {
        self.demo.as_mut().map(|d| &mut d.host)
    }

    /// Leaves Faraday Demo for Home: the bar's way back, the window's
    /// close button, or the Demo powered off. Its session ends with it;
    /// the theme it was in stays the Starter's.
    pub fn close_demo(&mut self) {
        if self.demo.take().is_none() {
            return;
        }
        self.out.push_back(StarterCommand::DemoClosed);
        self.pressed = None;
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// The Demo's next command for the shell, its frames included. A
    /// theme the Demo changed to is taken first.
    pub fn poll_demo_command(&mut self) -> Option<Command> {
        self.demo_theme();
        self.demo.as_mut()?.host.poll_command()
    }

    /// Where the Demo's app offers `action` in its last frame, in the
    /// window's pixels, for tests that touch it. A lookup.
    pub fn where_demo_offers(&self, action: Action) -> Option<(u16, u16)> {
        let held = self.demo.as_ref()?;
        let (x, y) = held.host.app.where_offered(action)?;
        let (hx, hy, _, _) = held.host.app_area()?;
        let (ax, ay, _, _) = self.demo_area()?;
        let px = |v: i32| v.clamp(0, i32::from(u16::MAX)) as u16;
        Some((px(i32::from(x) + hx + ax), px(i32::from(y) + hy + ay)))
    }

    /// The words the Demo's last frame draws: its strip's and its app's.
    pub(crate) fn demo_texts(&mut self) -> Vec<String> {
        let Some(held) = self.demo.as_mut() else {
            return Vec::new();
        };
        let _ = held.host.frame();
        let mut texts = held.host.strip_texts().to_vec();
        texts.extend(held.host.app.drawn_texts());
        texts
    }

    /// A press on the Starter's bar while the Demo is on screen: the way
    /// back leaves it; nothing else is offered there. Returns whether the
    /// press was the Demo's to take.
    pub(crate) fn demo_act(&mut self, action: StarterAction) -> bool {
        if self.demo.is_none() {
            return false;
        }
        if action == StarterAction::Back {
            self.close_demo();
        }
        true
    }

    /// The theme and the text size the Demo is in, when either changed
    /// there (its Settings, or the boot stick's settings file it read):
    /// the Starter's from now on.
    fn demo_theme(&mut self) {
        if let Some(t) = self.demo.as_ref().map(|d| d.host.app.theme())
            && t != self.theme
        {
            self.set_theme(t);
        }
        if let Some(t) = self.demo.as_ref().map(|d| d.host.app.text_size())
            && t != self.text
        {
            self.set_text_size(t);
        }
    }

    /// Draws the Demo, while it is open, at `text`, laid out again for it.
    pub(crate) fn demo_text_at(&mut self, text: faraday_ui::TextSize) {
        if let Some(d) = self.demo.as_mut() {
            d.host.app.draw_text_at(text);
        }
    }

    /// Where the Demo's frame lies in the window, in pixels: under the top
    /// app bar, the window's whole width. A lookup.
    pub(crate) fn demo_area(&self) -> Option<(i32, i32, u16, u16)> {
        let d = self.window?;
        let y = (faraday_draw::tokens::APP_BAR_H * self.f).round() as i32;
        let h = i32::from(d.height) - y;
        (d.width > 0 && h > 0).then_some((0, y, d.width, h as u16))
    }

    /// Tells the Demo's host its size when it changed ([`Held::size`]).
    pub(crate) fn fit_demo(&mut self) {
        let (Some(d), Some((_, _, w, h))) = (self.window, self.demo_area()) else {
            return;
        };
        let Some(held) = self.demo.as_mut() else {
            return;
        };
        if held.size == Some((w, h)) {
            return;
        }
        held.size = Some((w, h));
        held.host.event(Event::Display(DisplayInfo {
            width: w,
            height: h,
            ..d
        }));
    }

    /// An event while the Demo is on screen: what falls on its frame,
    /// moved to its own corner, and every key, way back and answer the
    /// shell brings are its; the Starter keeps what falls on its own bar
    /// and the window's own events. Returns the event when it is the
    /// Starter's.
    pub(crate) fn demo_event(&mut self, event: Event) -> Option<Event> {
        let Some((ax, ay, aw, ah)) = self.demo_area() else {
            return Some(event);
        };
        let held = self.demo.as_mut()?;
        let inside = |x: u16, y: u16| {
            let (x, y) = (i32::from(x), i32::from(y));
            x >= ax && y >= ay && x < ax + i32::from(aw) && y < ay + i32::from(ah)
        };
        let moved = |x: u16, y: u16| {
            (
                (i32::from(x) - ax).clamp(0, i32::from(u16::MAX)) as u16,
                (i32::from(y) - ay).clamp(0, i32::from(u16::MAX)) as u16,
            )
        };
        let to_demo = match event {
            Event::Display(_) => return Some(event),
            Event::Tick { .. } => {
                held.host.event(event.clone());
                self.demo_theme();
                return Some(event);
            }
            Event::Touch { x, y, phase } => {
                if phase == TouchPhase::Down {
                    held.touch = inside(x, y);
                }
                if !held.touch {
                    return Some(event);
                }
                if phase == TouchPhase::Up {
                    held.touch = false;
                }
                let (x, y) = moved(x, y);
                Event::Touch { x, y, phase }
            }
            Event::Hover { x, y } if inside(x, y) => {
                let (x, y) = moved(x, y);
                Event::Hover { x, y }
            }
            Event::Hover { .. } => {
                held.host.event(Event::HoverEnd);
                return Some(event);
            }
            Event::Wheel { x, y, dy } if inside(x, y) => {
                let (x, y) = moved(x, y);
                Event::Wheel { x, y, dy }
            }
            Event::Scroll { x, y, dy } if inside(x, y) => {
                let (x, y) = moved(x, y);
                Event::Scroll { x, y, dy }
            }
            Event::ScrollEnd { x, y } => {
                let (x, y) = moved(x, y);
                Event::ScrollEnd { x, y }
            }
            // A turn of the wheel over the Starter's own bar moves nothing.
            Event::Wheel { .. } | Event::Scroll { .. } => return None,
            other => other,
        };
        held.host.event(to_demo);
        self.demo_theme();
        None
    }

    /// Costs the Demo's frame, drawn by its host, and a copy of its pixels
    /// under the Starter's bar, which is drawn over it.
    pub(crate) fn compose_demo(&mut self) {
        self.dirty = false;
        if let Some((x, y, _, _)) = self.demo_area()
            && let (Some(held), Some(canvas)) = (self.demo.as_mut(), self.canvas.as_mut())
        {
            canvas.place(x, y, &held.host.frame());
        }
        self.render();
    }
}
