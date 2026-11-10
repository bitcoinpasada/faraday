//! Check a timestamp in Tools (`docs/decisions/F230.md`): a step flow of
//! four cards, Proof, File, Block header and Result. The proof (`.ots`)
//! and the file it stamps are chosen from Files, the file also by typing
//! its SHA-256; the block header is typed or pasted as hex, scanned, or
//! chosen from Files. Everything checked is public data and is kept
//! until the tool is left. The reading and the walk are `faraday-ots`'s;
//! this area gives them the files and says what they found. The drawing
//! is `screen.rs`.

use crate::strings::{EN, fill};
use crate::{Action, Faraday, Screen, flow};
use faraday_crypto::work::Compute;
use faraday_ots::{Error, HashOp, Header, Network, Outcome, Proof};
use faraday_shell_api::Key;

pub(crate) mod screen;

/// The cards, in order.
pub mod ostep {
    /// The `.ots` file.
    pub const PROOF: u8 = 0;
    /// The file it stamps.
    pub const FILE: u8 = 1;
    /// The block header.
    pub const HEADER: u8 = 2;
    /// What was found.
    pub const RESULT: u8 = 3;
    /// How many cards.
    pub const COUNT: usize = 4;
}

/// The two fields typing goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// The file's SHA-256.
    Hash,
    /// The block header's hex.
    Header,
}

/// A press in the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OtsAction {
    /// The Tools tile.
    Open,
    /// A card's header.
    Step(u8),
    /// This file of Files as the proof.
    Proof(usize),
    /// This file of Files as the file the proof stamps.
    File(usize),
    /// This file of Files as the block header.
    Header(usize),
    /// A field takes focus.
    Focus(Field),
    /// What is typed in the field is taken.
    Take(Field),
    /// The scanner for the block header.
    Scan,
}

/// A value taken from a file, a field or a scan, and where it came from.
pub(crate) struct Given<T> {
    /// The file's name, or Typed or Scanned.
    pub name: String,
    /// The value.
    pub value: T,
}

/// The file a proof stamps, as hashes: the SHA-256 always, the other
/// hashes a proof may name when the whole file was given.
pub(crate) struct FileGiven {
    /// The file's name, or Typed.
    pub name: String,
    /// Its SHA-256.
    pub sha256: Vec<u8>,
    /// Its SHA-1, RIPEMD-160 and Keccak-256, for a file given whole.
    pub others: Vec<(HashOp, Vec<u8>)>,
}

impl FileGiven {
    /// The digest under `op`, when this file gives one.
    fn digest(&self, op: HashOp) -> Option<&[u8]> {
        if op == HashOp::Sha256 {
            return Some(&self.sha256);
        }
        self.others
            .iter()
            .find(|(o, _)| *o == op)
            .map(|(_, d)| d.as_slice())
    }
}

/// How the result stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// A path is counted.
    Committed,
    /// No path is counted.
    NotCommitted,
    /// The file is not the one the proof states.
    FileDiffers,
    /// The proof names a hash the digest given cannot answer.
    NeedsFile,
}

/// The Result card's text, made once all three are in.
pub(crate) struct Shown {
    /// The verdict.
    pub verdict: Verdict,
    /// The block's hash, height and time of a counted path.
    pub facts: Vec<(String, String)>,
    /// One line per attestation that is not counted.
    pub lines: Vec<String>,
}

/// The tool's state.
pub struct OtsState {
    pub(crate) open: Option<u8>,
    pub(crate) scroll: flow::Scroll,
    pub(crate) back: Screen,
    pub(crate) proof: Option<Given<Proof>>,
    pub(crate) file: Option<FileGiven>,
    pub(crate) header: Option<Given<Header>>,
    pub(crate) hash_text: String,
    pub(crate) header_text: String,
    pub(crate) typing: Option<Field>,
    /// Why a card refused what it was given.
    pub(crate) refused: [Option<String>; 3],
    pub(crate) shown: Option<Shown>,
}

impl OtsState {
    fn new(back: Screen) -> OtsState {
        OtsState {
            open: Some(ostep::PROOF),
            scroll: flow::Scroll::default(),
            back,
            proof: None,
            file: None,
            header: None,
            hash_text: String::default(),
            header_text: String::default(),
            typing: None,
            refused: [None, None, None],
            shown: None,
        }
    }

