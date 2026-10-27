//! Light or Dark at start (F126): what each system's own setting answers,
//! as the readers get it, and the theme the Starter opens in.

use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_starter_shell::scheme;

#[test]
fn the_starter_opens_in_cobalt_dark_where_the_computer_asks_for_dark_and_light_otherwise() {
    assert_eq!(
        scheme::at_start(Some(true)),
        Theme::new(ThemeName::Cobalt, Mode::Dark, Contrast::Standard)
    );
    for said in [Some(false), None] {
        assert_eq!(
            scheme::at_start(said),
            Theme::new(ThemeName::Cobalt, Mode::Light, Contrast::Standard)
        );
    }
}

/// The XDG desktop portal's `color-scheme`, as the specification numbers
/// it.
#[test]
fn the_desktop_portals_color_scheme_is_read_as_its_specification_numbers_it() {
    assert_eq!(scheme::portal(1), Some(true));
    assert_eq!(scheme::portal(2), Some(false));
    assert_eq!(scheme::portal(0), None);
    assert_eq!(scheme::portal(7), None);
}

/// What `gsettings get org.gnome.desktop.interface …` prints, newline and
/// quotes included.
#[test]
fn gnomes_own_settings_are_read_as_gsettings_prints_them() {
    assert_eq!(scheme::gnome_color_scheme("'prefer-dark'\n"), Some(true));
    assert_eq!(scheme::gnome_color_scheme("'prefer-light'\n"), Some(false));
    assert_eq!(scheme::gnome_color_scheme("'default'\n"), None);
    assert_eq!(scheme::gnome_color_scheme(""), None);
    assert_eq!(scheme::gtk_theme("'Adwaita-dark'\n"), Some(true));
    assert_eq!(scheme::gtk_theme("'Yaru:dark'\n"), Some(true));
    assert_eq!(scheme::gtk_theme("'Adwaita'\n"), None);
    assert_eq!(scheme::gtk_theme("'Darkish'\n"), None);
}

/// Windows' `AppsUseLightTheme`, as the registry's DWORD bytes give it.
#[test]
fn windows_app_mode_is_read_from_its_registry_value() {
    use faraday_starter_shell::windows::registry::dword;
    assert_eq!(scheme::windows(dword(&0u32.to_le_bytes())), Some(true));
    assert_eq!(scheme::windows(dword(&1u32.to_le_bytes())), Some(false));
    assert_eq!(scheme::windows(None), None);
}

/// `defaults read -g AppleInterfaceStyle`: `Dark` in Dark mode; in Light
/// mode the key is absent and the command fails.
#[test]
fn macos_interface_style_is_read_as_defaults_prints_it() {
    assert_eq!(scheme::apple_interface_style(Some("Dark\n")), Some(true));
    assert_eq!(scheme::apple_interface_style(None), Some(false));
    assert_eq!(scheme::apple_interface_style(Some("Light\n")), None);
}
