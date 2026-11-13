//! The QR vault tool's Open card offers (`docs/VAULT.md` §13 Open): what
//! an opened item does next, as the same item scanned in its own form
//! would. The item stays in `faraday-keys`; each offer borrows it through
//! its named reveal and lets go, and a key loads only with no stick
//! plugged in (`docs/SECURITY-MODEL.md`).

use faraday_keys::{QrError, QrHolds, QrPassOffer, Reveal};
use zeroize::Zeroizing;

use super::st;
use crate::scan_read::Kind;
use crate::strings::{EN, fill};
use crate::{Faraday, Screen};

/// The line the stick rule is told in, when a stick is plugged in.
fn stick_line(app: &Faraday) -> Option<String> {
    (!app.stick.is_empty()).then(|| crate::stick::keys_load(app.medium))
}

/// A refusal from loading the opened seed, on the Open card.
pub(crate) fn say_error(app: &mut Faraday, line: Option<String>) {
    say(app, line);
}

/// Says `line` on the Open card.
fn say(app: &mut Faraday, line: Option<String>) {
    if let Some(s) = st(app) {
        s.error = line;
    }
}

/// Costs a PBKDF2 for each key with words and an elliptic-curve
/// multiplication, for a passphrase. An answer has just been held: a seed
/// opens the seed sheet at once, as a scanned SeedQR does, and a
/// passphrase works out what it does for the keys loaded.
pub(super) fn after_open(app: &mut Faraday) {
    match app.session.keys.qr_holds() {
        Some(QrHolds::Seed { .. }) => seed(app),
        Some(QrHolds::Passphrase { .. }) => refresh_pass(app),
        _ => {}
    }
}

/// Costs what `Keys::qr_passphrase_offer` costs. What the held passphrase
/// does for the keys loaded, worked out again.
fn refresh_pass(app: &mut Faraday) {
    let c = app.compute();
    let offer = app.session.keys.qr_passphrase_offer(&c);
    if let Some(s) = st(app) {
        s.pass_offer = offer;
    }
}

/// The opened seed on the seed sheet: **Use without a passphrase** or
/// **Add a passphrase**, then the flow of a scanned SeedQR. The words wait
/// on the sheet as a scanned SeedQR's do, and go with it.
pub(super) fn seed(app: &mut Faraday) {
    if app.screen != Screen::QrTool {
        return;
    }
    if let Some(line) = stick_line(app) {
        return say(app, Some(line));
    }
    let words = Zeroizing::new(app.session.keys.qr_words(Reveal::QrVault).to_string());
    if words.is_empty() {
        return;
    }
    say(app, None);
    // Add a key's form starts empty: nothing left from an earlier visit is
    // read, and the words reach it only to be refused or loaded.
    app.entry.reset();
    app.seed_read_open(&words);
    app.seed_read_marked(Kind::QrVault);
}

/// The opened codex32 share in Add a key's codex32 form, typed in.
pub(super) fn share(app: &mut Faraday) {
    if let Some(line) = stick_line(app) {
        return say(app, Some(line));
    }
    let given = Zeroizing::new(app.session.keys.qr_share(Reveal::QrVault).to_string());
    if given.is_empty() {
        return;
    }
    say(app, None);
    app.codex32_form_with(&given);
}

/// Adds the opened passphrase to the loaded key whose words make the key
/// it names.
pub(super) fn add_to_key(app: &mut Faraday) {
    if let Some(line) = stick_line(app) {
        return say(app, Some(line));
    }
    let Some(QrPassOffer::Add(base)) = st(app).and_then(|s| s.pass_offer) else {
        return;
    };
    let label = app
        .session
        .keys
        .find(base)
        .map(|k| fill(EN.seed_read_key_passphrase, &[&k.label.to_string()]))
        .unwrap_or_default();
    let c = app.compute();
    match app.session.keys.qr_add_passphrase(&c, &label) {
        Ok(fp) => {
            say(app, None);
            app.toast(&fill(EN.qr_tool_pass_added, &[&fp.to_string()]));
        }
        Err(QrError::Key(r)) if !matches!(r, crate::wallet::Refusal::Duplicate(_)) => {
            return say(app, Some(String::from(&*r.text())));
        }
        Err(_) => {}
    }
    refresh_pass(app);
}

/// Saves the opened note as a new note last in the open vault.
pub(super) fn save_note(app: &mut Faraday) {
    let Some(o) = app.vaults.open.get(app.vaults.current) else {
        return;
    };
    let (id, name) = (o.id, o.name.clone());
    if st(app).is_none_or(|s| s.note_saved == Some(id)) {
        return;
    }
    let c = app.compute();
    let keys = &app.session.keys;
    match app.vaults.edit(id, |v| v.qr_save_note(&c, keys, id)) {
        Ok(_) => {
            app.vaults.changed(id);
            if let Some(s) = st(app) {
                s.note_saved = Some(id);
            }
            say(app, None);
            app.toast(&fill(EN.qr_tool_note_saved, &[&name]));
        }
        Err(QrError::Short(len)) => {
            let line = app.vaults.short_text(id, len);
            say(app, Some(line));
        }
        Err(QrError::NoRoom) => {
            let line = if app.vaults.store().read_only(id).is_some() {
                EN.vaults_cannot_write
            } else {
                EN.vaults_slot_too_small
            };
            say(app, Some(line.to_string()));
        }
        Err(_) => {}
    }
}

/// Practice: the sample QR vault code is brought to the Open card, as a
/// scanned one is (`docs/DEMO.md` §14.1).
pub(super) fn use_sample(app: &mut Faraday) {
    let Some(set) = app.put_on() else { return };
    let code = set.inputs.qr_vault;
    if code.is_empty() || st(app).is_none_or(|s| s.code.is_some()) {
        return;
    }
    // A fresh Open state, as a scanned code's: nothing from Make shows.
    app.session.keys.qr_forget();
    if let Some(s) = st(app) {
        *s = super::QrToolState::new(s.back);
        s.code = Some(zeroize::Zeroizing::new(code.to_vec()));
        s.open = Some(super::qstep::CONTENTS);
        s.focus = Some(super::Field::Pass);
    }
}

/// A press on Practice's **Use faraday** in the tool's passphrase fields:
/// the Open card's, or Make's pair (field 0 fills its repeat too, where
/// that is empty).
pub(crate) fn fill_faraday(app: &mut Faraday, field: crate::demo::PassField) {
    use crate::demo::PassField as P;
    let word = crate::samples::PASSPHRASE;
    let Some(s) = st(app) else { return };
    match field {
        P::QrOpen => s.pass.text.set(word),
        P::QrMake(0) => {
            if s.pass.text.is_empty() {
                s.pass.text.set(word);
            }
            if s.again.text.is_empty() {
                s.again.text.set(word);
            }
        }
        P::QrMake(_) => {
            if s.again.text.is_empty() {
                s.again.text.set(word);
            }
        }
        _ => return,
    }
    s.edited();
}
