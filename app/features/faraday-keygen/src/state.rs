use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct KeyGen {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<KeyGen>;

impl KeyGen {
    pub(crate) fn new() -> KeyGen {
        KeyGen { skeleton: () }
    }
}
