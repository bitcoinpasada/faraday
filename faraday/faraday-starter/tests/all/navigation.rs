//! What a person sees of the Starter's navigation (`docs/STARTER.md`
//! §3.1d): the bar or the rail by the window's width, the four
//! destinations, task pages without either, the ⋮ menu, Install and
//! Update in the top bar.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};
use faraday_starter::api::{
    Bus, Computer, Disk, FaradayStick, InstallItem, Installed, Os, Payload, PayloadFile,
    StarterEvent,
};
use faraday_starter::decide::{Item, Target};
use faraday_starter::{Class, Page, Starter, StarterAction};

const GB: u64 = 1_000_000_000;

fn display(width: u16, height: u16) -> Event {
    Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

fn payload() -> Payload {
    let file = |name: &str, n: char| PayloadFile {
        name: name.into(),
        sha256: n.to_string().repeat(64),
        inner: name.trim_end_matches(".xz").into(),
        inner_sha256: n.to_ascii_uppercase().to_string().repeat(64),
        inner_size: 4 * GB,
        build_id: "0.2.0".into(),
    };
    Payload {
        version: "0.2.0".into(),
        commit: "4d0680b1a2b34d0680b1a2b34d0680b1a2b34d06".into(),
        files: vec![
            file("faraday-stick-0.2.0-x86_64-uefi.img.xz", 'a'),
            file("faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz", 'b'),
        ],
    }
}

fn stick() -> Disk {
    Disk {
        id: "sdb".into(),
        device: "/dev/sdb".into(),
        vendor: "Maker".into(),
        model: "Stick".into(),
        serial: "SN".into(),
        size: 16 * GB,
        bus: Bus::Usb,
        volumes: Vec::new(),
        in_use: None,
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
            read: true,
        }),
    }
}

fn starter(width: u16, height: u16) -> Starter {
    let mut s = Starter::new();
    s.event(display(width, height));
    s.starter_event(StarterEvent::Computer(Computer {
        os: Os::Linux,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }));
    s.starter_event(StarterEvent::Payload(Some(payload())));
    s.starter_event(StarterEvent::Disks(Vec::new()));
    s
}

fn installed(version: &str) -> StarterEvent {
    StarterEvent::Installed(Some(Installed {
        version: version.into(),
        items: vec![(InstallItem::App, "~/.local/bin/faraday-starter".into())],
    }))
}

/// Where the last frame drawn offers `action`, after drawing it.
fn at(s: &mut Starter, action: StarterAction) -> Option<(u16, u16)> {
    let _ = s.drawn_texts();
    s.where_offered(action)
}

fn tap(s: &mut Starter, action: StarterAction) {
    let (x, y) = at(s, action).expect("offered");
    s.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    s.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
}

const DESTINATIONS: [StarterAction; 4] = [
    StarterAction::Home,
    StarterAction::MakePage,
    StarterAction::ToolsPage,
    StarterAction::Learn,
];

#[test]
fn a_narrow_window_has_the_bar_at_its_foot_and_a_wider_one_the_rail_at_its_left() {
    for (w, h, class) in [
        (400, 700, Class::Compact),
        (700, 900, Class::Medium),
        (1300, 800, Class::Expanded),
    ] {
        let mut s = starter(w, h);
        assert_eq!(s.class(), class);
        for d in DESTINATIONS {
            let (x, y) = at(&mut s, d).expect("a destination is offered");
            if class == Class::Compact {
                assert!(y > h - 80, "{w}: {d:?} at {y}");
            } else {
                assert!(x < 80, "{w}: {d:?} at {x}");
            }
        }
    }
}

#[test]
fn each_destination_opens_its_page() {
    let mut s = starter(700, 900);
    tap(&mut s, StarterAction::MakePage);
    assert_eq!(s.page, Page::Make);
    assert!(s.drawn_texts().iter().any(|t| t == "Make a Pi card"));
    tap(&mut s, StarterAction::ToolsPage);
    assert_eq!(s.page, Page::Tools);
    assert!(s.drawn_texts().iter().any(|t| t == "Open Transfer"));
    tap(&mut s, StarterAction::Home);
    assert_eq!(s.page, Page::Home);
    tap(&mut s, StarterAction::Learn);
    assert!(s.showing_held());
}

