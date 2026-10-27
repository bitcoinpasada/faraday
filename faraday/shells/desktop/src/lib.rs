//! Faraday Demo's side of a window (`docs/DESKTOP-HOST.md`): everything
//! the Demo does on this computer apart from the window itself. The
//! stick folders, the Inbox and Outbox kept in memory, the Downloads
//! listing Transfer shows, the clipboard for Paste, the preview size kept
//! across a lock, and the fresh app a lock starts.
//!
//! Two windows hold it: Faraday Demo's own (`src/main.rs`, the
//! developer's `cargo run -p faraday-desktop`), and Faraday Starter's,
//! where the Demo runs under the Starter's top app bar (F126,
//! `docs/STARTER.md` §5.3). Each window keeps its loop, its pacing, its
//! pointer and its camera, and asks this for the rest.
//!
//! Sticks are folders under one folder, by default `faraday-sticks` in
//! the home folder: `BOOTSTICK`, the boot stick, which starts empty and to
//! which the Demo never adds a file (owner, 2026-10-19: a person puts
//! there the files to boot with), and one folder for each backup stick,
//! `STICK-2`, `STICK-3` and on, made when a stick is first put in and
//! empty. Nothing in them is ever moved, renamed or removed, so they stay
//! as a person left them across installs and new versions (owner,
//! 2026-10-20). The boot stick is in when the Demo opens, as the device
//! starts with the stick it booted from.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use faraday_core::{BOOT_FOLDER, Faraday, StickInfo, StorageEvent};
use faraday_shell_api::{Command, Event};
use faraday_storage::{Boxes, Dirs, Host, serve_with, stick_info};
use faraday_window::camera;
use faraday_window::clipboard::Clipboard;

/// How often the Downloads folder, the cameras and the sticks are looked
/// at again.
pub const RESCAN: Duration = Duration::from_millis(700);
/// The boot stick's folder (Stick 1).
pub const BOOT: &str = BOOT_FOLDER;
/// The first backup stick's folder, Stick 2.
pub const FIRST: &str = "STICK-2";

/// The folders the Demo works in.
#[derive(Debug, Clone)]
pub struct Paths {
    /// The stick folders.
    pub sticks: PathBuf,
    /// The folder Transfer sends from and saves into.
    pub downloads: PathBuf,
    /// Where Make PDF saves.
    pub print: PathBuf,
}

impl Paths {
    /// The defaults in the home folder: `faraday-sticks`, `Downloads` and
    /// `faraday-print`; in the folder the program runs in where there is no
    /// home folder.
    pub fn in_home() -> Paths {
        Paths {
            sticks: home()
                .map(|h| h.join("faraday-sticks"))
                .unwrap_or_else(|| PathBuf::from("faraday-sticks")),
            downloads: home()
                .map(|h| h.join("Downloads"))
                .unwrap_or_else(|| PathBuf::from("Downloads")),
            print: home()
                .map(|h| h.join("faraday-print"))
                .unwrap_or_else(|| PathBuf::from("faraday-print")),
        }
    }
}

/// What a command of the app's asks of the window, once the Demo has
/// answered its own part.
#[derive(Debug)]
pub enum Asked {
    /// Nothing more.
    Nothing,
    /// This event, for the app.
    Event(Event),
    /// A lock: the window holds a fresh app ([`Demo::restart`]).
    Restart,
    /// The app ended: power off.
    Exit,
}

/// What Transfer lists of this computer: the Downloads folder and the
/// cameras.
#[derive(Clone, PartialEq)]
struct Listing {
    files: StorageEvent,
    cameras: Vec<(String, String)>,
}

