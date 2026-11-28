//! File classification: what a held file is (`main`'s `wallet::classify`), as far as who may read
//! it is concerned. A file that holds a secret is named so wherever the secret sits: a seed's
//! words in any list, an extended private key, a share, an authenticator export, beside other
//! text, a comment or a JSON member the readers skip.

use faraday_bip::bip39::Language;
use faraday_crypto::work::Compute;
use faraday_kernel::session::FileKind;
use faraday_text::Text;

use crate::key_file::{read_key, read_keys};
use crate::share::is_share;
use crate::words::{other_list_entries, other_list_index, typed_index, typed_mnemonic};

/// The most a plain text file may hold to be kept as a note.
pub const TEXT_MAX: usize = 64 * 1024;

/// Plain text a person wrote or saved: printable, lines, and short
/// enough for a note.
fn is_text(text: &str) -> bool {
    !text.trim().is_empty()
        && text.len() <= TEXT_MAX
        && text
            .chars()
            .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
}

/// The length in bytes under which a piece of text is no key, seed or
/// share, unless it starts with an extended private key's version:
/// BIP-392's `spspend1` marker, 8 bytes, is the shortest of the rest, while
/// `secret_text` names a text by its 4-byte private version alone.
const MIN_SECRET_LEN: usize = 8;

/// Whether `text` is too short for any detector, which is not so for one
/// that starts with a private version.
fn too_short_to_be_secret(text: &str) -> bool {
    text.len() < MIN_SECRET_LEN && !faraday_intake::starts_with_private_version(text)
}

/// Costs what `secret_text` costs. Whether a piece of text, a user ID
/// or an armour header, is a seed's words, an extended private key or a
/// share: the detectors that read a plain text file.
fn text_may_be_secret(c: &Compute, text: &str) -> bool {
    // Apart from a private version's 4 bytes, the shortest secret is a
    // silent payments spend key's `spspend1`, 8 bytes; a shorter piece is
    // none, and a file of short lines is not read line by line at the
    // cost of each.
    if too_short_to_be_secret(text.trim()) {
        return false;
    }
    faraday_intake::secret_text(c, text).is_some()
        || seed_in_words(c, &faraday_intake::file_words(text))
        || is_share(text)
        || faraday_intake::part_of(text).is_some()
}

/// Costs what `text_may_be_secret` costs on the body and on each of its
/// lines, and a `words_across` over them. The kind a signed message's
/// file takes when its body holds a secret: a seed's words, a seed part
/// or entries as in a plain text file, and a secret wallet file for a
/// private key, an Electrum file with a seed, or a seed, a seed part or an
/// authenticator's export beside other text, a `#` line among them. None
/// for any other body, a multisig share among them, which holds public
/// keys and stays a message.
fn secret_body_kind(c: &Compute, body: &str) -> Option<FileKind> {
    if too_short_to_be_secret(body.trim()) {
        return None;
    }
    if faraday_intake::part_of(body).is_some() {
        return Some(FileKind::SeedPart);
    }
    if seed_in_words(c, &faraday_intake::file_words(body)) {
        return Some(FileKind::Words);
    }
    if faraday_codec::classify::looks_like_entries(body)
        || faraday_intake::is_otpauth_migration(body)
    {
        return Some(FileKind::Entries);
    }
    (faraday_intake::secret_text(c, body).is_some()
        || faraday_intake::wallet::holds_secret(body)
        || lines_hold_secret(c, &body.lines().collect::<Vec<_>>()))
    .then_some(FileKind::SecretWallet)
}

/// A line without its comment marks, however many.
fn bare(l: &str) -> &str {
    l.trim()
        .trim_start_matches(|ch: char| ch == '#' || ch.is_whitespace())
}

/// The pieces of a line a secret can sit in beside other text: each
/// white-space separated word, cut at quotes, commas and brackets, and
/// what follows a label's colon. At most 64.
fn fragments(l: &str) -> impl Iterator<Item = &str> {
    l.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '"' | '\''
                    | '`'
                    | '\u{201c}'
                    | '\u{201d}'
                    | ','
                    | ';'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '<'
                    | '>'
            )
    })
    .filter(|f| !f.is_empty())
    .flat_map(|f| {
        std::iter::once(f).chain(
            f.split_once(':')
                .map(|(_, r)| r)
                .filter(|r| !r.is_empty() && !r.starts_with("//")),
        )
    })
    .take(64)
}

