//! Send text: any text a person brings to the online app, shown as codes
//! for the device's camera, except a secret (F236). The text is never
//! kept past Transfer: it is wiped when the screen is left.
//!
//! The refusal is the device's own classifier, reused: the whole text is
//! put through [`scan_read::secret_kind`] and read as a file, and so is
//! every part of it a secret can hide in (a token, a run of digits, a run
//! of words, an armoured key). It is worked out when the text changes and
//! once more at **Show as QR**; drawing reads the result.

use crate::scan_read::{self, Kind};
use crate::secrets::Exposure;
use crate::wallet;
use faraday_bip::bip39::Language;
use faraday_crypto::work::Compute;
use std::collections::HashSet;
use std::sync::OnceLock;
use zeroize::Zeroizing;

/// The most text sent: at the medium part size and the default speed
/// about a hundred frames (F236). More goes as a file.
pub const MAX_TEXT: usize = 16 * 1024;

/// The most text the field holds: what is pasted past it is dropped, so
/// the check never reads more than this.
const HOLD: usize = 4 * MAX_TEXT;

/// A press on the Send text card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPress {
    /// Put the clipboard's text in the field.
    Paste,
    /// Empty the field.
    Clear,
    /// Show the text as codes.
    Show,
}

/// What the check found in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextCheck {
    /// Nothing is refused: empty, or fit to show.
    Fine,
    /// Over [`MAX_TEXT`] bytes.
    TooLong,
    /// The whole text is this secret.
    Secret(Kind),
    /// This secret is in the text.
    Inside(Kind),
}

/// The Send text card's field, and what its check said.
#[derive(Debug)]
pub struct SendText {
    text: Zeroizing<String>,
    check: TextCheck,
}

impl Default for SendText {
    fn default() -> Self {
        Self {
            text: Zeroizing::new(String::new()),
            check: TextCheck::Fine,
        }
    }
}

impl SendText {
    /// The text in the field.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// What the check said of it.
    pub fn check(&self) -> TextCheck {
        self.check
    }

    /// The text as it is sent: its ends trimmed. `None` while there is
    /// nothing to send or the check refused it.
    pub fn sendable(&self) -> Option<&str> {
        let t = self.text.trim();
        (!t.is_empty() && self.check == TextCheck::Fine).then_some(t)
    }

    /// Whether the field holds anything.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Empties the field, wiping what it held.
    pub(crate) fn clear(&mut self) {
        self.text = Zeroizing::new(String::new());
        self.check = TextCheck::Fine;
    }

    /// Costs what [`check`] costs. A key typed into the field: whether it
    /// was taken. A character, Enter as a new line and Backspace.
    pub(crate) fn key(&mut self, c: &Compute, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        match key {
            K::Char(ch) if !ch.is_control() => {
                if self.text.len() + ch.len_utf8() <= HOLD {
                    self.text.push(ch);
                }
            }
            K::Enter => {
                if self.text.len() < HOLD {
                    self.text.push('\n');
                }
            }
            K::Backspace => {
                self.text.pop();
            }
            _ => return false,
        }
        self.recheck(c);
        true
    }

    /// Costs what [`check`] costs. The clipboard's text in place of
    /// what the field held.
    pub(crate) fn put(&mut self, c: &Compute, text: &str) {
        let mut cut = text.len().min(HOLD);
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        self.text = Zeroizing::new(text[..cut].to_string());
        self.recheck(c);
    }

    /// Costs what [`check`] costs. The check made again, on the text now
    /// in the field.
    pub(crate) fn recheck(&mut self, c: &Compute) {
        self.check = check(c, &self.text);
    }
}

/// Costs a classification of the whole text as a file and, per part of
/// it, what `scan_read::secret_kind` costs: a checksum or a parse for a
/// part that can be a secret, a lookup for each word. What is refused in
/// `text`: the device's classifier on the whole and on every part a secret
/// can hide in, a text over [`MAX_TEXT`]. Bare hex is not refused: a
/// SHA-256, a transaction id and a block hash are hex (F236).
pub fn check(c: &Compute, text: &str) -> TextCheck {
    let t = text.trim();
    if t.is_empty() {
        return TextCheck::Fine;
    }
    if t.len() > MAX_TEXT {
        return TextCheck::TooLong;
    }
    if let Some(kind) = scan_read::secret_kind(c, t.as_bytes()) {
        return TextCheck::Secret(kind);
    }
    if wallet::classify(c, "text.txt", t.as_bytes()).exposure() == Exposure::Secret {
        return TextCheck::Secret(Kind::Wif);
    }
    match inside(c, t) {
        Some(kind) => TextCheck::Inside(kind),
        None => TextCheck::Fine,
    }
}

/// Costs what [`check`] costs for the parts. The secret kind some part of
/// `t` is, if any.
fn inside(c: &Compute, t: &str) -> Option<Kind> {
    armour(t)
        .or_else(|| digits(t))
        .or_else(|| tokens(c, t))
        .or_else(|| words(c, t))
}

/// An armoured private key (PEM, PGP).
fn armour(t: &str) -> Option<Kind> {
    let upper = t.to_ascii_uppercase();
    (upper.contains("-----BEGIN") && upper.contains("PRIVATE KEY")).then_some(Kind::Wif)
}

