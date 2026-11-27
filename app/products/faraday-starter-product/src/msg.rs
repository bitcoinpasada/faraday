use faraday_kernel::Message;
use faraday_present::Pressed;

#[derive(Message)]
pub enum Msg {
    Starter(faraday_starter::Msg),
    Press(faraday_starter::Press),
}

impl From<faraday_starter::Press> for Msg {
    fn from(_press: faraday_starter::Press) -> Msg {
        todo!()
    }
}

/// One variant per top state's press (`Root::Press`, 06 §4.2).
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Settings(faraday_settings::Press),
    Session(faraday_session::Press),
    Vaults(faraday_vaults::Press),
    App(faraday_online::Press),
    Outside(faraday_starter::Press),
}