/// Costs `secret_text` on the line and on each fragment of it, and a
/// part's parse of each. Whether one line is a secret by itself, as the
/// whole line or after a label, in quotes or beside prose.
fn line_is_secret(c: &Compute, l: &str) -> bool {
    let is = |t: &str| {
        !too_short_to_be_secret(t)
            && (faraday_intake::secret_text(c, t).is_some()
                || faraday_intake::is_otpauth_migration(t)
                || faraday_intake::part_of(t).is_some())
    };
    is(bare(l)) || fragments(bare(l)).any(is)
}

/// Costs a `line_is_secret` on each line, a seed part's parse of each
/// two or three lines joined, and a `words_across` over the lines.
/// Whether `lines` hold a secret: a line that is one or has one in it, a
/// SLIP-39 share wrapped over lines, or a seed's words among other text,
/// punctuation read as a space.
fn lines_hold_secret(c: &Compute, lines: &[&str]) -> bool {
    if lines.iter().any(|l| line_is_secret(c, l)) {
        return true;
    }
    let bares: Vec<&str> = lines
        .iter()
        .map(|l| bare(l))
        .filter(|l| !l.is_empty())
        .collect();
    for n in [2, 3] {
        for w in bares.windows(n) {
            let joined = Text::joined(w, " ");
            if faraday_intake::part_of(joined.as_str()).is_some() {
                return true;
            }
        }
    }
    if slip39_across(bares.iter().copied()) {
        return true;
    }
    let spaced: Vec<Text> = lines
        .iter()
        .map(|l| {
            let mut t = Text::with_room(l.len());
            for ch in bare(l).chars() {
                t.push(if ch.is_ascii_punctuation() { ' ' } else { ch });
            }
            t
        })
        .collect();
    words_across(spaced.iter().map(Text::as_str))
}

