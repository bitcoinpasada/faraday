//! The facts and settings a test starts from, for each product.

use faraday_kernel::settings::{Appearance, Contrast, MotionSpeed, TextSize, ThemeId};
use faraday_kernel::{Facts, SelfTest, Sticks, SystemFacts, shared};
use faraday_port::event::{Abilities, Machine, SecureBootAtStart};
use faraday_port::values::{Display, Medium, Orientation};

/// What the settings owner holds before the person chooses anything.
pub(crate) fn settings() -> shared::Settings {
    shared::Settings {
        idle_lock_min: 30,
        idle_off_min: 60,
        qr_frame_ms: 500,
        zone_min: None,
        offset_secs: 0,
        checked: false,
        appearance: Appearance {
            theme: ThemeId(0),
            dark: false,
            contrast: Contrast::Standard,
            text: TextSize::Standard,
            motion: None,
            speed: MotionSpeed::Relaxed,
        },
        scale_pct: 100,
        rotation: Orientation::Landscape,
        seal_amounts: false,
        nonce_history: false,
    }
}

fn facts(medium: Medium, display: Display, abilities: Abilities) -> Facts {
    Facts {
        display,
        medium,
        abilities,
        machine: Machine {
            maker: None,
            product: None,
            family: None,
            chassis: None,
            hypervisor: false,
            screen: None,
            secure_boot: SecureBootAtStart::Unknown,
            memory_total_mib: None,
            cores: 4,
        },
        root_read_only: None,
        sticks: Sticks {
            attached: false,
            listed: Vec::new(),
            boot_medium: None,
            problems: Vec::new(),
        },
        cameras: Vec::new(),
        downloads: None,
        entropy_source: None,
        selftest: SelfTest::NotRun,
        clean: true,
        system: SystemFacts {
            computer: None,
            disks: Vec::new(),
            no_writer: false,
            installed: None,
            dev_image: None,
            images_wanted: Vec::new(),
            concerns: Vec::new(),
            other_encryption: None,
            boot_entry: None,
        },
    }
}

fn display(width: u16, height: u16) -> Display {
    Display {
        width,
        height,
        dpi: 96,
        orientation: Orientation::Landscape,
        rotation_offered: None,
        external: false,
        preview: None,
    }
}

/// The device: a touch screen, a stick, a camera, held-back input; a lock ends the process.
pub(crate) fn device() -> Facts {
    facts(
        Medium::Stick,
        display(1024, 600),
        Abilities {
            clipboard: false,
            print: false,
            host_files: false,
            opens: false,
            camera: true,
            names_sticks: true,
            holds_back_input: true,
            restart_computer: true,
            moves_window: false,
            system: false,
            lock_ends_process: true,
        },
    )
}

/// Faraday Demo: a window on a computer, folders standing for sticks.
pub(crate) fn demo() -> Facts {
    facts(
        Medium::Desktop,
        display(1280, 800),
        Abilities {
            clipboard: true,
            print: true,
            host_files: true,
            opens: true,
            camera: true,
            names_sticks: false,
            holds_back_input: false,
            restart_computer: false,
            moves_window: true,
            system: false,
            lock_ends_process: false,
        },
    )
}
