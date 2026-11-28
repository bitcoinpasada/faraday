//! The device product run whole: the runtime over the fixture adapters on the small panel, a
//! person's touches and keys in, the page and the sheets the frame shows out.

use std::path::Path;
use std::sync::Arc;

use faraday_fixture_adapters::{Fixtures, Scripted};
use faraday_port::event::{Event, Input};
use faraday_port::{Adapters, Waker};
use faraday_present::Pressed as _;
use faraday_runtime::{Phase, Runtime, Workers};

use crate::{AppPress, Device, Page, Press};

struct NoWake;

impl Waker for NoWake {
    fn wake(&self) {}
}

/// The device's runtime on a 480 by 640 display at 286 dpi, which is the small panel: Home is
/// the Pi's menu.
struct Run {
    runtime: Runtime<Device>,
    fixtures: Fixtures,
}

impl Run {
    fn start() -> Run {
        let mut fixtures = Fixtures::device(Path::new(""));
        fixtures.display_of(480, 640, 286);
        let runtime = Runtime::new(Device, Arc::new(NoWake), Workers::Inline);
        let mut run = Run { runtime, fixtures };
        run.settle();
        run
    }

    fn settle(&mut self) {
        for _ in 0..10_000 {
            let mut moved = false;
            while let Some(event) = self.fixtures.poll() {
                moved = true;
                for effect in self.runtime.event(event) {
                    self.fixtures.carry(effect);
                }
            }
            for effect in self.runtime.woken() {
                moved = true;
                self.fixtures.carry(effect);
            }
            if !moved {
                break;
            }
        }
        // The frame drawn, so its hits are the ones a touch meets.
        let _ = self.runtime.frame();
    }

    /// Scrolls the page by wheel notches until the control is drawn.
    fn scroll_to(&mut self, press: &Press) {
        for _ in 0..60 {
            if self.runtime.hits().iter().any(|h| h.press == *press) {
                return;
            }
            self.input(Input::Wheel {
                at: faraday_port::values::Point { x: 240, y: 320 },
                dy: 40,
            });
            // The scroll glides over the frames after it.
            for _ in 0..4 {
                self.fixtures.advance(100);
                self.settle();
            }
        }
    }

    /// Touches the control of the top layer that sends `press`.
    fn touch(&mut self, press: &Press) {
        let top = self.runtime.hits().iter().map(|h| h.layer).max();
        let hit = self
            .runtime
            .hits()
            .iter()
            .find(|h| Some(h.layer) == top && h.press == *press)
            .unwrap_or_else(|| {
                let mut kinds: Vec<_> = self
                    .runtime
                    .hits()
                    .iter()
                    .map(|h| (h.layer, h.press.kind()))
                    .collect();
                kinds.dedup();
                panic!("the control is not on the screen: {kinds:?}")
            });
        let at = faraday_port::values::Point {
            x: hit.rect.x + hit.rect.w / 2,
            y: hit.rect.y + hit.rect.h / 2,
        };
        self.fixtures.input(&[Scripted::Touch(at)]);
        self.settle();
    }

    fn input(&mut self, input: Input) {
        for effect in self.runtime.event(Event::Input(input)) {
            self.fixtures.carry(effect);
        }
        self.settle();
    }

    fn page(&self) -> (&'static str, u32) {
        let page = self.runtime.seen().page;
        (page.scope, page.n)
    }

    fn sheets(&self) -> usize {
        self.runtime.seen().sheets.len()
    }
}

fn settings_page() -> (&'static str, u32) {
    let a = faraday_settings::anchor(faraday_settings::Page::Settings);
    (a.scope, a.n)
}

fn set_time_page() -> (&'static str, u32) {
    let a = faraday_settings::anchor(faraday_settings::Page::SetTime);
    (a.scope, a.n)
}

/// Home's Settings tile, then the Settings page.
fn at_settings() -> Run {
    let mut run = Run::start();
    run.touch(&Press::App(AppPress::Device(faraday_device::Press::Home(
        faraday_home::Press::Place(faraday_home::Place::Settings),
    ))));
    assert_eq!(run.page(), settings_page());
    run
}

fn open_set_time(run: &mut Run) {
    let open = Press::Settings(faraday_settings::Press::OpenSetTime);
    run.scroll_to(&open);
    run.touch(&open);
    assert_eq!(run.page(), set_time_page());
}

/// Invariant: With the Power sheet up over Set the time, Back puts the sheet away and the page stays; the next Back goes to Settings.
/// Regression: Back goes to the settings owner at once and leaves the Power sheet up over Settings, or the keys typed with a sheet up go into Set the time's fields.
/// Owner: app
#[test]
fn back_on_set_the_time_puts_a_sheet_away_first() {
    let mut run = at_settings();
    open_set_time(&mut run);
    run.touch(&Press::App(AppPress::Power));
    assert_eq!(run.sheets(), 1);
    run.input(Input::Back);
    assert_eq!(run.sheets(), 0);
    assert_eq!(run.page(), set_time_page());
    run.input(Input::Back);
    assert_eq!(run.page(), settings_page());
}

/// Invariant: Settings opened again after leaving Set the time by the bar is Settings, not Set the time with what was typed.
/// Regression: The frame shows Settings while the settings owner keeps Set the time, so the page drawn is the owner's and not the one the navigation names.
/// Owner: app
#[test]
fn settings_opens_on_settings_after_set_the_time_was_left_by_the_bar() {
    let mut run = at_settings();
    open_set_time(&mut run);
    run.touch(&Press::App(AppPress::Open(Page::Device(
        faraday_device::Page::Home,
    ))));
    run.touch(&Press::App(AppPress::Device(faraday_device::Press::Home(
        faraday_home::Press::Place(faraday_home::Place::Settings),
    ))));
    assert_eq!(run.page(), settings_page());
}

/// Invariant: A mode change's Lock and use runs the lock to its end: the vaults owner answers the runtime's Close, the carry is kept and the process ends with the lock.
/// Regression: The lock waits for the vaults owner's answer to its Close and never reaches its keep, so the process does not end.
/// Owner: app
#[test]
fn a_mode_change_lock_finishes() {
    let mut run = at_settings();
    run.touch(&Press::Settings(faraday_settings::Press::ModeAsk(
        faraday_settings::ModeAsked::Testnet,
    )));
    assert_eq!(run.sheets(), 1);
    run.touch(&Press::App(AppPress::Device(faraday_device::Press::Power(
        faraday_power::Press::Lock,
    ))));
    assert_eq!(run.runtime.phase(), Phase::Ended);
    assert!(run.fixtures.kept().is_some());
}
