use faraday_bip::keys::Network;
use faraday_kernel::{Carried, Facts, Mode, Public, Show, View};
use faraday_organisms::Ui;
use faraday_port::event::{Input, Key};
use faraday_runtime::{Addr, Nothing, Root, Stream, Tops};

use crate::{App, AppPress, Msg, Page, Press};

/// The product value: `impl faraday_runtime::Root`.
pub struct Device;

/// The device app takes no options (06 §2).
#[derive(Public)]
pub struct NoOptions;

/// S9: Mainnet, Testnet, Signet, Regtest.
pub const MODES: &[Mode] = &[
    Mode {
        network: Network::Mainnet,
        practice: false,
        online: false,
        samples: None,
    },
    Mode {
        network: Network::Testnet,
        practice: false,
        online: false,
        samples: None,
    },
    Mode {
        network: Network::Signet,
        practice: false,
        online: false,
        samples: None,
    },
    Mode {
        network: Network::Regtest,
        practice: false,
        online: false,
        samples: None,
    },
];

impl Root for Device {
    type Settings = faraday_settings::Settings;
    type Session = faraday_session::Session;
    type Vaults = faraday_vaults::Vaults;
    type App = App;
    type Outside = Nothing;
    type Press = Press;
    type Options = NoOptions;
    const OUTSIDE: bool = false;

    fn mode(&self, _carried: &mut Carried, _facts: &Facts) -> Mode {
        // The mode a lock carries is not built: a build starts on Mainnet (06 §5.1).
        MODES[0]
    }

    fn press(press: Press) -> Addr<Self> {
        match press {
            // Settings' Lock and Power off buttons are the power feature's (06 §4.3).
            Press::Settings(faraday_settings::Press::Lock) => Addr::App(Msg::Device(
                faraday_device::Msg::Power(faraday_power::Msg::Lock { asks_first: true }),
            )),
            Press::Settings(faraday_settings::Press::PowerAsk) => Addr::App(Msg::Device(
                faraday_device::Msg::Power(faraday_power::Msg::Show(faraday_power::Which::Power)),
            )),
            Press::Settings(p) => Addr::Settings(p.into()),
            Press::Session(p) => Addr::Session(p.into()),
            Press::Vaults(p) => Addr::Vaults(p.into()),
            Press::App(p) => Addr::App(p.into()),
        }
    }

    fn input(&self, tops: &Tops<'_, Self>, input: &Input) -> Option<Addr<Self>> {
        let app = tops.app()?;
        let home_shown = app.frame().sheets().is_empty()
            && app.frame().nav().shown() == Page::Device(faraday_device::Page::Home);
        let home = |m| Addr::App(Msg::Device(faraday_device::Msg::Home(m)));
        // Typing on Set the time goes to its fields (06 §4.3).
        let set_time = matches!(app.frame().nav().shown(), Page::Settings(_))
            && tops.settings().page() == faraday_settings::Page::SetTime;
        match input {
            Input::Key(Key::Char(c)) if set_time => {
                Some(Addr::Settings(faraday_settings::Msg::Typed(*c)))
            }
            Input::Key(Key::Backspace) if set_time => {
                Some(Addr::Settings(faraday_settings::Msg::Erase))
            }
            Input::Key(Key::Tab) if set_time => Some(Addr::Settings(faraday_settings::Msg::Tab)),
            Input::Key(Key::Enter) if set_time => {
                Some(Addr::Settings(faraday_settings::Msg::Enter))
            }
            Input::Key(Key::Escape) if set_time => {
                Some(Addr::Settings(faraday_settings::Msg::Escape))
            }
            Input::Back | Input::Key(Key::Escape) => Some(Addr::App(AppPress::Back.into())),
            Input::Forward => Some(Addr::App(AppPress::Forward.into())),
            // Typing on Home goes to the bar's search (06 §4.3).
            Input::Key(Key::Char(c)) if home_shown => Some(home(faraday_home::Msg::Typed(*c))),
            Input::Key(Key::Backspace) if home_shown => Some(home(faraday_home::Msg::Erase)),
            _ => None,
        }
    }

    /// No stream fact has a feature to go to yet: the device check and the camera are not built.
    fn fact(&self, _tops: &Tops<'_, Self>, _fact: Stream) -> Option<Addr<Self>> {
        None
    }

    /// A mode change shows the Lock sheet, which asks first.
    fn mode_lock_asked(&self) -> Option<Addr<Self>> {
        Some(Addr::App(Msg::Device(faraday_device::Msg::Power(
            faraday_power::Msg::ModeLock,
        ))))
    }

    /// The mode change's Lock sheet closed: the asked mode is given up.
    fn mode_cancel_asked(&self) -> Option<Addr<Self>> {
        Some(Addr::Settings(faraday_settings::Msg::Press(
            faraday_settings::Press::ModeCancel,
        )))
    }

    /// The device has no window.
    fn close_asked(&self) -> Option<Addr<Self>> {
        None
    }

    fn frame(
        &self,
        tops: &Tops<'_, Self>,
        view: &View<'_>,
        show: &Show<'_>,
        ui: &mut Ui<'_, Press>,
    ) {
        let Some(app) = tops.app() else {
            return;
        };
        faraday_present::draw(&crate::present::screen(app, view, show), ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: Settings' Lock and Power off buttons reach the power feature, which asks first, and every other Settings press stays with the settings owner.
    /// Regression: Lock on Settings locks with no question, or Power off does nothing because the settings owner takes a press that is not its own.
    /// Owner: app
    #[test]
    fn settings_lock_and_power_off_go_to_the_power_feature() {
        let to = |p| <Device as Root>::press(Press::Settings(p));
        assert!(matches!(
            to(faraday_settings::Press::Lock),
            Addr::App(Msg::Device(faraday_device::Msg::Power(
                faraday_power::Msg::Lock { asks_first: true }
            )))
        ));
        assert!(matches!(
            to(faraday_settings::Press::PowerAsk),
            Addr::App(Msg::Device(faraday_device::Msg::Power(
                faraday_power::Msg::Show(faraday_power::Which::Power)
            )))
        ));
        assert!(matches!(
            to(faraday_settings::Press::OpenSetTime),
            Addr::Settings(_)
        ));
    }
}
