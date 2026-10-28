//! What a Starter frame costs (`docs/SPEED.md` §11.2), in counts, not
//! time: on every page and sheet, a frame drawn again with nothing
//! changed, the pointer moved over the whole window, and the clock
//! ticking at rest do no costly work. The one allowance is a sheet's
//! kept frosted page put back, one whole-canvas copy a frame
//! (`Work::FullCanvas`); the blur that made it is done once, when the
//! sheet opens.

use faraday_core::work::{self, KINDS, Work};
use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware, TouchPhase};
use faraday_starter::api::{
    Bus, Computer, Concern, Disk, FaradayStick, InstallItem, Installed, Kind, Os, Payload,
    PayloadFile, StarterEvent,
};
use faraday_starter::decide::Target;
use faraday_starter::{Starter, StarterAction};

const GB: u64 = 1_000_000_000;
const W: u16 = 1100;
const H: u16 = 760;

fn payload() -> Payload {
    let file = |name: &str, inner: &str, build: &str, n: char| PayloadFile {
        name: name.into(),
        sha256: n.to_string().repeat(64),
        inner: inner.into(),
        inner_sha256: n.to_ascii_uppercase().to_string().repeat(64),
        inner_size: 3 * GB,
        build_id: build.into(),
    };
    Payload {
        version: "0.2.0".into(),
        commit: "4d0680b1a2b34d0680b1a2b34d0680b1a2b34d06".into(),
        files: vec![
            file(
                "faraday-stick-0.2.0-x86_64-uefi-strict.img.xz",
                "faraday-stick-0.2.0-x86_64-uefi-strict.img",
                "0.2.0",
                'a',
            ),
            file(
                "faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz",
                "faraday-pi-0.2.0-pi3-waveshare-28dpi.img",
                "0.2.0",
                'b',
            ),
        ],
    }
}

fn laptop(os: Os, secure_boot: bool) -> Computer {
    Computer {
        os,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: "Example".into(),
        uefi: true,
        arm: false,
        secure_boot: Some(secure_boot),
        tpm_unlock: false,
    }
}

fn disk(id: &str, faraday: bool) -> Disk {
    Disk {
        id: id.into(),
        device: format!("/dev/{id}"),
        vendor: "Maker".into(),
        model: "Flash".into(),
        serial: format!("SN-{id}"),
        size: 32 * GB,
        bus: Bus::Usb,
        volumes: Vec::new(),
        in_use: None,
        faraday: faraday.then(|| FaradayStick {
            release: Some("6.6.84-faraday-0.1.0".into()),
            read: true,
        }),
    }
}

/// A Starter on a Windows laptop with a payload, a Faraday stick and two
/// other disks attached.
fn starter() -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width: W,
        height: H,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    s.starter_event(StarterEvent::Computer(laptop(Os::Windows, true)));
    s.starter_event(StarterEvent::Payload(Some(payload())));
    s.starter_event(StarterEvent::Disks(vec![
        disk("sdb", false),
        disk("sdc", false),
        disk("sde", true),
    ]));
    s
}

/// Every page and sheet the test walks, by name.
fn states() -> Vec<(&'static str, Starter)> {
    let mut out = Vec::new();
    out.push(("Home", starter()));
    let mut s = starter();
    s.press(StarterAction::Next);
    out.push(("the restart sheet", s));
    let mut s = starter();
    s.press(StarterAction::Start);
    out.push(("Start", s));
    let mut s = starter();
    s.press(StarterAction::Start);
    s.press(StarterAction::Restart);
    out.push(("Start's restart sheet", s));
    let mut s = starter();
    s.press(StarterAction::Make(Target::Stick));
    out.push(("Which stick", s));
    let mut s = starter();
    s.press(StarterAction::About);
    out.push(("About", s));
    let mut s = starter();
    s.press(StarterAction::Templates);
    out.push(("Blank templates", s));
    let mut s = starter();
    s.press(StarterAction::Install);
    out.push(("Install", s));
    let mut s = starter();
    s.starter_event(StarterEvent::Installed(Some(Installed {
        version: "0.2.0".into(),
        items: vec![(InstallItem::App, "faraday-starter".into())],
    })));
    s.press(StarterAction::Installed);
    s.press(StarterAction::Uninstall);
    out.push(("the uninstall sheet", s));
    let mut s = starter();
    s.starter_event(StarterEvent::Concerns(vec![Concern {
        kind: Kind::Tv,
        name: "EXAMPLE TV".into(),
        id: "drm:card1-HDMI-A-1".into(),
    }]));
    let n = StarterAction::ContinueAnyway(s.concern_number("drm:card1-HDMI-A-1"));
    s.press(n);
    out.push(("the Continue anyway sheet", s));
    let mut s = starter();
    s.press(StarterAction::Transfer);
    out.push(("Transfer", s));
    let mut s = starter();
    s.press(StarterAction::Learn);
    out.push(("Learn", s));
    out
}

