//! The checklist's Krux code item (`docs/KEF.md` §15.1): the form
//! ([`crate::kef_form`]) on the card of seed `i`'s code, made from the
//! seed itself. A form is kept for each seed whose item was opened; it is
//! put away (`KefForm::close`) when the item closes or the page is left,
//! keeping the code made and whether it was checked. The drawing is in
//! `screens/backup.rs`.

use super::WalletBackupArea;
use crate::dice_entry::Press;
use crate::kef_form::screen::Ask;
use crate::kef_form::{KefAct, KefForm, Source};
use crate::kef_tool::KefToolAction;
use crate::{Action, Faraday, Screen, Sheet, bstep, plan};
use faraday_shell_api::Key;

impl WalletBackupArea {
    /// Seed `i`'s Krux code form, once its item was opened.
    pub(crate) fn kef_form(&self, i: usize) -> Option<&KefForm> {
        self.state
            .as_ref()?
            .kef
            .iter()
            .find(|(j, _)| *j == i)
            .map(|(_, f)| f)
    }

    /// The seed whose Krux code item is open.
    pub(crate) fn kef_open(&self) -> Option<usize> {
        let b = self.state.as_ref()?;
        if b.stage != crate::BStage::Checklist {
            return None;
        }
        match bstep::item(b.open?)? {
            plan::Item::KefCode(i) => Some(i),
            _ => None,
        }
    }

    /// Whether any form holds something typed that is secret.
    pub(crate) fn kef_holds_secret(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|b| b.kef.iter().any(|(_, f)| f.holds_secret()))
    }

    /// Whether seed `i`'s Krux code was made in this backup.
    pub(crate) fn kef_made(&self, i: usize) -> bool {
        self.kef_form(i).is_some_and(|f| f.made().is_some())
    }

    /// Whether seed `i`'s Krux code was opened and held the seed.
    pub(crate) fn kef_checked(&self, i: usize) -> bool {
        self.kef_form(i).is_some_and(KefForm::checked)
    }

    fn kef_take(&mut self, i: usize) -> Option<KefForm> {
        let kef = &mut self.state.as_mut()?.kef;
        let at = kef.iter().position(|(j, _)| *j == i)?;
        Some(kef.remove(at).1)
    }

    fn kef_put(&mut self, i: usize, form: KefForm) {
        if let Some(b) = self.state.as_mut() {
            b.kef.push((i, form));
        }
    }

    fn kef_seeds(&self) -> Vec<usize> {
        self.state
            .as_ref()
            .map(|b| b.kef.iter().map(|(i, _)| *i).collect())
            .unwrap_or_default()
    }
}

/// Runs `f` on seed `i`'s form, taken out of the state for it so that it
/// can use the app.
fn with_form<R>(
    app: &mut Faraday,
    i: usize,
    f: impl FnOnce(&mut KefForm, &mut Faraday) -> R,
) -> Option<R> {
    let mut form = app.backup.kef_take(i)?;
    let out = f(&mut form, app);
    if app.backup.state().is_some() {
        app.backup.kef_put(i, form);
    }
    Some(out)
}

/// The backup is about to be dropped: every form's work is cancelled and
/// what was typed is wiped.
pub(crate) fn dropped(app: &mut Faraday) {
    for i in app.backup.kef_seeds() {
        with_form(app, i, |f, app| f.close(app));
    }
}

/// Closes the eyes of the forms: the idle warning.
pub(crate) fn hide(app: &mut Faraday) {
    for i in app.backup.kef_seeds() {
        with_form(app, i, |f, _| f.hide_eyes());
    }
}

/// The open item's form, run on.
fn with_open<R>(app: &mut Faraday, f: impl FnOnce(&mut KefForm, &mut Faraday) -> R) -> Option<R> {
    let i = app.backup.kef_open()?;
    with_form(app, i, f)
}

/// A form for seed `i`: its bytes, its wallet's fingerprint.
fn new_form(app: &Faraday, i: usize) -> Option<KefForm> {
    let w = app.backup.state()?.wallet;
    let key = app.backup_seed_list_now(w).get(i)?.1?;
    let info = app.session.keys.info(key)?;
    let bytes = usize::from(info.word_count?) * 4 / 3;
    Some(KefForm::new(
        Source::Seed {
            index: key,
            bytes,
            fingerprint: Some(info.fingerprint.0),
        },
        false,
    ))
}

