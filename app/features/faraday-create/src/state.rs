use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct MakeWallet {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<MakeWallet>;

impl MakeWallet {
    pub(crate) fn new() -> MakeWallet {
        MakeWallet { skeleton: () }
    }
}
