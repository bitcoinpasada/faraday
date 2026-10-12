//! The hub app (`docs/HUB.md`): its state, screens and decisions, with no
//! I/O. It is an [`osk_shell_api::App`], as `faraday-core`'s `Faraday`
//! is: the shell sends input and display events through [`App::event`],
//! reads the frame, and takes the drawing commands from
//! [`App::poll_command`]. What upstream's interface has no event for
//! goes through a second channel beside it, as `Faraday`'s storage does:
//! the shell reports what it reads through [`Hub::hub_event`] and takes
//! what the hub asks for from [`Hub::poll_hub`] ([`api`]).
//!
//! Transfer and Learn are the desktop app's own screens (§2): the hub
//! holds a `Faraday` in online mode ([`Faraday::held`]) and draws it
//! while one of them is open; its way back returns to the hub's Home.
//! The shell serves that `Faraday`'s storage channel through
//! [`Hub::held`].

pub mod api;
pub mod decide;
mod draw;

/// What a [`api::Reason`] reads as on screen, from `strings/en.rs`.
pub fn reason_text(reason: &api::Reason) -> String {
    draw::reason_text(reason)
}

use std::collections::VecDeque;

use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, Command, DisplayInfo, Event, Frame, Key, TouchPhase};
use osk_ui::{Canvas, Rect};

use api::{
    Computer, DevImage, Disk, HubCommand, HubEvent, Installed, Mode, Os, Payload, PayloadFile,
    PayloadKind, Reason, Source, WriteOrder, WriteStage,
};
use decide::{Facts, Item, Target};

/// The website's guides (§7.4): the only address the hub app contains.
pub const GUIDES: &str = "https://faradayoffline.app/start/";

/// The hub app's own version, which About shows and Install compares.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A page of the hub app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// Home: This computer, the checklist, the rows (§3.1).
    Home,
    /// Which stick? or Which card? (§3.2 step 1, §3.3).
    Disks,
    /// What to write? (§3.2 step 2).
    Mode,
    /// Confirm (§3.2 step 3).
    Confirm,
    /// Writing (§3.2 step 4).
    Writing,
    /// Done, or the writer's reason (§3.2 step 5).
    Done,
    /// Start this computer from the stick (§3.4).
    Start,
    /// Install (§3.5).
    Install,
    /// Installed (§3.5).
    Installed,
    /// About (§3.7).
    About,
    /// BitLocker's state. The guard's own page (§6.5) is batch W's.
    BitLocker,
}

/// A sheet over a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    /// A restart, asked once (§3.1, §3.4 step 2, §7.3).
    Restart(RestartTo),
    /// Uninstall, asked once (§3.5).
    Uninstall,
}

/// Where a restart goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartTo {
    /// The firmware's settings (§7.3).
    Firmware,
    /// The attached Faraday stick (§3.1).
    Stick,
}

/// The This computer card's button (§3.1): the next action for this
/// computer's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    /// Make a Faraday stick: opens §3.2.
    Make,
    /// Do the BitLocker step: opens §6.5.
    BitLocker,
    /// Resume BitLocker, while the hub app's suspension stands: opens
    /// §6.3's page when Secure Boot is as the marker recorded it.
    Resume {
        /// It can be taken now.
        offered: bool,
    },
    /// Restart into firmware settings, after the sheet.
    Firmware,
    /// Restart from the stick, after the sheet.
    Stick,
}

/// A guide, by its Learn page (§7.2, `docs/learn/faraday/boot/`). A page
/// that does not exist shows its title as a line and opens nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnId {
    /// Starting a Windows computer from the stick.
    StartWindows,
    /// Starting a Linux computer from the stick.
    StartLinux,
    /// BitLocker and starting from a stick.
    BitLocker,
    /// When the stick does not start.
    NoStart,
    /// Replacing Faraday on a stick.
    KeepData,
    /// A maker family's page, by its id in `faraday_core::learn::MAKERS`
    /// (§7.6).
    Maker(&'static str),
}

impl LearnId {
    /// The page's id among Faraday's own (`faraday_core::learn::BOOT`),
    /// which is also its name on the website.
    pub fn id(self) -> &'static str {
        match self {
            LearnId::StartWindows => "windows",
            LearnId::StartLinux => "linux",
            LearnId::BitLocker => "bitlocker",
            LearnId::NoStart => "not-starting",
            LearnId::KeepData => "replace-faraday",
            LearnId::Maker(id) => id,
        }
    }

    /// The page.
    pub fn page(self) -> Option<&'static osk_learn::Page> {
        faraday_core::learn::page(self.id())
    }

    /// Its address on the website (§7.4).
    pub fn address(self) -> String {
        format!("{GUIDES}{}", self.id())
    }

    /// The guide for this computer (§3.4): its maker's page where there
    /// is one (§7.6), and otherwise the general page for its operating
    /// system.
    pub fn for_computer(c: &Computer) -> LearnId {
        if let Some(m) = faraday_core::learn::maker(&c.maker, &c.family, &c.model) {
            return LearnId::Maker(m.id);
        }
        match c.os {
            Os::Windows => LearnId::StartWindows,
            Os::Linux => LearnId::StartLinux,
        }
    }

    const ALL: [LearnId; 5] = [
        LearnId::StartWindows,
        LearnId::StartLinux,
        LearnId::BitLocker,
        LearnId::NoStart,
        LearnId::KeepData,
    ];
}

