use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    skeleton: (),
}

impl State {
    pub(crate) fn new() -> State {
        State { skeleton: () }
    }
}