    /// Whether the card is filled.
    pub(crate) fn done(&self, step: u8) -> bool {
        match step {
            ostep::PROOF => self.proof.is_some(),
            ostep::FILE => self.file.is_some(),
            ostep::HEADER => self.header.is_some(),
            _ => self.shown.is_some(),
        }
    }

    /// Whether a field has focus.
    pub fn typing(&self) -> bool {
        self.typing.is_some()
    }
}

/// The tool's place in Tools.
#[derive(Default)]
pub struct OtsArea {
    state: Option<OtsState>,
}

impl OtsArea {
    /// The state, while the tool is open.
    pub fn state(&self) -> Option<&OtsState> {
        self.state.as_ref()
    }

    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    pub(crate) fn scroll_mut(&mut self) -> Option<&mut flow::Scroll> {
        self.state.as_mut().map(|s| &mut s.scroll)
    }

    pub(crate) fn clear(&mut self) {
        self.state = None;
    }
}

/// The state, while the tool is open.
pub fn state(app: &Faraday) -> Option<&OtsState> {
    app.tools.ots().state()
}

fn st(app: &mut Faraday) -> Option<&mut OtsState> {
    app.tools.ots_mut().state.as_mut()
}

/// The first bytes of every proof file.
const PROOF_START: &[u8] = b"\x00OpenTimestamps\x00";

/// Whether a file of Files reads as a proof: by its first bytes.
pub(crate) fn is_proof(bytes: &[u8]) -> bool {
    bytes.starts_with(PROOF_START)
}

/// A header in a file or a code: 80 bytes as they are, or their hex.
pub(crate) fn header_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() == 80 {
        return Some(bytes.to_vec());
    }
    let text = std::str::from_utf8(bytes).ok()?;
    unhex(text, 80)
}

/// The bytes `text` writes in hex, spaces and line breaks left out, when
/// there are `len` of them.
fn unhex(text: &str, len: usize) -> Option<Vec<u8>> {
    let digits: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if digits.len() != 2 * len {
        return None;
    }
    digits
        .chunks(2)
        .map(|p| {
            let s = std::str::from_utf8(p).ok()?;
            u8::from_str_radix(s, 16).ok()
        })
        .collect()
}

/// The network a header is checked for.
fn network_of(n: faraday_bip::keys::Network) -> Network {
    use faraday_bip::keys::Network as N;
    match n {
        N::Mainnet => Network::Mainnet,
        N::Testnet => Network::Testnet3,
        N::Signet => Network::Signet,
        N::Regtest => Network::Regtest,
    }
}

/// One refusal of the reader, as one plain line.
pub(crate) fn error_text(e: Error) -> &'static str {
    match e {
        Error::ProofTooLarge => EN.ots_err_proof_too_large,
        Error::TooManyEntries => EN.ots_err_too_many_entries,
        Error::TooMuchWork => EN.ots_err_too_much_work,
        Error::BadMagic => EN.ots_err_bad_magic,
        Error::UnsupportedVersion => EN.ots_err_version,
        Error::Truncated => EN.ots_err_truncated,
        Error::TrailingBytes => EN.ots_err_trailing,
        Error::BadFileHashOp => EN.ots_err_file_hash_op,
        Error::UnknownOp => EN.ots_err_unknown_op,
        Error::EmptyArgument => EN.ots_err_empty_argument,
        Error::ArgumentTooLong => EN.ots_err_argument_long,
        Error::ResultTooLong => EN.ots_err_result_long,
        Error::TooDeep => EN.ots_err_too_deep,
        Error::NumberTooLarge => EN.ots_err_number,
        Error::HeightTooLarge => EN.ots_err_height,
        Error::PayloadTooLong => EN.ots_err_payload_long,
        Error::PayloadTrailingBytes => EN.ots_err_payload_trailing,
        Error::UriTooLong => EN.ots_err_uri_long,
        Error::UriInvalid => EN.ots_err_uri_invalid,
        Error::HeaderLength => EN.ots_err_header_length,
        Error::HeaderTarget => EN.ots_err_header_target,
        Error::HeaderTargetTooEasy => EN.ots_err_header_easy,
        Error::HeaderWork => EN.ots_err_header_work,
    }
}

