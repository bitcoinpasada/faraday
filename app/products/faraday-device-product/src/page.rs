use faraday_device::frame::FrameSheetModel;
use faraday_organisms::Ui;
use faraday_power::Power;
use faraday_present::{Drawn, Page};

use crate::{AppPress, Body, Over, Press};

/// A page or sheet of the device composite draws with its presses lifted into the app's.
fn device_press(press: faraday_device::Press) -> Press {
    Press::App(AppPress::Device(press))
}

impl Drawn<Press> for Body<'_> {
    fn draw(&self, ui: &mut Ui<'_, Press>) {
        match self {
            Body::Device(body) => ui.within(device_press, |ui| body.draw(ui)),
            Body::Settings(model) => ui.within(Press::Settings, |ui| {
                <faraday_settings::Settings as Page>::draw(model, ui);
            }),
            // The vaults' page and the failed self-test page are not built yet.
            Body::Vaults(_) | Body::SelfTestFailed(_) => todo!(),
        }
    }
}

impl Drawn<Press> for Over<'_> {
    fn draw(&self, ui: &mut Ui<'_, Press>) {
        match self {
            Over::Device(sheet) => ui.within(device_press, |ui| sheet.draw(ui)),
            Over::Frame(FrameSheetModel::Power(model)) => {
                ui.within(
                    |p| device_press(faraday_device::Press::Power(p)),
                    |ui| {
                        Power::draw(model, ui);
                    },
                );
            }
            // The device check's and the guide's sheets and the vaults' are not built yet.
            Over::Frame(_) | Over::Vaults(_) => todo!(),
        }
    }
}
