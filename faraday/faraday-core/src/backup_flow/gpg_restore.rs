//! A GPG key restored from its paperkey and its public certificate
//! (`docs/GPG-PAPERKEY.md` §4, §6), the backup flow's state for it: the
//! certificate the flow is for ([`RestoreFrom`], public, kept across a
//! lock with the plan), the sheet's rows, and step 4's lines being typed
//! back ([`RestoreLive`]), held in the flow area. The lines live in
//! faraday-keys' restore set from the moment each is checked; the line
//! being typed is transient input, wiped when it is handed in, when the
//! flow goes and at a lock. The cards are `gpg_restore_screen.rs`; the glue
//! that saves the key into the vaults is `steps.rs`.

use faraday_crypto::work::Compute;
use faraday_keys::gpg::{CodeRefused, LineError, LineRefused, Restore, Restored};
use faraday_pgp::{Armor, Certificate, Refusal, fingerprint_text};
use faraday_shell_api::Key as KeyIn;

use super::{BackupFlowArea, RestorePress};
use crate::Item;
use crate::secret_text::SecretText;
use crate::strings::{EN, fill};

/// The head card's number among the cards (`live.open`): not a step's.
pub const HEAD: u8 = 100;

/// **Restore from a paperkey**: the press that opens the sheet.
pub const SHEET: crate::Action =
    crate::Action::Flow(super::steps::FlowAction::Restore(RestorePress::Sheet));

/// Vault contents' GPG kind: its **Restore from a paperkey** button.
pub(crate) fn contents_button() -> (String, crate::Action) {
    (EN.gpg_restore_action.to_string(), SHEET)
}

/// The bytes a line holds at paperkey's default width.
const LINE_BYTES: usize = 22;

/// The bytes a line shows on a row where the whole line does not fit:
/// half of paperkey's line (§4.5).
pub(crate) const WRAP_BYTES: usize = 11;

/// The certificate a restore flow is for: the file as it came, and what
/// it states, read once when it was chosen. Public, all of it; kept
/// across a lock with the plan. What it states is a claim until the key
/// is restored, which makes every signature of the key's again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreFrom {
    /// The armoured certificate, as it came.
    pub file: Vec<u8>,
    /// Its first user ID.
    pub user_id: String,
    /// The primary key's fingerprint.
    pub fingerprint: [u8; 20],
    /// The signing subkey's.
    pub subkey: [u8; 20],
    /// When the key was made.
    pub created: u32,
    /// Seconds after creation it expires, 0 for never, as the newest
    /// self-signature says.
    pub expiry: u32,
    /// When a revocation by the key says it was made.
    pub revoked: Option<u32>,
    /// The signatures by other keys, which the record does not keep.
    pub set_aside: usize,
    /// Line 1 of its paperkey, as paperkey prints its bytes: `00 04` and
    /// the fingerprint.
    pub line_one: String,
    /// The fingerprint as GnuPG prints it, in groups of four.
    pub fingerprint_text: String,
    /// The subkey's, the same way.
    pub subkey_text: String,
    /// The fingerprint's last eight digits: the flow's chip.
    pub chip: String,
}

impl RestoreFrom {
    /// Costs a SHA-1 of each key. The certificate in `file`, read for a
    /// restore, or why it is not (`docs/GPG-PAPERKEY.md` §4.4).
    pub fn read(file: &[u8]) -> Result<RestoreFrom, String> {
        let cert = Certificate::read(file).map_err(|r| refusal_text(&r))?;
        let fp = cert.fingerprint();
        let mut line_one = String::from("00 04");
        for b in fp {
            line_one.push_str(&format!(" {b:02X}"));
        }
        let hex: String = fp.iter().map(|b| format!("{b:02X}")).collect();
        Ok(RestoreFrom {
            fingerprint_text: fingerprint_text(&fp),
            subkey_text: fingerprint_text(&cert.subkey_fingerprint()),
            chip: hex[hex.len() - 8..].to_string(),
            file: file.to_vec(),
            user_id: cert.user_ids().first().cloned().unwrap_or_default(),
            fingerprint: fp,
            subkey: cert.subkey_fingerprint(),
            created: cert.created(),
            expiry: cert.expiry(),
            revoked: cert.revoked_at(),
            set_aside: cert.set_aside(),
            line_one,
        })
    }
}

