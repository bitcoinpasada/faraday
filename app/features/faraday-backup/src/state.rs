use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct BackUp {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<BackUp>;

impl BackUp {
    pub(crate) fn new() -> BackUp {
        BackUp { skeleton: () }
    }
}