/// Everything a press can do on the hub's own pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubAction {
    /// Back one step.
    Back,
    /// Home.
    Home,
    /// Make a Faraday stick, or a Pi card: the disk list.
    Make(Target),
    /// The This computer card's button ([`Card`]).
    Next,
    /// A checklist item.
    Item(Item),
    /// A disk of the list, by the number the hub gave it when it was
    /// first seen (it keeps it while the list changes around it).
    Disk(u16),
    /// What to write, ticked.
    Mode(Mode),
    /// Continue past What to write.
    Continue,
    /// Confirm's action: Erase and write, or Write.
    Write,
    /// Make another, or try again: the disk list.
    Again,
    /// Start this computer from the stick.
    Start,
    /// The Start page's Restart into firmware settings: asks first.
    Restart,
    /// The restart the sheet asks about, now.
    RestartNow,
    /// Close the sheet.
    Cancel,
    /// Install (or update the installed copy).
    Install,
    /// The desktop app check box.
    InstallDesktop,
    /// Install now.
    InstallNow,
    /// Installed: what and where.
    Installed,
    /// Uninstall: asks first.
    Uninstall,
    /// Uninstall now.
    UninstallNow,
    /// Try the desktop app.
    TryDesktop,
    /// Transfer, the desktop app's own.
    Transfer,
    /// Learn, the desktop app's sheet with the hub's pages.
    Learn,
    /// A guide's Learn page.
    LearnPage(LearnId),
    /// About.
    About,
    /// BitLocker's page.
    BitLocker,
}

/// A maker page's number in an action's code: this plus its place in
/// `faraday_core::learn::MAKERS`.
const MAKER_CODE: u32 = 0x100;

const ITEMS: [Item; 5] = [
    Item::MakeStick,
    Item::BitLocker,
    Item::Start,
    Item::SecureBootOn,
    Item::Resume,
];

impl HubAction {
    /// The number `faraday_core`'s [`Action::Host`] carries for it.
    pub fn code(self) -> u32 {
        let (kind, arg): (u32, u32) = match self {
            HubAction::Back => (0, 0),
            HubAction::Home => (1, 0),
            HubAction::Make(t) => (2, u32::from(t == Target::Pi)),
            HubAction::Next => (3, 0),
            HubAction::Item(i) => (4, ITEMS.iter().position(|x| *x == i).unwrap_or(0) as u32),
            HubAction::Disk(n) => (5, u32::from(n)),
            HubAction::Mode(m) => (6, u32::from(m == Mode::KeepData)),
            HubAction::Continue => (7, 0),
            HubAction::Write => (8, 0),
            HubAction::Again => (9, 0),
            HubAction::Start => (10, 0),
            HubAction::Restart => (11, 0),
            HubAction::RestartNow => (12, 0),
            HubAction::Cancel => (13, 0),
            HubAction::Install => (14, 0),
            HubAction::InstallDesktop => (15, 0),
            HubAction::InstallNow => (16, 0),
            HubAction::Installed => (17, 0),
            HubAction::Uninstall => (18, 0),
            HubAction::UninstallNow => (19, 0),
            HubAction::TryDesktop => (20, 0),
            HubAction::Transfer => (21, 0),
            HubAction::Learn => (22, 0),
            HubAction::LearnPage(LearnId::Maker(id)) => (
                23,
                MAKER_CODE
                    + faraday_core::learn::MAKERS
                        .iter()
                        .position(|m| m.id == id)
                        .unwrap_or(0) as u32,
            ),
            HubAction::LearnPage(id) => (
                23,
                LearnId::ALL.iter().position(|x| *x == id).unwrap_or(0) as u32,
            ),
            HubAction::About => (24, 0),
            HubAction::BitLocker => (25, 0),
        };
        (kind << 16) | arg
    }

