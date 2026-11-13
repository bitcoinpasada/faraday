//! A scanned Krux encrypted code, opened (`docs/KEF.md` §6, §7, §8, §15):
//! the read card's header and the sheet that asks the passphrase, shows
//! the bar while the key is worked out on the worker, and takes what
//! comes out down §6's road. Nothing runs before **Open** is pressed.
//!
//! The header (version, id, count, the time the count takes here) is
//! worked out once, at the read; drawing reads it. A short timed run at
//! the first read gives the time: its rate arrives when the run ends, and
//! the card and a sheet still asking gain the time then.

use crate::drawing::View;
use crate::secret_text::SecretText;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Faraday, Sheet};
use faraday_draw::tokens::{BODY_S, GAP as T_GAP, GAP_SMALL, LABEL_L};
use faraday_draw::widgets::Icon;
use faraday_keys::kef::Envelope;
use faraday_shell_api::Key as KeyIn;
use faraday_text::Text;
use zeroize::Zeroizing;

/// The most Krux itself writes (`docs/KEF.md` §10): above it the card
/// asks once more.
pub(crate) const COUNT_ASKED: u32 = 550_000;

/// How deep codes inside codes are offered (`docs/KEF.md` §6).
const DEPTH_MAX: u8 = 8;

/// The most of an id the card and the sheet show, in characters: an id
/// may run to 252 bytes, and the field stays in view.
const ID_SHOWN: usize = 24;

use crate::worker::kef::TIMED_ROUNDS;

/// A code's header on the read card.
pub(crate) struct Header {
    /// The code, which the passphrase opens.
    pub(crate) envelope: Envelope,
    /// The count is above what Krux writes.
    pub(crate) high: bool,
    /// The second press of **Open** is the one that goes on.
    pub(crate) sure: bool,
}

/// What the sheet is doing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// The passphrase is asked.
    Ask,
    /// The key is worked out on the worker.
    Working,
    /// The contents parse as another encrypted code: offered as one, with
    /// a way to read them as what else they may be.
    Offer,
    /// The contents are text that names nothing.
    Note,
    /// The contents are bytes that are not text.
    Bytes,
}

/// What the sheet's presses do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KefAct {
    /// Typing goes to the passphrase field.
    Field,
    /// Open with what is typed.
    Open,
    /// Switch between typed text and hex bytes.
    Hex,
    /// The eye on the contents.
    Show,
    /// The contents offered as a code are something else.
    NotEncrypted,
    /// The note is carried to the next seed's passphrase.
    UseAsPassphrase,
    /// A note of 16 or 32 bytes is read as a seed's entropy.
    ReadAsSeed,
}

/// The sheet's state, wiped when it closes.
pub(crate) struct KefRead {
    envelope: Envelope,
    /// The header's facts as one line, worked out when the sheet opens.
    facts: Text,
    depth: u8,
    stage: Stage,
    hex: bool,
    pass: SecretText,
    focus: bool,
    error: Option<Text>,
    /// The passphrases still to try if the one running fails.
    forms: Vec<Zeroizing<Vec<u8>>>,
    /// The contents, as shown (a note's text or the bytes in hex).
    shown: Option<Text>,
    eye: bool,
    retried: bool,
    /// The contents offered as a code, kept until one of the two ways.
    offered: Option<Zeroizing<Vec<u8>>>,
    /// A count above what Krux writes was seen and **Open** pressed once.
    sure: bool,
}

impl KefRead {
    /// Whether it holds a secret: a passphrase typed or still to try, the
    /// contents shown or offered.
    pub(crate) fn holds_secret(&self) -> bool {
        !self.pass.is_empty()
            || !self.forms.is_empty()
            || self.shown.is_some()
            || self.offered.is_some()
    }

    pub(crate) fn new(envelope: Envelope, depth: u8, rate: Option<u64>) -> Self {
        Self {
            facts: joined(&facts(&envelope, rate)),
            envelope,
            depth,
            stage: Stage::Ask,
            hex: false,
            pass: SecretText::new(),
            focus: true,
            error: None,
            forms: Vec::new(),
            shown: None,
            eye: false,
            retried: false,
            offered: None,
            sure: false,
        }
    }

    /// The text typing goes to.
    pub(crate) fn field(&self) -> Option<&SecretText> {
        (self.stage == Stage::Ask && self.focus).then_some(&self.pass)
    }
}

