//! Which screen the Pi's boot script finds (`docs/IMAGES.md` §3.2), run
//! over a stand-in `/sys`, configfs, `/dev` and screen directory with the
//! probe answers faked: what a person sees is the screen the Pi starts
//! on, HDMI when none is found, and the arguments the shell is given.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../image/common/rootfs-overlay/etc/init.d/screen")
}

/// A stand-in system, in its own directory.
struct Rig {
    root: PathBuf,
}

/// What a panel's probe expects: (screen, controller name, bus, address),
/// and its panel.conf.
const SCREENS: &[(&str, &str, &str, &str, &str)] = &[(
    "dpi28",
    "Goodix",
    "i2c-gpio",
    "5d",
    "SIZE=480x640\nDPI=286\nTOUCH_NAME=Goodix\nTOUCH_GRID=640x480\nNATIVE=portrait\n",
)];

const DPI28: &str =
    "--size 480x640 --dpi 286 --touch-name Goodix --touch-grid 640x480 --native portrait";

impl Rig {
    fn new(name: &str) -> Rig {
        let root =
            std::env::temp_dir().join(format!("faraday-screen-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for d in [
            "sys/class/drm/card0-HDMI-A-1",
            "sys/class/input",
            "sys/class/graphics/fb0",
            "sys/bus/platform/drivers/vc4-drm",
            "cfg/device-tree/overlays",
            "dev",
            "run",
        ] {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        let drv = root.join("sys/bus/platform/drivers/vc4-drm");
        fs::write(drv.join("bind"), "").unwrap();
        fs::write(drv.join("unbind"), "").unwrap();
        symlink("../../../../devices/gpu", drv.join("gpu")).unwrap();
        let screens = root.join("etc/screens");
        for (s, name, bus, addr, conf) in SCREENS {
            let d = screens.join(s);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("panel.conf"), conf).unwrap();
            fs::write(
                d.join("probe.conf"),
                format!("KIND=gpio\nTOUCH_NAME={name}\nI2C_BUS={bus}\nI2C_ADDR={addr}\n"),
            )
            .unwrap();
            fs::write(d.join("touch.dtbo"), "touch").unwrap();
            fs::write(d.join("display.dtbo"), "display").unwrap();
        }
        fs::create_dir_all(screens.join("hdmi")).unwrap();
        fs::write(screens.join("hdmi/panel.conf"), "").unwrap();
        fs::write(screens.join("order"), "dpi28\n").unwrap();
        let rig = Rig { root };
        rig.hdmi("disconnected");
        rig
    }

    fn hdmi(&self, status: &str) {
        fs::write(
            self.root.join("sys/class/drm/card0-HDMI-A-1/status"),
            status,
        )
        .unwrap();
    }

    /// The input device a controller answers as, on the bus the overlay
    /// creates.
    fn controller(&self, bus: &str, addr: &str, name: &str) {
        controller_in(&self.root, bus, addr, name);
    }

    /// The framebuffer the rebind leaves, at `size` (`W,H`).
    fn framebuffer(&self, size: &str) {
        fs::write(self.root.join("dev/fb0"), "").unwrap();
        fs::write(self.root.join("sys/class/graphics/fb0/virtual_size"), size).unwrap();
    }

    fn run(&self) -> (String, String) {
        let r = &self.root;
        let ok = Command::new("sh")
            .arg(script())
            .env("FARADAY_SCREEN_SYS", r.join("sys"))
            .env("FARADAY_SCREEN_CONFIGFS", r.join("cfg"))
            .env("FARADAY_SCREEN_DIR", r.join("etc/screens"))
            .env("FARADAY_SCREEN_RUN", r.join("run/faraday-screen"))
            .env("FARADAY_SCREEN_DEV", r.join("dev"))
            .env("FARADAY_SCREEN_KMSG", r.join("kmsg"))
            .env("FARADAY_SCREEN_MOUNT", "true")
            .env("FARADAY_SCREEN_UMOUNT", "true")
            .env("FARADAY_SCREEN_WAIT", "5")
            .env("FARADAY_SCREEN_FB_WAIT", "3")
            .status()
            .unwrap()
            .success();
        assert!(ok, "the script ends 0 whatever it finds");
        let args = fs::read_to_string(r.join("run/faraday-screen/args")).unwrap();
        let log = fs::read_to_string(r.join("kmsg")).unwrap_or_default();
        (args.trim().to_string(), log)
    }

    /// Overlays still loaded when the script ended.
    fn loaded(&self) -> Vec<String> {
        fs::read_dir(self.root.join("cfg/device-tree/overlays"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn controller_in(root: &Path, bus: &str, addr: &str, name: &str) {
    let dev = root.join(format!("sys/devices/{bus}/3-00{addr}/input/input3"));
    fs::create_dir_all(&dev).unwrap();
    fs::write(dev.join("name"), format!("{name} Capacitive TouchScreen\n")).unwrap();
    let link = root.join("sys/class/input/input3");
    let _ = fs::remove_file(&link);
    symlink(&dev, link).unwrap();
}

#[test]
fn an_hdmi_screen_wins_over_the_panel_also_attached() {
    let rig = Rig::new("hdmi");
    rig.hdmi("connected");
    rig.controller("i2c-gpio", "5d", "Goodix");
    rig.framebuffer("480,640");
    let (args, _) = rig.run();
    assert_eq!(args, "");
    assert!(rig.loaded().is_empty(), "no panel's overlay is loaded");
}

#[test]
fn the_2_8_inch_gt911_answering_gives_its_screen() {
    let rig = Rig::new("gt911");
    rig.controller("i2c-gpio", "5d", "Goodix");
    rig.framebuffer("480,640");
    let (args, _) = rig.run();
    assert_eq!(args, DPI28);
    assert_eq!(rig.loaded().len(), 2, "its touch and display overlays stay");
}

#[test]
fn a_controller_of_another_name_address_or_bus_is_not_the_screen() {
    for (bus, addr, name) in [
        ("i2c-gpio", "5d", "Other"),
        ("i2c-gpio", "14", "Goodix"),
        ("i2c-1", "5d", "Goodix"),
    ] {
        let rig = Rig::new("other");
        rig.controller(bus, addr, name);
        rig.framebuffer("480,640");
        let (args, _) = rig.run();
        assert_eq!(args, "", "{name} at {addr} on {bus}");
        assert!(rig.loaded().is_empty());
    }
}

#[test]
fn a_controller_that_answers_late_is_found_within_the_wait() {
    let rig = Rig::new("late");
    rig.framebuffer("480,640");
    let root = rig.root.clone();
    let t = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        controller_in(&root, "i2c-gpio", "5d", "Goodix");
    });
    let (args, _) = rig.run();
    t.join().unwrap();
    assert_eq!(args, DPI28);
}

#[test]
fn a_rebind_leaving_no_framebuffer_starts_on_hdmi() {
    let rig = Rig::new("nofb");
    rig.controller("i2c-gpio", "5d", "Goodix");
    let (args, log) = rig.run();
    assert_eq!(args, "");
    assert!(log.contains("no framebuffer"));
    assert!(rig.loaded().is_empty(), "the overlays are removed");
}

#[test]
fn a_framebuffer_at_another_mode_starts_on_hdmi() {
    let rig = Rig::new("mode");
    rig.controller("i2c-gpio", "5d", "Goodix");
    rig.framebuffer("1920,1080");
    let (args, _) = rig.run();
    assert_eq!(args, "");
}

#[test]
fn nothing_found_is_hdmi_and_the_overlays_are_gone() {
    let rig = Rig::new("none");
    let (args, log) = rig.run();
    assert_eq!(args, "");
    assert!(log.contains("no panel found"));
    assert!(rig.loaded().is_empty());
}
