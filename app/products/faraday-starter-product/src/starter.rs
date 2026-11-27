use faraday_bip::keys::Network;
use faraday_kernel::{Carried, Facts, Mode, Public, Show, View};
use faraday_organisms::Ui;
use faraday_port::event::Input;
use faraday_runtime::{Addr, Root, Stream, Tops};
use faraday_text::Text;

use crate::{Outside, Press};

/// The product value: `impl faraday_runtime::Root`; `App = Online`, `Outside = Outside`.
pub struct Starter {
    options: StarterOptions,
}

/// `--uninstall` (the Installed page with Uninstall asked) and `--image` (an image for
/// make-stick, as if dropped).
#[derive(Public)]
pub struct StarterOptions {
    pub uninstall: bool,
    pub image: Option<Text>,
}

/// S9: Practice (the default; Testnet), Testnet, Signet, Regtest; never Mainnet.
pub const MODES: &[Mode] = &[
    Mode {
        network: Network::Testnet,
        practice: true,
        online: true,
    },
    Mode {
        network: Network::Testnet,
        practice: false,
        online: true,
    },
    Mode {
        network: Network::Signet,
        practice: false,
        online: true,
    },
    Mode {
        network: Network::Regtest,
        practice: false,
        online: true,
    },
];

impl Root for Starter {
    type Settings = faraday_settings::Settings;
    type Session = faraday_session::Session;
    type Vaults = faraday_vaults::Vaults;
    type App = faraday_online::Online;
    type Outside = Outside;
    type Press = Press;
    type Options = StarterOptions;

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
