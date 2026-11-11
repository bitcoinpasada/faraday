//! What the app holds: the tab and step, what was chosen and typed, and
//! what the worker sent back. Drawing reads this and changes nothing.

use aqrv::{Header, Item, Size, Step};
use zeroize::Zeroizing;

use crate::samples::SAMPLES;
use crate::worker::{Fail, Made, Shown, Working};

/// The sizes the choice offers, smallest first.
pub const SIZES: [Size; 3] = [Size::Tiny, Size::Small, Size::Large];

/// The app's two tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Making a code.
    Make,
    /// Opening one.
    Open,
}

/// Make's four screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// The item.
    Item,
    /// Size, memory and passphrase.
    Settings,
    /// The worker is running.
    Working,
    /// The code.
    Code,
}

/// Where the item comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A sample, by its place in `SAMPLES`.
    Sample(usize),
    /// What is typed.
    Own,
}

/// The kinds of item that can be typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnKind {
    /// BIP-39 entropy as hex.
    Entropy,
    /// A codex32 string.
    Codex32,
    /// A BIP-39 passphrase.
    Passphrase,
    /// A note.
    Note,
    /// A raw key as hex.
    RawKey,
}

impl OwnKind {
    /// Every typed kind, in the order the choice shows them.
    pub const ALL: [OwnKind; 5] = [
        OwnKind::Note,
        OwnKind::Passphrase,
        OwnKind::Codex32,
        OwnKind::Entropy,
        OwnKind::RawKey,
    ];
}

/// A field typing goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The typed item.
    Own,
    /// The passphrase.
    Pass,
    /// The passphrase again.
    Again,
    /// The path of a file to open.
    Path,
    /// The passphrase of the code being opened.
    OpenPass,
}

/// How far Open has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenStage {
    /// No file yet.
    Empty,
    /// The file is being read.
    Reading,
    /// The shell could not read the file.
    Unreadable,
    /// The file is not a code: no passphrase is asked for.
    NotACode,
    /// A code; the passphrase is asked for.
    Asking,
    /// The worker is opening it.
    Working,
    /// Opened: the item is shown.
    Opened,
    /// Neither tag nor layout holds.
    DoesNotOpen,
    /// Argon2id's memory could not be had.
    NotEnoughMemory,
}

/// A dialog over the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    /// The working of the code just made.
    Working,
    /// About.
    About,
}

/// What Open holds.
pub struct OpenState {
    /// The typed path.
    pub path: Zeroizing<String>,
    /// How far it has come.
    pub stage: OpenStage,
    /// The code's bytes, once the file is one.
    pub envelope: Option<Vec<u8>>,
    /// Its size and step.
    pub header: Option<Header>,
    /// The passphrase typed.
    pub pass: Zeroizing<String>,
    /// The item, once opened.
    pub shown: Option<Shown>,
    /// The request the worker is answering.
    pub job: Option<u64>,
}

impl OpenState {
    fn new() -> OpenState {
        OpenState {
            path: Zeroizing::new(String::new()),
            stage: OpenStage::Empty,
            envelope: None,
            header: None,
            pass: Zeroizing::new(String::new()),
            shown: None,
            job: None,
        }
    }
}

/// Everything a press can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Shows a tab.
    Tab(Tab),
    /// Chooses a sample.
    Sample(usize),
    /// Chooses to type an item.
    Own,
    /// Chooses the typed kind.
    Kind(OwnKind),
    /// Chooses the size.
    Size(Size),
    /// Chooses the memory step.
    Step(Step),
    /// Puts typing into a field.
    Field(Field),
    /// Shows or hides the passphrase.
    Eye,
    /// The next screen.
    Next,
    /// The screen before.
    Back,
    /// Makes the code.
    Make,
    /// Cancels the work and discards its result.
    Cancel,
    /// Starts again at the item.
    Again,
    /// Reads the file at the typed path.
    ReadPath,
    /// Opens the code with the typed passphrase.
    Unlock,
    /// Starts Open again with no file.
    OpenAnother,
    /// Saves the code as a PNG picture.
    SavePicture,
    /// Saves the code as a `.aqrv` file.
    SaveFile,
    /// Shows the working of the code made.
    ShowWorking,
    /// Shows About.
    About,
    /// Closes the dialog.
    CloseDialog,
    /// Closes the snackbar.
    CloseSnack,
}

/// The app's state.
pub struct State {
    /// The tab shown.
    pub tab: Tab,
    /// Make's screen.
    pub page: Page,
    /// Where the item comes from.
    pub source: Source,
    /// The typed kind.
    pub own_kind: OwnKind,
    /// What is typed as the item.
    pub own_text: Zeroizing<String>,
    /// The size chosen.
    pub size: Size,
    /// The memory step chosen.
    pub step: Step,
    /// The passphrase.
    pub pass: Zeroizing<String>,
    /// The passphrase again.
    pub again: Zeroizing<String>,
    /// The passphrase is shown, not dotted.
    pub shown: bool,
    /// The field typing goes to.
    pub focus: Option<Field>,
    /// How long the 64 MiB run took, in seconds, once it has.
    pub timing: Option<f32>,
    /// The code made, once it is.
    pub made: Option<Made>,
    /// Why no code was made.
    pub failed: Option<Fail>,
    /// The request the worker is answering; any other answer is dropped.
    pub job: Option<u64>,
    /// How far the page is scrolled, in design units.
    pub offset: f32,
    /// What Open holds.
    pub open: OpenState,
    /// The dialog shown.
    pub dialog: Option<Dialog>,
    /// How far the dialog is scrolled.
    pub dialog_offset: f32,
    /// The working, once the worker has it.
    pub working: Option<Working>,
    /// The request for the working.
    pub work_job: Option<u64>,
    /// What the snackbar says.
    pub snack: Option<String>,
    pub(crate) next_job: u64,
}

