//! The checklist's Krux code item (`docs/KEF.md` §15.1): the form
//! ([`crate::kef_form`]) on the card of seed `i`'s code, made from the
//! seed itself. A form is kept for each seed whose item was opened; it is
//! put away (`KefForm::close`) when the item closes or the page is left,
//! keeping the code made and whether it was checked. The drawing is in
//! `screens/backup.rs`.

use super::WalletBackupArea;
use crate::backups::KefMark;
use crate::dice_entry::Press;
use crate::kef_form::screen::Ask;
use crate::kef_form::{KefAct, KefForm, Source};
use crate::kef_tool::KefToolAction;
use crate::receipt::sha256_of;
use crate::strings::EN;
use crate::worker::kef_make::{self, IdChoice, Options, Saved};
use crate::{Action, Faraday, Screen, Sheet, bstep, plan};
use faraday_crypto::work::Compute;
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

/// Seed `i`'s Krux code as it stands: from the form once the item was
/// opened, else from the record.
fn kept(app: &Faraday, i: usize) -> Option<KefMark> {
    match app.backup.kef_form(i) {
        Some(f) => {
            let m = f.made()?;
            let s = f.saved()?;
            Some(KefMark {
                png: s.png.clone(),
                file: s.file.clone(),
                checked: f.checked(),
                sha: m.sha,
                low: m.low,
                label: m.label,
                scanned: f.scanned(),
                side: side_of_made(m),
            })
        }
        None => app.kef_mark(i),
    }
}

/// The side in modules of a made code's picture, 0 where it fits none.
fn side_of_made(m: &kef_make::Made) -> u16 {
    m.view
        .as_ref()
        .ok()
        .and_then(|v| v.frames.first())
        .and_then(|q| u16::try_from(q.size()).ok())
        .unwrap_or(0)
}

/// Costs a map of the plan. Whether the plan puts seed `i`'s Krux code on
/// Paper at a place.
fn on_paper(app: &Faraday, c: &Compute, i: usize) -> bool {
    let Some(b) = app.backup.state() else {
        return false;
    };
    let shape = app.plan_shape_now(b.wallet);
    plan::kef_paper_on(&app.kept.plan_mapped(c, &shape, &b.answers), i)
}

/// Which box holds the first copy of `name`, in the order the stick's
/// writer takes: the Outbox (`false`), then Files (`true`).
fn box_of(app: &Faraday, name: &str) -> Option<bool> {
    if app.outbox.iter().any(|o| o.name == name) {
        Some(false)
    } else if app.inbox.iter().any(|o| o.name == name) {
        Some(true)
    } else {
        None
    }
}

/// Costs a SHA-256 pass over the file. The bytes of the `.kef` file `m`
/// names, from the first copy of the name (the Outbox, else Files, where
/// the files the flow wrote are moved to), when they are the bytes the
/// record has the hash of. A file of the name made since is another code:
/// the answer is then `None`.
fn kef_bytes(app: &Faraday, c: &Compute, m: &KefMark) -> Option<Vec<u8>> {
    app.outbox
        .iter()
        .chain(app.inbox.iter())
        .find(|o| o.name == m.file)
        .filter(|o| sha256_of(c, &o.bytes) == m.sha)
        .map(|o| o.bytes.clone())
}

/// Whether the files of code `m` are there by their names, the picture in
/// the box of the `.kef` file: in the Outbox or written (Files). Drawing
/// and the stick's list read this; the bytes are held to the hash where
/// the item is counted done and where it is taken up again.
fn named(app: &Faraday, m: &KefMark) -> bool {
    let at = box_of(app, &m.file);
    at.is_some() && m.png.as_deref().is_none_or(|p| box_of(app, p) == at)
}

/// Whether seed `i`'s Krux code is made and not yet opened back, by its
/// files' names: what the closed item reads Not checked for.
pub(crate) fn pending(app: &Faraday, i: usize) -> bool {
    kept(app, i).is_some_and(|m| !m.checked && named(app, &m))
}

/// Costs a SHA-256 pass over the `.kef` file. Whether seed `i`'s Krux code
/// was opened back and held the seed, with both its files there and the
/// `.kef` file's bytes the ones made.
///
/// On Paper the form sets `checked` only once the drawn copy was scanned
/// back, or with no camera to scan it with, when the check is made
/// (`docs/KEF.md` §15.1): a camera plugged in later does not undo it.
pub(crate) fn checked(app: &Faraday, c: &Compute, i: usize) -> bool {
    kept(app, i).is_some_and(|m| m.checked && named(app, &m) && kef_bytes(app, c, &m).is_some())
}

