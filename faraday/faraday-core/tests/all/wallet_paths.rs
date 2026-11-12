//! A wallet with a relative wait states its spend paths and the margin to
//! move each coin within on its card, and its sheet carries them with
//! the table to fill in (`docs/BACKUP-FLOW.md` §18.4).

use faraday_core::wallet::Origin;
use faraday_core::wallet_backup::sheet_wallet;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_paper::pdf;
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

use crate::common::texts;

/// A session on a desktop window with `miniscript` loaded as a wallet.
pub(crate) fn with(miniscript: &str) -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let p = "m/48'/1'/0'/2'";
    let descriptor = miniscript
        .replace("@A", &format!("{}/<0;1>/*", testkit::key(0, p)))
        .replace("@B", &format!("{}/<0;1>/*", testkit::key(1, p)));
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
    assert_eq!(app.screen, Screen::Wallets);
    app
}

fn screen(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

pub(crate) const YEAR: &str = "wsh(or_d(pk(@A),and_v(v:pkh(@B),older(52560))))";

#[test]
fn a_wallet_with_a_wait_states_its_paths_and_the_margin() {
    let mut app = with(YEAR);
    let shown = screen(&mut app);
    assert!(shown.contains("Spend paths"), "{shown}");
    assert!(shown.contains("Any time"), "{shown}");
    assert!(
        shown.contains("52 560 blocks after each coin arrives · about 365 days"),
        "{shown}"
    );
    assert!(shown.contains("Move each coin within"), "{shown}");
    assert!(shown.contains("about 328 days of arriving"), "{shown}");
}

#[test]
fn a_wallet_with_no_wait_has_no_group() {
    for ms in ["wsh(multi(2,@A,@B))", "wsh(pk(@A))"] {
        let mut app = with(ms);
        let shown = screen(&mut app);
        assert!(!shown.contains("Spend paths"), "{ms}: {shown}");
        assert!(!shown.contains("Move each coin within"), "{ms}: {shown}");
    }
}

#[test]
fn a_wait_in_time_reads_days_without_about() {
    // The time flag and 8000 units of 512 seconds: 47 days.
    let mut app = with("wsh(or_d(pk(@A),and_v(v:pkh(@B),older(4202304))))");
    let shown = screen(&mut app);
    assert!(shown.contains("47 days after each coin arrives"), "{shown}");
    assert!(shown.contains("42 days of arriving"), "{shown}");
    assert!(!shown.contains("about 42"), "{shown}");
}

#[test]
fn an_absolute_lock_has_no_margin() {
    let mut app = with("wsh(or_d(pk(@A),and_v(v:pkh(@B),after(900000))))");
    let shown = screen(&mut app);
    assert!(!shown.contains("Move each coin within"), "{shown}");
    assert!(!shown.contains("Spend paths"), "{shown}");
}

#[test]
fn the_wallet_sheet_carries_the_paths_and_the_table() {
    let app = with(YEAR);
    let c = testkit::compute();
    let w = &app.session.wallets[0];
    let sheet = sheet_wallet(&c, &app.session, w);
    assert!(sheet.contains("path: Key 1\nwait: Any time\n"), "{sheet}");
    assert!(sheet.contains("unmoved: 365 52560 328\n"), "{sheet}");
    let made = pdf::sheet(&c, &sheet).expect("a PDF");
    let has = |p: &[u8], s: &str| p.windows(s.len()).any(|w| w == s.as_bytes());
    assert!(has(&made, "(Spend paths) Tj"));
    assert!(has(&made, "(Received on) Tj"));
    assert!(has(&made, "(Move before) Tj"));
    // A sheet file with no `path` lines prints as a wallet sheet did.
    let plain: String = sheet
        .lines()
        .filter(|l| {
            !["path:", "wait:", "unmoved:", "alone:"]
                .iter()
                .any(|k| l.starts_with(k))
        })
        .map(|l| format!("{l}\n"))
        .collect();
    let old = pdf::sheet(&c, &plain).expect("a PDF");
    assert!(!has(&old, "Spend paths"));
    assert!(!has(&old, "Received on"));
}
