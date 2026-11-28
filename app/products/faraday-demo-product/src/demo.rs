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

    /// A mode change shows the Lock sheet, which asks first (the device composite's, inside the
    /// online app). Every mode in `MODES` is one the person may ask for; the root falls back to
    /// the default for a mode not on the list.
    fn mode_lock_asked(&self) -> Option<Addr<Self>> {
        Some(Addr::App(faraday_online::Msg::Device(
            faraday_device::Msg::Power(faraday_power::Msg::ModeLock),
        )))
    }

    /// The mode change's Lock sheet closed without locking: the asked mode is given up.
    fn mode_cancel_asked(&self) -> Option<Addr<Self>> {
        Some(Addr::Settings(faraday_settings::Msg::Press(
            faraday_settings::Press::ModeCancel,
        )))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: Faraday Demo asks the Lock sheet for a mode change and gives the pick up when that sheet closes without locking, as the device does.
    /// Regression: The hooks stay at their default, so a mode pick arms the settings owner with no sheet shown and the next plain lock changes the mode.
    /// Owner: app
    #[test]
    fn a_mode_change_asks_the_lock_sheet_and_a_closed_sheet_cancels_it() {
        let root = Demo { options: DemoOptions { text: None, transfer: false } };
        assert!(matches!(
            root.mode_lock_asked(),
            Some(Addr::App(faraday_online::Msg::Device(faraday_device::Msg::Power(
                faraday_power::Msg::ModeLock
            ))))
        ));
        assert!(matches!(
            root.mode_cancel_asked(),
            Some(Addr::Settings(faraday_settings::Msg::Press(
                faraday_settings::Press::ModeCancel
            )))
        ));
    }
}
