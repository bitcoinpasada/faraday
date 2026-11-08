//! Making a Krux encrypted code (`docs/KEF.md` §4.1, §5, §6, §9, §10,
//! §15): the options and their defaults, the jobs that seal a seed's
//! entropy or a text, a refusal that names a version that works, and the
//! code made: a labelled picture and a file for the Outbox. The seed's form
//! on the backup board and the tool in Tools both call these; neither has
//! a copy.

use faraday_codec::kef::{Mode, Padding, VERSIONS, id_text, spec_of};
use faraday_codec::qr::{Ecc, alphanumeric_capacity};
use faraday_crypto::work::Compute;
use faraday_keys::kef::{
    COUNT_MIN, COUNT_SETTING_MAX, DEFAULT_COUNT, DEFAULT_ID_LEN, DEFAULT_VERSION, ID_MAX,
    LONG_ID_LEN, Refusal, SealError, unwrap,
};

use crate::qr_view::QrView;
use crate::strings::{EN, fill};
use crate::worker::kef::{KefId, KefStart};

/// What the code's id is (`docs/KEF.md` §10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdChoice {
    /// This many fresh random bytes: 8 by default, 16 one level down.
    Random(usize),
    /// A label typed by the person; public, printed on the code.
    Label(String),
    /// The wallet's fingerprint; public, printed on the code.
    Fingerprint([u8; 4]),
}

/// The options of one code, at the defaults of `docs/KEF.md` §10: version
/// 20, an 8-byte random id, 100,000 rounds, medium error correction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// The KEF version.
    pub version: u8,
    /// Low error correction, offered for version 5.
    pub low: bool,
    /// The count asked for; a public id adds Krux's random change.
    pub count: u32,
    /// The id.
    pub id: IdChoice,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            version: DEFAULT_VERSION,
            low: false,
            count: DEFAULT_COUNT,
            id: IdChoice::Random(DEFAULT_ID_LEN),
        }
    }
}

impl Options {
    /// What a name or count wrong in these options says, if any.
    pub fn problem(&self) -> Option<&'static str> {
        if !(COUNT_MIN..=COUNT_SETTING_MAX).contains(&self.count) {
            return Some(EN.kef_refuse_count);
        }
        let bad_id = match &self.id {
            IdChoice::Random(n) => *n != DEFAULT_ID_LEN && *n != LONG_ID_LEN,
            IdChoice::Label(l) => l.is_empty() || l.len() > ID_MAX,
            IdChoice::Fingerprint(_) => false,
        };
        if bad_id {
            return Some(EN.kef_refuse_id);
        }
        if spec_of(self.version).is_none() {
            return Some(EN.kef_refuse_version);
        }
        None
    }

    /// Low error correction is for version 5 only.
    pub fn low_offered(&self) -> bool {
        self.version == 5
    }

    pub(crate) fn kef_id(&self) -> KefId {
        match &self.id {
            IdChoice::Random(n) => KefId::Random(*n),
            IdChoice::Label(l) => KefId::Public(l.as_bytes().to_vec()),
            IdChoice::Fingerprint(f) => KefId::Public(f.to_vec()),
        }
    }
}

/// The side, in modules, of the code a seed's entropy or a text of
/// `plaintext_len` bytes will make with `options` (`docs/KEF.md` §5): the
/// smallest version that holds the envelope as base43 text. `None` for a
/// compressed version, whose size is not known before the work, and for
/// contents too large for one code.
pub fn side(options: &Options, plaintext_len: usize) -> Option<usize> {
    let spec = spec_of(options.version)?;
    if spec.compressed {
        return None;
    }
    let id_len = match &options.id {
        IdChoice::Random(n) => *n,
        IdChoice::Label(l) => l.len(),
        IdChoice::Fingerprint(_) => 4,
    };
    let hidden = usize::from(spec.auth.unsigned_abs()) * usize::from(spec.auth < 0);
    let exposed = usize::from(spec.auth.max(0).unsigned_abs());
    let body = plaintext_len + hidden;
    let body = match spec.padding {
        Padding::Nul => body.next_multiple_of(16),
        Padding::Pkcs7 => (body / 16 + 1) * 16,
        Padding::None => body,
    };
    let n = 5 + id_len + spec.iv_len() + body + exposed;
    // The envelope's first byte is the id's length, so the number is
    // below (id length + 1) * 256^(n - 1).
    let bits = ((id_len + 1) as f64).log2() + (8 * (n - 1)) as f64;
    let chars = (bits * std::f64::consts::LN_2 / 43f64.ln()).floor() as usize + 1;
    let ecc = if options.low && options.low_offered() {
        Ecc::Low
    } else {
        Ecc::Medium
    };
    (1..=25u8)
        .find(|&v| alphanumeric_capacity(v, ecc) >= chars)
        .map(|v| 17 + 4 * usize::from(v))
}

