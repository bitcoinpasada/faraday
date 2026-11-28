use faraday_kernel::{AtLock, Carried, Ctx, Effects, Feature};

use crate::{Msg, Out};

/// Every child's state.
#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    pub(crate) home: faraday_home::State,
    #[lock(kept)]
    pub(crate) power: faraday_power::State,
    #[lock(kept)]
    pub(crate) devices: faraday_devices::State,
    #[lock(kept)]
    scan: faraday_scan::State,
    #[lock(kept)]
    pub(crate) guide: faraday_guide_sheet::State,
    #[lock(kept)]
    keys: faraday_keys_area::State,
    #[lock(kept)]
    wallets: faraday_wallets_area::State,
    #[lock(kept)]
    signing: faraday_signing_area::State,
    #[lock(kept)]
    checks: faraday_checks_area::State,
    #[lock(kept)]
    tools: faraday_tools_area::State,
    #[lock(kept)]
    files: faraday_files_area::State,
}

impl State {
    /// Every child started from what the last build carried, and what Home and Power tell at
    /// start. The other children's starts tell nothing yet.
    pub(crate) fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (State, Effects<Msg, Out>) {
        let (home, home_fx) = <faraday_home::Home as Feature>::start(carried, cx);
        let (power, power_fx) = <faraday_power::Power as Feature>::start(carried, cx);
        let (devices, devices_fx) = <faraday_devices::Devices as Feature>::start(carried, cx);
        let state = State {
            home,
            power,
            devices,
            scan: <faraday_scan::Scan as Feature>::start(carried, cx).0,
            guide: <faraday_guide_sheet::GuideSheet as Feature>::start(carried, cx).0,
            keys: <faraday_keys_area::Keys as Feature>::start(carried, cx).0,
            wallets: <faraday_wallets_area::Wallets as Feature>::start(carried, cx).0,
            signing: <faraday_signing_area::Signing as Feature>::start(carried, cx).0,
            checks: <faraday_checks_area::Checks as Feature>::start(carried, cx).0,
            tools: <faraday_tools_area::Tools as Feature>::start(carried, cx).0,
            files: <faraday_files_area::Files as Feature>::start(carried, cx).0,
        };
        let effects = home_fx
            .lift(Msg::Home, crate::update::home_out)
            .and(power_fx.lift(Msg::Power, crate::update::power_out))
            .and(devices_fx.lift(Msg::Devices, crate::update::devices_out));
        (state, effects)
    }

    /// The device check's and the held-back inputs' state.
    #[must_use]
    pub fn devices(&self) -> &faraday_devices::State {
        &self.devices
    }

    /// The power feature's state.
    #[must_use]
    pub fn power(&self) -> &faraday_power::State {
        &self.power
    }
}
