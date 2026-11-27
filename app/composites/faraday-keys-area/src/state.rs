use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    keygen: faraday_keygen::State,
    #[lock(kept)]
    randomness: faraday_randomness::State,
    #[lock(kept)]
    codex_shares: faraday_codex_shares::State,
    #[lock(kept)]
    add_key: faraday_add_key::State,
    #[lock(kept)]
    krux: faraday_krux::State,
    #[lock(kept)]
    bip85: faraday_bip85::State,
    #[lock(kept)]
    convert: faraday_convert::State,
    #[lock(kept)]
    vanity: faraday_vanity::State,
    #[lock(kept)]
    secure_boot: faraday_secure_boot::State,
}
