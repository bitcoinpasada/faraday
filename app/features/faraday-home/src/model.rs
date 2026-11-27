use faraday_templates::menu_page::MenuPageModel;

use crate::Press;

/// What `present` works out for the screen (04 §2): the menu page's model, whose search row
/// holds the field and whose sections are the first-run actions or, while words are typed, the
/// guide words found.
pub struct Model<'s> {
    pub(crate) page: MenuPageModel<'s, Press>,
}
