//! A tool or test-support crate: the tour root's, the journeys' and the leak crate's adapters:
//! fixture sticks, fixed randomness, scripted clock and input, recorded print (09 §8).
//!
//! `Fixtures` is one `Adapters` value for the three products (02 §10, the tour-root column). A
//! stick is a folder of the test kit, read once when the fixtures are made; what a journey
//! writes to a stick stays in memory and is listed by `written`, so the kit is never changed.

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::Path;

use faraday_crypto::Secret;
use faraday_port::ask::{Answer, Failure};
use faraday_port::camera;
use faraday_port::event::{Abilities, Input, Key, Machine, SecureBootAtStart, SticksFact, Time};
use faraday_port::event::{Event, Start, TouchPhase};
use faraday_port::host::HostAnswer;
use faraday_port::storage::{
    Carry, OnTaken, Room, Sha256, StickId, StickInfo, StorageAnswer, StorageAsk, Unrenamed, Wiped,
    Written, Zeroing,
};
use faraday_port::system::{Reason, SystemAnswer, SystemAsk, Version};
use faraday_port::values::{Display, Medium, Orientation, Point};
use faraday_port::{Adapters, Ask, Effect, Exit, Order};
use faraday_text::Text;

/// The product the fixtures stand in for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Product {
    Device,
    Demo,
    Starter,
}

/// One fixture stick: a folder of the kit, loaded.
struct Stick {
    id: StickId,
    files: BTreeMap<String, Vec<u8>>,
    attached: bool,
}

/// Sticks from `out/testkit`, randomness, a clock and an input script.
pub struct Fixtures {
    product: Product,
    sticks: Vec<Stick>,
    now_ms: u64,
    unix_secs: u64,
    started: bool,
    events: VecDeque<Event>,
    script: VecDeque<Scripted>,
    written: Vec<(StickId, Text, Sha256)>,
    orders: Vec<Order>,
    printed: Vec<(Text, bool)>,
    kept: Option<Carry>,
    settings: Option<Text>,
    ended: bool,
    size: Option<(u16, u16, u16)>,
}

impl Fixtures {
    #[must_use]
    pub fn device(kit: &Path) -> Self {
        Fixtures::new(Product::Device, kit)
    }

    #[must_use]
    pub fn demo(kit: &Path) -> Self {
        Fixtures::new(Product::Demo, kit)
    }

    #[must_use]
    pub fn starter(kit: &Path) -> Self {
        Fixtures::new(Product::Starter, kit)
    }

    /// Every folder of `kit` is a stick, attached at start; a `kit` that is not there gives
    /// none.
    fn new(product: Product, kit: &Path) -> Self {
        let mut sticks = Vec::new();
        if let Ok(folders) = fs::read_dir(kit) {
            for folder in folders.flatten() {
                if !folder.path().is_dir() {
                    continue;
                }
                let mut files = BTreeMap::new();
                if let Ok(entries) = fs::read_dir(folder.path()) {
                    for entry in entries.flatten() {
                        if let (Some(name), Ok(bytes)) =
                            (entry.file_name().to_str(), fs::read(entry.path()))
                        {
                            files.insert(String::from(name), bytes);
                        }
                    }
                }
                if let Some(name) = folder.file_name().to_str() {
                    sticks.push(Stick {
                        id: StickId(Text::of(name)),
                        files,
                        attached: true,
                    });
                }
            }
        }
        sticks.sort_by(|a, b| a.id.0.as_str().cmp(b.id.0.as_str()));
        Fixtures {
            product,
            sticks,
            now_ms: 0,
            unix_secs: 1_790_000_000,
            started: false,
            events: VecDeque::new(),
            script: VecDeque::new(),
            written: Vec::new(),
            orders: Vec::new(),
            printed: Vec::new(),
            kept: None,
            settings: None,
            ended: false,
            size: None,
        }
    }

    /// The display the run starts on: its width and height in pixels and its dpi, in place of the
    /// product's own. Set before the first `poll`.
    pub fn display_of(&mut self, width: u16, height: u16, dpi: u16) {
        self.size = Some((width, height, dpi));
    }

    /// Queues a script of input, in order.
    pub fn input(&mut self, script: &[Scripted]) {
        self.script.extend(script.iter().cloned());
    }

    /// Moves the scripted clock on by `ms`; the next `poll` gives the `Time` event.
    pub fn advance(&mut self, ms: u64) {
        self.now_ms += ms;
        self.unix_secs += ms / 1000;
        self.events.push_back(Event::Time(self.time()));
    }

