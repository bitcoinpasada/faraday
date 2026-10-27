//! Search in an open vault's items and in the TOTP codes (`docs/VAULT.md`
//! §12, `docs/TOTP.md` §5.3): typing in the search bar keeps the items or
//! codes whose title holds what is typed, case not counted; its × shows
//! them all again; nothing matching says so in a line. The list follows
//! an entry added. On the Pi's panel the bar takes the on-screen keyboard
//! and nothing is cut off; on a wide window the contents run to the
//! window's edge and the codes stand centred in their reading area.

use crate::common::type_text;
use faraday_core::codes::codes_rows;
use faraday_core::testkit;
use faraday_core::vault::{SearchAct, SearchIn, VaultAction as V};
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware, TouchPhase,
};
use faraday_ui::keyboard::OskPress;
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Cost, Record};

const PASSPHRASE: &str = "many entries";

/// Five bank entries with a TOTP secret, then twenty-five mail entries,
/// the first three with one: more than a window shows.
fn titles() -> Vec<(String, bool)> {
    let banks = (0..5).map(|i| (format!("Bank {i}"), true));
    let mails = (0..25).map(|i| (format!("Mail {i}"), i < 3));
    banks.chain(mails).collect()
}

/// A one-slot vault holding [`titles`]' entries.
fn many_entries() -> Vec<u8> {
    let c = testkit::compute();
    let mut records = vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Passwords")];
    for (t, totp) in titles() {
        let mut r = Record::new(kind::ENTRY)
            .with(field::TITLE, t.as_bytes())
            .with(field::PASSWORD, b"hunter2");
        if totp {
            r = r.with(
                field::TOTP,
                b"otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP",
            );
        }
        records.push(r);
    }
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 3,
        lanes: 1,
    };
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[PASSPHRASE.as_bytes()],
        &[Contents { records }],
        &[0x42; 32],
    )
    .unwrap()
}

