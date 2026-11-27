use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct Convert {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<Convert>;

impl Convert {
    pub(crate) fn new() -> Convert {
        Convert { skeleton: () }
    }
}