/// Costs `slip39::index_of` for each word of 4 to 8 letters, and a
/// checksum for each of the 14 lengths at every word of a run (a run past
/// 48 is dropped). Whether the words among `parts` hold a SLIP-39 share:
/// a run of consecutive list words in any case, broken by any other
/// token (a place number does not break it), whose last 20 to 33 are a
/// share, however many lines it is over.
fn slip39_across<'a>(parts: impl Iterator<Item = &'a str>) -> bool {
    use faraday_bip::slip39;
    let mut run = faraday_crypto::Zeroizing::new(Vec::<u16>::with_capacity(slip39::MAX_WORDS));
    let mut seen = 0usize;
    for part in parts {
        let words = faraday_intake::file_words(part);
        for w in words.as_str().split_whitespace() {
            let mut low = [0u8; 8];
            let index = if (4..=8).contains(&w.len()) && w.bytes().all(|b| b.is_ascii_alphabetic())
            {
                low[..w.len()].copy_from_slice(w.as_bytes());
                low.make_ascii_lowercase();
                std::str::from_utf8(&low[..w.len()])
                    .ok()
                    .and_then(slip39::index_of)
            } else {
                None
            };
            let Some(i) = index else {
                run.clear();
                seen = 0;
                continue;
            };
            seen += 1;
            if seen > 48 {
                continue;
            }
            if run.len() == slip39::MAX_WORDS {
                run.remove(0);
            }
            run.push(i);
            for n in slip39::MIN_WORDS..=slip39::MAX_WORDS {
                if run.len() >= n && slip39::Share::from_indices(&run[run.len() - n..]).is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

/// The lines of a multisig share that `restore::parts` does not read as a
/// quorum, a path or a key: comments, a name, a format and any other key.
fn share_skipped_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| {
            let Some((k, _)) = l.split_once(':') else {
                return true;
            };
            let k = k.trim();
            l.starts_with('#')
                || !(k.eq_ignore_ascii_case("policy")
                    || k.eq_ignore_ascii_case("derivation")
                    || (k.len() == 8 && k.chars().all(|ch| ch.is_ascii_hexdigit())))
        })
        .collect()
}

/// Costs a `read_keys` on the line: base58 decodes. Whether a line of a
/// key file is a key `read_keys` reads.
fn is_key_line(line: &str) -> bool {
    !too_short_to_be_secret(line) && !read_keys(line).is_empty()
}

/// Costs a pass over the words of every part, with a checksum for each
/// of five lengths at every list word of a run (a run past 48 is
/// dropped, and a token that cannot be a list word is not looked up).
/// Whether the words among `parts` hold a seed: a run of consecutive list
/// words, in the order they come, broken by any other token (a place
/// number does not break it), of which some 12, 15, 18, 21 or 24 in a row
/// are a mnemonic. A seed spread over lines, numbered or in a JSON array
/// is found with other text around it, headings and labels that are list
/// words too. Each part is read as a file's words are (place numbers
/// dropped).
fn words_across<'a>(parts: impl Iterator<Item = &'a str>) -> bool {
    // The words of every part, each part read as a file's words are,
    // held in wiping room.
    let parts: Vec<Text> = parts.map(faraday_intake::file_words).collect();
    // One pass counts, for each list, the words that are on it; a list
    // with fewer than 12 of them cannot hold a seed and is not walked.
    let mut on_list = vec![0usize; Language::ALL.len()];
    for part in &parts {
        for w in part.as_str().split_whitespace() {
            for e in other_list_entries(w).iter() {
                on_list[e.0 as usize] += 1;
            }
        }
    }
    Language::ALL.iter().enumerate().any(|(at, &lang)| {
        (lang == Language::English || on_list[at] >= 12) && words_across_in(lang, &parts)
    })
}

/// Costs what `words_across` costs for one list. Whether the words in
/// `parts` hold a seed written in `lang`'s list.
fn words_across_in(lang: Language, parts: &[Text]) -> bool {
    // The word numbers of the run: a wiping buffer sized once.
    let mut run = faraday_crypto::Zeroizing::new(Vec::<u16>::with_capacity(25));
    let mut overlong = false;
    let mut seen = 0usize;
    for part in parts {
        for w in part.as_str().split_whitespace() {
            let index = if lang == Language::English {
                // The list's words are 3 to 8 ASCII letters, read in any
                // case as a mnemonic is, folded in a buffer of the word's
                // own size.
                let mut low = [0u8; 8];
                if (3..=8).contains(&w.len()) && w.bytes().all(|b| b.is_ascii_alphabetic()) {
                    low[..w.len()].copy_from_slice(w.as_bytes());
                    low.make_ascii_lowercase();
                    std::str::from_utf8(&low[..w.len()])
                        .ok()
                        .and_then(|l| typed_index(Language::English, l))
                } else {
                    None
                }
            } else {
                other_list_index(lang, w)
            };
            let Some(i) = index else {
                run.clear();
                overlong = false;
                seen = 0;
                continue;
            };
            if overlong {
                continue;
            }
            if run.len() == 24 {
                // Slide: the window of the last 24 words is the longest
                // that is checked.
                run.remove(0);
            }
            seen += 1;
            if seen > 48 {
                // Past 48 words of one run, no more are checked.
                overlong = true;
                continue;
            }
            run.push(i);
            for n in [12, 15, 18, 21, 24] {
                if run.len() >= n
                    && faraday_keys::words::from_indices(lang, &run[run.len() - n..]).is_ok()
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Costs a parse of the words in each list that holds half of them. Whether `words` (as
/// [`faraday_intake::file_words`] gives them) are a seed in any list the
/// device imports: English as typed, any other as a sentence is parsed.
fn seed_in_words(c: &Compute, words: &str) -> bool {
    seed_mnemonic(c, words).is_some()
}

/// Costs a parse of the words in each list that holds half of them. The
/// seed `words` hold (as [`faraday_intake::file_words`] gives them) and
/// is read in, found as [`seed_in_words`] finds it.
pub(crate) fn seed_mnemonic(c: &Compute, words: &str) -> Option<faraday_bip::bip39::Mnemonic> {
    if let Some(m) = typed_mnemonic(Language::English, words) {
        return Some(m);
    }
    let n = words.split_whitespace().count();
    if !matches!(n, 12 | 15 | 18 | 21 | 24) {
        return None;
    }
    // One lookup per word counts it for every list; only a list that holds
    // at least half of the words is parsed.
    let mut on_list = vec![0usize; Language::ALL.len()];
    for w in words.split_whitespace() {
        for e in other_list_entries(w).iter() {
            on_list[e.0 as usize] += 1;
        }
    }
    Language::ALL
        .iter()
        .enumerate()
        .filter(|&(at, &l)| l != Language::English && on_list[at] * 2 >= n)
        .find_map(|(_, &l)| faraday_keys::words::parse(c, l, words).ok())
}

/// Whether `bytes` are a PSBT `main`'s `read_psbt` reads: binary or base64, version 0 or version
/// 2, without a silent payment field (BIP-375), a version 2 one that reads as version 0. One
/// that is refused falls through to a text or other file, as on `main`.
fn is_psbt(c: &Compute, bytes: &[u8]) -> bool {
    use faraday_psbt::Psbt;
    use faraday_psbt::v2::PsbtV2;
    let text = std::str::from_utf8(bytes).ok().map(str::trim);
    if let Some(p) = Psbt::parse_bytes(c, bytes)
        .ok()
        .or_else(|| Psbt::parse_base64(c, text?).ok())
    {
        return !v0_has_silent_fields(&p);
    }
    PsbtV2::parse_bytes(c, bytes)
        .ok()
        .or_else(|| PsbtV2::parse_base64(c, text?).ok())
        .is_some_and(|v2| !v2.has_silent_fields() && v2.to_v0(c).is_ok())
}

/// Whether a version 0 PSBT carries a field type of BIP-375 (global 0x07 or 0x08, input 0x1d or
/// 0x1e, output 0x09 or 0x0a) among its unknown fields. BIP-375 allows them in version 2 only, and
/// a version 0 file that has them is a silent payment's, to be refused as one (`main`'s
/// `v0_has_silent_fields`).
fn v0_has_silent_fields(p: &faraday_psbt::Psbt) -> bool {
    use faraday_psbt::v2::{
        GLOBAL_SP_DLEQ, GLOBAL_SP_ECDH_SHARE, IN_SP_DLEQ, IN_SP_ECDH_SHARE, OUT_SP_V0_INFO,
        OUT_SP_V0_LABEL,
    };
    let inner = p.inner();
    let has = |types: Vec<u8>, wanted: [u8; 2]| types.iter().any(|t| wanted.contains(t));
    has(
        inner.unknown.keys().map(|k| k.type_value).collect(),
        [GLOBAL_SP_ECDH_SHARE, GLOBAL_SP_DLEQ],
    ) || inner.inputs.iter().any(|i| {
        has(
            i.unknown.keys().map(|k| k.type_value).collect(),
            [IN_SP_ECDH_SHARE, IN_SP_DLEQ],
        )
    }) || inner.outputs.iter().any(|o| {
        has(
            o.unknown.keys().map(|k| k.type_value).collect(),
            [OUT_SP_V0_INFO, OUT_SP_V0_LABEL],
        )
    })
}

/// The text as a signed message, when its first line is an address: the one reading of "this is
/// a signed message". An address is at least 14 characters and has no white space; a shorter
/// first line (a word of a seed written one to a line) is not handed to bitcoin's address
/// parser, which would decode it into a buffer of its own.
fn signed_message(text: &str) -> Option<faraday_psbt::message::SignedText> {
    faraday_psbt::message::parse_signed(text).filter(|m| {
        m.address.len() >= 14
            && !m.address.contains(char::is_whitespace)
            && m.address
                .parse::<faraday_bip::bitcoin::Address<_>>()
                .is_ok()
    })
}

/// Whether an Inbox file is an EFI image, by its name.
fn is_image(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".efi")
}

/// Costs a parse of a PSBT, and for text a few passes over it and a check of each line, each
/// linear in the file's length (a file of many short lines or of one line of many short tokens
/// costs by its length). Says what a file holds.
pub fn classify(c: &Compute, name: &str, bytes: &[u8]) -> FileKind {
    if faraday_codec::vault_file::is_vault_file(bytes) {
        return FileKind::Vault;
    }
    if bytes.starts_with(b"%PDF-") {
        return FileKind::Pdf;
    }
    if bytes.starts_with(b"SQLite format 3\0") {
        return FileKind::CoreWallet;
    }
    if faraday_intake::is_older_kdbx(bytes) {
        return FileKind::Kdbx;
    }
    if faraday_intake::is_kdbx(bytes) {
        return FileKind::KdbxOther;
    }
    if is_image(name) && bytes.starts_with(b"MZ") {
        return FileKind::EfiImage;
    }
    if is_psbt(c, bytes) {
        return FileKind::Psbt;
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        if text.starts_with("faraday-sheet 1") {
            return FileKind::Sheet;
        }
        if let Some(block) = faraday_pgp::read_public(bytes)
            && !block.texts.iter().any(|t| text_may_be_secret(c, t))
            && !words_across(block.texts.iter().map(Text::as_str))
        {
            return FileKind::Gpg;
        }
        if let Some(m) = signed_message(text) {
            // The body is checked for a secret only here, so a text file that is no signed
            // message does not pay for it.
            return secret_body_kind(c, m.message.as_str()).unwrap_or(FileKind::Message);
        }
        if is_share(text) {
            // A share's reader skips comments and lines it does not know: a seed there makes
            // the file a secret one.
            if lines_hold_secret(c, &share_skipped_lines(text)) {
                return FileKind::SecretWallet;
            }
            return FileKind::Share;
        }
        if faraday_intake::wallet::holds_secret(text) {
            return FileKind::SecretWallet;
        }
        if faraday_intake::wallet::read_wallet(c, text).is_ok() {
            // Secret text beside a descriptor, in a JSON member the reader skips or a comment,
            // makes the wallet file a secret one. A file that is not a wallet keeps the kind
            // it has below.
            let unread = faraday_intake::wallet::unread_texts(text);
            if unread.iter().any(|t| text_may_be_secret(c, t.as_str()))
                || words_across(unread.iter().map(Text::as_str))
            {
                return FileKind::SecretWallet;
            }
            return FileKind::Wallet;
        }
        if read_key(text).is_some() {
            // `read_keys` drops every line it cannot read: a seed beside the key, on a line or
            // in a JSON member, makes a secret file.
            let unread = faraday_intake::wallet::unread_texts(text);
            let skipped = text.lines().map(str::trim).filter(|l| !is_key_line(l));
            if unread
                .iter()
                .map(Text::as_str)
                .chain(text.lines())
                .any(|t| text_may_be_secret(c, t.trim().trim_start_matches('#').trim()))
                || words_across(skipped)
                || words_across(unread.iter().map(Text::as_str))
            {
                return FileKind::KeyWithSecret;
            }
            return FileKind::Key;
        }
        if seed_in_words(c, &faraday_intake::file_words(text)) {
            return FileKind::Words;
        }
        // Google Authenticator's export holds every TOTP secret of the account: kept as entries
        // are, never filed as text.
        if faraday_codec::classify::looks_like_entries(text)
            || faraday_intake::is_otpauth_migration(text)
        {
            return FileKind::Entries;
        }
        if faraday_intake::part_of(text).is_some() {
            return FileKind::SeedPart;
        }
        // A raw transaction in hex, under any name: what a wallet or an explorer gives to
        // broadcast.
        let t = text.trim();
        if !t.is_empty()
            && t.bytes().all(|b| b.is_ascii_hexdigit())
            && faraday_psbt::transaction::read_raw(t.as_bytes()).is_some()
        {
            return FileKind::Transaction;
        }
        if line_form(c, text).is_some() {
            return FileKind::KeyLine;
        }
        if is_text(text) {
            return FileKind::Text;
        }
    }
    // The weakest test there is (an envelope has no magic number), so the last: a `.kef` file of
    // the bytes the Krux tool writes, never a text form and never another name (a renamed code
    // stays `Other`, which From Files still lists).
    if name
        .len()
        .checked_sub(4)
        .and_then(|i| name.get(i..))
        .is_some_and(|ext| ext.eq_ignore_ascii_case(".kef"))
        && faraday_codec::kef::recognize(bytes)
            .is_some_and(|r| r.form == faraday_codec::kef::Form::Raw)
    {
        return FileKind::KruxCode;
    }
    FileKind::Other
}

/// Whether a file's whole text is one key of the two one-line forms: an extended private key (of
/// any version) or a master seed in hex of a BIP-32 length. A hex file may be a hash, which is
/// why a listed file loads only when it is opened.
pub(crate) fn line_form(c: &Compute, text: &str) -> Option<LineForm> {
    let t = text.trim();
    if t.is_empty() || t.split_whitespace().nth(1).is_some() {
        return None;
    }
    if matches!(
        faraday_intake::secret_text(c, t),
        Some(faraday_intake::TextSecret::ExtendedPrivateKey)
    ) {
        return Some(LineForm::Xprv);
    }
    faraday_keys::xprv::parse_hex_seed(t)
        .is_ok()
        .then_some(LineForm::HexSeed)
}

/// The two forms read from one line of text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LineForm {
    Xprv,
    HexSeed,
}