#[test]
fn a_task_page_shows_neither_and_back_returns_to_where_it_was_scrolled() {
    let mut s = starter(400, 400);
    // The page's length is known once it is drawn.
    let _ = s.drawn_texts();
    s.event(Event::Wheel { x: 0, y: 0, dy: 40 });
    let before = at(&mut s, StarterAction::Item(Item::MakeStick)).expect("the first checklist row");
    tap(&mut s, StarterAction::Item(Item::MakeStick));
    assert_eq!(s.page, Page::Disks);
    let _ = s.drawn_texts();
    for d in DESTINATIONS {
        assert!(!s.offers_action(d), "{d:?} on a task page");
    }
    assert!(s.offers_action(StarterAction::Back));
    tap(&mut s, StarterAction::Back);
    assert_eq!(s.page, Page::Home);
    assert_eq!(
        at(&mut s, StarterAction::Item(Item::MakeStick)),
        Some(before)
    );
}

#[test]
fn a_destination_pressed_again_goes_to_its_top() {
    let mut s = starter(400, 400);
    // The page's length is known once it is drawn.
    let _ = s.drawn_texts();
    s.event(Event::Wheel { x: 0, y: 0, dy: 40 });
    let scrolled = at(&mut s, StarterAction::Item(Item::MakeStick)).expect("offered");
    tap(&mut s, StarterAction::Home);
    let top = at(&mut s, StarterAction::Item(Item::MakeStick)).expect("offered");
    assert!(top.1 > scrolled.1, "{top:?} {scrolled:?}");
}

#[test]
fn a_destination_has_at_most_one_primary_button_and_one_until_a_stick_is_attached() {
    for (w, h) in [(400, 700), (700, 900), (1300, 800)] {
        let mut s = starter(w, h);
        for page in [Page::Home, Page::Make, Page::Tools] {
            s.press(match page {
                Page::Make => StarterAction::MakePage,
                Page::Tools => StarterAction::ToolsPage,
                _ => StarterAction::Home,
            });
            let _ = s.drawn_texts();
            assert_eq!(s.primary_buttons(), 1, "{w}: {page:?}");
        }
        // With a stick attached Make has none; Home still has its card's.
        s.starter_event(StarterEvent::Disks(vec![stick()]));
        s.press(StarterAction::MakePage);
        let _ = s.drawn_texts();
        assert_eq!(s.primary_buttons(), 0, "{w}: Make with a stick");
        s.press(StarterAction::Home);
        let _ = s.drawn_texts();
        assert_eq!(s.primary_buttons(), 1, "{w}: Home with a stick");
    }
}

#[test]
fn the_menu_lists_uninstall_only_while_installed_and_the_boot_entry_only_while_offered() {
    let mut s = starter(700, 900);
    s.press(StarterAction::ToolsPage);
    s.press(StarterAction::Menu);
    let t = s.drawn_texts();
    assert!(t.iter().any(|x| x == "About"));
    assert!(!t.iter().any(|x| x == "Uninstall"));
    assert!(!t.iter().any(|x| x == "Remove the Faraday boot entry"));

    s.starter_event(installed("0.2.0"));
    s.starter_event(StarterEvent::BootEntry(true));
    let t = s.drawn_texts();
    assert!(t.iter().any(|x| x == "Uninstall"));
    assert!(t.iter().any(|x| x == "Remove the Faraday boot entry"));

    // A stick attached: the entry is no longer offered.
    s.starter_event(StarterEvent::Disks(vec![stick()]));
    assert!(
        !s.drawn_texts()
            .iter()
            .any(|x| x == "Remove the Faraday boot entry")
    );

    // A choice opens its page and closes the menu.
    tap(&mut s, StarterAction::Installed);
    assert_eq!(s.page, Page::Installed);
    assert!(!s.drawn_texts().iter().any(|x| x == "About"));
}

