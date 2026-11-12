//! The toast is Material 3's snackbar (`docs/THEMES.md` §7, DESIGN
//! §4.11): in every theme, mode and contrast its text reads on its
//! container, and it goes when its time runs out.

use faraday_core::ui::Theme;
use faraday_core::{Faraday, StorageEvent};
use faraday_draw::canvas::InkKind;
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// The app on a 480 × 640 screen drawn in `theme`, a PDF just saved:
/// its toast up and fully shown.
fn toasted(theme: Theme) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.draw_in(theme);
    app.storage(StorageEvent::Printed {
        path: "plan.pdf".to_string(),
    });
    let mut clock = 1_000;
    for _ in 0..40 {
        clock += 16;
        app.event(Event::Tick { now_ms: clock });
        while app.poll_command().is_some() {}
        let _ = app.frame();
    }
    app
}

#[test]
fn a_toasts_text_reads_on_its_container_in_every_theme() {
    use crate::common::{over, ratio, under};
    let mut short = Vec::new();
    for theme in Theme::all() {
        let mut app = toasted(theme);
        let toast = app.toast_text().expect("the toast is up").to_string();
        let ink = app.drawn_ink();
        let Some((rect, color)) = ink.iter().find_map(|i| match &i.kind {
            InkKind::Text { text, color, .. } if *text == toast => Some((i.rect, *color)),
            _ => None,
        }) else {
            short.push(format!("{theme:?}: no toast drawn"));
            continue;
        };
        let frame = app.frame();
        let (w, h) = (i32::from(frame.width), i32::from(frame.height));
        let bg = under(frame.rgba, w, h, (rect.x, rect.y, rect.w, rect.h));
        let need = if theme.is_high_contrast() { 7.0 } else { 4.5 };
        let got = ratio(over(color, bg), bg);
        if got < need {
            short.push(format!("{theme:?}: the toast's text reads {got:.2}:1"));
        }
    }
    assert!(short.is_empty(), "{short:#?}");
}

/// Whether the frame on screen draws `text`.
fn draws(app: &mut Faraday, text: &str) -> bool {
    app.drawn_ink()
        .iter()
        .any(|i| matches!(&i.kind, InkKind::Text { text: t, .. } if t == text))
}

#[test]
fn a_toast_is_drawn_until_its_time_runs_out_and_not_after() {
    let mut app = toasted(Theme::all().next().expect("a theme"));
    let toast = app.toast_text().expect("the toast is up").to_string();
    // The toast came up at the save, at the app's first moment, before
    // `toasted`'s ticks began.
    let until = faraday_draw::tokens::TOAST_SHOW_MS;
    app.event(Event::Tick {
        now_ms: until - 200,
    });
    let _ = app.frame();
    assert!(draws(&mut app, &toast), "the toast shows as it fades");
    app.event(Event::Tick { now_ms: until });
    let _ = app.frame();
    assert!(
        !draws(&mut app, &toast),
        "faded out, nothing of it is drawn"
    );
}
