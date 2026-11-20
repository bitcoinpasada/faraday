//! The passphrase field: one type holds a typed passphrase wherever it is
//! typed, takes the keys of the standard it feeds, refuses what the stick
//! rule or the standard refuses, is emptied when its screen or sheet is
//! left and when a stick arrives, and hands its text on in one of two ways
//! (`docs/KEYS.md` §2, "Transient input").
//!
//! [`Where`] lists every field that is built on it, with no wildcard arm,
//! so a field added there is not built until each of the checks below
//! knows it. Fields not yet moved onto it keep their own state and are
//! handled where they are.

pub(crate) mod eye;
pub(crate) mod pair;
pub(crate) mod screen;

use crate::secret_text::SecretText;
use crate::{Faraday, KeyIn, Screen, Sheet};

/// What a passphrase field takes, from the standard it feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "the kinds of the fields still to move onto it")]
pub(crate) enum Kind {
    /// A BIP-39 passphrase: printable ASCII (F155), at most
    /// `MAX_PASSPHRASE_BYTES`.
    Bip39,
    /// A Krux code's passphrase being made: printable ASCII, at most
    /// `PASSPHRASE_MAX`.
    KefMake,
    /// A Krux code's passphrase being read: any non-control character (the
    /// code was made elsewhere), the same most.
    KefRead,
    /// An aezeed's or a `hsm_secret`'s passphrase: printable ASCII as
    /// before, no most (their standards set none).
    Lightning,
    /// A vault's passphrase: any non-control character, no most (none is
    /// set).
    Vault,
    /// A KeePass export's passphrase: as a vault's.
    Kdbx,
    /// A Bitcoin Core wallet's passphrase: any non-control character, 1024
    /// bytes.
    CoreWallet,
}

impl Kind {
    /// Whether the kind takes `c`.
    fn takes(self, c: char) -> bool {
        match self {
            Kind::Bip39 | Kind::KefMake | Kind::Lightning => (' '..='~').contains(&c),
            Kind::KefRead | Kind::Vault | Kind::Kdbx | Kind::CoreWallet => !c.is_control(),
        }
    }

    /// The most bytes the kind holds, where there is a most.
    fn most(self) -> Option<usize> {
        match self {
            Kind::Bip39 => Some(faraday_bip::bip39::MAX_PASSPHRASE_BYTES),
            Kind::KefMake => Some(faraday_keys::kef::PASSPHRASE_MAX),
            Kind::CoreWallet => Some(1024),
            Kind::KefRead | Kind::Lightning | Kind::Vault | Kind::Kdbx => None,
        }
    }
}

/// Why a key was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refused {
    /// The character is outside printable ASCII.
    Ascii,
    /// The passphrase is at its most.
    TooLong,
    /// A stick is attached: a passphrase takes no typing.
    Stick,
}

/// What a key did to a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassKey {
    /// A character went in or came out, or the other field of a pair took
    /// the typing.
    Typed,
    /// The key was refused, and why.
    Refused(Refused),
    /// Enter: the screen decides what it means.
    Enter,
    /// Escape: the screen decides what it means.
    Escape,
    /// Not a key of a field.
    Ignored,
}

/// Which of a pair's two fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    /// The passphrase.
    First,
    /// The one typed again.
    Again,
}

/// A BIP-39 passphrase typed twice: what Add a key opens with.
impl Default for PassField {
    fn default() -> PassField {
        PassField::pair(Kind::Bip39)
    }
}

/// What the caller knows that the field does not.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Gate {
    /// A stick is in the way (`Faraday::stick.in_the_way()`).
    pub stick: bool,
}

/// One passphrase entry: the text, its repeat where it is being made, and
/// which of the two typing goes to. Holds no plain text and has no Clone,
/// Debug or PartialEq (`docs/SECRET-LEAKS.md` §6).
pub(crate) struct PassField {
    kind: Kind,
    text: SecretText,
    /// Some for a passphrase being made, typed twice.
    again: Option<SecretText>,
    /// Typing goes to the repeat.
    on_again: bool,
}