    /// What a journey asserts: every file written, by stick, name and hash.
    #[must_use]
    pub fn written(&self) -> &[(StickId, Text, Sha256)] {
        &self.written
    }

    /// Every order carried, in order.
    #[must_use]
    pub fn orders(&self) -> &[Order] {
        &self.orders
    }

    /// Every print asked: the name and whether it holds a secret (print is recorded, not done).
    #[must_use]
    pub fn printed(&self) -> &[(Text, bool)] {
        &self.printed
    }

    /// What the last `Ask::Keep` carried, which the next start gives back as `carried`.
    #[must_use]
    pub fn kept(&self) -> Option<&Carry> {
        self.kept.as_ref()
    }

    /// Whether `Exit::End` was carried: the run is over.
    #[must_use]
    pub fn ended(&self) -> bool {
        self.ended
    }

    fn time(&self) -> Time {
        Time {
            now_ms: self.now_ms,
            unix_secs: self.unix_secs,
        }
    }

    fn display(&self) -> Display {
        let (width, height) = if self.product == Product::Device {
            (1024, 600)
        } else {
            (1280, 800)
        };
        let (width, height, dpi) = self.size.unwrap_or((width, height, 96));
        Display {
            width,
            height,
            dpi,
            orientation: Orientation::Landscape,
            rotation_offered: None,
            external: false,
            preview: None,
        }
    }

    fn start(&mut self) -> Event {
        let device = self.product == Product::Device;
        let starter = self.product == Product::Starter;
        let abilities = Abilities {
            clipboard: !device,
            print: !device,
            host_files: !device,
            opens: !device,
            camera: false,
            names_sticks: device,
            holds_back_input: device,
            restart_computer: device,
            moves_window: !device,
            system: starter,
            lock_ends_process: device,
        };
        self.listed();
        Event::Start(Start {
            display: self.display(),
            medium: if device {
                Medium::Stick
            } else {
                Medium::Desktop
            },
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
            time: self.time(),
            carried: self.kept.take(),
            settings: self.settings.clone(),
            scheme: None,
            root_read_only: device.then_some(true),
        })
    }

    /// Queues the facts of the sticks attached now.
    fn listed(&mut self) {
        let attached: Vec<StickInfo> = self
            .sticks
            .iter()
            .filter(|s| s.attached)
            .map(Stick::info)
            .collect();
        self.events
            .push_back(Event::Sticks(SticksFact::Attached(!attached.is_empty())));
        self.events
            .push_back(Event::Sticks(SticksFact::Listed(attached)));
    }

    fn stick(&mut self, id: &StickId) -> Option<&mut Stick> {
        self.sticks.iter_mut().find(|s| s.id == *id && s.attached)
    }

    /// The next event the script gives, if any.
    fn scripted(&mut self) -> Option<Event> {
        while let Some(step) = self.script.pop_front() {
            match step {
                Scripted::Touch(at) => {
                    self.events.push_back(Event::Input(Input::Touch {
                        at,
                        phase: TouchPhase::Up,
                    }));
                    return Some(Event::Input(Input::Touch {
                        at,
                        phase: TouchPhase::Down,
                    }));
                }
                Scripted::Key(key) => return Some(Event::Input(Input::Key(key))),
                Scripted::Text(text) => {
                    let mut chars = text.as_str().chars();
                    let Some(first) = chars.next() else {
                        continue;
                    };
                    for c in chars {
                        self.events
                            .push_back(Event::Input(Input::Key(Key::Char(c))));
                    }
                    return Some(Event::Input(Input::Key(Key::Char(first))));
                }
                Scripted::Stick(id, attached) => {
                    if let Some(stick) = self.sticks.iter_mut().find(|s| s.id == id) {
                        stick.attached = attached;
                        self.listed();
                        return self.events.pop_front();
                    }
                }
                Scripted::Wait(ms) => {
                    self.now_ms += ms;
                    self.unix_secs += ms / 1000;
                    return Some(Event::Time(self.time()));
                }
            }
        }
        None
    }

