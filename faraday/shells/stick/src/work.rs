//! The core's worker and the main loop (`docs/WORKERS.md` §7.1): the
//! worker wakes the loop with a note when an answer is ready, and a job
//! that runs keeps the frames coming.

use std::sync::mpsc::SyncSender;

use faraday_core::Faraday;

use crate::Wake;

/// What the core calls from its worker's thread when an answer is ready:
/// a note to the loop. A full channel means the loop is behind and takes
/// the answer on its next pass anyway.
pub fn wake(tx: SyncSender<Wake>) -> Box<dyn Fn() + Send> {
    Box::new(move || {
        let _ = tx.try_send(Wake::Work);
    })
}

/// Whether a job runs on the worker.
pub fn running(app: Option<&Faraday>) -> bool {
    app.is_some_and(|a| a.busy().is_some())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::sync_channel;
    use std::time::{Duration, Instant};

    use faraday_core::testkit;
    use faraday_core::{Action, DESKTOP_MODES};
    use faraday_shell_api::{App, Event};

    use super::*;
    use crate::keyboard::{PANIC_HOLD, PanicTimer};

    #[test]
    fn the_panic_chord_powers_off_while_a_job_runs() {
        let (tx, rx) = sync_channel(8);
        let mut app = testkit::started();
        app.set_modes(DESKTOP_MODES);
        testkit::into_demo(&mut app);
        app.samples = Some(testkit::demo_samples());
        app.set_wake(wake(tx));
        app.press(Action::Samples);
        app.press(Action::SampleTick(faraday_core::samples::Pick::Key(0)));
        testkit::press_on_worker(&mut app, Action::SamplesLoad);
        assert!(running(Some(&app)));
        // The chord is timed by the loop, which does not wait on the job:
        // held for the full time while it runs, it is due.
        let t0 = Instant::now();
        let mut timer = PanicTimer::default();
        timer.chord(true, t0);
        assert!(!timer.due(t0 + PANIC_HOLD / 2));
        assert!(timer.due(t0 + PANIC_HOLD));
        // The answer wakes the loop, and its next pass applies it.
        assert_eq!(rx.recv_timeout(Duration::from_secs(120)), Ok(Wake::Work));
        app.event(Event::Tick { now_ms: 1000 });
        assert!(!running(Some(&app)));
    }
}
