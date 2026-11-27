use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    skeleton: (),
}
