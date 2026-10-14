//! The maker pages' firmware sketches (`docs/DECISIONS.md` F18,
//! `docs/STARTER.md` §7.8): the data a maker page's `::sketch <id>`
//! paragraph names ([`faraday_learn::sketch::is_marker`]), which
//! `faraday_ui::sketch` turns into a mock-up of the firmware screen it
//! depicts. The shape it is built from, and the marker itself, are
//! `faraday-learn`'s; this crate holds only the data, read from the
//! makers' simulators, manuals and support articles on 2026-10-13. One
//! sketch stands for a family of menus: it names no model, no maker
//! inside the frame and no real disk; a stick is "USB Flash Drive",
//! and a confirm code is an example.

use faraday_learn::sketch::{
    Dialog, MARKER, Nav, Palette, Row, Sketch, blank, change, group, row, target,
};

// ThinkPad, graphical setup (2022 on).
const THINKPAD_NAV: &[&str] = &[
    "Main",
    "Config",
    "Date/Time",
    "Security",
    "Startup",
    "Restart",
];
const THINKPAD_KEYS: &[&str] = &[
    "F1 General Help",
    "F9 Setup Defaults",
    "Esc Exit",
    "F10 Save and Exit",
];

// Lenovo IdeaPad, Yoga and Legion, graphical setup.
const IDEAPAD_NAV: &[&str] = &["Information", "Configuration", "Security", "Boot", "Exit"];
const IDEAPAD_KEYS: &[&str] = &[
    "F1 Help",
    "↑↓ Select Item",
    "+/- Change Values",
    "F9 Setup Defaults",
    "Esc Exit",
    "←→ Select Menu",
    "Enter Select ▸ Sub-Menu",
    "F10 Save and Exit",
];

/// Lenovo's save step: the same box on both families.
const LENOVO_SAVE: Dialog = Dialog {
    title: "Setup Confirmation",
    text: "Save configuration changes and exit now?",
    rows: &[],
    buttons: &["Yes", "No"],
    choose: 0,
};

// HP consumer notebooks, 2019 on.
const HP_TABS: &[&str] = &["Main", "Security", "Configuration", "Boot Options", "Exit"];

// HP business notebooks and workstations.
const HP_BUSINESS_TABS: &[&str] = &["Main", "Security", "Advanced", "UEFI Drivers"];

// The boot manager's hint line on Insyde-based firmware.
const BOOT_MANAGER_KEYS: &[&str] =
    &["↑ and ↓ to change option, ENTER to select an option, ESC to exit"];

// ASUS notebooks, light Advanced Settings.
const ASUS_TABS: &[&str] = &["Main", "Advanced", "Boot", "Security", "Save & Exit"];
const ASUS_ROWS: &[Row] = &[
    row("Platform Mode", "Deployed"),
    row("Secure Boot", "Active"),
    change("Secure Boot Control", "Enabled", "Disabled"),
    row("Key Management", "▸"),
];
const ASUS_KEYS: &[&str] = &[
    "←→ Switch Tab",
    "↑↓ Select Item",
    "+ − Change Option",
    "F9 Optimized Defaults",
    "F10 Save & Reset",
    "ESC Exit / Back",
];

// ASUS desktop boards, Advanced Mode.
const ASUS_DESKTOP_TABS: &[&str] = &[
    "My Favorites",
    "Main",
    "Ai Tweaker",
    "Advanced",
    "Monitor",
    "Boot",
    "Tool",
    "Exit",
];
const ASUS_DESKTOP_ROWS: &[Row] = &[
    row("Secure Boot state", "User"),
    change("OS Type", "Windows UEFI mode", "Other OS"),
    row("Secure Boot Mode", ""),
    row("Key Management", "▸"),
];

// Acer notebooks: tabs and keys from Acer's own steps, drawn as the
// text-mode setup they use.
const ACER_TABS: &[&str] = &["Information", "Main", "Security", "Boot", "Exit"];
const ACER_KEYS: &[&str] = &[
    "F1 Help",
    "↑↓ Select Item",
    "F5/F6 Change Values",
    "F9 Setup Defaults",
    "Esc Exit",
    "←→ Select Menu",
    "Enter Select ▸ SubMenu",
    "F10 Save and Exit",
];

