//! Faraday Companion (`docs/COMPANION.md`): the phone app's state,
//! screens and decisions, with no I/O. It is an [`osk_shell_api::App`],
//! as `faraday-core`'s `Faraday` is: the shell sends input and display
//! events through [`App::event`], reads the frame, and takes upstream's
//! commands from [`App::poll_command`]. What upstream's interface has no
//! event for goes through a second channel beside it ([`api`]): the
//! shell reports through [`Companion::companion_event`] and takes what
//! the core asks for from [`Companion::poll_companion`].
//!
//! Its screens are `osk-ui`'s §5 screens on the `mobile` class, portrait,
//! as `opensigner-core`'s views are. It holds no part of Faraday's vault,
//! signer or key loaders (§3): a seed, a private key or a sealed file that
//! arrives is refused by [`route`], and nothing of it is kept.

pub mod api;
pub mod route;
pub mod store;
mod view;

use std::collections::VecDeque;
use std::rc::Rc;

use faraday_intake::Secret;
use faraday_qr::bitlocker::KeyText;
use faraday_qr::send::{Format, Made, PARTS, SPEEDS, Source};
use osk_codec::QrMatrix;
use osk_shell_api::{App, Command, DisplayInfo, Event, FileKind, Frame};
use osk_ui::components::Preview;
use osk_ui::layout::{self, Layout, LayoutCtx, Node};
use osk_ui::{Action, Canvas, Id, Scale, SizeClass, Theme, UiState};
use zeroize::{Zeroize, Zeroizing};

pub use api::{CompanionCommand, CompanionEvent};
use route::{Outcome, Reader};
use store::{Kept, Kind};

/// The Companion's own version, which About shows.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where a page of text comes from: one of `osk-learn`'s pages, by its
/// place in `osk_learn::EN.pages()`, or one of Faraday's own, by id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doc {
    /// `osk-learn`'s page.
    Osk(usize),
    /// Faraday's own page (`faraday_learn`).
    Own(&'static str),
}

/// Text that arrived and is shown, not kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    /// Any other text, with Keep.
    Text(String),
    /// An address, with its kind.
    Address {
        /// The address.
        text: String,
        /// A silent payment address.
        silent: bool,
    },
    /// A web address, never opened.
    Web(String),
}

/// Why a file was not shown as codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotSent {
    /// Over the envelope's 256 KiB.
    TooLarge,
    /// Nothing in it.
    Empty,
    /// No code holds it in the format asked.
    NoCode,
    /// The shell has no document picker.
    NoPicker,
}

/// What a deletion asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirm {
    /// One kept file, by its SHA-256.
    One([u8; 32]),
    /// Every kept file.
    All,
    /// The store that could not be read.
    Store,
}

/// A page of the Companion (`docs/COMPANION.md` §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    /// Home (§4.1).
    Home,
    /// Receive from Faraday: the scanner (§4.2).
    Receive,
    /// A whole file arrived and is kept, by its SHA-256; `again` when it
    /// was kept already.
    Arrived {
        /// The file.
        sha256: [u8; 32],
        /// It was in Received before.
        again: bool,
    },
    /// A secret that stays on the device was refused (§5).
    Refused(Secret),
    /// Text, an address or a web address, shown.
    Shown(Shown),
    /// A BitLocker recovery key, shown masked and kept nowhere.
    BitLocker,
    /// Send to Faraday: the file to send (§4.3).
    SendList,
    /// Send to Faraday: the codes.
    Send,
    /// A file that could not go as codes.
    NotSent(NotSent),
    /// Received (§4.4).
    Received,
    /// A kept file's record, by its SHA-256.
    Record([u8; 32]),
    /// A deletion, asked once.
    Confirm(Confirm),
    /// Learn: every page (§4.7).
    Learn,
    /// Start from the stick: the boot pages (§4.7).
    Start,
    /// One page.
    Doc(Doc),
    /// About (§4.8).
    About,
}

