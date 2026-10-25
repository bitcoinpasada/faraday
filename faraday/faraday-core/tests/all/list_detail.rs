//! List and detail (`docs/LAYOUT.md` §5, §17): from expanded windows up,
//! Wallets, Files, Vaults, Backups and Settings show their list in a pane
//! at the start and the one chosen beside it; on a compact window the
//! list and the one chosen are each a page.

use crate::common::process;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// The list pane's width (`LIST_PANE`).
const LIST_PANE: i32 = 360;

/// Where the list pane ends beside the expanded rail: the 240 dp rail,
/// the 24 dp margin and the pane.
const LIST_RIGHT: i32 = 240 + 24 + LIST_PANE;

fn window(app: &mut Faraday, width: u16, height: u16, dpi: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// The test stick's wallets and vault in Files, the wallets loaded in
/// the kit's order, at `width` × `height` and `dpi`.
fn with_wallets(width: u16, height: u16, dpi: u16) -> Faraday {
    let files: Vec<(String, Vec<u8>)> = testkit::files()
        .expect("the test stick's files")
        .into_iter()
        .filter(|(n, _)| n.ends_with("-wallet.txt") || n == "vault.ofv")
        .collect();
    let names: Vec<String> = files
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| n.ends_with("-wallet.txt"))
        .collect();
    let mut app = process(files, Vec::new(), Vec::new());
    window(&mut app, width, height, dpi);
    for n in names.iter().take(4) {
        let i = app
            .inbox()
            .iter()
            .position(|it| &it.name == n)
            .expect("the wallet's file");
        app.press(Action::LoadWallet(i));
    }
    assert!(app.session.wallets.len() >= 3, "three wallets loaded");
    app
}

/// Where `text` is drawn, each place: (x, y, w, h).
fn boxes(app: &mut Faraday, text: &str) -> Vec<(i32, i32, i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == text)
        .map(|(_, r)| r)
        .collect()
}

/// The right edge of the list pane's item that chooses `a`.
fn item_right(app: &mut Faraday, a: Action) -> i32 {
    let _ = app.frame();
    let (x, _, w, _) = app.hit_box(a).expect("the item in the list pane");
    i32::from(x) + i32::from(w)
}

#[test]
fn nothing_chosen_backups_shows_the_first_wallet_beside_the_list() {
    let mut app = with_wallets(1920, 1080, 160);
    app.press(Action::Nav(Screen::Backups));
    let _ = app.frame();
    assert!(app.hit_box(Action::BackupsPage(1)).is_some(), "the list");
    // The first wallet's name twice: its item, and its map's head.
    let names: Vec<String> = app.session.wallets.iter().map(|w| w.name.clone()).collect();
    let shown: Vec<&String> = names
        .iter()
        .filter(|n| boxes(&mut app, n).iter().any(|b| b.0 > LIST_RIGHT))
        .collect();
    assert_eq!(
        shown.len(),
        1,
        "one wallet's map beside the list: {shown:?}"
    );
    let first = boxes(&mut app, shown[0])
        .into_iter()
        .filter(|b| b.0 < LIST_RIGHT && b.0 > 240)
        .map(|b| b.1)
        .min()
        .expect("its item");
    let others = names
        .iter()
        .filter(|n| *n != shown[0])
        .flat_map(|n| boxes(&mut app, n))
        .filter(|b| b.0 < LIST_RIGHT && b.0 > 240)
        .map(|b| b.1);
    assert!(
        others.into_iter().all(|y| y > first),
        "it is the list's first"
    );
}

#[test]
fn choosing_a_wallet_at_1920_shows_its_page_beside_the_list() {
    let mut app = with_wallets(1920, 1080, 160);
    app.press(Action::Nav(Screen::Wallets));
    let right = item_right(&mut app, Action::PickWallet(2));
    app.press(Action::PickWallet(2));
    assert_eq!(app.screen, Screen::Wallets, "the list stays on screen");
    let third = app.session.wallets[2].name.clone();
    let first = app.session.wallets[0].name.clone();
    assert!(
        boxes(&mut app, &third).iter().any(|b| b.0 > right),
        "{third}'s page beside the list"
    );
    assert!(
        boxes(&mut app, &first).iter().all(|b| b.0 < right),
        "{first} only in the list now"
    );
    assert!(
        app.hit_box(Action::PickWallet(0)).is_some(),
        "the list stays"
    );
}

#[test]
fn at_480x640_choosing_a_wallet_opens_its_page() {
    let mut app = with_wallets(480, 640, 286);
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    assert!(app.hit_box(Action::OpenWallet(1)).is_some(), "the list");
    app.press(Action::OpenWallet(1));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Wallets);
    assert!(
        app.hit_box(Action::PickWallet(0)).is_none(),
        "no list pane beside the page"
    );
    assert!(app.hit_box(Action::Rename).is_some(), "the wallet's page");
}

#[test]
fn the_rail_window_shows_the_list_and_the_wallet_side_by_side() {
    // 960 × 600 dp: expanded and low, the rail's window.
    let mut app = with_wallets(960, 600, 160);
    app.press(Action::Nav(Screen::Wallets));
    let right = item_right(&mut app, Action::PickWallet(1));
    app.press(Action::PickWallet(1));
    let rename = {
        let _ = app.frame();
        app.hit_box(Action::Rename).expect("the wallet's Rename")
    };
    assert!(i32::from(rename.0) > right, "the page beside the list");
}

