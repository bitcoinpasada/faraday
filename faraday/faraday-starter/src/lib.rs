//! The Starter (`docs/STARTER.md`): its state, screens and decisions, with no
//! I/O. It is an [`osk_shell_api::App`], as `faraday-core`'s `Faraday`
//! is: the shell sends input and display events through [`App::event`],
//! reads the frame, and takes the drawing commands from
//! [`App::poll_command`]. What upstream's interface has no event for
//! goes through a second channel beside it, as `Faraday`'s storage does:
//! the shell reports what it reads through [`Starter::starter_event`] and takes
//! what the Starter asks for from [`Starter::next_command`] ([`api`]).
//!
//! Transfer and Learn are the desktop app's own screens (§2): the Starter
//! holds a `Faraday` in online mode ([`Faraday::held`]) and draws it
//! while one of them is open; its way back returns to the Starter's Home.
//! The shell serves that `Faraday`'s storage channel through
//! [`Starter::held`].

pub mod api;
pub mod concern;
pub mod decide;
mod draw;
pub mod macs;
pub mod notes;
pub mod template;

/// What a [`api::Reason`] reads as on screen, from `faraday_core::strings`.
pub fn reason_text(reason: &api::Reason) -> String {
    draw::reason_text(reason)
}

use std::collections::VecDeque;

use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, Command, DisplayInfo, Event, Frame, Key, TouchPhase};
use osk_ui::{Canvas, Rect};

use api::{
    BitLocker, BitLockerStep, Computer, Concern, DevImage, Disk, Installed, Kind, Mode, Os,
    Payload, PayloadFile, PayloadKind, Protection, Reason, Source, StarterCommand, StarterEvent,
    WriteOrder, WriteStage,
};
use decide::{Facts, Item, Target};
use macs::StickFor;

/// The website's guides (§7.4), shared with the stick.
pub use faraday_learn::GUIDES;

/// The Starter's own version, which About shows and Install compares.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A page of the Starter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// Home: This computer, the checklist, the rows (§3.1).
    Home,
    /// For which computer? (§3.2 step 0), once the payload carries both
    /// stick images.
    Which,
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
    /// Blank templates (§3.8): the choices as value rows, Print, Save the PDF.
    Templates,
    /// Blank templates' Words choice.
    TemplateWords,
    /// Blank templates' Copy choice.
    TemplateCopy,
    /// Blank templates' Places count.
    TemplatePlaces,
}

/// A sheet over a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    /// A restart, asked once (§3.1, §3.4 step 2, §7.3).
    Restart(RestartTo),
    /// Uninstall, asked once (§3.5).
    Uninstall,
    /// Continue anyway, asked once for the item [`Starter::asking`] names
    /// (§3.1a).
    LetThrough,
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
    /// Restart into firmware settings, after the sheet.
    Firmware,
    /// Restart from the stick, after the sheet.
    Stick,
    /// Start this Mac from the stick: opens §3.4's Mac steps. A Mac has
    /// no restart into firmware settings or from the stick.
    StartMac,
    /// Another encryption product's rescue disk or password is not yet
    /// confirmed (§6.4): the button confirms it, and the restart follows.
    Rescue,
}

/// A guide, by its Learn page (§7.2, `docs/learn/faraday/boot/`). A page
/// that does not exist shows its title as a line and opens nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnId {
    /// Starting a Windows computer from the stick.
    StartWindows,
    /// Starting a Linux computer from the stick.
    StartLinux,
    /// When the stick does not start.
    NoStart,
    /// Replacing Faraday on a stick.
    KeepData,
    /// A maker family's page, by its id in `faraday_learn::MAKERS`
    /// (§7.6).
    Maker(&'static str),
    /// What Secure Boot and BitLocker are (§3.1b).
    SecureBootBitLocker,
    /// Screens and devices that can record (§3.1a).
    ScreensThatRecord,
    /// About the boot entry (§3.1c).
    BootEntry,
    /// Starting an Intel Mac from the stick (`docs/MAC.md` §3.1–§3.2).
    MacIntel,
}

impl LearnId {
    /// The page's id among the guides (`faraday_guides::BOOT`), which
    /// is also its name on the website.
    pub fn id(self) -> &'static str {
        match self {
            LearnId::StartWindows => "windows",
            LearnId::StartLinux => "linux",
            LearnId::NoStart => "not-starting",
            LearnId::KeepData => "replace-faraday",
            LearnId::Maker(id) => id,
            LearnId::SecureBootBitLocker => "secure-boot-bitlocker",
            LearnId::ScreensThatRecord => "screens-that-record",
            LearnId::BootEntry => "boot-entry",
            LearnId::MacIntel => "mac-intel",
        }
    }

    /// The page.
    pub fn page(self) -> Option<&'static osk_learn::Page> {
        faraday_guides::page(self.id())
    }

    /// Its address on the website (§7.4).
    pub fn address(self) -> String {
        faraday_learn::guide_address(self.id())
    }

    /// The guide for this computer (§3.4): on a Mac, whatever system it
    /// runs, the Mac guide; otherwise its maker's page where there is one
    /// (§7.6), and the general page for its operating system where there
    /// is not.
    pub fn for_computer(c: &Computer) -> LearnId {
        if macs::is_apple(&c.maker) {
            return LearnId::MacIntel;
        }
        let general = match c.os {
            Os::Windows => LearnId::StartWindows,
            // macOS on a computer whose maker is not Apple: none is
            // expected, and Linux's page is the nearer of the two.
            Os::Linux | Os::MacOs => LearnId::StartLinux,
        };
        match faraday_learn::guide_id(&c.maker, &c.family, &c.model, general.id()) {
            id if id == general.id() => general,
            id => LearnId::Maker(id),
        }
    }

    const ALL: [LearnId; 8] = [
        LearnId::StartWindows,
        LearnId::StartLinux,
        LearnId::NoStart,
        LearnId::KeepData,
        LearnId::SecureBootBitLocker,
        LearnId::ScreensThatRecord,
        LearnId::BootEntry,
        LearnId::MacIntel,
    ];
}

