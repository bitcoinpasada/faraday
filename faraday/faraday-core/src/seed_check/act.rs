//! My seed makes my wallet's presses: what each card takes, the key read
//! by `faraday-keys` and the Result worked out, and every way out of the
//! flow, each of which drops the key but Keep this key.

use super::area::{Facts, SeedState, step};
use super::compare::{self, Compare};
use super::{Account, Refused, SeedAction as A, Source, coin, make};
use crate::secret_text::SecretText;
use crate::strings::EN;
use crate::{Action, Faraday, Screen};
use faraday_bip::keys::ScriptType;
use faraday_shell_api::Key;
use faraday_text::Text;

fn st(app: &mut Faraday) -> Option<&mut SeedState> {
    app.tools.seed_check.state_mut()
}

/// The words a mnemonic spells, as wiping text sized first so that no
/// copy is left behind by a reallocation.
fn words_text(m: &faraday_bip::bip39::Mnemonic) -> Text {
    let lang = m.language();
    let size: usize = m.indices().iter().map(|i| lang.word(*i).len() + 1).sum();
    let mut text = Text::with_room(size);
    for (k, i) in m.indices().iter().enumerate() {
        if k > 0 {
            text.push(' ');
        }
        text.push_str(lang.word(*i));
    }
    text
}

/// Opens the flow with nothing taken, remembering where it was opened
/// from.
pub(crate) fn open(app: &mut Faraday) {
    let back = app.screen;
    app.session.keys.check_forget();
    app.entry.reset();
    app.tools.seed_check.begin(back);
    app.screen = Screen::SeedCheck;
}

/// Check a multisig setup's Check a seed against it: the flow begins
/// with nothing taken, for that page's screen, which stays on show.
pub(crate) fn open_for_check(app: &mut Faraday) {
    app.session.keys.check_forget();
    app.entry.reset();
    app.tools.seed_check.begin(Screen::MultisigCheck);
}

/// What Check a seed against it held is wiped.
pub(crate) fn drop_for_check(app: &mut Faraday) {
    if app
        .tools
        .seed_check
        .state()
        .is_some_and(SeedState::for_check)
    {
        drop_all(app);
    }
}

/// Whether the flow's cards are on show: its own screen, or Check a
/// multisig setup's with the flow open.
pub(crate) fn here(app: &Faraday) -> bool {
    match app.screen {
        Screen::SeedCheck => true,
        Screen::MultisigCheck => {
            crate::multisig_check::state(app)
                .is_some_and(|m| m.open == Some(crate::multisig_check::mstep::RESULT))
                && app
                    .tools
                    .seed_check
                    .state()
                    .is_some_and(SeedState::for_check)
        }
        _ => false,
    }
}

/// A seed read for Check a seed against it is matched to the setup.
fn read_for_check(app: &mut Faraday) {
    if app
        .tools
        .seed_check
        .state()
        .is_some_and(|s| s.for_check() && s.facts.is_some())
    {
        crate::multisig_check::seed_given(app);
    }
}

/// Leaves the flow for where it was opened: everything it held is wiped.
fn leave(app: &mut Faraday) {
    let back = app.tools.seed_check.state().map(|s| s.back);
    drop_all(app);
    if let Some(back) = back {
        app.act(Action::Nav(back));
    }
}

/// What the flow held is wiped: its state, the entry form's words and
/// the seed `faraday-keys` holds for it.
fn drop_all(app: &mut Faraday) {
    app.session.keys.check_forget();
    app.tools.seed_check.clear();
    app.entry.reset();
}

/// The cards from `from` on are to be taken again: what they gave is
/// dropped, and the key too from Passphrase back.
fn reset_from(app: &mut Faraday, from: u8) {
    if from <= step::PASSPHRASE {
        app.session.keys.check_forget();
    }
    let Some(s) = st(app) else { return };
    if from == step::SEED {
        s.words = SecretText::new();
        s.source = None;
        s.typing = false;
    }
    if from <= step::PASSPHRASE {
        s.facts = None;
        s.adding = false;
        s.pass = SecretText::new();
        s.pass2 = SecretText::new();
        s.focus = None;
        s.source = None;
    }
    if from <= step::ACCOUNT {
        s.picked = None;
        s.account = None;
        s.path_open = false;
        s.path_focus = false;
        s.made = None;
        s.compare = None;
    }
    s.error = None;
}

