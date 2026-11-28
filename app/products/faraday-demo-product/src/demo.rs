use faraday_bip::keys::Network;
use faraday_kernel::{Carried, Facts, Mode, Public, Show, View};
use faraday_organisms::Ui;
use faraday_port::event::Input;
use faraday_runtime::{Addr, Nothing, Root, Stream, Tops};
use faraday_templates::TextSize;

use crate::Press;

/// The product value: `impl faraday_runtime::Root`; `App = faraday_online::Online`.
pub struct Demo {
    options: DemoOptions,
}

/// `--text` (the starting text size) and `--transfer` (Transfer shown first).
#[derive(Public)]
pub struct DemoOptions {
    pub text: Option<TextSize>,
    pub transfer: bool,
}

/// S9: Practice (the default; Testnet), Testnet, Signet, Regtest; never Mainnet.
pub const MODES: &[Mode] = &[
    Mode {
        network: Network::Testnet,
        practice: true,
        online: true,
        samples: None,
    },
    Mode {
        network: Network::Testnet,
        practice: false,
        online: true,
        samples: None,
    },
    Mode {
        network: Network::Signet,
        practice: false,
        online: true,
        samples: None,
    },
    Mode {
        network: Network::Regtest,
        practice: false,
        online: true,
        samples: None,
    },
];

impl Root for Demo {
    type Settings = faraday_settings::Settings;
    type Session = faraday_session::Session;
    type Vaults = faraday_vaults::Vaults;
    type App = faraday_online::Online;
    type Outside = Nothing;
    type Press = Press;
    type Options = DemoOptions;
    const OUTSIDE: bool = false;

    fn mode(&self, _carried: &mut Carried, _facts: &Facts) -> Mode {
        todo!()
    }

    fn press(_press: Press) -> Addr<Self> {
        todo!()
    }

    fn input(&self, _tops: &Tops<'_, Self>, _input: &Input) -> Option<Addr<Self>> {
        todo!()
    }

    fn fact(&self, _tops: &Tops<'_, Self>, _fact: Stream) -> Option<Addr<Self>> {
        todo!()
    }

    fn close_asked(&self) -> Option<Addr<Self>> {
        todo!()
    }

    fn frame(
        &self,
        _tops: &Tops<'_, Self>,
        _view: &View<'_>,
        _show: &Show<'_>,
        _ui: &mut Ui<'_, Press>,
    ) {
        todo!()
    }
}