/// Everything a press can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// The app bar's chevron.
    Back,
    /// Home's rows.
    Receive,
    /// Home's Send to Faraday.
    Send,
    /// Home's Received.
    Received,
    /// Home's Start from the stick.
    Start,
    /// Home's Learn.
    Learn,
    /// Home's About.
    About,
    /// The scanner's Paste.
    Paste,
    /// Back to the scanner.
    ScanAgain,
    /// Opens the file that arrived.
    Open,
    /// Keeps the text shown.
    Keep,
    /// Closes what is shown.
    Done,
    /// The document picker.
    Choose,
    /// Sends kept file `n`, newest first.
    SendKept(usize),
    /// The Animated switch.
    Animated,
    /// The next format.
    Format,
    /// The next frame rate.
    Rate,
    /// Opens kept file `n`, newest first.
    Record(usize),
    /// The record's Show as QR.
    ShowAsQr,
    /// The record's Save.
    Save,
    /// The record's Delete.
    Delete,
    /// Received's Delete all.
    DeleteAll,
    /// Received's way to delete a store it cannot read.
    DeleteStore,
    /// Yes to a deletion.
    ConfirmDelete,
    /// No to a deletion.
    KeepIt,
    /// Learn's page `n`.
    LearnPage(usize),
    /// Start from the stick's page `n`.
    StartPage(usize),
    /// The eye over the recovery key.
    Eye,
}

const BASE_RECORD: u32 = 1000;
const BASE_SEND: u32 = 2000;
const BASE_LEARN: u32 = 3000;
const BASE_START: u32 = 4000;
const SPAN: u32 = 1000;

impl Act {
    /// The id the screens give it.
    pub fn id(self) -> Id {
        Id(match self {
            Act::Back => 1,
            Act::Receive => 2,
            Act::Send => 3,
            Act::Received => 4,
            Act::Start => 5,
            Act::Learn => 6,
            Act::About => 7,
            Act::Paste => 8,
            Act::ScanAgain => 9,
            Act::Open => 10,
            Act::Keep => 11,
            Act::Done => 12,
            Act::Choose => 13,
            Act::Animated => 14,
            Act::Format => 15,
            Act::Rate => 16,
            Act::ShowAsQr => 17,
            Act::Save => 18,
            Act::Delete => 19,
            Act::DeleteAll => 20,
            Act::DeleteStore => 21,
            Act::ConfirmDelete => 22,
            Act::KeepIt => 23,
            Act::Eye => 24,
            Act::Record(n) => BASE_RECORD + n.min(SPAN as usize - 1) as u32,
            Act::SendKept(n) => BASE_SEND + n.min(SPAN as usize - 1) as u32,
            Act::LearnPage(n) => BASE_LEARN + n.min(SPAN as usize - 1) as u32,
            Act::StartPage(n) => BASE_START + n.min(SPAN as usize - 1) as u32,
        })
    }

    /// The act a screen's id stands for.
    pub fn of(id: Id) -> Option<Act> {
        let n = id.0;
        let at = |base: u32| (n - base) as usize;
        Some(match n {
            1 => Act::Back,
            2 => Act::Receive,
            3 => Act::Send,
            4 => Act::Received,
            5 => Act::Start,
            6 => Act::Learn,
            7 => Act::About,
            8 => Act::Paste,
            9 => Act::ScanAgain,
            10 => Act::Open,
            11 => Act::Keep,
            12 => Act::Done,
            13 => Act::Choose,
            14 => Act::Animated,
            15 => Act::Format,
            16 => Act::Rate,
            17 => Act::ShowAsQr,
            18 => Act::Save,
            19 => Act::Delete,
            20 => Act::DeleteAll,
            21 => Act::DeleteStore,
            22 => Act::ConfirmDelete,
            23 => Act::KeepIt,
            24 => Act::Eye,
            _ if (BASE_RECORD..BASE_RECORD + SPAN).contains(&n) => Act::Record(at(BASE_RECORD)),
            _ if (BASE_SEND..BASE_SEND + SPAN).contains(&n) => Act::SendKept(at(BASE_SEND)),
            _ if (BASE_LEARN..BASE_LEARN + SPAN).contains(&n) => Act::LearnPage(at(BASE_LEARN)),
            _ if (BASE_START..BASE_START + SPAN).contains(&n) => Act::StartPage(at(BASE_START)),
            _ => return None,
        })
    }
}

/// What the scanner says inside its square.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanLine {
    /// No frame yet.
    Waiting,
    /// Frames come.
    Looking,
    /// No camera.
    NoCamera,
    /// A transfer in parts.
    Part {
        /// Parts read.
        have: usize,
        /// Parts in all.
        total: usize,
        /// The parts still missing, where the transport numbers them.
        missing: Vec<usize>,
    },
    /// A code that could not be read.
    NotRead,
    /// Paste found no text.
    NothingToPaste,
}

