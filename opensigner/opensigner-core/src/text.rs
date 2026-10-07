//! Display text helpers: fingerprints, network names, amounts and hex as
//! the screens show them, and accent composition for candidate words.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::bip39::{Language, Script};
use osk_bip::keys::{DerivationPath, Fingerprint, Network, ScriptType};
use osk_bip::spend::{Lock, SpendPath};
use osk_ui::components::secrets::WordWidth;

use crate::load::EntryList;
use crate::strings::Strings;

/// The account path a purpose preset of the path editor writes, as its
/// row's value: `m/44h/0h/0h` for Legacy on mainnet (`docs/DESIGN.md`
/// §4.6). The coin type follows the network, so the same row means the
/// same thing on regtest.
pub fn preset_path(script: ScriptType, network: Network) -> String {
    alloc::format!(
        "m/{}h/{}h/{}h",
        script.purpose(),
        network.coin_type(),
        crate::explore::PRESET_ACCOUNT
    )
}

/// A whole number with its thousands in groups, `52 560`, joined by the
/// separator amounts use, which the text face carries and which never
/// breaks a number across two lines.
pub fn thousands(n: u32) -> String {
    let digits = alloc::format!("{n}");
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(osk_ui::tokens::GROUP_SEPARATOR);
        }
        out.push(c);
    }
    out
}

/// A long string in the groups of four every long string is read in
/// (`docs/DESIGN.md` §4.5), separated by a space: the codex32 string in
/// the entry field, which a person copies character by character.
pub fn grouped(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / osk_ui::tokens::CHUNK_GROUP);
    for (i, c) in text.chars().enumerate() {
        if i > 0 && i.is_multiple_of(osk_ui::tokens::CHUNK_GROUP) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The fingerprint as eight hex characters.
pub fn fingerprint_hex(fp: Fingerprint) -> String {
    String::from(core::str::from_utf8(&fp.to_hex()).expect("ascii hex"))
}

/// The chain as the design system's badge names it (`docs/DESIGN.md`
/// §4.8): a caution pill off mainnet, nothing on it.
pub fn network(n: Network) -> osk_ui::components::Network {
    match n {
        Network::Mainnet => osk_ui::components::Network::Mainnet,
        Network::Testnet => osk_ui::components::Network::Testnet,
        Network::Signet => osk_ui::components::Network::Signet,
        Network::Regtest => osk_ui::components::Network::Regtest,
    }
}

/// Human name of a wordlist.
pub fn language_name(lang: Language) -> &'static str {
    match lang {
        Language::English => "English",
        Language::Japanese => "Japanese",
        Language::Korean => "Korean",
        Language::Spanish => "Spanish",
        Language::ChineseSimplified => "Chinese (Simplified)",
        Language::ChineseTraditional => "Chinese (Traditional)",
        Language::French => "French",
        Language::Italian => "Italian",
        Language::Czech => "Czech",
        Language::Portuguese => "Portuguese",
    }
}

/// Short segment label for a script type.
pub fn script_short(script: ScriptType, s: &Strings) -> &'static str {
    match script {
        ScriptType::NativeSegwit => s.script_segwit,
        ScriptType::Taproot => s.script_taproot,
        ScriptType::NestedSegwit => s.script_nested,
        ScriptType::Legacy => s.script_legacy,
    }
}

/// The script an input spends, in the one vocabulary §4.6 allows:
/// SegWit, Taproot, Nested, Legacy — never `p2wpkh` or `BIP-84`.
pub fn script_kind(kind: osk_psbt::ScriptKind, s: &Strings) -> String {
    use osk_psbt::ScriptKind;
    match kind {
        ScriptKind::P2pkh => String::from(s.script_legacy),
        ScriptKind::P2shP2wpkh => String::from(s.script_nested),
        ScriptKind::P2wpkh => String::from(s.script_segwit),
        ScriptKind::P2trKey => String::from(s.script_taproot),
        ScriptKind::P2trScript => String::from(s.script_taproot_path),
        ScriptKind::Miniscript { .. } => String::from(s.script_miniscript),
        ScriptKind::Multisig { m, n, .. } => crate::strings::fill(
            s.script_multisig,
            &[&alloc::format!("{m}"), &alloc::format!("{n}")],
        ),
        ScriptKind::Unknown => String::from(s.script_unknown),
    }
}

