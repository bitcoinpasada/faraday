use faraday_kernel::{AtLock, Flow};

/// The flow's own state.
#[derive(AtLock)]
pub struct Bip85 {
    #[lock(public)]
    skeleton: (),
}

/// The state of the flow: the kernel's part and the flow's own.
pub type State = Flow<Bip85>;

impl Bip85 {
    pub(crate) fn new() -> Bip85 {
        Bip85 { skeleton: () }
    }
}
