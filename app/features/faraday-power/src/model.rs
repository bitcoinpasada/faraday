use faraday_organisms::ButtonModel;
use faraday_organisms::Shown;
use faraday_organisms::sheet::DialogModel;

use crate::Press;

/// What `present` works out for the screen (04 §2): the sheet on show, none when none is.
pub struct Model<'s> {
    pub(crate) sheet: Option<DialogModel<'s, Press>>,
}

/// The model of the failed self-test page: its title, the check that did not reproduce and the
/// one control, which powers off (Quit in the desktop app).
pub struct FailedModel<'s> {
    pub(crate) title: Shown<'s>,
    pub(crate) row: Shown<'s>,
    pub(crate) check: Shown<'s>,
    pub(crate) exit: ButtonModel<'s, Press>,
}
