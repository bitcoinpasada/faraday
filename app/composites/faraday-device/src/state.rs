use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    home: faraday_home::State,
    #[lock(kept)]
    power: faraday_power::State,
    #[lock(kept)]
    devices: faraday_devices::State,
    #[lock(kept)]
    scan: faraday_scan::State,
    #[lock(kept)]
    guide: faraday_guide_sheet::State,
    #[lock(kept)]
    keys: faraday_keys_area::State,
    #[lock(kept)]
    wallets: faraday_wallets_area::State,
    #[lock(kept)]
    signing: faraday_signing_area::State,
    #[lock(kept)]
    checks: faraday_checks_area::State,
    #[lock(kept)]
    tools: faraday_tools_area::State,
    #[lock(kept)]
    files: faraday_files_area::State,
}
