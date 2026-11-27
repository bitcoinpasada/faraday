use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct Krux {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<Krux>;

impl Krux {
    pub(crate) fn new() -> Krux {
        Krux { skeleton: () }
    }
}