    /// 02 §10's tour-root column, one ask at a time.
    fn answer(&mut self, ask: Ask) -> Answer {
        let device = self.product == Product::Device;
        match ask {
            Ask::Storage(ask) => Answer::Storage(self.storage(ask)),
            Ask::Keep(carry) => {
                self.kept = Some(carry);
                Answer::Kept(Ok(()))
            }
            Ask::Forget => {
                self.kept = None;
                Answer::Forgotten(Ok(()))
            }
            Ask::Host(_) => Answer::Host(HostAnswer::Opened(Err(Failure::Unsupported))),
            Ask::Camera(_) => Answer::Camera(Err(camera::Failure::Unavailable)),
            Ask::Clipboard => Answer::Clipboard(Err(if device {
                Failure::Unsupported
            } else {
                Failure::Empty
            })),
            Ask::Print(job) => {
                self.printed.push((job.name.clone(), job.holds_secret));
                Answer::Printed(Ok(job.name))
            }
            // Fixed randomness waits for the port's `EntropyBytes`, which has no body yet.
            Ask::Entropy => Answer::Entropy(Err(Failure::Unavailable)),
            Ask::Memory => Answer::Memory(Some(1024)),
            Ask::Settings(text) => {
                if self.product == Product::Starter {
                    self.settings = Some(text);
                    Answer::Settings(Ok(()))
                } else {
                    Answer::Settings(Err(Failure::Unsupported))
                }
            }
            Ask::System(ask) => Answer::System(system(ask)),
        }
    }

    fn storage(&mut self, ask: StorageAsk) -> StorageAnswer {
        match ask {
            StorageAsk::Read { stick, name } => StorageAnswer::Read(
                self.stick(&stick)
                    .and_then(|s| s.files.get(name.as_str()))
                    .map(|bytes| Secret::new(bytes.clone()))
                    .ok_or(Failure::Gone),
            ),
            StorageAsk::ReadCodes { .. } => StorageAnswer::Codes(Err(Failure::Unsupported)),
            StorageAsk::Write {
                stick,
                name,
                bytes,
                replace,
                on_taken,
                ..
            } => {
                let sha = Sha256(faraday_crypto::sha256(bytes.expose()));
                let Some(found) = self.stick(&stick) else {
                    return StorageAnswer::Written(Err(Failure::Gone));
                };
                let taken = found.files.get(name.as_str()).cloned();
                let as_name = match (taken, replace, on_taken) {
                    (Some(old), false, OnTaken::Ask) => {
                        return StorageAnswer::Written(Ok(Written::NameTaken {
                            on_stick: name,
                            sha: Sha256(faraday_crypto::sha256(&old)),
                        }));
                    }
                    (Some(_), false, OnTaken::Beside) => Text::of(&beside(name.as_str())),
                    _ => name,
                };
                found
                    .files
                    .insert(String::from(as_name.as_str()), bytes.expose().clone());
                self.written.push((stick, as_name.clone(), sha));
                StorageAnswer::Written(Ok(Written::Wrote { wrote_as: as_name }))
            }
            StorageAsk::Wipe { stick, name } => StorageAnswer::Wiped(
                self.stick(&stick)
                    .and_then(|s| s.files.remove(name.as_str()))
                    .map(|_| Wiped::Zeroed)
                    .ok_or(Failure::Gone),
            ),
            StorageAsk::Space { stick } => StorageAnswer::Space(
                self.stick(&stick)
                    .map(|_| Room {
                        free: 64 << 20,
                        size: 128 << 20,
                    })
                    .ok_or(Failure::Gone),
            ),
            StorageAsk::ZeroFree { stick, .. } => StorageAnswer::Zeroing(
                self.stick(&stick)
                    .map(|_| Zeroing {
                        next: 0,
                        total: 1,
                        done: true,
                    })
                    .ok_or(Failure::Gone),
            ),
            StorageAsk::MakeEmpty { stick } => StorageAnswer::Emptied(
                self.stick(&stick)
                    .map(|s| s.files.clear())
                    .ok_or(Failure::Gone),
            ),
            StorageAsk::Rename {
                stick, from, to, ..
            } => StorageAnswer::Renamed(match self.stick(&stick) {
                None => Err(Unrenamed::Failed(Failure::Gone)),
                Some(s) if s.files.contains_key(to.as_str()) => Err(Unrenamed::Taken),
                Some(s) => match s.files.remove(from.as_str()) {
                    Some(bytes) => {
                        s.files.insert(String::from(to.as_str()), bytes);
                        Ok(())
                    }
                    None => Err(Unrenamed::Failed(Failure::Gone)),
                },
            }),
            StorageAsk::Name { stick, .. } => StorageAnswer::Named(
                self.stick(&stick)
                    .map(|_| ())
                    .ok_or(faraday_port::storage::NameRefused::Failed(Failure::Gone)),
            ),
            StorageAsk::Hash { stick, name } => StorageAnswer::Hashed(
                self.stick(&stick)
                    .and_then(|s| s.files.get(name.as_str()))
                    .map(|bytes| Sha256(faraday_crypto::sha256(bytes)))
                    .ok_or(Failure::Gone),
            ),
        }
    }
}

