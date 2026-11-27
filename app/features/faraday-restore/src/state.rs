use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct Restore {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<Restore>;

impl Restore {
    pub(crate) fn new() -> Restore {
        Restore { skeleton: () }
    }
}
