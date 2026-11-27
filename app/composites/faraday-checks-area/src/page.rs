use faraday_organisms::Ui;
use faraday_present::Drawn;

use crate::{Body, Over, Press};

impl Drawn<Press> for Body<'_> {
    fn draw(&self, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}

impl Drawn<Press> for Over<'_> {
    fn draw(&self, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
