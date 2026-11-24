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

/// Shares and parts waiting in the entry form are wiped, and with them
/// the flag that the Passphrase card reads them: every way into the Seed
/// card that is not the parts' own starts clean.
fn drop_waiting(app: &mut Faraday) {
    if st(app).is_some_and(|s| s.shares) {
        app.entry.reset();
        if let Some(s) = st(app) {
            s.shares = false;
        }
    }
}

/// The cards from `from` on are to be taken again: what they gave is
/// dropped, and the key too from Passphrase back.
fn reset_from(app: &mut Faraday, from: u8) {
    reset_cards(app, from, true);
}

/// [`reset_from`], the key kept where `forget` is false: the passphrase
/// taken again from the words `faraday-keys` holds.
fn reset_cards(app: &mut Faraday, from: u8, forget: bool) {
    if from <= step::PASSPHRASE && forget {
        app.session.keys.check_forget();
    }
    if from == step::SEED {
        app.entry.reset();
    }
    let Some(s) = st(app) else { return };
    if from == step::SEED {
        s.words = SecretText::new();
        s.typed = false;
        s.shares = false;
        s.source = None;
        s.typing = false;
    }
    if from <= step::PASSPHRASE {
        s.facts = None;
        s.adding = false;
        s.pass.clear();
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
            drop_waiting(app);
            app.entry.reset();
            if let Some(s) = st(app) {
                s.typing = true;
                s.error = None;
            }
        }
        A::Words => words(app),
        A::Form(i) => form_pick(app, i),
        A::FormDone => form_done(app),
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
        s.open = Some(if k == step::PASSPHRASE && !s.typed && !s.shares {
            step::SEED
        } else {
            k
        });
    }
}