/// What a refusal says (`docs/GPG-PAPERKEY.md` §4.4).
pub fn refusal_text(r: &Refusal) -> String {
    match r {
        Refusal::NotCertificate => EN.gpg_restore_not_public.to_string(),
        Refusal::Algorithm(a) => fill(EN.gpg_restore_algorithm, &[a]),
        Refusal::Version(v) => fill(EN.gpg_restore_version, &[&v.to_string()]),
        Refusal::Shape => EN.gpg_restore_shape.to_string(),
        Refusal::NoUserId => EN.gpg_restore_no_user_id.to_string(),
        Refusal::FormatVersion(v) => fill(EN.gpg_restore_format, &[&v.to_string()]),
        Refusal::OtherKey(fp) => fill(EN.gpg_restore_other_key, &[&fingerprint_text(fp)]),
        Refusal::OtherSubkey(fp) => fill(EN.gpg_restore_other_subkey, &[&fingerprint_text(fp)]),
        Refusal::Count(n) => fill(EN.gpg_restore_count, &[&n.to_string()]),
        Refusal::Protected => EN.gpg_restore_protected.to_string(),
        Refusal::Crc => EN.gpg_restore_crc_all.to_string(),
        Refusal::Unreadable => EN.gpg_restore_unreadable.to_string(),
        Refusal::Mismatch => EN.gpg_restore_mismatch.to_string(),
        Refusal::NotOurs => EN.gpg_restore_not_ours.to_string(),
    }
}

/// One public key in Files, as the sheet lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetRow {
    /// Its place in the Inbox.
    pub index: usize,
    /// Its user ID, or its file name where it does not read.
    pub user_id: String,
    /// Its fingerprint in groups of four; empty where it does not read.
    pub fingerprint: String,
    /// Why it is not restored here, where it is not.
    pub refused: Option<String>,
}

/// Costs a SHA-1 of each key of each public key. The sheet's rows: every
/// public key in Files, those that read as a certificate to restore from
/// and, under them, those that do not with why.
pub fn sheet_rows(inbox: &[Item]) -> Vec<SheetRow> {
    let mut rows: Vec<SheetRow> = inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| {
            i.kind == crate::FileKind::Gpg
                && faraday_pgp::read_public_armor(&i.bytes) == Some(Armor::PublicKey)
        })
        .map(|(index, i)| match RestoreFrom::read(&i.bytes) {
            Ok(r) => SheetRow {
                index,
                user_id: r.user_id,
                fingerprint: fingerprint_text(&r.fingerprint),
                refused: None,
            },
            Err(e) => SheetRow {
                index,
                user_id: i.name.clone(),
                fingerprint: String::new(),
                refused: Some(e),
            },
        })
        .collect();
    rows.sort_by_key(|r| r.refused.is_some());
    rows
}

/// Whether the Inbox file is a public key a restore can start from: what
/// Files' ⋮ offers **Restore from its paperkey** on. Costs a SHA-1 of
/// each key.
pub fn restorable(item: &Item) -> bool {
    item.kind == crate::FileKind::Gpg && RestoreFrom::read(&item.bytes).is_ok()
}

/// The hex digits after a line's colon, or in all of it without one.
fn digits(typed: &str) -> usize {
    let rest = typed.split_once(':').map_or(typed, |(_, r)| r);
    rest.chars().filter(char::is_ascii_hexdigit).count()
}

/// Step 4's lines being typed (`docs/GPG-PAPERKEY.md` §4.5, §6): the
/// restore set, which holds each line checked, the line being typed, and
/// what the card says under it. Drawing reads it; the presses change it.
pub struct RestoreLive {
    /// The key it restores.
    pub(crate) fingerprint: [u8; 20],
    set: Restore,
    /// The line being typed: transient input, wiped when it is handed in.
    typed: SecretText,
    /// Why the line typed was not taken: shown under it.
    error: Option<String>,
    /// Why **Restore the key** did not restore: shown under the button.
    refused: Option<String>,
    /// The open vault that holds the key already, by its place, and its
    /// name.
    in_vault: Option<(usize, String)>,
    /// Typing goes to the line.
    pub(crate) focused: bool,
    /// The fingerprint line 1 names, in groups of four, once it is
    /// checked: public. Kept in step with the lines at every line taken
    /// or cleared.
    fingerprint_shown: Option<String>,
}

