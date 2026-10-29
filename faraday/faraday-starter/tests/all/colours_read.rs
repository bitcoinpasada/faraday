//! What the Starter writes reads against what is under it, as drawn
//! (`docs/THEMES.md` §11.2): its main pages and sheets rendered in every
//! scheme, wide and narrow, and every opaque line of text measured in the
//! colour it was drawn in against the pixels round it, 4.5:1 or more
//! (7:1 at High contrast). A control not available now is see-through and
//! not held to the ratio (§8).

use std::collections::HashMap;

use faraday_draw::Color;
use faraday_draw::canvas::InkKind;
use faraday_shell_api::{App, Event, TouchPhase};
use faraday_starter::api::{
    BitLocker, Concern, DevImage, Disk, Kind, Mode, Protection, Reason, StarterEvent,
};
use faraday_starter::decide::Target;
use faraday_starter::{Starter, StarterAction};
use faraday_ui::Theme;

use super::starter::{disk, display, faraday_stick, linux, payload, windows};

const GB: u64 = 1_000_000_000;

/// WCAG relative luminance.
fn luminance(c: Color) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio.
fn ratio(a: Color, b: Color) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The colour most of the pixels just outside the box (x, y, w, h) are:
/// what a text was drawn on.
fn under(rgba: &[u8], width: i32, height: i32, (x, y, w, h): (i32, i32, i32, i32)) -> Color {
    let mut seen: HashMap<(u8, u8, u8), usize> = HashMap::new();
    let mut take = |px: i32, py: i32| {
        if px >= 0 && py >= 0 && px < width && py < height {
            let i = ((py * width + px) * 4) as usize;
            *seen.entry((rgba[i], rgba[i + 1], rgba[i + 2])).or_default() += 1;
        }
    };
    for px in x - 1..=x + w {
        take(px, y - 1);
        take(px, y + h);
    }
    for py in y..y + h {
        take(x - 1, py);
        take(x + w, py);
    }
    let ((r, g, b), _) = seen
        .into_iter()
        .max_by_key(|&(c, n)| (n, c))
        .expect("a box on the frame");
    Color::rgb(r, g, b)
}

/// Every opaque line of text the frame draws: what it reads, how far it
/// stands out from the pixels round it, and the two colours.
fn all_text(starter: &mut Starter) -> Vec<(String, f64, Color, Color)> {
    let ink = starter.drawn_ink();
    let f = starter.frame();
    let (width, height) = (i32::from(f.width), i32::from(f.height));
    ink.into_iter()
        .filter_map(|i| match i.kind {
            InkKind::Text { text, color, .. } if color.a == 255 && !text.trim().is_empty() => {
                let bg = under(
                    f.rgba,
                    width,
                    height,
                    (i.rect.x, i.rect.y, i.rect.w, i.rect.h),
                );
                Some((text, ratio(color, bg), color, bg))
            }
            _ => None,
        })
        .collect()
}

fn concern(kind: Kind, name: &str, id: &str) -> Concern {
    Concern {
        kind,
        name: name.into(),
        id: id.into(),
    }
}

fn many() -> Vec<Disk> {
    vec![
        disk("sdb", "DataTraveler", 64 * GB),
        disk("sdc", "Elements", 2_000 * GB),
        faraday_stick("sde", "Ultra Fit", 32 * GB, Some("6.6.84-faraday-0.1.0")),
    ]
}

/// A Starter `w` by `h` drawn in `theme`, on a Windows or a Linux
/// computer, with `many()` attached.
fn start(w: u16, h: u16, theme: Theme, windows_pc: bool) -> Starter {
    let mut s = Starter::new();
    s.set_theme(theme);
    s.event(display(w, h, 160));
    let pc = if windows_pc {
        windows(true)
    } else {
        linux(true)
    };
    s.starter_event(StarterEvent::Computer(pc));
    s.starter_event(StarterEvent::Payload(Some(payload("0.2.0"))));
    s.starter_event(StarterEvent::Disks(many()));
    s
}

/// A finger put down on `action` and held there.
fn hold(s: &mut Starter, action: StarterAction) {
    let _ = s.drawn_texts();
    let (x, y) = s.where_offered(action).expect("offered");
    s.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
}

/// To Confirm for the plain disk, writing everything.
fn to_confirm(s: &mut Starter) {
    s.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(s.disk_number("sdb"));
    s.press(n);
    s.press(StarterAction::Mode(Mode::Whole));
    s.press(StarterAction::Continue);
}