pub(crate) fn act(app: &mut Faraday, action: A) {
    match action {
        A::Open => open(app),
        A::Step(k) => step_to(app, k),
        A::Show(k) => open_card(app, k),
        A::Type => {
            app.entry.reset();
            if let Some(s) = st(app) {
                s.typing = true;
                s.error = None;
            }
        }
        A::Words => words(app),
        A::Loaded(i) => loaded(app, i),
        A::NoPass => read(app, false),
        A::AddPass => {
            if crate::stick::refuses_secret(app) {
                return;
            }
            if let Some(s) = st(app) {
                s.adding = true;
                s.focus = Some(0);
            }
        }
        A::PassField(f) => {
            if crate::stick::refuses_secret(app) {
                return;
            }
            if let Some(s) = st(app) {
                s.focus = Some(f.min(1));
                s.path_focus = false;
            }
        }
        A::UsePass => {
            if app.typing.pass_eye.waits() {
                return;
            }
            if st(app).is_some_and(|s| !s.pass.is_empty()) && crate::stick::refuses_secret(app) {
                return;
            }
            read(app, true)
        }
        A::Pick(a) => pick(app, a),
        A::Other => {
            if let Some(s) = st(app) {
                s.path_open = true;
                s.path_focus = true;
                s.error = None;
            }
        }
        A::PathField => {
            if let Some(s) = st(app) {
                s.path_focus = true;
            }
        }
        A::Script(i) => {
            if let Some(s) = st(app)
                && let Some(&t) = ScriptType::ALL.get(usize::from(i))
            {
                s.script = t;
            }
        }
        A::UsePath => use_path(app),
        A::Spelling(plain) => {
            if let Some(s) = st(app) {
                s.plain = plain;
            }
        }
        A::Qr(which) => show_code(app, which),
        A::Another => {
            reset_from(app, step::ACCOUNT);
            open_card(app, step::ACCOUNT);
        }
        A::AnotherSeed => {
            reset_from(app, step::SEED);
            open_card(app, step::SEED);
        }
        A::Compare => {
            if let Some(s) = st(app)
                && s.made.is_some()
            {
                s.compare = Some(Compare {
                    focus: true,
                    ..Compare::default()
                });
            }
        }
        A::CompareField => {
            if let Some(c) = st(app).and_then(|s| s.compare_mut()) {
                c.focus = true;
            }
        }
        A::CompareScan => compare::scan(app),
        A::CompareFile(i) => compare_file(app, i),
        A::CompareGo => compare::run(app),
        A::Sample => app.seed_use_sample(),
        A::CompareSample => app.seed_compare_use_sample(),
        A::Keep => keep(app),
        A::Done | A::Cancel => leave(app),
    }
    if let Some(s) = st(app) {
        s.scroll.follow = true;
    }
}

fn open_card(app: &mut Faraday, k: u8) {
    if let Some(s) = st(app) {
        // A key that is not typed words has no passphrase to take again
        // without its seed, as `step_to` knows.
        s.open = Some(if k == step::PASSPHRASE && s.words.is_empty() {
            step::SEED
        } else {
            k
        });
    }
}

