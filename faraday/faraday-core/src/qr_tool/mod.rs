//! The QR vault tool in Tools (`docs/VAULT.md` §13): a step flow of three
//! cards, Contents, Passphrase and Code. A typed note or a loaded key's
//! words are sealed as a QR vault under a typed passphrase
//! ([`faraday_keys::qr_seal_note_job`], `Keys::qr_seal_key_job`, a job on the
//! worker), and the code is
//! shown, or put in the Outbox as a labelled picture and as the `.pgp`
//! file. Nothing typed or sealed is kept when the tool is left, at the
//! idle warning or at a lock. The drawing is `screen.rs`.

use crate::vault::{TextBox, vault_stem};
use crate::wallet::FileKind;
use crate::{Action, Faraday, QrFormat, QrSource, QrView, Screen, flow};
use faraday_crypto::work::Compute;
use faraday_keys::{
    QR_RANDOM_LEN, QR_SEAL_EXPONENTS, QrError, QrHolds, qr_note_over, qr_seal_note_job,
};
use faraday_shell_api::Key;
use zeroize::{Zeroize, Zeroizing};

mod item;
pub(crate) mod offers;
mod open;
pub(crate) mod screen;
mod seal;

pub(crate) use open::{OpenDone, OpenWork, open_stopped, opened};
pub(crate) use seal::{SealDone, SealWork, seal_stopped, sealed};

use crate::strings::{EN, fill};

/// The cards, in order.
pub mod qstep {
    /// What the code holds.
    pub(crate) const CONTENTS: u8 = 0;
    /// The passphrase, the memory step and the name.
    pub(crate) const PASSPHRASE: u8 = 1;
    /// The code.
    pub const CODE: u8 = 2;
    /// How many.
    pub const COUNT: usize = 3;
}

/// The file name of a code with no name typed.
const DEFAULT_STEM: &str = "qr-vault";

/// The most characters a name takes.
const NAME_CHARS: usize = 40;

/// What the code holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// A typed note.
    Note,
    /// A loaded key.
    Key,
    /// An item of an open vault (`item.rs`).
    Item,
}

/// A field that takes typing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// The note.
    Note,
    /// The passphrase.
    Pass,
    /// The passphrase again.
    Again,
    /// The name.
    Name,
}

/// A press in the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrToolAction {
    /// Opens the tool, from the page on show.
    Open,
    /// Opens the tool on the QR vault in this Inbox file (Files' Open in
    /// QR vault); an item of another kind is not taken.
    OpenFile(usize),
    /// Opens or closes a card.
    Step(u8),
    /// Chooses what the code holds.
    Pick(Pick),
    /// Typing goes to this field.
    Focus(Field),
    /// The eye on the note and the passphrases.
    Eye,
    /// Chooses a loaded key, by its place.
    Key(usize),
    /// Goes on from Contents with the typed note.
    Next,
    /// Chooses a memory step: the exponent.
    Memory(u8),
    /// Seals the code.
    Make,
    /// Opens the code brought, under the passphrase typed.
    OpenCode,
    /// Shows the code.
    Show,
    /// The code's picture to the Outbox.
    Png,
    /// The code's file to the Outbox.
    File,
    /// Back to the first card, with nothing kept.
    Another,
    /// The opened seed on the seed sheet (open again).
    Seed,
    /// The opened codex32 share in Add a key's codex32 form.
    Share,
    /// The opened passphrase added to the key its words make.
    AddToKey,
    /// The opened note saved into the open vault.
    SaveNote,
    /// Practice: a sample QR vault code in place of one scanned.
    UseSample,
}

/// A code made. Public (`docs/QR.md` §7.1); wiped with the tool.
struct Made {
    bytes: Vec<u8>,
    stem: String,
    date: Option<String>,
}