/// After the open item changed: the forms of the other seeds are put
/// away, and the open Krux code item has its form.
pub(crate) fn opened(app: &mut Faraday) {
    let cur = app.backup.kef_open();
    for i in app.backup.kef_seeds() {
        if Some(i) != cur {
            with_form(app, i, |f, app| f.close(app));
        }
    }
    if let Some(i) = cur
        && app.backup.kef_form(i).is_none()
        && let Some(form) = new_form(app, i)
    {
        app.backup.kef_put(i, form);
    }
    sync(app);
}

/// A press on the form.
pub(crate) fn act(app: &mut Faraday, a: KefAct) {
    with_open(app, |f, app| f.act(app, a));
    poll(app);
    sync(app);
}

/// A press on the dice entry or the roller.
pub(crate) fn dice(app: &mut Faraday, p: Press) {
    with_open(app, |f, app| f.dice_press(app, p));
    sync(app);
}

/// Brings the Options sheet to where the form stands.
fn sync(app: &mut Faraday) {
    if app.screen != Screen::Backup {
        return;
    }
    let options = app
        .backup
        .kef_open()
        .and_then(|i| app.backup.kef_form(i))
        .is_some_and(KefForm::options_open);
    if options && app.sheet.is_none() {
        app.sheet = Some(Sheet::KefOptions);
    } else if !options && app.sheet == Some(Sheet::KefOptions) {
        app.sheet = None;
    }
}

/// The field typing goes to, with the press that focuses it and its text.
pub(crate) fn field(app: &Faraday) -> Option<(Action, &str)> {
    if app.screen != Screen::Backup {
        return None;
    }
    let (a, t) = app.backup.kef_form(app.backup.kef_open()?)?.field()?;
    Some((Action::KefTool(KefToolAction::Backup(a)), t))
}

/// A key on the open item: typing in the form or in Options; Escape closes
/// Options. Returns whether it was taken.
pub(crate) fn key(app: &mut Faraday, key: Key) -> bool {
    if app.screen != Screen::Backup || app.backup.kef_open().is_none() {
        return false;
    }
    if app.sheet.is_some() && app.sheet != Some(Sheet::KefOptions) {
        return false;
    }
    if with_open(app, |f, app| f.key(app, key)).unwrap_or(false) {
        poll(app);
        sync(app);
        return true;
    }
    if key == Key::Escape && app.sheet == Some(Sheet::KefOptions) {
        act(app, KefAct::Back);
        return true;
    }
    false
}

/// After a worker's answer: the open form takes it.
pub(crate) fn poll(app: &mut Faraday) {
    if with_open(app, |f, app| f.poll(app)).unwrap_or(false) {
        sync(app);
        app.out.redraw();
    }
}

/// The dice roller's tick. Returns whether a frame is needed.
pub(crate) fn tick(app: &mut Faraday) -> bool {
    with_open(app, |f, app| f.tick(app)).unwrap_or(false)
}

/// Cancel while the work runs.
pub(crate) fn cancel(app: &mut Faraday) {
    if with_open(app, |f, app| f.cancel(app)).is_some() {
        sync(app);
    }
}

/// After a press: every form is put away once the page is left, unless it
/// is Add randomness, opened from the form, that will come back. Options
/// closed from outside is closed in the form too.
pub(crate) fn tidy(app: &mut Faraday) {
    if app.backup.state().is_none() {
        return;
    }
    let cur = app.backup.kef_open();
    let left = !matches!(app.screen, Screen::Backup | Screen::Randomness);
    for i in app.backup.kef_seeds() {
        if left || Some(i) != cur {
            with_form(app, i, |f, app| f.close(app));
        }
    }
    if left {
        return;
    }
    if app.sheet != Some(Sheet::KefOptions)
        && app
            .backup
            .kef_open()
            .and_then(|i| app.backup.kef_form(i))
            .is_some_and(KefForm::options_open)
    {
        with_open(app, |f, app| f.act(app, KefAct::Back));
    }
}

/// What the form's drawing asks of its owner here.
pub(crate) fn ask() -> Ask {
    Ask {
        wrap: |a| Action::KefTool(KefToolAction::Backup(a)),
        wrap_dice: |p| Action::KefTool(KefToolAction::BackupDice(p)),
        on_rolls: |_| Action::KefTool(KefToolAction::Backup(KefAct::DiceField)),
    }
}

/// The Options sheet, over the page.
pub(crate) fn options_sheet(app: &crate::drawing::View, ui: &mut crate::ui::Ui, w: f32, h: f32) {
    if let Some(f) = app.backup.kef_open().and_then(|i| app.backup.kef_form(i)) {
        crate::kef_form::screen::sheet(app, ui, (w, h), f, ask());
    }
}
