//! The app's name and its mark are filled from their outlines once and
//! kept by the canvas (`docs/SPEED.md` §3.8): a frame drawn again fills
//! none of them, and what is kept follows what it is drawn from. A new
//! theme colours what was kept and fills nothing; a new display fills
//! the name and the mark once more, at its own size.

use faraday_shell_api::App;

use super::{DESKTOP, PI, Panel, show};
use crate::{Action, Faraday, Screen};

/// A large window at scale 2.
const BIG: Panel = Panel {
    width: 2560,
    height: 1600,
    dpi: 320,
};

/// The shapes the app's canvas has filled from their outlines.
fn filled(app: &Faraday) -> u64 {
    app.canvas.as_ref().map_or(0, |c| c.outlines_filled())
}

/// Draws a frame as a shell does, motion settled first; returns the
/// shapes it filled from their outlines and its pixels.
fn frame(app: &mut Faraday) -> (u64, Vec<u8>) {
    app.settle();
    let before = filled(app);
    app.out.dirty = true;
    let rgba = app.frame().rgba.to_vec();
    (filled(app) - before, rgba)
}

#[test]
fn a_frame_drawn_again_fills_the_name_and_the_mark_from_what_was_kept() {
    let mut app = crate::testkit::started();
    for (name, panel) in [("desktop", DESKTOP), ("Pi", PI), ("large", BIG)] {
        show(&mut app, panel);
        app.press(Action::Nav(Screen::Home));
        let _ = frame(&mut app);
        assert!(
            filled(&app) > 0,
            "{name}: Home's first frame on a new display fills the name and the mark"
        );
        for screen in [Screen::Wallets, Screen::Settings, Screen::Home] {
            app.press(Action::Nav(screen));
            let _ = frame(&mut app);
            let (n, _) = frame(&mut app);
            assert_eq!(
                n, 0,
                "{name}, {screen:?}: a frame drawn again fills nothing"
            );
        }
    }
}

#[test]
fn a_new_theme_colours_the_kept_name_and_fills_nothing() {
    let mut app = crate::testkit::started();
    show(&mut app, DESKTOP);
    app.press(Action::Nav(Screen::Home));
    let _ = frame(&mut app);
    let (_, light) = frame(&mut app);
    app.press(Action::Theme(crate::ui::Theme {
        mode: crate::ui::Mode::Dark,
        ..crate::ui::Theme::default()
    }));
    let (n, dark) = frame(&mut app);
    assert_eq!(n, 0, "a new theme fills no outline");
    assert_ne!(light, dark, "the new theme is drawn");
    let (n, again) = frame(&mut app);
    assert_eq!((n, &again), (0, &dark), "and is drawn the same again");
}
