//! A page scrolled on the Pi as a person does it: the panel shows the
//! frame after every frame put out, whatever the gesture.

use std::sync::Mutex;

use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware, TouchPhase};

use crate::fb::{self, Depth, Geometry};
use crate::orient::Turn;
use crate::panel::{Panel, lock};
use crate::present::{Known, Presenter};

fn panel_of(depth: Depth) -> Geometry {
    Geometry {
        width: 480,
        height: 640,
        depth,
        stride: 480 * depth.bytes_per_pixel(),
    }
}

fn core_at(dpi: u16, screen: Screen) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(screen));
    app
}

/// A small deterministic generator.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }
}

struct Rig {
    g: Geometry,
    app: Faraday,
    p: Presenter,
    panel: Mutex<Panel<Vec<u8>>>,
    now: u64,
    frames: u32,
    moved: u32,
    shown: u32,
    before: Vec<u8>,
}

impl Rig {
    fn new(g: Geometry, dpi: u16, screen: Screen) -> Rig {
        Rig {
            g,
            app: core_at(dpi, screen),
            p: Presenter::default(),
            panel: Mutex::new(Panel::new(Vec::new(), g)),
            now: 2000,
            frames: 0,
            moved: 0,
            shown: 0,
            before: Vec::new(),
        }
    }

    /// Delivers `event` and then a tick, and puts out a frame as the
    /// shell's loop does if the core asked for one; says where the panel
    /// differs from the frame, if it does.
    fn step(&mut self, event: Option<Event>, tag: &str) -> Result<bool, String> {
        if let Some(e) = event {
            self.app.event(e);
        }
        self.now += 16;
        self.app.event(Event::Tick { now_ms: self.now });
        let mut draw = false;
        while let Some(c) = self.app.poll_command() {
            draw |= matches!(c, Command::Draw);
        }
        if !draw {
            return Ok(false);
        }
        let parts = self.app.parts();
        let (moved, changed) = (parts.moved, parts.changed);
        let rgba = parts.base.rgba.to_vec();
        self.p
            .present_parts(
                &rgba,
                Known { moved, changed },
                self.g,
                Turn::R0,
                &self.panel,
                None,
            )
            .expect("written");
        self.frames += 1;
        self.moved += u32::from(moved.is_some());
        self.shown += u32::from(self.before != rgba);
        self.before.clone_from(&rgba);
        let mut want = vec![0u8; self.g.row_bytes() * usize::from(self.g.height)];
        fb::convert_into(&rgba, self.g.depth, &mut want);
        let held = lock(&self.panel).sink().clone();
        let row = self.g.row_bytes();
        match (0..usize::from(self.g.height))
            .find(|&r| held[r * row..][..row] != want[r * row..][..row])
        {
            None => Ok(true),
            Some(r) => Err(format!(
                "{tag}: frame {}: row {r} of the panel differs from the frame (moved {moved:?}, changed {changed:?})",
                self.frames
            )),
        }
    }
}

/// A gesture of the hand on the page: a drag of `n` moves of `d` rows
/// each, a lift, and the page coasting or settling for `rest` ticks.
fn gesture(rig: &mut Rig, rng: &mut Rng, tag: &str) -> Result<(), String> {
    let x = 240u16;
    let mut y = 100 + rng.below(450) as u16;
    let (n, per) = (1 + rng.below(14), 2 + rng.below(40) as i32);
    let up = rng.below(2) == 0;
    let touch = |y: u16, phase| Event::Touch { x, y, phase };
    rig.step(Some(touch(y, TouchPhase::Down)), tag)?;
    for _ in 0..n {
        let by = per + rng.below(6) as i32;
        let ny = (i32::from(y) + if up { -by } else { by }).clamp(5, 630);
        y = ny as u16;
        rig.step(Some(touch(y, TouchPhase::Move)), tag)?;
        if rng.below(3) == 0 {
            rig.step(None, tag)?;
        }
    }
    rig.step(Some(touch(y, TouchPhase::Up)), tag)?;
    for _ in 0..rng.below(90) {
        rig.step(None, tag)?;
    }
    Ok(())
}

/// A touchpad's or wheel's burst: `n` steps of the page, then the lift.
fn wheel(rig: &mut Rig, rng: &mut Rng, tag: &str) -> Result<(), String> {
    let sign = if rng.below(2) == 0 { 1 } else { -1 };
    for _ in 0..1 + rng.below(8) {
        let dy = sign * (1 + rng.below(70) as i16);
        rig.step(Some(Event::Scroll { x: 240, y: 320, dy }), tag)?;
        for _ in 0..rng.below(3) {
            rig.step(None, tag)?;
        }
    }
    rig.step(Some(Event::ScrollEnd { x: 240, y: 320 }), tag)?;
    for _ in 0..rng.below(90) {
        rig.step(None, tag)?;
    }
    Ok(())
}

fn run(
    depth: Depth,
    dpi: u16,
    screen: Screen,
    seed: u64,
) -> (u32, u32, u32, (u32, u32), Result<(), String>) {
    let mut rig = Rig::new(panel_of(depth), dpi, screen);
    let mut rng = Rng(seed);
    let tag = format!("{depth:?} dpi {dpi} {screen:?} seed {seed}");
    let mut idle = (0, 0);
    let result = (|| {
        for _ in 0..40 {
            rig.step(None, &tag)?;
        }
        idle.0 = rig.frames;
        for _ in 0..10 {
            if seed.is_multiple_of(2) {
                gesture(&mut rig, &mut rng, &tag)?;
            } else {
                wheel(&mut rig, &mut rng, &tag)?;
            }
            if rng.below(4) == 0 {
                let dy = rng.below(120) as i16 - 60;
                rig.step(Some(Event::Scroll { x: 240, y: 320, dy }), &tag)?;
            }
        }
        let from = rig.frames;
        for _ in 0..60 {
            rig.step(None, &tag)?;
        }
        idle.1 = rig.frames - from;
        Ok(())
    })();
    (rig.frames, rig.moved, rig.shown, idle, result)
}

#[test]
fn the_panel_shows_the_frame_after_every_scroll() {
    let (mut scrolled, mut moved) = (0, 0);
    for screen in [Screen::Home, Screen::Settings] {
        for depth in [Depth::Rgb565, Depth::Rgba8888] {
            for seed in 1..=2 {
                let (_, m, shown, _, result) = run(depth, 160, screen, seed);
                if let Err(e) = result {
                    panic!("{e}");
                }
                scrolled += shown;
                moved += m;
            }
        }
    }
    assert!(scrolled > 100, "the pages scrolled: {scrolled} frames");
    assert!(moved > 0, "a scroll came as a moved frame");
}