/// A file shown as codes (§4.3).
pub struct Sending {
    /// The file's name.
    pub name: String,
    /// What is sent.
    pub source: Source,
    /// How it is written.
    pub format: Format,
    /// Shown in turn rather than as one code.
    pub animated: bool,
    /// Animated because one code does not hold it.
    pub forced: bool,
    /// Whether the switch can turn animation on.
    pub can_animate: bool,
    /// What the codes are.
    pub made: Made,
    /// The codes.
    pub frames: Vec<Rc<QrMatrix>>,
    /// The code on show.
    pub frame: usize,
    next_ms: u64,
}

impl Drop for Sending {
    fn drop(&mut self) {
        match &mut self.source {
            Source::Psbt(b) | Source::File(_, b) => b.zeroize(),
            Source::Text(t) => t.zeroize(),
            Source::Key(k, a) => {
                k.zeroize();
                a.zeroize();
            }
        }
    }
}

impl Sending {
    fn new(name: String, source: Source) -> Result<Sending, NotSent> {
        let mut s = Sending {
            name,
            source,
            format: Format::Ur,
            animated: false,
            forced: false,
            can_animate: false,
            made: Made::TextOne,
            frames: Vec::new(),
            frame: 0,
            next_ms: 0,
        };
        s.make()?;
        Ok(s)
    }

    /// The codes for the format and the switch as they stand.
    fn make(&mut self) -> Result<(), NotSent> {
        let part = PARTS[1];
        let not_sent = |e: faraday_qr::send::Error| match e {
            faraday_qr::send::Error::Code(_) => NotSent::NoCode,
            _ => NotSent::TooLarge,
        };
        let one = faraday_qr::send::one_code(&self.source, self.format);
        let many =
            faraday_qr::send::animated_frames(&self.source, self.format, part).map_err(not_sent)?;
        self.can_animate = many.1.len() > 1;
        self.forced = one.is_none();
        let (made, frames) = match one {
            Some(one) if !(self.animated && self.can_animate) => one,
            _ => many,
        };
        self.animated = frames.len() > 1;
        self.made = made;
        self.frames = frames.into_iter().map(Rc::new).collect();
        self.frame = 0;
        self.next_ms = 0;
        Ok(())
    }

    /// The formats this source can go in, in the order offered.
    pub fn formats(&self) -> Vec<Format> {
        Format::ALL
            .into_iter()
            .filter(|f| *f != Format::Parts || self.source.is_text())
            .collect()
    }

    /// The format after this one.
    pub fn next_format(&self) -> Format {
        let all = self.formats();
        let at = all.iter().position(|f| *f == self.format).unwrap_or(0);
        all[(at + 1) % all.len()]
    }
}

/// The phone's wall clock, as the shell last gave it.
#[derive(Debug, Clone, Copy)]
struct Clock {
    unix_s: u64,
    offset_s: i32,
    at_ms: u64,
}

/// About's facts from the system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System {
    /// The version name and code.
    pub build: String,
    /// The signing certificate's SHA-256.
    pub certificate_sha256: [u8; 32],
    /// The permissions held.
    pub permissions: Vec<String>,
}

/// The Companion.
pub struct Companion {
    display: Option<DisplayInfo>,
    canvas: Option<Canvas>,
    ctx: Option<LayoutCtx>,
    ui: Option<UiState>,
    theme: Theme,
    tree: Option<Node>,
    layout: Option<Layout>,
    dirty: bool,
    commands: VecDeque<Command>,
    out: VecDeque<CompanionCommand>,
    /// The page on show.
    pub page: Page,
    stack: Vec<Page>,
    now_ms: u64,
    clock: Option<Clock>,
    /// The kept files, oldest first.
    kept: Vec<Kept>,
    /// The store the shell handed back could not be read: it is not
    /// written over until the person deletes it.
    pub unreadable: bool,
    reader: Reader,
    /// What the scanner says.
    pub scan_line: ScanLine,
    preview: Option<Preview>,
    camera: bool,
    screen_on: bool,
    pasting: bool,
    picking: bool,
    bitlocker: Option<KeyText>,
    revealed: bool,
    /// The file shown as codes.
    pub sending: Option<Sending>,
    /// Milliseconds a frame of an animated code.
    pub speed_ms: u64,
    saved: Option<([u8; 32], bool)>,
    /// About's facts from the system, once reported.
    pub system: Option<System>,
}

impl Default for Companion {
    fn default() -> Self {
        Companion::new()
    }
}

