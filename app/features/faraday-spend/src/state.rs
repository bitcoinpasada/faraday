use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct Spend {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<Spend>;

impl Spend {
    pub(crate) fn new() -> Spend {
        Spend { skeleton: () }
    }
}
