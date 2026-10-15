//! A shell for the tests of the backup flow: sticks that hold files, a
//! machine that restarts clean on a lock, and the presses a person makes
//! to bring in each stick, make the key and write each stick
//! (`docs/BACKUP-FLOW.md`).
#![allow(dead_code)]

use faraday_core::backup_flow::Kind;
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::catalog::{Go, TILES};
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

pub type Files = Vec<(String, Vec<u8>)>;

pub const CLOCK: u64 = 1_791_482_652;

/// A stick the shell can show.
#[derive(Clone)]
pub struct Disk {
    pub id: String,
    pub label: String,
    pub serial: Option<u32>,
    pub files: Files,
    pub free: u64,
    pub read_only: bool,
    pub not_fat: bool,
    /// The medium the machine started from.
    pub boot: bool,
    /// Its room cannot be asked for: the stick cannot be opened.
    pub unreadable: bool,
}

impl Disk {
    pub fn empty(id: &str, label: &str) -> Disk {
        Disk {
            id: id.into(),
            label: label.into(),
            serial: Some(0x1234_5678),
            files: Vec::new(),
            free: 50_000_000,
            read_only: false,
            not_fat: false,
            boot: false,
            unreadable: false,
        }
    }

    pub fn info(&self) -> StickInfo {
        StickInfo {
            id: self.id.clone(),
            label: self.label.clone(),
            boot: self.boot,
            serial: self.serial,
            files: self
                .files
                .iter()
                .map(|(n, b)| (n.clone(), b.len() as u64))
                .collect(),
        }
    }

    pub fn file(&self, name: &str) -> Option<&Vec<u8>> {
        self.files.iter().find(|(n, _)| n == name).map(|(_, b)| b)
    }
}

/// The machine around the app: its sticks, what it kept across a lock.
#[derive(Default)]
pub struct Shell {
    pub disks: Vec<Disk>,
    pub present: Vec<String>,
    pub boxes: (Files, Files, Files),
    /// Every write answered: the disk's id and the name written as.
    pub writes: Vec<(String, String)>,
    /// Every screen the app showed after a press or an answer.
    pub screens: Vec<Screen>,
    pub sheets: Vec<Sheet>,
    /// Writes are not answered: they wait here.
    pub hold_writes: bool,
    pub stashed: Vec<StorageCommand>,
    /// Every write is answered as failed.
    pub fail_writes: bool,
    /// Keeping the state across a lock fails.
    pub fail_save: bool,
    /// Every write is answered as failed on its read-back.
    pub mismatch_writes: bool,
    /// Reads, space and writes asked while the process was not clean.
    pub unclean_io: Vec<String>,
}

pub fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

pub fn settle(app: &mut Faraday, from: u64) {
    settle_n(app, from, 20);
}