#[test]
fn the_list_pane_scrolls_on_its_own() {
    let mut app = with_wallets(1280, 800, 160);
    // Every wallet of the kit, so the list is taller than the window.
    let more: Vec<usize> = (0..app.inbox().len())
        .filter(|&i| app.inbox()[i].name.ends_with("-wallet.txt"))
        .collect();
    for _ in more {
        if let Some(i) = app
            .inbox()
            .iter()
            .position(|it| it.name.ends_with("-wallet.txt"))
        {
            app.press(Action::LoadWallet(i));
        }
    }
    app.press(Action::Nav(Screen::Wallets));
    let _ = app.frame();
    let item = app.hit_box(Action::PickWallet(0)).expect("the first item");
    let rename = app.hit_box(Action::Rename).expect("the detail's Rename");
    let (x, y) = (item.0 + item.2 / 2, item.1 + item.3 / 2);
    for _ in 0..10 {
        app.event(Event::Scroll { x, y, dy: 40 });
        let _ = app.frame();
    }
    app.event(Event::ScrollEnd { x, y });
    for _ in 0..30 {
        let _ = app.frame();
    }
    let moved = app.hit_box(Action::PickWallet(0)).map_or(0, |b| b.1);
    assert!(moved < item.1, "the list moved up: {} to {moved}", item.1);
    assert_eq!(
        app.hit_box(Action::Rename).map(|b| b.1),
        Some(rename.1),
        "the detail stayed"
    );
}

#[test]
fn backups_at_1920_shows_the_chosen_wallets_map_beside_the_wallets() {
    let mut app = with_wallets(1920, 1080, 160);
    app.press(Action::Nav(Screen::Backups));
    let right = item_right(&mut app, Action::BackupsPage(1));
    app.press(Action::BackupsPage(1));
    let _ = app.frame();
    let backup = app
        .hit_box(Action::Backup(1))
        .or_else(|| (0..8).find_map(|i| app.hit_box(Action::Backup(i))))
        .expect("the chosen wallet's Back up");
    assert!(i32::from(backup.0) > right, "its Back up beside the list");
    assert!(
        app.hit_box(Action::BackupsPage(0)).is_some(),
        "the list stays"
    );
}

#[test]
fn files_shows_its_two_lists_side_by_side_and_the_stick_in_the_top_row() {
    let mut app = with_wallets(1920, 1080, 160);
    app.press(Action::Nav(Screen::Files));
    // Beside the expanded rail, which names Files too.
    // The lists' titles, under what the Inbox gives, which names the
    // stick too.
    let page = |v: Vec<(i32, i32, i32, i32)>| {
        v.into_iter()
            .filter(|b| b.0 > 240)
            .max_by_key(|b| b.1)
            .expect("on the page")
    };
    let from = page(boxes(&mut app, "From the stick"));
    let for_ = page(boxes(&mut app, "For the stick"));
    assert_eq!(from.1, for_.1, "the two lists' titles on one line");
    assert!(for_.0 - from.0 >= LIST_PANE, "{from:?} {for_:?}");
    let waiting = boxes(&mut app, "Waiting for a stick");
    assert!(
        waiting.iter().any(|b| b.1 + b.3 <= 48 && b.0 > for_.0),
        "the stick's chip in the top row at the right: {waiting:?}"
    );
}

#[test]
fn vaults_choosing_a_locked_vault_shows_its_unlock_beside_the_list() {
    let mut app = with_wallets(1920, 1080, 160);
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    assert!(app.hit_box(Action::Vault(V::Open(0))).is_some(), "the list");
    assert!(app.hit_box(Action::Vault(V::Unlock)).is_none());
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.screen, Screen::Unlock);
    let _ = app.frame();
    let unlock = app
        .hit_box(Action::Vault(V::Unlock))
        .expect("Unlock beside the list");
    assert!(i32::from(unlock.0) >= LIST_RIGHT);
    assert!(
        app.hit_box(Action::Vault(V::Open(0))).is_some(),
        "the vaults' list stays"
    );
}

/// The list pane's item for Settings' section `name`.
fn section_of(app: &mut Faraday, name: &str) -> Action {
    let at = boxes(app, name)
        .into_iter()
        .min_by_key(|b| b.0)
        .expect("the section in the list");
    let (cx, cy) = (at.0 + at.2 / 2, at.1 + at.3 / 2);
    (0..16u8)
        .map(Action::SettingsSection)
        .find(|&a| {
            app.hit_box(a).is_some_and(|(x, y, w, h)| {
                let (x, y, w, h) = (i32::from(x), i32::from(y), i32::from(w), i32::from(h));
                (x..x + w).contains(&cx) && (y..y + h).contains(&cy)
            })
        })
        .expect("its item takes a press")
}

#[test]
fn settings_at_1920_shows_its_sections_and_the_one_chosen() {
    let mut app = with_wallets(1920, 1080, 160);
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    for name in ["Session", "Time", "Appearance", "Display scale", "About"] {
        assert!(!boxes(&mut app, name).is_empty(), "{name} in the list");
    }
    // Nothing chosen: the first section, Mode; Display scale's choices
    // wait until it is chosen.
    assert!(app.hit_box(Action::Scale(125)).is_none());
    let scale = section_of(&mut app, "Display scale");
    let right = item_right(&mut app, scale);
    app.press(scale);
    let _ = app.frame();
    let choice = app
        .hit_box(Action::Scale(125))
        .expect("Display scale's 125 %");
    assert!(i32::from(choice.0) > right, "beside the list");
    assert!(
        boxes(&mut app, "Lock after").is_empty(),
        "one section at a time"
    );
    // Chosen, it works as it always has.
    app.press(Action::Scale(125));
    assert_eq!(app.scale_pct(), 125);
}

#[test]
fn settings_on_a_compact_window_is_every_section_in_one_column() {
    let mut app = with_wallets(480, 640, 286);
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    assert!(
        app.hit_box(Action::SettingsSection(0)).is_none(),
        "no list pane"
    );
    assert!(
        !boxes(&mut app, "Lock after").is_empty(),
        "Session's timers"
    );
}
