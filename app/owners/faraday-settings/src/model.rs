use faraday_organisms::content::Content;
use faraday_templates::menu_page::MenuPageModel;

use crate::Press;

/// What `present` works out for the screen (04 §2): the Settings menu page, or Set the time.
pub enum Model<'s> {
    /// Settings: the cards on the kit's menu page.
    Settings(Box<MenuPageModel<'s, Press>>),
    /// Set the time: the clock, the zone, the fields and the buttons, or the list of zones.
    SetTime(Content<'s, Press>),
}
