use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct OtsCheck {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<OtsCheck>;

impl OtsCheck {
    pub(crate) fn new() -> OtsCheck {
        OtsCheck { skeleton: () }
    }
}