impl PassField {
    /// A passphrase typed once.
    pub(crate) fn new(kind: Kind) -> PassField {
        PassField {
            kind,
            text: SecretText::new(),
            again: None,
            on_again: false,
        }
    }

    /// A passphrase being made, typed twice.
    pub(crate) fn pair(kind: Kind) -> PassField {
        PassField {
            again: Some(SecretText::new()),
            ..PassField::new(kind)
        }
    }

    /// Applies one key. The stick rule comes first: with a stick in the
    /// way no character, Backspace, Delete, Enter or Tab goes in. Then the
    /// kind's characters and most, Backspace, and Tab between a pair.
    /// Enter and Escape are for the screen.
    pub(crate) fn key(&mut self, key: KeyIn, gate: Gate) -> PassKey {
        if gate.stick
            && matches!(
                key,
                KeyIn::Char(_) | KeyIn::Backspace | KeyIn::Delete | KeyIn::Enter | KeyIn::Tab
            )
        {
            return PassKey::Refused(Refused::Stick);
        }
        match key {
            // A control character is no key of a field.
            KeyIn::Char(c) if c.is_control() => PassKey::Ignored,
            KeyIn::Char(c) if !self.kind.takes(c) => PassKey::Refused(Refused::Ascii),
            KeyIn::Char(c) => {
                let most = self.kind.most();
                let at = self.target();
                if most.is_some_and(|m| at.len() + c.len_utf8() > m) {
                    return PassKey::Refused(Refused::TooLong);
                }
                at.push(c);
                PassKey::Typed
            }
            KeyIn::Backspace => {
                self.target().pop();
                PassKey::Typed
            }
            KeyIn::Tab | KeyIn::BackTab if self.again.is_some() => {
                self.on_again = !self.on_again;
                PassKey::Typed
            }
            KeyIn::Enter => PassKey::Enter,
            KeyIn::Escape => PassKey::Escape,
            _ => PassKey::Ignored,
        }
    }

    /// The text typing goes to.
    fn target(&mut self) -> &mut SecretText {
        match (self.on_again, self.again.as_mut()) {
            (true, Some(again)) => again,
            _ => &mut self.text,
        }
    }

    /// Typing goes to the repeat (of a pair).
    pub(crate) fn on_again(&self) -> bool {
        self.on_again && self.again.is_some()
    }

    /// Typing goes to the repeat, or back to the first.
    pub(crate) fn set_on_again(&mut self, on: bool) {
        self.on_again = on;
    }

    /// Sets the passphrase, and its repeat where it has one: Practice's
    /// `faraday` chip.
    pub(crate) fn fill(&mut self, s: &str) {
        self.text.set(s);
        if let Some(again) = self.again.as_mut() {
            again.set(s);
        }
    }

    /// Sets the passphrase only, not its repeat.
    pub(crate) fn fill_first(&mut self, s: &str) {
        self.text.set(s);
    }

    /// Sets the repeat only.
    pub(crate) fn fill_again(&mut self, s: &str) {
        if let Some(again) = self.again.as_mut() {
            again.set(s);
        }
    }

    /// The passphrase.
    pub(crate) fn as_str(&self) -> &str {
        self.text.as_str()
    }

    /// The repeat; empty where there is none.
    pub(crate) fn again_str(&self) -> &str {
        self.again.as_ref().map_or("", SecretText::as_str)
    }

    /// The passphrase is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Either text holds something.
    pub(crate) fn holds_any(&self) -> bool {
        !self.as_str().is_empty() || !self.again_str().is_empty()
    }

    /// Both texts' bytes: what the hardware-key wait counts.
    pub(crate) fn len(&self) -> usize {
        self.as_str().len() + self.again_str().len()
    }

    /// A pair is typed alike and not empty; a single one is not empty.
    pub(crate) fn ready(&self) -> bool {
        match self.again {
            Some(_) => pair::ready(self.as_str(), self.again_str()),
            None => !self.is_empty(),
        }
    }