/// Faraday Demo's side of a window.
pub struct Demo {
    sticks_dir: PathBuf,
    /// Where PDFs are saved, and the Downloads folder Transfer uses.
    host: Host,
    /// What the listing thread reads: the Downloads folder and the
    /// cameras, sent when either changed.
    listing: Receiver<Listing>,
    /// Set when the Demo is dropped: the listing thread ends.
    stop: Arc<AtomicBool>,
    /// The newest of what the listing thread sent; a fresh app is given
    /// it again.
    latest: Option<Listing>,
    /// The Downloads folder as last listed to the app.
    listed: Option<StorageEvent>,
    /// The cameras as last listed to the app.
    cameras: Option<Vec<(String, String)>>,
    boxes: Boxes,
    /// The sticks' folders, their pictures read on a thread of their own.
    dirs: Dirs,
    attached: BTreeSet<String>,
    seen: Vec<StickInfo>,
    /// Sticks pressed in or out since they were last put in place.
    toggles: Vec<String>,
    /// A storage command was served since the sticks were last looked at.
    served: bool,
    /// When the sticks, the clock and the memory are next looked at.
    next_scan: Instant,
    /// The system clipboard, read for Paste.
    clipboard: Clipboard,
    /// The screen size the app is shown at, in dp, or `None` for the
    /// window's own (`docs/PREVIEW-SIZE.md`): kept across a lock.
    preview: Option<(u16, u16)>,
}

impl Drop for Demo {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Demo {
    /// Costs making the stick folders the first time ([`prepare`]) and
    /// starts the thread that lists Downloads and the cameras. The boot
    /// stick is in; none is when its folder has been renamed or removed.
    pub fn open(paths: Paths) -> Result<Demo, String> {
        prepare(&paths.sticks)?;
        let mut attached = BTreeSet::new();
        if paths.sticks.join(BOOT).is_dir() {
            attached.insert(BOOT.to_string());
        }
        let opens = faraday_storage::can_open_folders();
        let (sender, listing) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        watch_listing(paths.downloads.clone(), opens, sender, Arc::clone(&stop));
        Ok(Demo {
            sticks_dir: paths.sticks,
            host: Host {
                print: paths.print,
                downloads: paths.downloads,
                opens,
            },
            listing,
            stop,
            latest: None,
            listed: None,
            cameras: None,
            boxes: Boxes::Memory {
                inbox: Vec::new(),
                outbox: Vec::new(),
                kept: Vec::new(),
            },
            dirs: Dirs::default(),
            attached,
            seen: Vec::new(),
            toggles: Vec::new(),
            served: false,
            next_scan: Instant::now(),
            clipboard: Clipboard::system(),
            preview: None,
        })
    }

    /// The stick folders.
    pub fn sticks_dir(&self) -> &Path {
        &self.sticks_dir
    }

    /// The preview size shown, in dp, or `None` for the window's own.
    pub fn preview(&self) -> Option<(u16, u16)> {
        self.preview
    }

    /// What the app is told of the preview at its start.
    pub fn offered(&self) -> Event {
        Event::PreviewOffered {
            chosen: self.preview,
        }
    }

    /// Answers a command of the app's that is the Demo's: Paste's
    /// clipboard, the preview size, and the end of a session. Every other
    /// command is the window's ([`Asked::Nothing`]).
    pub fn command(&mut self, app: &mut Faraday, c: Command) -> Asked {
        match c {
            Command::RequestClipboard { kind } => Asked::Event(self.clipboard.answer(kind)),
            Command::PreviewSize { size } => {
                self.preview = size;
                Asked::Nothing
            }
            Command::Exit if app.restart_requested() => {
                // What the lock saved is kept for the fresh app.
                self.serve(app);
                Asked::Restart
            }
            Command::Exit => Asked::Exit,
            _ => Asked::Nothing,
        }
    }

    /// After the app took an event: the sticks it pressed in or out are
    /// noted and what it asked of storage is served. Returns the time
    /// storage took.
    pub fn took(&mut self, app: &mut Faraday) -> Duration {
        self.toggles.extend(app.take_stick_toggles());
        self.served |= app.storage_pending();
        let at = Instant::now();
        self.serve(app);
        at.elapsed()
    }

    /// Costs a look at every stick folder when a stick was pressed in or
    /// out or storage was served since the last look ([`Demo::took`]):
    /// the sticks are put in or pulled and looked at now; otherwise
    /// nothing. Returns whether it looked.
    pub fn settle(&mut self, app: &mut Faraday) -> bool {
        let looks = std::mem::take(&mut self.served) || !self.toggles.is_empty();
        for name in std::mem::take(&mut self.toggles) {
            self.toggle(&name);
        }
        if looks {
            self.rescan(app, false);
        }
        looks
    }