/// A card's header: it closes if open; a card before the one open is
/// taken again (**Change**), the key kept only where its card is left
/// alone. Back and Forward (`Show`) only open a card; what a card takes
/// drops the cards after it before it stores what it took.
fn step_to(app: &mut Faraday, k: u8) {
    let Some(s) = st(app) else { return };
    if s.open == Some(k) {
        s.open = None;
        return;
    }
    let reach = match k {
        step::SEED => true,
        step::PASSPHRASE => s.done(step::SEED),
        step::ACCOUNT => s.facts.is_some(),
        _ => s.made.is_some(),
    };
    if !reach {
        return;
    }
    // A key that is not typed words (a SeedQR read, a key loaded) has no
    // passphrase to take again without its seed: Seed takes it.
    let from = if k == step::PASSPHRASE && s.words.is_empty() {
        step::SEED
    } else {
        k
    };
    if k == step::RESULT {
        s.open = Some(k);
        return;
    }
    reset_from(app, from);
    if let Some(s) = st(app) {
        s.open = Some(from);
    }
}

/// The typed words, whole and with their checksum, are taken and the
/// Passphrase card opens.
fn words(app: &mut Faraday) {
    let m = match app.entry.mnemonic() {
        Ok(m) => m,
        Err(e) => {
            app.entry.set_error(e);
            return;
        }
    };
    let text = words_text(&m);
    app.entry.reset();
    reset_from(app, step::PASSPHRASE);
    if let Some(s) = st(app) {
        s.words = SecretText::of(&text);
        s.typing = false;
        s.error = None;
        s.open = Some(step::PASSPHRASE);
    }
}

/// Costs what `Keys::check_read` costs. The words held, with the
/// passphrase typed twice or none, are read into the key the check holds.
fn read(app: &mut Faraday, with: bool) {
    let c = app.compute();
    let Some(s) = st(app) else { return };
    if with && !crate::screens::pass_pair::ready(s.pass.as_str(), s.pass2.as_str()) {
        return;
    }
    let (words, pass) = (
        s.words.clone(),
        if with {
            s.pass.clone()
        } else {
            SecretText::new()
        },
    );
    let got = app.session.keys.check_read(&c, &words, &pass);
    let ok = got.is_ok();
    if got.is_ok() {
        reset_from(app, step::ACCOUNT);
    }
    let read_ok = got.is_ok();
    let Some(s) = st(app) else { return };
    match got {
        Ok(f) => {
            s.facts = Some(Facts {
                fingerprint: f.fingerprint,
                with_pass: !pass.is_empty(),
                words: Some(f.words),
            });
            s.source = Some(Source::Held);
            s.pass = SecretText::new();
            s.pass2 = SecretText::new();
            s.adding = false;
            s.focus = None;
            s.error = None;
            s.open = Some(step::ACCOUNT);
        }
        Err(e) => s.error = Some(e.text()),
    }
    if read_ok {
        app.typing.pass_eye.close();
        app.typing.pass_eye.reset();
    }
    if ok {
        read_for_check(app);
    }
}

/// Costs what `Keys::check_read` costs. A SeedQR's words and the passphrase the sheet asked for are read as
/// the key the check holds, and the Account card opens.
pub(crate) fn scanned(app: &mut Faraday, words: &str, pass: &str) {
    let c = app.compute();
    let got = app.session.keys.check_read(&c, words, pass);
    let ok = got.is_ok();
    if got.is_ok() {
        // The key just read is the one the check holds: only what hung on
        // the key before is dropped.
        reset_from(app, step::ACCOUNT);
    }
    let Some(s) = st(app) else { return };
    match got {
        Ok(f) => {
            s.pass = SecretText::new();
            s.pass2 = SecretText::new();
            s.adding = false;
            s.focus = None;
            s.words = SecretText::new();
            s.facts = Some(Facts {
                fingerprint: f.fingerprint,
                with_pass: !pass.is_empty(),
                words: Some(f.words),
            });
            s.source = Some(Source::Held);
            s.typing = false;
            s.error = None;
            s.open = Some(step::ACCOUNT);
        }
        Err(e) => {
            s.error = Some(e.text());
            s.open = Some(step::SEED);
        }
    }
    if ok {
        read_for_check(app);
    }
}

/// Words a scanner read that are no BIP-39 seed: the Seed card says so.
pub(crate) fn not_a_seed(app: &mut Faraday) {
    if let Some(s) = st(app) {
        s.error = Some(Text::of(EN.seed_check_not_seed));
        s.open = Some(step::SEED);
    }
}

