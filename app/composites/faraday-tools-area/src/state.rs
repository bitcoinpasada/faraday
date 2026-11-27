use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    calculators: faraday_calculators::State,
    #[lock(kept)]
    lightning: faraday_lightning::State,
    #[lock(kept)]
    ots_check: faraday_ots_check::State,
}
