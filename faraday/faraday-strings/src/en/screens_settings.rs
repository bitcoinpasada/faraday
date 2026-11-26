//! Faraday's English words for the screens settings area.

macro_rules! words_screens_settings {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- settings (faraday-core/src/screens/settings.rs) -----
    /// Text in `settings_compact`.
    settings_time: "Time",
    /// Line in `settings_compact`.
    settings_now: "Now",
    /// Button in `settings_compact`.
    settings_set_the_time: "Set the time",
    /// Title in `settings`.
    settings_settings: "Settings",
    /// Line in `settings`.
    settings_lock_and_power: "Lock and power",
    /// Text in `settings`. `{}`: `n`, `if n == 1 { EN.settings_key } else { …`.
    settings_loaded: "{} {} loaded",
    /// Text in `settings`.
    settings_key: "key",
    /// Text in `settings`.
    settings_keys: "keys",
    /// Button in `settings`.
    settings_lock: "Lock",
    /// Button in `settings`.
    settings_remove_every: "Remove every key",
    /// The title of the sheet Remove every key opens while a key is kept
    /// nowhere else.
    settings_remove_every_title: "Remove every key",
    /// The hold on that sheet.
    settings_hold_remove_every: "Hold to remove every key",
    /// The line above the list on that sheet.
    settings_remove_every_line: "Keys kept nowhere else are lost",
    /// What the sheet says follows.
    settings_remove_every_then: "Every key loaded is removed from memory",
    /// Row in a key's details, and the title of the sheet it opens while
    /// the key is kept nowhere else.
    settings_remove_key: "Remove this key",
    /// The hold on that sheet.
    settings_hold_remove_key: "Hold to remove",
    /// The line above the key on that sheet.
    settings_remove_key_line: "A key kept nowhere else is lost",
    /// What that sheet says follows.
    settings_remove_key_then: "This key is removed from memory",
    /// Button in `settings`.
    settings_power_off_button: "Power off",
    /// Button in `settings` in the desktop app, where there is no power to turn off.
    settings_quit: "Quit",
    /// Text in `settings`.
    settings_lock_after: "Lock after",
    /// Text in `settings`.
    settings_power_off: "Power off after",
    /// Text in `settings`.
    settings_never: "Never",
    /// Text in `settings`. `{}`: `m`.
    settings_min: "{} min",
    /// Text in `settings`. `{}`: `names` joined by ", ".
    settings_ignored_until: "Ignored until unplugged: {}",
    /// Row in `settings`, and the mode sheet's title row there.
    settings_mode: "Mode",
    /// Line in `settings`.
    settings_appearance: "Appearance",
    /// Appearance's row for how screens change (`docs/MOTION.md` §3.5).
    settings_motion: "Motion",
    /// Motion style: screens glide and fade through.
    settings_motion_smooth: "Smooth motion",
    /// Motion style: two screens blend.
    settings_motion_crossfade: "Crossfade",
    /// Motion style: nothing moves (Reduce motion).
    settings_motion_off: "Off",
    /// What Smooth motion does.
    settings_motion_smooth_line: "Screens glide and fade",
    /// What Crossfade does.
    settings_motion_crossfade_line: "Screens blend into each other",
    /// What Off does.
    settings_motion_off_line: "Screens change at once",
    /// Appearance's row for how long a change takes.
    settings_speed: "Speed",
    /// Speed: 250 ms.
    settings_speed_quick: "Quick",
    /// Speed: 400 ms.
    settings_speed_medium: "Medium",
    /// Speed: 600 ms.
    settings_speed_relaxed: "Relaxed",
    /// Appearance's row under Contrast: how large text is drawn
    /// (`docs/TEXT-SIZE.md` §2).
    settings_text_size: "Text size",
    /// Text size's first choice: text at its own size.
    settings_text_standard: "Standard",
    /// Text size's second choice: text 1.2 times its size.
    settings_text_large: "Large",
    /// Text size's third choice: text 1.4 times its size.
    settings_text_larger: "Larger",
    /// Settings' section holding Scale and, on a panel that can turn,
    /// Rotation (`docs/ORIENTATION.md` §6).
    settings_display: "Display",
    /// Display's row: the size of text and controls.
    settings_scale: "Scale",
    /// Display's row on a panel that can turn: which way it shows the app.
    settings_rotation: "Rotation",
    /// Rotation's first choice: wider than tall.
    settings_landscape: "Landscape",
    /// Rotation's second choice: taller than wide.
    settings_portrait: "Portrait",
    /// Text in `settings`.
    settings_auto: "Auto",
    /// Preview size's choice: the window's own size.
    settings_preview_this_window: "This window",
    /// Preview size's choice: the Pi's 2.8-inch panel, 268 × 358 dp.
    settings_preview_pi: "Pi panel",
    /// Preview size's choice: a narrow window, 400 × 800 dp.
    settings_preview_narrow: "Narrow window",
    /// Preview size's choice: half a screen, 760 × 1000 dp.
    settings_preview_half: "Half screen",
    /// Preview size's choice: a 7-inch panel, 964 × 565 dp.
    settings_preview_7in: "7-inch panel",
    /// Preview size's choice: a 7-inch panel turned to portrait, 565 ×
    /// 964 dp (`docs/ORIENTATION.md` §10).
    settings_preview_7in_portrait: "7-inch panel, portrait",
    /// Preview size's choice: a 10-inch panel, 1365 × 853 dp.
    settings_preview_10in: "10-inch panel",
    /// Preview size's choice: a 10-inch panel in portrait, 853 × 1365 dp.
    settings_preview_10in_portrait: "10-inch panel, portrait",
    /// Preview size's choice: the desktop app's window, 1280 × 800 dp.
    settings_preview_desktop: "Desktop window",
    /// Preview size's choice: a full monitor, 1920 × 1080 dp.
    settings_preview_monitor: "Full monitor",
    /// Preview size's value under the choices. `{}`: width, height in dp.
    settings_preview_dp: "{} × {} dp",
    /// Faraday Demo's host strip (docs/DESKTOP-HOST.md): the preview
    /// size's button. `{}`: the size shown.
    host_size: "Size · {}",
    /// Faraday Demo's host strip: the preview size's button where the
    /// strip is narrow.
    host_size_short: "Size",
    /// Faraday Demo's host strip: the sticks' menu where the strip is
    /// narrowest.
    host_sticks_short: "Sticks",
    /// Faraday Demo's host strip: the computer's die's Rolls per click,
    /// opening its choices. `{}`: the one chosen, or the number typed.
    host_rolls_per_click: "Rolls per click · {}",
    /// Faraday Demo's host strip: the practice aids that do not fit, in a
    /// menu.
    host_more: "More",
    /// Line in `settings`.
    settings_what_remembered: "What is remembered",
    /// Line in `settings`.
    settings_amounts_signed: "Amounts signed",
    /// Line in `settings`.
    settings_signatures_made: "Signatures made, for the nonce check",
    /// Text in `settings`. `{}`: `n`, `if n == 1 { EN.settings_transaction }…`.
    settings_remembered: "{} {} remembered",
    /// Text in `settings`.
    settings_transaction: "transaction",
    /// Text in `settings`.
    settings_transactions: "transactions",
    /// Text in `settings`.
    settings_in_open_vault: "In an open vault",
    /// Text in `settings`.
    settings_until_power_off: "Until power-off",
    /// Text in `settings`. `{}`: `n`, `if n == 1 { EN.settings_signature }…`.
    settings_signatures_remembered: "{} {} remembered",
    /// Text in `settings`.
    settings_signature: "signature",
    /// Text in `settings`.
    settings_signatures: "signatures",
    /// Text in `settings`.
    settings_nonces_off: "Off",
    /// Line in `settings`.
    settings_about: "About",
    /// Text in `settings`. `{}`: `crate::version_label()`.
    settings_faraday: "Faraday {}",
    /// Text in `settings`: the own-risk statement.
    settings_own_risk: "Pre-release, not reviewed by anyone outside the project. Keep a backup that does not depend on Faraday. No warranty of any kind.",
    /// Text in `settings`.
    settings_keys_are_typed: "Keys are typed, scanned as SeedQR or loaded from a vault, and kept for the session only",
    /// Text in `settings`. `{}`: `app.medium.noun()`, `app.medium.a()`.
    settings_vaults_files: "Vaults · files move by {} · QR by camera and from PNG files on {}",
    /// Text in `settings`. `{}`: `app.medium.caps()`.
    settings_are_folders: "{} are folders in ~/faraday-sticks",
    /// Text in `settings`. `{}`: `app.medium.caps()`.
    settings_medium_reader_note: "{} are read by an unprivileged disk process; the kernel mounts none",
    /// Text in `settings`.
    settings_not_run: "not run",
    // ----- institute_note (faraday-core/src/screens/settings.rs) -----
    /// Text in `institute_note`. `{}`: `SITE`.
    institute_note_visit: "Visit {} to learn about Bitcoin’s history, economics, and technology. Not affiliated.",
        }
    };
}