/// Takes the commands asked for and draws a frame if one was: whether
/// one was.
fn pass(s: &mut Starter) -> bool {
    let mut draw = false;
    while let Some(c) = s.poll_command() {
        draw |= c == Command::Draw;
    }
    if draw {
        let _ = s.frame();
    }
    draw
}

/// The costly work done since `before` beyond `frames` frames' worth of
/// kept-page copies, by kind; empty when there is none.
fn beyond(before: &work::Counts, frames: u64) -> Vec<(Work, u64)> {
    let done = work::since(before);
    KINDS
        .iter()
        .map(|&k| {
            let allowed = if k == Work::FullCanvas { frames } else { 0 };
            (k, done.of(k).saturating_sub(allowed))
        })
        .filter(|(_, n)| *n > 0)
        .collect()
}

/// A starter at rest: its opening frames drawn and its clock run until
/// nothing moves.
fn settled(mut s: Starter) -> Starter {
    let mut now = 0;
    for _ in 0..240 {
        now += 16;
        s.event(Event::Tick { now_ms: now });
        s.held().settle();
        pass(&mut s);
    }
    let _ = s.frame();
    s
}

#[test]
fn a_frame_drawn_again_does_no_costly_work() {
    for (name, s) in states() {
        let mut s = settled(s);
        let shown = s.frame().rgba.to_vec();
        // A press and a release where nothing is: the frame is asked
        // for and drawn again, and nothing on it changes. Over the held
        // app's sheet a press outside it closes it, so there the frame is
        // drawn again as a test reads its words.
        let mut frames = 0;
        let before = if s.showing_held() {
            let _ = s.drawn_texts();
            let before = work::snapshot();
            let _ = s.frame();
            frames += 1;
            before
        } else {
            let before = work::snapshot();
            for phase in [TouchPhase::Down, TouchPhase::Up] {
                s.event(Event::Touch {
                    x: W - 2,
                    y: H - 2,
                    phase,
                });
                frames += u64::from(pass(&mut s));
            }
            before
        };
        assert_eq!(beyond(&before, frames), [], "{name}");
        assert!(
            s.frame().rgba == shown.as_slice(),
            "{name}: the frame changed"
        );
    }
}

#[test]
fn moving_the_pointer_does_no_costly_work() {
    for (name, s) in states() {
        let mut s = settled(s);
        let before = work::snapshot();
        let mut frames = 0;
        for y in (0..H).step_by(24) {
            for x in (0..W).step_by(24) {
                s.event(Event::Hover { x, y });
                frames += u64::from(pass(&mut s));
            }
        }
        s.event(Event::HoverEnd);
        frames += u64::from(pass(&mut s));
        assert_eq!(beyond(&before, frames), [], "{name}");
    }
}

#[test]
fn at_rest_the_clock_asks_for_nothing() {
    for (name, s) in states() {
        let mut s = settled(s);
        let before = work::snapshot();
        let mut asked = 0;
        for i in 1..=120u64 {
            s.event(Event::Tick {
                now_ms: 10_000 + i * 16,
            });
            asked += u64::from(pass(&mut s));
        }
        assert_eq!(asked, 0, "{name}: a tick asked for a frame");
        assert_eq!(beyond(&before, 0), [], "{name}");
    }
}

#[test]
fn the_page_under_a_sheet_shows_what_changed_under_it() {
    // The restart sheet open on Home, then a wireless receiver plugged
    // in: Home under the sheet says so, as it does when the sheet is
    // opened after.
    let receiver = || {
        StarterEvent::Concerns(vec![Concern {
            kind: Kind::Receiver,
            name: "Example USB Receiver".into(),
            id: "usb:1-2".into(),
        }])
    };
    let mut s = starter();
    s.press(StarterAction::Next);
    let mut s = settled(s);
    let before = s.frame().rgba.to_vec();
    s.starter_event(receiver());
    pass(&mut s);
    let after = s.frame().rgba.to_vec();
    let mut t = starter();
    t.starter_event(receiver());
    t.press(StarterAction::Next);
    let mut t = settled(t);
    assert!(after != before, "the page under the sheet changed");
    assert!(t.frame().rgba == after.as_slice());
}