impl Drop for Made {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

/// The tool, while it is open.
pub struct QrToolState {
    open: Option<u8>,
    done: [bool; qstep::COUNT],
    scroll: flow::Scroll,
    back: Screen,
    pick: Option<Pick>,
    key: Option<usize>,
    /// The vault item the tool was opened on, for [`Pick::Item`].
    item: Option<item::Source>,
    note: TextBox,
    /// The room left in the note, in plain characters.
    left: usize,
    pass: TextBox,
    again: TextBox,
    name: TextBox,
    focus: Option<Field>,
    shown: bool,
    exponent: u8,
    error: Option<String>,
    made: Option<Made>,
    /// A QR vault's bytes brought to the Open card (public: encrypted).
    code: Option<Vec<u8>>,
    /// What the code opened to; the item itself is held in `faraday-keys`.
    holds: Option<QrHolds>,
    /// What an opened passphrase does for the keys loaded, worked out when
    /// it opened and again after it is added.
    pass_offer: Option<faraday_keys::QrPassOffer>,
    /// The vault the opened note was saved into: Save as a note says so
    /// for that vault.
    note_saved: Option<faraday_keys::vault::VaultId>,
}

impl QrToolState {
    fn new(back: Screen) -> QrToolState {
        QrToolState {
            open: Some(qstep::CONTENTS),
            done: [false; qstep::COUNT],
            scroll: flow::Scroll::default(),
            back,
            pick: None,
            key: None,
            item: None,
            note: TextBox::default(),
            left: room(""),
            pass: TextBox::default(),
            again: TextBox::default(),
            name: TextBox::default(),
            focus: None,
            shown: false,
            exponent: *QR_SEAL_EXPONENTS.start(),
            error: None,
            made: None,
            code: None,
            holds: None,
            pass_offer: None,
            note_saved: None,
        }
    }

    /// The QR vault code a scan or a file brought, for the Open card.
    pub fn code_to_open(&self) -> Option<&[u8]> {
        self.code.as_deref()
    }

    /// What the code brought opened to, once opened.
    pub fn holds(&self) -> Option<QrHolds> {
        self.holds
    }

    /// What an opened passphrase does for the keys loaded.
    pub fn pass_offer(&self) -> Option<faraday_keys::QrPassOffer> {
        self.pass_offer
    }

    /// Whether the tool holds something typed that is secret.
    pub(crate) fn holds_secret(&self) -> bool {
        !self.note.text.is_empty() || !self.pass.text.is_empty() || !self.again.text.is_empty()
    }

    /// The field typing goes to.
    pub fn focus(&self) -> Option<Field> {
        self.focus
    }

    /// What the code holds, once chosen.
    pub fn pick(&self) -> Option<Pick> {
        self.pick
    }

    /// The vault item the tool was opened on.
    pub(crate) fn item(&self) -> Option<&item::Source> {
        self.item.as_ref()
    }

    /// The loaded key chosen.
    pub fn key(&self) -> Option<usize> {
        self.key
    }

    /// The note's length in characters.
    pub fn note_chars(&self) -> usize {
        self.note.text.as_str().chars().count()
    }

    /// The room left in the note, in plain characters.
    pub fn room_left(&self) -> usize {
        self.left
    }

    /// The memory step chosen: the Argon2id exponent.
    pub fn exponent(&self) -> u8 {
        self.exponent
    }

    /// Whether the two passphrases are typed, the same and not empty.
    pub fn passphrase_ready(&self) -> bool {
        !self.pass.text.is_empty() && self.pass.text.as_str() == self.again.text.as_str()
    }

    /// Whether the code is made.
    pub fn is_made(&self) -> bool {
        self.made.is_some()
    }

    /// The name of the file the code goes to, without its suffix.
    pub fn stem(&self) -> String {
        stem_of(self.name.text.as_str())
    }

    fn text_of(&self, f: Field) -> &str {
        match f {
            Field::Note => self.note.text.as_str(),
            Field::Pass => self.pass.text.as_str(),
            Field::Again => self.again.text.as_str(),
            Field::Name => self.name.text.as_str(),
        }
    }

