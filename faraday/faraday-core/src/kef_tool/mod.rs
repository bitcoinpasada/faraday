//! Krux code in Tools (`docs/KEF.md` §15.2): a step flow of three cards,
//! Contents, the form's first step and Code. The text comes from a field
//! or from a loaded wallet's public descriptor; the form
//! ([`crate::kef_form`]) makes the code. Nothing is kept when the tool is
//! left. The drawing is `screen.rs`.

use crate::dice_entry::Press;
use crate::kef_form::{KefAct, KefForm, Source, Stage};
use crate::strings::EN;
use crate::vault::TextBox;
use crate::{Action, Faraday, Screen, Sheet, flow};
use faraday_crypto::work::Compute;
use faraday_keys::kef::COMPRESS_MAX;
use faraday_shell_api::Key;
use faraday_text::Text;

pub(crate) mod screen;

/// The cards, in order.
pub mod kstep {
    /// What the code holds.
    pub(crate) const CONTENTS: u8 = 0;
    /// The form's first step.
    pub(crate) const MAKE: u8 = 1;
    /// The code.
    pub const CODE: u8 = 2;
    /// How many.
    pub const COUNT: usize = 3;
    /// The card above them, where a code is read: not a step of making one,
    /// so it has no place among the three and no mark in `done`.
    pub const READ: u8 = 3;
}

/// Where the text comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Typed in a field.
    Type,
    /// A loaded wallet's descriptor.
    Wallet,
    /// A loaded key's words.
    Key,
}

/// A press in the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KefToolAction {
    /// Opens the tool, from the page on show.
    Open,
    /// Opens or closes a card.
    Step(u8),
    /// Chooses where the text comes from.
    Pick(Pick),
    /// Typing goes to the text.
    Field,
    /// The eye on the text.
    Eye,
    /// Chooses a loaded wallet, by its place.
    Wallet(usize),
    /// Chooses a loaded key, by its place among the keys.
    Key(usize),
    /// Goes on from Contents with the typed text.
    Next,
    /// Puts the made code's files in the Outbox again, from the
    /// take-away's row for one that is no longer there.
    SaveFiles,
    /// A press on the form.
    Form(KefAct),
    /// A press on the dice entry or the roller.
    Dice(Press),
    /// A press on the form of a seed's Krux code item of the checklist.
    Backup(KefAct),
    /// A press on that form's dice entry or roller.
    BackupDice(Press),
}

/// The tool, while it is open.
pub struct KefToolState {
    pub(crate) open: Option<u8>,
    pub(crate) done: [bool; kstep::COUNT],
    pub(crate) scroll: flow::Scroll,
    pub(crate) back: Screen,
    pub(crate) pick: Option<Pick>,
    pub(crate) typed: TextBox,
    pub(crate) typing: bool,
    pub(crate) shown: bool,
    pub(crate) wallet: Option<usize>,
    pub(crate) key: Option<usize>,
    pub(crate) form: Option<KefForm>,
    /// The SHA-256 of each file the made code put in the Outbox, taken
    /// when it was put: the picture's, when there is one, and the file's.
    /// Public: the files hold the sealed code.
    pub(crate) out: Option<(Option<[u8; 32]>, [u8; 32])>,
}

impl KefToolState {
    /// Whether the tool holds something typed that is secret: the text
    /// to seal, or what its form holds (`KefForm::holds_secret`).
    pub(crate) fn holds_secret(&self) -> bool {
        !self.typed.text.is_empty() || self.form.as_ref().is_some_and(KefForm::holds_secret)
    }

    /// The form, once there is one.
    pub fn form(&self) -> Option<&KefForm> {
        self.form.as_ref()
    }

    /// Whether the typed text takes keys.
    pub fn typing(&self) -> bool {
        self.open == Some(kstep::CONTENTS) && self.pick == Some(Pick::Type) && self.typing
    }

    /// The loaded key the code is made of, by its place among the keys.
    pub fn key(&self) -> Option<usize> {
        self.key
    }