/// The facts as one line, with a dot between.
fn joined(facts: &[Text]) -> Text {
    let mut out = Text::new();
    for (i, f) in facts.iter().enumerate() {
        if i > 0 {
            out.push_str(" · ");
        }
        out.push_str(f);
    }
    out
}

/// The id as the card shows it: text, or `0x` hex (`docs/KEF.md` §8).
fn id_shown(id: &[u8]) -> Text {
    let whole = match faraday_codec::kef::id_text(id) {
        Some(t) => Text::of(t),
        None => Text::fill("0x{}", &[&*faraday_crypto::hex::encode_string(id)]),
    };
    if whole.chars().count() <= ID_SHOWN {
        return whole;
    }
    let mut cut: Text = whole.chars().take(ID_SHOWN).collect();
    cut.push('…');
    cut
}

/// Whole bytes from hex text, spaces left out.
fn unhex(text: &str) -> Option<Zeroizing<Vec<u8>>> {
    let digits: Zeroizing<Vec<u8>> = Zeroizing::new(
        text.bytes()
            .filter(|b| !b.is_ascii_whitespace())
            .collect::<Vec<u8>>(),
    );
    if digits.is_empty() || !digits.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Zeroizing::new(Vec::with_capacity(digits.len() / 2));
    for pair in digits.chunks(2) {
        let s = std::str::from_utf8(pair).ok()?;
        out.push(u8::from_str_radix(s, 16).ok()?);
    }
    Some(out)
}

/// The time the count takes here, from the rate in milliseconds per timed
/// run: "under a second", "12 s", "3 min".
fn time_text(count: u32, rate_ms: u64) -> Text {
    let ms = u64::from(count) * rate_ms / u64::from(TIMED_ROUNDS);
    if ms < 1000 {
        Text::of(EN.kef_under_a_second)
    } else if ms < 120_000 {
        Text::fill(EN.kef_seconds, &[&Text::num(ms / 1000)])
    } else {
        Text::fill(EN.kef_minutes, &[&Text::num(ms / 60_000)])
    }
}

/// The facts a code's header gives: version, id, count and time.
fn facts(envelope: &Envelope, rate: Option<u64>) -> Vec<Text> {
    let spec = envelope.spec();
    let count = envelope.count;
    let count_line = match rate {
        Some(r) => Text::fill(
            EN.kef_count_fact,
            &[&Text::num(u64::from(count)), &time_text(count, r)],
        ),
        None => Text::fill(EN.kef_count_only, &[&Text::num(u64::from(count))]),
    };
    let id = Text::fill(EN.kef_id_fact, &[&id_shown(&envelope.id)]);
    // The line about a high count takes the version's name's room, so the
    // card holds three lines on the smallest screen.
    if count > COUNT_ASKED {
        return vec![
            Text::fill(
                EN.kef_version_fact,
                &[&Text::num(u64::from(spec.version)), &id],
            ),
            count_line,
            Text::of(EN.kef_count_high),
        ];
    }
    vec![
        Text::fill(
            EN.kef_version_fact,
            &[&Text::num(u64::from(spec.version)), spec.name],
        ),
        id,
        count_line,
    ]
}

/// The read card's header and facts for the code `code` is, if it is
/// one (`docs/KEF.md` §8): worked out once, at the read. The first one in
/// a session sends the short timed run to the worker; the card shows the
/// count alone until its answer comes.
pub(crate) fn kef_header(app: &mut Faraday, code: &[u8]) -> Option<(Header, Vec<Text>)> {
    let found = faraday_codec::kef::recognize(code)?;
    let envelope = found.envelope;
    if app.seed_read.rate().is_none() && !app.seed_read.timing() {
        let out = app.kef_time_start();
        app.seed_read.set_timing(out);
    }
    let facts = facts(&envelope, app.seed_read.rate());
    let high = envelope.count > COUNT_ASKED;
    Some((
        Header {
            envelope,
            high,
            sure: false,
        },
        facts,
    ))
}

/// The timed run's answer: its rate is known, and the card's count line
/// and the sheet's facts, while it asks or offers, gain the time. Returns
/// whether anything changed.
pub(crate) fn kef_timed(app: &mut Faraday, rate_ms: Option<u64>) -> bool {
    let Some(rate) = app.seed_read.finish_timing(rate_ms) else {
        return false;
    };
    let mut changed = false;
    if let Some(k) = app.seed_read.kef.as_mut()
        && matches!(k.stage, Stage::Ask | Stage::Offer)
    {
        k.facts = joined(&facts(&k.envelope, Some(rate)));
        changed = true;
    }
    changed |= app
        .camera
        .remake_card_facts(|h| facts(&h.envelope, Some(rate)));
    changed
}

