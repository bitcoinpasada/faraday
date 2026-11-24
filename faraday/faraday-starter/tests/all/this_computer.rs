//! What Home's This computer card says about whether this computer can
//! start Faraday (`docs/STARTER.md` §3.1): a summary line and a check
//! for the start mode, Secure Boot and the camera, with the processor,
//! what is attached and BitLocker only when something is wrong.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::Starter;
use faraday_starter::api::{
    BitLocker, Camera, Computer, Concern, Kind, Os, Protection, StarterEvent,
};
use faraday_ui::TextSize;

fn display() -> Event {
    Event::Display(DisplayInfo {
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
    })
}

/// A computer with everything in order: UEFI, Secure Boot off, a camera
/// the stick can use.
fn good(os: Os) -> Computer {
    Computer {
        os,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(false),
        tpm_unlock: false,
        camera: Camera::Usable,
    }
}

/// What Home shows for `computer`.
fn home(computer: Computer) -> String {
    let mut starter = Starter::new();
    starter.event(display());
    starter.starter_event(StarterEvent::Computer(computer));
    starter.drawn_texts().join(" | ")
}

#[test]
fn a_computer_with_everything_in_order_can_start_faraday() {
    let t = home(good(Os::Linux));
    assert!(t.contains("This computer can start Faraday"), "{t}");
    assert!(!t.contains("without a camera"), "{t}");
    assert!(t.contains("Start mode | UEFI"), "{t}");
    assert!(t.contains("Secure Boot | Off"), "{t}");
    assert!(t.contains("Camera | Should work on the stick"), "{t}");
    assert!(!t.contains("Processor"), "{t}");
    assert!(!t.contains("| BitLocker | "), "{t}");
}

#[test]
fn secure_boot_on_is_a_step_and_the_computer_can_still_start() {
    let t = home(Computer {
        secure_boot: Some(true),
        ..good(Os::Linux)
    });
    assert!(t.contains("This computer can start Faraday"), "{t}");
    assert!(t.contains("Secure Boot | On"), "{t}");
}

#[test]
fn a_legacy_start_cannot_start_faraday() {
    let t = home(Computer {
        uefi: false,
        ..good(Os::Linux)
    });
    assert!(t.contains("This computer cannot start Faraday"), "{t}");
    assert!(t.contains("Legacy BIOS: not compatible"), "{t}");
}

#[test]
fn an_arm_computer_cannot_start_faraday() {
    let t = home(Computer {
        arm: true,
        ..good(Os::Windows)
    });
    assert!(t.contains("This computer cannot start Faraday"), "{t}");
    assert!(t.contains("Processor | ARM: not compatible"), "{t}");
}

#[test]
fn no_camera_says_files_go_by_stick() {
    let t = home(Computer {
        camera: Camera::None,
        ..good(Os::Linux)
    });
    assert!(
        t.replace(" | ", " ")
            .contains("This computer can start Faraday, without a camera"),
        "{t}"
    );
    assert!(
        t.replace(" | ", " ")
            .contains("None found: files go by stick"),
        "{t}"
    );
}

#[test]
fn an_unusable_camera_says_files_go_by_stick() {
    let t = home(Computer {
        camera: Camera::NotUsable,
        ..good(Os::Linux)
    });
    assert!(
        t.replace(" | ", " ")
            .contains("This computer can start Faraday, without a camera"),
        "{t}"
    );
    assert!(
        t.replace(" | ", " ")
            .contains("Not usable on the stick: files go by stick"),
        "{t}"
    );
}

#[test]
fn a_camera_that_could_not_be_read_is_not_called_missing() {
    let t = home(Computer {
        camera: Camera::Unknown,
        ..good(Os::Linux)
    });
    assert!(t.contains("This computer can start Faraday"), "{t}");
    assert!(!t.contains("without a camera"), "{t}");
    assert!(t.contains("Camera | Could not be checked"), "{t}");
}

#[test]
fn a_virtual_machine_is_listed_and_the_computer_cannot_start_faraday() {
    let mut starter = Starter::new();
    starter.event(display());
    starter.starter_event(StarterEvent::Computer(good(Os::Linux)));
    starter.starter_event(StarterEvent::Concerns(vec![Concern {
        kind: Kind::VirtualMachine,
        name: "Example Hypervisor".into(),
        id: "vm".into(),
    }]));
    let t = starter.drawn_texts().join(" | ");
    assert!(t.contains("This computer cannot start Faraday"), "{t}");
    assert!(t.contains("Example Hypervisor"), "{t}");
}

#[test]
fn bitlocker_on_is_a_step_on_windows_and_absent_when_off() {
    let mut starter = Starter::new();
    starter.event(display());
    starter.starter_event(StarterEvent::Computer(good(Os::Windows)));
    let drive = |protection| {
        Some(BitLocker {
            drive: "C:".into(),
            protection,
            starts_left: None,
        })
    };
    starter.starter_event(StarterEvent::BitLocker(drive(Protection::On)));
    let t = starter.drawn_texts().join(" | ");
    assert!(t.contains("This computer can start Faraday"), "{t}");
    assert!(t.contains("BitLocker | On"), "{t}");
    starter.starter_event(StarterEvent::BitLocker(drive(Protection::None)));
    let t = starter.drawn_texts().join(" | ");
    assert!(!t.contains("| BitLocker | "), "{t}");
}

/// A checklist row's title is shown whole in the narrow left pane, at
/// every text size: it wraps and the row grows, nothing is cut.
#[test]
fn a_long_checklist_title_shows_whole_at_every_text_size() {
    for size in [TextSize::Standard, TextSize::Large, TextSize::Larger] {
        let mut starter = Starter::new();
        starter.event(display());
        starter.set_text_size(size);
        starter.starter_event(StarterEvent::Computer(good(Os::Linux)));
        let texts = starter.drawn_texts();
        let t = texts.join(" ");
        assert!(
            t.contains("Start this computer from the stick"),
            "{size:?}: {t}"
        );
        assert!(!texts.iter().any(|x| x.contains('…')), "{size:?}: {t}");
    }
}