/// The side in modules of seed `i`'s Krux code
/// on a blank template: the code's if one is made and a sheet has its side, else §10's default
/// (`docs/KEF.md` §15.5).
pub(crate) fn template_side(app: &Faraday, i: usize) -> usize {
    kept(app, i)
        .map(|m| usize::from(m.side))
        .filter(|&s| s > 0 && s <= faraday_draw::tokens::KEF_BY_HAND_MAX_SIDE)
        .unwrap_or(faraday_draw::tokens::KEF_DEFAULT_SIDE)
}

/// Seed `i`'s Krux code files as the stick lists them: the picture's name
/// when it has one, and the `.kef` file's. Nothing while the code is not
/// made.
pub(crate) fn stick_names(app: &Faraday, i: usize) -> Vec<String> {
    let Some(m) = kept(app, i).filter(|m| named(app, m)) else {
        return Vec::new();
    };
    m.png.into_iter().chain([m.file]).collect()
}

/// Costs a SHA-256 pass, the picture and what `kef_make::made` costs.
/// Puts seed `i`'s Krux code back in the Outbox from Files, under its
/// names as the record has them: the bytes the record has the hash of and
/// the picture made from them. Only where neither name is taken in the
/// Outbox; no file is replaced. Nothing is put where the bytes are not
/// there.
pub(crate) fn stick_put(app: &mut Faraday, i: usize) {
    let Some(m) = kept(app, i) else {
        return;
    };
    let c = app.compute();
    let Some(bytes) = kef_bytes(app, &c, &m) else {
        return;
    };
    // A file of a name already in the Outbox is never replaced.
    let free = |n: &str| !app.outbox.iter().any(|o| o.name == n);
    if box_of(app, &m.file) != Some(true) || !free(&m.file) {
        return;
    }
    if let Some(name) = &m.png
        && !free(name)
    {
        return;
    }
    if let Some(name) = &m.png
        && let Some(made) = remade(&c, bytes.clone(), &m)
        && let Ok(view) = &made.view
        && let Some((_, png)) = view.png(&c)
    {
        app.put_outbox(name, png);
    }
    app.put_outbox(&m.file, bytes);
}

/// Costs what `kef_make::made` costs. The code of `bytes` as the record
/// `m` has it drawn; `None` where the bytes do not read.
fn remade(c: &Compute, bytes: Vec<u8>, m: &KefMark) -> Option<kef_make::Made> {
    let env = faraday_keys::kef::unwrap(&bytes).ok()?;
    let options = Options {
        version: env.version,
        low: m.low,
        count: env.count,
        id: if m.label {
            IdChoice::Label(String::new())
        } else {
            Options::default().id
        },
    };
    Some(kef_make::made(c, bytes, &options))
}

/// Keeps seed `i`'s code in the record as its form stands.
fn sync_record(app: &mut Faraday, i: usize) {
    let mark = app.backup.kef_form(i).and_then(|_| kept(app, i));
    app.backups_kef_set(i, mark);
}