/// The card's **Open**: the camera goes and the sheet asks the
/// passphrase. `asked` is whether the question above 550,000 rounds was
/// answered already; if not, the sheet's first **Open** is that answer.
pub(crate) fn kef_open_sheet(app: &mut Faraday, envelope: Envelope, depth: u8, asked: bool) {
    let rate = app.seed_read.rate();
    let mut k = KefRead::new(envelope, depth, rate);
    k.sure = asked;
    app.seed_read.set_kef(Some(k));
    app.sheet = Some(Sheet::KefRead);
}

/// The sheet's presses.
pub(crate) fn kef_act(app: &mut Faraday, act: KefAct) {
    let Some(k) = app.seed_read.kef.as_mut() else {
        return;
    };
    match act {
        KefAct::Field => k.focus = true,
        KefAct::Hex => {
            k.hex = !k.hex;
            k.pass.clear();
            k.error = None;
            k.focus = true;
        }
        KefAct::Show => k.eye = !k.eye,
        KefAct::Open if k.stage == Stage::Offer => {
            if k.envelope.count > COUNT_ASKED && !k.sure {
                k.sure = true;
            } else {
                k.stage = Stage::Ask;
                k.offered = None;
            }
        }
        KefAct::NotEncrypted => {
            if let Some(plain) = k.offered.take() {
                k.stage = Stage::Ask;
                kef_road_on(app, plain);
            }
        }
        KefAct::Open => kef_go(app),
        KefAct::UseAsPassphrase => kef_use_as_passphrase(app),
        KefAct::ReadAsSeed => kef_read_as_seed(app),
    }
}

/// Starts the job with what is typed. Nothing runs before this press.
fn kef_go(app: &mut Faraday) {
    let Some(k) = app.seed_read.kef.as_mut() else {
        return;
    };
    if k.stage != Stage::Ask {
        return;
    }
    if k.envelope.count > COUNT_ASKED && !k.sure {
        k.sure = true;
        return;
    }
    let mut forms = if k.hex {
        match unhex(k.pass.as_str()) {
            Some(bytes) => vec![bytes],
            None => {
                k.error = Some(Text::of(EN.kef_hex_bad));
                return;
            }
        }
    } else if k.pass.is_empty() {
        vec![Zeroizing::new(Vec::new())]
    } else {
        faraday_keys::kef::reading_passphrases(k.pass.as_str())
    };
    // Popped from the back, so the typed form goes first.
    forms.reverse();
    k.forms = forms;
    k.error = None;
    kef_try_next(app);
}

/// The next passphrase form goes to the worker; none left is a failure.
fn kef_try_next(app: &mut Faraday) {
    let Some(k) = app.seed_read.kef.as_mut() else {
        return;
    };
    let Some(pass) = k.forms.pop() else {
        k.stage = Stage::Ask;
        k.error = Some(Text::of(EN.kef_failed));
        return;
    };
    k.stage = Stage::Working;
    let envelope = k.envelope.clone();
    app.kef_open(pass, envelope, EN.kef_working);
}

/// The worker's answer for a code opened here, taken when it comes
/// (`Faraday::apply_done`). Returns whether the sheet used it.
pub(crate) fn kef_opened_here(app: &mut Faraday) {
    if app.seed_read.kef().is_none() {
        return;
    }
    // Another sheet over this one (a Lock sheet, say): the answer has
    // nowhere to go and is dropped with the sheet.
    if app.sheet != Some(Sheet::KefRead) {
        drop(app.kef_take_opened());
        app.seed_read.set_kef(None);
        return;
    }
    let Some(answer) = app.kef_take_opened() else {
        return;
    };
    match answer {
        Ok(opened) => {
            let retried = opened.retried;
            if let Some(k) = app.seed_read.kef.as_mut() {
                k.retried = retried;
                k.forms.clear();
            }
            kef_road(app, opened.plaintext);
        }
        Err(faraday_keys::kef::OpenError::Cancelled) => {
            if let Some(k) = app.seed_read.kef.as_mut() {
                k.stage = Stage::Ask;
                k.forms.clear();
            }
        }
        Err(_) => kef_try_next(app),
    }
}

/// Costs a mnemonic's checksum and what the intake costs. §6's road for
/// what came out: 16 or 32 bytes that are not text to the seed sheet, a code inside a
/// code offered again, text through the intake, a note, or hex.
fn kef_road(app: &mut Faraday, plain: Zeroizing<Vec<u8>>) {
    kef_road_from(app, plain, true);
}