    /// A pair differs (never for a single one).
    pub(crate) fn differ(&self) -> bool {
        self.again.is_some() && pair::differ(self.as_str(), self.again_str())
    }

    /// Moves the passphrase out and leaves the field empty: where the
    /// screen ends with the hand-off.
    #[expect(dead_code, reason = "used where a screen ends with the hand-off")]
    pub(crate) fn take(&mut self) -> SecretText {
        std::mem::take(&mut self.text)
    }

    /// One named copy for a job, where the screen keeps its field for a
    /// Back or an error; it wipes with the job.
    pub(crate) fn job_copy(&self) -> SecretText {
        SecretText::of(self.as_str())
    }

    /// The passphrase, for a call of `faraday-keys` that takes one.
    pub(crate) fn borrow(&self) -> &SecretText {
        &self.text
    }

    /// Wipes both texts and puts typing back on the first.
    pub(crate) fn clear(&mut self) {
        self.text.clear();
        if let Some(again) = self.again.as_mut() {
            again.clear();
        }
        self.on_again = false;
    }
}

/// The toast and the line that say why a key was refused.
pub(crate) fn refused_text(medium: crate::Medium, why: Refused) -> faraday_text::Text {
    match why {
        Refused::Ascii => faraday_text::Text::of(crate::state::ascii_refused()),
        Refused::TooLong => faraday_text::Text::of(crate::strings::EN.passphrase_too_long),
        Refused::Stick => faraday_text::Text::of(&crate::stick::keys_load(medium)),
    }
}

/// Every passphrase field built on [`PassField`], with the screen or sheet
/// it lives on. Matched with no wildcard arm in each function below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Where {
    /// The Inbox's possible wallet: its passphrase and the one again.
    Potential,
    /// Lightning: the aezeed's, or the `hsm_secret`'s, passphrase.
    Aezeed,
    /// Add a key's BIP-39 passphrase, and the one again where it is asked
    /// twice.
    Entry,
    /// The seed read sheet's passphrase and the one again.
    SeedRead,
    /// The Krux read's passphrase.
    KefRead,
}

impl Where {
    /// Every field.
    const ALL: [Where; 5] = [
        Where::Potential,
        Where::Aezeed,
        Where::Entry,
        Where::SeedRead,
        Where::KefRead,
    ];

    /// Whether the field's screen or sheet is up: a sheet counts as up
    /// while the idle warning covers it.
    fn up(self, app: &Faraday) -> bool {
        let sheet = |s: Sheet| {
            app.sheet == Some(s)
                || (app.sheet == Some(Sheet::IdleWarn) && app.idle.under() == Some(s))
        };
        match self {
            Where::Potential => sheet(Sheet::Potential),
            Where::Entry => app.screen == Screen::Entry,
            Where::SeedRead => sheet(Sheet::SeedRead),
            Where::KefRead => sheet(Sheet::KefRead),
            // Its state is kept while the secret sheet shows what it made
            // and on the way to a vault or a key it needs (`after_input`):
            // the node and the passphrase it was made with stay together.
            Where::Aezeed => {
                app.screen == Screen::Lightning
                    || sheet(Sheet::SecretOut)
                    || (matches!(
                        app.screen,
                        Screen::CreateVault | Screen::Vaults | Screen::Unlock | Screen::BackupFlow
                    ) && app.vaults.back_to == Some(Screen::Lightning))
                    || app.adding_key_for(Screen::Lightning)
            }
        }
    }

    /// Whether a character typed in the field on a hardware keyboard makes
    /// the step wait for the eye to be opened once: a BIP-39 passphrase
    /// being made does, one only entered does not.
    fn waits(self) -> bool {
        match self {
            Where::Potential | Where::Entry => true,
            Where::Aezeed | Where::SeedRead | Where::KefRead => false,
        }
    }

    /// The field, where there is one.
    fn field(self, app: &Faraday) -> Option<&PassField> {
        match self {
            Where::Potential => app.inbox.potential().map(|p| &p.pass),
            Where::Aezeed => app.lightning.state().map(|l| &l.passphrase),
            Where::Entry => Some(&app.entry.state().passphrase),
            Where::SeedRead => app.seed_read.state().map(|s| &s.pass),
            Where::KefRead => app.seed_read.kef().map(|k| k.pass()),
        }
    }