impl Companion {
    /// A Companion on Home, with nothing kept.
    pub fn new() -> Companion {
        Companion {
            display: None,
            canvas: None,
            ctx: None,
            ui: None,
            theme: Theme::DARK,
            tree: None,
            layout: None,
            dirty: false,
            commands: VecDeque::new(),
            out: VecDeque::new(),
            page: Page::Home,
            stack: Vec::new(),
            now_ms: 0,
            clock: None,
            kept: Vec::new(),
            unreadable: false,
            reader: Reader::default(),
            scan_line: ScanLine::Waiting,
            preview: None,
            camera: false,
            screen_on: false,
            pasting: false,
            picking: false,
            bitlocker: None,
            revealed: false,
            sending: None,
            speed_ms: SPEEDS[0],
            saved: None,
            system: None,
        }
    }

    /// The kept files, newest first, as Received lists them.
    pub fn received(&self) -> Vec<&Kept> {
        self.kept.iter().rev().collect()
    }

    /// The store as the core would write it now.
    pub fn store_bytes(&self) -> Vec<u8> {
        store::write(&self.kept)
    }

    /// The BitLocker recovery key on show, kept nowhere else.
    pub fn bitlocker(&self) -> Option<&KeyText> {
        self.bitlocker.as_ref()
    }

    /// One report from the shell beyond upstream's events.
    pub fn companion_event(&mut self, event: CompanionEvent) {
        match event {
            CompanionEvent::Time { unix_s, offset_s } => {
                self.clock = Some(Clock {
                    unix_s,
                    offset_s,
                    at_ms: self.now_ms,
                });
            }
            CompanionEvent::Received { bytes } => match store::read(&bytes) {
                Some(files) => {
                    // Anything that arrived before the store came back
                    // stays, after what was kept before.
                    let arrived = std::mem::take(&mut self.kept);
                    self.kept = files;
                    let mut grew = false;
                    for f in arrived {
                        if !self.kept.iter().any(|k| k.sha256 == f.sha256) {
                            self.kept.push(f);
                            grew = true;
                        }
                    }
                    self.unreadable = false;
                    if grew {
                        self.write_store();
                    }
                }
                None => self.unreadable = true,
            },
            CompanionEvent::System {
                build,
                certificate_sha256,
                permissions,
            } => {
                self.system = Some(System {
                    build,
                    certificate_sha256,
                    permissions,
                });
            }
        }
        self.render();
    }

    /// The next thing the core asks of the shell beyond upstream's
    /// commands.
    pub fn poll_companion(&mut self) -> Option<CompanionCommand> {
        self.out.pop_front()
    }

    /// A press, as a touch on it would make.
    pub fn press(&mut self, act: Act) {
        self.act(act);
        self.render();
    }

    /// Whether the screen on show offers `act`.
    pub fn offers(&self, act: Act) -> bool {
        let id = act.id();
        self.layout
            .as_ref()
            .is_some_and(|l| l.items.iter().any(|p| p.id == Some(id) && p.hit.is_some()))
    }

    /// Where the screen on show offers `act`, in pixels: the middle of
    /// it, for a test that touches it.
    pub fn where_offered(&self, act: Act) -> Option<(u16, u16)> {
        let id = act.id();
        let p = self
            .layout
            .as_ref()?
            .items
            .iter()
            .find(|p| p.id == Some(id) && p.hit.is_some())?;
        let r = p.rect;
        Some(((r.x + r.w / 2) as u16, (r.y + r.h / 2) as u16))
    }

    /// Every string the screen on show draws, in tree order.
    pub fn texts(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(tree) = &self.tree {
            view::texts(tree, &mut out);
        }
        out
    }

    /// The wall-clock time now, seconds since the epoch, or 0 before the
    /// shell gave it.
    fn wall_now(&self) -> u64 {
        self.clock
            .map_or(0, |c| c.unix_s + self.now_ms.saturating_sub(c.at_ms) / 1000)
    }

    /// The phone's offset from UTC, in seconds.
    fn offset_s(&self) -> i32 {
        self.clock.map_or(0, |c| c.offset_s)
    }

    /// Opens `page` over the one on show.
    fn go(&mut self, page: Page) {
        let was = std::mem::replace(&mut self.page, page);
        self.stack.push(was);
    }

    /// Shows `page` in place of the one on show.
    fn replace(&mut self, page: Page) {
        self.page = page;
    }