/// Costs what [`Faraday::kef_road`] costs. The road past the offer of
/// a code inside a code: the contents are not one.
fn kef_road_on(app: &mut Faraday, plain: Zeroizing<Vec<u8>>) {
    kef_road_from(app, plain, false);
}

fn kef_road_from(app: &mut Faraday, plain: Zeroizing<Vec<u8>>, offer: bool) {
    if app.seed_read.kef().is_none() {
        return;
    }
    if matches!(plain.len(), 16 | 32) && !is_note_text(&plain) {
        match kef_as_seed(app, plain) {
            Ok(()) => return,
            Err(back) => return kef_road_after_seed(app, back, offer),
        }
    }
    kef_road_after_seed(app, plain, offer)
}

/// Whether bytes are text a person wrote: valid UTF-8 holding no control
/// character other than tab, newline and carriage return (`docs/KEF.md` §6).
fn is_note_text(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok_and(|t| {
        !t.chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\t' | '\n' | '\r'))
    })
}

/// Costs what the seed sheet's intake costs. The road's seed branch: 16 or
/// 32 bytes as entropy to the seed sheet with the id line. Gives the bytes
/// back when they did not go.
fn kef_as_seed(app: &mut Faraday, plain: Zeroizing<Vec<u8>>) -> Result<(), Zeroizing<Vec<u8>>> {
    let Some(k) = app.seed_read.kef() else {
        return Err(plain);
    };
    let (depth, id) = (k.depth, k.envelope.id.clone());
    let lang = faraday_bip::bip39::Language::English;
    let Ok(m) = faraday_keys::words::from_compact_seedqr(&plain, lang) else {
        return Err(plain);
    };
    // Raw bytes that are also a code: the seed sheet offers to open
    // them as one (`docs/KEF.md` §8), within the depth.
    let also = (depth + 1 < DEPTH_MAX)
        .then(|| faraday_codec::kef::recognize(&plain))
        .flatten();
    let words = lang.words();
    let mut typed = crate::secret_text::room();
    for (n, &i) in m.indices().iter().enumerate() {
        if n > 0 {
            typed.push(' ');
        }
        typed.push_str(words[usize::from(i)]);
    }
    drop(plain);
    app.seed_read.set_kef(None);
    app.sheet = None;
    app.seed_scanned(&typed, scan_kind());
    app.seed_read_kef_id(&id);
    if let Some(found) = also {
        app.seed_read_offer_kef(found.envelope);
    }
    Ok(())
}

/// Costs what the intake costs. The rest of §6's road, past the seed branch.
fn kef_road_after_seed(app: &mut Faraday, plain: Zeroizing<Vec<u8>>, offer: bool) {
    let Some(depth) = app.seed_read.kef().map(|k| k.depth) else {
        return;
    };
    let c = app.compute();
    if offer
        && depth + 1 < DEPTH_MAX
        && let Some(found) = faraday_codec::kef::recognize(&plain)
    {
        let mut next = KefRead::new(found.envelope, depth + 1, app.seed_read.rate());
        next.stage = Stage::Offer;
        next.offered = Some(plain);
        app.seed_read.set_kef(Some(next));
        return;
    }
    let Ok(text) = std::str::from_utf8(&plain) else {
        return kef_show(
            app,
            Stage::Bytes,
            faraday_crypto::hex::encode_secret(&plain),
        );
    };
    // Past the depth, or said not to be encrypted: a code that parses as
    // one goes on as an unknown code, never as a new card at depth 0.
    let is_code = faraday_codec::kef::recognize(&plain).is_some();
    // 16 or 32 bytes of note text stay a note here, with the card's own
    // **Read as a seed** (`docs/KEF.md` §6), though a scanned code of
    // those bytes is a CompactSeedQR.
    let secret = match crate::scan_read::secret_kind(&c, &plain) {
        Some(crate::scan_read::Kind::CompactSeedQr) => !is_note_text(&plain),
        other => other.is_some(),
    };
    let names_something = secret
        || !is_code
            && !matches!(
                crate::scan_read::public_kind(
                    crate::wallet::classify(&c, "", &plain),
                    false,
                    &plain
                ),
                crate::scan_read::Kind::Text | crate::scan_read::Kind::Unreadable
            );
    if is_code && !secret {
        app.seed_read.set_kef(None);
        app.sheet = None;
        let mut scan = crate::ScanState::default();
        scan.ask = crate::Ask::Any;
        app.camera.open(scan);
        app.sheet = Some(Sheet::Scan);
        let kind = crate::scan_read::unknown_kind(&plain);
        return app.read_card_show(&c, kind, (None, "txt", plain));
    }
    if names_something {
        app.seed_read.set_kef(None);
        app.sheet = None;
        let mut scan = crate::ScanState::default();
        scan.ask = crate::Ask::Any;
        app.camera.open(scan);
        app.sheet = Some(Sheet::Scan);
        return app.scanned(plain.to_vec());
    }
    let shown = Text::of(text);
    drop(plain);
    kef_show(app, Stage::Note, shown);
}

