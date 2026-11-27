use faraday_kernel::{AtLock, Carried, Ctx, Feature};

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    spend: faraday_spend::State,
    #[lock(kept)]
    decode: faraday_decode::State,
    #[lock(kept)]
    messages: faraday_messages::State,
}

impl State {
    /// Every child started from what the last build carried; what a child's start tells is
    /// nothing yet.
    pub(crate) fn start(carried: &mut Carried, cx: &Ctx<'_>) -> State {
        State {
            spend: <faraday_spend::Spend as Feature>::start(carried, cx).0,
            decode: <faraday_decode::Decode as Feature>::start(carried, cx).0,
            messages: <faraday_messages::Messages as Feature>::start(carried, cx).0,
        }
    }
}
