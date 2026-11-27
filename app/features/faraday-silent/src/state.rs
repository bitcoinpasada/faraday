use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct Silent {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<Silent>;

impl Silent {
    pub(crate) fn new() -> Silent {
        Silent { skeleton: () }
    }
}