impl State {
    /// A new state at the item.
    pub fn new() -> State {
        State {
            tab: Tab::Make,
            page: Page::Item,
            source: Source::Sample(0),
            own_kind: OwnKind::Note,
            own_text: Zeroizing::new(String::new()),
            size: Size::Small,
            step: Step::DEFAULT,
            pass: Zeroizing::new(String::new()),
            again: Zeroizing::new(String::new()),
            shown: false,
            focus: None,
            timing: None,
            made: None,
            failed: None,
            job: None,
            offset: 0.0,
            open: OpenState::new(),
            dialog: None,
            dialog_offset: 0.0,
            working: None,
            work_job: None,
            snack: None,
            next_job: 0,
        }
    }

    /// The item chosen or typed, if it is one.
    pub fn item(&self) -> Option<Item> {
        match self.source {
            Source::Sample(i) => SAMPLES.get(i).map(|s| s.item()),
            Source::Own => own_item(self.own_kind, &self.own_text),
        }
    }

    /// Whether the item fits `size` (§3). A codex32 string's seed length
    /// is worked out from the string's, so the worker may still refuse
    /// one this lets through.
    pub fn fits(&self, size: Size) -> bool {
        self.item().is_some_and(|item| fits(&item, size))
    }

    /// The passphrase is typed twice, alike and not empty.
    pub fn passphrase_ready(&self) -> bool {
        !self.pass.is_empty() && *self.pass == *self.again
    }

    /// The seconds the step's run takes, from the 64 MiB run doubled for
    /// each step up.
    pub fn seconds(&self, step: Step) -> Option<f32> {
        let up = Step::ALL.iter().position(|s| *s == step)?;
        self.timing.map(|t| t * (1u32 << up) as f32)
    }

    /// Types `c` into the focused field.
    pub fn type_char(&mut self, c: char) {
        if let Some(text) = self.focused_text() {
            text.push(c);
        }
    }

    /// Takes the last character out of the focused field.
    pub fn backspace(&mut self) {
        if let Some(text) = self.focused_text() {
            text.pop();
        }
    }

    fn focused_text(&mut self) -> Option<&mut Zeroizing<String>> {
        match self.focus? {
            Field::Own => Some(&mut self.own_text),
            Field::Pass => Some(&mut self.pass),
            Field::Again => Some(&mut self.again),
            Field::Path => Some(&mut self.open.path),
            Field::OpenPass => Some(&mut self.open.pass),
        }
    }

    /// Whether the worker has a request out that a caller may wait for.
    pub fn working_on(&self) -> bool {
        self.open.job.is_some() || self.work_job.is_some()
    }

    /// Keeps the size chosen one the item fits.
    pub fn keep_size(&mut self) {
        if !self.fits(self.size)
            && let Some(s) = SIZES.into_iter().find(|s| self.fits(*s))
        {
            self.size = s;
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State::new()
    }
}

fn own_item(kind: OwnKind, text: &str) -> Option<Item> {
    let hex = |t: &str| faraday_codec::encodings::from_hex(t.trim());
    match kind {
        OwnKind::Note if !text.is_empty() => Some(Item::Note(Zeroizing::new(text.to_owned()))),
        OwnKind::Passphrase if !text.is_empty() => {
            Some(Item::Passphrase(Zeroizing::new(text.to_owned())))
        }
        OwnKind::Codex32 if text.len() >= 48 => {
            Some(Item::Codex32(Zeroizing::new(text.trim().to_owned())))
        }
        OwnKind::Entropy => {
            let bytes = hex(text)?;
            matches!(bytes.len(), 16 | 20 | 24 | 28 | 32)
                .then(|| Item::Entropy(Zeroizing::new(bytes)))
        }
        OwnKind::RawKey => {
            let bytes = hex(text)?;
            (bytes.len() == 32).then(|| {
                let mut key = Zeroizing::new([0u8; 32]);
                key.copy_from_slice(&bytes);
                Item::RawKey(key)
            })
        }
        _ => None,
    }
}

fn fits(item: &Item, size: Size) -> bool {
    if let Some(kind) = item.kind()
        && !size.holds(kind)
    {
        return false;
    }
    match item {
        Item::Entropy(bytes) => bytes.len() <= size.room(),
        Item::RawKey(_) => 32 <= size.room(),
        Item::Passphrase(t) | Item::Note(t) => t.len() < size.room(),
        // A string's seed is five bits a character after 22 that are not
        // (prefix, identifier, index, checksum); the small size takes no
        // more than 24 bytes of it (§3.1).
        Item::Codex32(s) => {
            let seed = s.len().saturating_sub(22) * 5 / 8;
            seed <= match size {
                Size::Tiny => 0,
                Size::Small => 24,
                Size::Large => 32,
            }
        }
    }
}