/// The key loaded at `i` is the key as loaded: Passphrase is skipped.
fn loaded(app: &mut Faraday, i: usize) {
    let Some(info) = app.session.keys.all().nth(i) else {
        return;
    };
    let facts = Facts {
        fingerprint: info.fingerprint,
        with_pass: info.has_passphrase,
        words: info.word_count.map(usize::from),
    };
    reset_from(app, step::PASSPHRASE);
    let Some(s) = st(app) else { return };
    s.words = SecretText::new();
    s.facts = Some(facts);
    s.source = Some(Source::Loaded(i));
    s.error = None;
    s.open = Some(step::ACCOUNT);
}

fn pick(app: &mut Faraday, a: Account) {
    let net = app.session.keys.network();
    let path = a.path(coin(net));
    let Some(s) = st(app) else { return };
    s.account = Some(a);
    work(app, &path, ScriptType::NativeSegwit);
}

fn use_path(app: &mut Faraday) {
    let Some(s) = st(app) else { return };
    let (path, script, picked) = (s.path.clone(), s.script, s.account);
    s.account = None;
    work(app, &path, script);
    // A path that is refused leaves the account picked as it was.
    if let Some(s) = st(app)
        && s.error.is_some()
        && s.account.is_none()
    {
        s.account = picked;
    }
}

/// Costs what [`make`] costs. The Result at `path`.
fn work(app: &mut Faraday, path: &str, asked: ScriptType) {
    let c = app.compute();
    let net = app.session.keys.network();
    let Some(source) = st(app).and_then(|s| s.source) else {
        return;
    };
    let got = make(&c, &app.session.keys, source, path, asked, net);
    let Some(s) = st(app) else { return };
    match got {
        Ok(m) => {
            let name = match s.account {
                Some(a) => a.name(),
                None => m.script,
            };
            s.picked = Some((Text::of(name), m.path.clone()));
            s.made = Some(m);
            s.compare = None;
            s.path_open = false;
            s.path_focus = false;
            s.error = None;
            s.open = Some(step::RESULT);
        }
        Err(Refused::Path(line)) => s.error = Some(Text::of(line)),
        // The key went (a lock): the flow starts over.
        Err(Refused::Key) => {
            let back = s.back;
            app.session.keys.check_forget();
            app.tools.seed_check.begin(back);
        }
    }
}

/// Costs what [`compare::put`] costs. The file at `i` in Files, a wallet
/// description or a key, is the value compared, and is compared.
fn compare_file(app: &mut Faraday, i: usize) {
    use crate::wallet::FileKind;
    let Some(item) = app.inbox.get(i) else { return };
    if !matches!(item.kind, FileKind::Wallet | FileKind::Key) {
        return;
    }
    let text = Text::of(Text::from_utf8_lossy(&item.bytes).trim());
    compare::put(app, text);
}

/// The xpub's or the receive address's code, as the Result shows it.
fn show_code(app: &mut Faraday, which: u8) {
    let c = app.compute();
    let Some(m) = app
        .tools
        .seed_check
        .state()
        .and_then(|s| s.made().map(|m| (m, s.plain)))
    else {
        return;
    };
    let (m, plain) = m;
    let (title, text) = if which == 0 {
        let t = match (&m.spelled, plain) {
            (Some(s), false) => s.clone(),
            _ => m.plain.clone(),
        };
        (EN.seed_check_xpub_code, t)
    } else {
        match &m.receive {
            Some(t) => (EN.seed_check_receive_code, t.clone()),
            None => return,
        }
    };
    app.open_qr(crate::qr_view::QrView::text(&c, title, &text));
}

