use faraday_organisms::sheet::DialogModel;

use crate::Press;

/// What `present` works out for the screen (04 §2): the sheet on show, none when none is.
pub struct Model<'s> {
    pub(crate) sheet: Option<DialogModel<'s, Press>>,
}
