use faraday_kernel::AtLock;

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    spend: faraday_spend::State,
    #[lock(kept)]
    decode: faraday_decode::State,
    #[lock(kept)]
    messages: faraday_messages::State,
}
