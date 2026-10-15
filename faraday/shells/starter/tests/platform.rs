//! Restart from the stick through the per-system seam (`docs/STARTER.md`
//! §3.1): a system with no one-time boot of its own, as Windows, answers
//! it with its advanced startup (`shutdown /r /o /t 0`), whose *Use a
//! device* starts the stick. A stand-in records what it was asked.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender};

use faraday_starter::api::{Computer, Installed, Reason, StarterEvent, WriteOrder};
use faraday_starter_shell::payload::Payload;
use faraday_starter_shell::platform::Platform;
use faraday_starter_shell::writer::Job;

#[derive(Default)]
struct Recorded(Mutex<Vec<&'static str>>);

impl Recorded {
    fn note(&self, s: &'static str) {
        self.0.lock().expect("lock").push(s);
    }
}

impl Platform for Recorded {
    fn computer(&self) -> Computer {
        Computer::default()
    }
    fn watch(&self, _out: Sender<StarterEvent>, _forget: Receiver<String>) {}
    fn write(&self, _order: WriteOrder, _job: Job, _out: Sender<StarterEvent>) {}
    fn restart_into_firmware(&self) -> Result<(), Reason> {
        self.note("firmware");
        Ok(())
    }
    fn restart_to_choose_device(&self) -> Result<(), Reason> {
        self.note("choose a device");
        Ok(())
    }
    fn installed(&self) -> Option<Installed> {
        None
    }
    fn install(&self) -> Result<String, Reason> {
        Err(Reason::Unsupported)
    }
    fn uninstall(&self) -> Result<(), Reason> {
        Ok(())
    }
    fn downloads(&self) -> PathBuf {
        PathBuf::from("Downloads")
    }
}

#[test]
fn restart_from_the_stick_is_windows_advanced_startup_where_there_is_no_one_time_boot() {
    let system = Recorded::default();
    assert_eq!(system.restart_from_stick("\\\\.\\PhysicalDrive2"), Ok(()));
    assert_eq!(*system.0.lock().expect("lock"), vec!["choose a device"]);
    // And such a system has no firmware entry for uninstall to remove.
    assert_eq!(system.remove_stick_entry(), Ok(()));
}

#[test]
fn the_payload_the_starter_shows_is_the_stick_and_pi_images_and_never_a_desktop_app() {
    let line = |name: &str| {
        format!(
            "{name}\t{}\tinner\t{}\t1\t0.2.0\n",
            "a".repeat(64),
            "b".repeat(64)
        )
    };
    let header =
        "# Faraday Starter payload 0.2.0, commit 0123456789abcdef0123456789abcdef01234567\n";
    let images = format!(
        "{header}{}{}",
        line("faraday-stick-0.2.0-x86_64-uefi.img.xz"),
        line("faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz"),
    );
    let payload = Payload::from_parts(images.as_bytes(), Vec::new()).expect("a payload");
    let shown: Vec<String> = payload.event().files.into_iter().map(|f| f.name).collect();
    assert_eq!(
        shown,
        [
            "faraday-stick-0.2.0-x86_64-uefi.img.xz",
            "faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz"
        ]
    );
    // A manifest that names a desktop app, as one before F30 did, is
    // refused whole.
    for desktop in [
        "faraday-desktop-0.2.0-linux-x86_64.xz",
        "faraday-desktop-0.2.0-windows-x86_64.exe.xz",
    ] {
        let with = format!("{images}{}", line(desktop));
        assert!(
            Payload::from_parts(with.as_bytes(), Vec::new()).is_err(),
            "{desktop}"
        );
    }
}
