use faraday_kernel::{AtLock, Carried, Ctx, Feature};

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    boot_import: faraday_boot_import::State,
    #[lock(kept)]
    stick_visit: faraday_stick_visit::State,
    #[lock(kept)]
    zeroing: faraday_zeroing::State,
}

impl State {
    /// Every child started from what the last build carried; what a child's start tells is
    /// nothing yet.
    pub(crate) fn start(carried: &mut Carried, cx: &Ctx<'_>) -> State {
        State {
            boot_import: <faraday_boot_import::BootImport as Feature>::start(carried, cx).0,
            stick_visit: <faraday_stick_visit::StickVisit as Feature>::start(carried, cx).0,
            zeroing: <faraday_zeroing::Zeroing as Feature>::start(carried, cx).0,
        }
    }
}
