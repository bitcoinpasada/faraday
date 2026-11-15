//! Check (`docs/DESIGN.md` §5, Check): the list of rows, the forms of a
//! backup in another form, and New key opened on its dice source.

use crate::tour::Tour;
use faraday_core::check::{Press, Row};
use faraday_core::{Action, Screen};

pub(crate) fn check_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Check));
    t.shot("check")?;
    t.press(Action::Check(Press::Row(Row::Backup)));
    t.shot("check-backups")?;
    t.press(Action::Cancel);
    t.press(Action::Check(Press::Row(Row::Dice)));
    t.shot("check-dice")
}