    /// Whether the timer's look at the sticks is due at `now`; when it is,
    /// the next is set.
    pub fn due(&mut self, now: Instant) -> bool {
        if now < self.next_scan {
            return false;
        }
        self.next_scan = now + RESCAN;
        true
    }

    /// Costs a look at every stick folder: tells the app which sticks are
    /// in when that changed (or always with `force`), with the clock and
    /// the memory free, and serves what it asked of storage. The commands
    /// it asks for are left for the window to poll. Returns whether the
    /// app was told.
    pub fn rescan(&mut self, app: &mut Faraday, force: bool) -> bool {
        let now = self.sticks();
        if !force && now == self.seen {
            return false;
        }
        self.seen = now.clone();
        app.storage(StorageEvent::Sticks(now));
        if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            app.storage(StorageEvent::Clock {
                unix_secs: t.as_secs(),
            });
        }
        if let Some(available_mib) = faraday_storage::memory_available_mib() {
            app.storage(StorageEvent::Memory { available_mib });
        }
        self.serve(app);
        true
    }

    /// Tells the app what Downloads holds and which cameras there are,
    /// when that changed: the newest of what the listing thread read, or
    /// the last one again for a fresh app. A lookup; the reading is done
    /// on that thread. Returns whether the app was told anything.
    pub fn relist(&mut self, app: &mut Faraday) -> bool {
        if let Some(now) = self.listing.try_iter().last() {
            self.latest = Some(now);
        }
        let Some(now) = self.latest.clone() else {
            return false;
        };
        let mut changed = false;
        if self.listed.as_ref() != Some(&now.files) {
            app.storage(now.files.clone());
            self.listed = Some(now.files);
            changed = true;
        }
        if self.cameras.as_ref() != Some(&now.cameras) {
            app.storage(StorageEvent::Cameras(now.cameras.clone()));
            self.cameras = Some(now.cameras);
            changed = true;
        }
        changed
    }

    /// A file dropped on the window, for Transfer to send.
    pub fn dropped(&mut self, app: &mut Faraday, path: &Path) {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        app.storage(StorageEvent::Dropped {
            path: path.display().to_string(),
            size,
        });
        self.serve(app);
    }

    /// The app a lock starts in place of `old`, in the mode `old` chose or
    /// was in: the window holds it, tells it its display and the preview
    /// ([`Demo::offered`]), then hands it to [`Demo::restarted`].
    pub fn restart(&mut self, old: &Faraday) -> Faraday {
        // Pictures the last app asked for are not this one's.
        self.dirs = Dirs::default();
        self.seen.clear();
        self.toggles.clear();
        self.served = false;
        desktop_app_in(old.mode_after_restart())
    }

    /// Costs a look at every stick folder: the fresh app of a lock is
    /// given the Inbox and Outbox the old one kept, the sticks, and the
    /// newest listing again.
    pub fn restarted(&mut self, app: &mut Faraday) {
        app.storage(self.boxes.restore());
        self.rescan(app, true);
        self.listed = None;
        self.cameras = None;
        self.relist(app);
    }

    /// Serves what the app asked of storage, printing what the storage
    /// says on stderr.
    fn serve(&mut self, app: &mut Faraday) {
        for line in serve_with(app, &mut self.boxes, Some(&self.host), &mut self.dirs) {
            eprintln!("{line}");
        }
    }

    fn sticks(&self) -> Vec<StickInfo> {
        self.attached
            .iter()
            // A folder renamed or removed is a stick pulled.
            .filter(|name| self.sticks_dir.join(name).is_dir())
            // The boot stick's first plugging in a session, at start,
            // reads its settings file and brings up the boot import.
            .map(|name| stick_info(&self.sticks_dir.join(name), name, name == BOOT))
            .collect()
    }

    /// Plugs the stick of folder `name` in, or pulls it out. A backup
    /// stick's folder is made the first time it is plugged in.
    fn toggle(&mut self, name: &str) {
        if self.attached.remove(name) {
            return;
        }
        if name != BOOT {
            let _ = std::fs::create_dir_all(self.sticks_dir.join(name));
        }
        self.attached.insert(name.to_string());
    }
}