/// `m/84h/0h/0h/0/0`: a derivation path with `h` for hardened levels;
/// `m` alone for the master.
pub fn path(p: &DerivationPath) -> String {
    let mut out = String::from("m");
    for child in p {
        out.push('/');
        out.push_str(&alloc::format!("{child:#}"));
    }
    out
}

/// `SegWit receive 0`: the script type, the chain and the index, as the
/// Verify result already words it (UX review 2026-09-07, §2.1).
pub fn address_label(script: ScriptType, change: bool, index: u32, s: &Strings) -> String {
    crate::strings::fill(
        s.addresses_index,
        &[
            script_short(script, s),
            if change {
                s.addresses_change_low
            } else {
                s.addresses_receive_low
            },
            &alloc::format!("{index}"),
        ],
    )
}

/// Lower-case hex.
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&alloc::format!("{b:02x}"));
    }
    out
}

/// A wordlist word as a reader sees it: the composed form, which joins
/// the Japanese list's voicing marks and the Korean list's conjoining
/// jamo and precomposes the accented Latin lists. Every screen that
/// shows a whole word goes through here — the candidate strip, the words
/// panel, the Words screen, the quiz and Explore.
pub fn shown_word(list: EntryList, idx: u16) -> String {
    String::from(list.word_display(idx))
}

/// The keys typed so far, composed for reading: kana with their voicing
/// marks joined, jamo run through the two-set automaton, letters as they
/// were typed. This is what the entry field shows.
pub fn shown_prefix(list: EntryList, keys: &[char]) -> String {
    match list.script() {
        Script::Kana => osk_bip::kana::compose(keys.iter().copied())
            .as_chars()
            .iter()
            .collect(),
        Script::Jamo => osk_bip::hangul::compose(keys.iter().copied())
            .as_chars()
            .iter()
            .collect(),
        _ => keys.iter().collect(),
    }
}

/// How wide a word of `list` is in a panel: the characters of its
/// longest word, and one character of its script, which is what the
/// panel's face measures.
pub fn word_width(list: EntryList) -> WordWidth {
    WordWidth {
        chars: list.max_display_chars(),
        sample: list
            .word_display(0)
            .chars()
            .next()
            .unwrap_or(WordWidth::LATIN.sample),
    }
}

/// Letters a letter-punch plate takes from a word.
pub const STEEL_LETTERS: usize = 4;

/// Characters a steel row adds to the word it names: the four-digit
/// wordlist number, the four punched letters and the two separators.
pub const STEEL_EXTRA: usize = 14;

/// The row a numbered steel plate and a letter-punch plate take:
/// `0001 · ABAN · abandon` — the word's place in the wordlist as four
/// digits, its first four letters in capitals, and the word itself
/// (`docs/PLANNING.md` §8.2 item 6).
pub fn steel_row(lang: Language, idx: u16) -> String {
    let word = lang.word_display(idx);
    let mut punched = String::new();
    for c in word.chars().take(STEEL_LETTERS) {
        for upper in c.to_uppercase() {
            punched.push(upper);
        }
    }
    alloc::format!("{:0>4} \u{00b7} {punched} \u{00b7} {word}", idx + 1)
}

/// How wide a steel row of this wordlist is: the word's own width and
/// the fourteen characters the number, the letters and the separators
/// add.
pub fn steel_width(lang: Language) -> WordWidth {
    let word = word_width(EntryList::Bip39(lang));
    WordWidth {
        chars: word.chars + STEEL_EXTRA,
        sample: word.sample,
    }
}