/// Costs what `kef_make::made` costs. Takes up the code the record keeps
/// for seed `i` from its `.kef` file, in the Outbox or written, with no
/// passphrase: the file is read as anything from outside is, and is the
/// code only with the bytes the record has the hash of. A file that is
/// gone, differs or does not read leaves the form at the first step.
fn restore(app: &mut Faraday, i: usize, form: &mut KefForm) {
    let Some(m) = app.kef_mark(i) else {
        return;
    };
    let c = app.compute();
    let Some(bytes) = kef_bytes(app, &c, &m).filter(|_| named(app, &m)) else {
        return;
    };
    let Some(made) = remade(&c, bytes, &m) else {
        return;
    };
    form.restore(
        made,
        Saved {
            png: m.png,
            file: m.file,
        },
        m.checked,
        m.scanned,
    );
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
fn new_form(app: &Faraday, c: &Compute, i: usize) -> Option<KefForm> {
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
        on_paper(app, c, i),
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
        && let c = app.compute()
        && let Some(mut form) = new_form(app, &c, i)
    {
        restore(app, i, &mut form);
        app.backup.kef_put(i, form);
    }
    if let Some(i) = cur {
        sync_paper(app, i);
    }
    if let Some(i) = cur
        && app.backup.kef_form(i).is_some_and(|f| f.made().is_some())
    {
        sync_record(app, i);
    }
    sync(app);
}

/// Seed `i`'s form follows the map: on Paper where a place holds the
/// code.
fn sync_paper(app: &mut Faraday, i: usize) {
    let c = app.compute();
    let paper = on_paper(app, &c, i);
    with_form(app, i, |f, _| f.set_paper(paper));
}

/// Not on Paper: the open item's code comes off every place
/// (`docs/KEF.md` §15.1). The item goes with it where no stick holds the
/// code either. The drawn copy's scan stays marked: one scan per seed.
fn not_on_paper(app: &mut Faraday) {
    let Some(i) = app.backup.kef_open() else {
        return;
    };
    let Some(b) = app.backup.state() else {
        return;
    };
    let shape = app.plan_shape_now(b.wallet);
    let answers = plan::kef_not_on_paper(&shape, &b.answers, i);
    let w = b.wallet;
    app.backup.follow_answers(w, answers);
    app.plan_save();
    app.backups_sync();
    sync_paper(app, i);
    // The item gone with the code: the form is put away as closing does,
    // and the checklist moves on to the next item.
    let Some(n) = app.backup.state().and_then(|b| b.open) else {
        return;
    };
    let c = app.compute();
    let items = app.backup_items_now(&c);
    if !items.contains(&n) {
        with_form(app, i, |f, app| f.close(app));
        let next = items.iter().copied().find(|&j| j > n);
        app.backup_item_open(next);
    }
}

/// **Scan my copy**: the camera on the drawn code of the open item. A
/// lookup.
fn scan_open(app: &mut Faraday) {
    let Some(i) = app.backup.kef_open() else {
        return;
    };
    if !app.backup.kef_form(i).is_some_and(KefForm::scan_offered) || app.cameras().is_empty() {
        return;
    }
    let mut scan = crate::ScanState::default();
    scan.purpose = crate::ScanPurpose::CheckKef(i);
    app.camera.open(scan);
    app.sheet = Some(Sheet::Scan);
    app.out
        .commands
        .push_back(faraday_shell_api::Command::CameraOn);
}

/// What the camera read for seed `i`'s drawn Krux code: the code's base43
/// text, byte for byte, closes the camera and opens the field and Compare;
/// anything else is named on the scanner. A camera closed with no match
/// counts one miss, as a SeedQR's does. Nothing read
/// is loaded or filed.
pub(crate) fn scanned(app: &mut Faraday, i: usize, bytes: &[u8]) {
    let text = app.backup.kef_form(i).and_then(KefForm::code_text);
    let Some(text) = text else {
        app.camera.close();
        app.sheet = None;
        app.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOff);
        return;
    };
    if bytes.trim_ascii() == text.as_bytes() {
        with_form(app, i, |f, _| f.scan_matched());
        app.camera.close();
        app.sheet = None;
        app.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOff);
        sync_record(app, i);
    } else {
        if let Some(scan) = app.camera.scan_mut() {
            scan.note = Some(EN.kef_not_this_code.to_string());
        }
    }
}

/// The code made for seed `i` as a scanner's sample: its base43 text.
pub(crate) fn sample_text(app: &Faraday, i: usize) -> Option<String> {
    app.backup.kef_form(i)?.code_text()
}

/// A press on the form.
pub(crate) fn act(app: &mut Faraday, a: KefAct) {
    if a == KefAct::NotOnPaper {
        not_on_paper(app);
        return;
    }
    if a == KefAct::ScanCopy {
        scan_open(app);
        return;
    }
    let before = app
        .backup
        .kef_open()
        .is_some_and(|i| app.backup.kef_form(i).is_some_and(|f| f.made().is_some()));
    with_open(app, |f, app| f.act(app, a));
    poll(app);
    if let Some(i) = app.backup.kef_open()
        && (before || app.backup.kef_form(i).is_some_and(|f| f.made().is_some()))
    {
        sync_record(app, i);
    }
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
        if let Some(i) = app.backup.kef_open() {
            sync_record(app, i);
        }
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
        pin: |n| Action::KefTool(KefToolAction::Backup(KefAct::Pin(n))),
        rows: |r| Action::KefTool(KefToolAction::Backup(KefAct::Rows(r))),
        on_rolls: |_| Action::KefTool(KefToolAction::Backup(KefAct::DiceField)),
    }
}

/// The Options sheet, over the page.
pub(crate) fn options_sheet(app: &crate::drawing::View, ui: &mut crate::ui::Ui, w: f32, h: f32) {
    if let Some(f) = app.backup.kef_open().and_then(|i| app.backup.kef_form(i)) {
        crate::kef_form::screen::sheet(app, ui, (w, h), f, ask());
    }
}
