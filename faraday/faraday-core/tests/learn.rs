//! The ? on a screen opens OpenSigner's Learn pages for what is on it: a
//! multisig being created opens Multisig, a new key opens Randomness, and
//! another tab shows the next page.

use faraday_core::{Action, Faraday, Sheet};

fn shown(app: &Faraday) -> &'static str {
    app.learn.pages[app.learn.page].title
}

#[test]
fn the_question_mark_opens_the_pages_for_the_screen() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(4));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(shown(&app), "Multisig");
    app.press(Action::LearnPage(1));
    assert_eq!(shown(&app), "Xpubs and privacy");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);

    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::Learn);
    assert_eq!(shown(&app), "Randomness");
}

#[test]
fn the_backups_list_names_the_page_on_a_backup_plans_check_after_backups() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Backups));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(shown(&app), "Backups");
    app.press(Action::LearnPage(1));
    assert_eq!(shown(&app), "A backup plan's check");
}