    /// The text the form holds, in bytes.
    pub fn typed_len(&self) -> usize {
        self.typed.text.len()
    }
}

/// The tool's state, kept in Tools.
#[derive(Default)]
pub struct KefToolArea {
    state: Option<KefToolState>,
}

impl KefToolArea {
    /// Wallet `gone` left the session list: the place kept moves up. When
    /// it is the one gone the tool is as after a new pick: the descriptor
    /// taken from the wallet and the steps done with it go.
    pub(crate) fn wallet_left(&mut self, gone: usize) -> Option<KefForm> {
        let s = self.state.as_mut()?;
        let w = s.wallet?;
        match crate::wallet::place_after_removal(w, gone) {
            Some(w) => {
                s.wallet = Some(w);
                None
            }
            None => {
                s.wallet = None;
                s.key = None;
                // The card that was open belonged to the form: Contents
                // is the one with nothing to hold.
                s.open = Some(kstep::CONTENTS);
                s.done = [false; kstep::COUNT];
                // Handed back to be closed, a code being made cancelled.
                s.form.take()
            }
        }
    }

    /// The tool's state, while it is open.
    pub fn state(&self) -> Option<&KefToolState> {
        self.state.as_ref()
    }

    /// Takes the column's scroll as the pass left it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    /// How far the column is scrolled, units, while it is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|x| x.scroll.y)
    }

    /// The column scrolled to `y` units, while it is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(x) = self.state.as_mut() {
            x.scroll.y = y;
        }
    }

    /// Forgets the tool and what was typed.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }
}

/// The tool's state.
pub fn state(app: &Faraday) -> Option<&KefToolState> {
    app.tools.kef_tool.state()
}

fn st(app: &mut Faraday) -> Option<&mut KefToolState> {
    app.tools.kef_tool.state.as_mut()
}

/// The text the wallet at `i` makes: its checksummed public descriptor,
/// and the fingerprint offered as the id when it has one key.
fn wallet_text(app: &Faraday, c: &Compute, i: usize) -> Result<(Text, Option<[u8; 4]>), Text> {
    let w = app
        .session
        .wallets
        .get(i)
        .ok_or_else(|| Text::of(EN.code_view_that_wallet))?;
    // A silent payments wallet's text is its address: none while its key
    // waits for its backup or a loaded key contradicts the record.
    if w.policy().silent().is_some() {
        if app.receive_held(c, w) {
            return Err(Text::of(EN.code_view_no_address));
        }
        app.silent_marks(c, i, Vec::new())?;
    }
    let keys = w.policy().keys();
    let fp = match keys {
        [k] => k.fingerprint().map(|f| f.0),
        _ => None,
    };
    Ok((Text::of(&w.policy().to_descriptor_checksummed()), fp))
}

/// The loaded keys a code can be made of (12 or 24 English words), by
/// their place among the keys.
pub(crate) fn offered_keys(app: &Faraday) -> Vec<usize> {
    (0..app.session.keys.len())
        .filter(|&i| app.session.keys.info(i).is_some_and(|k| k.kef_offered()))
        .collect()
}

/// Starts the form on key `i`'s words, as the entropy Krux reads them
/// from (`docs/KEF.md` §6), the fingerprint of the words alone (without the
/// key's passphrase) offered as the id. The
/// words reach the encryption only inside `faraday-keys`.
fn begin_key(app: &mut Faraday, i: usize) {
    let Some(info) = app.session.keys.info(i).filter(|k| k.kef_offered()) else {
        return;
    };
    let bytes = usize::from(info.word_count.unwrap_or(0)) * 4 / 3;
    let fingerprint = true;
    let id = info.id;
    let mut form = KefForm::new(
        Source::Seed {
            key: id,
            bytes,
            fingerprint,
        },
        false,
    );
    form.offer_by_hand();
    finish_begin(app, form, None, Some(i));
}

/// Runs `f` on the form's passphrase and the check's, where the form is.
pub(crate) fn with_pass(app: &mut Faraday, f: &mut dyn FnMut(&mut crate::pass_field::PassField)) {
    if let Some(form) = st(app).and_then(|s| s.form.as_mut()) {
        form.with_pass(f);
    }
}

