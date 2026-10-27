//! Faraday Demo in the Starter's window, served as the shell serves it
//! (F126): the theme chosen in it is the Starter's, in memory only, with
//! nothing written to the computer for it.

use std::fs;
use std::path::{Path, PathBuf};

use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_core::{Action, Host};
use faraday_desktop::{Demo, Paths, desktop_app_in};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::{Starter, StarterAction};

/// Every file and folder under `root`, relative and sorted.
fn tree(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).expect("readable") {
            let p = e.expect("entry").path();
            out.push(p.strip_prefix(root).expect("under").display().to_string());
            if p.is_dir() {
                walk(&p, root, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// What the shell does after an event: the Demo's commands taken and its
/// storage served.
fn serve(s: &mut Starter, demo: &mut Demo) {
    while s.poll_demo_command().is_some() {}
    if let Some(host) = s.demo() {
        demo.took(&mut host.app);
        demo.settle(&mut host.app);
    }
    while s.poll_demo_command().is_some() {}
}

#[test]
fn a_theme_chosen_in_the_demo_is_the_starters_and_nothing_is_written_for_it() {
    let root: PathBuf =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("starter-demo-theme-not-stored");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Downloads")).expect("folder");
    let paths = Paths {
        sticks: root.join("faraday-sticks"),
        downloads: root.join("Downloads"),
        print: root.join("faraday-print"),
    };
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
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
    s.press(StarterAction::OpenDesktop);
    let mut demo = Demo::open(paths).expect("the stick folders");
    s.show_demo(Host::new(desktop_app_in(faraday_core::Mode::Demo)));
    if let Some(host) = s.demo() {
        demo.rescan(&mut host.app, true);
    }
    serve(&mut s, &mut demo);
    // What the Demo made when it opened: its stick folders.
    let before = tree(&root);

    let dark = Theme::new(ThemeName::Forest, Mode::Dark, Contrast::High);
    s.demo().expect("open").app.press(Action::Theme(dark));
    s.event(Event::Tick { now_ms: 1_000 });
    serve(&mut s, &mut demo);
    assert_eq!(s.theme(), dark);
    s.press(StarterAction::Back);
    drop(demo);
    assert_eq!(s.theme(), dark);
    assert_eq!(tree(&root), before);
    assert!(
        !before.iter().any(|f| f.contains("faraday-settings")),
        "{before:?}"
    );
    let _ = fs::remove_dir_all(&root);
}
