//! The density and the layout a Pi's HDMI screen gets from the EDID it
//! reports (`docs/ORIENTATION.md` §2.1, `docs/LAYOUT.md` §2), for the
//! EDIDs of real screens in `tests/edid/` (its README says where each
//! came from): the bytes on a fake `/sys/class/drm` tree, read as the
//! shell reads them, to the density the core is told and the class it
//! lays out at.
//!
//! The expected values are worked out by hand from each screen's size in
//! millimetres: the diagonal over 25.4 for inches, the pixels' diagonal
//! over the inches for the density (160 where the screen is over 11
//! inches), and the width in dp as pixels × 160 ÷ density.

use std::path::{Path, PathBuf};

use faraday_draw::geom::{Window, WindowClass};
use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

use crate::orient::{Setup, drm_edid};
use crate::{Args, dpi};

/// One recorded screen and what a person sees on it.
struct Screen {
    /// The screen as it is named in `tests/edid/README.md`.
    name: &'static str,
    edid: &'static [u8],
    /// The framebuffer's mode: the panel's native one.
    fb: (u16, u16),
    /// The density the core is told.
    dpi: u16,
    /// The layout class at that density.
    class: WindowClass,
    /// Whether Rotation is offered (a screen of 6 to 11 inches, or a
    /// portrait one).
    rotation: bool,
}

macro_rules! edid {
    ($name:literal) => {
        include_bytes!(concat!("../tests/edid/", $name, ".edid")).as_slice()
    };
}

const SCREENS: &[Screen] = &[
    // Small panels whose size is right.
    Screen {
        name: "hannstar-7in-800x480", // 154 × 84 mm: 6.9 in
        edid: edid!("hannstar-7in-800x480"),
        fb: (800, 480),
        dpi: 135,
        class: WindowClass::Expanded, // 948 dp
        rotation: true,
    },
    Screen {
        name: "displaylink-7in-1024x600", // 153 × 90 mm: 7.0 in
        edid: edid!("displaylink-7in-1024x600"),
        fb: (1024, 600),
        dpi: 170,
        class: WindowClass::Expanded, // 964 dp
        rotation: true,
    },
    Screen {
        name: "cpt-7in-1280x800", // 152 × 94 mm: 7.0 in
        edid: edid!("cpt-7in-1280x800"),
        fb: (1280, 800),
        dpi: 215,
        class: WindowClass::Expanded, // 953 dp
        rotation: true,
    },
    Screen {
        name: "waveshare-8.9in-2560x1600", // 192 × 120 mm: 8.9 in
        edid: edid!("waveshare-8.9in-2560x1600"),
        fb: (2560, 1600),
        dpi: 339,
        class: WindowClass::Large, // 1208 dp
        rotation: true,
    },
    Screen {
        name: "waveshare-10.1in-1920x1200", // 216 × 135 mm: 10.0 in
        edid: edid!("waveshare-10.1in-1920x1200"),
        fb: (1920, 1200),
        dpi: 226,
        class: WindowClass::Large, // 1359 dp
        rotation: true,
    },
    Screen {
        name: "eve-10.1in-1280x800", // 217 × 136 mm: 10.1 in
        edid: edid!("eve-10.1in-1280x800"),
        fb: (1280, 800),
        dpi: 150,
        class: WindowClass::Large, // 1365 dp
        rotation: true,
    },
    Screen {
        name: "vizta-10.1in-1024x600", // 223 × 125 mm: 10.1 in
        edid: edid!("vizta-10.1in-1024x600"),
        fb: (1024, 600),
        dpi: 118,
        class: WindowClass::Large, // 1389 dp
        rotation: true,
    },
    // A size that is wrong or missing.
    Screen {
        // The picture's 276 × 207 mm is not the panel's shape; the
        // screen's 220 × 130 mm is: 10.1 in.
        name: "vizta-10.1in-1024x600-wrong-image-size",
        edid: edid!("vizta-10.1in-1024x600-wrong-image-size"),
        fb: (1024, 600),
        dpi: 118,
        class: WindowClass::Large, // 1389 dp
        rotation: true,
    },
    Screen {
        // The picture's 60 × 60 mm is a placeholder; the screen's
        // 100 × 150 mm is the panel's shape: 7.1 in.
        name: "valve-7in-800x1280-placeholder",
        edid: edid!("valve-7in-800x1280-placeholder"),
        fb: (800, 1280),
        dpi: 213,
        class: WindowClass::Medium, // 601 dp
        rotation: true,
    },
    Screen {
        // The picture's 1920 × 1080 mm is the pixels of a landscape
        // screen on a portrait one, and the screen's size is zero: no
        // size, so the image's 160 dpi.
        name: "waveshare-portrait-1080x1920",
        edid: edid!("waveshare-portrait-1080x1920"),
        fb: (1080, 1920),
        dpi: 160,
        class: WindowClass::Expanded, // 1080 dp
        rotation: true,
    },
    // Either side of 11 inches.
    Screen {
        name: "samsung-10.79in-1920x1080", // 239 × 134 mm: 10.8 in
        edid: edid!("samsung-10.79in-1920x1080"),
        fb: (1920, 1080),
        dpi: 204,
        class: WindowClass::Large, // 1506 dp
        rotation: true,
    },
    Screen {
        name: "boe-10.99in-2160x1440", // 232 × 155 mm: 10.99 in
        edid: edid!("boe-10.99in-2160x1440"),
        fb: (2160, 1440),
        dpi: 236,
        class: WindowClass::Large, // 1464 dp
        rotation: true,
    },
    Screen {
        name: "boe-11.02in-2160x1440", // 233 × 155 mm: 11.02 in
        edid: edid!("boe-11.02in-2160x1440"),
        fb: (2160, 1440),
        dpi: 160,
        class: WindowClass::ExtraLarge, // 2160 dp
        rotation: false,
    },
    Screen {
        name: "mstar-11.6in-1920x1080", // 256 × 144 mm: 11.6 in
        edid: edid!("mstar-11.6in-1920x1080"),
        fb: (1920, 1080),
        dpi: 160,
        class: WindowClass::ExtraLarge, // 1920 dp
        rotation: false,
    },
    Screen {
        name: "waveshare-11.9in-2560x1600", // 256 × 160 mm: 11.9 in
        edid: edid!("waveshare-11.9in-2560x1600"),
        fb: (2560, 1600),
        dpi: 160,
        class: WindowClass::ExtraLarge, // 2560 dp
        rotation: false,
    },
    // A desk monitor and televisions.
    Screen {
        name: "dell-24in-1920x1080", // 527 × 296 mm: 23.8 in
        edid: edid!("dell-24in-1920x1080"),
        fb: (1920, 1080),
        dpi: 160,
        class: WindowClass::ExtraLarge,
        rotation: false,
    },
    Screen {
        name: "samsung-tv-1920x1080", // 930 × 530 mm: 42 in
        edid: edid!("samsung-tv-1920x1080"),
        fb: (1920, 1080),
        dpi: 160,
        class: WindowClass::ExtraLarge,
        rotation: false,
    },
    Screen {
        // A television whose mode is 1920 × 540 and whose 160 × 90 mm
        // is not that mode's shape: no size.
        name: "sunplus-tv-1920x540",
        edid: edid!("sunplus-tv-1920x540"),
        fb: (1920, 540),
        dpi: 160,
        class: WindowClass::ExtraLarge,
        rotation: false,
    },
];