impl RestoreLive {
    /// Costs a SHA-1 of each key. The lines of the key `from` is for, none
    /// typed yet.
    pub fn new(from: &RestoreFrom) -> Option<RestoreLive> {
        let cert = Certificate::read(&from.file).ok()?;
        Some(RestoreLive {
            fingerprint: from.fingerprint,
            set: Restore::new(cert),
            typed: SecretText::new(),
            error: None,
            refused: None,
            in_vault: None,
            focused: true,
            fingerprint_shown: None,
        })
    }

    /// The fingerprint line 1 names, once it is checked.
    pub fn fingerprint_text(&self) -> Option<&str> {
        self.fingerprint_shown.as_deref()
    }

    /// The fingerprint shown kept in step with the lines checked.
    fn sync_shown(&mut self) {
        self.fingerprint_shown = self.set.fingerprints().first().map(fingerprint_text);
    }

    /// The number of the line being typed.
    pub fn next(&self) -> u32 {
        self.set.next()
    }

    /// The lines checked, the CRC line included.
    pub fn checked(&self) -> usize {
        self.set.checked()
    }

    /// How many lines the paperkey has, once the lengths are read.
    pub fn lines(&self) -> Option<usize> {
        self.set.lines()
    }

    /// Every line is checked.
    pub fn complete(&self) -> bool {
        self.set.complete()
    }

    /// The line being typed, as typed.
    pub fn typed(&self) -> &str {
        self.typed.as_str()
    }

    /// Why the line typed was not taken.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Why the key was not restored.
    pub fn refused(&self) -> Option<&str> {
        self.refused.as_deref()
    }

    /// The open vault that holds the key already.
    pub fn in_vault(&self) -> Option<&(usize, String)> {
        self.in_vault.as_ref()
    }

    /// The bytes this line holds when it is whole: 22, fewer for the last
    /// data line, none for the CRC line, once the lengths say.
    fn expected(&self) -> usize {
        self.set.left().map_or(LINE_BYTES, |l| l.min(LINE_BYTES))
    }

    /// A character typed: a hex digit, a space or the colon after the
    /// line's number goes into the line; any other is refused and the line
    /// kept. A line whole and right ends by itself.
    fn push(&mut self, c: char) {
        if self.set.complete() {
            return;
        }
        if !(c.is_ascii_hexdigit() || c == ' ' || c == ':')
            || (c == ':' && self.typed.contains(':'))
        {
            self.error = Some(EN.gpg_restore_chars.to_string());
            return;
        }
        self.typed.push(c.to_ascii_uppercase());
        self.error = None;
        let d = digits(&self.typed);
        if c.is_ascii_hexdigit() && d == 2 * self.expected() + 6 {
            self.hand_in(true);
        }
    }

    /// Backspace: the last character of the line goes; at the start of an
    /// empty line, nothing (a checked line is not reopened by a key).
    fn back(&mut self) {
        self.typed.pop();
        self.error = None;
    }

    /// The line handed to the restore set: taken and cleared, or kept with
    /// why. `quiet` is a line that ended by itself: a CRC that does not
    /// match then only means the line runs on.
    fn hand_in(&mut self, quiet: bool) {
        let n = self.set.next();
        // An empty line ends nothing: the one before it ended by itself.
        if self.typed.trim().is_empty() {
            return;
        }
        // Digits that are not pairs and a CRC are refused as a CRC that
        // does not match, as paperkey refuses them.
        match self.set.line(n, &self.typed) {
            Ok(()) => {
                self.typed.clear();
                self.error = None;
                self.sync_shown();
            }
            Err(LineRefused::Line(LineError::Crc)) if quiet => {}
            Err(e) => self.error = Some(line_text(n, &e)),
        }
    }

    /// Costs a CRC-24 over each line and over the whole. A paperkey read
    /// from a code: every line taken at once, in place of those typed, or
    /// why not, those typed kept.
    fn paperkey_code(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.set.paperkey_code(bytes).map_err(|e| code_text(&e))?;
        self.sync_shown();
        self.typed.clear();
        self.error = None;
        self.refused = None;
        self.in_vault = None;
        Ok(())
    }

    /// Line `n` and those after it cleared, to be typed again.
    fn again(&mut self, n: u32) {
        self.set.again(n);
        self.sync_shown();
        self.typed.clear();
        self.error = None;
        self.refused = None;
        self.in_vault = None;
        self.focused = true;
    }