// Microsoft Surface.
const SURFACE_NAV: &[&str] = &[
    "PC information",
    "Security",
    "Devices",
    "Boot configuration",
    "Management",
    "About",
    "Exit",
];

/// Every sketch, by its id.
pub const SKETCHES: &[Sketch] = &[
    // Lenovo ThinkPad.
    Sketch {
        title: "Setup",
        nav: Nav::Side,
        items: THINKPAD_NAV,
        selected: 3,
        heading: "← Secure Boot",
        rows: &[
            change("Secure Boot", "On", "Off"),
            row("Secure Boot Mode", "User Mode"),
            row("Secure Boot Key State", "Standard Mode"),
            row("Reset to Setup Mode", "Enter"),
            row("Restore Factory Keys", "Enter"),
            row("Clear All Secure Boot Keys", "Enter"),
        ],
        keys: THINKPAD_KEYS,
        ..blank("thinkpad-secure-boot", Palette::White)
    },
    Sketch {
        title: "Setup",
        nav: Nav::Side,
        items: THINKPAD_NAV,
        selected: 5,
        heading: "Restart",
        rows: &[
            target("Exit Saving Changes", ""),
            row("Exit Discarding Changes", ""),
            row("Load Factory Defaults", ""),
            row("Discard Changes", ""),
            row("Save Changes", ""),
        ],
        dialog: Some(LENOVO_SAVE),
        keys: THINKPAD_KEYS,
        ..blank("thinkpad-save", Palette::White)
    },
    // Lenovo IdeaPad, Yoga, Legion.
    Sketch {
        title: "Setup",
        nav: Nav::Side,
        items: IDEAPAD_NAV,
        selected: 2,
        heading: "Security",
        rows: &[
            row("Device Guard", "Disabled"),
            change("Secure Boot", "Enabled", "Disabled"),
            row("Secure Boot Status", "Enabled"),
            row("Platform Mode", "User Mode"),
            row("Secure Boot Mode", "Standard"),
            row("Reset to Setup Mode", ""),
            row("Restore Factory Keys", ""),
        ],
        keys: IDEAPAD_KEYS,
        ..blank("ideapad-secure-boot", Palette::White)
    },
    Sketch {
        title: "Setup",
        nav: Nav::Side,
        items: IDEAPAD_NAV,
        selected: 4,
        heading: "Exit",
        rows: &[
            target("Exit Saving Changes", ""),
            row("Exit Discarding Changes", ""),
            row("Discard Changes", ""),
            row("Save Changes", ""),
            row("Load Default Settings", ""),
        ],
        dialog: Some(LENOVO_SAVE),
        keys: IDEAPAD_KEYS,
        ..blank("ideapad-save", Palette::White)
    },
    Sketch {
        title: "Setup",
        nav: Nav::Side,
        items: IDEAPAD_NAV,
        selected: 3,
        heading: "Boot",
        rows: &[
            target("USB Boot", "Enabled"),
            row("PXE Boot to LAN", "Enabled"),
            row("IPV4 PXE First", "Enabled"),
        ],
        keys: IDEAPAD_KEYS,
        ..blank("ideapad-usb-boot", Palette::White)
    },
    Sketch {
        title: "Boot Manager",
        heading: "Boot Option Menu",
        rows: &[
            row("Windows Boot Manager", ""),
            target("EFI USB Device (USB Flash Drive)", ""),
            row("USB ZIP : USB Flash Drive", ""),
            row("Network Boot", ""),
        ],
        keys: BOOT_MANAGER_KEYS,
        ..blank("ideapad-boot-menu", Palette::Grey)
    },
    // HP consumer notebooks, 2019 on.
    Sketch {
        title: "Setup",
        nav: Nav::Tabs,
        items: HP_TABS,
        selected: 3,
        rows: &[
            row("USB Boot", "Enabled"),
            row("Network Boot", "Enabled"),
            change("Secure Boot", "Enabled", "Disabled"),
            row("Platform Key", "Enrolled"),
            row("Pending Action", "None"),
            row("Clear All Secure Boot Keys", ""),
        ],
        ..blank("hp-secure-boot", Palette::Blue)
    },
    Sketch {
        title: "Setup",
        nav: Nav::Tabs,
        items: HP_TABS,
        selected: 4,
        rows: &[
            target("Save Changes and Exit", ""),
            row("Ignore Changes and Exit", ""),
            row("Load Setup Defaults", ""),
        ],
        dialog: Some(Dialog {
            title: "",
            text: "Save Changes and Exit?",
            rows: &[],
            buttons: &["Yes", "No"],
            choose: 0,
        }),
        ..blank("hp-save", Palette::Blue)
    },
    Sketch {
        title: "Boot Manager",
        heading: "Boot Option Menu",
        rows: &[
            row("OS Boot Manager (UEFI) - Windows Boot Manager", ""),
            row("Boot From EFI File", ""),
            target("USB Hard Drive - USB Flash Drive", ""),
            row("Internal CD/DVD ROM Drive", ""),
        ],
        keys: &[
            "↑ and ↓ to change option, ENTER to select an option.",
            "Press F10 to BIOS Setup Options, ESC to exit.",
        ],
        ..blank("hp-boot-menu", Palette::Black)
    },
    Sketch {
        title: "Operating System Boot Mode Change",
        text: "A change to the operating system Secure Boot mode is pending. Please enter the pass code displayed below to complete the change. If you did not initiate this request, press the ESC key to continue without accepting the pending change.",
        rows: &[
            row("Operating System Boot Mode Change (021)", ""),
            target("7294 + ENTER", "to complete the change"),
            row("ESC", "Continue without change"),
        ],
        ..blank("hp-code", Palette::Black)
    },
    // HP business notebooks and workstations.
    Sketch {
        nav: Nav::Tabs,
        items: HP_BUSINESS_TABS,
        selected: 1,
        heading: "← Secure Boot Configuration",
        rows: &[
            change("Secure Boot", "[x]", "[ ]"),
            group("Secure Boot Key Management"),
            row("Import Custom Secure Boot keys", "[ ]"),
            row("Clear Secure Boot keys", "[ ]"),
            row("Reset Secure Boot keys to factory defaults", "[ ]"),
            row("Enable MS UEFI CA key", "[x]"),
        ],
        ..blank("hp-business-secure-boot", Palette::White)
    },
    Sketch {
        nav: Nav::Tabs,
        items: HP_BUSINESS_TABS,
        selected: 2,
        heading: "← Boot Options",
        rows: &[
            row("Startup Delay (sec.)", "0"),
            row("Fast Boot", "[ ]"),
            target("USB Storage Boot", "[x]"),
            row("Network (PXE) Boot", "[ ]"),
        ],
        ..blank("hp-business-usb-boot", Palette::White)
    },
    Sketch {
        nav: Nav::Tabs,
        items: HP_BUSINESS_TABS,
        selected: 0,
        rows: &[
            row("Apply Factory Defaults and Exit", ""),
            row("Ignore Changes and Exit", ""),
            target("Save Changes and Exit", ""),
        ],
        ..blank("hp-business-save", Palette::White)
    },
    Sketch {
        title: "Secure boot",
        text: "A request has been made to change this system's secure boot configuration, which may affect the secure boot keys and/or may disable secure boot. Please type in and enter the below number for authorization.",
        rows: &[target("7294", "")],
        ..blank("hp-business-code", Palette::White)
    },
    // Dell.
    Sketch {
        title: "BIOS Setup",
        nav: Nav::Side,
        items: &[
            "Overview",
            "Boot Configuration",
            "Integrated Devices",
            "Storage",
            "Display",
            "Connection",
            "Power",
            "Security",
            "Passwords",
            "Pre-boot Behavior",
        ],
        selected: 1,
        heading: "Boot Configuration",
        rows: &[
            group("Secure Boot"),
            change("Enable Secure Boot", "On", "Off"),
            row("Enable Microsoft UEFI CA", "On"),
            row("Secure Boot Mode", "Deployed Mode"),
            group("Expert Key Management"),
            row("Enable Custom Mode", "Off"),
        ],
        keys: &["LOAD DEFAULTS", "APPLY CHANGES", "Exit"],
        ..blank("dell-secure-boot", Palette::Grey)
    },
    Sketch {
        title: "Settings",
        nav: Nav::Side,
        items: &[
            "General",
            "System Configuration",
            "Video",
            "Security",
            "Secure Boot",
            "Performance",
            "Power Management",
            "POST Behavior",
        ],
        selected: 4,
        heading: "Secure Boot Enable",
        rows: &[change("Secure Boot Enable", "[x]", "[ ]")],
        keys: &["Load Defaults", "Apply", "Exit"],
        ..blank("dell-secure-boot-before-2021", Palette::Grey)
    },
    Sketch {
        title: "One Time Boot Menu",
        rows: &[
            group("UEFI Boot"),
            row("Windows Boot Manager", ""),
            target("UEFI: USB Flash Drive, Partition 1", ""),
            group("Other Options"),
            row("BIOS Setup", ""),
            row("Diagnostics", ""),
        ],
        ..blank("dell-boot-menu", Palette::Grey)
    },
    // ASUS.
    Sketch {
        title: "Advanced Settings",
        nav: Nav::Tabs,
        items: ASUS_TABS,
        selected: 3,
        heading: "← Security\\Secure Boot",
        rows: ASUS_ROWS,
        keys: ASUS_KEYS,
        ..blank("asus-secure-boot", Palette::White)
    },
    Sketch {
        title: "Advanced Settings",
        nav: Nav::Tabs,
        items: ASUS_TABS,
        selected: 3,
        heading: "← Security\\Secure Boot",
        rows: ASUS_ROWS,
        dialog: Some(Dialog {
            title: "Save & Reset",
            text: "Save configuration and reset?",
            rows: &[],
            buttons: &["Confirm", "Cancel"],
            choose: 0,
        }),
        keys: ASUS_KEYS,
        ..blank("asus-save", Palette::White)
    },
    Sketch {
        title: "Advanced Mode",
        nav: Nav::Tabs,
        items: ASUS_DESKTOP_TABS,
        selected: 5,
        heading: "← Boot\\Secure Boot",
        rows: ASUS_DESKTOP_ROWS,
        keys: &["EzMode(F7)", "Hot Keys"],
        ..blank("asus-desktop-secure-boot", Palette::Red)
    },
    Sketch {
        title: "Advanced Mode",
        nav: Nav::Tabs,
        items: ASUS_DESKTOP_TABS,
        selected: 7,
        heading: "← Boot\\Secure Boot",
        rows: ASUS_DESKTOP_ROWS,
        dialog: Some(Dialog {
            title: "Save Changes & Reset",
            text: "Save configuration and reset?",
            rows: &[row("OS Type", "[Windows UEFI mode] -> [Other OS]")],
            buttons: &["Ok", "Cancel"],
            choose: 0,
        }),
        keys: &["EzMode(F7)", "Hot Keys"],
        ..blank("asus-desktop-save", Palette::Red)
    },
    Sketch {
        dialog: Some(Dialog {
            title: "Please select boot device:",
            text: "",
            rows: &[
                row("Windows Boot Manager", ""),
                target("UEFI: USB Flash Drive, Partition 1", ""),
                row("Enter Setup", ""),
            ],
            buttons: &[],
            choose: 0,
        }),
        ..blank("asus-boot-menu", Palette::Black)
    },
    // Acer.
    Sketch {
        title: "Setup Utility",
        nav: Nav::Tabs,
        items: ACER_TABS,
        selected: 2,
        rows: &[target("Set Supervisor Password", "")],
        keys: ACER_KEYS,
        ..blank("acer-supervisor", Palette::Black)
    },
    Sketch {
        title: "Setup Utility",
        nav: Nav::Tabs,
        items: ACER_TABS,
        selected: 3,
        rows: &[change("Secure Boot", "Enabled", "Disabled")],
        keys: ACER_KEYS,
        ..blank("acer-secure-boot", Palette::Black)
    },
    Sketch {
        title: "Setup Utility",
        nav: Nav::Tabs,
        items: ACER_TABS,
        selected: 1,
        rows: &[change("F12 Boot Menu", "Disabled", "Enabled")],
        keys: ACER_KEYS,
        ..blank("acer-boot-menu-key", Palette::Black)
    },
    // MSI notebooks.
    Sketch {
        nav: Nav::Tabs,
        items: &["Main", "Advanced", "Boot", "Security", "Save & Exit"],
        selected: 3,
        heading: "Security\\Secure Boot",
        rows: &[
            row("System Mode", "User"),
            change("Secure Boot Support", "Enabled", "Disabled"),
            row("Secure Boot", "Active"),
            row("Secure Boot Mode", "Standard"),
        ],
        keys: &[
            "→←: Select Screen",
            "↑↓/Click: Select Item",
            "Enter/Dbl Click: Select",
            "+/-: Change Opt.",
            "F10: Save & Reset",
            "ESC: Exit",
        ],
        ..blank("msi-secure-boot", Palette::Red)
    },
    Sketch {
        dialog: Some(Dialog {
            title: "Please select boot device:",
            text: "",
            rows: &[
                row("Windows Boot Manager", ""),
                target("UEFI: USB Flash Drive, Partition 1", ""),
            ],
            buttons: &[],
            choose: 0,
        }),
        ..blank("msi-boot-menu", Palette::Red)
    },
    // Microsoft Surface.
    Sketch {
        title: "UEFI",
        nav: Nav::Side,
        items: SURFACE_NAV,
        selected: 1,
        heading: "Secure Boot",
        rows: &[target("Change configuration", "")],
        dialog: Some(Dialog {
            title: "Change Secure Boot configuration",
            text: "Select a Secure Boot certificate keyset.",
            rows: &[
                row("Microsoft only", ""),
                row("Microsoft & 3rd-party CA", ""),
                target("None", ""),
            ],
            buttons: &["OK", "Cancel"],
            choose: 0,
        }),
        ..blank("surface-secure-boot", Palette::White)
    },
    Sketch {
        title: "UEFI",
        nav: Nav::Side,
        items: SURFACE_NAV,
        selected: 3,
        heading: "Configure boot device order",
        rows: &[
            row("Windows Boot Manager", "[x]"),
            target("USB Storage", "[x]"),
            row("PXE Network", "[x]"),
            group("Advanced options"),
            row("Enable alternate boot sequence", "On"),
            target("Enable Boot from USB devices", "On"),
        ],
        ..blank("surface-boot", Palette::White)
    },
    Sketch {
        title: "UEFI",
        nav: Nav::Side,
        items: SURFACE_NAV,
        selected: 6,
        heading: "Exit",
        rows: &[target("Restart now", "")],
        ..blank("surface-exit", Palette::White)
    },
    // Framework.
    Sketch {
        rows: &[
            row("Continue", ""),
            row("Boot Manager", ""),
            row("Boot From File", ""),
            target("Administer Secure Boot", ""),
            row("Setup Utility", ""),
        ],
        keys: &[
            "F1 Help",
            "↑↓ Select Item",
            "←→ Select Item",
            "Enter Select ▸ Submenu",
        ],
        ..blank("framework-front-page", Palette::Gold)
    },
    Sketch {
        title: "Administer Secure Boot",
        rows: &[
            group("System Status:"),
            row("Secure Boot Status", "Enabled"),
            group("Options:"),
            change("Enforce Secure Boot", "Enabled", "Disabled"),
            row("Erase all Secure Boot Settings", "Disabled"),
            row("Restore Secure Boot to Factory Settings", "Disabled"),
        ],
        keys: &[
            "F1 Help",
            "ESC Exit",
            "↑↓ Select Item",
            "F5 F6 Change Values",
            "Enter Select ▸ Submenu",
            "F10 Save and Exit",
        ],
        ..blank("framework-secure-boot", Palette::Gold)
    },
    Sketch {
        title: "Boot Manager",
        heading: "Boot Option Menu",
        rows: &[
            group("EFI Boot Devices"),
            target("EFI USB Device (USB Flash Drive)", ""),
            row("Windows Boot Manager", ""),
        ],
        keys: BOOT_MANAGER_KEYS,
        ..blank("framework-boot-menu", Palette::Gold)
    },
];

/// The sketch with this id, or `None` for an id there is none for.
pub fn sketch(id: &str) -> Option<&'static Sketch> {
    SKETCHES.iter().find(|s| s.id == id)
}

/// The sketch a page's paragraph asks for, when it is a marker naming
/// one: `::sketch <id>`.
pub fn sketch_of(paragraph: &str) -> Option<&'static Sketch> {
    paragraph
        .trim()
        .strip_prefix(MARKER)
        .and_then(|id| sketch(id.trim()))
}