/// A fake `/sys/class/drm` holding `edid` on the connected connector,
/// beside the card, a connector nothing is plugged into and one whose
/// EDID is another screen's.
fn drm_tree(name: &str, edid: &[u8]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("faraday-drm-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let put = |dir: &str, file: &str, value: &[u8]| {
        std::fs::create_dir_all(root.join(dir)).unwrap();
        std::fs::write(root.join(dir).join(file), value).unwrap();
    };
    put("card0", "dev", b"226:0\n");
    put("card0-HDMI-A-1", "status", b"disconnected\n");
    put("card0-HDMI-A-1", "edid", &[]);
    put("card0-HDMI-A-2", "status", b"connected\n");
    put("card0-HDMI-A-2", "edid", edid);
    root
}

/// The density and class the shell gives for the EDID on `root`.
fn shell_sees(root: &Path, fb: (u16, u16)) -> (u16, WindowClass, bool) {
    let edid = drm_edid(root);
    let size = faraday_paper::edid::size(&edid);
    let dpi = dpi(&Args::default(), "faraday.dpi=160", size, fb);
    let display = DisplayInfo {
        width: fb.0,
        height: fb.1,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    };
    let class = Window::of(&display, 1.0).class();
    (dpi, class, Setup::detected(fb, size, "", "").offered)
}

/// A recorded screen gets the density its size gives where it is 11
/// inches or less, the image's 160 dpi everywhere else, and the layout
/// class that density lays it out at.
#[test]
fn each_recorded_screen_gets_its_density_and_class() {
    for s in SCREENS {
        let root = drm_tree(s.name, s.edid);
        let seen = shell_sees(&root, s.fb);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(seen, (s.dpi, s.class, s.rotation), "{}", s.name);
    }
}

/// A screen the firmware drives in another mode than its native one
/// keeps the image's density, whatever size its EDID gives.
#[test]
fn a_small_screen_in_another_mode_keeps_the_image_density() {
    let s = &SCREENS[1];
    let root = drm_tree("other-mode", s.edid);
    let seen = shell_sees(&root, (800, 600));
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(seen.0, 160);
}
