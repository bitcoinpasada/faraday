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
    fn install(&self, _payload: Option<&Payload>) -> Result<String, Reason> {
        Err(Reason::Unsupported)
    }
    fn uninstall(&self) -> Result<(), Reason> {
        Ok(())
    }
    fn run_desktop(&self, _payload: Option<&Payload>) -> Result<(), Reason> {
        Err(Reason::Unsupported)
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
fn a_payload_carrying_two_systems_desktop_apps_offers_only_this_systems() {
    let line = |name: &str| {
        format!(
            "{name}\t{}\tinner\t{}\t1\t0.2.0\n",
            "a".repeat(64),
            "b".repeat(64)
        )
    };
    let manifest = format!(
        "# Faraday Starter payload 0.2.0, commit 0123456789abcdef0123456789abcdef01234567\n{}{}{}",
        line("faraday-desktop-0.2.0-linux-x86_64.xz"),
        line("faraday-desktop-0.2.0-windows-x86_64.exe.xz"),
        line("faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz"),
    );
    let payload = Payload::from_parts(manifest.as_bytes(), Vec::new()).expect("a payload");
    let ours = match std::env::consts::OS {
        "linux" => "faraday-desktop-0.2.0-linux-x86_64.xz",
        "windows" => "faraday-desktop-0.2.0-windows-x86_64.exe.xz",
        _ => return,
    };
    let offered: Vec<String> = payload.event().files.into_iter().map(|f| f.name).collect();
    assert_eq!(
        offered,
        [ours, "faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz"]
    );
    assert_eq!(payload.desktop_line().map(|l| l.name.as_str()), Some(ours));
}
