//! Faraday Desktop's stick pills in its top bar (`docs/LAYOUT.md` §3):
//! no strip above the bar, the wordmark at the window's top, the pills
//! left of Load samples, the mode pill and the ?, the sticks that do not
//! fit folded into the Sticks menu, the boot stick's pill always shown,
//! and nothing in the bar drawn over anything else.

use crate::common::*;
use faraday_core::{Action, BOOT_FOLDER, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

type Box4 = (i32, i32, i32, i32);

fn at(app: &mut Faraday, w: u16, h: u16) {
    app.event(Event::Display(DisplayInfo {
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
}

/// Faraday Desktop as it starts, in Demo with its samples, with `sticks`
/// in (the boot stick, Stick 2 and on), the window `w` × `h`.
fn desktop(shell: &mut Shell, sticks: usize, (w, h): (u16, u16)) -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.shell_sticks = true;
    app.online = true;
    testkit::into_demo(&mut app);
    app.samples = Some(testkit::demo_samples());
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    for n in 1..=sticks {
        let folder = if n == 1 {
            BOOT_FOLDER.to_string()
        } else {
            format!("STICK-{n}")
        };
        let mut d = Disk::empty(&folder, &folder);
        d.boot = n == 1;
        shell.add(d);
        shell.plug(&mut app, &folder);
    }
    at(&mut app, w, h);
    app.press(Action::Nav(Screen::Home));
    app.settle();
    app
}

fn boxed(app: &Faraday, a: Action) -> Option<Box4> {
    let b = app.hit_box(a)?;
    Some((
        i32::from(b.0),
        i32::from(b.1),
        i32::from(b.2),
        i32::from(b.3),
    ))
}

fn click(app: &mut Faraday, a: Action) {
    let _ = app.frame();
    let (x, y) = app.where_offered(a).expect("offered");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
}

/// The shell of the desktop app, in little: a folder pressed is plugged
/// in when it is out and pulled when it is in.
fn shell_follows(shell: &mut Shell, app: &mut Faraday) {
    for folder in app.take_stick_toggles() {
        if shell.present.contains(&folder) {
            shell.pull(app, &folder);
        } else {
            if !shell.disks.iter().any(|d| d.id == folder) {
                shell.add(Disk::empty(&folder, &folder));
            }
            shell.plug(app, &folder);
        }
    }
}

fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts()
}

/// The pills' boxes as drawn: every stick's and the Sticks pill.
fn pill_boxes(app: &Faraday) -> Vec<Box4> {
    (1..=8)
        .map(Action::StickPill)
        .chain([Action::SticksMenu])
        .filter_map(|a| boxed(app, a))
        .collect()
}

/// Whether the frame has ink other than the bar's own colour in the box.
fn inked(app: &mut Faraday, (x0, y0, x1, y1): (usize, usize, usize, usize)) -> bool {
    let f = app.frame();
    let w = usize::from(f.width);
    let at = |x: usize, y: usize| {
        let i = (y * w + x) * 4;
        [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]]
    };
    let ground = at(x1, y0);
    (y0..y1).any(|y| (x0..x1).any(|x| at(x, y) != ground))
}

#[test]
fn the_pills_stand_in_the_top_bar_left_of_load_samples_with_no_strip_above() {
    for size in [(1280, 800), (1920, 1080)] {
        let mut shell = Shell::new();
        let mut app = desktop(&mut shell, 2, size);
        let t = texts(&mut app);
        assert!(t.iter().any(|s| s == "Boot stick · in"), "{size:?} {t:?}");
        assert!(t.iter().any(|s| s == "Stick 2 · in"), "{size:?} {t:?}");
        let samples = boxed(&app, Action::Samples).expect("Load samples");
        let mode = boxed(&app, Action::ModeSheet).expect("the mode pill");
        let boot = boxed(&app, Action::StickPill(1)).expect("the boot stick's pill");
        let two = boxed(&app, Action::StickPill(2)).expect("Stick 2's pill");
        // One row: the pills level with the mode pill, from the top of
        // the window as it is, the boot stick first, then Stick 2, then
        // Load samples.
        assert_eq!((boot.1, boot.3), (mode.1, mode.3), "{size:?}");
        assert_eq!((two.1, two.3), (mode.1, mode.3), "{size:?}");
        assert!(boot.0 + boot.2 < two.0, "{size:?} {boot:?} {two:?}");
        assert!(two.0 + two.2 < samples.0, "{size:?} {two:?} {samples:?}");
        assert!(samples.0 + samples.2 < mode.0, "{size:?}");
        // Nothing stands above them, and the name heads the sidebar at
        // the window's top.
        assert!(boot.1 < 20, "{size:?} {boot:?}");
        assert!(inked(&mut app, (58, 0, 220, 56)), "{size:?}: the wordmark");
    }
}

#[test]
fn on_a_rail_window_the_wordmark_and_the_pills_share_one_bar_at_the_top() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell, 2, (960, 580));
    let _ = app.frame();
    let mode = boxed(&app, Action::ModeSheet).expect("the mode pill");
    let boot = boxed(&app, Action::StickPill(1)).expect("the boot stick's pill");
    let home = boxed(&app, Action::Nav(Screen::Home)).expect("the rail's Home");
    // The top app bar starts at the window's top: the pills are centred in
    // its 64 dp, level with the mode pill and the rail's first item.
    assert_eq!(boot.1 + boot.3 / 2, 32, "{boot:?}");
    assert_eq!(boot.1, mode.1);
    assert!(home.1 < 32 && home.1 + home.3 > 32, "{home:?}");
    // The name, at the bar's left beside the rail, inside the bar.
    assert!(inked(&mut app, (110, 0, 300, 64)), "the wordmark");
    assert!(
        !inked(&mut app, (110, 64, 300, 70)),
        "a strip pushes the bar down"
    );
}

