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
use std::collections::{HashMap, HashSet};
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
    /// Part of a code that can hold a secret, and the rest is missing.
    Partial,
    /// Bare hex of the length of BIP-39 entropy or a BIP-32 seed.
    SeedHex,
}

/// The Send text card's field, and what its check said.
pub struct SendText {
    text: Zeroizing<String>,
    check: TextCheck,
    /// A paste was longer than the field holds, so it is cut: refused
    /// until the field is cleared.
    over: bool,
}

/// Prints the length of the text, never the text.
impl std::fmt::Debug for SendText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(SendText))
            .field("len", &self.text.len())
            .field("check", &self.check)
            .field("over", &self.over)
            .finish()
    }
}

impl Default for SendText {
    fn default() -> Self {
        Self {
            text: Zeroizing::new(String::new()),
            check: TextCheck::Fine,
            over: false,
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
        self.over = false;
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
        self.over = cut < text.len();
        self.recheck(c);
    }

    /// Costs what [`check`] costs. The check made again, on the text now
    /// in the field.
    pub(crate) fn recheck(&mut self, c: &Compute) {
        self.check = if self.over {
            TextCheck::TooLong
        } else {
            check(c, &self.text)
        };
    }
}

/// Costs a classification of the whole text as a file and, per part of
/// it, what `scan_read::secret_kind` costs: a checksum or a parse for a
/// part that can be a secret, a lookup for each word. What is refused in
/// `text`: the device's classifier on the whole and on every part a secret
/// can hide in, a part of the shape of a key whether or not its checksum
/// holds, a text over [`MAX_TEXT`]. A 64-digit hex string is not refused:
/// a SHA-256, a transaction id and a block hash are that (F236).
pub fn check(c: &Compute, text: &str) -> TextCheck {
    let t = text.trim();
    if t.is_empty() {
        return TextCheck::Fine;
    }
    if t.len() > MAX_TEXT {
        return TextCheck::TooLong;
    }
    let cleaned = without_invisible(t);
    let t = cleaned.trim();
    if let Some(kind) = scan_read::secret_kind(c, t.as_bytes()) {
        return TextCheck::Secret(kind);
    }
    let file = wallet::classify(c, "text.txt", t.as_bytes());
    if file.exposure() == Exposure::Secret {
        return TextCheck::Secret(file_secret(file));
    }
    inside(c, t).unwrap_or(TextCheck::Fine)
}

/// The text without the characters that show nothing and could split a
/// word the checks read: a zero-width space, joiner or non-joiner, a word
/// joiner, a byte order mark and a soft hyphen.
fn without_invisible(t: &str) -> std::borrow::Cow<'_, str> {
    let hidden = |ch: char| {
        matches!(
            ch,
            '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}' | '\u{00AD}'
        )
    };
    if t.contains(hidden) {
        std::borrow::Cow::Owned(t.replace(hidden, ""))
    } else {
        std::borrow::Cow::Borrowed(t)
    }
}

/// What a file kind that is a secret is called.
fn file_secret(kind: wallet::FileKind) -> Kind {
    use wallet::FileKind as F;
    match kind {
        F::Words => Kind::SeedWords,
        F::SeedPart => Kind::Codex32Share,
        F::Entries => Kind::Entries,
        F::SecretWallet => Kind::Wallet,
        _ => Kind::Wif,
    }
}

/// Costs what [`check`] costs for the parts. What some part of `t` is, if
/// it is a secret.
fn inside(c: &Compute, t: &str) -> Option<TextCheck> {
    armour(t)
        .or_else(|| digits(t))
        .map(TextCheck::Inside)
        .or_else(|| tokens(c, t))
        .or_else(|| words(c, t).map(TextCheck::Inside))
}

/// An armoured private key (PEM, PGP).
fn armour(t: &str) -> Option<Kind> {
    let upper = t.to_ascii_uppercase();
    (upper.contains("-----BEGIN") && upper.contains("PRIVATE KEY")).then_some(Kind::Wif)
}

/// A run of 48 or 96 digits, or of 12 or 24 groups of four: a SeedQR.
fn digits(t: &str) -> Option<Kind> {
    let plain = t
        .split(|ch: char| !ch.is_ascii_digit())
        .any(|run| matches!(run.len(), 48 | 96));
    let mut groups = 0;
    let mut grouped = false;
    for tok in t.split_whitespace() {
        if tok.len() == 4 && tok.bytes().all(|b| b.is_ascii_digit()) {
            groups += 1;
            grouped |= groups >= 12;
        } else {
            groups = 0;
        }
    }
    (plain || grouped).then_some(Kind::SeedQr)
}

