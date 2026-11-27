use faraday_organisms::Pressed;

/// One variant per top state's press (`Root::Press`, 06 §4.2).
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Settings(faraday_settings::Press),
    Session(faraday_session::Press),
    Vaults(faraday_vaults::Press),
    App(faraday_online::Press),
}