/// The app at `width` × `height` pixels and `dpi`, the vault unlocked on
/// its contents, the clock known.
fn unlocked(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
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
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    app.storage(StorageEvent::Restored {
        inbox: vec![("passwords.ofv".into(), many_entries())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 2_000_000_000,
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault opens");
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    app
}

fn search(s: SearchIn) -> Action {
    Action::Vault(V::Search(SearchAct::Focus(s)))
}

fn clear(s: SearchIn) -> Action {
    Action::Vault(V::Search(SearchAct::Clear(s)))
}

fn drawn(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_texts().iter().any(|t| t == text)
}

#[test]
fn typing_in_the_items_search_shows_only_the_titles_that_hold_it() {
    let mut app = unlocked(1280, 800, 160);
    assert!(drawn(&mut app, "Mail 0"), "every entry is listed at first");
    app.press(search(SearchIn::Items));
    type_text(&mut app, "bAnK 3");
    assert!(drawn(&mut app, "Bank 3"));
    for t in ["Bank 2", "Mail 0", "Mail 3"] {
        assert!(!drawn(&mut app, t), "{t} is not listed");
    }
    app.press(clear(SearchIn::Items));
    assert!(drawn(&mut app, "Mail 0"), "cleared, every entry is listed");
    assert!(drawn(&mut app, "Bank 2"));
    type_text(&mut app, "parcel");
    assert!(drawn(&mut app, "No titles match"));
    assert!(!drawn(&mut app, "Bank 0"));
}

#[test]
fn backspace_in_the_items_search_widens_what_is_listed() {
    let mut app = unlocked(1280, 800, 160);
    app.press(search(SearchIn::Items));
    type_text(&mut app, "mail 1");
    assert!(drawn(&mut app, "Mail 12"));
    assert!(!drawn(&mut app, "Mail 2"));
    app.event(Event::Key(faraday_shell_api::Key::Backspace));
    app.event(Event::Key(faraday_shell_api::Key::Backspace));
    assert!(drawn(&mut app, "Mail 2"));
    assert!(!drawn(&mut app, "Bank 0"));
}

#[test]
fn a_search_follows_an_entry_added_and_a_query_changed() {
    let mut app = unlocked(1280, 800, 160);
    app.press(search(SearchIn::Items));
    type_text(&mut app, "bank");
    let id = app.vaults().open[0].id;
    for title in ["Bank 9", "Shop"] {
        testkit::vault_edit(&mut app, id, |s| {
            s.set_fields(id, None, kind::ENTRY, &[(field::TITLE, title.as_bytes())])
        })
        .unwrap();
    }
    assert!(
        drawn(&mut app, "Bank 9"),
        "a new entry that matches is listed"
    );
    assert!(!drawn(&mut app, "Shop"), "one that does not is not");
    type_text(&mut app, " 9");
    assert!(drawn(&mut app, "Bank 9"));
    assert!(!drawn(&mut app, "Bank 0"), "the list follows the query");
}

#[test]
fn typing_in_the_codes_search_shows_only_the_codes_whose_titles_hold_it() {
    let mut app = unlocked(1280, 800, 160);
    app.press(Action::OpenCodes);
    assert_eq!(app.screen, Screen::Codes);
    let now = app.clock_now().unwrap();
    assert_eq!(codes_rows(&app, now).len(), 8, "every TOTP entry has a row");
    app.press(search(SearchIn::Codes));
    type_text(&mut app, "MAIL");
    let rows: Vec<String> = codes_rows(&app, now).into_iter().map(|r| r.title).collect();
    assert_eq!(rows, ["Mail 0", "Mail 1", "Mail 2"]);
    assert!(drawn(&mut app, "Mail 1"));
    assert!(!drawn(&mut app, "Bank 0"));
    app.press(clear(SearchIn::Codes));
    assert_eq!(codes_rows(&app, now).len(), 8, "cleared, every row is back");
    assert!(drawn(&mut app, "Bank 0"));
    type_text(&mut app, "parcel");
    assert!(drawn(&mut app, "No codes match"));
}

#[test]
fn a_code_found_by_search_opens_its_entry() {
    let mut app = unlocked(1280, 800, 160);
    app.press(search(SearchIn::Items));
    type_text(&mut app, "bank");
    app.press(Action::OpenCodes);
    app.press(search(SearchIn::Codes));
    type_text(&mut app, "mail 2");
    let now = app.clock_now().unwrap();
    let row = codes_rows(&app, now).remove(0);
    app.press(Action::CodesOpen(row.vault, row.item));
    assert_eq!(app.screen, Screen::VaultContents);
    let place = app.vault_selected_index().expect("an entry selected");
    let records = app.vaults().records(app.vaults().open[0].id);
    assert_eq!(
        records.get(place).and_then(|r| r.public_text(field::TITLE)),
        Some("Mail 2"),
        "the row's own entry is selected, though the items search held another"
    );
}

const PI_W: u16 = 480;
const PI_H: u16 = 640;

/// Scrolls the Pi's page until `action` is on it, then taps it.
fn tap_on_the_panel(app: &mut Faraday, action: Action) {
    for _ in 0..40 {
        let _ = app.frame();
        if let Some((x, y)) = app.where_offered(action)
            && y < PI_H - 40
        {
            for phase in [TouchPhase::Down, TouchPhase::Up] {
                app.event(Event::Touch { x, y, phase });
            }
            let _ = app.frame();
            return;
        }
        app.event(Event::Scroll {
            x: PI_W / 2,
            y: PI_H / 3,
            dy: 40,
        });
        app.settle();
    }
    panic!("{action:?} is never on the panel");
}

/// Every text the page shows from its top to its foot, and what it cuts
/// off at its sides.
fn whole_page(app: &mut Faraday) -> (Vec<String>, Vec<String>) {
    let (mut texts, mut cut) = (Vec::new(), Vec::new());
    for _ in 0..60 {
        app.event(Event::Scroll {
            x: PI_W / 2,
            y: PI_H / 2,
            dy: -60,
        });
    }
    app.settle();
    let mut last = None;
    for _ in 0..60 {
        let _ = app.frame();
        cut.extend(app.drawn_past_sides());
        texts.extend(app.drawn_texts());
        // At the foot when nothing moved: the same texts in the same
        // places.
        let shown = app.drawn_past_sides_and_text().1;
        if last.as_ref() == Some(&shown) {
            break;
        }
        last = Some(shown);
        app.event(Event::Scroll {
            x: PI_W / 2,
            y: PI_H / 2,
            dy: 60,
        });
        app.settle();
    }
    (texts, cut)
}

#[test]
fn on_the_pi_panel_the_search_takes_the_keyboard_and_nothing_is_cut_off() {
    let mut app = unlocked(PI_W, PI_H, 286);
    tap_on_the_panel(&mut app, search(SearchIn::Items));
    let key = app
        .hit_box(Action::Osk(OskPress::Char('q')))
        .expect("the keyboard is up");
    let bar = app
        .hit_box(search(SearchIn::Items))
        .expect("the bar stays on the panel");
    assert!(
        bar.1 + bar.3 <= key.1,
        "the bar {bar:?} stands above the keyboard {key:?}"
    );
    for c in "bank".chars() {
        app.press(Action::Osk(OskPress::Char(c)));
    }
    assert!(drawn(&mut app, "bank"), "what is typed is in the bar");
    assert!(
        drawn(&mut app, "Bank 0"),
        "the search view shows what it finds under the bar"
    );
    app.press(Action::Osk(OskPress::Done));
    let (texts, cut) = whole_page(&mut app);
    for i in 0..5 {
        assert!(texts.contains(&format!("Bank {i}")), "Bank {i} is listed");
    }
    assert!(!texts.iter().any(|t| t == "Mail 0"), "Mail 0 is not");
    assert!(cut.is_empty(), "cut off: {cut:?}");

    app.press(Action::OpenCodes);
    tap_on_the_panel(&mut app, search(SearchIn::Codes));
    assert!(app.hit_box(Action::Osk(OskPress::Char('m'))).is_some());
    for c in "mail".chars() {
        app.press(Action::Osk(OskPress::Char(c)));
    }
    app.press(Action::Osk(OskPress::Done));
    let (texts, cut) = whole_page(&mut app);
    assert!(texts.iter().any(|t| t == "Mail 2"), "Mail 2 is listed");
    assert!(!texts.iter().any(|t| t == "Bank 0"));
    assert!(cut.is_empty(), "codes cut off: {cut:?}");
}

#[test]
fn on_the_pi_panel_back_closes_the_search_view_and_keeps_the_list_filtered() {
    let mut app = unlocked(PI_W, PI_H, 286);
    for (screen, bar, typed, kept, gone) in [
        (
            Screen::VaultContents,
            SearchIn::Items,
            "bank",
            "Bank 0",
            "Mail 0",
        ),
        (Screen::Codes, SearchIn::Codes, "mail", "Mail 2", "Bank 0"),
    ] {
        if screen == Screen::Codes {
            app.press(Action::OpenCodes);
        }
        tap_on_the_panel(&mut app, search(bar));
        for c in typed.chars() {
            app.press(Action::Osk(OskPress::Char(c)));
        }
        assert!(app.hit_box(Action::Osk(OskPress::Char('q'))).is_some());
        // The bar's way back closes the search view, as Done does.
        assert_eq!(
            app.back_link(),
            Some(Action::Vault(V::Search(SearchAct::Leave(bar))))
        );
        app.press(Action::Back);
        let _ = app.frame();
        assert_eq!(app.screen, screen, "Back stays on the page");
        assert!(
            app.hit_box(Action::Osk(OskPress::Char('q'))).is_none(),
            "the keyboard is down"
        );
        assert_ne!(
            app.back_link(),
            Some(Action::Vault(V::Search(SearchAct::Leave(bar))))
        );
        let (texts, _) = whole_page(&mut app);
        assert!(texts.iter().any(|t| t == kept), "{kept} is still listed");
        assert!(!texts.iter().any(|t| t == gone), "{gone} stays filtered");
        assert!(texts.iter().any(|t| t == typed), "the query is kept");
    }
}

#[test]
fn on_a_very_wide_window_the_contents_take_the_whole_room_and_the_codes_stand_centred() {
    const SIDEBAR: f32 = 240.0;
    for (w, h) in [(1920u16, 1080u16), (2560, 1600)] {
        let mut app = unlocked(w, h, 160);
        let _ = app.frame();
        // The kinds start beside the navigation, and the item runs on
        // past the old 1360 dp area's end.
        let kinds = app
            .hit_box(Action::Vault(V::Category(2)))
            .expect("Entries among the kinds");
        assert!(
            (f32::from(kinds.0) - (SIDEBAR + 40.0)).abs() <= 2.0,
            "{w}: the kinds start at {}",
            kinds.0
        );
        let lock = app.hit_box(Action::LockAsk).expect("Lock on the contents");
        let right = f32::from(lock.0 + lock.2);
        let old_end = SIDEBAR + (f32::from(w) - SIDEBAR + 1360.0) / 2.0;
        assert!(right > old_end, "{w}: Lock ends {right}, past {old_end}");
        app.press(Action::OpenCodes);
        let _ = app.frame();
        let bar = app
            .hit_box(search(SearchIn::Codes))
            .expect("the codes' search bar");
        let middle = f32::from(bar.0) + f32::from(bar.2) / 2.0;
        let room = SIDEBAR + (f32::from(w) - SIDEBAR) / 2.0;
        assert!(
            (middle - room).abs() <= 2.0,
            "{w}: the codes' middle {middle}, the room's {room}"
        );
    }
}