/// `n` frames and ticks, a second apart.
pub fn settle_n(app: &mut Faraday, from: u64, n: u64) {
    for t in from..from + n {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn display(app: &mut Faraday) {
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
}

/// A process with these files, the clock known and its time kept.
pub fn process(inbox: Files, outbox: Files, kept: Files) -> Faraday {
    let mut app = Faraday::new();
    display(&mut app);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    app.vaults.ms_per_unit = Some(180);
    app
}

impl Shell {
    pub fn new() -> Shell {
        Shell::default()
    }

    pub fn add(&mut self, d: Disk) {
        self.disks.push(d);
    }

    pub fn disk(&self, id: &str) -> &Disk {
        self.disks.iter().find(|d| d.id == id).expect("a disk")
    }

    pub fn disk_mut(&mut self, id: &str) -> &mut Disk {
        self.disks.iter_mut().find(|d| d.id == id).expect("a disk")
    }

    pub fn infos(&self) -> Vec<StickInfo> {
        self.present.iter().map(|id| self.disk(id).info()).collect()
    }

    /// A fresh process from what the last save kept, with the sticks still
    /// in the machine.
    pub fn restart(&mut self) -> Faraday {
        self.pump_saved_only();
        let (inbox, outbox, kept) = self.boxes.clone();
        let mut app = process(inbox, outbox, kept);
        app.press(Action::OpenSetTime);
        app.press(Action::SetTimeKeep);
        self.shown(&app);
        if !self.present.is_empty() {
            app.storage(StorageEvent::Sticks(self.infos()));
        }
        self.pump(&mut app);
        app
    }

    fn pump_saved_only(&mut self) {}

    fn shown(&mut self, app: &Faraday) {
        self.screens.push(app.screen);
        if let Some(s) = app.sheet {
            self.sheets.push(s);
        }
    }

    /// The shell answers every request the app has made.
    pub fn pump(&mut self, app: &mut Faraday) {
        for _ in 0..200 {
            let Some(c) = app.poll_storage() else {
                break;
            };
            // A stick is read and written only by a clean process
            // (`PLAN.md` §5.3): the marker is checked on every request.
            if matches!(
                c,
                StorageCommand::Read { .. }
                    | StorageCommand::StickSpace { .. }
                    | StorageCommand::Write { .. }
            ) && !app.clean()
            {
                self.unclean_io
                    .push(format!("{c:?}").chars().take(60).collect());
            }
            match c {
                StorageCommand::Read { stick, name } => {
                    let d = self.disk(&stick);
                    match d.file(&name) {
                        Some(b) => {
                            let bytes = b.clone();
                            app.storage(StorageEvent::Read { stick, name, bytes });
                        }
                        None => app.storage(StorageEvent::ReadFailed {
                            stick,
                            name,
                            reason: "gone".into(),
                        }),
                    }
                }
                StorageCommand::StickSpace { stick } => {
                    let (free, ro, nf, un) = {
                        let d = self.disk(&stick);
                        (d.free, d.read_only, d.not_fat, d.unreadable)
                    };
                    if un {
                        app.storage(StorageEvent::ZeroFailed {
                            stick,
                            reason: "cannot open".into(),
                        });
                    } else if ro || nf {
                        app.storage(StorageEvent::StickProblem {
                            stick,
                            read_only: ro,
                        });
                    } else {
                        app.storage(StorageEvent::StickSpace {
                            stick,
                            free,
                            size: 100_000_000,
                        });
                    }
                }
                StorageCommand::Write {
                    stick,
                    name,
                    bytes,
                    replace,
                } if self.hold_writes => {
                    self.stashed.push(StorageCommand::Write {
                        stick,
                        name,
                        bytes,
                        replace,
                    });
                }
                StorageCommand::Write {
                    stick,
                    name,
                    bytes,
                    replace,
                } => {
                    if self.fail_writes {
                        app.storage(StorageEvent::WriteFailed {
                            stick,
                            name,
                            reason: "no space".into(),
                        });
                        continue;
                    }
                    // A stick that keeps other bytes than it was given: the
                    // disk process reads the file back, finds it differs and
                    // says so.
                    let mut bytes = bytes;
                    let mut bad = false;
                    if self.mismatch_writes
                        && let Some(b) = bytes.last_mut()
                    {
                        *b ^= 1;
                        bad = true;
                    }
                    let d = self.disk_mut(&stick);
                    if d.read_only || bytes.len() as u64 > d.free {
                        app.storage(StorageEvent::WriteFailed {
                            stick,
                            name,
                            reason: "no room".into(),
                        });
                        continue;
                    }
                    let wrote_as = write_any(&mut d.files, &name, &bytes, replace);
                    d.free -= bytes.len() as u64;
                    if bad {
                        app.storage(StorageEvent::WriteFailed {
                            stick,
                            name,
                            reason: "read back differs".into(),
                        });
                        continue;
                    }
                    self.writes.push((stick.clone(), wrote_as.clone()));
                    app.storage(StorageEvent::Written {
                        stick: stick.clone(),
                        name,
                        wrote_as,
                    });
                    app.storage(StorageEvent::Sticks(self.infos()));
                }
                StorageCommand::SaveBoxes {
                    inbox,
                    outbox,
                    kept,
                } => {
                    if self.fail_save {
                        app.storage(StorageEvent::NotKept);
                    } else {
                        self.boxes = (inbox, outbox, kept);
                    }
                }
                _ => {}
            }
        }
        self.shown(app);
    }

    /// The writes that were held land on their sticks, with no answer: a
    /// write that landed whose answer was lost.
    pub fn land_stashed(&mut self, which: impl Fn(&str) -> bool) {
        let stashed = std::mem::take(&mut self.stashed);
        for c in stashed {
            if let StorageCommand::Write {
                stick,
                name,
                bytes,
                replace,
            } = c
                && which(&name)
            {
                let d = self.disk_mut(&stick);
                d.free = d.free.saturating_sub(bytes.len() as u64);
                write_any(&mut d.files, &name, &bytes, replace);
            }
        }
    }

    /// How many vault files were written.
    pub fn vault_writes(&self) -> usize {
        self.writes
            .iter()
            .filter(|(_, n)| n.ends_with(".ofv"))
            .count()
    }

    pub fn plug(&mut self, app: &mut Faraday, id: &str) {
        if !self.present.iter().any(|p| p == id) {
            self.present.push(id.into());
        }
        app.storage(StorageEvent::Sticks(self.infos()));
        self.pump(app);
    }

    pub fn pull(&mut self, app: &mut Faraday, id: &str) {
        self.present.retain(|p| p != id);
        app.storage(StorageEvent::Sticks(self.infos()));
        self.pump(app);
    }

    pub fn press(&mut self, app: &mut Faraday, a: Action) {
        app.press(a);
        self.pump(app);
    }

    pub fn flow(&mut self, app: &mut Faraday, a: F) {
        self.press(app, Action::Flow(a));
    }

    /// A lock: the process the app was is gone, and a clean one reads what
    /// it kept.
    pub fn lock(&mut self, app: &mut Faraday) -> Faraday {
        app.press(Action::Lock);
        self.pump(app);
        assert!(app.restart_requested(), "locked");
        self.restart()
    }
}

/// What `faraday-files` does with a write: a vault over the vault of its
/// salt when `replace`, else beside under a free name; other files beside
/// or over.
pub fn write_any(files: &mut Files, name: &str, bytes: &[u8], replace: bool) -> String {
    if name.ends_with(".ofv")
        && bytes.starts_with(b"OFVT")
        && let Ok(h) = faraday_vault::read_header(bytes)
    {
        if replace
            && let Some(at) = files.iter().position(|(n, b)| {
                n.ends_with(".ofv")
                    && b.len() == bytes.len()
                    && faraday_vault::read_header(b).is_ok_and(|o| o.salt == h.salt)
            })
        {
            files[at].1 = bytes.to_vec();
            return files[at].0.clone();
        }
    } else if replace && let Some(at) = files.iter().position(|(n, _)| n == name) {
        files[at].1 = bytes.to_vec();
        return name.to_string();
    }
    let free = (1..100)
        .map(|k| {
            if k == 1 {
                name.to_string()
            } else {
                let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
                format!("{stem}-{k}.{ext}")
            }
        })
        .find(|n| !files.iter().any(|(m, _)| m == n))
        .unwrap();
    files.push((free.clone(), bytes.to_vec()));
    free
}

/// A vault file with a label and these passphrases.
pub fn vault(label: &str, phrases: &[&str], seed: u8) -> Vec<u8> {
    let cost = faraday_vault::Cost {
        memory_kib: faraday_vault::MIN_MEMORY_KIB,
        passes: 1,
        lanes: 1,
    };
    let contents: Vec<faraday_vault::Contents> = phrases
        .iter()
        .map(|_| faraday_vault::Contents {
            records: vec![
                faraday_vault::Record::new(faraday_vault::records::kind::SLOT_LABEL)
                    .with(faraday_vault::records::field::LABEL, label.as_bytes()),
            ],
        })
        .collect();
    let p: Vec<&[u8]> = phrases.iter().map(|p| p.as_bytes()).collect();
    faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &p,
        &contents,
        &[seed; 32],
    )
    .unwrap()
}

/// The tile of Tools that makes this kind.
pub fn tile_of(kind: Kind) -> u8 {
    let c = kind.category();
    TILES
        .iter()
        .position(|t| t.go == Go::VaultCategory(c))
        .expect("a tile") as u8
}

/// Starts the flow as Tools' tile does.
pub fn start(app: &mut Faraday, kind: Kind) {
    app.press(Action::Catalog(tile_of(kind)));
}

pub fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts()
}