    /// The action a number stands for.
    pub fn from_code(code: u32) -> Option<HubAction> {
        let arg = code & 0xffff;
        Some(match code >> 16 {
            0 => HubAction::Back,
            1 => HubAction::Home,
            2 => HubAction::Make(if arg == 1 { Target::Pi } else { Target::Stick }),
            3 => HubAction::Next,
            4 => HubAction::Item(*ITEMS.get(arg as usize)?),
            5 => HubAction::Disk(arg as u16),
            6 => HubAction::Mode(if arg == 1 {
                Mode::KeepData
            } else {
                Mode::Whole
            }),
            7 => HubAction::Continue,
            8 => HubAction::Write,
            9 => HubAction::Again,
            10 => HubAction::Start,
            11 => HubAction::Restart,
            12 => HubAction::RestartNow,
            13 => HubAction::Cancel,
            14 => HubAction::Install,
            15 => HubAction::InstallDesktop,
            16 => HubAction::InstallNow,
            17 => HubAction::Installed,
            18 => HubAction::Uninstall,
            19 => HubAction::UninstallNow,
            20 => HubAction::TryDesktop,
            21 => HubAction::Transfer,
            22 => HubAction::Learn,
            23 if arg >= MAKER_CODE => HubAction::LearnPage(LearnId::Maker(
                faraday_core::learn::MAKERS
                    .get((arg - MAKER_CODE) as usize)?
                    .id,
            )),
            23 => HubAction::LearnPage(*LearnId::ALL.get(arg as usize)?),
            24 => HubAction::About,
            25 => HubAction::BitLocker,
            _ => return None,
        })
    }

    /// As a `faraday_core` action, for drawing with its `Ui`.
    pub fn host(self) -> Action {
        Action::Host(self.code())
    }
}

/// A stick or card being made: what was tapped and chosen, and the write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Making {
    /// A stick or a Pi card.
    pub target: Target,
    /// The disk tapped, by its id. Nothing until a person taps one.
    pub disk: Option<String>,
    /// What to write, once ticked.
    pub mode: Option<Mode>,
    /// The disk as Confirm showed it: id, device, serial, size.
    pub confirmed: Option<(String, String, String, u64)>,
    /// Why Confirm did not go on.
    pub refused: Option<String>,
    /// The write's progress: stage, done, total.
    pub progress: Option<(WriteStage, u64, u64)>,
    /// How the write ended.
    pub result: Option<Result<(), Reason>>,
}

/// What a stick or card is written from: a payload file, or a developer's
/// image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Where the bytes come from.
    pub source: Source,
    /// The build ID it carries, when known.
    pub build: Option<String>,
    /// The SHA-256 of what is written, hex.
    pub sha256: String,
    /// The bytes written for a whole stick.
    pub size: u64,
    /// A developer's image, not the hub app's own.
    pub dev: bool,
    /// A boot partition alone: keep data only.
    pub boot_only: bool,
}

/// The hub app.
pub struct Hub {
    canvas: Option<Canvas>,
    f: f32,
    w: f32,
    h: f32,
    compact: bool,
    hits: Vec<(Rect, Action)>,
    pressed: Option<Action>,
    hovered: Option<Action>,
    hover: Option<(i32, i32)>,
    commands: VecDeque<Command>,
    out: VecDeque<HubCommand>,
    dirty: bool,

    /// This computer.
    pub computer: Computer,
    /// BitLocker, on Windows.
    pub bitlocker: Option<api::BitLocker>,
    /// The disks attached.
    pub disks: Vec<Disk>,
    /// The payload, when the hub app carries one.
    pub payload: Option<Payload>,
    /// A developer's image, in place of the payload's stick image.
    pub dev: Option<DevImage>,
    /// What is installed.
    pub installed: Option<Installed>,
    /// This computer cannot write a disk from here.
    pub no_writer: bool,

    /// The page shown.
    pub page: Page,
    /// The sheet over it.
    pub sheet: Option<Sheet>,
    /// The stick or card being made.
    pub making: Option<Making>,
    /// Install's check box.
    pub install_desktop: bool,
    /// An install under way.
    pub installing: bool,
    /// How the last install ended.
    pub install_result: Option<Result<String, Reason>>,
    /// Why the last uninstall failed.
    pub uninstall_failed: Option<Reason>,
    /// Why the desktop app did not start.
    pub desktop_failed: Option<Reason>,
    /// Why the restart into firmware settings did not happen.
    pub restart_failed: Option<Reason>,
    /// Why the restart from the stick did not happen: the card offers
    /// the restart into firmware settings instead (§3.1).
    pub stick_restart_failed: Option<Reason>,
    /// Why uninstall left the firmware's `Faraday stick` entry.
    pub entry_kept: Option<Reason>,
    /// Protection was resumed through the hub app this session.
    pub resumed: bool,

    /// How far the page is scrolled, units.
    offset: f32,
    /// How far it can scroll, as last drawn.
    max: f32,
    /// Every disk id seen, so a row keeps its number as the list changes.
    seen: Vec<String>,

    /// The desktop app's Transfer and Learn.
    held: Faraday,
    /// The held app is on screen.
    showing: bool,
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

impl Hub {
    /// A hub with nothing reported yet.
    pub fn new() -> Self {
        let mut held = Faraday::new();
        held.online = true;
        held.held = true;
        Hub {
            canvas: None,
            f: 1.0,
            w: 0.0,
            h: 0.0,
            compact: false,
            hits: Vec::new(),
            pressed: None,
            hovered: None,
            hover: None,
            commands: VecDeque::new(),
            out: VecDeque::new(),
            dirty: true,
            computer: Computer::default(),
            bitlocker: None,
            disks: Vec::new(),
            payload: None,
            dev: None,
            installed: None,
            no_writer: false,
            page: Page::Home,
            sheet: None,
            making: None,
            install_desktop: false,
            installing: false,
            install_result: None,
            uninstall_failed: None,
            desktop_failed: None,
            restart_failed: None,
            stick_restart_failed: None,
            entry_kept: None,
            resumed: false,
            offset: 0.0,
            max: 0.0,
            seen: Vec::new(),
            held,
            showing: false,
        }
    }