/// A run of 48 or 96 digits: a SeedQR.
fn digits(t: &str) -> Option<Kind> {
    t.split(|ch: char| !ch.is_ascii_digit())
        .any(|run| matches!(run.len(), 48 | 96))
        .then_some(Kind::SeedQr)
}

/// Costs what `scan_read::secret_text_kind` costs, for each part of a
/// token long enough to be a key, and a classification for one that
/// holds a private key's letters. Each whitespace-separated token, whole,
/// trimmed of the punctuation round it and cut at the signs that join a
/// value to its name, as a secret of the classifier, and the `ur:` parts
/// put together.
fn tokens(c: &Compute, t: &str) -> Option<Kind> {
    let mut ur = faraday_codec::ur::Decoder::new();
    for tok in t.split_whitespace() {
        let lower = tok.to_ascii_lowercase();
        if lower.starts_with("ur:")
            && let Some(kind) = ur_part(c, &mut ur, tok)
        {
            return Some(kind);
        }
        let trimmed = tok.trim_matches(|ch: char| !ch.is_alphanumeric());
        let pieces = tok.split(|ch: char| ",;=()[]{}<>\"'|".contains(ch));
        for part in std::iter::once(tok)
            .chain(std::iter::once(trimmed))
            .chain(pieces)
        {
            if part.len() < 26 && !part.contains("://") && !part.starts_with("ur:") {
                continue;
            }
            if let Some(kind) = scan_read::secret_text_kind(c, part) {
                return Some(kind);
            }
            if part.to_ascii_lowercase().contains("prv")
                && wallet::classify(c, "part.txt", part.as_bytes()).exposure() == Exposure::Secret
            {
                return Some(Kind::Xprv);
            }
        }
    }
    None
}

/// Costs a CBOR read, and what `scan_read::secret_kind` costs for the
/// text of a `ur:bytes`. One `ur:` token fed to the decoder: the secret
/// kind a message it completes is, if it is one.
fn ur_part(c: &Compute, dec: &mut faraday_codec::ur::Decoder, tok: &str) -> Option<Kind> {
    use faraday_codec::ur::Message;
    let Ok(crate::qr_view::UrFed::Whole(message)) = crate::ur_feed(dec, tok) else {
        return None;
    };
    match message {
        Message::Other { ur_type, cbor } => match faraday_qr::registry::read(&ur_type, &cbor) {
            Err(faraday_qr::registry::Error::Private) => Some(Kind::Xprv),
            Err(faraday_qr::registry::Error::Seed) => Some(Kind::UrSeed),
            _ => None,
        },
        Message::Bytes(b) => scan_read::secret_kind(c, &b),
        Message::Psbt(_) => None,
    }
}

/// The words of each BIP-39 list Faraday reads, and the SLIP-39 list,
/// as they are written.
fn lists() -> &'static (Vec<HashSet<&'static str>>, HashSet<&'static str>) {
    static LISTS: OnceLock<(Vec<HashSet<&'static str>>, HashSet<&'static str>)> = OnceLock::new();
    LISTS.get_or_init(|| {
        let bip39 = Language::ALL
            .iter()
            .map(|l| l.words().iter().copied().collect())
            .collect();
        let slip39 = (0..faraday_bip::slip39::WORDLIST_LEN as u16)
            .map(faraday_bip::slip39::word)
            .collect();
        (bip39, slip39)
    })
}

/// Costs what `scan_read::secret_text_kind` costs for a run found. Twelve
/// or more words in a row of one BIP-39 list, checksum or not, or twenty
/// or more of the SLIP-39 list: most of a seed is a seed. Numbers and
/// punctuation between the words (a numbered list) do not break a run.
fn words(c: &Compute, t: &str) -> Option<Kind> {
    let (bip39, slip39) = lists();
    let mut runs: Vec<Vec<String>> = vec![Vec::new(); bip39.len() + 1];
    for tok in t.split_whitespace() {
        let w = tok.trim_matches(|ch: char| !ch.is_alphabetic());
        if w.is_empty() {
            continue;
        }
        let w = w.to_lowercase();
        for (i, list) in bip39.iter().enumerate() {
            if list.contains(w.as_str()) {
                runs[i].push(w.clone());
            } else {
                runs[i].clear();
            }
        }
        let last = bip39.len();
        if slip39.contains(w.as_str()) {
            runs[last].push(w);
        } else {
            runs[last].clear();
        }
        if runs[last].len() >= faraday_bip::slip39::MIN_WORDS {
            return Some(Kind::Slip39Share);
        }
        if let Some(run) = runs[..last].iter().find(|r| r.len() >= 12) {
            return Some(run_kind(c, run));
        }
    }
    None
}

/// Costs what `scan_read::secret_text_kind` costs, up to ten times. The
/// name of a run of 12 or more words: what the classifier says of its
/// first and its last 12, 15, 18, 21 or 24 words, else words that fail
/// their checksum.
fn run_kind(c: &Compute, run: &[String]) -> Kind {
    for n in [12usize, 15, 18, 21, 24] {
        if n > run.len() {
            break;
        }
        for at in [0, run.len() - n] {
            let joined = Zeroizing::new(run[at..at + n].join(" "));
            if let Some(kind) = scan_read::secret_text_kind(c, &joined) {
                return kind;
            }
        }
    }
    Kind::BadChecksum
}
