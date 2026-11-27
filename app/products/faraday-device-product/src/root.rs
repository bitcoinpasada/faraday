use faraday_bip::keys::Network;
use faraday_kernel::{Carried, Facts, Mode, Public, Show, View};
use faraday_organisms::Ui;
use faraday_port::event::Input;
use faraday_runtime::{Addr, Nothing, Root, Stream, Tops};

use crate::{App, Press};

/// The product value: `impl faraday_runtime::Root`.
pub struct Device;

/// The device app takes no options (06 §2).
// Spec: 09 §4.8 gives `type Options = ()`, which the kernel's `Public` does not cover.
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