/// The name of a hash, as the proof's operations name it.
fn op_name(op: HashOp) -> &'static str {
    match op {
        HashOp::Sha1 => EN.ots_op_sha1,
        HashOp::Ripemd160 => EN.ots_op_ripemd160,
        HashOp::Sha256 => EN.ots_op_sha256,
        HashOp::Keccak256 => EN.ots_op_keccak256,
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A time in seconds since 1970 as `2015-04-12 09:30 UTC`.
fn time_text(t: u32) -> String {
    let days = i64::from(t / 86_400);
    let (y, m, d) = faraday_crypto::date::civil_from_days(days);
    let s = t % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        s / 3600,
        s % 3600 / 60
    )
}

/// Costs the proof's walk (`Proof::verify`) worked out once: what the
/// three given say, in words.
fn shown(c: &Compute, proof: &Proof, file: &FileGiven, header: &Header) -> Shown {
    let Some(digest) = file.digest(proof.file_op()) else {
        return Shown {
            verdict: Verdict::NeedsFile,
            facts: vec![],
            lines: vec![fill(EN.ots_needs_file, &[op_name(proof.file_op())])],
        };
    };
    let report = proof.verify(c, digest, header);
    if !report.file_matches {
        return Shown {
            verdict: Verdict::FileDiffers,
            facts: vec![],
            lines: vec![],
        };
    }
    let mut facts = Vec::new();
    let mut lines = Vec::new();
    for o in &report.paths {
        match o {
            Outcome::BitcoinMatched {
                block_hash,
                height,
                time,
            } => {
                if facts.is_empty() {
                    facts.push((EN.ots_block_hash.to_string(), hex(block_hash)));
                    facts.push((EN.ots_height.to_string(), height.to_string()));
                    facts.push((EN.ots_time.to_string(), time_text(*time)));
                }
            }
            Outcome::BitcoinMismatch { height } => {
                lines.push(fill(EN.ots_mismatch, &[&height.to_string()]))
            }
            Outcome::BitcoinNotSha256 {
                height,
                op,
                reaches_header,
            } => lines.push(fill(
                if *reaches_header {
                    EN.ots_not_sha256_reaches
                } else {
                    EN.ots_not_sha256
                },
                &[&height.to_string(), op_name(*op)],
            )),
            Outcome::Pending { uri } => lines.push(fill(EN.ots_pending, &[uri])),
            Outcome::Litecoin { height } => {
                lines.push(fill(EN.ots_litecoin, &[&height.to_string()]));
            }
            Outcome::Ethereum { height } => {
                lines.push(fill(EN.ots_ethereum, &[&height.to_string()]));
            }
            Outcome::Unknown { tag } => lines.push(fill(EN.ots_unknown, &[&hex(tag)])),
        }
    }
    Shown {
        verdict: if report.committed() {
            Verdict::Committed
        } else {
            Verdict::NotCommitted
        },
        facts,
        lines,
    }
}

pub(crate) fn act(app: &mut Faraday, action: OtsAction) {
    match action {
        OtsAction::Open => {
            let back = app.screen;
            app.tools.ots_mut().state = Some(OtsState::new(back));
            app.screen = Screen::OtsCheck;
        }
        OtsAction::Step(k) => {
            if let Some(s) = st(app) {
                s.open = if s.open == Some(k) { None } else { Some(k) };
                s.typing = None;
                s.scroll.follow = true;
            }
        }
        OtsAction::Proof(i) => {
            let c = app.compute();
            let Some(item) = app.inbox.get(i) else { return };
            let (name, bytes) = (item.name.clone(), item.bytes.clone());
            take_proof(app, &c, name, &bytes);
        }
        OtsAction::File(i) => {
            let c = app.compute();
            let Some(item) = app.inbox.get(i) else { return };
            let (name, bytes) = (item.name.clone(), item.bytes.clone());
            take_file(app, &c, name, &bytes);
        }
        OtsAction::Header(i) => {
            let Some(item) = app.inbox.get(i) else { return };
            let (name, bytes) = (item.name.clone(), item.bytes.clone());
            take_header(app, name, &bytes);
        }
        OtsAction::Focus(f) => {
            if let Some(s) = st(app) {
                s.typing = Some(f);
            }
        }
        OtsAction::Take(f) => take_typed(app, f),
        OtsAction::Scan => {
            let mut scan = crate::qr_view::ScanState::default();
            scan.purpose = crate::ScanPurpose::OtsHeader;
            app.camera.open(scan);
            app.sheet = Some(crate::Sheet::Scan);
            app.out
                .commands
                .push_back(faraday_shell_api::Command::CameraOn);
        }
    }
}