/// A stick is attached: the typed text and the form's secret fields lose
/// focus and, with `wipe`, what was typed.
pub(crate) fn stick_attached(app: &mut Faraday, wipe: bool) {
    if let Some(s) = st(app) {
        s.typing = false;
        if wipe {
            s.typed.clear();
        }
        if let Some(f) = s.form.as_mut() {
            f.stick_attached(wipe);
        }
    }
    crate::wallet_backup::kef::stick_attached(app, wipe);
}

/// A press in the tool.
pub(crate) fn act(app: &mut Faraday, action: KefToolAction) {
    match action {
        KefToolAction::Open => {
            let back = app.screen;
            app.tools.kef_tool.state = Some(KefToolState {
                open: Some(kstep::CONTENTS),
                done: [false; kstep::COUNT],
                scroll: flow::Scroll::default(),
                back,
                pick: None,
                typed: TextBox::default(),
                typing: false,
                shown: false,
                wallet: None,
                key: None,
                form: None,
                out: None,
            });
            app.screen = Screen::KefTool;
        }
        KefToolAction::Step(k) => step(app, k),
        KefToolAction::Pick(p) => {
            let no_stick = app.stick.is_empty();
            if let Some(s) = st(app) {
                s.pick = Some(p);
                s.typing = p == Pick::Type && no_stick;
                s.wallet = None;
                s.key = None;
                s.form = None;
                s.done = [false; kstep::COUNT];
            }
            if p == Pick::Key
                && let [only] = offered_keys(app)[..]
            {
                begin_key(app, only);
            }
        }
        KefToolAction::SaveFiles => {
            // Those still waiting keep the tick the person left them.
            let waiting: Vec<Text> = app.outbox.iter().map(|i| Text::of(&i.name)).collect();
            with_form(app, |f, app| f.save_again(app));
            note_files(app, &waiting);
        }
        KefToolAction::Field => {
            if crate::stick::refuses_secret(app) {
                return;
            }
            if let Some(s) = st(app) {
                s.typing = true;
            }
        }
        KefToolAction::Eye => {
            if let Some(s) = st(app) {
                s.shown = !s.shown;
            }
        }
        KefToolAction::Wallet(i) => {
            let c = app.compute();
            match wallet_text(app, &c, i) {
                Ok((text, fp)) => {
                    // A row of the pane picks without the choice's button.
                    picked(app, Pick::Wallet);
                    begin(app, text, fp, Some(i));
                }
                Err(line) => app.toast(&line),
            }
        }
        KefToolAction::Key(i) => {
            if app.session.keys.info(i).is_some_and(|k| k.kef_offered()) {
                picked(app, Pick::Key);
            }
            begin_key(app, i);
        }
        KefToolAction::Next => {
            let Some(s) = st(app) else { return };
            if s.typed.text.is_empty() {
                return;
            }
            if crate::stick::refuses_secret(app) {
                return;
            }
            let Some(s) = st(app) else { return };
            let text = Text::of(s.typed.text.as_str());
            begin(app, text, None, None);
        }
        KefToolAction::Form(KefAct::Done) => {
            if let Some(back) = state(app).map(|s| s.back) {
                app.act(Action::Nav(back));
            }
            return;
        }
        KefToolAction::Form(a) => {
            with_form(app, |f, app| f.act(app, a));
        }
        KefToolAction::Dice(p) => {
            with_form(app, |f, app| f.dice_press(app, p));
        }
        KefToolAction::Backup(a) => return crate::wallet_backup::kef::act(app, a),
        KefToolAction::BackupDice(p) => return crate::wallet_backup::kef::dice(app, p),
    }
    // A job run inline has its answer by now.
    poll(app);
    sync(app);
}

/// The text's source is `p`, as the choice's button leaves it, for a press
/// that picks from the pane.
fn picked(app: &mut Faraday, p: Pick) {
    if let Some(s) = st(app) {
        s.pick = Some(p);
        s.typing = false;
    }
}

