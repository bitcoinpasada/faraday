//! Settings' Guided (`docs/DESIGN.md` §5, `docs/LAYOUT.md` §17): the
//! flows' Steps only or Guided, offered on Settings too; with Guided on
//! each card carries a short explainer under its title, with it off the
//! cards carry their labels, values and actions only; and it is the one
//! setting the flows read.

use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// A desktop window `width` × `height` at 160 dpi, one pixel a unit.
fn on(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// Every card's explainer, by its first words (a line may wrap after
/// them).
fn explainers() -> Vec<String> {
    [
        EN.settings_guide_mode,
        EN.settings_guide_session,
        EN.settings_guide_time,
        EN.settings_guide_randomness,
        EN.settings_guide_appearance,
        EN.settings_guide_scale,
        EN.settings_guide_signed,
        EN.settings_guide_stick,
        EN.settings_guide_about,
    ]
    .iter()
    .map(|s| s.split(' ').take(3).collect::<Vec<_>>().join(" "))
    .collect()
}

/// What the frame draws, its lines joined.
fn texts(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.drawn_texts().join(" ")
}

/// Presses `a` where the screen offers it.
fn choose(app: &mut Faraday, a: Action) {
    let _ = app.frame();
    assert!(app.offers(a), "{a:?} offered");
    app.press(a);
}

#[test]
fn guided_on_settings_shows_an_explainer_on_every_card_and_off_none() {
    // Tall enough for the whole page at once.
    let mut app = on(1280, 2000);
    app.press(Action::Nav(Screen::Settings));
    choose(&mut app, Action::Guided(false));
    let off = texts(&mut app);
    for e in explainers() {
        assert!(!off.contains(&e), "no {e:?} with Steps only");
    }
    choose(&mut app, Action::Guided(true));
    let on = texts(&mut app);
    for e in explainers() {
        assert!(on.contains(&e), "{e:?} with Guided: {on}");
    }
}

#[test]
fn guided_on_the_pi_panel_shows_the_open_sections_explainer() {
    let mut app = Faraday::new();
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
    app.press(Action::Nav(Screen::Settings));
    choose(&mut app, Action::Guided(true));
    crate::common::settings_section(&mut app, "Display scale");
    let scale = explainers()[5].clone();
    assert!(
        texts(&mut app).contains(&scale),
        "Display scale's explainer"
    );
    choose(&mut app, Action::Guided(false));
    assert!(!texts(&mut app).contains(&scale), "gone with Steps only");
}

#[test]
fn settings_guided_and_a_flows_guided_are_one_setting() {
    use faraday_core::learn::Topic;
    let mut app = on(1280, 800);
    // New key opens on Length, whose Guided text links its Learn page.
    let flow_guided = |app: &mut Faraday| {
        app.press(Action::KeyGen(None));
        let _ = app.frame();
        app.offers(Action::LearnTopic(Topic::Words))
    };
    // Steps only from Settings: the flow shows no Guided text.
    app.press(Action::Nav(Screen::Settings));
    choose(&mut app, Action::Guided(false));
    assert!(!flow_guided(&mut app), "Steps only, set from Settings");
    // Guided from Settings: the flow shows its Guided text.
    app.press(Action::Nav(Screen::Settings));
    choose(&mut app, Action::Guided(true));
    assert!(flow_guided(&mut app), "Guided, set from Settings");
    // Steps only from the flow's own control: Settings' cards carry no
    // explainer.
    choose(&mut app, Action::Guided(false));
    app.press(Action::Nav(Screen::Settings));
    let settings = texts(&mut app);
    assert!(!settings.contains(&explainers()[1]), "set from the flow");
}