    /// Every line forgotten, the certificate kept: a lock's.
    fn wipe(&mut self) {
        self.set.wipe();
        self.sync_shown();
        self.typed.clear();
        self.error = None;
        self.refused = None;
        self.in_vault = None;
    }

    /// Costs two Ed25519 public keys and an Ed25519 signature for each
    /// signature of the key's (`Restore::finish`). The key restored, or
    /// why not, said under the button.
    pub(crate) fn finish(&mut self, c: &Compute) -> Option<Restored> {
        self.in_vault = None;
        match self.set.finish(c) {
            Ok(r) => {
                self.refused = None;
                Some(r)
            }
            Err(e) => {
                self.refused = Some(refusal_text(&e));
                None
            }
        }
    }

    /// The key is in open vault `o`, named `name`: nothing is saved.
    pub(crate) fn refuse_in_vault(&mut self, o: usize, name: String) {
        self.refused = Some(fill(EN.gpg_restore_in_vault, &[&name]));
        self.in_vault = Some((o, name));
    }
}

/// The key restored as the flow saves a key made: its record, known by
/// its fingerprint, shown under its first user ID; it puts nothing For
/// the stick, as its certificate is out already.
pub(crate) fn built(r: Restored) -> super::Built {
    super::Built {
        name: r.made.facts.user_ids.first().cloned().unwrap_or_default(),
        subject: r.made.facts.fingerprint.to_vec(),
        record: r.made.key,
        outbox: Vec::new(),
    }
}

/// What a line refused says.
fn line_text(n: u32, e: &LineRefused) -> String {
    match e {
        LineRefused::Line(LineError::Chars) => EN.gpg_restore_chars.to_string(),
        // The line handed in is always the next one, so a number refused
        // is the one typed before the colon.
        LineRefused::Line(LineError::Number { typed }) => {
            fill(EN.gpg_restore_number, &[&n.to_string(), &typed.to_string()])
        }
        LineRefused::Line(LineError::Crc) => fill(EN.gpg_restore_crc, &[&n.to_string()]),
        LineRefused::Paperkey(r) => refusal_text(r),
    }
}

/// What a paperkey code refused says.
fn code_text(e: &CodeRefused) -> String {
    match e {
        CodeRefused::NotPaperkey => EN.gpg_restore_not_paperkey.to_string(),
        CodeRefused::Line(n, e) => line_text(*n, e),
        CodeRefused::Paperkey(r) => refusal_text(r),
        CodeRefused::Short => EN.gpg_restore_crc_missing.to_string(),
        CodeRefused::After(n) => fill(EN.gpg_restore_after, &[&n.to_string()]),
    }
}

/// The certificate file kept across a lock, as hex.
pub(crate) fn file_hex(file: &[u8]) -> String {
    file.iter().map(|b| format!("{b:02x}")).collect()
}

/// The certificate file read back from [`file_hex`].
pub(crate) fn file_of_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

impl BackupFlowArea {
    /// The lines being typed for flow `i`, while it is a restore.
    pub(crate) fn restoring(&self, i: usize) -> Option<&RestoreLive> {
        let from = self.all.get(i)?.restore.as_ref()?;
        self.restoring
            .as_deref()
            .filter(|r| r.fingerprint == from.fingerprint)
    }

    fn restoring_mut(&mut self, i: usize) -> Option<&mut RestoreLive> {
        let fp = self.all.get(i)?.restore.as_ref()?.fingerprint;
        self.restoring
            .as_deref_mut()
            .filter(|r| r.fingerprint == fp)
    }

    /// Costs a SHA-1 of each key, when the lines are not held yet. Step 4
    /// of restore flow `i` on screen: its lines held, line 1 asked for
    /// after a lock (`docs/GPG-PAPERKEY.md` §6).
    pub(crate) fn restore_ready(&mut self, i: usize) {
        if self.restoring(i).is_some() {
            return;
        }
        self.restoring = self
            .all
            .get(i)
            .and_then(|f| f.restore.as_ref())
            .and_then(RestoreLive::new)
            .map(Box::new);
    }