/// Starts the form on `text` and opens its card.
fn begin(app: &mut Faraday, text: Text, fp: Option<[u8; 4]>, wallet: Option<usize>) {
    let mut form = KefForm::new(Source::Text(text), false);
    form.offer_fingerprint(fp);
    form.offer_by_hand();
    finish_begin(app, form, wallet, None);
}

/// Puts the form in the state and opens its card.
fn finish_begin(app: &mut Faraday, form: KefForm, wallet: Option<usize>, key: Option<usize>) {
    if let Some(s) = st(app) {
        s.form = Some(form);
        s.wallet = wallet;
        s.key = key;
        s.done[usize::from(kstep::CONTENTS)] = true;
        s.done[usize::from(kstep::MAKE)] = false;
        s.done[usize::from(kstep::CODE)] = false;
        s.open = Some(kstep::MAKE);
        s.scroll.follow = true;
    }
}

fn step(app: &mut Faraday, k: u8) {
    let Some(s) = st(app) else { return };
    // A step is always open: pressing the open one does nothing.
    if s.open == Some(k) {
        return;
    }
    let reachable = match k {
        kstep::CONTENTS | kstep::READ => true,
        kstep::MAKE => s.done[usize::from(kstep::CONTENTS)],
        _ => s.done[usize::from(kstep::MAKE)],
    };
    if reachable {
        s.open = Some(k);
        s.scroll.follow = true;
    }
}

/// Runs `f` on the form, taken out of the state for it so that it can
/// use the app.
fn with_form<R>(app: &mut Faraday, f: impl FnOnce(&mut KefForm, &mut Faraday) -> R) -> Option<R> {
    let mut form = st(app)?.form.take()?;
    let out = f(&mut form, app);
    match st(app) {
        Some(s) => s.form = Some(form),
        None => drop(form),
    }
    Some(out)
}

/// Brings the cards and the Options sheet to where the form stands.
fn sync(app: &mut Faraday) {
    let Some(s) = st(app) else { return };
    let Some(f) = s.form.as_ref() else { return };
    let (stage, options) = (f.stage(), f.options_open());
    match stage {
        Stage::Made | Stage::Check | Stage::Comparing => {
            s.done[usize::from(kstep::MAKE)] = true;
            s.done[usize::from(kstep::CODE)] = true;
            if s.open == Some(kstep::MAKE) {
                s.open = Some(kstep::CODE);
                s.scroll.follow = true;
            }
        }
        Stage::Entry | Stage::Making => {
            s.out = None;
            s.done[usize::from(kstep::MAKE)] = false;
            s.done[usize::from(kstep::CODE)] = false;
            if s.open == Some(kstep::CODE) {
                s.open = Some(kstep::MAKE);
                s.scroll.follow = true;
            }
        }
    }
    if app.screen != Screen::KefTool {
        return;
    }
    if options && app.sheet.is_none() {
        app.sheet = Some(Sheet::KefOptions);
    } else if !options && app.sheet == Some(Sheet::KefOptions) {
        app.sheet = None;
    }
}

/// The field typing goes to on the tool, with the press that focuses it
/// and its text: the typed contents, or a field of the form.
pub(crate) fn field(app: &Faraday) -> Option<(Action, &str)> {
    if app.screen == Screen::Backup {
        return crate::wallet_backup::kef::field(app);
    }
    if app.screen != Screen::KefTool {
        return None;
    }
    let s = state(app)?;
    if s.typing() {
        return Some((Action::KefTool(KefToolAction::Field), s.typed.text.as_str()));
    }
    let (a, t) = s.form()?.field()?;
    Some((wrap(a), t))
}