/// Costs what `Keys::check_keep` costs. Keep this key: loaded as Add a key loads typed words, then back where
/// the flow was opened. Refused as Add a key refuses.
fn keep(app: &mut Faraday) {
    if !app.may_load_keys() || st(app).is_none_or(|s| s.is_loaded() || s.made.is_none()) {
        return;
    }
    let c = app.compute();
    match app.session.keys.check_keep(&c, "") {
        Ok(fp) => {
            app.toast(&crate::strings::fill(
                EN.entry_add_key_added,
                &[crate::wallet::fp_text(fp).as_str()],
            ));
            app.refresh_spend();
            leave(app);
        }
        Err(e) => {
            if let Some(s) = st(app) {
                s.error = Some(e.text());
            }
        }
    }
}

/// The field typing goes to, with what is typed in it.
pub(crate) fn field(app: &Faraday) -> Option<(Action, &str)> {
    if !here(app) {
        return None;
    }
    let (a, text) = app.tools.seed_check.state()?.field()?;
    Some((Action::SeedCheck(a), text))
}

/// A stick arrived or the stick rule refused: the passphrase fields lose
/// focus, and with `wipe` what is typed in them goes.
pub(crate) fn stick_attached(app: &mut Faraday, wipe: bool) {
    if let Some(s) = st(app) {
        s.focus = None;
        if wipe {
            s.pass.clear();
            s.pass2.clear();
        }
    }
}

/// Whether Enter now ends the passphrase fields with two PBKDF2, so the
/// key is left for the tick after a frame (`Faraday::key_slow`).
pub(crate) fn enter_is_slow(app: &Faraday) -> bool {
    if !here(app) || app.sheet.is_some() {
        return false;
    }
    let Some(s) = app.tools.seed_check.state() else {
        return false;
    };
    s.field()
        .is_some_and(|(target, _)| target != A::PathField && target != A::CompareField)
        && !app.typing.pass_eye.waits()
        && crate::screens::pass_pair::ready(s.pass.as_str(), s.pass2.as_str())
}

/// Typing into the passphrase fields and the path field.
pub(crate) fn key(app: &mut Faraday, key: Key) -> bool {
    if !here(app) || app.sheet.is_some() {
        return false;
    }
    let Some(s) = st(app) else { return false };
    let Some((target, _)) = s.field() else {
        return false;
    };
    let on_path = target == A::PathField;
    // Compare reads no secret: its field is typed into with a stick in.
    if target == A::CompareField {
        return compare_key(app, key);
    }
    if !on_path && crate::stick::refuses_secret_key(app, key) {
        return true;
    }
    let Some(s) = st(app) else { return false };
    match key {
        Key::Enter => {
            let again = if on_path { A::UsePath } else { A::UsePass };
            act(app, again);
            true
        }
        Key::Escape => {
            s.focus = None;
            s.path_focus = false;
            true
        }
        Key::Tab if !on_path => {
            s.focus = Some(1 - s.focus.unwrap_or(0).min(1));
            true
        }
        Key::Backspace => {
            if on_path {
                s.path.pop();
            } else if s.focus == Some(0) {
                s.pass.pop();
            } else {
                s.pass2.pop();
            }
            s.error = None;
            true
        }
        Key::Char(ch) if on_path => {
            if (ch.is_ascii_digit() || matches!(ch, 'h' | '\'' | '/')) && s.path.len() < 60 {
                s.path.push(ch);
            }
            s.error = None;
            true
        }
        Key::Char(ch) if (' '..='~').contains(&ch) => {
            if s.focus == Some(0) {
                s.pass.push(ch);
            } else {
                s.pass2.push(ch);
            }
            s.error = None;
            true
        }
        Key::Char(ch) if !ch.is_control() => {
            s.error = Some(Text::of(crate::state::ascii_refused()));
            true
        }
        _ => false,
    }
}