/// Everything a press can do on the Starter's own pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterAction {
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
    /// A disk of the list, by the number the Starter gave it when it was
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
    /// Install now.
    InstallNow,
    /// Installed: what and where.
    Installed,
    /// Uninstall: asks first.
    Uninstall,
    /// Uninstall now.
    UninstallNow,
    /// Transfer, the desktop app's own.
    Transfer,
    /// Learn, the desktop app's sheet with the Starter's pages.
    Learn,
    /// A guide's Learn page.
    LearnPage(LearnId),
    /// About.
    About,
    /// I have its rescue disk or password: another encryption product
    /// (§6.4).
    Rescue,
    /// Check again: what is attached read again now (§3.1a).
    CheckAgain,
    /// Continue anyway for an item of concern, by the number the Starter
    /// gave it when it was first seen: asks once.
    ContinueAnyway(u16),
    /// Continue anyway, as the sheet asked.
    LetThrough,
    /// Remove the Faraday boot entry (Linux, §3.1c).
    RemoveEntry,
    /// For which computer? answered (§3.2 step 0).
    StickFor(StickFor),
    /// Blank templates (§3.8).
    Templates,
    /// A blank template's choice, opened from its row.
    TemplateChoice(TemplateChoice),
    /// Words, ticked: 12 or 24.
    TemplateWords(usize),
    /// Copy, ticked.
    TemplateCopy(template::Copy),
    /// Places: one fewer.
    PlacesFewer,
    /// Places: one more.
    PlacesMore,
    /// Print the blank template: its PDF opened in the PDF viewer.
    PrintTemplate,
    /// Save the blank template's PDF into Downloads.
    SaveTemplate,
    /// Open the saved PDF.
    OpenTemplate,
    /// Print what to write down on the Start page (§3.4, F47): its PDF
    /// opened in the system's viewer.
    PrintNotes,
    /// Include the BitLocker recovery key on the sheet, ticked or not
    /// (F49).
    IncludeKey,
    /// Temporarily suspend BitLocker (§6, F49).
    SuspendBitLocker,
    /// Resume BitLocker (§6, F49).
    ResumeBitLocker,
}

/// Which blank-template choice a row opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateChoice {
    /// Words: 12 or 24.
    Words,
    /// Copy: Words, SeedQR or Both.
    Copy,
    /// Places: a count.
    Places,
}

/// A maker page's number in an action's code: this plus its place in
/// `faraday_learn::MAKERS`.
const MAKER_CODE: u32 = 0x100;

const ITEMS: [Item; 3] = [Item::MakeStick, Item::Start, Item::SecureBootOn];

impl StarterAction {
    /// The number `faraday_core`'s [`Action::Host`] carries for it.
    pub fn code(self) -> u32 {
        let (kind, arg): (u32, u32) = match self {
            StarterAction::Back => (0, 0),
            StarterAction::Home => (1, 0),
            StarterAction::Make(t) => (2, u32::from(t == Target::Pi)),
            StarterAction::Next => (3, 0),
            StarterAction::Item(i) => (4, ITEMS.iter().position(|x| *x == i).unwrap_or(0) as u32),
            StarterAction::Disk(n) => (5, u32::from(n)),
            StarterAction::Mode(m) => (6, u32::from(m == Mode::KeepData)),
            StarterAction::Continue => (7, 0),
            StarterAction::Write => (8, 0),
            StarterAction::Again => (9, 0),
            StarterAction::Start => (10, 0),
            StarterAction::Restart => (11, 0),
            StarterAction::RestartNow => (12, 0),
            StarterAction::Cancel => (13, 0),
            StarterAction::Install => (14, 0),
            StarterAction::InstallNow => (16, 0),
            StarterAction::Installed => (17, 0),
            StarterAction::Uninstall => (18, 0),
            StarterAction::UninstallNow => (19, 0),
            StarterAction::Transfer => (21, 0),
            StarterAction::Learn => (22, 0),
            StarterAction::LearnPage(LearnId::Maker(id)) => (
                23,
                MAKER_CODE
                    + faraday_learn::MAKERS
                        .iter()
                        .position(|m| m.id == id)
                        .unwrap_or(0) as u32,
            ),
            StarterAction::LearnPage(id) => (
                23,
                LearnId::ALL.iter().position(|x| *x == id).unwrap_or(0) as u32,
            ),
            StarterAction::About => (24, 0),
            StarterAction::Rescue => (33, 0),
            StarterAction::CheckAgain => (34, 0),
            StarterAction::ContinueAnyway(n) => (35, u32::from(n)),
            StarterAction::LetThrough => (36, 0),
            StarterAction::RemoveEntry => (37, 0),
            StarterAction::StickFor(k) => (40, u32::from(k == StickFor::AppleSilicon)),
            StarterAction::Templates => (41, 0),
            StarterAction::TemplateChoice(c) => (42, c as u32),
            StarterAction::TemplateWords(n) => (43, n as u32),
            StarterAction::TemplateCopy(c) => (44, c as u32),
            StarterAction::PlacesFewer => (45, 0),
            StarterAction::PlacesMore => (46, 0),
            StarterAction::PrintTemplate => (47, 0),
            StarterAction::SaveTemplate => (48, 0),
            StarterAction::OpenTemplate => (49, 0),
            StarterAction::PrintNotes => (50, 0),
            StarterAction::IncludeKey => (51, 0),
            StarterAction::SuspendBitLocker => (52, 0),
            StarterAction::ResumeBitLocker => (53, 0),
        };
        (kind << 16) | arg
    }

