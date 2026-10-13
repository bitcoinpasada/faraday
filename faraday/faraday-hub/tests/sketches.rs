//! Faraday Starter draws the maker pages' firmware sketches
//! (`docs/HUB.md` §7.8, F18) where the page asks for them, and never
//! prints the marker.

use faraday_hub::{Hub, HubAction, LearnId};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn starter() -> Hub {
    let mut hub = Hub::new();
    hub.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    hub
}

/// Every string drawn while the page is scrolled from its top to its end.
fn seen_scrolling(hub: &mut Hub) -> Vec<String> {
    let mut seen = Vec::new();
    hub.held().learn.scroll = 0.0;
    loop {
        seen.extend(hub.drawn_texts());
        let learn = &mut hub.held().learn;
        if learn.scroll >= learn.max {
            return seen;
        }
        learn.scroll = (learn.scroll + 200.0).min(learn.max);
    }
}

#[test]
fn the_hp_page_shows_where_secure_boot_is_and_the_confirm_code() {
    let mut hub = starter();
    hub.press(HubAction::LearnPage(LearnId::Maker("maker-hp")));
    let seen = seen_scrolling(&mut hub);
    let shows = |s: &str| seen.iter().any(|t| t == s);
    // The Boot Options tab, its Secure Boot row turned to Disabled, the
    // save box, and the code to type.
    assert!(shows("Boot Options"));
    assert!(shows("→ [Disabled]"));
    assert!(shows("Save Changes and Exit?"));
    assert!(shows("7294 + ENTER"));
    assert!(!seen.iter().any(|t| t.trim_start().starts_with("::sketch")));
}

#[test]
fn a_boot_menu_sketch_marks_the_stick() {
    let mut hub = starter();
    hub.press(HubAction::LearnPage(LearnId::Maker("maker-lenovo-ideapad")));
    let seen = seen_scrolling(&mut hub);
    assert!(
        seen.iter().any(|t| t == "EFI USB Device (USB Flash Drive)"),
        "{seen:?}"
    );
    assert!(!seen.iter().any(|t| t.trim_start().starts_with("::sketch")));
}