/// Moves on to the next card that is not filled, or to the Result.
fn advance(s: &mut OtsState, from: u8) {
    s.typing = None;
    s.scroll.follow = true;
    s.open = (from + 1..=ostep::RESULT)
        .find(|k| *k == ostep::RESULT || !s.done(*k))
        .or(Some(ostep::RESULT));
    if s.shown.is_none() && s.open == Some(ostep::RESULT) {
        s.open = (ostep::PROOF..ostep::RESULT).find(|k| !s.done(*k));
    }
}

/// Costs the proof's parse (`Proof::parse`).
fn take_proof(app: &mut Faraday, c: &Compute, name: String, bytes: &[u8]) {
    let parsed = Proof::parse(c, bytes);
    let Some(s) = st(app) else { return };
    match parsed {
        Ok(p) => {
            s.proof = Some(Given { name, value: p });
            s.refused[0] = None;
            judge(app, c);
            if let Some(s) = st(app) {
                advance(s, ostep::PROOF);
            }
        }
        Err(e) => s.refused[0] = Some(error_text(e).to_string()),
    }
}

/// Costs a SHA-256, SHA-1, RIPEMD-160 and Keccak-256 pass over the file.
fn take_file(app: &mut Faraday, c: &Compute, name: String, bytes: &[u8]) {
    let sha256 = faraday_crypto::sha256_whole(c, bytes).to_vec();
    let others = [HashOp::Sha1, HashOp::Ripemd160, HashOp::Keccak256]
        .into_iter()
        .map(|op| (op, op.digest_whole(c, bytes)))
        .collect();
    let Some(s) = st(app) else { return };
    s.file = Some(FileGiven {
        name,
        sha256,
        others,
    });
    s.refused[1] = None;
    judge(app, c);
    if let Some(s) = st(app) {
        advance(s, ostep::FILE);
    }
}

/// Takes a header from a file, a field or a scan; false when it is none.
pub(crate) fn take_header(app: &mut Faraday, name: String, bytes: &[u8]) -> bool {
    let network = network_of(app.session.network());
    let parsed = match header_bytes(bytes) {
        Some(b) => Header::parse(network, &b).map_err(error_text),
        None => Err(EN.ots_not_a_header),
    };
    let c = app.compute();
    let Some(s) = st(app) else { return false };
    match parsed {
        Ok(h) => {
            s.header = Some(Given { name, value: h });
            s.refused[2] = None;
            judge(app, &c);
            if let Some(s) = st(app) {
                advance(s, ostep::HEADER);
            }
            true
        }
        Err(why) => {
            s.refused[2] = Some(why.to_string());
            false
        }
    }
}

fn take_typed(app: &mut Faraday, f: Field) {
    let c = app.compute();
    let Some(s) = st(app) else { return };
    match f {
        Field::Hash => {
            let text = s.hash_text.as_str().to_string();
            match unhex(&text, 32) {
                Some(d) => {
                    s.file = Some(FileGiven {
                        name: EN.ots_typed.to_string(),
                        sha256: d,
                        others: Vec::new(),
                    });
                    s.refused[1] = None;
                    judge(app, &c);
                    if let Some(s) = st(app) {
                        advance(s, ostep::FILE);
                    }
                }
                None => s.refused[1] = Some(EN.ots_not_a_hash.to_string()),
            }
        }
        Field::Header => {
            let text = s.header_text.as_str().to_string();
            take_header(app, EN.ots_typed.to_string(), text.as_bytes());
        }
    }
}