    /// The facts Home's checklist is read from.
    pub fn facts(&self) -> Facts<'_> {
        Facts {
            computer: &self.computer,
            bitlocker: self.bitlocker.as_ref(),
            disks: &self.disks,
            resumed: self.resumed,
        }
    }

    /// The This computer card's button (§3.1), the first that applies:
    /// while the hub app's BitLocker suspension stands, its next step
    /// (Secure Boot back on through the firmware, then Resume); then
    /// Make a Faraday stick with no stick attached, or on a computer
    /// that cannot start one; then the BitLocker step on Windows; then
    /// Restart from the stick with Secure Boot off, and Restart into
    /// firmware settings with it on, unknown, or when the restart from
    /// the stick did not happen.
    pub fn card(&self) -> Card {
        let f = self.facts();
        if f.suspension_stands() {
            match f.next_step() {
                Some(Item::SecureBootOn) => return Card::Firmware,
                Some(Item::Resume) => {
                    return Card::Resume {
                        offered: f.resume_offered(),
                    };
                }
                _ => {}
            }
        }
        if f.stick().is_none() || !f.can_start() || !self.computer.uefi {
            return Card::Make;
        }
        if f.protected_open() {
            return Card::BitLocker;
        }
        if self.computer.secure_boot == Some(false) && self.stick_restart_failed.is_none() {
            Card::Stick
        } else {
            Card::Firmware
        }
    }

    /// The held desktop app, for its storage channel
    /// (`Faraday::storage`, `Faraday::poll_storage`): Transfer's
    /// Downloads folder and the files it saves.
    pub fn held(&mut self) -> &mut Faraday {
        &mut self.held
    }

    /// Whether the held desktop app (Transfer or Learn) is on screen.
    pub fn showing_held(&self) -> bool {
        self.showing
    }

    /// The next thing the hub asks the shell to do.
    pub fn poll_hub(&mut self) -> Option<HubCommand> {
        self.out.pop_front()
    }

    /// The payload file of `kind`, when the hub carries one.
    pub fn payload_file(&self, kind: PayloadKind) -> Option<&PayloadFile> {
        self.payload
            .as_ref()?
            .files
            .iter()
            .find(|f| f.kind() == Some(kind))
    }

    /// What a stick or card is written from: for a stick, a developer's
    /// image when one was given, else the payload's.
    pub fn image(&self, target: Target) -> Option<Image> {
        if target == Target::Stick
            && let Some(d) = &self.dev
        {
            return Some(Image {
                source: Source::Image {
                    path: d.path.clone(),
                    sha256: d.sha256.clone(),
                },
                build: d.build_id.clone(),
                sha256: d.sha256.clone(),
                size: d.size,
                dev: true,
                boot_only: d.boot_only,
            });
        }
        let kind = match target {
            Target::Stick => PayloadKind::Stick,
            Target::Pi => PayloadKind::Pi,
        };
        let f = self.payload_file(kind)?;
        Some(Image {
            source: Source::Payload {
                name: f.name.clone(),
                inner_sha256: f.inner_sha256.clone(),
            },
            build: Some(f.build_id.clone()),
            sha256: f.inner_sha256.clone(),
            size: f.inner_size,
            dev: false,
            boot_only: false,
        })
    }

    /// The bytes a disk must hold for `target`: none for a boot
    /// partition alone, which goes onto a Faraday stick's own.
    fn needs(&self, target: Target) -> Option<u64> {
        self.image(target).filter(|i| !i.boot_only).map(|i| i.size)
    }

