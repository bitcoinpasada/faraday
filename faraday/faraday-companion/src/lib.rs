//! Faraday Companion (`docs/COMPANION.md`): the phone app's state,
//! screens and decisions, with no I/O. It is an [`osk_shell_api::App`],
//! as `faraday-core`'s `Faraday` is: the shell sends input and display
//! events through [`App::event`], reads the frame, and takes upstream's
//! commands from [`App::poll_command`]. What upstream's interface has no
//! event for goes through a second channel beside it ([`api`]): the
//! shell reports through [`Companion::companion_event`] and takes what
//! the core asks for from [`Companion::poll_companion`].
//!
//! Its screens are drawn with Faraday's own drawing (`faraday-ui`), as
//! Faraday Starter's are, on a phone's width, portrait ([`draw`]). It
//! holds no part of Faraday's vault,
//! signer or key loaders (§3): a seed, a private key or a sealed file that
//! arrives is refused by [`route`], and nothing of it is kept.

pub mod api;
mod draw;
pub mod route;
pub mod store;

use std::collections::VecDeque;
use std::rc::Rc;

pub use faraday_ui::Theme;

use faraday_intake::Secret;
use faraday_qr::bitlocker::KeyText;
use faraday_qr::send::{Format, Made, PARTS, SPEEDS, Source};
use osk_codec::QrMatrix;
use osk_shell_api::{App, Command, DisplayInfo, Event, FileKind, Frame, Key, TouchPhase};
use osk_ui::{Canvas, Rect};
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

/// Everything a press can do: the action type the Companion draws its
/// screens with (`faraday_ui::Ui<Act>`).
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
    /// About's theme `n`, in [`Theme::ALL`]'s order.
    Theme(usize),
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
    canvas: Option<Canvas>,
    /// Pixels per design unit.
    f: f32,
    /// The page's width and height in design units, the system's insets
    /// left out.
    w: f32,
    h: f32,
    /// The system's bars over the top and the bottom of the screen, in
    /// design units.
    inset_top: f32,
    inset_bottom: f32,
    /// The display is under 600 dp wide: one column, the touch sizes of
    /// the stick's small panel.
    compact: bool,
    /// The palette.
    pub theme: Theme,
    /// Where each act can be pressed, as last drawn, in pixels.
    hits: Vec<(Rect, Act)>,
    /// The act under a finger that is down.
    pressed: Option<Act>,
    /// The act under the pointer, with no button down.
    hovered: Option<Act>,
    /// Where a finger went down, the offset then, and whether it has
    /// moved far enough to scroll rather than press.
    drag: Option<(i32, f32, bool)>,
    /// How far the page, or the sheet over it, is scrolled, units.
    offset: f32,
    /// How far it can scroll, as last drawn.
    max: f32,
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
    /// The camera's last frame on Receive: its size, its luma and its
    /// chroma, wiped when it is replaced.
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

/// A frame from the camera, as the scanner shows it.
struct Preview {
    width: u16,
    height: u16,
    luma: Zeroizing<Vec<u8>>,
    chroma: Option<Zeroizing<Vec<u8>>>,
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
            canvas: None,
            f: 1.0,
            w: 0.0,
            h: 0.0,
            inset_top: 0.0,
            inset_bottom: 0.0,
            compact: true,
            theme: Theme::default(),
            hits: Vec::new(),
            pressed: None,
            hovered: None,
            drag: None,
            offset: 0.0,
            max: 0.0,
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

    /// Whether the screen on show offers `act` to a press.
    pub fn offers(&self, act: Act) -> bool {
        self.hits.iter().any(|(_, a)| *a == act)
    }

    /// Where the screen on show offers `act`, in pixels: the middle of
    /// it, for a test that touches it.
    pub fn where_offered(&self, act: Act) -> Option<(u16, u16)> {
        let (r, _) = self.hits.iter().rev().find(|(_, a)| *a == act)?;
        Some(((r.x + r.w / 2) as u16, (r.y + r.h / 2) as u16))
    }

    /// Every string the screen on show draws, in the order drawn. A
    /// masked secret draws its dots, not itself.
    pub fn texts(&mut self) -> Vec<String> {
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
        texts
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
        self.offset = 0.0;
    }

    /// Shows `page` in place of the one on show.
    fn replace(&mut self, page: Page) {
        self.page = page;
        self.offset = 0.0;
    }

