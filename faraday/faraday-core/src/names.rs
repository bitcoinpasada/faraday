//! The file and path names the Starter's install uses (`docs/STARTER.md`
//! §5.2, `docs/DECISIONS.md` F16): the one place they are written, so a
//! rename is a change here and in the docs. The words people read
//! ("Faraday Starter", "Faraday Demo") are `strings/en.rs`'s, never
//! these (the strings lint leaves this file out for that reason). Plain
//! constants only: `shells/starter/build.rs` includes this file. The release
//! files' names are `faraday/tools/stage-release.sh`'s `starter_release_name`.

/// The installed Starter on Linux, in `~/.local/bin`.
pub const LINUX_BIN: &str = "faraday-starter";
/// The Starter's menu entry and icon (reverse-DNS, so never a developer's
/// own `faraday.desktop`), and its Windows manifest identity.
pub const MENU_ID: &str = "app.faradayoffline.FaradayStarter";
/// The developer's Faraday Demo's Windows manifest identity
/// (`shells/desktop/build.rs`); an earlier Starter's install of the Demo
/// had its menu entry under it ([`LEGACY_DESKTOP`]).
pub const MENU_DESKTOP_ID: &str = "app.faradayoffline.FaradayDemo";

/// `%LOCALAPPDATA%\Programs\<this>`, the uninstall key and the Run value.
pub const WIN_NAME: &str = "Faraday Starter";
/// The installed Starter on Windows.
pub const WIN_EXE: &str = "Faraday Starter.exe";
/// The Start menu shortcut.
pub const WIN_SHORTCUT: &str = "Faraday Starter.lnk";
/// The developer's Faraday Demo on Windows (`shells/desktop/build.rs`),
/// and the name an earlier Starter installed it under ([`LEGACY_DESKTOP`]).
pub const WIN_DESKTOP_EXE: &str = "Faraday Demo.exe";

/// One earlier install's names, found and removed by install and
/// uninstall when it is Faraday's. These never change.
pub struct Legacy {
    /// Linux: the app in `~/.local/bin`.
    pub linux_bin: &'static str,
    /// Linux: its menu entry and icon.
    pub menu_id: &'static str,
    /// Windows: its folder, uninstall key and Run value.
    pub win_name: &'static str,
    /// Windows: its executable.
    pub win_exe: &'static str,
    /// Windows: its Start menu shortcut.
    pub win_shortcut: &'static str,
}

/// Names earlier builds installed under: the test install of 2026-10-12,
/// and the "Faraday Hub" build (the code's working name before it was
/// renamed to Starter). These stay so the Starter still finds and
/// removes them on an upgrade; they never change.
pub const LEGACY: [Legacy; 2] = [
    Legacy {
        linux_bin: "faraday",
        menu_id: "app.faradayoffline.Faraday",
        win_name: "Faraday",
        win_exe: "Faraday.exe",
        win_shortcut: "Faraday.lnk",
    },
    Legacy {
        linux_bin: "faraday-hub",
        menu_id: "app.faradayoffline.FaradayHub",
        win_name: "Faraday Hub",
        win_exe: "Faraday Hub.exe",
        win_shortcut: "Faraday Hub.lnk",
    },
];

/// The names Faraday Demo was installed under beside the Starter, before
/// it ran inside the Starter's window (F126), found and removed by
/// install, update and uninstall when the file there answers `--version`
/// as Faraday's desktop app (old words included). These never change.
pub struct LegacyDesktop {
    /// Linux: the app in `~/.local/bin`.
    pub linux_bin: &'static str,
    /// Linux: its menu entry.
    pub menu_id: &'static str,
    /// Windows: its executable, beside the Starter's.
    pub win_exe: &'static str,
    /// Windows: its Start menu shortcut.
    pub win_shortcut: &'static str,
}

/// The names an earlier Starter installed the desktop app under: as
/// "Faraday Demo" (F119), and before that as "Faraday Desktop".
pub const LEGACY_DESKTOP: [LegacyDesktop; 2] = [
    LegacyDesktop {
        linux_bin: "faraday-demo",
        menu_id: MENU_DESKTOP_ID,
        win_exe: WIN_DESKTOP_EXE,
        win_shortcut: "Faraday Demo.lnk",
    },
    LegacyDesktop {
        linux_bin: "faraday-desktop",
        menu_id: "app.faradayoffline.FaradayDesktop",
        win_exe: "Faraday Desktop.exe",
        win_shortcut: "Faraday Desktop.lnk",
    },
];
