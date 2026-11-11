//! How long ago each copy was checked is read once where the vault's notes
//! are read (`docs/BACKUP-FLOW.md` §18.3, `docs/SPEED.md` §3.3): the
//! wallet's card, with its chart and the strip's line under it, is drawn
//! again with nothing worked out.

use super::{DESKTOP, heavy_on};
use crate::work::{KINDS, since, snapshot};
use crate::{Action, Faraday, Screen, testkit};
use faraday_shell_api::App;

/// The heavy session with the first wallet's plan made and its card open,
/// the clock set or not.
fn card(clock: bool) -> Faraday {
    let (mut app, w) = heavy_on(DESKTOP);
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, crate::plan::Preset::PaperVault);
    app.press(Action::BChecklist);
    if clock {
        app.press(Action::OpenSetTime);
        app.press(Action::SetTimeKeep);
    }
    app.press(Action::OpenWallet(w));
    assert_eq!(app.screen, Screen::Wallets);
    app
}

/// Two frames drawn again, nothing changed: no counted work at all.
fn frames_work_out_nothing(app: &mut Faraday) {
    app.settle();
    let _ = app.frame();
    let before = snapshot();
    let _ = app.frame();
    let _ = app.frame();
    let again = since(&before);
    for k in KINDS {
        assert_eq!(again.of(k), 0, "the frames did {k:?} {}", again.of(k));
    }
}

#[test]
fn a_card_with_a_chart_and_its_ages_works_out_nothing_on_a_frame_drawn_again() {
    for clock in [true, false] {
        let mut app = card(clock);
        frames_work_out_nothing(&mut app);
    }
}