    /// Back one page.
    fn back(&mut self) {
        self.page = self.stack.pop().unwrap_or(Page::Home);
        self.leaving();
    }

    /// What a page left behind is wiped of.
    fn leaving(&mut self) {
        if self.page != Page::BitLocker {
            self.bitlocker = None;
            self.revealed = false;
        }
        if self.page != Page::Send {
            self.sending = None;
        }
    }

    fn act(&mut self, act: Act) {
        match act {
            Act::Back | Act::Done | Act::KeepIt => self.back(),
            Act::Receive => {
                self.go(Page::Receive);
                self.open_scanner();
            }
            Act::ScanAgain => {
                self.replace(Page::Receive);
                self.leaving();
                self.open_scanner();
            }
            Act::Send => self.go(Page::SendList),
            Act::Received => self.go(Page::Received),
            Act::Start => self.go(Page::Start),
            Act::Learn => self.go(Page::Learn),
            Act::About => self.go(Page::About),
            Act::Paste => {
                if self.page == Page::Receive {
                    self.pasting = true;
                    self.commands.push_back(Command::RequestClipboard {
                        kind: FileKind::Text,
                    });
                }
            }
            Act::Open => {
                if let Page::Arrived { sha256, .. } = self.page {
                    self.go(Page::Record(sha256));
                }
            }
            Act::Keep => {
                let text = match &self.page {
                    Page::Shown(Shown::Text(t)) => Some(t.as_bytes().to_vec()),
                    _ => None,
                };
                if let Some(bytes) = text {
                    let (sha256, again) = self.keep(None, Kind::Text, bytes, "txt");
                    self.replace(Page::Arrived { sha256, again });
                }
            }
            Act::Choose => {
                if self.page == Page::SendList {
                    self.picking = true;
                    self.commands.push_back(Command::RequestFile {
                        kind: FileKind::Any,
                    });
                }
            }
            Act::SendKept(n) => {
                let file = self
                    .received()
                    .get(n)
                    .map(|k| (k.name.clone(), k.kind, k.bytes.clone()));
                if let Some((name, kind, bytes)) = file {
                    self.send(name, kind, bytes);
                }
            }
            Act::Record(n) => {
                let sha = self.received().get(n).map(|k| k.sha256);
                if let Some(sha) = sha {
                    self.saved = None;
                    self.go(Page::Record(sha));
                }
            }
            Act::ShowAsQr => {
                let file = match self.page {
                    Page::Record(sha) => self
                        .kept
                        .iter()
                        .find(|k| k.sha256 == sha)
                        .map(|k| (k.name.clone(), k.kind, k.bytes.clone())),
                    _ => None,
                };
                if let Some((name, kind, bytes)) = file {
                    self.send(name, kind, bytes);
                }
            }
            Act::Save => {
                if let Page::Record(sha) = self.page
                    && let Some(k) = self.kept.iter().find(|k| k.sha256 == sha)
                {
                    let kind = match k.kind {
                        Kind::Psbt => FileKind::Psbt,
                        Kind::Text | Kind::Description | Kind::Transaction => FileKind::Text,
                        Kind::File => FileKind::Any,
                    };
                    self.saved = None;
                    self.commands.push_back(Command::WriteFile {
                        kind,
                        name_hint: k.name.clone(),
                        bytes: k.bytes.clone(),
                    });
                }
            }
            Act::Delete => {
                if let Page::Record(sha) = self.page {
                    self.go(Page::Confirm(Confirm::One(sha)));
                }
            }
            Act::DeleteAll => {
                if !self.kept.is_empty() {
                    self.go(Page::Confirm(Confirm::All));
                }
            }
            Act::DeleteStore => {
                if self.unreadable {
                    self.go(Page::Confirm(Confirm::Store));
                }
            }
            Act::ConfirmDelete => {
                if let Page::Confirm(c) = self.page {
                    self.delete(c);
                }
            }
            Act::LearnPage(n) => {
                if let Some(d) = view::learn_docs().get(n) {
                    self.go(Page::Doc(*d));
                }
            }
            Act::StartPage(n) => {
                if let Some(d) = view::start_docs().get(n) {
                    self.go(Page::Doc(*d));
                }
            }
            Act::Eye => self.revealed = !self.revealed,
            Act::Animated => {
                if let Some(s) = self.sending.as_mut()
                    && !s.forced
                    && s.can_animate
                {
                    s.animated = !s.animated;
                    let _ = s.make();
                }
            }
            Act::Format => {
                if let Some(s) = self.sending.as_mut() {
                    let was = s.format;
                    s.format = s.next_format();
                    if s.make().is_err() {
                        s.format = was;
                        let _ = s.make();
                    }
                }
            }
            Act::Rate => {
                let at = SPEEDS.iter().position(|s| *s == self.speed_ms).unwrap_or(0);
                self.speed_ms = SPEEDS[(at + 1) % SPEEDS.len()];
            }
        }
        self.sync();
    }