#[test]
fn with_five_sticks_the_rest_fold_into_the_sticks_menu_which_plugs_them() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell, 5, (1280, 800));
    let t = texts(&mut app);
    assert!(t.iter().any(|s| s == "Boot stick · in"), "{t:?}");
    let more: Vec<&String> = t.iter().filter(|s| s.starts_with("Sticks · ")).collect();
    assert_eq!(more.len(), 1, "{t:?}");
    assert!(
        !t.iter().any(|s| s == "Stick 5 · in"),
        "Stick 5 folded: {t:?}"
    );
    // The menu lists every stick, each in or out.
    click(&mut app, Action::SticksMenu);
    assert_eq!(app.sheet, Some(Sheet::SticksMenu));
    app.settle();
    let t = texts(&mut app);
    for want in [
        "Boot stick · in",
        "Stick 2 · in",
        "Stick 3 · in",
        "Stick 4 · in",
        "Stick 5 · in",
    ] {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
    // Stick 4 chosen: pulled, and the menu closes.
    click(&mut app, Action::StickPill(4));
    shell_follows(&mut shell, &mut app);
    assert_eq!(app.sheet, None);
    click(&mut app, Action::SticksMenu);
    app.settle();
    let t = texts(&mut app);
    assert!(t.iter().any(|s| s == "Stick 4 · out"), "{t:?}");
    // And put back from it.
    click(&mut app, Action::StickPill(4));
    shell_follows(&mut shell, &mut app);
    assert!(shell.present.contains(&"STICK-4".to_string()));
    // The bar never takes a second row: every pill on one line.
    let _ = app.frame();
    let rows: Vec<i32> = pill_boxes(&app).iter().map(|b| b.1).collect();
    assert!(rows.windows(2).all(|w| w[0] == w[1]), "{rows:?}");
}

#[test]
fn the_boot_sticks_pill_shows_at_every_size_and_narrow_it_opens_the_menu() {
    for size in [(480, 640), (960, 580), (1280, 800), (1920, 1080)] {
        let mut shell = Shell::new();
        let mut app = desktop(&mut shell, 5, size);
        let _ = app.frame();
        let boxes = app.drawn_text_boxes();
        let boot = boxes
            .iter()
            .find(|(t, _)| t.starts_with("Boot stick"))
            .unwrap_or_else(|| panic!("{size:?}: no boot stick pill in {boxes:?}"));
        assert!(boot.1.1 < 64, "{size:?}: {boot:?} is not in the bar");
    }
    // On a narrow window the boot stick's pill stands alone and opens the
    // menu of every stick.
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell, 5, (480, 640));
    let t = texts(&mut app);
    assert!(!t.iter().any(|s| s.starts_with("Stick 2")), "{t:?}");
    click(&mut app, Action::SticksMenu);
    assert_eq!(app.sheet, Some(Sheet::SticksMenu));
    let t = texts(&mut app);
    assert!(t.iter().any(|s| s == "Stick 4 · in"), "{t:?}");
}

#[test]
fn the_menu_closes_at_once_under_reduce_motion_and_moves_out_otherwise() {
    for reduce in [true, false] {
        let mut shell = Shell::new();
        let mut app = desktop(&mut shell, 5, (1280, 800));
        app.press(Action::ReduceMotion(reduce));
        app.event(Event::Tick { now_ms: 1_000 });
        click(&mut app, Action::SticksMenu);
        app.event(Event::Tick { now_ms: 2_000 });
        app.event(Event::Tick { now_ms: 3_000 });
        click(&mut app, Action::Cancel);
        assert_eq!(app.sheet, None);
        app.event(Event::Tick { now_ms: 3_010 });
        let t = texts(&mut app);
        let shown = t.iter().any(|s| s == "Stick 5 · in");
        assert_eq!(shown, !reduce, "reduce motion {reduce}: {t:?}");
        app.event(Event::Tick { now_ms: 4_000 });
        app.event(Event::Tick { now_ms: 4_016 });
        let t = texts(&mut app);
        assert!(!t.iter().any(|s| s == "Stick 5 · in"), "{t:?}");
    }
}