impl KefRead {
    /// The contents hidden: the eye is closed.
    pub(super) fn hide(&mut self) {
        self.eye = false;
    }
}

fn kef_show(app: &mut Faraday, stage: Stage, shown: Text) {
    if let Some(k) = app.seed_read.kef.as_mut() {
        k.stage = stage;
        k.shown = Some(shown);
        k.eye = false;
    }
}

/// The note's own bytes go down the road's seed branch, as raw bytes of
/// that length do (`docs/KEF.md` §6).
fn kef_read_as_seed(app: &mut Faraday) {
    let Some(k) = app.seed_read.kef() else {
        return;
    };
    if k.stage != Stage::Note {
        return;
    }
    let Some(shown) = k.shown.as_ref().filter(|t| matches!(t.len(), 16 | 32)) else {
        return;
    };
    let bytes = Zeroizing::new(shown.as_bytes().to_vec());
    let _ = kef_as_seed(app, bytes);
}

/// The note is carried to the next seed read as its passphrase: the
/// scanner for a seed opens (`docs/KEF.md` §6).
fn kef_use_as_passphrase(app: &mut Faraday) {
    let Some(k) = app.seed_read.kef.as_mut() else {
        return;
    };
    let Some(text) = k.shown.take() else {
        return;
    };
    app.seed_read.set_kef(None);
    app.sheet = None;
    app.seed_read
        .carry_passphrase(crate::secret_text::SecretText::of(&text));
    app.act(Action::ScanSeed);
}

/// A stick arrived: what was typed into a Krux code's sheet still counts,
/// and the sheet closes: its job stops and its contents are dropped, a
/// carried passphrase with them.
pub(crate) fn kef_stick_arrived(app: &mut Faraday) {
    app.note_secrets();
    drop(app.seed_read.take_carried_passphrase());
    if app.seed_read.kef().is_some() {
        app.kef_job_drop();
        app.seed_read.set_kef(None);
        if app.sheet == Some(Sheet::KefRead) {
            app.sheet = None;
        }
    }
}

/// Cancel on the sheet: while the key is worked out it stops the work
/// and asks the passphrase again; anywhere else it closes the sheet.
pub(crate) fn kef_cancel(app: &mut Faraday) {
    let Some(k) = app.seed_read.kef.as_mut() else {
        app.sheet = None;
        return;
    };
    if k.stage == Stage::Working {
        k.stage = Stage::Ask;
        k.forms.clear();
        return;
    }
    app.seed_read.set_kef(None);
    app.sheet = None;
}

/// Keys typed into the sheet's field. Returns whether it took the key.
pub(crate) fn kef_read_key(app: &mut Faraday, key: KeyIn) -> bool {
    if app.sheet != Some(Sheet::KefRead) {
        return false;
    }
    let Some(k) = app.seed_read.kef.as_mut() else {
        return false;
    };
    if k.stage != Stage::Ask {
        return false;
    }
    match key {
        KeyIn::Char(ch) if !ch.is_control() => k.pass.push(ch),
        KeyIn::Backspace => {
            k.pass.pop();
        }
        KeyIn::Enter => {
            kef_go(app);
            return true;
        }
        _ => return false,
    }
    k.error = None;
    true
}

/// Whether typing goes to the sheet's field now.
pub(crate) fn kef_typing(app: &Faraday) -> bool {
    app.sheet == Some(Sheet::KefRead) && app.seed_read.kef().is_some_and(|k| k.field().is_some())
}

/// The sheet's field text, for the caret.
pub(crate) fn kef_field(app: &Faraday) -> Option<&SecretText> {
    app.seed_read.kef()?.field()
}

/// The kind a contents of 16 or 32 bytes are read as on the seed sheet.
fn scan_kind() -> crate::scan_read::Kind {
    crate::scan_read::Kind::CompactSeedQr
}