/// Whether the screen drawn now offers this action.
pub fn offered(app: &mut Faraday, a: Action) -> bool {
    let _ = app.frame();
    app.offers(a)
}

pub fn has(app: &mut Faraday, want: &str) -> bool {
    texts(app).iter().any(|t| t == want)
}

pub fn has_part(app: &mut Faraday, part: &str) -> bool {
    texts(app).iter().any(|t| t.contains(part))
}

/// Create a vault inside the flow: the passphrase typed twice, then made.
pub fn create_vault(shell: &mut Shell, app: &mut Faraday, v: usize, phrase: &str) {
    shell.flow(app, F::CreateVault(v));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(app, phrase);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, phrase);
    app.press(Action::Vault(V::CGo));
    settle(app, 10);
    settle(app, 40);
}

/// Unlock the vault the flow asks for with this passphrase.
pub fn unlock(app: &mut Faraday, phrase: &str) {
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    settle(app, 100);
}

/// Sets the clock checked, as a person on the stick does before a key.
pub fn keep_time(app: &mut Faraday) {
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
}

/// Make the key with the fields typed.
/// Returns the toast the press left, before any tick.
pub fn make_key(shell: &mut Shell, app: &mut Faraday, fields: &[&str]) -> Option<String> {
    for (k, text) in fields.iter().enumerate() {
        app.press(Action::Vault(V::FocusField(k)));
        for _ in 0..80 {
            app.event(Event::Key(Key::Backspace));
        }
        type_text(app, text);
    }
    shell.flow(app, F::Make);
    let toast = app.toast_text().map(str::to_string);
    settle_n(app, 200, 2);
    shell.pump(app);
    toast
}

