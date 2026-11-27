use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    boot_import: faraday_boot_import::State,
    #[lock(kept)]
    stick_visit: faraday_stick_visit::State,
    #[lock(kept)]
    zeroing: faraday_zeroing::State,
}