/// The sheet's buttons for what it is doing.
fn sheet_items(k: &KefRead) -> Vec<(&'static str, Style, Action)> {
    let mut items: Vec<(&str, Style, Action)> = Vec::new();
    let open = if k.envelope.count > COUNT_ASKED && k.sure {
        EN.kef_open_anyway
    } else {
        EN.kef_open
    };
    match k.stage {
        Stage::Ask => {
            items.push((EN.kef_cancel, Style::Ghost, Action::Cancel));
            items.push((
                if k.hex {
                    EN.kef_enter_text
                } else {
                    EN.kef_enter_hex
                },
                Style::Secondary,
                Action::KefRead(KefAct::Hex),
            ));
            items.push((open, Style::Primary, Action::KefRead(KefAct::Open)));
        }
        Stage::Offer => {
            items.push((EN.kef_cancel, Style::Ghost, Action::Cancel));
            items.push((
                EN.kef_not_encrypted,
                Style::Secondary,
                Action::KefRead(KefAct::NotEncrypted),
            ));
            items.push((open, Style::Primary, Action::KefRead(KefAct::Open)));
        }
        Stage::Working => items.push((EN.kef_cancel, Style::Primary, Action::Cancel)),
        Stage::Note => {
            items.push((EN.kef_done, Style::Ghost, Action::Cancel));
            if k.shown.as_ref().is_some_and(|t| matches!(t.len(), 16 | 32)) {
                items.push((
                    EN.kef_read_as_seed,
                    Style::Secondary,
                    Action::KefRead(KefAct::ReadAsSeed),
                ));
            }
            items.push((
                EN.kef_use_passphrase,
                Style::Primary,
                Action::KefRead(KefAct::UseAsPassphrase),
            ));
        }
        Stage::Bytes => items.push((EN.kef_done, Style::Primary, Action::Cancel)),
    }
    items
}

/// The sheet.
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(k) = app.seed_read.kef() else {
        return;
    };
    let items = sheet_items(k);
    let title = match k.stage {
        Stage::Note => EN.kef_note_title,
        Stage::Bytes => EN.kef_bytes_title,
        _ => EN.kef_sheet_title,
    };
    let body = |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 {
        let mut cy = y;
        if matches!(k.stage, Stage::Ask | Stage::Working | Stage::Offer) {
            // One paragraph, so the field stays in view on a small screen.
            cy += ui.wrap_as((x, cy, iw), BODY_S, Role::OnSurface, &k.facts) + T_GAP;
        }
        match k.stage {
            Stage::Ask => {
                cy += crate::screens::pass_field(
                    ui,
                    x,
                    cy,
                    iw,
                    if k.hex { EN.kef_hex } else { EN.kef_passphrase },
                    &k.pass,
                    (k.focus, None),
                    Action::KefRead(KefAct::Field),
                );
                if let Some(e) = &k.error {
                    cy += ui.wrap_as((x, cy + GAP_SMALL, iw), LABEL_L, Role::Error, e) + T_GAP;
                }
            }
            Stage::Offer => {}
            Stage::Working => {
                if let Some(b) = app.busy() {
                    crate::screens::busy_bar(app, ui, (x, cy + T_GAP, iw), b);
                    cy += faraday_draw::tokens::PROGRESS_H + T_GAP + T_GAP;
                }
            }
            Stage::Note | Stage::Bytes => {
                if let Some(text) = &k.shown {
                    let eye = (Action::KefRead(KefAct::Show), k.eye);
                    cy += crate::vault::screen::secret::secret(
                        ui,
                        (x, cy, iw),
                        text,
                        k.stage == Stage::Bytes,
                        eye,
                    ) + T_GAP;
                }
                if k.retried {
                    cy += ui.wrap_as((x, cy, iw), BODY_S, Role::OnSurfaceVariant, EN.kef_retried)
                        + T_GAP;
                }
            }
        }
        cy - y
    };
    let head = (Icon::Keys.into(), Role::Primary.paint(), title);
    crate::compact::dialog(
        ui,
        (w, h, faraday_draw::tokens::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| body(ui, x, y, iw),
        &items,
    );
}

/// A note shown on the sheet, for the budget tests.
#[cfg(test)]
pub(crate) fn test_note(app: &mut Faraday, text: &str) {
    let code = faraday_codec::kef::wrap(b"id", 20, 10_000, &[7u8; 20]).unwrap();
    let env = faraday_codec::kef::recognize(&code).unwrap().envelope;
    kef_open_sheet(app, env, 0, true);
    kef_show(app, Stage::Note, Text::of(text));
}
