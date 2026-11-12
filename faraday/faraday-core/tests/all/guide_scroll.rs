//! Scrolling the guide in its pane (wide window) or its sheet (compact
//! window) moves only the text: the frame after a scroll is what a full
//! redraw of the same state shows, and the pane's window stays where it is.

use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn window(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.event(display(width, height));
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Guide);
    let _ = app.frame();
    app
}

fn drain(app: &mut Faraday) {
    while app.poll_command().is_some() {}
}

fn frame(app: &mut Faraday) -> Vec<u8> {
    app.frame().rgba.to_vec()
}

fn display(w: u16, h: u16) -> Event {
    Event::Display(DisplayInfo {
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
    })
}

/// Scrolls the guide by wheel notches and by touchpad steps, and after each
/// compares the frame with the same state drawn in full again.
fn scrolled_frames_are_whole(w: u16, h: u16, at: (u16, u16)) {
    let mut app = window(w, h);
    let first = frame(&mut app);
    let _ = app.take_fade_counts();
    let mut moved = 0;
    let mut clock = 100_000u64;
    for i in 0..14 {
        let dy = if i % 3 == 0 { 25 } else { 70 };
        let ev = if i % 2 == 0 {
            Event::Scroll {
                x: at.0,
                y: at.1,
                dy,
            }
        } else {
            Event::Wheel {
                x: at.0,
                y: at.1,
                dy,
            }
        };
        app.event(ev);
        drain(&mut app);
        // Let a glide run to its end, a frame at a time as the shell draws.
        let mut now = frame(&mut app);
        for _ in 0..40 {
            clock += 16;
            app.event(Event::Tick { now_ms: clock });
            drain(&mut app);
            now = frame(&mut app);
        }
        assert_eq!(app.take_fade_counts().started, 0, "a scroll fades nothing");
        if now != first {
            moved += 1;
        }
        // The window's edge and corners stay where they are: no row at the
        // top or the foot of the screen differs from the first frame.
        let row = usize::from(w) * 4;
        let edge = 8 * row;
        assert!(
            now[..edge] == first[..edge] && now[now.len() - edge..] == first[first.len() - edge..],
            "{w}x{h} step {i}: the window's edge moved with the text"
        );
        app.event(display(w, h));
        drain(&mut app);
        let whole = frame(&mut app);
        assert!(
            now == whole,
            "{w}x{h} step {i}: the frame after the scroll is not the frame drawn in full ({} bytes differ)",
            now.iter().zip(&whole).filter(|(a, b)| a != b).count()
        );
    }
    assert!(moved > 0, "the guide scrolled");
}

#[test]
fn the_frame_after_a_scroll_in_the_pane_is_what_a_full_redraw_shows() {
    scrolled_frames_are_whole(1920, 1080, (1700, 540));
}

#[test]
fn the_frame_after_a_scroll_in_the_sheet_is_what_a_full_redraw_shows() {
    scrolled_frames_are_whole(480, 640, (240, 320));
}