    /// Runs `f` on the field, where there is one.
    fn with(self, app: &mut Faraday, f: &mut dyn FnMut(&mut PassField)) {
        match self {
            Where::Potential => app.inbox.with_pass(f),
            Where::Aezeed => app.lightning.with_pass(f),
            Where::Entry => app.entry.with_pass(f),
            Where::SeedRead => app.seed_read.with_read_pass(f),
            Where::KefRead => app.seed_read.with_kef_pass(f),
        }
    }
}

/// Runs `f` on each field that exists, with where it is and whether its
/// screen or sheet is up.
pub(crate) fn each(app: &mut Faraday, mut f: impl FnMut(Where, &mut PassField, bool)) {
    for w in Where::ALL {
        let up = w.up(app);
        w.with(app, &mut |field| f(w, field, up));
    }
}

/// [`each`] for a read: the field and whether it is up.
fn each_ref(app: &Faraday, mut f: impl FnMut(Where, &PassField, bool)) {
    for w in Where::ALL {
        if let Some(field) = w.field(app) {
            f(w, field, w.up(app));
        }
    }
}

/// After an act or an event: every field whose screen or sheet is no longer
/// up is emptied, however it was left. A flow in progress keeps its fields
/// (F41, `in_progress.rs`): none of the fields listed is one.
pub(crate) fn left(app: &mut Faraday) {
    each(app, |_, field, up| {
        if !up && field.holds_any() {
            field.clear();
        }
    });
}

/// A stick arrived: every field is emptied.
pub(crate) fn stick_arrived(app: &mut Faraday) {
    each(app, |_, field, _| field.clear());
}

/// How many bytes the fields up on the screen hold, or `None` with none
/// up: `waiting` counts only those a hardware key makes the step wait on.
fn up_len(app: &Faraday, waiting: bool) -> Option<usize> {
    let mut sum = None;
    each_ref(app, |w, field, up| {
        if up && (!waiting || w.waits()) {
            *sum.get_or_insert(0) += field.len();
        }
    });
    sum
}

/// How many bytes the passphrase fields of the screen hold: a Create
/// slot's key and the seed check (New key keeps its own count), and the
/// fields on [`Where`].
fn pass_len(app: &Faraday, waiting: bool) -> Option<usize> {
    let own = match app.screen {
        Screen::Create => {
            let c = app.create.state()?;
            c.pass_slot?;
            Some(c.pass.as_str().len() + c.pass2.as_str().len())
        }
        _ if crate::seed_check::here(app) => Some(app.tools.seed_check.state()?.pass_len()),
        _ => None,
    };
    match (own, up_len(app, waiting)) {
        (None, None) => None,
        (a, b) => Some(a.unwrap_or(0) + b.unwrap_or(0)),
    }
}

/// The passphrase length before a hardware key, to hand to [`after_key`].
pub(crate) fn before_key(app: &Faraday) -> Option<usize> {
    pass_len(app, true)
}

/// After a hardware key: a character that went into a passphrase field
/// makes the passphrase wait for the eye.
pub(crate) fn after_key(app: &mut Faraday, key: KeyIn, before: Option<usize>) {
    if matches!(key, KeyIn::Char(_))
        && let (Some(was), Some(now)) = (before, pass_len(app, true))
        && now > was
    {
        app.typing.pass_eye.typed_by_hardware();
    }
}

/// After an event: with no passphrase field up, nothing is left to wait
/// for, and leaving them closes the eye.
pub(crate) fn settle(app: &mut Faraday) {
    if app.screen == Screen::KeyGen {
        return;
    }
    match pass_len(app, false) {
        None => {
            app.typing.pass_eye.reset();
            app.typing.pass_eye.close();
        }
        Some(_) if pass_len(app, true).unwrap_or(0) == 0 => app.typing.pass_eye.reset(),
        Some(_) => {}
    }
}