/// `wpkh · 73c5da0a · #qf45pmyh`: the function, the origin fingerprint
/// and the checksum of a descriptor, which are the parts a person checks
/// against the coordinator. The whole descriptor is one tap away.
pub fn descriptor_summary(text: &str) -> String {
    let function = text.split('(').next().unwrap_or("");
    let fingerprint = text
        .split('[')
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("");
    let checksum = text
        .rsplit('#')
        .next()
        .filter(|_| text.contains('#'))
        .unwrap_or("");
    let mut out = String::from(function);
    if !fingerprint.is_empty() {
        out.push_str(" \u{00b7} ");
        out.push_str(fingerprint);
    }
    if !checksum.is_empty() {
        out.push_str(" \u{00b7} #");
        out.push_str(checksum);
    }
    out
}

/// A file's date as the Files screen shows it, `YYYY-MM-DD HH:MM` in
/// UTC, from seconds since the epoch. No time zone: a device someone
/// built has no zone database and no way to know where it is, so the
/// date is the one the card's file system recorded.
pub fn file_time(secs: u64) -> String {
    let (year, month, day) = civil_from_days((secs / 86_400) as i64);
    let rest = secs % 86_400;
    alloc::format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        rest / 3_600,
        (rest % 3_600) / 60
    )
}

/// The letter a spend path names one of the wallet's keys by: A for the
/// first key of the policy, B for the second. Past the twenty-sixth the
/// key is named by its number, which no wallet a person reads reaches.
pub fn key_letter(at: usize) -> String {
    match u8::try_from(at) {
        Ok(n) if n < 26 => String::from(char::from(b'A' + n)),
        _ => alloc::format!("{}", at + 1),
    }
}

/// A date as `YYYY-MM-DD`, from seconds since the epoch: what an
/// absolute locktime above 500 000 000 names.
fn date(secs: u32) -> String {
    let (year, month, day) = civil_from_days(i64::from(secs) / 86_400);
    alloc::format!("{year:04}-{month:02}-{day:02}")
}

/// How long a count of blocks is, at ten minutes a block. An
/// approximation, and said as one.
fn about(minutes: u64, s: &Strings) -> String {
    let days = minutes / 1_440;
    if days < 365 {
        return match days {
            1 => String::from(s.wallet_about_day),
            _ => crate::strings::fill1(s.wallet_about_days, &alloc::format!("{days}")),
        };
    }
    let tenths = days * 10 / 365;
    if tenths == 10 {
        return String::from(s.wallet_about_year);
    }
    let years = if tenths.is_multiple_of(10) {
        alloc::format!("{}", tenths / 10)
    } else {
        alloc::format!("{}.{}", tenths / 10, tenths % 10)
    };
    crate::strings::fill1(s.wallet_about_years, &years)
}

/// One wait, as the review states it.
fn lock(lock: Lock, s: &Strings) -> String {
    match lock {
        Lock::Blocks(n) => crate::strings::fill(
            s.wallet_lock_blocks,
            &[&thousands(n), &about(u64::from(n) * 10, s)],
        ),
        // 512 seconds is 8 minutes 32 seconds; the count is of those.
        Lock::Intervals(n) => crate::strings::fill(
            s.wallet_lock_seconds,
            &[&thousands(n), &about(u64::from(n) * 512 / 60, s)],
        ),
        Lock::Height(n) => crate::strings::fill1(s.wallet_lock_height, &thousands(n)),
        Lock::Time(n) => date(n),
        Lock::Preimage => String::from(s.wallet_lock_preimage),
    }
}

/// A list of names as a sentence reads them: `A`, `A and B`,
/// `A, B and C`.
fn listed(names: &[String], s: &Strings) -> String {
    match names {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => {
            let mut head = rest[0].clone();
            for name in &rest[1..] {
                head = crate::strings::fill(s.wallet_path_list, &[&head, name]);
            }
            crate::strings::fill(s.wallet_path_last, &[&head, last])
        }
    }
}