/// The signs that join a value to its name or are put round it.
const JOINS: &str = ",;=()[]{}<>\"'|";

/// Whether `part` begins a `ur:` string.
fn is_ur(part: &str) -> bool {
    part.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("ur:"))
}

/// The types of UR known to be public: a PSBT. Parts of any other type
/// are not shown until the message is whole and read.
fn ur_is_public(ur_type: &str) -> bool {
    let t = ur_type.to_ascii_lowercase();
    matches!(t.as_str(), "psbt" | "crypto-psbt")
}

/// The types of UR that hold a private key or a seed's share, and that
/// the registry does not read: one is refused whole.
fn ur_is_secret(ur_type: &str) -> bool {
    let t = ur_type.to_ascii_lowercase();
    let t = t.strip_prefix("crypto-").unwrap_or(&t);
    matches!(t, "eckey" | "sskr")
}

/// Puts `piece` in `parts`, trimmed of the punctuation round it, and each
/// piece of it after a label (cut at `:` and `/`) long enough to be a key,
/// whole and trimmed.
fn add_labelled<'a>(parts: &mut Vec<&'a str>, piece: &'a str) {
    let trim = |s: &'a str| s.trim_matches(|ch: char| !ch.is_alphanumeric());
    parts.push(piece);
    parts.push(trim(piece));
    for sub in piece.split([':', '/']).filter(|s| s.len() >= 26) {
        parts.push(sub);
        parts.push(trim(sub));
    }
    for sub in trim(piece).split([':', '/']).filter(|s| s.len() >= 26) {
        parts.push(sub);
        parts.push(trim(sub));
    }
}

/// Costs what `scan_read::secret_text_kind` costs, for each part of a
/// token long enough to be a key, and a classification for one that
/// holds a private key's letters. Each whitespace-separated token, whole,
/// trimmed of the punctuation round it, cut at the signs that join a
/// value to its name (`,;=()[]{}<>"'|`), and again at `:` and `/` (a
/// label such as `p2wpkh:` or `nostr:`), as a secret of the classifier or
/// a key's shape; and the `ur:` parts put together.
fn tokens(c: &Compute, t: &str) -> Option<TextCheck> {
    let mut ur = faraday_codec::ur::Decoder::new();
    for tok in t.split_whitespace() {
        let trimmed = tok.trim_matches(|ch: char| !ch.is_alphanumeric());
        let mut parts: Vec<&str> = Vec::new();
        add_labelled(&mut parts, tok);
        add_labelled(&mut parts, trimmed);
        for piece in tok.split(|ch: char| JOINS.contains(ch)) {
            add_labelled(&mut parts, piece);
            if let Some(i) = piece.find("://") {
                let start = piece[..i]
                    .rfind(|ch: char| !(ch.is_ascii_alphanumeric() || "+-.".contains(ch)))
                    .map_or(0, |j| j + 1);
                parts.push(&piece[start..]);
            }
        }
        // The `ur:` strings, at the start of a part or after a sign.
        let mut urs: Vec<&str> = Vec::new();
        for &part in &parts {
            let lower = part.to_ascii_lowercase();
            for (i, _) in lower.match_indices("ur:") {
                let opens = i == 0
                    || !part[..i]
                        .chars()
                        .next_back()
                        .is_some_and(|ch| ch.is_ascii_alphanumeric());
                if opens && !urs.contains(&&part[i..]) {
                    urs.push(&part[i..]);
                }
            }
        }
        for u in urs {
            if let Some(found) = ur_part(c, &mut ur, u) {
                return Some(found);
            }
        }
        let mut seen: Vec<&str> = Vec::new();
        for part in parts {
            if seen.contains(&part) {
                continue;
            }
            seen.push(part);
            if part.len() < 26 && !part.contains("://") && !is_ur(part) {
                continue;
            }
            if seed_hex(part) {
                return Some(TextCheck::SeedHex);
            }
            if let Some(kind) = scan_read::secret_text_kind(c, part).or_else(|| shape(part)) {
                return Some(TextCheck::Inside(kind));
            }
            if part.to_ascii_lowercase().contains("prv")
                && wallet::classify(c, "part.txt", part.as_bytes()).exposure() == Exposure::Secret
            {
                return Some(TextCheck::Inside(Kind::Xprv));
            }
        }
    }
    // A message of a type not known to be public that never came whole.
    ur.ur_type()
        .is_some_and(|ty| !ur_is_public(&ty))
        .then_some(TextCheck::Partial)
}