    /// Something changed that the code was made from: it goes.
    fn edited(&mut self) {
        self.made = None;
        self.error = None;
        self.done[usize::from(qstep::CODE)] = false;
    }

    fn wipe_typed(&mut self) {
        self.note.clear();
        self.pass.clear();
        self.again.clear();
        self.left = room("");
    }
}

/// The name a typed one makes: its file stem.
fn stem_of(typed: &str) -> String {
    if typed.trim().is_empty() {
        DEFAULT_STEM.to_string()
    } else {
        vault_stem(typed)
    }
}

/// The plain characters that still fit after `note`: the most `a`s whose
/// addition keeps the note inside the limit.
fn room(note: &str) -> usize {
    let mut text = Zeroizing::new(String::from(note));
    let base = text.len();
    let fits = |text: &mut Zeroizing<String>, n: usize| {
        text.truncate(base);
        text.extend(std::iter::repeat_n('a', n));
        qr_note_over(text) == 0
    };
    if !fits(&mut text, 0) {
        return 0;
    }
    let (mut lo, mut hi) = (0usize, 512usize);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if fits(&mut text, mid) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

/// The tool's state, kept in Tools.
#[derive(Default)]
pub struct QrToolArea {
    state: Option<QrToolState>,
}

impl QrToolArea {
    /// The tool's state, while it is open.
    pub fn state(&self) -> Option<&QrToolState> {
        self.state.as_ref()
    }

    /// Takes the column's scroll as the pass left it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    /// The column's scroll, while the tool is open.
    pub(crate) fn scroll_mut(&mut self) -> Option<&mut flow::Scroll> {
        self.state.as_mut().map(|s| &mut s.scroll)
    }

    /// Forgets the tool and what was typed.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }
}

/// The tool's state.
pub fn state(app: &Faraday) -> Option<&QrToolState> {
    app.tools.qr_tool().state()
}

fn st(app: &mut Faraday) -> Option<&mut QrToolState> {
    app.tools.qr_tool_mut().state.as_mut()
}

/// Opens the tool on its first card, from the page on show.
pub(crate) fn open(app: &mut Faraday) {
    let back = app.screen;
    app.session.keys.qr_forget();
    app.tools.qr_tool_mut().state = Some(QrToolState::new(back));
    app.screen = Screen::QrTool;
}

/// Opens the tool at its Passphrase step on a vault item that fits
/// (`vault/qr_save.rs`).
pub(crate) fn open_item(app: &mut Faraday, id: faraday_keys::vault::VaultId, at: usize) {
    item::open(app, id, at);
}

/// Opens the tool, from the page on show, holding a QR vault code a
/// scanner read or a file in Files held, for the Open card.
pub(crate) fn open_code(app: &mut Faraday, bytes: Vec<u8>) {
    open(app);
    if let Some(s) = st(app) {
        s.code = Some(bytes);
        s.open = Some(qstep::CONTENTS);
        s.focus = Some(Field::Pass);
    }
}

/// A press in the tool.
pub(crate) fn act(app: &mut Faraday, action: QrToolAction) {
    match action {
        QrToolAction::Open => open(app),
        QrToolAction::OpenFile(k) => {
            let kept = app.inbox.get(k).filter(|i| i.kind == FileKind::QrVault);
            if let Some(code) = kept.map(|i| i.bytes.clone()) {
                open_code(app, code);
            }
        }
        QrToolAction::Step(k) => step(app, k),
        QrToolAction::Pick(p) => {
            if let Some(s) = st(app) {
                s.pick = Some(p);
                s.key = None;
                s.focus = (p == Pick::Note).then_some(Field::Note);
                s.done = [false; qstep::COUNT];
                s.edited();
            }
        }
        QrToolAction::Focus(f) => {
            if let Some(s) = st(app) {
                s.focus = Some(f);
            }
        }
        QrToolAction::Eye => {
            if let Some(s) = st(app) {
                s.shown = !s.shown;
            }
        }
        QrToolAction::Key(i) => choose_key(app, i),
        QrToolAction::Next => {
            if let Some(s) = st(app)
                && s.pick == Some(Pick::Note)
                && !s.note.text.is_empty()
            {
                go_to_passphrase(s);
            }
        }
        QrToolAction::Memory(e) => {
            if let Some(s) = st(app)
                && QR_SEAL_EXPONENTS.contains(&e)
            {
                s.exponent = e;
                s.edited();
            }
        }
        QrToolAction::Make => make(app),
        QrToolAction::OpenCode => open::press(app),
        QrToolAction::Seed => offers::seed(app),
        QrToolAction::Share => offers::share(app),
        QrToolAction::AddToKey => offers::add_to_key(app),
        QrToolAction::SaveNote => offers::save_note(app),
        QrToolAction::UseSample => offers::use_sample(app),
        QrToolAction::Show => show(app),
        QrToolAction::Png => png(app),
        QrToolAction::File => file(app),
        QrToolAction::Another => {
            if let Some(s) = st(app) {
                let back = s.back;
                let scroll = s.scroll;
                *s = QrToolState::new(back);
                s.scroll = scroll;
                s.scroll.follow = true;
            }
        }
    }
}

fn go_to_passphrase(s: &mut QrToolState) {
    s.done[usize::from(qstep::CONTENTS)] = true;
    s.done[usize::from(qstep::PASSPHRASE)] = false;
    s.done[usize::from(qstep::CODE)] = false;
    s.made = None;
    s.open = Some(qstep::PASSPHRASE);
    s.focus = Some(Field::Pass);
    s.scroll.follow = true;
}

fn choose_key(app: &mut Faraday, i: usize) {
    if !app.session.keys.qr_key_offered(i) {
        return;
    }
    if let Some(s) = st(app) {
        s.pick = Some(Pick::Key);
        s.key = Some(i);
        go_to_passphrase(s);
    }
}

fn step(app: &mut Faraday, k: u8) {
    let Some(s) = st(app) else { return };
    if s.code.is_some() {
        return;
    }
    if s.open == Some(k) {
        s.open = None;
        return;
    }
    let reachable = match k {
        qstep::CONTENTS => true,
        qstep::PASSPHRASE => s.done[usize::from(qstep::CONTENTS)],
        _ => s.done[usize::from(qstep::PASSPHRASE)],
    };
    if reachable {
        s.open = Some(k);
        s.scroll.follow = true;
    }
}

/// The line a refusal is told in.
fn refusal_line(e: &QrError) -> String {
    match e {
        QrError::Memory(mib) => fill(EN.qr_tool_no_memory, &[&mib.to_string()]),
        QrError::NoItem => EN.qr_tool_item_gone.to_string(),
        _ => EN.qr_tool_not_made.to_string(),
    }
}

/// Seals the code. The job is made here, its randomness drawn as ever, and
/// the seal runs on the worker under the busy screen (`seal.rs`); the
/// code is read back before it is given.
fn make(app: &mut Faraday) {
    let Some(s) = st(app) else { return };
    let Some(pick) = s.pick else { return };
    if !s.passphrase_ready() || (pick == Pick::Note && s.note.text.is_empty()) {
        return;
    }
    let (note, pass) = (
        Zeroizing::new(s.note.text.as_str().to_string()),
        Zeroizing::new(s.pass.text.as_str().to_string()),
    );
    let (key, exponent) = (s.key, s.exponent);
    let stem = s.stem();
    s.error = None;
    // Every random byte of a code is the system's own: none is made
    // before it has arrived.
    if app.fresh.pool_is_empty() {
        app.out.command(faraday_shell_api::Command::RequestEntropy);
        say(app, EN.vaults_no_randomness.to_string());
        return;
    }
    let Some(drawn) = app.fresh_bytes(b"qr vault", QR_RANDOM_LEN) else {
        say(app, EN.vaults_no_randomness.to_string());
        return;
    };
    let mut random = Zeroizing::new([0u8; QR_RANDOM_LEN]);
    random.copy_from_slice(&drawn);
    drop(drawn);
    let c = app.compute();
    let job = match (pick, key) {
        (Pick::Note, _) => Ok(qr_seal_note_job(&note, &pass, exponent, random)),
        (Pick::Key, Some(i)) => app
            .session
            .keys
            .qr_seal_key_job(&c, i, &pass, exponent, random),
        (Pick::Key, None) => Err(QrError::NoItem),
        (Pick::Item, _) => item::job(app, &c, &pass, exponent, random),
    };
    match job {
        Ok(job) => seal::start(app, job, stem),
        Err(e) => say(app, refusal_line(&e)),
    }
}

fn say(app: &mut Faraday, line: String) {
    if let Some(s) = st(app) {
        s.error = Some(line);
    }
}

/// Costs a QR encoding. The code on a sheet: the 396 bytes of the
/// message as one static code at medium error correction, version 15,
/// public (`docs/QR.md` §7.1), its picture labelled with the name and
/// the date sealed and never with what it holds.
fn view(c: &Compute, made: &Made) -> Result<QrView, String> {
    use faraday_codec::qr::{Ecc, Payload, encode_bounded};
    /// The version a QR vault is drawn at.
    const VERSION: u8 = 15;
    let code = encode_bounded(c, Payload::Bytes(&made.bytes), Ecc::Medium, VERSION)
        .map_err(|_| EN.kef_code_too_big.to_string())?;
    let mut label = vec![made.stem.clone()];
    label.extend(made.date.clone());
    Ok(QrView {
        title: EN.qr_tool_view_title.to_string(),
        subtitle: EN.qr_tool_view_subtitle.to_string(),
        frames: vec![code],
        frame: 0,
        next_ms: 0,
        source: QrSource::File(format!("{}.pgp", made.stem), made.bytes.clone()),
        format: QrFormat::Ur,
        part: crate::QR_PARTS[1],
        secret: false,
        public: true,
        label,
        png_name: format!("{}.png", made.stem),
        whole: None,
        one_code: true,
    })
}

fn show(app: &mut Faraday) {
    let c = app.compute();
    let Some(made) = state(app).and_then(|s| s.made.as_ref()) else {
        return;
    };
    let v = view(&c, made);
    app.open_qr(v);
}

fn png(app: &mut Faraday) {
    let c = app.compute();
    let Some(made) = state(app).and_then(|s| s.made.as_ref()) else {
        return;
    };
    let Ok(v) = view(&c, made) else { return };
    if let Some((name, bytes)) = v.png(&c) {
        let put = app.put_outbox_beside(&name, bytes);
        app.toast(&fill(EN.qr_tool_saved, &[&put]));
    }
}

fn file(app: &mut Faraday) {
    let Some(made) = state(app).and_then(|s| s.made.as_ref()) else {
        return;
    };
    let (name, bytes) = (format!("{}.pgp", made.stem), made.bytes.clone());
    let put = app.put_outbox_beside(&name, bytes);
    app.toast(&fill(EN.qr_tool_saved, &[&put]));
}

/// The field typing goes to on the tool, with the press that focuses it
/// and its text.
pub(crate) fn field(app: &Faraday) -> Option<(Action, &str)> {
    if app.screen != Screen::QrTool {
        return None;
    }
    let s = state(app)?;
    let f = s.focus.filter(|f| match f {
        Field::Pass if s.code.is_some() => s.holds.is_none(),
        Field::Note => s.open == Some(qstep::CONTENTS) && s.pick == Some(Pick::Note),
        _ => s.open == Some(qstep::PASSPHRASE),
    })?;
    Some((Action::QrTool(QrToolAction::Focus(f)), s.text_of(f)))
}

/// A key on the tool: typing in the focused field; Enter goes on;
/// Escape is Back. Returns whether it was taken.
pub(crate) fn key(app: &mut Faraday, key: Key) -> bool {
    if app.screen != Screen::QrTool || state(app).is_none() || app.sheet.is_some() {
        return false;
    }
    let focus = field(app).and_then(|(a, _)| match a {
        Action::QrTool(QrToolAction::Focus(f)) => Some(f),
        _ => None,
    });
    if let Some(f) = focus {
        match key {
            Key::Enter if state(app).is_some_and(|s| s.code.is_some()) => {
                app.press_in(Action::QrTool(QrToolAction::OpenCode));
                return true;
            }
            Key::Tab if state(app).is_some_and(|s| s.code.is_some()) => return true,
            Key::Enter => {
                match f {
                    Field::Note => act(app, QrToolAction::Next),
                    Field::Pass => act(app, QrToolAction::Focus(Field::Again)),
                    Field::Again => act(app, QrToolAction::Focus(Field::Name)),
                    Field::Name => {
                        if state(app).is_some_and(QrToolState::passphrase_ready) {
                            app.press_in(Action::QrTool(QrToolAction::Make));
                        }
                    }
                }
                return true;
            }
            Key::Tab => {
                let next = match f {
                    Field::Note | Field::Name => f,
                    Field::Pass => Field::Again,
                    Field::Again => Field::Name,
                };
                act(app, QrToolAction::Focus(next));
                return true;
            }
            _ => {
                if type_in(app, f, key) {
                    return true;
                }
            }
        }
    }
    if key == Key::Escape {
        app.back();
        return true;
    }
    false
}

fn type_in(app: &mut Faraday, f: Field, key: Key) -> bool {
    let Some(s) = st(app) else { return false };
    let box_ = match f {
        Field::Note => &mut s.note,
        Field::Pass => &mut s.pass,
        Field::Again => &mut s.again,
        Field::Name => &mut s.name,
    };
    match key {
        Key::Char(c) if !c.is_control() => {
            if f == Field::Name && box_.text.as_str().chars().count() >= NAME_CHARS {
                return true;
            }
            box_.text.push(c);
            if f == Field::Note && qr_note_over(box_.text.as_str()) > 0 {
                box_.text.pop();
            }
        }
        Key::Backspace => {
            box_.text.pop();
        }
        _ => return false,
    }
    if f == Field::Note {
        s.left = room(s.note.text.as_str());
    }
    s.edited();
    true
}

/// Closes the eye: the idle warning.
pub(crate) fn hide(app: &mut Faraday) {
    if let Some(s) = st(app) {
        s.shown = false;
        s.pass_offer = None;
        if s.holds.take().is_some() {
            // What was opened goes with the warning: the passphrase is
            // typed again.
            s.focus = Some(Field::Pass);
        }
    }
    app.session.keys.qr_forget();
}

/// After a press: the tool ends, with what was typed and the code,
/// unless it is on show or Add randomness, opened from it, will come
/// back.
pub(crate) fn tidy(app: &mut Faraday) {
    let kept = matches!(app.screen, Screen::QrTool | Screen::Randomness);
    if !kept {
        app.tools.qr_tool_mut().clear();
        app.session.keys.qr_forget();
    }
}

/// The tool's steps, plugged into Back, Forward and the choices that
/// move on (F103, F104).
pub(crate) struct QrToolSteps;

/// The one [`QrToolSteps`].
pub(crate) static QR_TOOL_STEPS: QrToolSteps = QrToolSteps;

impl crate::trail::StepFlow for QrToolSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = state(app)?;
        if s.code.is_some() {
            return Some(crate::trail::Steps::new(vec![qstep::CONTENTS], s.open));
        }
        let upto = if s.done[usize::from(qstep::PASSPHRASE)] {
            qstep::COUNT
        } else if s.done[usize::from(qstep::CONTENTS)] {
            2
        } else {
            1
        };
        Some(crate::trail::Steps::new((0..upto as u8).collect(), s.open))
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::QrTool(QrToolAction::Step(k))
    }

    fn name(&self, app: &Faraday, k: u8) -> &'static str {
        if state(app).is_some_and(|s| s.code.is_some()) {
            return EN.qr_tool_open;
        }
        screen::TITLES[usize::from(k)]
    }
}