    /// Back one page.
    fn back(&mut self) {
        self.page = self.stack.pop().unwrap_or(Page::Home);
        self.offset = 0.0;
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
                if let Some(d) = draw::learn_docs().get(n) {
                    self.go(Page::Doc(*d));
                }
            }
            Act::StartPage(n) => {
                if let Some(d) = draw::start_docs().get(n) {
                    self.go(Page::Doc(*d));
                }
            }
            Act::Eye => self.revealed = !self.revealed,
            Act::Theme(n) => {
                if let Some(t) = Theme::ALL.get(n) {
                    self.theme = *t;
                }
            }
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

    /// Draws the frame for the state as it is, and asks the shell to
    /// show it.
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
            ui.theme = self.theme;
            draw::frame(self, &mut ui)
        };
        self.max = max;
        self.hits = hits;
        self.canvas = Some(canvas);
        self.commands.push_back(Command::Draw);
    }

    /// The display's size and density: the page draws at the desktop's
    /// density, 160 dpi a unit, and a display under 600 dp wide gets the
    /// one column and the touch sizes of the stick's small panel.
    fn display(&mut self, d: DisplayInfo) {
        let density = f32::from(d.dpi.max(1)) / 160.0;
        let f = density.max(0.5);
        self.compact = f32::from(d.width) / density < 600.0;
        let scaled = DisplayInfo {
            dpi: (160.0 * f).round().max(1.0) as u16,
            ..d
        };
        self.f = f;
        self.w = f32::from(d.width) / f;
        self.inset_top = f32::from(d.inset_top) / f;
        self.inset_bottom = f32::from(d.inset_bottom) / f;
        self.h = f32::from(d.height) / f - self.inset_top - self.inset_bottom;
        self.canvas = Some(Canvas::new(&scaled));
    }

    /// The act drawn under pixel (x, y), the last drawn on top.
    fn hit_at(&self, x: i32, y: i32) -> Option<Act> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
            .map(|(_, a)| *a)
    }

    /// A finger on the screen: a press where it lifts on what it went
    /// down on, and a scroll once it has moved further than a press
    /// would. Returns whether a frame is due.
    fn touch(&mut self, x: i32, y: i32, phase: TouchPhase) -> bool {
        // How far a finger moves, in units, before it scrolls.
        const SLOP: f32 = 8.0;
        match phase {
            TouchPhase::Down => {
                self.pressed = self.hit_at(x, y);
                self.drag = Some((y, self.offset, false));
                true
            }
            TouchPhase::Move => {
                let Some((y0, from, moving)) = self.drag else {
                    return false;
                };
                let moved = (y - y0) as f32 / self.f;
                if !moving && moved.abs() < SLOP {
                    return false;
                }
                self.drag = Some((y0, from, true));
                self.pressed = None;
                self.offset = (from - moved).clamp(0.0, self.max.max(0.0));
                true
            }
            TouchPhase::Up => {
                let pressed = self.pressed.take();
                let moving = self.drag.take().is_some_and(|d| d.2);
                if let Some(a) = pressed
                    && !moving
                    && self.hit_at(x, y) == Some(a)
                {
                    self.act(a);
                }
                true
            }
        }
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
            luma,
            chroma,
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
                self.display(DisplayInfo {
                    width: info.width.max(1),
                    height: info.height.max(1),
                    ..info
                });
            }
            Event::Tick { now_ms } => {
                self.now_ms = now_ms;
                if !self.tick() {
                    return;
                }
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
            Event::Back | Event::Key(Key::Escape) => {
                if self.page != Page::Home {
                    self.act(Act::Back);
                }
            }
            Event::Touch { x, y, phase } => {
                if !self.touch(i32::from(x), i32::from(y), phase) {
                    return;
                }
            }
            Event::Hover { x, y } => {
                let now = self.hit_at(i32::from(x), i32::from(y));
                if now == self.hovered {
                    return;
                }
                self.hovered = now;
            }
            Event::HoverEnd => {
                if self.hovered.take().is_none() {
                    return;
                }
            }
            Event::Wheel { dy, .. } | Event::Scroll { dy, .. } => {
                let to = (self.offset + f32::from(dy) / self.f).clamp(0.0, self.max.max(0.0));
                if to == self.offset {
                    return;
                }
                self.offset = to;
            }
            _ => return,
        }
        self.render();
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.commands.pop_front()
    }

    fn frame(&mut self) -> Frame<'_> {
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
