use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    seed_checks: faraday_seed_checks::State,
}