#[test]
fn nothing_in_the_top_bar_is_drawn_over_anything_else() {
    let pages: [&dyn Fn(&mut Faraday); 9] = [
        &|a| a.press(Action::Nav(Screen::Home)),
        &|a| a.press(Action::Nav(Screen::Files)),
        &|a| a.press(Action::Nav(Screen::Settings)),
        &|a| a.press(Action::Nav(Screen::WalletsStart)),
        &|a| a.press(Action::Nav(Screen::Vaults)),
        &|a| a.press(Action::Nav(Screen::Catalog)),
        &|a| a.press(Action::Nav(Screen::Restore)),
        &|a| a.press(Action::Entry(None)),
        &|a| {
            a.press(Action::Nav(Screen::Home));
            a.press(Action::CreateWallet);
        },
    ];
    for size in [(480, 640), (960, 580), (1280, 800), (1920, 1080)] {
        for sticks in [2, 5] {
            let mut shell = Shell::new();
            let mut app = desktop(&mut shell, sticks, size);
            for (k, open) in pages.iter().enumerate() {
                open(&mut app);
                app.settle();
                let _ = app.frame();
                let what = format!("{size:?}, {sticks} sticks, page {k} ({:?})", app.screen);
                let pills = pill_boxes(&app);
                assert!(!pills.is_empty(), "{what}: no pills");
                for p in &pills {
                    assert!(p.0 >= 0 && p.0 + p.2 <= i32::from(size.0), "{what}: {p:?}");
                }
                let mut others: Vec<Box4> = [Action::Samples, Action::ModeSheet, Action::Learn]
                    .into_iter()
                    .filter_map(|a| boxed(&app, a))
                    .filter(|b| b.1 < 64)
                    .collect();
                others.extend(pills.iter().copied());
                // Every text in the bar's band stands wholly in one control or
                // meets none.
                for (t, r) in app.drawn_text_boxes() {
                    if r.1 >= 64 {
                        continue;
                    }
                    for z in &others {
                        let inside = r.0 >= z.0
                            && r.0 + r.2 <= z.0 + z.2
                            && r.1 >= z.1
                            && r.1 + r.3 <= z.1 + z.3;
                        let meets = r.0 < z.0 + z.2
                            && z.0 < r.0 + r.2
                            && r.1 < z.1 + z.3
                            && z.1 < r.1 + r.3;
                        assert!(inside || !meets, "{what}: {t:?} {r:?} meets {z:?}");
                    }
                }
                // No two controls of the bar overlap.
                for (i, a) in others.iter().enumerate() {
                    for b in &others[i + 1..] {
                        let meets = a.0 < b.0 + b.2
                            && b.0 < a.0 + a.2
                            && a.1 < b.1 + b.3
                            && b.1 < a.1 + a.3;
                        assert!(!meets || a == b, "{what}: {a:?} meets {b:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn at_the_pi_panels_size_the_pills_keep_their_strip_whole_inside_the_window() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell, 5, (480, 640));
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for page in [Screen::Home, Screen::Visit, Screen::Files] {
        app.press(Action::Nav(page));
        let _ = app.frame();
        let pills = pill_boxes(&app);
        assert_eq!(pills.len(), 5, "{page:?}: every stick's pill");
        for p in &pills {
            assert!(p.0 >= 0 && p.0 + p.2 <= 480, "{page:?}: {p:?}");
        }
        click(&mut app, Action::StickPill(3));
        shell_follows(&mut shell, &mut app);
        click(&mut app, Action::StickPill(3));
        shell_follows(&mut shell, &mut app);
        assert!(shell.present.contains(&"STICK-3".to_string()));
    }
}

#[test]
fn on_a_device_there_is_no_pill_in_the_bar() {
    for size in [(1280, 800), (480, 640)] {
        let mut app = process(Vec::new(), Vec::new(), Vec::new());
        at(&mut app, size.0, size.1);
        let t = texts(&mut app);
        assert!(!t.iter().any(|s| s.starts_with("Boot stick")), "{t:?}");
        assert!(!t.iter().any(|s| s.starts_with("Sticks · ")), "{t:?}");
        assert!(pill_boxes(&app).is_empty());
    }
}