/// Costs what [`shown`] costs: makes the Result once all three are in,
/// and drops it while one is not.
fn judge(app: &mut Faraday, c: &Compute) {
    let Some(s) = st(app) else { return };
    s.shown = match (&s.proof, &s.file, &s.header) {
        (Some(p), Some(f), Some(h)) => Some(shown(c, &p.value, f, &h.value)),
        _ => None,
    };
    if s.shown.is_some() {
        s.open = Some(ostep::RESULT);
    }
}

/// A header the scanner read: Scanned, and the scanner closes. Whether
/// the scanner on screen is this one.
pub(crate) fn scanned(app: &mut Faraday, bytes: &[u8]) -> bool {
    if app.camera.scan().map(|s| s.purpose) != Some(crate::ScanPurpose::OtsHeader) {
        return false;
    }
    if take_header(app, EN.ots_scanned.to_string(), bytes) {
        app.scan_close();
    } else {
        let why = st(app)
            .and_then(|s| s.refused[2].take())
            .unwrap_or_else(|| EN.ots_not_a_header.to_string());
        app.scan_say(why);
    }
    true
}

/// The field typing goes to, with what is typed.
pub(crate) fn field(app: &Faraday) -> Option<(Action, &str)> {
    if app.screen != Screen::OtsCheck {
        return None;
    }
    let s = state(app)?;
    let f = s.typing?;
    let text = match f {
        Field::Hash => s.hash_text.as_str(),
        Field::Header => s.header_text.as_str(),
    };
    Some((Action::OtsCheck(OtsAction::Focus(f)), text))
}

/// The longest hex text a field takes, with room for spaces.
const FIELD_CHARS: usize = 400;

pub(crate) fn key(app: &mut Faraday, key: Key) -> bool {
    if app.screen != Screen::OtsCheck || app.sheet.is_some() {
        return false;
    }
    let Some(s) = st(app) else { return false };
    let Some(f) = s.typing else { return false };
    let text = match f {
        Field::Hash => &mut s.hash_text,
        Field::Header => &mut s.header_text,
    };
    match key {
        Key::Char(ch) if !ch.is_control() => {
            if text.as_str().len() < FIELD_CHARS {
                text.push(ch);
            }
            true
        }
        Key::Backspace => {
            text.pop();
            true
        }
        Key::Enter => {
            act(app, OtsAction::Take(f));
            true
        }
        Key::Escape => {
            s.typing = None;
            true
        }
        _ => false,
    }
}

/// After a press: the tool ends unless it is on show.
pub(crate) fn tidy(app: &mut Faraday) {
    if app.screen != Screen::OtsCheck {
        app.tools.ots_mut().clear();
    }
}

/// The tile: the tool, with Files' proof when there is one, else (Practice)
/// the sample.
pub(crate) fn open_tile(app: &mut Faraday) {
    if app.inbox.iter().any(|i| is_proof(&i.bytes)) {
        act(app, OtsAction::Open);
    } else {
        open_sample(app);
    }
}

/// Costs what the three takes cost: opens the tool with the sample proof,
/// the SHA-256 of the file it stamps and its block's header taken:
/// Practice's stand-in for the files to copy in.
pub(crate) fn open_sample(app: &mut Faraday) {
    let Some(set) = app.put_on() else { return };
    let inputs = set.inputs;
    if inputs.timestamp_proof.is_empty() {
        return;
    }
    let c = app.compute();
    act(app, OtsAction::Open);
    take_proof(
        app,
        &c,
        EN.sample_ots_proof_file.to_string(),
        inputs.timestamp_proof,
    );
    if let Some(s) = st(app) {
        s.hash_text = inputs.timestamp_hash.to_string();
    }
    take_typed(app, Field::Hash);
    take_header(
        app,
        EN.sample_ots_header_file.to_string(),
        inputs.timestamp_header.as_bytes(),
    );
}

/// The tool's steps, plugged into Back, Forward and the choices that move on.
pub(crate) struct OtsSteps;

/// The one [`OtsSteps`].
pub(crate) static OTS_STEPS: OtsSteps = OtsSteps;

impl crate::trail::StepFlow for OtsSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = state(app)?;
        Some(crate::trail::Steps::new(
            (0..ostep::COUNT as u8).collect(),
            s.open,
        ))
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::OtsCheck(OtsAction::Step(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        screen::TITLES[usize::from(k)]
    }
}