/// A key on the tool: typing in the text, in the form or in Options;
/// Escape closes Options, otherwise it is Back. Returns whether it was
/// taken.
pub(crate) fn key(app: &mut Faraday, key: Key) -> bool {
    if app.screen == Screen::Backup {
        return crate::wallet_backup::kef::key(app, key);
    }
    if app.screen != Screen::KefTool || state(app).is_none() {
        return false;
    }
    if app.sheet.is_some() && app.sheet != Some(Sheet::KefOptions) {
        return false;
    }
    let taken = if state(app).is_some_and(|s| s.typing()) {
        if key == Key::Enter {
            act(app, KefToolAction::Next);
            return true;
        }
        type_text(app, key)
    } else {
        with_form(app, |f, app| f.key(app, key)).unwrap_or(false)
    };
    if taken {
        poll(app);
        sync(app);
        return true;
    }
    if key == Key::Escape {
        if app.sheet == Some(Sheet::KefOptions) {
            act(app, KefToolAction::Form(KefAct::Back));
        } else {
            app.back();
        }
        return true;
    }
    false
}

fn type_text(app: &mut Faraday, key: Key) -> bool {
    if crate::stick::refuses_secret_key(app, key) {
        return true;
    }
    let Some(s) = st(app) else { return false };
    match key {
        Key::Char(c) if !c.is_control() => {
            if s.typed.text.len() + c.len_utf8() <= COMPRESS_MAX {
                s.typed.text.push(c);
            }
            true
        }
        Key::Backspace => {
            s.typed.text.pop();
            true
        }
        _ => false,
    }
}

/// After a worker's answer: the form takes it and the cards follow.
pub(crate) fn poll(app: &mut Faraday) {
    if app.tools.kef_tool.state().is_none() {
        return;
    }
    if with_form(app, |f, app| f.poll(app)).unwrap_or(false) {
        sync(app);
        if state(app).is_some_and(|s| s.out.is_none()) {
            note_files(app, &[]);
        }
        app.out.redraw();
    }
}

/// The dice roller's tick. Returns whether a frame is needed.
pub(crate) fn tick(app: &mut Faraday) -> bool {
    with_form(app, |f, app| f.tick(app)).unwrap_or(false) | crate::wallet_backup::kef::tick(app)
}

/// Cancel while the work runs.
pub(crate) fn cancel(app: &mut Faraday) {
    if with_form(app, |f, app| f.cancel(app)).is_some() {
        sync(app);
    }
    crate::wallet_backup::kef::cancel(app);
}

/// Closes the eye: the idle warning.
pub(crate) fn hide(app: &mut Faraday) {
    if let Some(s) = st(app) {
        s.shown = false;
    }
    crate::wallet_backup::kef::hide(app);
}

/// After a press: the tool ends, what was typed with it, unless it is on
/// show or Add randomness, opened from it, will come back. Options
/// closed from outside is closed in the form too.
pub(crate) fn tidy(app: &mut Faraday) {
    crate::wallet_backup::kef::tidy(app);
    let kept = matches!(app.screen, Screen::KefTool | Screen::Randomness);
    if !kept {
        app.tools.kef_tool.clear();
        return;
    }
    if app.sheet != Some(Sheet::KefOptions)
        && state(app)
            .and_then(|s| s.form())
            .is_some_and(|f| f.options_open())
    {
        with_form(app, |f, app| f.act(app, KefAct::Back));
    }
}

/// Costs a SHA-256 pass over each of the made code's files, taken from the
/// Outbox as they were just put there: keeps each hash, on the item and
/// here, so that only those bytes count as written, and ticks each one
/// that is not among `waiting`, the files that were there before.
fn note_files(app: &mut Faraday, waiting: &[Text]) {
    let Some(saved) = state(app)
        .and_then(KefToolState::form)
        .and_then(KefForm::saved)
        .cloned()
    else {
        return;
    };
    let c = app.compute();
    let mut hash = |name: &str| {
        let item = app.outbox.iter_mut().find(|i| *i.name == *name)?;
        let sha = crate::sha256_of(&c, &item.bytes);
        item.sha = Some(sha);
        Some(sha)
    };
    let png = saved.png.as_deref().and_then(&mut hash);
    let file = hash(&saved.file);
    if let Some(s) = st(app) {
        s.out = file.map(|file| (png, file));
    }
    let put = saved.png.iter().chain([&saved.file]);
    let new = put.filter(|n| !waiting.contains(n));
    app.visit.out_extend(new.map(|n| &**n));
}