/// The versions Options lists in one mode, not the legacy ones.
pub fn versions_in(mode: Mode) -> Vec<u8> {
    VERSIONS
        .iter()
        .copied()
        .filter(|&v| spec_of(v).is_some_and(|s| s.mode == mode && !s.legacy()))
        .collect()
}

/// The legacy versions (0 and 1), listed one level further down.
pub fn legacy_versions() -> Vec<u8> {
    VERSIONS
        .iter()
        .copied()
        .filter(|&v| spec_of(v).is_some_and(|s| s.legacy()))
        .collect()
}

/// The modes in the order Options groups the versions.
pub const MODES: [Mode; 4] = [Mode::Gcm, Mode::Ctr, Mode::Cbc, Mode::Ecb];

/// What a refusal says, naming a version that works where there is one.
pub fn refusal_line(e: SealError) -> &'static str {
    match e {
        SealError::Empty => EN.kef_refuse_empty,
        SealError::Passphrase => EN.kef_refuse_passphrase,
        SealError::TooLong => EN.kef_refuse_too_long,
        SealError::Refused(Refusal::Count) => EN.kef_refuse_count,
        SealError::Refused(Refusal::IdLength) => EN.kef_refuse_id,
        SealError::Refused(_) | SealError::Iv | SealError::Cancelled => EN.kef_refuse_version,
        SealError::Duplicate { .. } => EN.kef_refuse_duplicate,
        SealError::NulEnd { .. } => EN.kef_refuse_nul_end,
    }
}

/// [`refusal_line`] with the version that works filled in.
pub fn refusal_text(e: SealError) -> String {
    match e {
        SealError::Duplicate { instead } | SealError::NulEnd { instead } => {
            fill(refusal_line(e), &[&instead.to_string()])
        }
        _ => refusal_line(e).to_string(),
    }
}

/// The line for a seed that is not offered.
pub fn not_offered_line() -> &'static str {
    EN.kef_not_offered
}

/// The line under the ECB versions in Options.
pub fn ecb_line() -> &'static str {
    EN.kef_ecb_line
}

/// The line about an id that is printed on the code.
pub fn public_id_line() -> &'static str {
    EN.kef_public_id_line
}

/// Why a code was not started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MakeStart {
    /// The seed is not one Krux's loader takes (12 or 24 words of the
    /// English list), or is not here.
    NotOffered,
    /// The options or the passphrase are refused before any work.
    Refused(&'static str),
    /// The worker did not start (`Faraday::kef_seal`).
    Not(KefStart),
}

/// A code made: its bytes, its picture and the files for the Outbox.
pub struct Made {
    /// The envelope's bytes.
    pub bytes: Vec<u8>,
    /// The code on a sheet, when it fits one code; otherwise why not.
    pub view: Result<QrView, String>,
    /// The picture's file name.
    pub png_name: String,
    /// The bytes' file name.
    pub file_name: String,
    /// The SHA-256 of the bytes; public.
    pub sha: [u8; 32],
    /// The picture was drawn at low error correction.
    pub low: bool,
    /// The picture carries a typed id's line.
    pub label: bool,
}

/// The names the Outbox gave a code's files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    /// The picture's, when the code fits one.
    pub png: Option<String>,
    /// The bytes' file.
    pub file: String,
}

impl Saved {
    /// The name a toast reads: the picture's, else the file's.
    pub fn toast_name(&self) -> &str {
        self.png.as_deref().unwrap_or(&self.file)
    }
}

/// Costs what `QrView::kef` costs, and a SHA-256 pass over the bytes. The code for `bytes`, labelled with
/// what is on the paper anyway: the version, the count and a typed id; it
/// never says what the code holds.
pub(crate) fn made(c: &Compute, bytes: Vec<u8>, options: &Options) -> Made {
    let envelope = unwrap(&bytes).ok();
    let (version, count, id) = envelope
        .as_ref()
        .map_or((options.version, options.count, Vec::new()), |e| {
            (e.version, e.count, e.id.clone())
        });
    let hex: String = id.iter().take(4).map(|b| format!("{b:02x}")).collect();
    let v = version.to_string();
    let png_name = fill(EN.kef_code_png_name, &[&v, &hex]);
    let file_name = fill(EN.kef_code_file_name, &[&v, &hex]);
    let mut lines = vec![fill(EN.kef_code_label_version, &[&v, &count.to_string()])];
    let mut label = false;
    if matches!(options.id, IdChoice::Label(_))
        && let Some(t) = id_text(&id)
    {
        lines.push(fill(EN.kef_code_label_id, &[t]));
        label = true;
    }
    let text = faraday_codec::kef::base43_encode(&bytes);
    let low = options.low && options.low_offered();
    let view = QrView::kef(c, &text, low, &png_name, lines);
    let sha = faraday_crypto::sha256_whole(c, &bytes);
    Made {
        bytes,
        view,
        png_name,
        file_name,
        sha,
        low,
        label,
    }
}
