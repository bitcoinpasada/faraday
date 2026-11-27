use faraday_kernel::{AtLock, Carried, Ctx, Feature};

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    seed_checks: faraday_seed_checks::State,
}

impl State {
    /// Every child started from what the last build carried; what a child's start tells is
    /// nothing yet.
    pub(crate) fn start(carried: &mut Carried, cx: &Ctx<'_>) -> State {
        State {
            seed_checks: <faraday_seed_checks::SeedChecks as Feature>::start(carried, cx).0,
        }
    }
}
