//! A page whose content changes in place under a press (`docs/STARTER.md`
//! §3.1d): what the person pressed, or what was above it where it is
//! gone, stays where it was on screen, within the page's new length, so
//! the pointer is not left over another control and the next one needed
//! is in view; a press after the change lands on what is under the
//! pointer. Shown on the Start page of a Windows computer with BitLocker
//! on and another encryption product, at a compact and a large window.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};
use faraday_starter::api::{
    BitLocker, BitLockerStep, Computer, Os, Protection, Reason, StarterCommand, StarterEvent,
};
use faraday_starter::{Page, Starter, StarterAction};

/// A compact window and a large one, pixels.
const SIZES: [(u16, u16); 2] = [(400, 520), (1280, 480)];

fn bitlocker(protection: Protection) -> Option<BitLocker> {
    Some(BitLocker {
        drive: "C:".into(),
        protection,
        starts_left: Some(3),
    })
}

/// The Start page in a `w` × `h` window, on Windows with BitLocker on
/// and an encrypted drive Faraday does not manage, drawn.
fn start_page((w, h): (u16, u16)) -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width: w,
        height: h,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    s.starter_event(StarterEvent::Computer(Computer {
        os: Os::Windows,
        maker: "Dell Inc.".into(),
        model: "Latitude 5440".into(),
        family: "Latitude".into(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }));
    s.starter_event(StarterEvent::Disks(Vec::new()));
    s.starter_event(StarterEvent::OtherEncryption(Some("VeraCrypt".into())));
    s.press(StarterAction::Start);
    assert_eq!(s.page, Page::Start);
    s.starter_event(StarterEvent::BitLocker(bitlocker(Protection::On)));
    drawn(&mut s);
    commands(&mut s);
    s
}

fn drawn(s: &mut Starter) {
    let _ = s.drawn_texts();
}

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

/// Where `action` is offered, pixels, as last drawn.
fn at(s: &mut Starter, action: StarterAction) -> (u16, u16) {
    drawn(s);
    s.where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not in view"))
}

/// Turns the wheel until `action` is in view, then on until it sits a
/// fifth of the way down the window, so that the page is scrolled and
/// the control is still under the pointer.
fn scroll_up_to(s: &mut Starter, action: StarterAction, h: u16) {
    let wheel = |s: &mut Starter, dy: i32| {
        s.event(Event::Wheel {
            x: 10,
            y: h / 2,
            dy: dy as i16,
        });
        drawn(s);
    };
    for _ in 0..40 {
        drawn(s);
        if s.where_offered(action).is_some() {
            break;
        }
        wheel(s, 40);
    }
    let (_, y) = at(s, action);
    let dy = i32::from(y) - i32::from(h) / 5;
    if dy > 0 {
        wheel(s, dy);
    }
    assert!(
        s.scrolled() > 0.0,
        "{action:?} at {y}: the page is not scrolled"
    );
    at(s, action);
}

fn tap(s: &mut Starter, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        s.event(Event::Touch { x, y, phase });
    }
    drawn(s);
}

#[test]
fn confirming_the_rescue_keeps_what_was_above_it_and_brings_the_next_control_up_into_view() {
    for size in SIZES {
        let mut s = start_page(size);
        scroll_up_to(&mut s, StarterAction::Rescue, size.1);
        let pointer = at(&mut s, StarterAction::Rescue);
        let print = at(&mut s, StarterAction::PrintNotes);
        // The warning and its button go: what followed them moves up
        // into their place, in view, and nothing else moves under the
        // pointer.
        tap(&mut s, pointer);
        assert!(s.where_offered(StarterAction::Rescue).is_none());
        let now = at(&mut s, StarterAction::PrintNotes);
        assert!(now.1 < print.1, "{size:?}: Print from {print:?} to {now:?}");
        let under = s.offered_at(i32::from(pointer.0), i32::from(pointer.1));
        assert!(under.is_none(), "{size:?}: the pointer is now on {under:?}");
        // A press now lands on what is under the pointer.
        let suspend = at(&mut s, StarterAction::SuspendBitLocker);
        tap(&mut s, suspend);
        assert_eq!(
            commands(&mut s),
            [StarterCommand::BitLocker(BitLockerStep::Suspend)],
            "{size:?}"
        );
    }
}

