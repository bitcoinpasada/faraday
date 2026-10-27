//! Light or Dark at start (F126, `docs/THEMES.md` §14): the Starter, and
//! Faraday Demo inside it, open in the mode this computer's own setting
//! asks apps for, read once when the Starter starts, and in Light when
//! the setting says neither or cannot be read. Nothing is stored: the
//! next start reads it again. The colour theme starts at Cobalt.
//!
//! What each system answers is read here by functions with no system
//! call in them, so the tests feed them fixed answers; the readers that
//! ask the system are each system's own (`Platform::dark`).
//!
//! - Linux: the XDG desktop portal's `org.freedesktop.appearance`
//!   `color-scheme` ([`portal`]), which GNOME, KDE and the portals of
//!   other desktops answer; where no portal answers, GNOME's own
//!   `color-scheme` ([`gnome_color_scheme`]) and, before GNOME 42, a GTK
//!   theme named dark ([`gtk_theme`]).
//! - Windows: the app mode in Settings, `AppsUseLightTheme` ([`windows`]).
//! - macOS: `AppleInterfaceStyle` ([`apple_interface_style`]).

use faraday_core::ui::{Mode, Theme};

/// The theme the Starter opens in: Cobalt at Standard contrast, Dark
/// where the computer's setting asks for it (`Some(true)`), Light
/// otherwise.
pub fn at_start(dark: Option<bool>) -> Theme {
    let mode = if dark == Some(true) {
        Mode::Dark
    } else {
        Mode::Light
    };
    Theme {
        mode,
        ..Theme::default()
    }
}

/// The portal's `color-scheme`: 1 asks for dark, 2 for light, 0 (no
/// preference) and anything else for neither.
pub fn portal(value: u32) -> Option<bool> {
    match value {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

/// What `gsettings get org.gnome.desktop.interface color-scheme` prints:
/// `'prefer-dark'`, `'prefer-light'`, or `'default'`, which is neither.
pub fn gnome_color_scheme(printed: &str) -> Option<bool> {
    match printed.trim().trim_matches('\'') {
        "prefer-dark" => Some(true),
        "prefer-light" => Some(false),
        _ => None,
    }
}

/// What `gsettings get org.gnome.desktop.interface gtk-theme` prints: a
/// theme named dark (`'Adwaita-dark'`, `'Yaru:dark'`) asks for dark; any
/// other name says neither.
pub fn gtk_theme(printed: &str) -> Option<bool> {
    let name = printed.trim().trim_matches('\'').to_ascii_lowercase();
    (name.ends_with("-dark") || name.ends_with(":dark")).then_some(true)
}

/// Windows' `AppsUseLightTheme`: 0 asks for dark, any other value for
/// light; no value says neither.
pub fn windows(apps_use_light_theme: Option<u32>) -> Option<bool> {
    apps_use_light_theme.map(|v| v == 0)
}

/// What `defaults read -g AppleInterfaceStyle` prints in Dark mode,
/// `Dark`; in Light mode the key is not there and the command fails,
/// which is given here as `None`.
pub fn apple_interface_style(printed: Option<&str>) -> Option<bool> {
    match printed.map(str::trim) {
        Some("Dark") => Some(true),
        Some(_) => None,
        None => Some(false),
    }
}

/// Linux's answer: the portal's, else GNOME's own settings. Costs a call
/// on the session bus and, where it says nothing, up to two runs of
/// `gsettings`; done once, at start.
#[cfg(target_os = "linux")]
pub fn linux() -> Option<bool> {
    if let Some(dark) = crate::linux::bus::portal_color_scheme().and_then(portal) {
        return Some(dark);
    }
    let gsettings = |key: &str| {
        let out = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", key])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    gsettings("color-scheme")
        .as_deref()
        .and_then(gnome_color_scheme)
        .or_else(|| gsettings("gtk-theme").as_deref().and_then(gtk_theme))
}

/// macOS's answer. Costs a run of `defaults`; done once, at start.
#[cfg(target_os = "macos")]
pub fn macos() -> Option<bool> {
    let out = std::process::Command::new("/usr/bin/defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let printed = out
        .status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned());
    apple_interface_style(printed.as_deref())
}