/// A card's header: a card before the one open is
/// taken again (**Change**), the key kept only where its card is left
/// alone. Back and Forward (`Show`) only open a card; what a card takes
/// drops the cards after it before it stores what it took.
fn step_to(app: &mut Faraday, k: u8) {
    let Some(s) = st(app) else { return };
    if s.open == Some(k) {
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
    let from = if k == step::PASSPHRASE && !s.typed && !s.shares {
        step::SEED
    } else {
        k
    };
    if k == step::RESULT {
        s.open = Some(k);
        return;
    }
    // Typed words already read are held by `faraday-keys` alone: the
    // passphrase is taken again from them, the key kept until it is.
    let reread = from == step::PASSPHRASE && s.typed && s.words.is_empty();
    reset_cards(app, from, !reread);
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
    take_words(app, &words_text(&m));
}

/// The words, whole and with their checksum, are held until the passphrase
/// is chosen, and the Passphrase card opens.
fn take_words(app: &mut Faraday, text: &Text) {
    drop_waiting(app);
    app.entry.reset();
    reset_from(app, step::PASSPHRASE);
    if let Some(s) = st(app) {
        s.words = SecretText::of(text);
        s.typed = true;
        s.typing = false;
        s.error = None;
        s.open = Some(step::PASSPHRASE);
    }
}

/// The words a Krux code opened, read from the Seed card's scanner or From
/// Files: taken as typed words are. Whether the card was on show to take
/// them.
pub(crate) fn take_scanned_words(app: &mut Faraday, words: &str) -> bool {
    if !here(app) || st(app).is_none() {
        return false;
    }
    take_words(app, &Text::of(words));
    true
}

/// A row of the forms the Seed card takes: its parts are typed there.
fn form_pick(app: &mut Faraday, i: u8) {
    let Some(&form) = super::FORMS.get(usize::from(i)) else {
        return;
    };
    drop_waiting(app);
    app.entry.choose_form(form);
    if let Some(s) = st(app) {
        s.typing = true;
        s.error = None;
        s.open = Some(step::SEED);
    }
}

/// A part of a form the Seed card takes, scanned or pasted: it goes in as
/// if typed. Whether it was one, which is only when the flow is on show
/// and `form` is in [`FORMS`](super::FORMS).
pub(crate) fn take_part(app: &mut Faraday, form: crate::forms::Form, text: &str) -> bool {
    if !here(app) || form == crate::forms::Form::Krux || !super::FORMS.contains(&form) {
        return false;
    }
    drop_waiting(app);
    let Some(s) = st(app) else { return false };
    if !s.typing || app.entry.state().form != form {
        app.entry.choose_form(form);
    }
    if let Some(s) = st(app) {
        s.typing = true;
        s.shares = false;
        s.error = None;
        s.open = Some(step::SEED);
    }
    app.entry.set_typed(text);
    app.form_add_part();
    true
}

/// The parts typed are enough: Seed XOR's make the words, which go on as
/// typed words; a codex32 backup's seed is the key the check holds, with
/// no words and no passphrase; SLIP-39's shares wait for the passphrase,
/// held by the entry form, which wipes them on drop.
pub(crate) fn form_done(app: &mut Faraday) {
    use crate::forms::Form;
    let form = app.entry.state().form;
    if !app.entry.state().parts.ready(form) {
        return;
    }
    match form {
        Form::Xor => {
            let made = app
                .entry
                .state()
                .parts
                .xor
                .entropy()
                .map_err(|_| Text::of(EN.forms_seed_xor_needs))
                .and_then(|e| {
                    faraday_keys::words::from_entropy(
                        faraday_bip::bip39::Language::English,
                        e.as_bytes(),
                    )
                    .map_err(|e| faraday_strings::bip::bip39_error(&e))
                });
            match made {
                Ok(m) => take_words(app, &words_text(&m)),
                Err(e) => app.entry.set_error(e),
            }
        }
        Form::Codex32 => {
            let c = app.compute();
            // The strings typed fewer than `k`, the open vaults' strings of
            // the set make up the rest, as Add a key reads them.
            let typed = &app.entry.state().parts.codex32;
            let seed = match app.vaults.store().codex32_seed_with_typed(&c, typed) {
                Some(r) => r.map_err(|e| e.text()),
                None => faraday_keys::codex32_seed(typed).map_err(|e| {
                    Text::fill(
                        EN.forms_the_strings_do,
                        &[&faraday_strings::bip::codex32_error(&e)],
                    )
                }),
            };
            let got = seed.and_then(|seed| {
                app.session
                    .keys
                    .check_read_seed(&c, &seed)
                    .map_err(|e| e.text())
            });
            if got.is_ok() {
                // The set's facts and strings go with the key the check
                // holds, for Keep this key to keep them as Add a key does.
                let typed = &app.entry.state().parts.codex32;
                if let Some(facts) = crate::forms::set_facts(typed) {
                    let texts: Vec<Text> = typed
                        .iter()
                        .map(|s| Text::of(s.encode().as_str()))
                        .collect();
                    let refs: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();
                    app.session.keys.check_set_codex32(facts, &refs);
                }
            }
            seed_taken(app, got, false);
        }
        Form::Slip39 => {
            reset_from(app, step::PASSPHRASE);
            if let Some(s) = st(app) {
                s.shares = true;
                s.typing = false;
                s.error = None;
                s.open = Some(step::PASSPHRASE);
            }
        }
        // Krux's code is read by its scanner or From Files, never put
        // together from parts.
        Form::Words | Form::Krux | Form::Xprv | Form::HexSeed => {}
    }
}

/// Costs what `slip39_seed` and `Keys::check_read_seed` cost. The shares
/// held, with the passphrase typed twice or none, make the key the check
/// holds.
fn shares_read(app: &mut Faraday, pass: &SecretText) {
    let c = app.compute();
    let got = faraday_keys::slip39_seed(
        &c,
        &app.entry.state().parts.slip39,
        pass.as_str().as_bytes(),
    )
    .map_err(|e| {
        Text::fill(
            EN.forms_the_shares_do,
            &[&faraday_strings::bip::slip39_error(&e)],
        )
    })
    .and_then(|seed| {
        app.session
            .keys
            .check_read_seed(&c, &seed)
            .map_err(|e| e.text())
    });
    seed_taken(app, got, !pass.is_empty());
}

/// A master seed read as the key the check holds, or what refused: the
/// parts are wiped and the Account card opens, with no words to take a
/// passphrase again from.
fn seed_taken(app: &mut Faraday, got: Result<faraday_keys::CheckFacts, Text>, with_pass: bool) {
    let ok = got.is_ok();
    if ok {
        reset_from(app, step::ACCOUNT);
        app.entry.reset();
        app.typing.pass_eye.close();
        app.typing.pass_eye.reset();
    }
    let Some(s) = st(app) else { return };
    match got {
        Ok(f) => {
            s.facts = Some(Facts {
                fingerprint: f.fingerprint,
                with_pass,
                words: f.words,
            });
            s.source = Some(Source::Held);
            s.words = SecretText::new();
            s.pass.clear();
            s.typed = false;
            s.shares = false;
            s.typing = false;
            s.adding = false;
            s.focus = None;
            s.error = None;
            s.open = Some(step::ACCOUNT);
        }
        Err(e) => {
            // The shares' refusal is said on the Passphrase card, the
            // strings' on the form.
            if s.shares {
                s.error = Some(e);
            } else {
                app.entry.set_error(e);
            }
        }
    }
    if ok {
        read_for_check(app);
    }
}

/// Costs what `Keys::check_read` costs. The words held, with the
/// passphrase typed twice or none, are read into the key the check holds.
fn read(app: &mut Faraday, with: bool) {
    let c = app.compute();
    let Some(s) = st(app) else { return };
    if with && !s.pass.ready() {
        return;
    }
    let pass = if with {
        s.pass.job_copy()
    } else {
        SecretText::new()
    };
    if s.shares {
        return shares_read(app, &pass);
    }
    if !s.typed {
        return;
    }
    // Words already read are held by `faraday-keys` alone.
    let words = s.words.clone();
    let got = if words.is_empty() {
        app.session.keys.check_reread(&c, &pass)
    } else {
        app.session.keys.check_read(&c, &words, &pass)
    };
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
                words: f.words,
            });
            s.source = Some(Source::Held);
            s.words = SecretText::new();
            s.pass.clear();
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
    drop_waiting(app);
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
            s.pass.clear();
            s.adding = false;
            s.focus = None;
            s.words = SecretText::new();
            s.typed = false;
            s.facts = Some(Facts {
                fingerprint: f.fingerprint,
                with_pass: !pass.is_empty(),
                words: f.words,
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
    drop_waiting(app);
    reset_from(app, step::PASSPHRASE);
    let Some(s) = st(app) else { return };
    s.words = SecretText::new();
    s.typed = false;
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

/// Costs what `Keys::check_keep` costs, or for a codex32 set finished with an open vault's strings what
/// `OpenVaults::codex32_with_typed` costs (up to 16 interpolations, HMAC-SHA512s and fingerprints, the same
/// work as the press that finishes the set). Keep this key: loaded as Add a key loads typed words, then back where
/// the flow was opened. Refused as Add a key refuses.
fn keep(app: &mut Faraday) {
    if !app.may_load_keys() || st(app).is_none_or(|s| s.is_loaded() || s.made.is_none()) {
        return;
    }
    let c = app.compute();
    // A set finished with an open vault's strings keeps what Add a key keeps.
    let kept = match app
        .vaults
        .store()
        .check_keep_codex32(&c, &mut app.session.keys, "")
    {
        Some(r) => r,
        None => app.session.keys.check_keep(&c, ""),
    };
    match kept {
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
/// focus, and with `wipe` the words typed and the shares waiting go (the
/// passphrase is emptied by the passphrase field).
pub(crate) fn stick_attached(app: &mut Faraday, wipe: bool) {
    if let Some(s) = st(app) {
        s.focus = None;
        if wipe && !s.words.is_empty() {
            // Words typed and not yet read are gone, so Passphrase has
            // nothing to take; once read, `faraday-keys` holds them.
            s.words.clear();
            s.typed = false;
            if s.open == Some(step::PASSPHRASE) {
                s.open = Some(step::SEED);
            }
        }
    }
    // Shares waiting for their passphrase are gone too.
    if wipe && st(app).is_some_and(|s| s.shares) {
        app.entry.reset();
        if let Some(s) = st(app) {
            s.shares = false;
            if s.open == Some(step::PASSPHRASE) {
                s.open = Some(step::SEED);
            }
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
        && s.pass.ready()
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
    if !on_path {
        return pass_key(app, key);
    }
    let Some(s) = st(app) else { return false };
    match key {
        Key::Enter => {
            act(app, A::UsePath);
            true
        }
        Key::Escape => {
            s.focus = None;
            s.path_focus = false;
            true
        }
        Key::Backspace => {
            s.path.pop();
            s.error = None;
            true
        }
        Key::Char(ch) => {
            if (ch.is_ascii_digit() || matches!(ch, 'h' | '\'' | '/')) && s.path.len() < 60 {
                s.path.push(ch);
            }
            s.error = None;
            true
        }
        _ => false,
    }
}

/// Typing into the passphrase fields: the field's own keys, with the stick
/// rule; Enter reads, Escape lets go of the field.
fn pass_key(app: &mut Faraday, key: Key) -> bool {
    use crate::pass_field::{Gate, PassKey};
    let (gate, medium) = (
        Gate {
            stick: app.stick.in_the_way(),
        },
        app.medium,
    );
    let Some(s) = st(app) else { return false };
    s.pass.set_on_again(s.focus == Some(1));
    match s.pass.key(key, gate) {
        PassKey::Typed => {
            s.focus = Some(u8::from(s.pass.on_again()));
            s.error = None;
            true
        }
        PassKey::Refused(why) => {
            s.error = Some(crate::pass_field::refused_text(medium, why));
            true
        }
        PassKey::Enter => {
            act(app, A::UsePass);
            true
        }
        PassKey::Escape => {
            s.focus = None;
            true
        }
        PassKey::Ignored => false,
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
        let no_pass = !s.typed && s.facts.is_some();
        let order: Vec<u8> = (0..step::COUNT as u8)
            .filter(|&k| !(no_pass && k == step::PASSPHRASE))
            .collect();
        let mut steps = crate::trail::Steps::new(order.clone(), s.open);
        steps.answered = order.into_iter().filter(|&k| s.done(k)).collect();
        Some(steps)
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::SeedCheck(A::Show(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        super::screen::TITLES[usize::from(k)]
    }

    fn contained(&self) -> Option<&dyn crate::flow::Contained> {
        Some(self)
    }
}

impl crate::flow::Contained for SeedSteps {
    fn rows(&self, app: &Faraday) -> Vec<crate::flow::Row> {
        let Some(s) = app.tools.seed_check.state() else {
            return Vec::new();
        };
        let no_pass = !s.typed && s.facts.is_some();
        (0..step::COUNT as u8)
            .filter(|&k| !(no_pass && k == step::PASSPHRASE))
            .map(|k| {
                let value = super::screen::summary(s, k);
                crate::flow::Row {
                    label: faraday_text::Text::of(super::screen::TITLES[usize::from(k)]),
                    value: (!value.is_empty()).then_some(value),
                    mono: k == step::ACCOUNT || k == step::RESULT,
                    opens: k,
                    current: s.open == Some(k),
                    mark: None,
                }
            })
            .collect()
    }

    fn hide_shown(&self, app: &mut crate::Faraday) {
        app.typing.pass_eye.close();
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }

    /// Taking a card again drops every card after it
    /// (`reset_from`): nothing later stands.
    fn still_fits(&self, _: &Faraday, _changed: u8, _later: u8) -> bool {
        false
    }
}

/// Practice's public passphrase in both fields, the chip's press.
pub(crate) fn fill_demo_pass(app: &mut Faraday) {
    if let Some(s) = st(app) {
        s.pass.fill(crate::demo::PASSPHRASE);
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
    drop_waiting(app);
    reset_from(app, step::PASSPHRASE);
    if let Some(s) = st(app) {
        s.words = SecretText::of(words);
        s.typed = true;
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