/// The pages and sheets, each as a person reaches them: Home with a
/// concern and the boot entry; Make; Tools; the disk list, What to
/// write, Confirm (with a development image's warning) and the writer's
/// refusal; Start with BitLocker on and its restart sheet; Install;
/// About; Blank templates; Appearance. Two of them with a
/// finger held down on a button or a row.
fn scenes(w: u16, h: u16, theme: Theme) -> Vec<(&'static str, Starter)> {
    let mut out = Vec::new();
    let mut home = start(w, h, theme, true);
    home.starter_event(StarterEvent::Concerns(vec![concern(
        Kind::Kvm,
        "PiKVM",
        "usb:1-2",
    )]));
    home.starter_event(StarterEvent::BootEntry(true));
    out.push(("home", home));

    let mut pressed = start(w, h, theme, false);
    hold(&mut pressed, StarterAction::Next);
    out.push(("home, a button held", pressed));

    let mut make = start(w, h, theme, false);
    make.press(StarterAction::MakePage);
    out.push(("make", make));
    let mut tools = start(w, h, theme, false);
    tools.press(StarterAction::ToolsPage);
    out.push(("tools", tools));

    let mut disks = start(w, h, theme, false);
    disks.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(disks.disk_number("sdb"));
    hold(&mut disks, n);
    out.push(("disks, a row held", disks));

    let mut mode = start(w, h, theme, false);
    mode.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(mode.disk_number("sde"));
    mode.press(n);
    mode.press(StarterAction::Mode(Mode::KeepData));
    out.push(("mode", mode));

    let mut confirm = start(w, h, theme, false);
    confirm.starter_event(StarterEvent::DevImage(DevImage {
        path: "out/stick/faraday-x86_64-uefi-strict-dev.img".into(),
        sha256: "9c1e44d0".repeat(8),
        size: 202_395_648,
        boot_only: false,
        build_id: Some("0.2.0+4d0680b1a2b3.dev".into()),
    }));
    to_confirm(&mut confirm);
    out.push(("confirm", confirm));

    let mut failed = start(w, h, theme, false);
    to_confirm(&mut failed);
    failed.press(StarterAction::Write);
    failed.starter_event(StarterEvent::Written(Err(Reason::BackDiffers {
        device: "/dev/sdb".into(),
        at: 1_200_000_000,
    })));
    out.push(("write failed", failed));

    let mut begin = start(w, h, theme, true);
    begin.press(StarterAction::Start);
    begin.starter_event(StarterEvent::BitLocker(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        starts_left: None,
    })));
    out.push(("start, BitLocker on", begin));

    let mut restart = start(w, h, theme, false);
    restart.press(StarterAction::Start);
    restart.press(StarterAction::Restart);
    out.push(("restart sheet", restart));

    let mut install = start(w, h, theme, false);
    install.press(StarterAction::Install);
    out.push(("install", install));
    let mut about = start(w, h, theme, false);
    about.press(StarterAction::About);
    out.push(("about", about));
    let mut templates = start(w, h, theme, false);
    templates.press(StarterAction::Templates);
    out.push(("blank templates", templates));
    let mut look = start(w, h, theme, false);
    look.press(StarterAction::Menu);
    look.press(StarterAction::Appearance);
    out.push(("appearance", look));
    out
}

/// In every scheme, wide and narrow, every opaque line of text on every
/// one of those pages reads, 4.5:1 (7:1 at High contrast).
#[test]
fn what_the_starter_writes_reads_on_its_fills_in_every_scheme() {
    // `scenes` keeps each of its Starters in a frame of its own: the test's
    // default stack is all they fill, so it runs on one of its own.
    std::thread::Builder::new()
        .stack_size(32 << 20)
        .spawn(fills_read)
        .expect("a thread")
        .join()
        .expect("every line reads");
}

fn fills_read() {
    let mut short = Vec::new();
    for theme in Theme::all() {
        let need = if theme.is_high_contrast() { 7.0 } else { 4.5 };
        for (w, h) in [(1280, 800), (480, 800)] {
            for (scene, mut starter) in scenes(w, h, theme) {
                let drawn = all_text(&mut starter);
                assert!(!drawn.is_empty(), "{theme:?} {scene}: draws text");
                for (text, got, ink, bg) in drawn {
                    if got < need {
                        short.push(format!(
                            "{theme:?} {w}x{h} {scene}: {text:?} is {got:.2}:1 ({:02x}{:02x}{:02x} on {:02x}{:02x}{:02x})",
                            ink.r, ink.g, ink.b, bg.r, bg.g, bg.b
                        ));
                    }
                }
            }
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}