    /// Deletes what `c` names; the store is written after.
    fn delete(&mut self, c: Confirm) {
        match c {
            Confirm::One(sha) => {
                if let Some(at) = self.kept.iter().position(|k| k.sha256 == sha) {
                    let mut k = self.kept.remove(at);
                    k.bytes.zeroize();
                }
                // The confirmation, then the record it was asked from;
                // a page that showed the file shows Received instead.
                self.stack.pop();
                self.page = self.stack.pop().unwrap_or(Page::Received);
                if matches!(self.page, Page::Arrived { sha256, .. } if sha256 == sha) {
                    self.page = Page::Received;
                }
            }
            Confirm::All => {
                for k in &mut self.kept {
                    k.bytes.zeroize();
                }
                self.kept.clear();
                self.page = self.stack.pop().unwrap_or(Page::Received);
            }
            Confirm::Store => {
                self.unreadable = false;
                self.page = self.stack.pop().unwrap_or(Page::Received);
            }
        }
        self.write_store();
    }

    /// The scanner, fresh: the camera on, nothing part-way through.
    fn open_scanner(&mut self) {
        self.reader.reset();
        self.scan_line = ScanLine::Waiting;
        self.preview = None;
    }

    /// `bytes` of `kind`, shown as codes.
    fn send(&mut self, name: String, kind: Kind, bytes: Vec<u8>) {
        let source = match kind {
            Kind::Psbt => match route::psbt_bytes(&bytes) {
                Some(p) => Source::Psbt(p),
                None => Source::File(name.clone(), bytes),
            },
            Kind::Description => Source::Text(String::from_utf8_lossy(&bytes).into_owned()),
            _ if bytes.is_empty() => {
                self.go(Page::NotSent(NotSent::Empty));
                return;
            }
            _ if bytes.len() > faraday_qr::envelope::MAX_FILE => {
                self.go(Page::NotSent(NotSent::TooLarge));
                return;
            }
            _ => Source::File(name.clone(), bytes),
        };
        match Sending::new(name, source) {
            Ok(s) => {
                self.sending = Some(s);
                self.go(Page::Send);
            }
            Err(why) => self.go(Page::NotSent(why)),
        }
    }

    /// A file the person picked: refused whole when it holds a secret,
    /// and otherwise shown as codes.
    fn picked(&mut self, bytes: Vec<u8>) {
        let mut bytes = bytes;
        if let Some(s) = faraday_intake::secret(&bytes) {
            bytes.zeroize();
            self.go(Page::Refused(s));
            return;
        }
        let kind = route::kind_of_file(&bytes);
        let ext = match kind {
            Kind::Psbt => "psbt",
            Kind::Description | Kind::Text | Kind::Transaction => "txt",
            Kind::File => "bin",
        };
        let name = faraday_strings::fill(faraday_strings::EN.companion_send_picked, &[ext]);
        self.send(name, kind, bytes);
    }

    /// Keeps a whole file in Received, once by its SHA-256. Returns its
    /// SHA-256 and whether it was there already.
    fn keep(
        &mut self,
        name: Option<String>,
        kind: Kind,
        bytes: Vec<u8>,
        ext: &str,
    ) -> ([u8; 32], bool) {
        let sha = store::digest(&bytes);
        if self.kept.iter().any(|k| k.sha256 == sha) {
            return (sha, true);
        }
        let taken = |n: &str| self.kept.iter().any(|k| k.name == n);
        let name = match name {
            Some(n) if !taken(&n) => n,
            Some(n) => {
                let (stem, ext) = n.rsplit_once('.').unwrap_or((&n, ""));
                (2..)
                    .map(|i| {
                        if ext.is_empty() {
                            format!("{stem}-{i}")
                        } else {
                            format!("{stem}-{i}.{ext}")
                        }
                    })
                    .find(|c| !taken(c))
                    .unwrap_or(n)
            }
            // `received-{n}`, numbered past every file kept so named,
            // whatever its extension.
            None => (1..)
                .find(|i| {
                    let stem = format!("received-{i}.");
                    !self.kept.iter().any(|k| k.name.starts_with(&stem))
                })
                .map(|i| format!("received-{i}.{ext}"))
                .unwrap_or_default(),
        };
        let time = self.wall_now();
        self.kept.push(Kept::new(name, kind, time, bytes));
        self.write_store();
        (sha, false)
    }