/// Whether `now` is to be sent: it differs from the last one sent (the
/// first always is). A lookup.
fn is_new(last: &mut Option<Listing>, now: &Listing) -> bool {
    if last.as_ref() == Some(now) {
        return false;
    }
    *last = Some(now.clone());
    true
}

/// Costs a directory listing with each file's metadata and a camera
/// enumeration (Media Foundation's on Windows) every [`RESCAN`], on a
/// thread of its own so the window's thread never waits on either; sends
/// the listing when it changed, the first at once. Ends once `stop` is
/// set or nobody listens.
fn watch_listing(downloads: PathBuf, opens: bool, out: Sender<Listing>, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let host = Host {
            print: PathBuf::new(),
            downloads,
            opens,
        };
        let mut last = None;
        while !stop.load(Ordering::Relaxed) {
            let now = Listing {
                files: host.files(),
                cameras: camera::list(),
            };
            if is_new(&mut last, &now) && out.send(now).is_err() {
                return;
            }
            std::thread::sleep(RESCAN);
        }
    });
}

/// A fresh app as Faraday Demo runs it: the online Faraday, which turns
/// sheets into PDFs, in `mode`. Every build starts in Practice (the key
/// store's public set, on testnet); a lock starts the fresh session in
/// the mode the old one chose or was in. No warning comes first: the
/// steps that take a seed carry a line instead (F81).
pub fn desktop_app_in(mode: faraday_core::Mode) -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
    app.seed_line = true;
    // The window answers `RequestClipboard` ([`Demo::command`]).
    app.clipboard = true;
    app.shell_sticks = true;
    // The dice roller (`docs/CODEX32.md` §5.6) is Faraday Demo's alone.
    app.set_roller_offered(true);
    // A pointer and a keyboard: Add randomness offers Pointer and keys.
    app.set_pointer_keys_offered(true);
    app.session = faraday_core::wallet::Session::on(mode.network());
    if mode == faraday_core::Mode::Demo {
        app.session.set_demo(&faraday_samples::SAMPLES);
    }
    app.samples = Some(&faraday_samples::SAMPLES);
    app.set_modes(faraday_core::DESKTOP_MODES);
    app
}

/// Makes the stick folders the first time: `faraday-sticks`, an empty
/// boot stick and an empty Stick 2. The boot stick starts empty and the
/// Demo never adds to it: a person puts there the files to boot with
/// (owner, 2026-10-19). Nothing in a folder is ever moved, renamed or
/// removed here, so a person's sticks stay as they left them across
/// installs and new versions (owner, 2026-10-20). A boot stick folder
/// renamed or removed is not made again: the app then starts with no
/// stick in. Stick 2 is a stick the app starts with, made if it is not
/// there.
pub fn prepare(dir: &Path) -> Result<(), String> {
    let boot = dir.join(BOOT);
    let first_stick = dir.join(FIRST);
    // Made once: the mark says the folders have been made here.
    let made = dir.join(".faraday-sticks");
    if !made.exists() {
        std::fs::create_dir_all(&boot).map_err(|e| format!("{}: {e}", boot.display()))?;
        std::fs::write(&made, "BOOTSTICK and STICK-2 made\n")
            .map_err(|e| format!("{}: {e}", made.display()))?;
    }
    std::fs::create_dir_all(&first_stick).map_err(|e| format!("{}: {e}", first_stick.display()))
}

