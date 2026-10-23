//! Use a sample (`docs/MODES.md` §6, §10 M7): in Demo, with samples set,
//! each step that takes a key draws a chip inside its empty field, or on
//! the scanner a tile, and a press puts a sample into the step through
//! the function the step's own input already calls, as if typed or
//! scanned. Which sample: the next one not loaded, by a cursor on the
//! step's state; a part of a set fills the next part of the set begun.
//! Nowhere outside Demo, and nowhere on a shell that sets no samples.
//!
//! Whether a chip shows, and which sample is next, are lookups against
//! the keys loaded and what the state keeps; a press that derives
//! (a cosigner's account key) does it with the handler's `Compute`.

use super::{SampleForm, SampleKey, Samples};
use crate::forms::{self, Form};
use crate::strings::EN;
use crate::ui::Style;
use crate::ui::{Ui, W};
use crate::wallet::{read_wallet, same_wallet};
use crate::{Action, Faraday, ScanPurpose, Screen, Sheet};
use faraday_bip::bip39::Language;
use faraday_crypto::work::Compute;
use faraday_draw::tokens::{self, FIELD_ACTION_INSET, FIELD_ACTION_TEXT};

/// What a step keeps of Use a sample: whether its chip is gone for this
/// visit, and which sample comes next. Kept beside the field it is for
/// and dropped with the step's state, so a step left and opened again
/// starts with the chip.
#[derive(Debug, Default, Clone, Copy)]
pub struct Use {
    /// The person pressed in the field or typed in it this visit.
    dismissed: bool,
    /// A press of the chip filled the field and the person has not yet
    /// used what it holds.
    filled: bool,
    /// Where the next press looks first among the samples it can give: for
    /// the words, one place for each word count; for a part, the first.
    next: [u8; 5],
    /// The sample key whose set the parts collected belong to.
    begun: Option<u8>,
}

impl Use {
    /// The field is pressed in or typed in: the chip is gone for the
    /// visit, whatever the field holds after.
    pub(crate) fn dismiss(&mut self) {
        self.dismissed = true;
    }

    /// Whether the chip may show, the field being empty.
    pub(crate) fn shown(&self) -> bool {
        !self.dismissed && !self.filled
    }

    /// A press of the chip filled the field.
    pub(crate) fn fill(&mut self) {
        self.filled = true;
    }

    /// What the chip filled is used or cleared: the field takes the chip
    /// again.
    pub(crate) fn emptied(&mut self) {
        self.filled = false;
    }
}

/// The chip inside the empty field at (`x`, `y`, `w`, `h`), its label the
/// shorter one where the field is narrower than the long one.
pub(crate) fn chip(ui: &mut Ui, rect: (f32, f32, f32, f32), action: Action) {
    chip_labelled(ui, rect, action, (EN.use_a_sample, EN.scan_sample));
}

/// [`chip`] with its own `(long, short)` labels.
pub(crate) fn chip_labelled(
    ui: &mut Ui,
    (x, y, w, h): (f32, f32, f32, f32),
    action: Action,
    (long, short): (&'static str, &'static str),
) {
    let need = ui.measure(FIELD_ACTION_TEXT, W::S, long) + 2.0 * FIELD_ACTION_INSET;
    let label = if need + 2.0 * FIELD_ACTION_INSET <= w {
        long
    } else {
        short
    };
    ui.field_action(x, y, w, h, label, action);
}

/// The `faraday` chip inside the passphrase field at (`x`, `y`, `w`, `h`),
/// at its left, or at its right end where the field holds the **No
/// passphrase** pill at the left (`docs/MODES.md` §6.2).
pub(crate) fn pass_chip(
    ui: &mut Ui,
    (x, y, w, h): (f32, f32, f32, f32),
    action: Action,
    right: bool,
) {
    let label = EN.passphrase_chip;
    if right {
        let width = ui.measure(FIELD_ACTION_TEXT, W::S, label) + 2.0 * FIELD_ACTION_INSET;
        ui.field_action(
            x + w - width - 2.0 * FIELD_ACTION_INSET,
            y,
            w,
            h,
            label,
            action,
        );
    } else {
        ui.field_action(x, y, w, h, label, action);
    }
}