    /// Hands the shell the whole store, unless the store it holds could
    /// not be read and has not been deleted.
    fn write_store(&mut self) {
        if !self.unreadable {
            self.out.push_back(CompanionCommand::KeepReceived {
                bytes: store::write(&self.kept),
            });
        }
    }

    /// One code read by the camera, or one paste, routed (§5).
    fn arrive(&mut self, bytes: Zeroizing<Vec<u8>>) {
        let outcome = self.reader.read(&bytes);
        drop(bytes);
        match outcome {
            Outcome::Part {
                have,
                total,
                missing,
            } => {
                self.scan_line = ScanLine::Part {
                    have,
                    total,
                    missing,
                }
            }
            Outcome::Refused(s) => self.replace(Page::Refused(s)),
            Outcome::Arrived {
                name,
                kind,
                bytes,
                ext,
            } => {
                let (sha256, again) = self.keep(name, kind, bytes, ext);
                self.replace(Page::Arrived { sha256, again });
            }
            Outcome::BitLocker(key) => {
                self.bitlocker = Some(key);
                self.revealed = false;
                self.replace(Page::BitLocker);
            }
            Outcome::Address { text, silent } => {
                self.replace(Page::Shown(Shown::Address { text, silent }))
            }
            Outcome::Web(t) => self.replace(Page::Shown(Shown::Web(t))),
            Outcome::Text(t) => self.replace(Page::Shown(Shown::Text(t))),
            Outcome::NotRead => self.scan_line = ScanLine::NotRead,
        }
        if self.page != Page::Receive {
            self.preview = None;
        }
        self.sync();
    }

    /// The camera and the screen as the page on show wants them.
    fn sync(&mut self) {
        let camera = self.page == Page::Receive;
        if camera != self.camera {
            self.camera = camera;
            self.commands.push_back(if camera {
                Command::CameraOn
            } else {
                Command::CameraOff
            });
        }
        let on = self.page == Page::Send && self.sending.is_some();
        if on != self.screen_on {
            self.screen_on = on;
            self.out.push_back(CompanionCommand::KeepScreenOn { on });
        }
    }