    /// The disks listed for what is being made, in order.
    pub fn offers(&self) -> Vec<decide::Offer<'_>> {
        let target = self.making.as_ref().map_or(Target::Stick, |m| m.target);
        decide::offers(&self.disks, target, self.needs(target))
    }

    /// The number a disk's row carries.
    pub fn disk_number(&self, id: &str) -> u16 {
        self.seen.iter().position(|s| s == id).unwrap_or(0) as u16
    }

    /// The disk tapped, as attached now.
    pub fn chosen_disk(&self) -> Option<&Disk> {
        let id = self.making.as_ref()?.disk.as_deref()?;
        self.disks.iter().find(|d| d.id == id)
    }

    /// Whether `mode` can be chosen for the disk tapped, and why not.
    pub fn mode_refusal(&self, mode: Mode) -> Option<ModeRefusal> {
        let m = self.making.as_ref()?;
        let image = self.image(m.target);
        let disk = self.chosen_disk();
        match mode {
            Mode::Whole if image.as_ref().is_some_and(|i| i.boot_only) => {
                Some(ModeRefusal::BootOnly)
            }
            Mode::Whole => None,
            // Keep data follows when the Pi has its upgrade (§9.5).
            Mode::KeepData if m.target == Target::Pi => Some(ModeRefusal::NotYet),
            Mode::KeepData => match disk.and_then(|d| d.faraday.as_ref()) {
                None => Some(ModeRefusal::NotFaraday),
                Some(stick) => {
                    let build = image.and_then(|i| i.build);
                    (decide::keep_data(stick, build.as_deref()) == faraday_core::upgrade::Fit::Same)
                        .then_some(ModeRefusal::Same)
                }
            },
        }
    }

    /// Whether the disk Confirm shows is still the one confirmed: the
    /// same id, serial and size.
    pub fn confirmed_matches(&self) -> bool {
        let Some((id, _, serial, size)) = self.making.as_ref().and_then(|m| m.confirmed.as_ref())
        else {
            return false;
        };
        self.disks
            .iter()
            .any(|d| &d.id == id && &d.serial == serial && d.size == *size)
    }

    /// Acts as if `action` had been pressed: for drivers that script the
    /// screens, as the snapshot example does, and for tests.
    pub fn press(&mut self, action: HubAction) {
        self.act(action);
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// Whether the last frame drawn offers `action` to a press.
    pub fn offers_action(&self, action: HubAction) -> bool {
        let a = action.host();
        !self.showing && self.hits.iter().any(|(_, h)| *h == a)
    }

    /// Where the last frame drawn offers `action`, in pixels: the middle
    /// of where it can be pressed, for tests that touch it.
    pub fn where_offered(&self, action: HubAction) -> Option<(u16, u16)> {
        let a = action.host();
        let (r, _) = self.hits.iter().rev().find(|(_, h)| *h == a)?;
        Some(((r.x + r.w / 2) as u16, (r.y + r.h / 2) as u16))
    }

    /// The strings the current frame draws.
    pub fn drawn_texts(&mut self) -> Vec<String> {
        if self.showing {
            return self.held.drawn_texts();
        }
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(true);
        }
        self.render();
        let texts = self
            .canvas
            .as_ref()
            .map(|c| {
                c.ink()
                    .iter()
                    .filter_map(|i| match &i.kind {
                        osk_ui::canvas::InkKind::Text { text, .. } => Some(text.clone()),
                        osk_ui::canvas::InkKind::Icon(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(false);
        }
        self.dirty = true;
        texts
    }

    /// Reports what the shell read (§2): this computer, the disks, the
    /// payload, a write's progress.
    pub fn hub_event(&mut self, event: HubEvent) {
        match event {
            HubEvent::Computer(c) => self.computer = c,
            HubEvent::BitLocker(b) => self.bitlocker = b,
            HubEvent::Disks(d) => {
                for disk in &d {
                    if !self.seen.contains(&disk.id) {
                        self.seen.push(disk.id.clone());
                    }
                }
                self.disks = d;
                // A disk pulled out before Confirm is no longer chosen.
                if let Some(m) = self.making.as_mut()
                    && m.confirmed.is_none()
                    && m.disk
                        .as_ref()
                        .is_some_and(|id| !self.disks.iter().any(|d| &d.id == id))
                {
                    m.disk = None;
                    m.mode = None;
                    if self.page == Page::Mode {
                        self.page = Page::Disks;
                    }
                }
            }
            HubEvent::Payload(p) => self.payload = p,
            HubEvent::DevImage(d) => self.dev = Some(d),
            HubEvent::Installed(i) => self.installed = i,
            HubEvent::NoWriter => self.no_writer = true,
            HubEvent::WriteProgress { stage, done, total } => {
                if let Some(m) = self.making.as_mut() {
                    m.progress = Some((stage, done, total));
                }
            }
            HubEvent::Written(r) => {
                if let Some(m) = self.making.as_mut() {
                    m.result = Some(r);
                    m.progress = None;
                    self.page = Page::Done;
                    self.offset = 0.0;
                }
            }
            HubEvent::InstallDone(r) => {
                self.installing = false;
                self.install_result = Some(r);
            }
            HubEvent::UninstallDone(r) => match r {
                // The entry left in the firmware is said on Installed.
                Ok(()) if self.entry_kept.is_some() => {
                    self.uninstall_failed = None;
                    self.installed = None;
                }
                Ok(()) => {
                    self.uninstall_failed = None;
                    self.installed = None;
                    self.go(Page::Home);
                }
                Err(why) => self.uninstall_failed = Some(why),
            },
            HubEvent::DesktopStarted(r) => self.desktop_failed = r.err(),
            HubEvent::RestartFailed(why) => self.restart_failed = Some(why),
            HubEvent::StickRestartFailed(why) => self.stick_restart_failed = Some(why),
            HubEvent::BootEntryKept(why) => self.entry_kept = Some(why),
        }
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    fn go(&mut self, page: Page) {
        self.page = page;
        self.sheet = None;
        self.offset = 0.0;
    }

    /// Opens the held desktop app on Transfer, or on the Learn sheet with
    /// `pages`.
    fn show_held(&mut self, pages: Option<Vec<&'static osk_learn::Page>>) {
        match pages {
            Some(pages) => {
                self.held.screen = Screen::Home;
                self.held.learn_with(pages);
            }
            None => self.held.press(Action::Nav(Screen::Transfer)),
        }
        self.showing = self.held_on_show();
        self.pressed = None;
    }

    /// Whether the held app is on one of its two screens: Transfer, or a
    /// sheet (Learn, or Transfer's codes and camera).
    fn held_on_show(&self) -> bool {
        self.held.screen == Screen::Transfer || self.held.sheet.is_some()
    }

    /// The Learn pages the hub's Learn row opens.
    pub fn learn_pages(&self) -> Vec<&'static osk_learn::Page> {
        let mut pages: Vec<&'static osk_learn::Page> =
            LearnId::ALL.iter().filter_map(|id| id.page()).collect();
        if let id @ LearnId::Maker(_) = LearnId::for_computer(&self.computer)
            && let Some(page) = id.page()
        {
            pages.insert(0, page);
        }
        pages.push(faraday_core::learn::upgrade_page());
        pages
    }

    fn act(&mut self, action: HubAction) {
        // While the sheet is up, only its own buttons.
        if self.sheet.is_some()
            && !matches!(
                action,
                HubAction::Cancel | HubAction::RestartNow | HubAction::UninstallNow
            )
        {
            return;
        }
        match action {
            HubAction::Back => self.back(),
            HubAction::Home => self.go(Page::Home),
            HubAction::Make(target) => {
                self.making = Some(Making {
                    target,
                    disk: None,
                    mode: None,
                    confirmed: None,
                    refused: None,
                    progress: None,
                    result: None,
                });
                self.go(Page::Disks);
            }
            HubAction::Next => match self.card() {
                Card::Make => self.act(HubAction::Make(Target::Stick)),
                Card::BitLocker | Card::Resume { offered: true } => self.go(Page::BitLocker),
                Card::Resume { offered: false } => {}
                Card::Firmware => {
                    self.restart_failed = None;
                    self.sheet = Some(Sheet::Restart(RestartTo::Firmware));
                }
                Card::Stick => self.sheet = Some(Sheet::Restart(RestartTo::Stick)),
            },
            HubAction::Item(item) => match item {
                Item::MakeStick => self.act(HubAction::Make(Target::Stick)),
                Item::BitLocker | Item::Resume => self.go(Page::BitLocker),
                Item::Start | Item::SecureBootOn => self.go(Page::Start),
            },
            HubAction::Disk(n) => {
                let Some(id) = self.seen.get(usize::from(n)).cloned() else {
                    return;
                };
                let open = self
                    .offers()
                    .iter()
                    .any(|o| o.disk.id == id && o.refused.is_none());
                if self.page != Page::Disks || !open || self.no_writer {
                    return;
                }
                if let Some(m) = self.making.as_mut() {
                    m.disk = Some(id);
                    m.mode = None;
                    m.confirmed = None;
                    m.refused = None;
                }
                self.go(Page::Mode);
            }
            HubAction::Mode(mode) => {
                if self.page == Page::Mode
                    && self.mode_refusal(mode).is_none()
                    && let Some(m) = self.making.as_mut()
                {
                    m.mode = Some(mode);
                }
            }
            HubAction::Continue => {
                let Some(disk) = self.chosen_disk().cloned() else {
                    return;
                };
                let ready = self
                    .making
                    .as_ref()
                    .and_then(|m| m.mode)
                    .is_some_and(|mode| self.mode_refusal(mode).is_none());
                if self.page == Page::Mode
                    && ready
                    && let Some(m) = self.making.as_mut()
                {
                    m.confirmed = Some((disk.id, disk.device, disk.serial, disk.size));
                    m.refused = None;
                    self.go(Page::Confirm);
                }
            }
            HubAction::Write => self.write(),
            HubAction::Again => {
                if let Some(target) = self.making.as_ref().map(|m| m.target) {
                    self.act(HubAction::Make(target));
                }
            }
            HubAction::Start => self.go(Page::Start),
            HubAction::Restart => {
                if self.page == Page::Start && !self.facts().protected_open() {
                    self.restart_failed = None;
                    self.sheet = Some(Sheet::Restart(RestartTo::Firmware));
                }
            }
            HubAction::RestartNow => match self.sheet {
                Some(Sheet::Restart(RestartTo::Firmware)) => {
                    self.sheet = None;
                    self.out.push_back(HubCommand::RestartIntoFirmware);
                }
                Some(Sheet::Restart(RestartTo::Stick)) => {
                    self.sheet = None;
                    // A stick pulled out while the sheet was up: nothing
                    // to start, and the card says so by offering Make.
                    if let Some(d) = self.facts().stick() {
                        let device = d.device.clone();
                        self.out.push_back(HubCommand::RestartFromStick { device });
                    }
                }
                _ => {}
            },
            HubAction::Cancel => self.sheet = None,
            HubAction::Install => {
                self.install_result = None;
                self.go(Page::Install);
            }
            HubAction::InstallDesktop => {
                if self.payload_file(PayloadKind::Desktop).is_some() {
                    self.install_desktop = !self.install_desktop;
                }
            }
            HubAction::InstallNow => {
                if self.page == Page::Install && !self.installing {
                    self.installing = true;
                    self.install_result = None;
                    let desktop =
                        self.install_desktop && self.payload_file(PayloadKind::Desktop).is_some();
                    self.out.push_back(HubCommand::Install { desktop });
                }
            }
            HubAction::Installed => {
                self.uninstall_failed = None;
                self.entry_kept = None;
                self.go(Page::Installed);
            }
            HubAction::Uninstall => {
                if self.page == Page::Installed && self.installed.is_some() {
                    self.sheet = Some(Sheet::Uninstall);
                }
            }
            HubAction::UninstallNow => {
                if self.sheet == Some(Sheet::Uninstall) {
                    self.sheet = None;
                    self.entry_kept = None;
                    self.out.push_back(HubCommand::Uninstall);
                }
            }
            HubAction::TryDesktop => {
                if self.offers_desktop() {
                    self.desktop_failed = None;
                    self.out.push_back(HubCommand::RunDesktop);
                }
            }
            HubAction::Transfer => self.show_held(None),
            HubAction::Learn => {
                let pages = self.learn_pages();
                self.show_held(Some(pages));
            }
            HubAction::LearnPage(id) => {
                if let Some(page) = id.page() {
                    self.show_held(Some(vec![page]));
                }
            }
            HubAction::About => self.go(Page::About),
            HubAction::BitLocker => self.go(Page::BitLocker),
        }
    }

    /// Whether Try the desktop app is offered: the payload carries it, or
    /// it is installed.
    pub fn offers_desktop(&self) -> bool {
        self.payload_file(PayloadKind::Desktop).is_some()
            || self.installed.as_ref().is_some_and(|i| {
                i.items
                    .iter()
                    .any(|(item, _)| *item == api::InstallItem::Desktop)
            })
    }

    /// Whether the running copy is newer than the installed one (or the
    /// installed one reports no version).
    pub fn update_offered(&self) -> bool {
        let Some(i) = &self.installed else {
            return false;
        };
        let n = faraday_core::upgrade::version_number;
        // A copy that does not report a version is older than any.
        match (n(VERSION), n(&i.version)) {
            (Some(r), Some(i)) => r > i,
            (Some(_), None) => true,
            _ => false,
        }
    }

    /// Confirm's action: the disk checked again, then the write asked of
    /// the shell, which checks it once more (§4.1).
    fn write(&mut self) {
        if self.page != Page::Confirm {
            return;
        }
        let Some(m) = self.making.as_ref() else {
            return;
        };
        let (Some((id, device, serial, size)), Some(mode)) = (m.confirmed.clone(), m.mode) else {
            return;
        };
        let Some(image) = self.image(m.target) else {
            return;
        };
        if !self.confirmed_matches() {
            if let Some(m) = self.making.as_mut() {
                m.refused = Some(draw::not_confirmed(m.target));
            }
            return;
        }
        if self.no_writer || self.mode_refusal(mode).is_some() {
            return;
        }
        self.out.push_back(HubCommand::Write(WriteOrder {
            source: image.source,
            disk: id,
            device,
            serial,
            size,
            mode,
        }));
        if let Some(m) = self.making.as_mut() {
            m.refused = None;
            m.progress = Some((WriteStage::Writing, 0, image.size));
        }
        self.go(Page::Writing);
    }

    fn back(&mut self) {
        if self.sheet.is_some() {
            self.sheet = None;
            return;
        }
        let to = match self.page {
            Page::Home => return,
            Page::Mode => Page::Disks,
            Page::Confirm => Page::Mode,
            // A write under way goes on: its page stays.
            Page::Writing => return,
            _ => Page::Home,
        };
        if to == Page::Disks
            && let Some(m) = self.making.as_mut()
        {
            m.disk = None;
            m.mode = None;
        }
        if to == Page::Mode
            && let Some(m) = self.making.as_mut()
        {
            m.confirmed = None;
            m.refused = None;
        }
        self.go(to);
    }

    fn display(&mut self, d: DisplayInfo) {
        let fit = (f32::from(d.width) / 1280.0).min(f32::from(d.height) / 800.0);
        let density = f32::from(d.dpi.max(1)) / 160.0;
        // As the desktop app lays out: a panel under 600 dp wide gets the
        // one-column layout at its own density.
        self.compact = f32::from(d.width) / density < 600.0;
        let f = if self.compact {
            density
        } else {
            fit.min(density * 1.25)
        }
        .max(0.5);
        let scaled = DisplayInfo {
            dpi: (160.0 * f).round().max(1.0) as u16,
            ..d
        };
        self.f = f;
        self.w = f32::from(d.width) / f;
        self.h = f32::from(d.height) / f;
        self.canvas = Some(Canvas::new(&scaled));
    }

    fn render(&mut self) {
        let Some(mut canvas) = self.canvas.take() else {
            return;
        };
        let mut hits = std::mem::take(&mut self.hits);
        hits.clear();
        self.offset = self.offset.clamp(0.0, self.max.max(0.0));
        let max = {
            let mut ui = faraday_core::ui::Ui::new(&mut canvas, self.f, &mut hits, self.pressed);
            ui.hovered = self.hovered;
            ui.compact = self.compact;
            ui.offset = self.offset;
            draw::frame(self, &mut ui)
        };
        self.max = max;
        self.hits = hits;
        self.canvas = Some(canvas);
    }

    fn hit_at(&self, x: i32, y: i32) -> Option<Action> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
            .map(|(_, a)| *a)
    }

    fn touch(&mut self, x: i32, y: i32, phase: TouchPhase) {
        match phase {
            TouchPhase::Down => self.pressed = self.hit_at(x, y),
            TouchPhase::Move => {
                if self.pressed.is_some() && self.hit_at(x, y) != self.pressed {
                    self.pressed = None;
                }
            }
            TouchPhase::Up => {
                let pressed = self.pressed.take();
                if let Some(a) = pressed
                    && self.hit_at(x, y) == Some(a)
                    && let Action::Host(code) = a
                    && let Some(action) = HubAction::from_code(code)
                {
                    self.act(action);
                }
            }
        }
    }

    /// After an event the held app took: once it leaves Transfer and its
    /// sheets, the hub's page it was opened from is shown again (Home,
    /// for Transfer and Learn's rows).
    fn held_after(&mut self) {
        if self.showing && !self.held_on_show() {
            self.showing = false;
            self.pressed = None;
            self.dirty = true;
            self.commands.push_back(Command::Draw);
        }
    }
}

/// Why an option of What to write cannot be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeRefusal {
    /// Keep data: the disk is not a Faraday stick.
    NotFaraday,
    /// Keep data: the stick already holds this build.
    Same,
    /// Keep data on a Pi card: not until the Pi has its upgrade (§9.5).
    NotYet,
    /// The whole disk: the image is a boot partition alone.
    BootOnly,
}

impl Facts<'_> {
    /// Windows with a protected drive whose BitLocker step is not done:
    /// the firmware steps' buttons are not offered (§3.4 step 1).
    pub fn protected_open(&self) -> bool {
        self.computer.os == Os::Windows && self.protected() && !self.guard_done()
    }
}

