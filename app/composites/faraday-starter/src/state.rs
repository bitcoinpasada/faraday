use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    home: faraday_starter_home::State,
    #[lock(kept)]
    install: faraday_install::State,
    #[lock(kept)]
    appearance: faraday_starter_appearance::State,
    #[lock(kept)]
    templates: faraday_print_templates::State,
    #[lock(kept)]
    guide: faraday_starter_guide::State,
    #[lock(kept)]
    make: faraday_make_stick::State,
    #[lock(kept)]
    start: faraday_starter_start::State,
    #[lock(kept)]
    transfer: faraday_transfer::State,
    #[lock(kept)]
    scan: faraday_scan::State,
}