#[test]
fn install_and_update_are_in_the_bar_as_the_install_page_says() {
    let mut s = starter(700, 900);
    let (_, y) = at(&mut s, StarterAction::Install).expect("Install while nothing is installed");
    assert!(y < 64);
    s.starter_event(installed(env!("CARGO_PKG_VERSION")));
    assert_eq!(at(&mut s, StarterAction::Install), None);
    s.starter_event(installed("0.0.1"));
    let (_, y) = at(&mut s, StarterAction::Install).expect("Update while the copy is newer");
    assert!(y < 64);
    assert!(s.drawn_texts().iter().any(|t| t == "Update"));
}

#[test]
fn larger_text_at_700_takes_the_compact_layout() {
    let mut s = starter(700, 900);
    let (x, _) = at(&mut s, StarterAction::MakePage).expect("offered");
    assert!(x < 80);
    s.set_text_size(faraday_ui::TextSize::Larger);
    assert_eq!(s.class(), Class::Compact);
    let (_, y) = at(&mut s, StarterAction::MakePage).expect("offered");
    assert!(y > 900 - 80);
}

#[test]
fn learn_keeps_the_rail_or_the_bar_and_a_destination_leaves_it() {
    for (w, h) in [(400, 700), (700, 900), (1300, 800)] {
        let mut s = starter(w, h);
        tap(&mut s, StarterAction::Learn);
        assert!(s.showing_held(), "{w}");
        let t = s.drawn_texts();
        // The top app bar names it; the guides are under it.
        assert!(t.iter().any(|x| x == "Learn"), "{w}: {t:?}");
        assert!(
            t.iter()
                .any(|x| x == "Starting a Windows computer from the stick"),
            "{w}: {t:?}"
        );
        for d in DESTINATIONS {
            let (x, y) = at(&mut s, d).expect("a destination beside Learn");
            if w < 600 {
                assert!(y > h - 80, "{w}: {d:?} at {y}");
            } else {
                assert!(x < 80, "{w}: {d:?} at {x}");
            }
        }
        assert!(!s.offers_action(StarterAction::Back), "{w}");
        tap(&mut s, StarterAction::MakePage);
        assert!(!s.showing_held(), "{w}");
        assert_eq!(s.page, Page::Make, "{w}");
    }
}

#[test]
fn transfer_is_a_task_page_under_the_top_app_bar_and_back_returns_to_tools() {
    let mut s = starter(700, 900);
    tap(&mut s, StarterAction::ToolsPage);
    tap(&mut s, StarterAction::Transfer);
    assert!(s.showing_held());
    let t = s.drawn_texts();
    assert!(t.iter().any(|x| x == "Transfer"), "{t:?}");
    for d in DESTINATIONS {
        assert!(!s.offers_action(d), "{d:?} beside Transfer");
    }
    // The desktop app's own controls lie under the Starter's bar.
    let (_, y) = s
        .where_held_offers(faraday_core::Action::TransferReceive)
        .expect("Scan with the camera");
    assert!(y > 64, "{y}");
    tap(&mut s, StarterAction::Back);
    assert!(!s.showing_held());
    assert_eq!(s.page, Page::Tools);
}

/// On a window wider than the page needs, the page's cards stand in a
/// column centred in the room beside the rail, and the top bar's ⋮ stands
/// at the window's right edge (`docs/LAYOUT.md` §15, `docs/STARTER.md`
/// §3.1d).
#[test]
fn a_wide_window_centres_the_page_and_puts_the_bar_at_its_edge() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 1440)] {
        let mut s = starter(w, h);
        tap(&mut s, StarterAction::MakePage);
        let (menu, _) = at(&mut s, StarterAction::Menu).expect("the menu");
        assert!(
            f32::from(w) - f32::from(menu) <= 48.0,
            "{w}: the ⋮ at {menu}"
        );
        let (rail, column) = (80.0, 1040.0);
        let area = f32::from(w) - rail;
        let left = rail + ((area - column) / 2.0).max(0.0);
        for target in [Target::Stick, Target::Pi] {
            let (x, _) = at(&mut s, StarterAction::Make(target)).expect("a Make button");
            let x = f32::from(x);
            assert!(
                x >= left && x <= left + column,
                "{w}: {target:?} at {x} outside the column {left}..{}",
                left + column
            );
        }
    }
}
