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
    },
    Mode {
        network: Network::Testnet,
        practice: false,
        online: false,
    },
    Mode {
        network: Network::Signet,
        practice: false,
        online: false,
    },
    Mode {
        network: Network::Regtest,
        practice: false,
        online: false,
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
        match input {
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