/// Add a key's word grid on a wide display, empty: the chip inside cell 1
/// only, after its number, where the word would be; a press anywhere in
/// the grid focuses it. `boxes` cells are drawn by the grid itself.
pub(crate) fn grid_chip(ui: &mut Ui, pos: (f32, f32, f32), boxes: usize) {
    grid_chip_labelled(ui, pos, boxes, (EN.use_a_sample, EN.scan_sample));
}

/// [`grid_chip`] with its own labels: Check my copy's **Fill words**.
pub(crate) fn grid_chip_labelled(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    boxes: usize,
    labels: (&'static str, &'static str),
) {
    let rows = boxes.div_ceil(4) as f32 * tokens::ENTRY_ROW - tokens::FIELD_RADIUS;
    ui.hit(x, y, w, rows, Action::EntryField);
    let cell = (w - tokens::ENTRY_CELL_GAP * 3_f32) / 4_f32;
    let at = tokens::ENTRY_CELL_NUMBER;
    let h = tokens::ENTRY_CELL_HEIGHT;
    chip_labelled(ui, (x + at, y, cell - at, h), Action::UseSample, labels);
}

/// The chip in the word being typed on a small panel, in the place of its
/// keys: a press in it is not a press in the field, which has none.
pub(crate) fn head_chip(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    labels: (&'static str, &'static str),
) {
    chip_labelled(
        ui,
        (x, y, w, tokens::FIELD_ACTION_HEIGHT),
        Action::UseSample,
        labels,
    );
}

/// The chip in a field of one share or part, of `h` high; a press in the
/// field focuses it.
pub(crate) fn field_chip(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.hit(x, y, w, h, Action::EntryField);
    chip(ui, (x, y, w, h), Action::UseSample);
}

/// The chip in a cosigner's key box of the row at `y`, `x` the row's left
/// and `w` its width.
pub(crate) fn box_chip(ui: &mut Ui, (x, y, w): (f32, f32, f32), action: Action) {
    let inset = tokens::SEED_FIELD_INSET;
    chip(
        ui,
        (x + inset, y, w - inset, tokens::SEED_FIELD_HEIGHT),
        action,
    );
}

/// The room the scanner's Sample tile takes in the foot on a wide
/// display, with the gap after it.
pub(crate) fn scan_tile_span(ui: &Ui, long: &'static str) -> f32 {
    let label = ui.measure(tokens::SCAN_TILE_TEXT, W::S, scan_tile_label(ui, long));
    label + tokens::SCAN_TILE_PAD + tokens::SCAN_TILE_GAP
}