    /// The action a number stands for.
    pub fn from_code(code: u32) -> Option<StarterAction> {
        let arg = code & 0xffff;
        Some(match code >> 16 {
            0 => StarterAction::Back,
            1 => StarterAction::Home,
            2 => StarterAction::Make(if arg == 1 { Target::Pi } else { Target::Stick }),
            3 => StarterAction::Next,
            4 => StarterAction::Item(*ITEMS.get(arg as usize)?),
            5 => StarterAction::Disk(arg as u16),
            6 => StarterAction::Mode(if arg == 1 {
                Mode::KeepData
            } else {
                Mode::Whole
            }),
            7 => StarterAction::Continue,
            8 => StarterAction::Write,
            9 => StarterAction::Again,
            10 => StarterAction::Start,
            11 => StarterAction::Restart,
            12 => StarterAction::RestartNow,
            13 => StarterAction::Cancel,
            14 => StarterAction::Install,
            16 => StarterAction::InstallNow,
            17 => StarterAction::Installed,
            18 => StarterAction::Uninstall,
            19 => StarterAction::UninstallNow,
            21 => StarterAction::Transfer,
            22 => StarterAction::Learn,
            23 if arg >= MAKER_CODE => StarterAction::LearnPage(LearnId::Maker(
                faraday_learn::MAKERS.get((arg - MAKER_CODE) as usize)?.id,
            )),
            23 => StarterAction::LearnPage(*LearnId::ALL.get(arg as usize)?),
            24 => StarterAction::About,
            33 => StarterAction::Rescue,
            34 => StarterAction::CheckAgain,
            35 => StarterAction::ContinueAnyway(arg as u16),
            36 => StarterAction::LetThrough,
            37 => StarterAction::RemoveEntry,
            40 => StarterAction::StickFor(if arg == 1 {
                StickFor::AppleSilicon
            } else {
                StickFor::Pc
            }),
            41 => StarterAction::Templates,
            42 => StarterAction::TemplateChoice(match arg {
                0 => TemplateChoice::Words,
                1 => TemplateChoice::Copy,
                _ => TemplateChoice::Places,
            }),
            43 => StarterAction::TemplateWords(arg as usize),
            44 => StarterAction::TemplateCopy(*template::Copy::ALL.get(arg as usize)?),
            45 => StarterAction::PlacesFewer,
            46 => StarterAction::PlacesMore,
            47 => StarterAction::PrintTemplate,
            48 => StarterAction::SaveTemplate,
            49 => StarterAction::OpenTemplate,
            50 => StarterAction::PrintNotes,
            51 => StarterAction::IncludeKey,
            52 => StarterAction::SuspendBitLocker,
            53 => StarterAction::ResumeBitLocker,
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
    /// For which computer the stick is (§3.2 step 0): nothing until a
    /// person answers, and nothing when the question is not asked.
    pub stick_for: Option<StickFor>,
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
    /// A developer's image, not the Starter's own.
    pub dev: bool,
    /// A boot partition alone: keep data only.
    pub boot_only: bool,
}

/// The Starter.
pub struct Starter {
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
    out: VecDeque<StarterCommand>,
    dirty: bool,

    /// This computer.
    pub computer: Computer,
    /// The QR codes of the guide pages the Start page and the restart
    /// sheet put on a phone, by address: made when this computer is
    /// reported ([`Starter::phone_codes_make`]), so drawing makes none.
    phone_codes: Vec<(String, osk_codec::qr::QrMatrix)>,
    /// The disks attached, with the versions read through the elevated
    /// copy this session in place of the shell's unread ones.
    pub disks: Vec<Disk>,
    /// The disks as the shell last sent them.
    shell_disks: Vec<Disk>,
    /// The versions the elevated copy read this session (Windows,
    /// tracker #21), by disk id, serial and size: asked once per stick,
    /// and forgotten once the stick is written.
    versions: Vec<((String, String, u64), Option<String>)>,
    /// A stick's version being read through the elevated copy: its id.
    pub version_reading: Option<String>,
    /// Why the last such read did not happen.
    pub version_failed: Option<Reason>,
    /// The payload, when the Starter carries one.
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
    /// An install under way.
    pub installing: bool,
    /// How the last install ended.
    pub install_result: Option<Result<String, Reason>>,
    /// Why the last uninstall failed.
    pub uninstall_failed: Option<Reason>,
    /// Why the restart into firmware settings did not happen.
    pub restart_failed: Option<Reason>,
    /// Why the restart from the stick did not happen: the card offers
    /// the restart into firmware settings instead (§3.1).
    pub stick_restart_failed: Option<Reason>,
    /// Why uninstall left the firmware's `Faraday stick` entry.
    pub entry_kept: Option<Reason>,
    /// The blank template's choices (§3.8).
    pub template: template::Template,
    /// Why the last Print on the Start page did not open (§3.4).
    pub notes_open_failed: Option<Reason>,
    /// BitLocker on the system drive, as last read (Windows, §6, F49).
    pub bitlocker: Option<BitLocker>,
    /// A BitLocker step under way: the elevation prompt, then the step.
    pub bitlocker_step: Option<BitLockerStep>,
    /// Why the last BitLocker step did not happen.
    pub bitlocker_failed: Option<Reason>,
    /// The sheet is to carry the recovery key (F49): unticked until the
    /// person ticks it, for the session.
    pub include_key: bool,
    /// The recovery key is being read for the sheet.
    pub key_reading: bool,
    /// Another encryption product found here (§6.4), by name.
    pub other: Option<String>,
    /// The person said they have its rescue disk or password.
    pub other_confirmed: bool,

    /// What is attached that is of concern (§3.1a), as last read.
    pub concerns: Vec<Concern>,
    /// Every concern's id seen, so its Continue anyway keeps its number.
    concern_seen: Vec<String>,
    /// The ids let through with Continue anyway, for the session.
    pub let_through: Vec<String>,
    /// The item Continue anyway's sheet asks about.
    pub asking: Option<String>,
    /// The firmware's `Faraday stick` boot entry exists (Linux, §3.1c).
    pub boot_entry: bool,
    /// Its removal is under way.
    pub entry_removing: bool,
    /// Why its removal did not happen.
    pub entry_remove_failed: Option<Reason>,

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
    /// The page under the open sheet, frosted and dimmed, as the frame
    /// that opened the sheet drew it (`faraday_ui::Ui::backdrop`): each
    /// frame after puts it back instead of drawing and blurring the page
    /// again. Invariant: it is the page as the state draws it now. It is
    /// dropped by everything that can change that page: a display, a
    /// report from the shell ([`Starter::starter_event`]), an action
    /// (`act`), a scroll; and by a frame with no sheet. A hover or a
    /// press does not change it: under a sheet only the sheet's own
    /// buttons are pressable, and none of them is on a page.
    frost: Option<Vec<u8>>,
}

impl Default for Starter {
    fn default() -> Self {
        Self::new()
    }
}

impl Starter {
    /// A Starter with nothing reported yet.
    pub fn new() -> Self {
        let mut held = Faraday::new();
        held.online = true;
        held.held = true;
        let mut s = Starter {
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
            phone_codes: Vec::new(),
            disks: Vec::new(),
            shell_disks: Vec::new(),
            versions: Vec::new(),
            version_reading: None,
            version_failed: None,
            payload: None,
            dev: None,
            installed: None,
            no_writer: false,
            page: Page::Home,
            sheet: None,
            making: None,
            installing: false,
            install_result: None,
            uninstall_failed: None,
            restart_failed: None,
            stick_restart_failed: None,
            entry_kept: None,
            template: template::Template::default(),
            notes_open_failed: None,
            bitlocker: None,
            bitlocker_step: None,
            bitlocker_failed: None,
            include_key: false,
            key_reading: false,
            other: None,
            other_confirmed: false,
            concerns: Vec::new(),
            concern_seen: Vec::new(),
            let_through: Vec::new(),
            asking: None,
            boot_entry: false,
            entry_removing: false,
            entry_remove_failed: None,
            offset: 0.0,
            max: 0.0,
            seen: Vec::new(),
            held,
            showing: false,
            frost: None,
        };
        s.phone_codes_make();
        s
    }

    /// Costs a QR encoding per guide address not made yet: the codes of
    /// the guide pages the Start page and the restart sheet show for this
    /// computer (its own guide, and the Mac guide a Mac's Start page
    /// names), made with the held app's token and kept until the computer
    /// changes ([`Starter::phone_code`]).
    fn phone_codes_make(&mut self) {
        let want = [
            LearnId::for_computer(&self.computer).address(),
            LearnId::MacIntel.address(),
        ];
        self.phone_codes.retain(|(a, _)| want.contains(a));
        let c = self.held.compute();
        for address in want {
            if self.phone_codes.iter().any(|(a, _)| *a == address) {
                continue;
            }
            if let Ok(m) = osk_codec::qr::encode(
                &c,
                osk_codec::qr::Payload::Bytes(address.as_bytes()),
                osk_codec::qr::Ecc::Medium,
            ) {
                self.phone_codes.push((address, m));
            }
        }
    }

    /// The QR code of guide `address`, made when the computer was
    /// reported ([`Starter::phone_codes_make`]). A lookup.
    pub(crate) fn phone_code(&self, address: &str) -> Option<&osk_codec::qr::QrMatrix> {
        self.phone_codes
            .iter()
            .find(|(a, _)| a == address)
            .map(|(_, m)| m)
    }

    /// The facts Home's checklist is read from.
    pub fn facts(&self) -> Facts<'_> {
        Facts {
            computer: &self.computer,
            disks: &self.disks,
            other_open: self.computer.os == Os::Windows
                && self.other.is_some()
                && !self.other_confirmed,
        }
    }

    /// The This computer card's button (§3.1), the first that applies:
    /// Make a Faraday stick with no stick attached, or on a computer
    /// that cannot start one; then Restart from the stick with Secure
    /// Boot off, and Restart into firmware settings with it on, unknown,
    /// or when the restart from the stick did not happen.
    pub fn card(&self) -> Card {
        let f = self.facts();
        // A Mac's firmware starts the stick whatever mode Windows was
        // started in through Boot Camp.
        let mac = f.mac().is_some();
        if f.stick().is_none() || !f.can_start() || (!self.computer.uefi && !mac) {
            return Card::Make;
        }
        if mac {
            return Card::StartMac;
        }
        if f.protected_open() {
            return Card::Rescue;
        }
        if self.computer.secure_boot == Some(false) && self.stick_restart_failed.is_none() {
            Card::Stick
        } else {
            Card::Firmware
        }
    }

    /// The items that hold the restart buttons (§3.1a): a TV and a KVM
    /// not let through, and a virtual machine always (the stick must
    /// start on the real computer). Home's card and the Start page name
    /// them. A wireless receiver holds nothing ([`Starter::receiver_line`]).
    pub fn restart_holds(&self) -> Vec<&Concern> {
        self.concerns
            .iter()
            .filter(|c| match c.kind {
                Kind::VirtualMachine => true,
                Kind::Tv | Kind::Kvm => !self.let_through.contains(&c.id),
                Kind::Receiver | Kind::BoltReceiver => false,
            })
            .collect()
    }

    /// Home's line for a wireless receiver attached (§3.1a), which holds
    /// nothing: the warning for any receiver but Logitech's Bolt, the
    /// Bolt's own line when it is the only kind attached.
    pub fn receiver_line(&self) -> Option<&'static str> {
        let has = |k: Kind| self.concerns.iter().any(|c| c.kind == k);
        if has(Kind::Receiver) {
            Some(faraday_core::strings::EN.devices_receiver_notice)
        } else if has(Kind::BoltReceiver) {
            Some(faraday_core::strings::EN.devices_bolt_notice)
        } else {
            None
        }
    }

    /// The number an item of concern's Continue anyway carries.
    pub fn concern_number(&self, id: &str) -> u16 {
        self.concern_seen.iter().position(|s| s == id).unwrap_or(0) as u16
    }

    /// Whether the Start page offers BitLocker's steps and the key on the
    /// sheet (§6, F49): on Windows, with BitLocker on or suspended on the
    /// system drive as last read. Not encrypted, or not read, offers
    /// nothing.
    pub fn bitlocker_offered(&self) -> bool {
        self.computer.os == Os::Windows
            && self.facts().mac().is_none()
            && self
                .bitlocker
                .as_ref()
                .is_some_and(|b| matches!(b.protection, Protection::On | Protection::Suspended))
    }

    /// Whether `step` can be asked now: offered, nothing under way, and
    /// the drive as the step needs it (Suspend with protection on,
    /// Resume while suspended).
    fn step_open(&self, step: BitLockerStep) -> bool {
        let want = match step {
            BitLockerStep::Suspend => Protection::On,
            BitLockerStep::Resume => Protection::Suspended,
        };
        self.page == Page::Start
            && self.bitlocker_offered()
            && self.bitlocker_step.is_none()
            && self
                .bitlocker
                .as_ref()
                .is_some_and(|b| b.protection == want)
    }

    /// Makes the sheet and asks the shell to open it: with the recovery
    /// key when one is given, which is dropped (and wiped) once the PDF
    /// is made.
    fn print_notes(&mut self, key: Option<api::RecoveryKey>) {
        let c = self.held.compute();
        if let Some(notes) = notes::Notes::for_computer(&self.computer) {
            self.notes_open_failed = None;
            let pdf = match &key {
                Some(k) => notes.pdf_with_key(&c, k),
                None => notes.pdf(&c),
            };
            self.out.push_back(StarterCommand::PrintNotes {
                name: notes::FILE_NAME.to_string(),
                pdf,
                with_key: key.is_some(),
            });
        }
    }

    /// Whether Remove the Faraday boot entry is offered on Home (§3.1c):
    /// on Linux, while the entry exists and no Faraday stick is attached.
    /// With a stick attached the person is about to start from it, which
    /// uses the entry again, so it is not offered then.
    pub fn entry_offered(&self) -> bool {
        self.computer.os == Os::Linux && self.boot_entry && self.facts().stick().is_none()
    }

    /// Pixels per dp the pages are drawn at: the window's density.
    pub fn scale(&self) -> f32 {
        self.f
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

    /// The next thing the Starter asks the shell to do.
    pub fn next_command(&mut self) -> Option<StarterCommand> {
        self.out.pop_front()
    }

    /// The payload file of `kind`, when the Starter carries one.
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
        let apple = self
            .making
            .as_ref()
            .is_some_and(|m| m.stick_for == Some(StickFor::AppleSilicon));
        let kind = match target {
            Target::Stick if apple => PayloadKind::AppleStick,
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

    /// Whether Make a Faraday stick asks For which computer? first
    /// (§3.2 step 0): only when the payload carries both stick images,
    /// and no developer's image stands in for the PC one.
    pub fn asks_which(&self) -> bool {
        self.dev.is_none()
            && self.payload_file(PayloadKind::Stick).is_some()
            && self.payload_file(PayloadKind::AppleStick).is_some()
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

    /// The disks as the shell sent them, each unread stick's version in
    /// place when the elevated copy read it this session.
    fn apply_versions(&mut self) {
        let mut disks = self.shell_disks.clone();
        for d in &mut disks {
            let key = (d.id.clone(), d.serial.clone(), d.size);
            if let Some(f) = d.faraday.as_mut().filter(|f| !f.read)
                && let Some((_, release)) = self.versions.iter().find(|(k, _)| *k == key)
            {
                f.release = release.clone();
                f.read = true;
            }
        }
        self.disks = disks;
    }

    /// Keep data is ticked on a stick whose version is not read yet
    /// (Windows, tracker #21): Continue waits for it.
    pub fn keep_unread(&self) -> bool {
        self.making.as_ref().and_then(|m| m.mode) == Some(Mode::KeepData)
            && self
                .chosen_disk()
                .and_then(|d| d.faraday.as_ref())
                .is_some_and(|f| !f.read)
    }

    /// Keep data ticked on a stick whose version is not read: the
    /// elevated copy asked to read it, once while the read is under way.
    fn read_version(&mut self) {
        if !self.keep_unread() || self.version_reading.is_some() {
            return;
        }
        let Some(d) = self.chosen_disk() else {
            return;
        };
        let (disk, serial, size) = (d.id.clone(), d.serial.clone(), d.size);
        self.version_reading = Some(disk.clone());
        self.out
            .push_back(StarterCommand::ReadVersion { disk, serial, size });
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
    pub fn press(&mut self, action: StarterAction) {
        self.act(action);
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// Whether the last frame drawn offers `action` to a press.
    pub fn offers_action(&self, action: StarterAction) -> bool {
        let a = action.host();
        !self.showing && self.hits.iter().any(|(_, h)| *h == a)
    }

    /// Where the last frame drawn offers `action`, in pixels: the middle
    /// of where it can be pressed, for tests that touch it.
    pub fn where_offered(&self, action: StarterAction) -> Option<(u16, u16)> {
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
        // The page under a sheet is drawn too, not put back from its
        // frosted copy, so its words are read.
        self.frost = None;
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
    pub fn starter_event(&mut self, event: StarterEvent) {
        self.frost = None;
        match event {
            StarterEvent::Computer(c) => {
                self.computer = c;
                self.phone_codes_make();
            }
            StarterEvent::Disks(d) => {
                for disk in &d {
                    if !self.seen.contains(&disk.id) {
                        self.seen.push(disk.id.clone());
                    }
                }
                self.shell_disks = d;
                self.apply_versions();
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
            StarterEvent::Payload(p) => self.payload = p,
            StarterEvent::DevImage(d) => self.dev = Some(d),
            StarterEvent::Installed(i) => self.installed = i,
            StarterEvent::NoWriter => self.no_writer = true,
            StarterEvent::WriteProgress { stage, done, total } => {
                if let Some(m) = self.making.as_mut() {
                    m.progress = Some((stage, done, total));
                }
            }
            StarterEvent::Written(r) => {
                // What the elevated copy read is no longer on the stick.
                if let Some((id, ..)) = self.making.as_ref().and_then(|m| m.confirmed.as_ref()) {
                    let id = id.clone();
                    self.versions.retain(|((v, ..), _)| *v != id);
                    self.apply_versions();
                }
                if let Some(m) = self.making.as_mut() {
                    m.result = Some(r);
                    m.progress = None;
                    self.page = Page::Done;
                    self.offset = 0.0;
                }
            }
            StarterEvent::InstallDone(r) => {
                self.installing = false;
                self.install_result = Some(r);
            }
            StarterEvent::UninstallDone(r) => match r {
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
            StarterEvent::RestartFailed(why) => self.restart_failed = Some(why),
            StarterEvent::StickRestartFailed(why) => self.stick_restart_failed = Some(why),
            StarterEvent::BootEntryKept(why) => self.entry_kept = Some(why),
            StarterEvent::OtherEncryption(o) => self.other = o,
            StarterEvent::Concerns(c) => {
                for item in &c {
                    if !self.concern_seen.contains(&item.id) {
                        self.concern_seen.push(item.id.clone());
                    }
                }
                self.concerns = c;
                // The item asked about is gone: nothing to ask.
                if self.sheet == Some(Sheet::LetThrough)
                    && !self
                        .asking
                        .as_ref()
                        .is_some_and(|id| self.concerns.iter().any(|c| &c.id == id))
                {
                    self.sheet = None;
                    self.asking = None;
                }
                // A restart asked about while nothing held it waits again.
                if matches!(self.sheet, Some(Sheet::Restart(_))) && !self.restart_holds().is_empty()
                {
                    self.sheet = None;
                }
            }
            StarterEvent::TemplateSaved(r) => {
                self.template.saving = false;
                self.template.saved = Some(r);
            }
            StarterEvent::TemplateOpened(r) => {
                self.template.open_failed = r.err();
            }
            StarterEvent::NotesOpened(r) => self.notes_open_failed = r.err(),
            StarterEvent::BitLocker(b) => {
                // The reading without elevation has no count: the one the
                // elevated copy read this session stands while the drive
                // stays suspended (Windows has not started since).
                let kept = self
                    .bitlocker
                    .as_ref()
                    .filter(|o| o.protection == Protection::Suspended)
                    .and_then(|o| o.starts_left);
                self.bitlocker = b.map(|mut b| {
                    if b.protection == Protection::Suspended && b.starts_left.is_none() {
                        b.starts_left = kept;
                    }
                    b
                });
            }
            StarterEvent::BitLockerDone { result, .. } => {
                self.bitlocker_step = None;
                self.bitlocker_failed = result.err();
            }
            StarterEvent::RecoveryKey(r) => {
                self.key_reading = false;
                match r {
                    Ok(key) if self.page == Page::Start && self.include_key => {
                        self.print_notes(Some(key));
                    }
                    Ok(_) => {}
                    Err(why) => self.notes_open_failed = Some(why),
                }
            }
            StarterEvent::VersionRead {
                disk,
                serial,
                size,
                result,
            } => {
                if self.version_reading.as_ref() == Some(&disk) {
                    self.version_reading = None;
                }
                match result {
                    Ok(release) => {
                        self.version_failed = None;
                        let key = (disk, serial, size);
                        self.versions.retain(|(k, _)| *k != key);
                        self.versions.push((key, release));
                        self.apply_versions();
                    }
                    Err(why) => self.version_failed = Some(why),
                }
                // Keep data stays ticked only on a stick whose version is
                // read and allows it.
                let unticked = self
                    .making
                    .as_ref()
                    .is_some_and(|m| m.mode == Some(Mode::KeepData))
                    && (self.keep_unread() || self.mode_refusal(Mode::KeepData).is_some());
                if unticked && let Some(m) = self.making.as_mut() {
                    m.mode = None;
                }
            }
            StarterEvent::BootEntry(e) => self.boot_entry = e,
            StarterEvent::BootEntryRemoved(r) => {
                self.entry_removing = false;
                match r {
                    Ok(()) => {
                        self.boot_entry = false;
                        self.entry_remove_failed = None;
                    }
                    Err(why) => self.entry_remove_failed = Some(why),
                }
            }
        }
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    fn go(&mut self, page: Page) {
        self.page = page;
        self.sheet = None;
        self.offset = 0.0;
        // BitLocker is read from Windows each time the Start page is
        // shown (F49): it may have been changed outside the Starter.
        // A Mac's Start page has no BitLocker block: its steps change
        // Startup Security, not Secure Boot.
        if page == Page::Start && self.computer.os == Os::Windows && self.facts().mac().is_none() {
            self.out.push_back(StarterCommand::ReadBitLocker);
        }
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

    /// The Learn pages the Starter's Learn row opens.
    pub fn learn_pages(&self) -> Vec<&'static osk_learn::Page> {
        let mut pages: Vec<&'static osk_learn::Page> =
            LearnId::ALL.iter().filter_map(|id| id.page()).collect();
        if let id @ LearnId::Maker(_) = LearnId::for_computer(&self.computer)
            && let Some(page) = id.page()
        {
            pages.insert(0, page);
        }
        pages.push(faraday_learn::upgrade_page());
        pages
    }

    fn act(&mut self, action: StarterAction) {
        self.frost = None;
        let c = self.held.compute();
        // While the sheet is up, only its own buttons.
        if self.sheet.is_some()
            && !matches!(
                action,
                StarterAction::Cancel
                    | StarterAction::RestartNow
                    | StarterAction::UninstallNow
                    | StarterAction::LetThrough
            )
            && !(action == StarterAction::LearnPage(LearnId::BootEntry)
                && self.sheet == Some(Sheet::Restart(RestartTo::Stick)))
        {
            return;
        }
        match action {
            StarterAction::Back => self.back(),
            StarterAction::Home => self.go(Page::Home),
            StarterAction::Make(target) => {
                self.making = Some(Making {
                    target,
                    stick_for: None,
                    disk: None,
                    mode: None,
                    confirmed: None,
                    refused: None,
                    progress: None,
                    result: None,
                });
                if target == Target::Stick && self.asks_which() {
                    self.go(Page::Which);
                } else {
                    self.go(Page::Disks);
                }
            }
            StarterAction::StickFor(k) => {
                if self.page == Page::Which
                    && let Some(m) = self.making.as_mut()
                {
                    m.stick_for = Some(k);
                    m.disk = None;
                    m.mode = None;
                    self.go(Page::Disks);
                }
            }
            StarterAction::Next => match self.card() {
                Card::Make => self.act(StarterAction::Make(Target::Stick)),
                Card::Rescue => self.other_confirmed = true,
                Card::Firmware | Card::Stick if !self.restart_holds().is_empty() => {}
                Card::Firmware => {
                    self.restart_failed = None;
                    self.sheet = Some(Sheet::Restart(RestartTo::Firmware));
                }
                Card::Stick => self.sheet = Some(Sheet::Restart(RestartTo::Stick)),
                Card::StartMac => self.go(Page::Start),
            },
            StarterAction::Item(item) => match item {
                Item::MakeStick => self.act(StarterAction::Make(Target::Stick)),
                Item::Start | Item::SecureBootOn => self.go(Page::Start),
            },
            StarterAction::Disk(n) => {
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
                self.version_failed = None;
                self.go(Page::Mode);
            }
            StarterAction::Mode(mode) => {
                if self.page == Page::Mode
                    && self.mode_refusal(mode).is_none()
                    && let Some(m) = self.making.as_mut()
                {
                    m.mode = Some(mode);
                    self.version_failed = None;
                    if mode == Mode::KeepData {
                        self.read_version();
                    }
                }
            }
            StarterAction::Continue => {
                let Some(disk) = self.chosen_disk().cloned() else {
                    return;
                };
                let ready = self
                    .making
                    .as_ref()
                    .and_then(|m| m.mode)
                    .is_some_and(|mode| self.mode_refusal(mode).is_none())
                    && !self.keep_unread();
                if self.page == Page::Mode
                    && ready
                    && let Some(m) = self.making.as_mut()
                {
                    m.confirmed = Some((disk.id, disk.device, disk.serial, disk.size));
                    m.refused = None;
                    self.go(Page::Confirm);
                }
            }
            StarterAction::Write => self.write(),
            StarterAction::Again => {
                if let Some(target) = self.making.as_ref().map(|m| m.target) {
                    self.act(StarterAction::Make(target));
                }
            }
            StarterAction::Start => self.go(Page::Start),
            StarterAction::Restart => {
                if self.page == Page::Start
                    && self.facts().mac().is_none()
                    && !self.facts().protected_open()
                    && self.restart_holds().is_empty()
                {
                    self.restart_failed = None;
                    self.sheet = Some(Sheet::Restart(RestartTo::Firmware));
                }
            }
            StarterAction::RestartNow if !self.restart_holds().is_empty() => self.sheet = None,
            StarterAction::RestartNow => match self.sheet {
                Some(Sheet::Restart(RestartTo::Firmware)) => {
                    self.sheet = None;
                    self.out.push_back(StarterCommand::RestartIntoFirmware);
                }
                Some(Sheet::Restart(RestartTo::Stick)) => {
                    self.sheet = None;
                    // A stick pulled out while the sheet was up: nothing
                    // to start, and the card says so by offering Make.
                    if let Some(d) = self.facts().stick() {
                        let device = d.device.clone();
                        self.out
                            .push_back(StarterCommand::RestartFromStick { device });
                    }
                }
                _ => {}
            },
            StarterAction::Cancel => {
                self.sheet = None;
                self.asking = None;
            }
            StarterAction::Install => {
                self.install_result = None;
                self.go(Page::Install);
            }
            StarterAction::InstallNow => {
                if self.page == Page::Install && !self.installing {
                    self.installing = true;
                    self.install_result = None;
                    self.out.push_back(StarterCommand::Install);
                }
            }
            StarterAction::Installed => {
                self.uninstall_failed = None;
                self.entry_kept = None;
                self.go(Page::Installed);
            }
            StarterAction::Uninstall => {
                if self.page == Page::Installed && self.installed.is_some() {
                    self.sheet = Some(Sheet::Uninstall);
                }
            }
            StarterAction::UninstallNow => {
                if self.sheet == Some(Sheet::Uninstall) {
                    self.sheet = None;
                    self.entry_kept = None;
                    self.out.push_back(StarterCommand::Uninstall);
                }
            }
            StarterAction::Transfer => self.show_held(None),
            StarterAction::Learn => {
                let pages = self.learn_pages();
                self.show_held(Some(pages));
            }
            StarterAction::LearnPage(id) => {
                if let Some(page) = id.page() {
                    self.show_held(Some(vec![page]));
                }
            }
            StarterAction::About => self.go(Page::About),
            StarterAction::Templates => {
                self.template.saved = None;
                self.template.open_failed = None;
                self.go(Page::Templates);
            }
            StarterAction::TemplateChoice(c) if self.page == Page::Templates => self.go(match c {
                TemplateChoice::Words => Page::TemplateWords,
                TemplateChoice::Copy => Page::TemplateCopy,
                TemplateChoice::Places => Page::TemplatePlaces,
            }),
            StarterAction::TemplateWords(n) if self.page == Page::TemplateWords => {
                if n == 12 || n == 24 {
                    self.template.words = n;
                    self.template.saved = None;
                }
            }
            StarterAction::TemplateCopy(c) if self.page == Page::TemplateCopy => {
                self.template.copy = c;
                self.template.saved = None;
            }
            StarterAction::PlacesFewer if self.page == Page::TemplatePlaces => {
                self.template.places = self.template.places.saturating_sub(1).max(1);
                self.template.saved = None;
            }
            StarterAction::PlacesMore if self.page == Page::TemplatePlaces => {
                self.template.places = self.template.places.saturating_add(1);
                self.template.saved = None;
            }
            StarterAction::PrintTemplate if self.page == Page::Templates => {
                if let Ok(pdf) = self.template.pdf(&c) {
                    self.template.open_failed = None;
                    self.out.push_back(StarterCommand::PrintTemplate {
                        name: self.template.file_name(),
                        pdf,
                    });
                }
            }
            StarterAction::SaveTemplate if self.page == Page::Templates => {
                if !self.template.saving
                    && let Ok(pdf) = self.template.pdf(&c)
                {
                    self.template.saving = true;
                    self.template.saved = None;
                    self.template.open_failed = None;
                    self.out.push_back(StarterCommand::SaveTemplate {
                        name: self.template.file_name(),
                        pdf,
                    });
                }
            }
            StarterAction::OpenTemplate if self.page == Page::Templates => {
                if let Some(Ok(path)) = &self.template.saved {
                    self.template.open_failed = None;
                    self.out
                        .push_back(StarterCommand::OpenTemplate { path: path.clone() });
                }
            }
            StarterAction::PrintNotes if self.page == Page::Start => {
                if self.include_key && self.bitlocker_offered() {
                    // The key is read through the elevated copy first; the
                    // sheet is made when it arrives.
                    if !self.key_reading {
                        self.key_reading = true;
                        self.notes_open_failed = None;
                        self.out.push_back(StarterCommand::ReadRecoveryKey);
                    }
                } else {
                    self.print_notes(None);
                }
            }
            StarterAction::IncludeKey if self.page == Page::Start && self.bitlocker_offered() => {
                self.include_key = !self.include_key;
            }
            StarterAction::SuspendBitLocker if self.step_open(BitLockerStep::Suspend) => {
                self.bitlocker_step = Some(BitLockerStep::Suspend);
                self.bitlocker_failed = None;
                self.out
                    .push_back(StarterCommand::BitLocker(BitLockerStep::Suspend));
            }
            StarterAction::ResumeBitLocker if self.step_open(BitLockerStep::Resume) => {
                self.bitlocker_step = Some(BitLockerStep::Resume);
                self.bitlocker_failed = None;
                self.out
                    .push_back(StarterCommand::BitLocker(BitLockerStep::Resume));
            }
            StarterAction::Rescue => {
                if self.page == Page::Start {
                    self.other_confirmed = true;
                }
            }
            StarterAction::CheckAgain => self.out.push_back(StarterCommand::CheckAgain),
            StarterAction::ContinueAnyway(n) => {
                let Some(id) = self.concern_seen.get(usize::from(n)).cloned() else {
                    return;
                };
                // A virtual machine is never let through.
                let open = self.concerns.iter().any(|c| {
                    c.id == id
                        && c.kind != Kind::VirtualMachine
                        && !self.let_through.contains(&c.id)
                });
                if open {
                    self.asking = Some(id);
                    self.sheet = Some(Sheet::LetThrough);
                }
            }
            StarterAction::LetThrough => {
                if self.sheet == Some(Sheet::LetThrough) {
                    self.sheet = None;
                    if let Some(id) = self.asking.take() {
                        self.let_through.push(id);
                    }
                }
            }
            StarterAction::RemoveEntry => {
                if self.page == Page::Home && self.entry_offered() && !self.entry_removing {
                    self.entry_removing = true;
                    self.entry_remove_failed = None;
                    self.out.push_back(StarterCommand::RemoveBootEntry);
                }
            }
            // A template action, or the Start page's Print, pressed on
            // another page does nothing.
            StarterAction::PrintNotes
            | StarterAction::IncludeKey
            | StarterAction::SuspendBitLocker
            | StarterAction::ResumeBitLocker
            | StarterAction::TemplateChoice(_)
            | StarterAction::TemplateWords(_)
            | StarterAction::TemplateCopy(_)
            | StarterAction::PlacesFewer
            | StarterAction::PlacesMore
            | StarterAction::PrintTemplate
            | StarterAction::SaveTemplate
            | StarterAction::OpenTemplate => {}
        }
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
        if self.no_writer || self.mode_refusal(mode).is_some() || self.keep_unread() {
            return;
        }
        self.out.push_back(StarterCommand::Write(WriteOrder {
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
            Page::Disks
                if self.asks_which()
                    && self
                        .making
                        .as_ref()
                        .is_some_and(|m| m.target == Target::Stick) =>
            {
                Page::Which
            }
            // A write under way goes on: its page stays.
            Page::Writing => return,
            Page::TemplateWords | Page::TemplateCopy | Page::TemplatePlaces => Page::Templates,
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
        // The pages are a column that lays out to the width: they draw at
        // the desktop's density at every window size, and a window under
        // 600 dp wide gets the one-column layout.
        let density = f32::from(d.dpi.max(1)) / 160.0;
        self.compact = f32::from(d.width) / density < 600.0;
        let f = density.max(0.5);
        let scaled = DisplayInfo {
            dpi: (160.0 * f).round().max(1.0) as u16,
            ..d
        };
        self.f = f;
        self.w = f32::from(d.width) / f;
        self.h = f32::from(d.height) / f;
        let mut canvas = Canvas::new(&scaled);
        // The canvas notes its pixel work, a blur or a whole copy, with
        // the held app's counter (`docs/COMPUTE.md` §2.2), for the
        // budget test.
        canvas.set_counter(faraday_core::work::core_count);
        self.canvas = Some(canvas);
        self.frost = None;
    }

    fn render(&mut self) {
        let Some(mut canvas) = self.canvas.take() else {
            return;
        };
        let mut hits = std::mem::take(&mut self.hits);
        hits.clear();
        self.offset = self.offset.clamp(0.0, self.max.max(0.0));
        let max = {
            let mut ui = faraday_ui::Ui::new(&mut canvas, self.f, &mut hits, self.pressed);
            ui.hovered = self.hovered;
            ui.compact = self.compact;
            ui.offset = self.offset;
            ui.frost = self.sheet.and(self.frost.take());
            let max = draw::frame(self, &mut ui);
            self.frost = ui.frost.take();
            max
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

    /// A contact at (x, y): whether the frame changes.
    fn touch(&mut self, x: i32, y: i32, phase: TouchPhase) -> bool {
        match phase {
            TouchPhase::Down => self.pressed = self.hit_at(x, y),
            TouchPhase::Move => {
                // A move that keeps the press as it was changes nothing.
                if self.pressed.is_none() || self.hit_at(x, y) == self.pressed {
                    return false;
                }
                self.pressed = None;
            }
            TouchPhase::Up => {
                let pressed = self.pressed.take();
                if let Some(a) = pressed
                    && self.hit_at(x, y) == Some(a)
                    && let Action::Host(code) = a
                    && let Some(action) = StarterAction::from_code(code)
                {
                    self.act(action);
                }
            }
        }
        true
    }

    /// After an event the held app took: once it leaves Transfer and its
    /// sheets, the Starter's page it was opened from is shown again (Home,
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
    /// Windows with another encryption product not yet confirmed: the
    /// firmware steps' buttons are not offered (§3.4, §6.4), until the
    /// person says they have its rescue disk or password.
    pub fn protected_open(&self) -> bool {
        self.computer.os == Os::Windows && self.other_open
    }
}

impl App for Starter {
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
                if !self.touch(i32::from(x), i32::from(y), phase) {
                    return;
                }
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
                let offset =
                    (self.offset + f32::from(dy) / self.f.max(0.1)).clamp(0.0, self.max.max(0.0));
                // At an end, a turn further moves nothing.
                if offset == self.offset {
                    return;
                }
                self.offset = offset;
                self.frost = None;
            }
            Event::Back if !self.showing => self.act(StarterAction::Back),
            Event::Key(Key::Escape) if !self.showing => self.act(StarterAction::Back),
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
