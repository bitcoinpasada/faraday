use faraday_home::Home;
use faraday_organisms::Ui;
use faraday_present::{Drawn, Page};

use crate::{Body, Over, Press};

impl Drawn<Press> for Body<'_> {
    fn draw(&self, ui: &mut Ui<'_, Press>) {
        match self {
            Body::Home(model) => ui.within(Press::Home, |ui| Home::draw(model, ui)),
            // The areas' pages are not built.
            Body::Keys(_)
            | Body::Wallets(_)
            | Body::Signing(_)
            | Body::Checks(_)
            | Body::Tools(_)
            | Body::Files(_) => todo!(),
        }
    }
}

impl Drawn<Press> for Over<'_> {
    fn draw(&self, _ui: &mut Ui<'_, Press>) {
        // No sheet of the scan feature or the areas comes up yet.
        todo!()
    }
}