/// The person's home folder: `HOME`, and on Windows `USERPROFILE` when
/// `HOME` is not set. Downloads is `Downloads` in it; a Downloads folder
/// that Windows has moved elsewhere is not followed.
pub fn home() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty());
    #[cfg(windows)]
    let home = home.or_else(|| std::env::var_os("USERPROFILE").filter(|h| !h.is_empty()));
    home.map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The boot stick starts empty and the Demo adds nothing to it; what a
    /// person put in the stick folders, hidden files included, is left
    /// byte for byte by every later start (owner, 2026-10-20).
    #[test]
    fn a_persons_stick_folders_stay_as_they_left_them() {
        let dir = std::env::temp_dir().join(format!("faraday-prepare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        prepare(&dir).unwrap();
        let boot = dir.join(BOOT);
        let two = dir.join(FIRST);
        assert_eq!(std::fs::read_dir(&boot).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&two).unwrap().count(), 0);
        let files = [
            (boot.join("vault.ofv"), b"vault bytes".to_vec()),
            (boot.join(".faraday-testkit"), b"an old mark".to_vec()),
            (boot.join("spend.psbt"), b"psbt".to_vec()),
            (two.join("SHA256SUMS"), b"sums".to_vec()),
            (dir.join("STICK-3").join("notes.txt"), b"notes".to_vec()),
        ];
        for (path, bytes) in &files {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let listing = |d: &Path| {
            let mut names: Vec<_> = std::fs::read_dir(d)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            names.sort();
            names
        };
        let before: Vec<_> = [&dir, &boot, &two].iter().map(|d| listing(d)).collect();
        prepare(&dir).unwrap();
        prepare(&dir).unwrap();
        let after: Vec<_> = [&dir, &boot, &two].iter().map(|d| listing(d)).collect();
        assert_eq!(before, after);
        for (path, bytes) in &files {
            assert_eq!(&std::fs::read(path).unwrap(), bytes, "{}", path.display());
        }
        // A boot stick removed is not made again.
        std::fs::remove_dir_all(&boot).unwrap();
        prepare(&dir).unwrap();
        assert!(!boot.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The listing thread tells the window only of a change.
    #[test]
    fn a_listing_is_sent_only_when_it_changed() {
        let mut last = None;
        let mut now = Listing {
            files: StorageEvent::Cameras(Vec::new()),
            cameras: Vec::new(),
        };
        assert!(is_new(&mut last, &now));
        assert!(!is_new(&mut last, &now));
        now.cameras.push(("0".into(), "Webcam".into()));
        assert!(is_new(&mut last, &now));
        assert!(!is_new(&mut last, &now));
    }

    /// The Demo opens on testnet, which takes any seed, with no warning
    /// to acknowledge; in Practice only the public seeds (F81).
    #[test]
    fn the_desktop_opens_in_demo_on_testnet_and_a_mode_change_locks() {
        let c = faraday_core::testkit::compute();
        let mut app = desktop_app_in(faraday_core::Mode::Demo);
        assert_eq!(app.mode(), faraday_core::Mode::Demo);
        assert_eq!(app.session.network(), faraday_core::testkit::NET);
        let other = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
        assert!(app.session.keys.add_words(&c, other, "", None).is_err());
        let public = faraday_core::testkit::test_words("zebra");
        assert!(app.session.keys.add_words(&c, &public, "", None).is_ok());
        app.press(faraday_core::Action::Mode(faraday_core::Mode::Signet));
        assert_eq!(app.mode_after_restart(), faraday_core::Mode::Signet);
        assert!(app.restart_requested());
    }

    /// Every build opens in Practice with no sheet; a lock keeps the mode,
    /// and a mode chosen is where the fresh session starts.
    #[test]
    fn every_build_opens_in_demo_with_no_sheet() {
        let app = desktop_app_in(faraday_core::Mode::Demo);
        assert!(app.sheet.is_none());
        assert_eq!(app.mode(), faraday_core::Mode::Demo);
        assert_eq!(app.session.network(), faraday_core::testkit::NET);
    }

    #[test]
    fn leaving_demo_for_testnet_starts_a_fresh_testnet_session() {
        let mut app = desktop_app_in(faraday_core::Mode::Demo);
        app.press(faraday_core::Action::Mode(faraday_core::Mode::Testnet));
        let fresh = desktop_app_in(app.mode_after_restart());
        assert!(fresh.sheet.is_none());
        assert_eq!(fresh.mode(), faraday_core::Mode::Testnet);
        assert!(!fresh.session.keys.demo());
        let locked = desktop_app_in(fresh.mode_after_restart());
        assert_eq!(locked.mode(), faraday_core::Mode::Testnet);
    }
}
