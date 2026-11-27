use faraday_kernel::{AtLock, Carried, Ctx, Feature};

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    calculators: faraday_calculators::State,
    #[lock(kept)]
    lightning: faraday_lightning::State,
    #[lock(kept)]
    ots_check: faraday_ots_check::State,
}

impl State {
    /// Every child started from what the last build carried; what a child's start tells is
    /// nothing yet.
    pub(crate) fn start(carried: &mut Carried, cx: &Ctx<'_>) -> State {
        State {
            calculators: <faraday_calculators::Calculators as Feature>::start(carried, cx).0,
            lightning: <faraday_lightning::Lightning as Feature>::start(carried, cx).0,
            ots_check: <faraday_ots_check::OtsCheck as Feature>::start(carried, cx).0,
        }
    }
}
