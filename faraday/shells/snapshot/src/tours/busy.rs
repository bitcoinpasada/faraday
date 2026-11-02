//! The busy tour.

use crate::tour::Tour;
use faraday_core::samples::Pick;
use faraday_core::strings::EN;
use faraday_core::{Action, testkit};

/// The screen while a job runs (`docs/WORKERS.md` §4, `docs/DESIGN.md`
/// §4.14): Load samples with a wallet ticked, its buttons dimmed and
/// Material's linear progress indicator along the foot, moving where the
/// work has no count and filled by the share done where it has one. No
/// job runs: the tour holds the busy state for the picture.
pub(crate) fn busy_tour(t: &mut Tour) -> Result<(), String> {
    crate::tours::guide::demo_session(t);
    t.press(Action::Samples);
    t.press(Action::SampleTick(Pick::Wallet(0)));
    t.shot("busy-1-ready")?;
    testkit::show_busy(&mut t.app, EN.working_samples, 0, 0);
    t.shot_moving("busy-2-indeterminate", 700)?;
    testkit::show_busy(&mut t.app, EN.working_samples, 2, 5);
    t.shot_moving("busy-3-determinate", 0)?;
    testkit::end_busy(&mut t.app);
    t.shot("busy-4-done")
}