/// One way a wallet can be spent, as the review states it: the keys it
/// wants, and the wait before they can use it.
pub fn spend_path(path: &SpendPath, s: &Strings) -> String {
    let names: Vec<String> = path.keys.iter().copied().map(key_letter).collect();
    let waits: Vec<String> = path.locks.iter().map(|l| lock(*l, s)).collect();
    let keys = match names.len() {
        0 => String::new(),
        1 => crate::strings::fill1(s.wallet_path_key, &names[0]),
        _ => crate::strings::fill1(s.wallet_path_keys, &listed(&names, s)),
    };
    match (names.is_empty(), waits.is_empty()) {
        (true, true) => String::from(s.wallet_path_anyone),
        (true, false) => crate::strings::fill1(s.wallet_path_after_only, &listed(&waits, s)),
        (false, true) => keys,
        (false, false) => crate::strings::fill(s.wallet_path_after, &[&keys, &listed(&waits, s)]),
    }
}

/// Year, month and day of the civil calendar from a count of days since
/// 1970-01-01 (Howard Hinnant's `civil_from_days`, public domain). The
/// proleptic Gregorian calendar, which is what a file system's clock
/// counts in.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe as i64 + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every word of every list reaches the screen in the form its
    /// readers write it in: `niño` with its tilde, `が` as one kana,
    /// `가격` as syllables.
    #[test]
    fn a_word_is_shown_the_way_its_readers_write_it() {
        assert_eq!(
            shown_word(EntryList::Bip39(Language::Spanish), 0),
            "\u{e1}baco"
        );
        assert_eq!(
            shown_word(EntryList::Bip39(Language::English), 0),
            "abandon"
        );
        assert_eq!(
            shown_word(EntryList::Bip39(Language::Japanese), 2),
            "\u{3042}\u{3044}\u{3060}"
        );
        assert_eq!(
            shown_word(EntryList::Bip39(Language::Korean), 0),
            "\u{ac00}\u{aca9}"
        );
        for lang in Language::ALL {
            for i in 0..2048u16 {
                let shown = shown_word(EntryList::Bip39(lang), i);
                assert!(
                    shown
                        .chars()
                        .all(|c| !('\u{0300}'..='\u{036F}').contains(&c)),
                    "{lang:?} {i}"
                );
                assert!(!shown.is_empty());
            }
        }
    }

    /// The field follows the fingers: `か` and the voicing key read as
    /// `が`, and the two-set keys of `가` read as `가` while it is being
    /// typed.
    #[test]
    fn the_field_composes_what_has_been_typed() {
        assert_eq!(
            shown_prefix(
                EntryList::Bip39(Language::Japanese),
                &['\u{304b}', '\u{309b}']
            ),
            "\u{304b}\u{309b}",
            "the standalone mark is not a word's mark"
        );
        assert_eq!(
            shown_prefix(
                EntryList::Bip39(Language::Japanese),
                &['\u{304b}', '\u{3099}']
            ),
            "\u{304c}"
        );
        assert_eq!(
            shown_prefix(
                EntryList::Bip39(Language::Korean),
                &['\u{3131}', '\u{314f}']
            ),
            "\u{ac00}"
        );
        assert_eq!(
            shown_prefix(
                EntryList::Bip39(Language::Korean),
                &['\u{3131}', '\u{314f}', '\u{3131}']
            ),
            "\u{ac01}"
        );
        assert_eq!(
            shown_prefix(EntryList::Bip39(Language::English), &['a', 'b']),
            "ab"
        );
    }

    #[test]
    fn paths_use_h_for_hardened() {
        let p: DerivationPath = "m/84'/0'/0'/0/5".parse().unwrap();
        assert_eq!(path(&p), "m/84h/0h/0h/0/5");
        assert_eq!(path(&DerivationPath::from(&[][..])), "m");
    }

    #[test]
    fn fingerprints_are_eight_hex_characters_and_never_chunked() {
        let fp = Fingerprint([0x73, 0xc5, 0xda, 0x0a]);
        assert_eq!(fingerprint_hex(fp), "73c5da0a");
        assert!(!osk_ui::widgets::is_chunked(&fingerprint_hex(fp)));
    }
}
