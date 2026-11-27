use faraday_kernel::{AtLock, Carried, Ctx, Feature};

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

impl State {
    /// Every child started from what the last build carried; what a child's start tells is
    /// nothing yet.
    pub(crate) fn start(carried: &mut Carried, cx: &Ctx<'_>) -> State {
        State {
            keygen: <faraday_keygen::KeyGen as Feature>::start(carried, cx).0,
            randomness: <faraday_randomness::Randomness as Feature>::start(carried, cx).0,
            codex_shares: <faraday_codex_shares::CodexShares as Feature>::start(carried, cx).0,
            add_key: <faraday_add_key::AddKey as Feature>::start(carried, cx).0,
            krux: <faraday_krux::Krux as Feature>::start(carried, cx).0,
            bip85: <faraday_bip85::Bip85 as Feature>::start(carried, cx).0,
            convert: <faraday_convert::Convert as Feature>::start(carried, cx).0,
            vanity: <faraday_vanity::Vanity as Feature>::start(carried, cx).0,
            secure_boot: <faraday_secure_boot::SecureBoot as Feature>::start(carried, cx).0,
        }
    }
}
