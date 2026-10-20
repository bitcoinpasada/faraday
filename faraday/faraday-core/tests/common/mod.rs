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
    /// The writes of files whose names end so are answered as failed on
    /// their read-back, so many times, then written as given.
    pub mismatch_names: Vec<(String, usize)>,
    /// Pictures whose names end so read as holding no code, so many
    /// times.
    pub blank_pictures: Vec<(String, usize)>,
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
                    let named = self
                        .mismatch_names
                        .iter_mut()
                        .find(|(end, n)| *n > 0 && name.ends_with(end.as_str()));
                    let named = match named {
                        Some((_, n)) => {
                            *n -= 1;
                            true
                        }
                        None => false,
                    };
                    if (self.mismatch_writes || named)
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
                // The disk process's picture reader, on the disk's file.
                StorageCommand::ReadQr { stick, name } => {
                    let Some(bytes) = self.disk(&stick).file(&name).cloned() else {
                        app.storage(StorageEvent::ReadFailed {
                            stick,
                            name,
                            reason: "gone".into(),
                        });
                        continue;
                    };
                    let blank = self
                        .blank_pictures
                        .iter_mut()
                        .find(|(end, n)| *n > 0 && name.ends_with(end.as_str()));
                    let payloads = match blank {
                        Some((_, n)) => {
                            *n -= 1;
                            Vec::new()
                        }
                        None => faraday_files::qr_in_png(&bytes).unwrap_or_default(),
                    };
                    app.storage(StorageEvent::QrRead {
                        name,
                        bytes,
                        payloads,
                    });
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

    /// The writes that were held land on their sticks and are answered,
    /// as they would have been without the hold.
    pub fn release_writes(&mut self, app: &mut Faraday) {
        self.hold_writes = false;
        for c in std::mem::take(&mut self.stashed) {
            if let StorageCommand::Write {
                stick,
                name,
                bytes,
                replace,
            } = c
            {
                let d = self.disk_mut(&stick);
                let wrote_as = write_any(&mut d.files, &name, &bytes, replace);
                d.free -= bytes.len() as u64;
                self.writes.push((stick.clone(), wrote_as.clone()));
                app.storage(StorageEvent::Written {
                    stick,
                    name,
                    wrote_as,
                });
            }
        }
        app.storage(StorageEvent::Sticks(self.infos()));
        self.pump(app);
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

/// Past **Lock and write**'s secret sheet for each seed the plan keeps as
/// a file on a stick: the line ticked and the seed let out, once per seed
/// (`docs/BACKUP-FLOW.md` §19.12 item 6). Returns how many sheets it
/// went through.
pub fn let_seed_files_out(app: &mut Faraday) -> usize {
    let mut n = 0;
    while app.sheet == Some(Sheet::SecretOut)
        && app.secret_out.as_ref().is_some_and(|o| o.to_lock.is_some())
        && n < 16
    {
        app.press(Action::SecretAck);
        app.press(Action::SecretUnprotected);
        n += 1;
    }
    n
}

// ---- the backup board (`docs/BACKUP-FLOW.md` §19) ----

use faraday_core::board::{BoardPress, Chip, Thing, Zone};
use faraday_core::plan::{self, At, Spot, What};

/// A press on Back up's board, after **Customize backup** where the board
/// is still read-only (`docs/BACKUP-FLOW.md` §19.6).
pub fn board(app: &mut Faraday, p: BoardPress) {
    if app.board_plan().is_some_and(|(_, a)| !a.custom) {
        app.press(Action::Board(BoardPress::Customize));
    }
    app.press(Action::Board(p));
    // A drop that let a place see the balance asks (§19.5): kept.
    if app.sheet == Some(faraday_core::Sheet::BoardSees) {
        app.press(Action::Board(BoardPress::SeesKeep));
    }
}

/// Tap to place: `t` pressed in the list, then zone `z`.
pub fn put(app: &mut Faraday, t: Thing, z: Zone) {
    board(app, BoardPress::Chip(Chip::List(t)));
    board(app, BoardPress::Zone(z));
}

/// The placed chip of `t` at `z` pressed, then zone `to`: moved there.
pub fn move_to(app: &mut Faraday, z: Zone, t: Thing, to: Zone) {
    board(app, BoardPress::Chip(Chip::Placed(z, t)));
    board(app, BoardPress::Zone(to));
}

/// The × on the placed chip of `t` at `z`.
pub fn take_off(app: &mut Faraday, z: Zone, t: Thing) {
    board(app, BoardPress::Off(z, t));
}

/// Form `k` of the placed chip of `t` at `z` turned on or off by its
/// toggle on the chip.
pub fn form(app: &mut Faraday, z: Zone, t: Thing, k: u8) {
    board(app, BoardPress::Form(z, t, k));
}

/// The map of the plan the board on screen edits, sticks and all: Back
/// up's, or the wallet flow's on the flow's screen.
pub fn backup_map(app: &Faraday) -> Vec<Spot> {
    let (shape, a) = app.board_plan().expect("a board");
    plan::map(&shape, &a)
}

/// How many places the plan the board on screen edits has.
pub fn board_places(app: &Faraday) -> usize {
    app.board_plan().expect("a board").1.places
}

/// The wallet's shape, as the board on screen has it.
pub fn board_shape(app: &Faraday) -> plan::Shape {
    app.board_plan().expect("a board").0
}

/// What spot `at` holds on Back up's map.
pub fn backup_held(app: &Faraday, at: At) -> Vec<What> {
    plan::holds_at(&backup_map(app), at)
}

/// The stick at place `p` that carries vault `v`.
pub fn vault_stick(app: &Faraday, p: usize, v: usize) -> Option<usize> {
    let m = backup_map(app);
    (0..plan::sticks_at(&m, p)).find(|&s| plan::stick_vault(&m, p, s) == Some(v))
}

/// Place `p`'s first stick with no vault: its stick of files.
pub fn files_stick(app: &Faraday, p: usize) -> Option<usize> {
    let m = backup_map(app);
    (0..plan::sticks_at(&m, p)).find(|&s| plan::stick_vault(&m, p, s).is_none())
}

/// Vault `v` is on the plan: **+ vault** pressed until it is, where it is
/// not a vault set up before the keys.
pub fn vault_made(app: &mut Faraday, v: usize) {
    for _ in 0..=v {
        let m = backup_map(app);
        let set_up = board_shape(app).vaults;
        if v < set_up || m.iter().any(|s| s.at == At::Vault(v)) {
            return;
        }
        board(app, BoardPress::AddVault);
    }
}

/// Vault `v` kept at place `p` (its chip dropped on the card, where the
/// place keeps no stick of it), holding exactly `seeds`.
pub fn vault_at(app: &mut Faraday, p: usize, v: usize, seeds: &[usize]) {
    vault_made(app, v);
    if vault_stick(app, p, v).is_none() {
        put(app, Thing::Vault(v), Zone::Card(p));
    }
    let s = vault_stick(app, p, v).expect("the vault's stick at the place");
    let z = Zone::Vault(p, s, v);
    let n = board_shape(app).seeds.len();
    for i in 0..n {
        let held = backup_held(app, At::Vault(v));
        let has = held.contains(&What::Seed(i)) || held.contains(&What::SeedPassphrase(i));
        if has && !seeds.contains(&i) {
            take_off(app, z, Thing::Seed(i));
        } else if !has && seeds.contains(&i) {
            put(app, Thing::Seed(i), z);
        }
    }
}

/// Seeds `seeds` as words files on place `p`'s stick of files, one made
/// on **+ stick** where the place has none.
pub fn files_at(app: &mut Faraday, p: usize, seeds: &[usize]) {
    for &i in seeds {
        match files_stick(app, p) {
            Some(s) => put(app, Thing::Seed(i), Zone::Files(p, s)),
            None => put(app, Thing::Seed(i), Zone::NewStick(p)),
        }
    }
}

/// Every seed's words and SeedQR taken off every place's paper.
pub fn no_words(app: &mut Faraday) {
    let places = board_places(app);
    for p in 0..places {
        for w in backup_held(app, At::Place(p)) {
            if let What::Words(i) | What::SeedQr(i) = w {
                take_off(app, Zone::Paper(p), Thing::Seed(i));
            }
        }
    }
}

/// The description, whole and in shares, taken off every place's paper.
pub fn no_paper_description(app: &mut Faraday) {
    let places = board_places(app);
    for p in 0..places {
        for w in backup_held(app, At::Place(p)) {
            match w {
                What::Sheet => take_off(app, Zone::Paper(p), Thing::Whole),
                What::Share(j) => take_off(app, Zone::Paper(p), Thing::Share(j)),
                _ => {}
            }
        }
    }
}

/// Every vault taken off the plan by the × on its chip in the list, and
/// **Take off** where it holds something.
pub fn no_vaults(app: &mut Faraday) {
    let vaults: Vec<usize> = backup_map(app)
        .iter()
        .filter_map(|s| match s.at {
            At::Vault(v) => Some(v),
            _ => None,
        })
        .collect();
    for v in vaults {
        board(app, BoardPress::RemoveVault(v));
        if backup_map(app).iter().any(|s| s.at == At::Vault(v)) {
            board(app, BoardPress::RemoveVaultSure(v));
        }
    }
}

/// Vault `v` taken off the plan: its × in the list, and **Take off**
/// where it holds something.
pub fn vault_off(app: &mut Faraday, v: usize) {
    if !backup_map(app).iter().any(|s| s.at == At::Vault(v)) {
        return;
    }
    board(app, BoardPress::RemoveVault(v));
    if backup_map(app).iter().any(|s| s.at == At::Vault(v)) {
        board(app, BoardPress::RemoveVaultSure(v));
    }
}

/// Every stick's chip of `t` at every place taken off by its ×.
fn off_sticks(app: &mut Faraday, f: &dyn Fn(What) -> Option<Thing>) {
    let places = board_places(app);
    for p in 0..places {
        let n = plan::sticks_at(&backup_map(app), p);
        // From the last stick back: a stick left empty goes.
        for s in (0..n).rev() {
            for w in backup_held(app, At::Stick(p, s)) {
                if let Some(t) = f(w)
                    && backup_held(app, At::Stick(p, s)).contains(&w)
                {
                    take_off(app, Zone::Files(p, s), t);
                }
            }
        }
    }
}

/// The description's files, whole and shares, taken off every stick.
pub fn no_description_files(app: &mut Faraday) {
    off_sticks(app, &|w| match w {
        What::WalletFile(_) | What::SoftwareFile => Some(Thing::Whole),
        What::ShareFile(j, _) => Some(Thing::Share(j)),
        _ => None,
    });
}

/// Every seed's files taken off the sticks at place `p`.
pub fn no_seed_files_at(app: &mut Faraday, p: usize) {
    let n = plan::sticks_at(&backup_map(app), p);
    for s in (0..n).rev() {
        for w in backup_held(app, At::Stick(p, s)) {
            if let What::SeedFile(i) | What::SeedQrFile(i) = w
                && backup_held(app, At::Stick(p, s)).contains(&w)
            {
                take_off(app, Zone::Files(p, s), Thing::Seed(i));
            }
        }
    }
}

/// The description, whole and its shares, taken out of vault `v` on the
/// board, at a place that keeps its stick.
pub fn description_out_of(app: &mut Faraday, v: usize) {
    let places = board_places(app);
    let Some((p, s)) = (0..places).find_map(|p| vault_stick(app, p, v).map(|s| (p, s))) else {
        return;
    };
    for w in backup_held(app, At::Vault(v)) {
        match w {
            What::Wallet => take_off(app, Zone::Vault(p, s, v), Thing::Whole),
            What::Share(j) => take_off(app, Zone::Vault(p, s, v), Thing::Share(j)),
            _ => {}
        }
    }
}

/// The whole description dropped on place `p`'s stick of files, as its
/// text file (one made on **+ stick** where the place has none).
pub fn description_file_at(app: &mut Faraday, p: usize) {
    match files_stick(app, p) {
        Some(s) => put(app, Thing::Whole, Zone::Files(p, s)),
        None => put(app, Thing::Whole, Zone::NewStick(p)),
    }
}

/// Vault `v`'s stick kept at place `p` too, dropped on its card where
/// the place keeps none.
pub fn stick_also_at(app: &mut Faraday, p: usize, v: usize) {
    if vault_stick(app, p, v).is_none() {
        put(app, Thing::Vault(v), Zone::Card(p));
    }
}

/// Seed `i`'s words and SeedQR taken off place `p`'s paper.
pub fn words_off_at(app: &mut Faraday, p: usize) {
    for w in backup_held(app, At::Place(p)) {
        if let What::Words(i) | What::SeedQr(i) = w
            && backup_held(app, At::Place(p)).contains(&w)
        {
            take_off(app, Zone::Paper(p), Thing::Seed(i));
        }
    }
}

/// Every passphrase taken off every place's paper.
pub fn no_paper_passphrases(app: &mut Faraday) {
    let places = board_places(app);
    for p in 0..places {
        for w in backup_held(app, At::Place(p)) {
            if let What::Passphrase(i) = w {
                take_off(app, Zone::Paper(p), Thing::Pass(i));
            }
        }
    }
}

/// `t` dropped into vault `v` at the first place that keeps its stick.
pub fn into_vault(app: &mut Faraday, v: usize, t: Thing) {
    let places = board_places(app);
    let (p, s) = (0..places)
        .find_map(|p| vault_stick(app, p, v).map(|s| (p, s)))
        .expect("the vault's stick has a place");
    put(app, t, Zone::Vault(p, s, v));
}

/// Seed `i`'s passphrase taken off place `p`'s paper.
pub fn pass_off_at(app: &mut Faraday, p: usize, i: usize) {
    if backup_held(app, At::Place(p)).contains(&What::Passphrase(i)) {
        take_off(app, Zone::Paper(p), Thing::Pass(i));
    }
}

/// What `w` at `at` is on the board: the zone of place `p` it is drawn
/// in and its thing, and its place among that zone's forms.
fn spot_of(app: &Faraday, p: usize, w: What) -> Option<(Zone, Thing, u8)> {
    let m = backup_map(app);
    let files = |i: usize| {
        (0..plan::sticks_at(&m, p))
            .find(|&s| {
                plan::holds_at(&m, At::Stick(p, s))
                    .iter()
                    .any(|x| matches!(x, What::SeedFile(j) | What::SeedQrFile(j) if *j == i))
            })
            .map(|s| Zone::Files(p, s))
    };
    Some(match w {
        What::Words(i) => (Zone::Paper(p), Thing::Seed(i), 0),
        What::SeedQr(i) => (Zone::Paper(p), Thing::Seed(i), 1),
        What::Passphrase(i) => (Zone::Paper(p), Thing::Pass(i), 0),
        What::SeedFile(i) => (files(i)?, Thing::Seed(i), 0),
        _ => return None,
    })
}

/// A row of §17's place cards set as its sheet's Done set it, made on the
/// board of the plan on screen: at place `p`, `row` kept for exactly
/// `seeds` (`docs/BACKUP-FLOW.md` §17.1). For a vault, its stick kept at
/// the place and the vault holding exactly `seeds`.
pub fn keep_row(app: &mut Faraday, p: usize, row: plan::Row, seeds: &[usize]) {
    use plan::Row;
    if let Row::Vault(v) = row {
        return vault_at(app, p, v, seeds);
    }
    let shape = board_shape(app);
    for i in shape.here() {
        let w = match row {
            Row::Words => What::Words(i),
            Row::SeedQr => What::SeedQr(i),
            Row::File => What::SeedFile(i),
            Row::Passphrase if shape.seeds[i].passphrase => What::Passphrase(i),
            _ => continue,
        };
        let has = |app: &Faraday| {
            let m = backup_map(app);
            match w {
                What::SeedFile(_) => (0..plan::sticks_at(&m, p))
                    .any(|s| plan::holds_at(&m, At::Stick(p, s)).contains(&w)),
                _ => plan::holds_at(&m, At::Place(p)).contains(&w),
            }
        };
        let want = seeds.contains(&i);
        if has(app) == want {
            continue;
        }
        if want {
            // On paper, a chip there already takes the form; else a drop.
            let (z, t, k) = match w {
                What::SeedFile(_) => match files_stick(app, p) {
                    Some(s) => (Zone::Files(p, s), Thing::Seed(i), 0),
                    None => (Zone::NewStick(p), Thing::Seed(i), 0),
                },
                _ => spot_of(app, p, w).unwrap(),
            };
            let chip_there = z == Zone::Paper(p)
                && plan::holds_at(&backup_map(app), At::Place(p))
                    .iter()
                    .any(|x| {
                        matches!((x, t),
                        (What::Words(j) | What::SeedQr(j), Thing::Seed(i2)) if *j == i2)
                    });
            if chip_there {
                form(app, z, t, k);
            } else {
                put(app, t, z);
                if k != 0 {
                    // The drop placed the first form: the one asked for
                    // instead.
                    form(app, z, t, k);
                    form(app, z, t, 0);
                }
            }
        } else {
            let (z, t, k) = spot_of(app, p, w).unwrap();
            let forms = plan::holds_at(&backup_map(app), At::Place(p))
                .iter()
                .filter(|x| {
                    matches!((x, t),
                    (What::Words(j) | What::SeedQr(j), Thing::Seed(i2)) if *j == i2)
                })
                .count();
            if z == Zone::Paper(p) && forms > 1 {
                form(app, z, t, k);
            } else {
                take_off(app, z, t);
            }
        }
    }
}