#[test]
fn suspend_then_its_answer_keep_the_control_under_the_pointer_and_the_restart_in_view() {
    for size in SIZES {
        let mut s = start_page(size);
        s.press(StarterAction::Rescue);
        drawn(&mut s);
        scroll_up_to(&mut s, StarterAction::SuspendBitLocker, size.1);
        let pointer = at(&mut s, StarterAction::SuspendBitLocker);
        tap(&mut s, pointer);
        assert_eq!(
            commands(&mut s),
            [StarterCommand::BitLocker(BitLockerStep::Suspend)],
            "{size:?}"
        );
        // Windows answers: the drive is suspended, and Resume takes
        // Suspend's place.
        s.starter_event(StarterEvent::BitLockerDone {
            step: BitLockerStep::Suspend,
            result: Ok(()),
        });
        s.starter_event(StarterEvent::BitLocker(bitlocker(Protection::Suspended)));
        drawn(&mut s);
        assert_eq!(
            s.offered_at(i32::from(pointer.0), i32::from(pointer.1)),
            Some(StarterAction::ResumeBitLocker),
            "{size:?}"
        );
        assert!(
            s.where_offered(StarterAction::Restart).is_some(),
            "{size:?}: the restart is out of view"
        );
        tap(&mut s, pointer);
        assert_eq!(
            commands(&mut s),
            [StarterCommand::BitLocker(BitLockerStep::Resume)],
            "{size:?}"
        );
    }
}

#[test]
fn a_page_grown_shorter_while_scrolled_to_its_end_shows_its_end_and_not_its_top() {
    for size in SIZES {
        let mut s = start_page(size);
        s.event(Event::Wheel {
            x: 10,
            y: size.1 / 2,
            dy: i16::MAX,
        });
        drawn(&mut s);
        let end = s.scrolled();
        assert!(end > 0.0);
        // The warning goes, by its own button, then BitLocker's block.
        s.press(StarterAction::Rescue);
        s.starter_event(StarterEvent::BitLocker(None));
        drawn(&mut s);
        let now = s.scrolled();
        assert!(now > 0.0 && now < end, "{size:?}: {end} then {now}");
        // Already at the end: a further turn moves nothing.
        s.event(Event::Wheel {
            x: 10,
            y: size.1 / 2,
            dy: 200,
        });
        drawn(&mut s);
        assert_eq!(s.scrolled(), now, "{size:?}");
    }
}

#[test]
fn a_refusal_said_above_the_button_pressed_leaves_it_under_the_pointer() {
    for size in SIZES {
        let mut s = start_page(size);
        s.press(StarterAction::Rescue);
        drawn(&mut s);
        scroll_up_to(&mut s, StarterAction::SuspendBitLocker, size.1);
        let pointer = at(&mut s, StarterAction::SuspendBitLocker);
        tap(&mut s, pointer);
        commands(&mut s);
        // Windows refuses: why is said between the drive's row and the
        // button, and the button stays where it was pressed.
        s.starter_event(StarterEvent::BitLockerDone {
            step: BitLockerStep::Suspend,
            result: Err(Reason::NoAdmin),
        });
        let now = at(&mut s, StarterAction::SuspendBitLocker);
        assert!(
            now.1.abs_diff(pointer.1) <= 1,
            "{size:?}: Suspend from {pointer:?} to {now:?}"
        );
        tap(&mut s, pointer);
        assert_eq!(
            commands(&mut s),
            [StarterCommand::BitLocker(BitLockerStep::Suspend)],
            "{size:?}"
        );
    }
}