/// Whether `part` is bare hex of 32, 40, 48, 56 or 128 digits (BIP-39
/// entropy, a BIP-32 seed), with or without `0x`. 64 digits (a hash) is not.
fn seed_hex(part: &str) -> bool {
    let bare = part
        .strip_prefix("0x")
        .or_else(|| part.strip_prefix("0X"))
        .unwrap_or(part);
    matches!(bare.len(), 32 | 40 | 48 | 56 | 128) && bare.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A string of the shape of a secret, whether or not its checksum holds
/// (a typo does not make a key safe to show): a WIF's length and first
/// character in base58, and a codex32 string's characters and length.
fn shape(part: &str) -> Option<Kind> {
    const BASE58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let first = part.chars().next()?;
    if ((part.len() == 51 && matches!(first, '5' | '9'))
        || (part.len() == 52 && matches!(first, 'K' | 'L' | 'c')))
        && part.chars().all(|ch| BASE58.contains(ch))
    {
        return Some(Kind::Wif);
    }
    const BECH32: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    if (48..=127).contains(&part.len())
        && part.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("ms1"))
        && part[3..]
            .chars()
            .all(|ch| BECH32.contains(ch.to_ascii_lowercase()))
    {
        let index = part.as_bytes()[8].to_ascii_lowercase();
        return Some(if index == b's' {
            Kind::Codex32Secret
        } else {
            Kind::Codex32Share
        });
    }
    None
}

/// Costs a CBOR read, and what [`check`] costs for the text of a
/// `ur:bytes`. One `ur:` string fed to the decoder: what it is if it is a
/// secret, or a part of a message of a type that can hold one and is not
/// whole yet (a part of the other message the decoder was reading, when it
/// is dropped for this one).
fn ur_part(c: &Compute, dec: &mut faraday_codec::ur::Decoder, tok: &str) -> Option<TextCheck> {
    use faraday_codec::ur::Message;
    let before = dec.ur_type();
    let Ok(fed) = crate::ur_feed(dec, tok) else {
        return None;
    };
    let message = match fed {
        crate::qr_view::UrFed::Whole(message) => message,
        crate::qr_view::UrFed::More { restarted } => {
            return (restarted && before.is_some_and(|ty| !ur_is_public(&ty)))
                .then_some(TextCheck::Partial);
        }
    };
    match message {
        Message::Other { ur_type, .. } if ur_is_secret(&ur_type) => Some(TextCheck::Partial),
        Message::Other { ur_type, cbor } => match faraday_qr::registry::read(&ur_type, &cbor) {
            Err(faraday_qr::registry::Error::Private) => Some(TextCheck::Inside(Kind::Xprv)),
            Err(faraday_qr::registry::Error::Seed) => Some(TextCheck::Inside(Kind::UrSeed)),
            _ => None,
        },
        Message::Bytes(b) => match check(c, &String::from_utf8_lossy(&b)) {
            TextCheck::Secret(k) | TextCheck::Inside(k) => Some(TextCheck::Inside(k)),
            r @ (TextCheck::Partial | TextCheck::SeedHex) => Some(r),
            TextCheck::Fine | TextCheck::TooLong => None,
        },
        Message::Psbt(_) => None,
    }
}

/// The words of each BIP-39 list Faraday reads, in NFKD and without their
/// accents where a Latin list has them, and the SLIP-39 list.
fn lists() -> &'static (Vec<HashSet<String>>, HashSet<&'static str>) {
    static LISTS: OnceLock<(Vec<HashSet<String>>, HashSet<&'static str>)> = OnceLock::new();
    LISTS.get_or_init(|| {
        let bip39 = Language::ALL
            .iter()
            .map(|l| {
                let mut set: HashSet<String> = HashSet::new();
                for w in l.words_nfkd() {
                    set.extend(faraday_bip::bip39::fold(w).map(|f| f.as_chars().iter().collect()));
                    set.insert((*w).to_owned());
                }
                set
            })
            .collect();
        let slip39 = (0..faraday_bip::slip39::WORDLIST_LEN as u16)
            .map(faraday_bip::slip39::word)
            .collect();
        (bip39, slip39)
    })
}

/// The English BIP-39 words by their first four letters (a word of three
/// letters by itself), which identify each: steel plates carry seeds so.
fn prefixes() -> &'static HashMap<&'static str, &'static str> {
    static PREFIXES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    PREFIXES.get_or_init(|| {
        let mut map = HashMap::new();
        for w in Language::English.words_nfkd() {
            map.entry(w.get(..4).unwrap_or(w)).or_insert(*w);
        }
        map
    })
}

