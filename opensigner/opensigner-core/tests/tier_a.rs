//! What a Tier A device says at start: the board has no secure boot
//! (security review 2026-09-11, M6).

mod common;

use common::{DESKTOP, Harness, PANEL, PHONE, SECURE_PHONE};
use opensigner_core::{AssuranceTier, ScreenKind, ids, strings::EN};

/// A Tier A device opens on the statement, says the fact, and goes to
/// Home on the one action it offers.
#[test]
fn a_tier_a_device_states_that_it_has_no_secure_boot_before_home() {
    let mut h = Harness::with_tier(PANEL, AssuranceTier::A, true);
    assert_eq!(h.app.screen(), ScreenKind::NoSecureBoot);
    let texts = h.app.texts();
    for want in [EN.no_secure_boot_result, EN.no_secure_boot_device] {
        assert!(texts.iter().any(|t| t == want), "{want:?}: {texts:?}");
    }
    assert!(
        h.app.rect_of(ids::BACK).is_none(),
        "the screen the device opens on has nowhere to go back to"
    );
    h.tap(ids::NO_SECURE_BOOT_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    // Home is where it stays: the statement is made once a start.
    h.tap(ids::at(ids::HOME_TILE_BASE, 5));
    assert_eq!(h.app.screen(), ScreenKind::Settings);
}

/// Every other tier runs on a device this is not true of, and opens on
/// Home.
#[test]
fn no_other_tier_shows_it() {
    for (display, tier) in [
        (SECURE_PHONE, AssuranceTier::B),
        (PHONE, AssuranceTier::B),
        (DESKTOP, AssuranceTier::C),
        (DESKTOP, AssuranceTier::D),
    ] {
        let h = Harness::with_tier(display, tier, true);
        assert_eq!(h.app.screen(), ScreenKind::Home, "{tier:?}");
    }
}