/// What the tile says: `long`, what it hands over, where it fits; else
/// the short word.
fn scan_tile_label(ui: &Ui, long: &'static str) -> &'static str {
    let span = ui.measure(tokens::SCAN_TILE_TEXT, W::S, long) + tokens::SCAN_TILE_PAD;
    if ui.compact || span <= tokens::SCAN_TILE_MAX_SPAN {
        long
    } else {
        EN.scan_sample
    }
}

/// The scanner's Sample tile: on a wide display in the foot, ending at
/// `left` (returned moved past it); on a small panel full width above
/// the foot and `row` of buttons above it. `x` and `w` are the sheet's.
pub(crate) fn scan_tile(
    ui: &mut Ui,
    (x, w, foot): (f32, f32, f32),
    (row, left): (f32, f32),
    long: &'static str,
) -> f32 {
    let (h, side) = (tokens::SCAN_TILE_HEIGHT, tokens::SCAN_SIDE);
    let label = scan_tile_label(ui, long);
    if ui.compact {
        let above = foot - tokens::SCAN_TILE_ROW - row;
        let wide = Some(w - 2.0 * side);
        ui.button(
            x + side,
            above,
            wide,
            h,
            label,
            Style::Secondary,
            Action::ScanSample,
        );
        return left;
    }
    let span = scan_tile_span(ui, long);
    let bw = span - tokens::SCAN_TILE_GAP;
    let at = left - bw;
    ui.button(
        at,
        foot,
        Some(bw),
        h,
        label,
        Style::Secondary,
        Action::ScanSample,
    );
    at - tokens::SCAN_TILE_GAP
}

/// The samples that are keys of BIP-39 words in the English list, with
/// their places in [`Samples::keys`].
fn english_keys(set: &Samples) -> impl Iterator<Item = (usize, &SampleKey)> {
    set.keys.iter().enumerate().filter(|(_, k)| {
        k.form == SampleForm::Words
            && k.plain.is_some()
            && k.parts
                .first()
                .and_then(|p| p.split_whitespace().next())
                .is_some_and(|w| forms::typed_index(Language::English, w).is_some())
    })
}

/// How many words a words sample has.
fn word_count(k: &SampleKey) -> usize {
    k.parts.first().map_or(0, |p| p.split_whitespace().count())
}

/// A words sample's Standard SeedQR: four digits for each word's place in
/// the English list. `None` for a word that is not on it.
fn seedqr_digits(words: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for w in words.split_whitespace() {
        let i = forms::typed_index(Language::English, w)?;
        out.extend(format!("{i:04}").bytes());
    }
    Some(out)
}

/// The fingerprint a cosigner's key text names, as typed: the origin's
/// first part.
fn origin_of(text: &str) -> Option<[u8; 4]> {
    let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let fp = t.strip_prefix('[')?.split(['/', ']']).next()?.to_string();
    let b = (0..4)
        .map(|i| u8::from_str_radix(fp.get(2 * i..2 * i + 2)?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    b.try_into().ok()
}

impl Faraday {
    /// The samples, where Use a sample is on at all: Demo with samples
    /// set.
    pub(crate) fn put_on(&self) -> Option<&'static Samples> {
        self.samples.filter(|_| self.samples_on())
    }

    /// The sample out of `cands` (places in [`Samples::keys`]) a press
    /// gives: the first not loaded from `cursor` on, wrapping; the one at
    /// `cursor` when all are. With the cursor to leave afterwards. A
    /// lookup against the keys loaded.
    fn sample_choose(&self, set: &Samples, cands: &[usize], cursor: u8) -> Option<(usize, u8)> {
        let n = cands.len();
        if n == 0 {
            return None;
        }
        let start = usize::from(cursor) % n;
        let at = (0..n)
            .map(|k| (start + k) % n)
            .find(|&i| !set.keys[cands[i]].held(&self.session.keys))
            .unwrap_or(start);
        Some((cands[at], ((at + 1) % n) as u8))
    }

    /// Whether Add a key's field is empty: nothing typed, and on a list
    /// typed on its own keyboard no word taken or begun.
    fn entry_empty(&self) -> bool {
        match self.entry.keys.as_deref() {
            Some(w) if self.entry.form == Form::Words => {
                w.committed() == 0 && w.prefix().is_empty()
            }
            _ => self.entry.typed.is_empty(),
        }
    }

    /// Whether Add a key's field, or New key's typed one, draws Use a
    /// sample now. A lookup.
    pub fn entry_sample_shown(&self) -> bool {
        self.put_on().is_some()
            && self.may_load_keys()
            && self.entry.sample.shown()
            && self.entry_empty()
    }

    /// A key typed on Add a key, or New key's typed step: the field is
    /// typed in, so the chip is gone for the visit. Returns whether the
    /// keys go there (`entry_typing_here`).
    pub(crate) fn entry_typing_key(&mut self) -> bool {
        let here = self.entry_typing_here();
        if here {
            self.entry.sample.dismiss();
        }
        here
    }

    /// The fingerprints of the keys loaded wallet `w` still needs here.
    fn wallet_missing(&self, w: usize) -> Vec<[u8; 4]> {
        let Some(wl) = self.session.wallets.get(w) else {
            return Vec::new();
        };
        self.session
            .slots(wl)
            .iter()
            .filter(|s| s.held_by.is_none())
            .filter_map(|s| s.fingerprint.map(|f| f.0))
            .collect()
    }

    /// The words sample Add a key is for, when it is for one key: the
    /// key `wanted`, else any key the wallet it is opened for is short of.
    fn entry_wanted_sample(&self, set: &Samples) -> Option<usize> {
        let missing = match (self.entry.wanted, self.entry.wallet) {
            (Some(fp), _) => vec![fp],
            (None, Some(w)) => self.wallet_missing(w),
            (None, None) => return None,
        };
        english_keys(set)
            .find(|(_, k)| k.plain.is_some_and(|p| missing.contains(&p)))
            .map(|(i, _)| i)
    }

    /// The words Add a key's chip gives: the key it is for, else the next
    /// words sample of the word count chosen that is not loaded.
    fn entry_sample_words(&mut self, set: &'static Samples) -> Option<&'static str> {
        let chosen = self.entry.words_chosen();
        let at = crate::EntryState::COUNTS
            .iter()
            .position(|n| usize::from(*n) == chosen)
            .unwrap_or(0);
        let cursor = self.entry.sample.next[at];
        let (i, next) = match self.entry_wanted_sample(set) {
            Some(i) => (i, cursor),
            None => {
                let cands: Vec<usize> = english_keys(set)
                    .filter(|(_, k)| word_count(k) == chosen)
                    .map(|(i, _)| i)
                    .collect();
                self.sample_choose(set, &cands, cursor)?
            }
        };
        let k = &set.keys[i];
        self.entry.sample.next[at] = next;
        self.entry.word_count = word_count(k) as u8;
        k.parts.first().copied()
    }

    /// The part Add a key's chip gives on a form of parts: the next part
    /// of the set begun, else part one of the first set not loaded.
    fn entry_sample_part(&mut self, set: &'static Samples) -> Option<&'static str> {
        let form = self.entry.form;
        let want = match form {
            Form::Slip39 => SampleForm::Slip39,
            Form::Codex32 => SampleForm::Codex32,
            Form::Xor => SampleForm::Xor,
            Form::Words => return None,
        };
        let have = self.entry.parts.lines(form).len();
        let begun = self
            .entry
            .sample
            .begun
            .filter(|_| have > 0)
            .and_then(|i| set.keys.get(usize::from(i)))
            .filter(|k| have < k.parts.len());
        if let Some(k) = begun {
            return Some(k.parts[have]);
        }
        let cands: Vec<usize> = set
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.form == want && k.plain.is_some() && !k.parts.is_empty())
            .map(|(i, _)| i)
            .collect();
        let (i, next) = self.sample_choose(set, &cands, self.entry.sample.next[0])?;
        self.entry.sample.next[0] = next;
        self.entry.sample.begun = u8::try_from(i).ok();
        set.keys[i].parts.first().copied()
    }

    /// Handles `Action::UseSample`: the chip inside Add a key's field, or
    /// New key's typed key. Puts the sample in as typed words or a typed
    /// part; the flow goes on from there.
    pub(crate) fn entry_use_sample(&mut self) {
        let Some(set) = self.put_on() else { return };
        if !self.entry_typing_here() || !self.may_load_keys() {
            return;
        }
        let words = self.entry.form == Form::Words;
        let Some(text) = (if words {
            self.entry_sample_words(set)
        } else {
            self.entry_sample_part(set)
        }) else {
            return;
        };
        self.entry.on_passphrase = false;
        if words {
            // The samples' words are English, whatever list was chosen.
            let english = forms::LANGUAGES
                .iter()
                .position(|l| *l == Language::English)
                .unwrap_or(0) as u8;
            if self.entry.language_at != english {
                self.act(Action::EntryLanguage(english));
            }
            if self.entry.keys.is_some() {
                let mut w = forms::word_typer(Language::English);
                forms::type_words(&mut w, Language::English, text);
                self.entry.keys = Some(w);
            } else {
                self.entry.typed.set(text);
                self.entry.typed.push(' ');
            }
        } else {
            self.entry.typed.set(text);
        }
        self.entry.error = None;
        self.entry.sample.filled = true;
        self.entry.forget_probe();
    }

    // The scanner.

    /// Whether the scanner on screen draws the Sample tile: a lookup.
    pub fn scan_sample_offered(&self) -> bool {
        let Some(set) = self.put_on() else {
            return false;
        };
        if self.sheet != Some(Sheet::Scan) {
            return false;
        }
        let Some(scan) = self.camera.scan() else {
            return false;
        };
        match scan.purpose {
            ScanPurpose::Seed => self.may_load_keys(),
            ScanPurpose::KeyPart => self.may_load_keys() && self.entry.form != Form::Words,
            ScanPurpose::WalletKey(w) => {
                let missing = self.wallet_missing(w);
                self.may_load_keys()
                    && english_keys(set).any(|(_, k)| k.plain.is_some_and(|p| missing.contains(&p)))
            }
            ScanPurpose::Intake | ScanPurpose::VaultEntry => true,
            ScanPurpose::Files => true,
            ScanPurpose::CheckCopy(fp) => english_keys(set)
                .any(|(_, k)| k.plain == Some(fp.0) || k.with_passphrase == Some(fp.0)),
            ScanPurpose::Transfer => false,
        }
    }

    /// What the Sample tile on the scanner on screen hands over, as the
    /// words on it: a lookup.
    pub fn scan_sample_label(&self) -> &'static str {
        let purpose = self.camera.scan().map(|s| s.purpose);
        match purpose {
            Some(ScanPurpose::Seed) => EN.scan_sample_seed,
            Some(ScanPurpose::KeyPart) => EN.scan_sample_share,
            Some(ScanPurpose::WalletKey(_)) => EN.scan_sample_key,
            Some(ScanPurpose::CheckCopy(_)) => EN.scan_sample_seedqr,
            Some(ScanPurpose::VaultEntry) => EN.scan_sample_code,
            Some(ScanPurpose::Files) if self.screen == Screen::Restore => EN.scan_sample_wallet,
            Some(ScanPurpose::Files) if self.screen == Screen::Catalog => EN.scan_sample_address,
            _ => EN.scan_sample_psbt,
        }
    }

    /// Whether a step that asks for the transaction draws Load sample
    /// PSBT: Demo, with samples set. A lookup.
    pub fn sample_psbt_offered(&self) -> bool {
        self.put_on().is_some()
    }

    /// Costs a wallet read for each sample wallet. Handles
    /// `Action::SamplePsbt`: the sample spend comes in as a scan of it
    /// would bring it, on this step: What do you have? takes it as a
    /// thing that came in, any other step as a file.
    pub(crate) fn sample_psbt_use(&mut self) {
        let Some(set) = self.put_on() else { return };
        let c = self.compute();
        let Some(bytes) = self.sample_spend(&c, set) else {
            return;
        };
        let had = self.camera.scan().is_some();
        if !had {
            let holding = self.screen == Screen::Family
                && self.family.open
                    == Some(crate::family::Open::Page(crate::family::page::HOLDING));
            let mut scan = crate::ScanState::default();
            scan.purpose = if holding {
                ScanPurpose::Intake
            } else {
                ScanPurpose::Files
            };
            self.camera.open(scan);
        }
        self.scanned(bytes.to_vec());
        if !had {
            self.camera.close();
        }
    }

    /// Costs a wallet read for each sample wallet, once a wallet is
    /// loaded. The sample wallets the session holds: the sample's place
    /// in [`Samples::wallets`] and the wallet's in the session.
    fn loaded_sample_wallets(&self, c: &Compute, set: &Samples) -> Vec<(usize, usize)> {
        let have: Vec<String> = self
            .session
            .wallets
            .iter()
            .map(|w| same_wallet(w.policy()))
            .collect();
        if have.is_empty() {
            return Vec::new();
        }
        set.wallets
            .iter()
            .enumerate()
            .filter_map(|(s, w)| {
                let d = same_wallet(&read_wallet(c, w.text).ok()?);
                have.iter().position(|h| *h == d).map(|i| (s, i))
            })
            .collect()
    }

    /// Costs a wallet read for each sample wallet. The unsigned spend of
    /// the sample wallet in hand, else of the first sample wallet loaded
    /// that has one, else of the first that has one.
    fn sample_spend(&self, c: &Compute, set: &'static Samples) -> Option<&'static [u8]> {
        let loaded = self.loaded_sample_wallets(c, set);
        let has = |s: usize| !set.wallets[s].spend.is_empty();
        let pick = loaded
            .iter()
            .find(|(s, w)| *w == self.wallet && has(*s))
            .or_else(|| loaded.iter().find(|(s, _)| has(*s)))
            .map(|(s, _)| *s)
            .or_else(|| (0..set.wallets.len()).find(|&s| has(s)))?;
        Some(set.wallets[pick].spend)
    }

    /// Costs a wallet read for each sample wallet. What the Sample tile
    /// scans for the scanner on screen, and moves the tile's cursor.
    fn scan_sample_bytes(&mut self, c: &Compute, set: &'static Samples) -> Option<Vec<u8>> {
        let scan = self.camera.scan()?;
        let (purpose, cursor) = (scan.purpose, scan.sample);
        match purpose {
            ScanPurpose::Seed => {
                let cands: Vec<usize> = english_keys(set).map(|(i, _)| i).collect();
                let (i, next) = self.sample_choose(set, &cands, cursor)?;
                if let Some(s) = self.camera.scan_mut() {
                    s.sample = next;
                }
                seedqr_digits(set.keys[i].parts.first()?)
            }
            ScanPurpose::KeyPart => self.entry_sample_part(set).map(|t| t.as_bytes().to_vec()),
            ScanPurpose::WalletKey(w) => {
                let missing = self.wallet_missing(w);
                let (_, k) = english_keys(set)
                    .find(|(_, k)| k.plain.is_some_and(|p| missing.contains(&p)))?;
                seedqr_digits(k.parts.first()?)
            }
            ScanPurpose::CheckCopy(fp) => {
                let (_, k) = english_keys(set)
                    .find(|(_, k)| k.plain == Some(fp.0) || k.with_passphrase == Some(fp.0))?;
                seedqr_digits(k.parts.first()?)
            }
            ScanPurpose::VaultEntry => Some(set.inputs.totp.as_bytes().to_vec()),
            ScanPurpose::Intake => self.sample_spend(c, set).map(<[u8]>::to_vec),
            ScanPurpose::Files if self.screen == Screen::Restore => {
                let loaded = self.loaded_sample_wallets(c, set);
                let open: Vec<usize> = (0..set.wallets.len())
                    .filter(|s| !loaded.iter().any(|(l, _)| l == s))
                    .collect();
                let all: Vec<usize> = (0..set.wallets.len()).collect();
                let list = if open.is_empty() { &all } else { &open };
                let at = usize::from(cursor) % list.len().max(1);
                if let Some(s) = self.camera.scan_mut() {
                    s.sample = ((at + 1) % list.len().max(1)) as u8;
                }
                list.get(at)
                    .map(|s| set.wallets[*s].text.as_bytes().to_vec())
            }
            // Check an address, from the Tools tile: the sample address.
            ScanPurpose::Files if self.screen == Screen::Catalog => {
                Some(set.inputs.address.as_bytes().to_vec())
            }
            ScanPurpose::Files => self.sample_spend(c, set).map(<[u8]>::to_vec),
            ScanPurpose::Transfer => None,
        }
    }

    /// Handles `Action::ScanSample`: the scanner reads the sample as if
    /// the camera had.
    pub(crate) fn scan_use_sample(&mut self) {
        let Some(set) = self.put_on() else { return };
        if !self.scan_sample_offered() {
            return;
        }
        let c = self.compute();
        if let Some(bytes) = self.scan_sample_bytes(&c, set) {
            let intake = self
                .camera
                .scan()
                .is_some_and(|s| s.purpose == ScanPurpose::Intake);
            self.scanned(bytes);
            // The intake scanner stays on for the next code; the sample
            // transaction is the last thing it has to bring.
            if intake && self.camera.scan().is_some() {
                self.camera.close();
                self.sheet = None;
                self.out.commands.push_back(crate::Command::CameraOff);
            }
        }
    }

    // A cosigner's account key.

    /// Whether cosigner box `k` of the seeds piece draws Use a sample:
    /// empty, not typed in, not pressed in. A lookup.
    pub fn seeds_sample_shown(&self, s: &crate::seeds::SeedsState, k: usize) -> bool {
        self.put_on().is_some()
            && s.sample_off & (1 << k.min(15)) == 0
            && s.cosigners.get(k).is_some_and(|t| t.trim().is_empty())
    }

    /// Costs a stretched key and a key derivation. Handles
    /// `SeedsAction::Sample`: cosigner box `k` gets the account key, at
    /// the path chosen, of the next sample seed that is not among the
    /// wallet's and not a cosigner already.
    pub(crate) fn seeds_use_sample(&mut self, k: usize) {
        let Some(set) = self.put_on() else { return };
        let net = self.session.network();
        let Some(s) = self.seeds().cloned() else {
            return;
        };
        if !self.seeds_sample_shown(&s, k) {
            return;
        }
        let here = self.seeds_here(&s);
        let used: Vec<[u8; 4]> = s.cosigners.iter().filter_map(|t| origin_of(t)).collect();
        let cands: Vec<usize> = english_keys(set)
            .filter(|(_, key)| {
                key.plain
                    .is_some_and(|p| !here.contains(&p) && !used.contains(&p))
            })
            .map(|(i, _)| i)
            .collect();
        let Some((i, next)) = self.sample_choose(set, &cands, s.sample_next) else {
            return;
        };
        let c = self.compute();
        let made = crate::create::path_spec(&s.path(net)).and_then(|spec| {
            faraday_keys::identity::key_text_of_words(
                &c,
                set.keys[i].parts.first().copied().unwrap_or_default(),
                "",
                net,
                &spec,
            )
        });
        if let Some(st) = self.seeds_mut() {
            match made {
                Ok(text) => {
                    if let Some(slot) = st.cosigners.get_mut(k) {
                        *slot = text;
                    }
                    st.sample_next = next;
                    st.focus = None;
                    st.error = None;
                }
                Err(e) => st.error = Some(e),
            }
        }
    }
}

/// A Use a sample button on a screen with no field to put it in, at
/// (`x`, `y`) and the touch floor's height: drawn only where samples are
/// set (Demo). Returns its height with the gap under it, 0 where not
/// drawn.
pub(crate) fn button(app: &Faraday, ui: &mut Ui, x: f32, y: f32) -> f32 {
    if app.put_on().is_none() {
        return 0.0;
    }
    let h = tokens::TEXT_FIELD;
    ui.button(
        x,
        y,
        None,
        h,
        EN.use_a_sample,
        Style::Secondary,
        Action::UseSample,
    );
    h + tokens::GAP
}

// The inputs of other flows (`docs/MODES.md` §5.3, §6).

impl Faraday {
    /// Handles `Action::UseSample` by the screen it was drawn on: the
    /// Vaults list, Lightning's cipher seed, Find an address, or an
    /// entry field.
    pub(crate) fn use_sample(&mut self) {
        match self.screen {
            Screen::Vaults => self.vaults_use_sample(),
            Screen::Lightning => self.lightning_use_sample(),
            Screen::Explore => self.explore_use_sample(),
            Screen::Backup => self.copy_check_use_sample(),
            _ => self.entry_use_sample(),
        }
    }

    /// Costs what `Item::new` costs. The Vaults screen with no vault:
    /// the sample vault goes into the Inbox as a file of the session,
    /// written nowhere, and is listed and unlocked like any other
    /// (under `faraday`).
    fn vaults_use_sample(&mut self) {
        let Some(set) = self.put_on() else { return };
        if !self.vault_files().is_empty() {
            return;
        }
        let c = self.compute();
        let item = crate::Item::new(&c, EN.sample_vault_file, set.inputs.vault.to_vec());
        self.inbox.push(item);
    }

    /// Costs what `Item::new` costs. Check a signed message with no
    /// message in the Inbox: the sample message goes in as a file of the
    /// session, and its check opens.
    pub(crate) fn check_use_sample(&mut self) {
        let Some(set) = self.put_on() else { return };
        let c = self.compute();
        let text = set.inputs.signed_message.as_bytes().to_vec();
        self.inbox
            .push(crate::Item::new(&c, EN.sample_message_file, text));
        self.act(Action::CheckMessage(self.inbox.len() - 1));
    }

    /// Whether the aezeed's words field draws Use a sample: Demo, the
    /// cipher seed chosen, nothing typed or pressed. A lookup.
    pub fn lightning_sample_shown(&self) -> bool {
        self.put_on().is_some() && self.lightning.sample_shown()
    }

    /// Handles `UseSample` on Lightning: the sample cipher seed's words
    /// are typed in; Read it goes on as it does for typed words.
    fn lightning_use_sample(&mut self) {
        let Some(set) = self.put_on() else { return };
        if !self.lightning_sample_shown() {
            return;
        }
        let words = set
            .keys
            .iter()
            .find(|k| k.form == SampleForm::Aezeed)
            .and_then(|k| k.parts.first().copied());
        if let Some(words) = words {
            self.lightning.use_sample(words);
        }
    }

    /// Whether Find an address's field draws Use a sample: Demo, nothing
    /// typed or pressed. A lookup.
    pub fn explore_sample_shown(&self) -> bool {
        self.put_on().is_some() && self.explore.sample_shown()
    }

    /// Handles `UseSample` on Find an address: the sample address is put
    /// in as typed; Search goes on as it does for a typed one.
    fn explore_use_sample(&mut self) {
        let Some(set) = self.put_on() else { return };
        if !self.explore_sample_shown() {
            return;
        }
        self.explore.use_sample(set.inputs.address);
    }
}

// Check my copy (the backup flow's typed words check).

impl Faraday {
    /// The words of the sample whose seed the open copy check is of:
    /// English words of the check's count. A lookup.
    fn copy_check_sample_words(&self) -> Option<&'static str> {
        let set = self.put_on()?;
        let b = self.backup.as_ref()?;
        let chk = b.check.as_ref().filter(|c| !c.passphrase)?;
        let fp = self.session.keys.info(b.key)?.fingerprint.0;
        english_keys(set)
            .find(|(_, k)| {
                (k.plain == Some(fp) || k.with_passphrase == Some(fp)) && word_count(k) == chk.count
            })
            .and_then(|(_, k)| k.parts.first().copied())
    }

    /// Whether Check my copy's empty grid draws **Fill words**: Demo with
    /// samples set, the key checked a sample, nothing typed or pressed.
    /// A lookup.
    pub fn copy_check_sample_shown(&self) -> bool {
        self.backup
            .as_ref()
            .and_then(|b| b.check.as_ref())
            .is_some_and(|c| {
                !c.sample_gone
                    && c.lang == Language::English
                    && c.taken() == 0
                    && c.typing().is_empty()
            })
            && self.copy_check_sample_words().is_some()
    }

    /// Costs what `copy_check_after` costs. Handles `Action::UseSample` on
    /// Check my copy: the sample's words are typed key by key, as a person
    /// would, and the check runs when the last is taken.
    pub(crate) fn copy_check_use_sample(&mut self) {
        if !self.copy_check_sample_shown() {
            return;
        }
        let Some(words) = self.copy_check_sample_words() else {
            return;
        };
        for word in words.split_whitespace() {
            for ch in word.chars() {
                self.copy_check_key(faraday_shell_api::Key::Char(ch));
            }
            self.copy_check_key(faraday_shell_api::Key::Char(' '));
        }
    }
}