/// Wraps a press on the form as an `Action`.
pub(crate) fn wrap(a: KefAct) -> Action {
    Action::KefTool(KefToolAction::Form(a))
}

/// The tool's steps, plugged into Back, Forward and the choices that
/// move on (F103, F104).
pub(crate) struct KefToolSteps;

/// The one [`KefToolSteps`].
pub(crate) static KEF_TOOL_STEPS: KefToolSteps = KefToolSteps;

impl crate::trail::StepFlow for KefToolSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = state(app)?;
        // The reading card leads, above the steps that make a code.
        let order = std::iter::once(kstep::READ)
            .chain(0..kstep::COUNT as u8)
            .collect();
        let mut steps = crate::trail::Steps::new(order, s.open);
        steps.first = Some(kstep::CONTENTS);
        steps.answered = (0..kstep::COUNT as u8)
            .filter(|&k| s.done[usize::from(k)])
            .collect();
        Some(steps)
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::KefTool(KefToolAction::Step(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        screen::TITLES[usize::from(k)]
    }

    fn contained(&self) -> Option<&dyn crate::flow::Contained> {
        Some(self)
    }
}

impl crate::flow::Contained for KefToolSteps {
    fn rows(&self, app: &Faraday) -> Vec<crate::flow::Row> {
        let Some(s) = state(app) else {
            return Vec::new();
        };
        std::iter::once(kstep::READ)
            .chain(0..kstep::COUNT as u8)
            .map(|k| {
                let said = screen::summary(app, s, k);
                crate::flow::Row {
                    label: faraday_text::Text::of(screen::TITLES[usize::from(k)]),
                    value: (!said.is_empty()).then(|| Text::of(&said)),
                    mono: false,
                    opens: k,
                    current: s.open == Some(k),
                    mark: None,
                    address: false,
                }
            })
            .collect()
    }

    fn pick(&self, app: &Faraday, k: u8) -> Option<crate::flow::Pick> {
        let s = state(app)?;
        (k == kstep::CONTENTS).then(|| screen::pick(app, s))
    }

    fn hide_shown(&self, app: &mut crate::Faraday) {
        hide(app);
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }

    fn take_away(&self, app: &crate::Faraday, k: u8) -> Option<crate::flow::TakeAway> {
        // The made code's picture and file: the sealed code, never what
        // it holds or its passphrase.
        let s = state(app)?;
        if k != kstep::CODE || !s.done[usize::from(k)] {
            return None;
        }
        let f = s.form()?;
        let (saved, made) = (f.saved()?, f.made()?);
        let (png_sha, file_sha) = s.out?;
        let row = |name: &str, base: &str, sha: [u8; 32], line| {
            let waits = app
                .outbox
                .iter()
                .any(|i| *i.name == *name && i.sha == Some(sha));
            crate::flow::TakeFile {
                name: faraday_text::Text::of(name),
                sha,
                again: if waits {
                    faraday_text::Text::of(name)
                } else {
                    crate::stick::visit_taken::outbox_free_name(&app.outbox, base)
                },
                line,
                make_label: EN.kef_tool_make_the_file,
                make: Action::KefTool(KefToolAction::SaveFiles),
            }
        };
        let picture = saved
            .png
            .as_deref()
            .zip(png_sha)
            .map(|(name, sha)| row(name, &made.png_name, sha, EN.kef_tool_take_picture));
        let file = row(
            &saved.file,
            &made.file_name,
            file_sha,
            EN.kef_tool_take_file,
        );
        Some(crate::flow::TakeAway {
            files: picture.into_iter().chain([file]).collect(),
            codes: Vec::new(),
        })
    }

    fn heads(&self, _: &Faraday) -> usize {
        1
    }
}

// The typed code, its passphrase, the opened payload.
impl crate::at_lock::AtLock for KefToolArea {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds::typed(self.state().is_some_and(KefToolState::holds_secret))
    }
    fn lock(&mut self) {
        self.clear();
    }
}