impl App for Hub {
    fn event(&mut self, event: Event) {
        match event {
            Event::Display(d) => {
                self.display(d);
                self.held.event(Event::Display(d));
            }
            Event::Tick { .. } => {
                self.held.event(event);
                self.held_after();
                return;
            }
            Event::Touch { x, y, phase } if !self.showing => {
                self.touch(i32::from(x), i32::from(y), phase);
            }
            Event::Hover { x, y } if !self.showing => {
                self.hover = Some((i32::from(x), i32::from(y)));
                let now = self.hit_at(i32::from(x), i32::from(y));
                if now == self.hovered {
                    return;
                }
                self.hovered = now;
            }
            Event::HoverEnd if !self.showing => {
                self.hover = None;
                if self.hovered.take().is_none() {
                    return;
                }
            }
            Event::Wheel { dy, .. } | Event::Scroll { dy, .. } if !self.showing => {
                self.offset =
                    (self.offset + f32::from(dy) / self.f.max(0.1)).clamp(0.0, self.max.max(0.0));
            }
            Event::Back if !self.showing => self.act(HubAction::Back),
            Event::Key(Key::Escape) if !self.showing => self.act(HubAction::Back),
            Event::Key(_) | Event::KeyUp(_) | Event::Shift { .. } | Event::Forward
                if !self.showing =>
            {
                return;
            }
            Event::ScrollEnd { .. } if !self.showing => return,
            // Everything else is the held app's: input while it is on
            // screen, and the answers to what it asked (entropy, the
            // camera, files).
            other => {
                self.held.event(other);
                self.held_after();
                return;
            }
        }
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    fn poll_command(&mut self) -> Option<Command> {
        if let Some(c) = self.commands.pop_front() {
            return Some(c);
        }
        // The held app's own: a frame only while it is on screen.
        while let Some(c) = self.held.poll_command() {
            if c == Command::Draw && !self.showing {
                continue;
            }
            return Some(c);
        }
        None
    }

    fn frame(&mut self) -> Frame<'_> {
        if self.showing {
            return self.held.frame();
        }
        if self.dirty {
            self.dirty = false;
            self.render();
        }
        match self.canvas.as_ref() {
            Some(c) => c.frame(),
            None => Frame {
                width: 0,
                height: 0,
                rgba: &[],
            },
        }
    }
}
