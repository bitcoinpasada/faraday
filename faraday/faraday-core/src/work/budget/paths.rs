//! A wallet's spend paths and the margin are read once when it is loaded
//! (`docs/BACKUP-FLOW.md` §18.4): the card, the small panel's page and
//! the chart's wallet sheet are drawn again with nothing worked out.

use super::{DESKTOP, PI, Panel, show};
use crate::glance::Press;
use crate::glance::sheet::{ChartAction, Target};
use crate::wallet::Origin;
use crate::work::{KINDS, since, snapshot};
use crate::{Action, Faraday, Screen, testkit};
use faraday_shell_api::App;

/// A wallet that opens to a second key after a year, loaded.
fn session(panel: Panel) -> (Faraday, usize) {
    let mut app = testkit::started();
    show(&mut app, panel);
    let p = "m/48'/1'/0'/2'";
    let descriptor = format!(
        "wsh(or_d(pk({}/<0;1>/*),and_v(v:pkh({}/<0;1>/*),older(52560))))",
        testkit::key(0, p),
        testkit::key(1, p)
    );
    let w = app
        .session
        .add_wallet(
            &testkit::compute(),
            "Inherit",
            &descriptor,
            "test",
            Origin::Named,
        )
        .expect("the wallet");
    app.press(Action::OpenWallet(w));
    (app, w)
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
fn the_card_with_spend_paths_works_out_nothing_on_a_frame_drawn_again() {
    for panel in [DESKTOP, PI] {
        let (mut app, _) = session(panel);
        assert_eq!(app.screen, Screen::Wallets);
        frames_work_out_nothing(&mut app);
    }
}

#[test]
fn the_spend_paths_page_works_out_nothing_on_a_frame_drawn_again() {
    let (mut app, _) = session(PI);
    app.press(Action::SpendPaths(true));
    frames_work_out_nothing(&mut app);
}

#[test]
fn the_chart_s_wallet_sheet_works_out_nothing_on_a_frame_drawn_again() {
    let (mut app, w) = session(DESKTOP);
    app.press(Action::Chart(ChartAction::Open(
        Press::Loaded(w),
        Target::Wallet,
    )));
    frames_work_out_nothing(&mut app);
}