    /// The head card's presses, and step 4's that need nothing but the
    /// lines: the field, Next line, Type line n again. The hex keyboard's
    /// keys come as keys typed ([`Self::restore_key`]).
    pub(crate) fn restore_press(&mut self, i: usize, p: RestorePress) {
        use RestorePress as R;
        match p {
            R::Head => {
                let Some(f) = self.all.get_mut(i) else {
                    return;
                };
                f.live.open = if f.live.open == Some(HEAD) {
                    Some(f.step.n())
                } else {
                    Some(HEAD)
                };
            }
            R::HeadContinue => {
                if let Some(f) = self.all.get_mut(i) {
                    f.live.open = Some(f.step.n());
                }
            }
            _ => {
                let Some(r) = self.restoring_mut(i) else {
                    return;
                };
                match p {
                    R::Field => r.focused = true,
                    R::Next => r.hand_in(false),
                    R::Again(n) => r.again(n),
                    _ => {}
                }
            }
        }
    }

    /// Whether step 4 of flow `i` takes typing now: a restore's line
    /// being typed, focused. Where the hex keyboard docks.
    pub(crate) fn restore_typing(&self, i: usize) -> bool {
        self.all
            .get(i)
            .is_some_and(|f| f.step == super::Step::Make && !f.made && !f.adding)
            && self
                .restoring(i)
                .is_some_and(|r| r.focused && !r.complete())
    }

    /// Costs what `RestoreLive::paperkey_code` costs. A paperkey code read for flow
    /// `i`'s step 4: its lines taken, or why not.
    pub(crate) fn restore_code(&mut self, i: usize, bytes: &[u8]) -> Result<(), String> {
        if !self.restore_typing(i) {
            return Err(EN.gpg_restore_not_paperkey.to_string());
        }
        self.restoring_mut(i)
            .ok_or_else(|| EN.gpg_restore_not_paperkey.to_string())?
            .paperkey_code(bytes)
    }

    /// A key typed while step 4's line takes typing; whether it took it.
    pub(crate) fn restore_key(&mut self, i: usize, key: KeyIn) -> bool {
        if !self.restore_typing(i) {
            return false;
        }
        let Some(r) = self.restoring_mut(i) else {
            return false;
        };
        match key {
            KeyIn::Char(c) => r.push(c),
            KeyIn::Backspace => r.back(),
            KeyIn::Enter => r.hand_in(false),
            _ => return false,
        }
        true
    }

    /// Costs two Ed25519 public keys of each GPG record in the open vaults
    /// (`record_in`), and what `RestoreLive::finish` costs. **Restore the
    /// key** of flow `i`: refused where an open vault holds the key
    /// already, saying which; else the key made from the lines, as Make
    /// gives the flow one, or why not, said under the button.
    pub(crate) fn restore_finish(
        &mut self,
        c: &Compute,
        i: usize,
        vaults: &crate::vault::Vaults,
    ) -> Option<super::Built> {
        let fp = super::hex_text(&self.all.get(i)?.restore.as_ref()?.fingerprint);
        let held = (0..vaults.open.len())
            .find(|&o| super::record_in(vaults, o, super::Kind::Gpg, &fp).is_some());
        let r = self.restoring_mut(i)?;
        if let Some(o) = held {
            r.refuse_in_vault(o, vaults.open[o].name.clone());
            return None;
        }
        r.finish(c).map(built)
    }

    /// A lock: every line typed is wiped, the certificates kept with their
    /// flows.
    pub(crate) fn restore_wipe(&mut self) {
        if let Some(r) = self.restoring.as_mut() {
            r.wipe();
        }
        self.restoring = None;
    }

    /// The sheet's rows, as read when it opened.
    pub(crate) fn restore_rows(&self) -> &[SheetRow] {
        &self.restore_rows
    }

    /// Costs what [`sheet_rows`] costs. The sheet's rows read from the
    /// Inbox's `items`, as it opens.
    pub(crate) fn restore_sheet(&mut self, items: &[Item]) {
        self.restore_rows = sheet_rows(items);
    }

    /// Costs a SHA-1 of each key, and what `BackupFlow::new` costs. The
    /// restore flow of the public key that is `item`: its unfinished one,
    /// or a new one at step 1. `None` for a file that does not read as one.
    pub(crate) fn restore_flow(&mut self, c: &Compute, item: Option<&Item>) -> Option<usize> {
        let from = RestoreFrom::read(&item?.bytes).ok()?;
        let open = self.all.iter().position(|f| {
            !f.made
                && f.restore
                    .as_ref()
                    .is_some_and(|r| r.fingerprint == from.fingerprint)
        });
        Some(open.unwrap_or_else(|| {
            self.all.push(super::BackupFlow::restoring(c, from));
            self.all.len() - 1
        }))
    }
}
