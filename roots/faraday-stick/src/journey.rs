//! The device app's journeys (07 §2.5): the product's model and the runtime over the fixture
//! adapters, the same construction `main` runs over the real ones.

use std::path::Path;
use std::sync::Arc;

use faraday_device_product::{Device, Pressed as _};
use faraday_fixture_adapters::{Fixtures, Scripted};
use faraday_port::values::Point;
use faraday_port::{Adapters, Exit, Order, Waker};
use faraday_runtime::{Runtime, Workers};

struct NoWake;

impl Waker for NoWake {
    fn wake(&self) {}
}

/// The runtime started over fixtures with no stick, on the device's own panel.
fn started() -> (Runtime<Device>, Fixtures) {
    let mut fixtures = Fixtures::device(Path::new("no-kit"));
    fixtures.display_of(1024, 600, 170);
    let mut rt = Runtime::new(Device, Arc::new(NoWake), Workers::Inline);
    settle(&mut rt, &mut fixtures);
    (rt, fixtures)
}

/// Every event the fixtures hold, each answered by the runtime, until nothing moves.
fn settle(rt: &mut Runtime<Device>, fixtures: &mut Fixtures) {
    loop {
        let mut moved = false;
        while let Some(event) = fixtures.poll() {
            moved = true;
            for effect in rt.event(event) {
                fixtures.carry(effect);
            }
        }
        for effect in rt.woken() {
            moved = true;
            fixtures.carry(effect);
        }
        if !moved {
            break;
        }
    }
    // The frame is taken as the loop hands it to the panel.
    let _ = rt.frame();
}

/// Whether an `Exit` that `which` accepts was carried.
fn exited(fixtures: &Fixtures, which: fn(&Exit) -> bool) -> bool {
    fixtures
        .orders()
        .iter()
        .any(|order| matches!(order, Order::Exit(exit) if which(exit)))
}

/// A touch on the control of the top layer whose press has this kind.
fn press(rt: &mut Runtime<Device>, fixtures: &mut Fixtures, kind: &str) {
    let hits = rt.hits();
    let top = hits.iter().map(|h| h.layer).max();
    let hit = hits
        .iter()
        .filter(|h| Some(h.layer) == top)
        .find(|h| h.press.kind() == kind)
        .unwrap_or_else(|| panic!("no control with the press {kind}"));
    let at = Point {
        x: hit.rect.x + hit.rect.w / 2,
        y: hit.rect.y + hit.rect.h / 2,
    };
    fixtures.input(&[Scripted::Touch(at)]);
    settle(rt, fixtures);
}

/// Invariant: The device starts on Home, and Power off in the Power sheet from the top bar ends the run with the end order and no other exit.
/// Regression: The Power control or the sheet's Power off is not wired to the end, so the person cannot power the device off from its own screen.
/// Owner: root
#[test]
fn power_off_from_the_bar_ends_the_run() {
    let (mut rt, mut fixtures) = started();
    assert_eq!(rt.seen().page.scope, "home.Home");
    press(&mut rt, &mut fixtures, "faraday-device-product.Power");
    press(&mut rt, &mut fixtures, "faraday-power.PowerOff");
    assert!(fixtures.ended());
    assert!(exited(&fixtures, |e| matches!(e, Exit::End)));
    assert!(!exited(&fixtures, |e| matches!(e, Exit::RestartComputer)));
}

/// Invariant: Restart in the Power sheet from the top bar ends the run with the restart order, which the stick adapter turns into the restart status, and not with the power-off one.
/// Regression: Restart ends the run as Power off does, so the computer powers off when the person asked it to restart.
/// Owner: root
#[test]
fn restart_from_the_bar_ends_the_run_with_the_restart_order() {
    let (mut rt, mut fixtures) = started();
    press(&mut rt, &mut fixtures, "faraday-device-product.Power");
    press(&mut rt, &mut fixtures, "faraday-power.Restart");
    assert!(exited(&fixtures, |e| matches!(e, Exit::RestartComputer)));
}