    /// The frame for the state as it is.
    fn render(&mut self) {
        let (Some(ctx), Some(display)) = (self.ctx, self.display) else {
            return;
        };
        let tree = view::build(self, &display);
        let Some(canvas) = self.canvas.as_ref() else {
            return;
        };
        let Some(ui) = self.ui.as_mut() else {
            return;
        };
        let layout = layout::solve_with(&tree, canvas.bounds(), &ctx, ui.scroll_offsets());
        ui.retain_focus(&layout);
        self.layout = Some(layout);
        self.tree = Some(tree);
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    /// A frame from the camera, on Receive: the preview, and the codes
    /// in it.
    fn camera_frame(&mut self, width: u16, height: u16, luma: Vec<u8>, chroma: Option<Vec<u8>>) {
        let luma = Zeroizing::new(luma);
        let chroma = chroma.map(Zeroizing::new);
        if self.page != Page::Receive {
            return;
        }
        if matches!(self.scan_line, ScanLine::Waiting | ScanLine::NoCamera) {
            self.scan_line = ScanLine::Looking;
        }
        let found = osk_codec::decode_luma(usize::from(width), usize::from(height), &luma);
        self.preview = Some(Preview {
            width,
            height,
            pixels: Rc::new(luma),
            chroma: chroma.map(Rc::new),
        });
        for d in found {
            if self.page != Page::Receive {
                let mut b = d.bytes;
                b.zeroize();
                continue;
            }
            self.arrive(Zeroizing::new(d.bytes));
        }
    }

    /// One tick: the animated code's next frame when it is due.
    fn tick(&mut self) -> bool {
        let now = self.now_ms;
        let speed = self.speed_ms;
        if let Some(s) = self.sending.as_mut()
            && s.frames.len() > 1
            && now >= s.next_ms
        {
            s.frame = (s.frame + 1) % s.frames.len();
            s.next_ms = now + speed;
            return true;
        }
        false
    }
}

impl App for Companion {
    fn event(&mut self, event: Event) {
        match event {
            Event::Display(info) => {
                let info = DisplayInfo {
                    width: info.width.max(1),
                    height: info.height.max(1),
                    ..info
                };
                let scale = Scale::new(info.dpi);
                self.display = Some(info);
                self.canvas = Some(Canvas::new(&info));
                self.ctx = Some(LayoutCtx::new(scale, SizeClass::of(&info)).with_insets(
                    f32::from(info.inset_top) / scale.factor(),
                    f32::from(info.inset_bottom) / scale.factor(),
                ));
                self.ui = Some(UiState::new(scale));
                self.render();
                return;
            }
            Event::Tick { now_ms } => {
                self.now_ms = now_ms;
                let moved = self.tick();
                let redraw = self.ui_event(Event::Tick { now_ms });
                if moved || redraw {
                    self.render();
                }
                return;
            }
            Event::CameraFrame {
                width,
                height,
                luma,
                chroma,
            } => {
                self.camera_frame(width, height, luma, chroma);
            }
            Event::Scanned { bytes } => {
                if self.page == Page::Receive {
                    self.arrive(Zeroizing::new(bytes));
                } else {
                    let mut bytes = bytes;
                    bytes.zeroize();
                }
            }
            Event::CameraUnavailable => {
                if self.page == Page::Receive {
                    self.scan_line = ScanLine::NoCamera;
                }
            }
            Event::Clipboard { text, .. } => {
                let text = Zeroizing::new(text);
                if std::mem::take(&mut self.pasting) && self.page == Page::Receive {
                    self.arrive(Zeroizing::new(text.as_bytes().to_vec()));
                }
            }
            Event::ClipboardUnavailable { .. } => {
                if std::mem::take(&mut self.pasting) && self.page == Page::Receive {
                    self.scan_line = ScanLine::NothingToPaste;
                }
            }
            Event::File { bytes, .. } => {
                if std::mem::take(&mut self.picking) && self.page == Page::SendList {
                    self.picked(bytes);
                    self.sync();
                } else {
                    let mut bytes = bytes;
                    bytes.zeroize();
                }
            }
            Event::FileUnavailable { .. } => {
                if std::mem::take(&mut self.picking) && self.page == Page::SendList {
                    self.go(Page::NotSent(NotSent::NoPicker));
                }
            }
            Event::FileCancelled { .. } => self.picking = false,
            Event::FileWritten { .. } => {
                if let Page::Record(sha) = self.page {
                    self.saved = Some((sha, true));
                }
            }
            Event::FileNotWritten { .. } => {
                if let Page::Record(sha) = self.page {
                    self.saved = Some((sha, false));
                }
            }
            Event::Back => {
                if self.page != Page::Home {
                    self.act(Act::Back);
                }
            }
            other => {
                if !self.ui_event(other) {
                    return;
                }
            }
        }
        self.render();
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.commands.pop_front()
    }

    fn frame(&mut self) -> Frame<'_> {
        if self.dirty {
            let Self {
                canvas,
                tree,
                layout,
                theme,
                ui,
                dirty,
                ..
            } = self;
            if let (Some(canvas), Some(tree), Some(layout), Some(ui)) =
                (canvas.as_mut(), tree.as_ref(), layout.as_ref(), ui.as_ref())
            {
                canvas.clear(theme.background);
                osk_ui::widgets::draw_tree(canvas, tree, layout, theme, ui);
                *dirty = false;
            }
        }
        self.canvas.as_ref().map_or(
            Frame {
                width: 0,
                height: 0,
                rgba: &[],
            },
            Canvas::frame,
        )
    }
}

impl Companion {
    /// An input event through `osk-ui`'s state; whether a frame is due.
    fn ui_event(&mut self, event: Event) -> bool {
        let (Some(ui), Some(layout)) = (self.ui.as_mut(), self.layout.as_ref()) else {
            return false;
        };
        match ui.event(layout, event) {
            Some(Action::Tap(id)) | Some(Action::HoldCompleted(id)) => {
                if let Some(act) = Act::of(id) {
                    self.act(act);
                }
                true
            }
            Some(Action::Back) => {
                if self.page != Page::Home {
                    self.act(Act::Back);
                }
                true
            }
            Some(_) => true,
            None => false,
        }
    }
}