impl Stick {
    fn info(&self) -> StickInfo {
        StickInfo {
            id: self.id.clone(),
            label: self.id.0.clone(),
            boot: false,
            serial: None,
            files: self
                .files
                .iter()
                .map(|(name, bytes)| (Text::of(name), bytes.len() as u64))
                .collect(),
            vaults: self
                .files
                .keys()
                .filter(|name| name.ends_with(".kdbx"))
                .map(|name| Text::of(name))
                .collect(),
            make: faraday_port::storage::StickMake::default(),
        }
    }
}

/// `name` with a number before its extension, as a write beside a taken name is made.
fn beside(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem} 2.{ext}"),
        None => format!("{name} 2"),
    }
}

/// Every system ask is answered as not carried: the fixtures have no operating system to act on.
fn system(ask: SystemAsk) -> SystemAnswer {
    let no = || Reason::NotCarried;
    match ask {
        SystemAsk::WriteStick(_) => SystemAnswer::WriteStick(Err(no())),
        SystemAsk::Install => SystemAnswer::Install(Err(no())),
        SystemAsk::Uninstall => SystemAnswer::Uninstall(Err(no())),
        SystemAsk::RemoveBootEntry => SystemAnswer::RemoveBootEntry(Err(no())),
        SystemAsk::RestartIntoFirmware => SystemAnswer::RestartIntoFirmware(Err(no())),
        SystemAsk::RestartFromStick { .. } => SystemAnswer::RestartFromStick(Err(no())),
        SystemAsk::ReadBitLocker => SystemAnswer::ReadBitLocker(None),
        SystemAsk::BitLocker(step) => SystemAnswer::BitLocker {
            step,
            result: Err(no()),
        },
        SystemAsk::ReadRecoveryKey => SystemAnswer::RecoveryKey(Err(no())),
        SystemAsk::ReadVersion {
            disk, serial, size, ..
        } => SystemAnswer::VersionRead {
            disk,
            serial,
            size,
            version: Version {
                result: Err(no()),
                signed: None,
            },
        },
        SystemAsk::ListBootFiles => SystemAnswer::BootFiles {
            folder: Text::of(""),
            files: Vec::new(),
        },
        SystemAsk::JudgeBoot {
            disk,
            serial,
            size,
            sha256,
            want,
            ..
        } => SystemAnswer::BootJudged {
            disk,
            serial,
            size,
            sha256,
            want,
            signed: None,
        },
        SystemAsk::CheckBootFile { path, .. } => SystemAnswer::BootFileChecked {
            path,
            result: Err(no()),
        },
        SystemAsk::FindImages { .. } => SystemAnswer::ImagesFound(Vec::new()),
        SystemAsk::CheckAgain => SystemAnswer::CheckedAgain,
        SystemAsk::PrepareDemoSticks => SystemAnswer::PrepareDemoSticks(Err(no())),
    }
}

impl Adapters for Fixtures {
    /// 02 §10's tour-root column: an ask is answered at once, the answer coming back through
    /// `poll`; an order is recorded and acted on where it changes what the fixtures report.
    fn carry(&mut self, effect: Effect) {
        match effect {
            Effect::Ask(id, ask) => {
                let answer = self.answer(ask);
                self.events.push_back(Event::Answer(id, answer));
            }
            Effect::Order(order) => {
                match &order {
                    Order::ToggleStick(id) => {
                        if let Some(stick) = self.sticks.iter_mut().find(|s| s.id == *id) {
                            stick.attached = !stick.attached;
                        }
                        self.listed();
                    }
                    Order::Exit(Exit::End) => self.ended = true,
                    _ => {}
                }
                self.orders.push(order);
            }
        }
    }

    /// `Start` first and once, then what an answer or a clock step queued, then the script.
    fn poll(&mut self) -> Option<Event> {
        if !self.started {
            self.started = true;
            return Some(self.start());
        }
        if let Some(event) = self.events.pop_front() {
            return Some(event);
        }
        self.scripted()
    }
}

/// One step of an input script.
#[derive(Clone)]
pub enum Scripted {
    /// A press and its release at the point.
    Touch(Point),
    Key(Key),
    /// Typed, one key a character.
    Text(Text),
    /// A stick plugged in or pulled out.
    Stick(StickId, bool),
    /// The clock moves on by this many milliseconds.
    Wait(u64),
}