/// Panics with everything drawn when `want` is not on the screen.
pub fn expect(app: &mut Faraday, want: &str) {
    let t = texts(app);
    assert!(t.iter().any(|x| x == want), "{want:?} not in {t:?}");
}

/// Panics with everything drawn when no text contains `part`.
pub fn expect_part(app: &mut Faraday, part: &str) {
    let t = texts(app);
    assert!(t.iter().any(|x| x.contains(part)), "{part:?} not in {t:?}");
}

/// A flow standing on Make over the vault open now: where a test that is
/// not about the flow starts from. (The flow itself is tested end to end
/// in `backup_flow.rs`.)
pub fn at_make(app: &mut Faraday, kind: Kind) {
    use faraday_core::backup_flow::{BackupFlow, Plan, Step};
    let (salt, name) = {
        let o = &app.vaults.open[app.vaults.current];
        (o.header().salt, o.name.clone())
    };
    let mut plan = Plan::key_default();
    plan.vaults[0].salt = Some(salt);
    plan.vaults[0].file = Some(name);
    plan.vaults[0].existing = true;
    let mut f = BackupFlow::with_plan(kind, plan);
    f.step = Step::Make;
    app.backup_flows.push(f);
    let i = app.backup_flows.len() - 1;
    app.press(Action::Flow(F::Show(i)));
}

/// Type the fields of the Make form and press Make. Returns the toast.
pub fn make_in_form(app: &mut Faraday, fields: &[&str]) -> Option<String> {
    for (k, text) in fields.iter().enumerate() {
        app.press(Action::Vault(V::FocusField(k)));
        for _ in 0..80 {
            app.event(Event::Key(Key::Backspace));
        }
        type_text(app, text);
    }
    app.press(Action::Flow(F::Make));
    let toast = app.toast_text().map(str::to_string);
    // An RSA key is made on the tick after "Making the key" is drawn.
    if app.vaults.working.is_some() {
        settle_n(app, 300, 2);
    }
    toast
}

/// Every place of every flow written, as if the sticks had been: the key
/// a vault unlocked from a stick holds signs at once.
pub fn release_all(app: &mut Faraday) {
    use faraday_core::backup_flow::Written;
    for f in &mut app.backup_flows {
        for s in &mut f.slots {
            s.written = Some(Written {
                stick: String::new(),
                serial: None,
                label: "STICK".into(),
                time: None,
                files: Vec::new(),
            });
        }
    }
}

/// A key made in the open vault by the flow's Make, then released, its
/// page selected. When Make is refused (the time is asked for first)
/// nothing is released and the screen stays.
pub fn make_released(app: &mut Faraday, kind: Kind, fields: &[&str]) -> Option<String> {
    at_make(app, kind);
    let toast = make_in_form(app, fields);
    if app.backup_flows.last().is_some_and(|f| f.made) {
        release_all(app);
        app.press(Action::Nav(Screen::VaultContents));
        app.press(Action::Vault(V::Category(kind.category())));
    }
    toast
}