/// Whether a character is a combining mark, which stays in a word.
fn is_mark(ch: char) -> bool {
    matches!(
        ch as u32,
        0x300..=0x36F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0x3099..=0x309A
            | 0xFE20..=0xFE2F
    )
}

/// Costs what `scan_read::secret_text_kind` costs for a run found. Eleven
/// or more words in a row of one BIP-39 list, checksum or not, or nineteen
/// or more of the SLIP-39 list: a seed one word short is most of a seed.
/// Also twelve words in a row of which eleven are on one BIP-39 list, or
/// twenty of which nineteen are on the SLIP-39 list: one misspelt or
/// off-list word anywhere in a seed. And twelve or more words in a row that
/// are each the first four letters of an English word (a three-letter word
/// as it is), the form of a seed on a steel plate.
/// Words are cut at every character that is no letter and no combining
/// mark, so a name or a quote glued to a word (`MNEMONIC="abandon`) does
/// not hide it; numbers and punctuation between the words (a numbered
/// list) do not break a run.
fn words(c: &Compute, t: &str) -> Option<Kind> {
    let (bip39, slip39) = lists();
    // As the lists are stored: decomposed, in lower case.
    let ws: Vec<String> = t
        .split_whitespace()
        .flat_map(|tok| tok.split(|ch: char| !(ch.is_alphabetic() || is_mark(ch))))
        .filter(|w| !w.is_empty())
        .map(|w| faraday_bip::bip39::nfkd_lower(w).collect())
        .collect();
    let slip = faraday_bip::slip39::MIN_WORDS;
    if longest_run(&ws, |w| slip39.contains(w)) >= slip - 1
        || dense_window(&ws, slip, slip - 1, |w| slip39.contains(w)).is_some()
    {
        return Some(Kind::Slip39Share);
    }
    // Twelve or more first-four-letter forms of English words in a row.
    let short = prefixes();
    let (mut start, mut best) = (0, (0, 0));
    for end in 0..=ws.len() {
        if end < ws.len() && short.contains_key(ws[end].as_str()) {
            continue;
        }
        if end - start > best.1 - best.0 {
            best = (start, end);
        }
        start = end + 1;
    }
    if best.1 - best.0 >= 12 {
        let full: Vec<String> = ws[best.0..best.1]
            .iter()
            .map(|w| (*short.get(w.as_str()).unwrap_or(&"")).to_string())
            .collect();
        return Some(run_kind(c, &full));
    }
    // A word on a list as it is, or without its accents.
    let on = |list: &HashSet<String>, w: &str| {
        list.contains(w)
            || faraday_bip::bip39::fold(w)
                .is_some_and(|f| list.contains(&f.as_chars().iter().collect::<String>()))
    };
    for list in bip39 {
        let mut start = 0;
        for end in 0..=ws.len() {
            if end < ws.len() && on(list, &ws[end]) {
                continue;
            }
            if end - start >= 11 {
                return Some(run_kind(c, &ws[start..end]));
            }
            start = end + 1;
        }
    }
    // A misspelt or off-list word anywhere in a seed's worth of words.
    for list in bip39 {
        if let Some(at) = dense_window(&ws, 12, 11, |w| on(list, w)) {
            return Some(run_kind(c, &ws[at..at + 12]));
        }
    }
    None
}

/// The start of the first window of `size` consecutive words of which at
/// least `need` are on the list: a seed with one word misspelt.
fn dense_window(
    ws: &[String],
    size: usize,
    need: usize,
    on_list: impl Fn(&str) -> bool,
) -> Option<usize> {
    let hits: Vec<bool> = ws.iter().map(|w| on_list(w)).collect();
    let mut count = 0;
    for (i, &hit) in hits.iter().enumerate() {
        count += usize::from(hit);
        if i >= size {
            count -= usize::from(hits[i - size]);
        }
        if i + 1 >= size && count >= need {
            return Some(i + 1 - size);
        }
    }
    None
}

/// The length of the longest run of words `on_list` holds.
fn longest_run(ws: &[String], on_list: impl Fn(&str) -> bool) -> usize {
    let (mut best, mut run) = (0, 0);
    for w in ws {
        run = if on_list(w) { run + 1 } else { 0 };
        best = best.max(run);
    }
    best
}

/// Costs what `scan_read::secret_text_kind` costs, up to ten times. The
/// name of a run of 11 or more words: what the classifier says of its
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