/// Typing into the comparison's field.
fn compare_key(app: &mut Faraday, key: Key) -> bool {
    // A line break typed back after an edit in the middle of a value is a
    // character of it, not Compare.
    let editing = app.typing.editing();
    let Some(c) = st(app).and_then(|s| s.compare_mut()) else {
        return false;
    };
    match key {
        Key::Enter if editing => {
            if c.text.len() < compare::FIELD_CHARS {
                c.text.push('\n');
            }
            c.answer = None;
            true
        }
        Key::Enter => {
            act(app, A::CompareGo);
            true
        }
        Key::Escape => {
            c.focus = false;
            true
        }
        Key::Backspace => {
            c.text.pop();
            c.answer = None;
            true
        }
        Key::Char(ch) if !ch.is_control() => {
            if c.text.len() < compare::FIELD_CHARS {
                c.text.push(ch);
            }
            c.answer = None;
            true
        }
        _ => false,
    }
}

/// After a press: the flow ends, its key dropped, unless it is on show;
/// and a key that went with a lock starts it over.
pub(crate) fn tidy(app: &mut Faraday) {
    if !here(app) {
        if app.tools.seed_check.state().is_some() {
            drop_all(app);
        }
        return;
    }
    let lost = app
        .tools
        .seed_check
        .state()
        .is_some_and(|s| match s.source {
            Some(Source::Held) => app.session.keys.check_facts().is_none(),
            Some(Source::Loaded(i)) => app.session.keys.all().nth(i).is_none(),
            None => false,
        });
    if lost {
        let back = app
            .tools
            .seed_check
            .state()
            .map_or(Screen::Catalog, |s| s.back);
        app.tools.seed_check.begin(back);
    }
}

/// The flow's cards, plugged into Back and Forward.
pub(crate) struct SeedSteps;

/// The one [`SeedSteps`].
pub(crate) static SEED_STEPS: SeedSteps = SeedSteps;

impl crate::trail::StepFlow for SeedSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = app.tools.seed_check.state()?;
        // A key with no words has no passphrase card to go through.
        let no_pass = s.words.is_empty() && s.facts.is_some();
        Some(crate::trail::Steps::new(
            (0..step::COUNT as u8)
                .filter(|&k| !(no_pass && k == step::PASSPHRASE))
                .collect(),
            s.open,
        ))
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::SeedCheck(A::Show(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        super::screen::TITLES[usize::from(k)]
    }
}

/// Practice's public passphrase in both fields, the chip's press.
pub(crate) fn fill_demo_pass(app: &mut Faraday) {
    if let Some(s) = st(app) {
        s.pass.set(crate::demo::PASSPHRASE);
        s.pass2.set(crate::demo::PASSPHRASE);
        s.error = None;
    }
}

/// Whether the Seed card shows its list of ways in, where Practice's
/// sample seed is offered (the words form has its own field's chip).
pub(crate) fn seed_sample_open(app: &Faraday) -> bool {
    app.tools
        .seed_check
        .state()
        .is_some_and(|s| s.open == Some(step::SEED) && !s.typing && s.source.is_none())
}

/// Practice's sample seed `words` taken as typed words: the Passphrase
/// card opens, as after Continue.
pub(crate) fn put_sample_words(app: &mut Faraday, words: &str) {
    reset_from(app, step::PASSPHRASE);
    if let Some(s) = st(app) {
        s.words = SecretText::of(words);
        s.typing = false;
        s.error = None;
        s.open = Some(step::PASSPHRASE);
    }
}

/// What a wallet app shows of the account picked, for Practice's sample
/// in the comparison's field: its key with the origin. `None` unless the
/// Result is open with the field empty.
pub(crate) fn compare_sample_text(app: &Faraday) -> Option<Text> {
    let s = app.tools.seed_check.state()?;
    let made = s.made()?;
    let cmp = s.compare()?;
    (s.open == Some(step::RESULT) && cmp.text.is_empty() && cmp.answer.is_none()).then(|| {
        Text::formatted(format_args!(
            "{}{}",
            made.origin.as_str(),
            made.plain.as_str()
        ))
    })
}

/// Practice's sample put in the comparison's field as typed.
pub(crate) fn put_compare_sample(app: &mut Faraday, text: Text) {
    if let Some(c) = st(app).and_then(|s| s.compare_mut()) {
        c.text = text;
        c.answer = None;
    }
}
