//! Upgrading a Faraday stick end to end in one process: the app, the
//! shell's side of the storage requests, and the boot copier over a
//! stand-in `/sys`, `/dev` and `/proc/version` whose partitions are image
//! files. Settings → Upgrade, the boot stick read as the source, an older
//! stick written, read back and shown as this Faraday, its data partition
//! untouched.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;

use faraday_boot::Boots;
use faraday_core::upgrade::Step;
use faraday_core::{Action, Faraday, Screen};
use faraday_storage::{Boxes, Dirs, boot_poll, serve_all};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const RUNNING: &str = "6.6.84-faraday-0.2.0+4d0680b1a2b3";
const MIB: usize = 1 << 20;

fn boot_image(len: usize, release: &str, seed: u8) -> Vec<u8> {
    let mut b: Vec<u8> = (0..len)
        .map(|i| (i as u32).wrapping_mul(2_654_435_761).to_le_bytes()[3] ^ seed)
        .collect();
    let text = format!("{release} (user@host) #1 SMP\0");
    b[299_999] = 0;
    b[300_000..300_000 + text.len()].copy_from_slice(text.as_bytes());
    b
}

struct Machine {
    root: PathBuf,
}

impl Machine {
    fn new(name: &str) -> Machine {
        let root = std::env::temp_dir().join(format!(
            "faraday-storage-upgrade-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sys/class/block")).unwrap();
        fs::create_dir_all(root.join("dev")).unwrap();
        fs::write(
            root.join("version"),
            format!("Linux version {RUNNING} (user@host) #1 SMP\n"),
        )
        .unwrap();
        Machine { root }
    }

    fn boots(&self) -> Boots {
        Boots::new(
            &self.root.join("sys"),
            &self.root.join("dev"),
            &self.root.join("version"),
        )
    }

    /// A Faraday stick on USB, both partitions handed out.
    fn stick(&self, disk: &str, seq: u32, boot: &[u8], data: &[u8]) {
        let d = self
            .root
            .join(format!("sys/devices/pci0/usb1/1-{seq}/block/{disk}"));
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("diskseq"), format!("{seq}\n")).unwrap();
        for (k, (name, image)) in [("OSKBOOT", boot), ("OSKDATA", data)].iter().enumerate() {
            let node = format!("{disk}{}", k + 1);
            let p = d.join(&node);
            fs::create_dir_all(&p).unwrap();
            fs::write(p.join("partition"), format!("{}\n", k + 1)).unwrap();
            fs::write(p.join("uevent"), format!("PARTNAME={name}\n")).unwrap();
            symlink(&p, self.root.join("sys/class/block").join(&node)).unwrap();
            let dev = self.root.join("dev").join(&node);
            fs::write(&dev, image).unwrap();
            fs::set_permissions(&dev, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    fn node(&self, name: &str) -> Vec<u8> {
        fs::read(self.root.join("dev").join(name)).unwrap()
    }
}

impl Drop for Machine {
    /// The tree is a test's own; leaving it fills the temp directory.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn device() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// The shell's pass: what the app asked for, answered.
fn serve(app: &mut Faraday, boots: &mut Boots) {
    let mut boxes = Boxes::Memory {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: Vec::new(),
    };
    serve_all(app, &mut boxes, None, &mut Dirs, Some(boots));
}

#[test]
fn an_older_stick_is_upgraded_from_the_boot_stick_and_keeps_its_data() {
    let m = Machine::new("flow");
    let source = boot_image(3 * MIB, RUNNING, 1);
    m.stick("sda", 1, &source, &vec![7u8; MIB]);
    let mut boots = m.boots();
    let mut app = device();
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::UpgradeOpen);
    assert!(app.upgrading());
    // The shell looks at the copier's partitions and serves the read the
    // app asks for.
    boot_poll(&mut app, &mut boots);
    serve(&mut app, &mut boots);
    let u = app.upgrade.as_ref().unwrap();
    assert_eq!(u.step(), Step::Target);
    assert_eq!(u.source.as_ref().map(|s| s.0.as_str()), Some("sda1@sda#1"));
    // The boot stick out, the vault stick in.
    fs::remove_file(m.root.join("sys/class/block/sda1")).unwrap();
    fs::remove_file(m.root.join("sys/class/block/sda2")).unwrap();
    let old = boot_image(4 * MIB, "6.6.84", 2);
    let vaults: Vec<u8> = (0..2 * MIB).map(|i| (i % 253) as u8).collect();
    m.stick("sdb", 2, &old, &vaults);
    boot_poll(&mut app, &mut boots);
    let target = app.upgrade.as_ref().unwrap().target().cloned().unwrap();
    assert_eq!(target.id, "sdb1@sdb#2");
    assert_eq!(target.release, None);
    let _ = app.frame();
    assert!(app.offers(Action::UpgradeWrite));
    app.press(Action::UpgradeWrite);
    serve(&mut app, &mut boots);
    let u = app.upgrade.as_ref().unwrap();
    assert_eq!(u.step(), Step::Done);
    assert_eq!(
        u.parts
            .iter()
            .find(|p| p.id == "sdb1@sdb#2")
            .and_then(|p| p.release.as_deref()),
        Some(RUNNING),
        "listed again as this Faraday"
    );
    let now = m.node("sdb1");
    assert!(now[..source.len()] == source[..]);
    assert!(now[source.len()..] == old[source.len()..]);
    assert_eq!(m.node("sdb2"), vaults);
}

#[test]
fn a_stick_with_a_smaller_boot_partition_is_not_written() {
    let m = Machine::new("small");
    let source = boot_image(3 * MIB, RUNNING, 3);
    m.stick("sda", 1, &source, &vec![0u8; MIB]);
    let small = boot_image(2 * MIB, "6.6.84", 4);
    m.stick("sdb", 2, &small, &vec![0u8; MIB]);
    let mut boots = m.boots();
    let mut app = device();
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::UpgradeOpen);
    boot_poll(&mut app, &mut boots);
    serve(&mut app, &mut boots);
    boot_poll(&mut app, &mut boots);
    app.press(Action::UpgradeWrite);
    serve(&mut app, &mut boots);
    let u = app.upgrade.as_ref().unwrap();
    assert_eq!(u.step(), Step::Target);
    assert_eq!(m.node("sdb1"), small);
}
