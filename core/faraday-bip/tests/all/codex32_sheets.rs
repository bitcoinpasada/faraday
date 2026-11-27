//! The codex32 booklet's worksheets and fresh sets, against what others
//! publish (`docs/CODEX32.md` §13.5):
//!
//! - every cell of the checksum worksheet against volvelle-website's
//!   own worksheet, run over every BIP 93 and Codex32QR string at 48 and
//!   74 characters (`tools/vectors/codex32-volvelle/cells.json`);
//! - the checksum table, the derivation tables and the recovery and
//!   fusion tables against the booklet's own, transcribed from its PDF
//!   (`tools/vectors/codex32-booklet/tables.json`);
//! - fresh sets against BIP 93's vectors 1 to 3 and the Codex32QR
//!   profile's vectors 1 to 3, which are sets made from initial shares;
//! - a sheet filled one character at a time against the sheet of the
//!   whole string, at every length BIP 93 publishes a string of.
//!
//! BIP 93's Python reference recomputes the rest, over random sets of
//! every threshold and size, in the crypto cross-check
//! (`tools/crosscheck/area_codex32.py`).

use faraday_bip::codex32::{
    ChecksumWorksheet, Codex32, Error, Length, SYMBOL_GLYPHS, Scheme, TranslationWorksheet,
    checksum_table, fresh_set, pair_symbol, translation_symbols,
};
use faraday_crypto::work::Compute;
use serde_json::Value;

const BIP93: &str = include_str!("../../../../tools/vectors/bip93/vectors.json");
const VOLVELLE: &str = include_str!("../../../../tools/vectors/codex32-volvelle/cells.json");
const BOOKLET: &str = include_str!("../../../../tools/vectors/codex32-booklet/tables.json");
const QR: &str = include_str!("../../../../tools/vectors/codex32qr/vectors.json");

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

fn json(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

fn sym(c: char) -> u8 {
    CHARSET
        .iter()
        .position(|&x| x == c.to_ascii_lowercase() as u8)
        .unwrap_or_else(|| panic!("{c} is not bech32")) as u8
}

fn ch(v: u8) -> char {
    (CHARSET[usize::from(v)] as char).to_ascii_uppercase()
}

fn err<T>(r: Result<T, Error>) -> Error {
    match r {
        Ok(_) => panic!("the call was accepted"),
        Err(e) => e,
    }
}

/// A row of cells as volvelle writes it: upper case, a space where blank.
fn row(cells: impl Iterator<Item = Option<u8>>) -> String {
    cells.map(|c| c.map_or(' ', ch)).collect()
}

/// The sheet in volvelle's rows: the first row, the residue of `ms`,
/// then each sum row with its step's data boxes and each lookup row,
/// and the final row.
fn rows(sheet: &ChecksumWorksheet) -> Vec<String> {
    let l = sheet.checksum_len();
    let mut out = vec![
        row((4..4 + l).map(|b| sheet.symbol(b))),
        row(sheet.hrp_row().iter().map(|&v| Some(v))),
    ];
    for j in 0..sheet.steps() {
        let data = sheet.sum_box(j) + l;
        let w = sheet.step_width(j);
        out.push(row((0..l)
            .map(|i| sheet.sum_cell(j, i))
            .chain((data..data + w).map(|b| sheet.symbol(b)))));
        out.push(row((0..l).map(|i| sheet.lookup_cell(j, i))));
    }
    out.push(row((0..l).map(|i| sheet.final_cell(i))));
    out
}

/// Every valid string BIP 93 publishes, at every length.
fn bip93_strings() -> Vec<String> {
    let v = json(BIP93);
    let mut out = Vec::new();
    for case in v["secrets"].as_array().unwrap() {
        out.push(case["string"].as_str().unwrap().to_lowercase());
    }
    for case in v["sets"].as_array().unwrap() {
        for s in case["shares"].as_array().unwrap() {
            out.push(s.as_str().unwrap().to_lowercase());
        }
        for s in case["derived"].as_object().unwrap().values() {
            out.push(s.as_str().unwrap().to_lowercase());
        }
        out.push(case["secret"].as_str().unwrap().to_lowercase());
    }
    out
}

#[test]
fn checksum_worksheets_match_volvelle_cell_for_cell() {
    let c = Compute::unchecked();
    let v = json(VOLVELLE);
    let sheets = v["sheets"].as_array().unwrap();
    assert!(sheets.len() >= 40, "every published string at 48 and 74");
    for sheet in sheets {
        let string = sheet["string"].as_str().unwrap();
        let ours = ChecksumWorksheet::of(&c, &Codex32::parse(string).unwrap());
        let theirs: Vec<&str> = sheet["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap())
            .collect();
        assert_eq!(rows(&ours), theirs, "{string}");
        assert!(ours.holds(), "{string}");
    }
}

#[test]
fn the_final_row_of_every_valid_string_reads_secretshare32() {
    let c = Compute::unchecked();
    for string in bip93_strings() {
        let sheet = ChecksumWorksheet::of(&c, &Codex32::parse(&string).unwrap());
        let target: String = sheet.target().iter().map(|&v| ch(v)).collect();
        let want = if string.len() == 127 {
            "SECRETSHARE32EX"
        } else {
            "SECRETSHARE32"
        };
        assert_eq!(target, want);
        let last: String = (0..sheet.checksum_len())
            .map(|i| ch(sheet.final_cell(i).unwrap()))
            .collect();
        assert_eq!(last, want, "{string}");
    }
}

/// The sheet after each character equals the whole sheet in every cell
/// it holds, and holds no cell right of the last box given.
#[test]
fn a_sheet_filled_one_character_at_a_time_is_the_whole_sheet_so_far() {
    let c = Compute::unchecked();
    for string in bip93_strings() {
        let s = Codex32::parse(&string).unwrap();
        let whole = ChecksumWorksheet::of(&c, &s);
        let mut sheet = ChecksumWorksheet::new(string.len()).unwrap();
        for (n, b) in string.bytes().enumerate().skip(3) {
            sheet.push(&c, sym(b as char)).unwrap();
            assert_eq!(sheet.last_given_box(), n + 1);
            let l = sheet.checksum_len();
            for j in 0..=sheet.steps() {
                for i in 0..l {
                    let given = sheet.sum_box(j) + i <= n + 1;
                    let want = whole.sum_cell(j, i).filter(|_| given);
                    assert_eq!(
                        sheet.sum_cell(j, i),
                        want,
                        "{string} box {} sum {j}.{i}",
                        n + 1
                    );
                    if j < sheet.steps() {
                        let lead = sheet.sum_box(j) + sheet.step_width(j) - 1 <= n + 1;
                        let want = whole.lookup_cell(j, i).filter(|_| lead);
                        assert_eq!(sheet.lookup_cell(j, i), want, "{string} lookup {j}.{i}");
                    }
                }
            }
        }
        assert!(sheet.holds(), "{string}");
        assert_eq!(sheet.push(&c, 0).err(), Some(Error::BadLength));
    }
}

#[test]
fn making_the_checksum_gives_the_strings_own() {
    let c = Compute::unchecked();
    for string in bip93_strings() {
        let mut sheet = ChecksumWorksheet::new(string.len()).unwrap();
        let end = sheet.last_data_box();
        for b in string[3..end].chars() {
            sheet.push(&c, sym(b)).unwrap();
        }
        assert!(
            sheet.final_cell(0).is_none(),
            "nothing below the checksum yet"
        );
        sheet.finish(&c).unwrap();
        let made: String = (4..=string.len())
            .map(|b| ch(sheet.symbol(b).unwrap()))
            .collect();
        assert_eq!(made.to_lowercase(), string[3..], "{string}");
        assert!(sheet.holds());
        assert_eq!(err(sheet.finish(&c)), Error::BadLength);
    }
}

#[test]
fn a_changed_box_shows_in_the_final_row() {
    let c = Compute::unchecked();
    let string = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
    for at in 3..string.len() {
        let mut sheet = ChecksumWorksheet::new(48).unwrap();
        for (n, b) in string.chars().enumerate().skip(3) {
            let v = sym(b);
            sheet.push(&c, if n == at { v ^ 1 } else { v }).unwrap();
        }
        assert!(!sheet.holds(), "box {}", at + 1);
        assert!(sheet.first_difference().is_some());
    }
}

#[test]
fn taking_back_a_character_and_typing_it_again_changes_nothing() {
    let c = Compute::unchecked();
    let string = "ms13cashsllhdmn9m42vcsamx24zrxgs3qqjzqud4m0d6nln";
    let whole = ChecksumWorksheet::of(&c, &Codex32::parse(string).unwrap());
    let mut sheet = ChecksumWorksheet::new(48).unwrap();
    for b in string[3..].chars() {
        sheet.push(&c, sym(b)).unwrap();
    }
    for _ in 0..20 {
        sheet.pop();
    }
    assert_eq!(sheet.last_given_box(), 28);
    assert!(sheet.final_cell(0).is_none());
    for b in string[28..].chars() {
        sheet.push(&c, sym(b)).unwrap();
    }
    assert_eq!(rows(&sheet), rows(&whole));
}

#[test]
fn a_worksheet_refuses_what_is_not_a_codex32_length_or_symbol() {
    let c = Compute::unchecked();
    assert_eq!(err(ChecksumWorksheet::new(47)), Error::BadLength);
    let mut sheet = ChecksumWorksheet::new(48).unwrap();
    assert_eq!(
        sheet.push(&c, 32).err(),
        Some(Error::BadCharacter { position: 3 })
    );
    assert_eq!(err(sheet.finish(&c)), Error::BadLength);
}

// ---------------------------------------------------------------------
// The booklet's tables
// ---------------------------------------------------------------------

#[test]
fn the_checksum_table_is_the_booklets() {
    let v = json(BOOKLET);
    let table = v["checksum"].as_object().unwrap();
    assert_eq!(table.len(), 1024);
    for (pair, want) in table {
        let mut p = pair.chars();
        let got: String = checksum_table(sym(p.next().unwrap()), sym(p.next().unwrap()))
            .iter()
            .map(|&v| ch(v))
            .collect();
        assert_eq!(&got, want.as_str().unwrap(), "pair {pair}");
    }
}

#[test]
fn the_glyphs_are_the_booklets_second_alphabet() {
    let v = json(BOOKLET);
    let fusion: String = SYMBOL_GLYPHS[1..].iter().collect();
    assert_eq!(v["fusion"]["columns"].as_str().unwrap(), fusion);
    assert_eq!(v["translation"]["columns"].as_str().unwrap(), fusion);
}

fn glyph(v: u8) -> char {
    SYMBOL_GLYPHS[usize::from(v)]
}

#[test]
fn recovery_symbols_are_the_booklets_recovery_table() {
    let v = json(BOOKLET);
    let table = &v["recovery"];
    let columns: Vec<char> = table["columns"].as_str().unwrap().chars().collect();
    for (other, cells) in table["rows"].as_object().unwrap() {
        let other = other.to_ascii_lowercase().as_bytes()[0];
        for (own, cell) in columns.iter().zip(cells.as_str().unwrap().chars()) {
            let own = own.to_ascii_lowercase() as u8;
            let got = pair_symbol(own, other, b's').map_or('×', glyph);
            assert_eq!(got, cell, "column {own} row {other}");
        }
    }
}

/// Every derivation table the booklet prints (k = 2 to 8, from A, C, D,
/// … and from S, A, C, …): the column for a new share is the inputs'
/// translation symbols towards it.
#[test]
fn translation_symbols_are_the_booklets_derivation_tables() {
    let v = json(BOOKLET);
    let tables = v["derivation"].as_array().unwrap();
    assert_eq!(tables.len(), 14);
    for t in tables {
        let initial = t["initial"].as_str().unwrap().to_ascii_lowercase();
        let rows: Vec<Vec<char>> = initial
            .chars()
            .map(|r| {
                t["rows"][r.to_ascii_uppercase().to_string()]
                    .as_str()
                    .unwrap()
                    .chars()
                    .collect()
            })
            .collect();
        for (col, target) in t["columns"].as_str().unwrap().chars().enumerate() {
            let mut out = [0u8; 9];
            let target = target.to_ascii_lowercase() as u8;
            translation_symbols(initial.as_bytes(), target, &mut out).unwrap();
            for (i, r) in rows.iter().enumerate() {
                assert_eq!(
                    glyph(out[i]),
                    r[col],
                    "k {} {initial} → {}",
                    t["k"],
                    target as char
                );
            }
        }
    }
}

/// Recovering by table, as the booklet's III.2.A says: each share's
/// recovery symbols, from its column of the recovery table, fused
/// pairwise with the fusion table, give the symbol the translation
/// worksheet carries. Every set of three shares.
#[test]
fn fusing_the_booklets_recovery_symbols_gives_the_translation_symbol() {
    let v = json(BOOKLET);
    let columns: Vec<char> = v["recovery"]["columns"].as_str().unwrap().chars().collect();
    let recovery = |own: char, other: char| -> char {
        let i = columns.iter().position(|&c| c == own).unwrap();
        v["recovery"]["rows"][other.to_string()]
            .as_str()
            .unwrap()
            .chars()
            .nth(i)
            .unwrap()
    };
    let heads: Vec<char> = v["fusion"]["columns"].as_str().unwrap().chars().collect();
    let fuse = |a: char, b: char| -> char {
        let i = heads.iter().position(|&c| c == b).unwrap();
        v["fusion"]["rows"][a.to_string()]
            .as_str()
            .unwrap()
            .chars()
            .nth(i)
            .unwrap()
    };
    for (n, &a) in columns.iter().enumerate() {
        for (m, &b) in columns.iter().enumerate().skip(n + 1) {
            for &d in &columns[m + 1..] {
                let set = [a, b, d];
                let indices: Vec<u8> = set.iter().map(|c| c.to_ascii_lowercase() as u8).collect();
                let sheet = TranslationWorksheet::new(48, &indices, b's').unwrap();
                for (i, &own) in set.iter().enumerate() {
                    let others: Vec<char> = set.iter().copied().filter(|&o| o != own).collect();
                    let fused = fuse(recovery(own, others[0]), recovery(own, others[1]));
                    assert_eq!(glyph(sheet.symbol(i)), fused, "{set:?} share {own}");
                    let j = (0..3).filter(|&j| j != i).collect::<Vec<_>>();
                    assert_eq!(glyph(sheet.pair(i, j[0])), recovery(own, others[0]));
                    assert_eq!(glyph(sheet.pair(i, j[1])), recovery(own, others[1]));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// Translation worksheets
// ---------------------------------------------------------------------

/// BIP 93's sets: each derived share and the secret, from the published
/// shares through the translation worksheet, filled one character at a
/// time, input after input as the booklet fills it.
#[test]
fn translation_worksheets_make_the_published_derived_shares() {
    let c = Compute::unchecked();
    let v = json(BIP93);
    for case in v["sets"].as_array().unwrap() {
        let shares: Vec<String> = case["shares"]
            .as_array()
            .unwrap()
            .iter()
            .chain(case["derived"].as_object().unwrap().values())
            .map(|s| s.as_str().unwrap().to_lowercase())
            .collect();
        let k = usize::from(shares[0].as_bytes()[3] - b'0');
        let inputs: Vec<Codex32> = shares[..k]
            .iter()
            .map(|s| Codex32::parse(s).unwrap())
            .collect();
        let indices: Vec<u8> = shares[..k].iter().map(|s| s.as_bytes()[8]).collect();
        let mut targets: Vec<(u8, String)> = shares[k..]
            .iter()
            .map(|s| (s.as_bytes()[8], s.clone()))
            .collect();
        targets.push((b's', case["secret"].as_str().unwrap().to_lowercase()));
        for (target, want) in targets {
            let whole = TranslationWorksheet::of(&c, &inputs, target).unwrap();
            assert_eq!(whole.result().unwrap().encode().as_str(), want);
            let mut sheet = TranslationWorksheet::new(want.len(), &indices, target).unwrap();
            assert_eq!(sheet.is_recovery(), target == b's');
            for (i, s) in shares[..k].iter().enumerate() {
                for (n, b) in s.chars().enumerate().skip(3) {
                    sheet.push(&c, i, sym(b)).unwrap();
                    assert_eq!(
                        sheet.translated_cell(i, n + 1),
                        whole.translated_cell(i, n + 1)
                    );
                    let last = i + 1 == k;
                    let sum = sheet.sum_cell(n + 1);
                    assert_eq!(sum, whole.sum_cell(n + 1).filter(|_| last));
                }
            }
            assert_eq!(sheet.result().unwrap().encode().as_str(), want);
            for b in 4..=want.len() {
                assert_eq!(sheet.sum_cell(b), Some(sym(want.as_bytes()[b - 1] as char)));
            }
        }
    }
}

#[test]
fn a_translation_worksheet_refuses_a_target_among_its_inputs() {
    assert_eq!(
        err(TranslationWorksheet::new(48, b"ac", b'a')),
        Error::BadShareIndex
    );
    assert_eq!(
        err(TranslationWorksheet::new(48, b"aa", b'c')),
        Error::DuplicateIndex
    );
    assert_eq!(
        err(TranslationWorksheet::new(48, b"ab", b'c')),
        Error::BadShareIndex
    );
    assert_eq!(pair_symbol(b'a', b'a', b's'), None);
}

// ---------------------------------------------------------------------
// Fresh sets
// ---------------------------------------------------------------------

fn data_of(s: &str, len: Length) -> Vec<u8> {
    s[9..9 + len.data_len()].chars().map(sym).collect()
}

/// BIP 93's vector 2 (k = 2, initial shares A and C) and vector 3
/// (k = 3, initial A, C and D): the set made from the initial shares'
/// data is the published one, secret and derived shares included.
#[test]
fn a_fresh_set_from_bip93s_initial_shares_is_its_set() {
    let c = Compute::unchecked();
    let v = json(BIP93);
    for case in v["sets"].as_array().unwrap() {
        let mut all: Vec<String> = case["shares"]
            .as_array()
            .unwrap()
            .iter()
            .chain(case["derived"].as_object().unwrap().values())
            .map(|s| s.as_str().unwrap().to_lowercase())
            .collect();
        all.sort_by_key(|s| {
            b"acdefghjklmnpqrtuvwxyz023456789"
                .iter()
                .position(|&x| x == s.as_bytes()[8])
        });
        let k = all[0].as_bytes()[3] - b'0';
        let identifier: [u8; 4] = all[0].as_bytes()[4..8].try_into().unwrap();
        let scheme = Scheme::new(k, all.len() as u8, identifier, Length::Chars48).unwrap();
        let data: Vec<Vec<u8>> = all[..usize::from(k)]
            .iter()
            .map(|s| data_of(s, Length::Chars48))
            .collect();
        let refs: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
        let set = fresh_set(&c, &scheme, &refs).unwrap();
        let made: Vec<String> = set
            .strings()
            .iter()
            .map(|s| s.encode().as_str().to_string())
            .collect();
        assert_eq!(made, all, "vector {}", case["vector"]);
        assert_eq!(
            set.secret().encode().as_str(),
            case["secret"].as_str().unwrap().to_lowercase()
        );
        assert_eq!(
            crate::common::hex(set.secret().to_seed().unwrap().expose().bytes()),
            case["seed"].as_str().unwrap()
        );
    }
}

#[test]
fn a_fresh_secret_alone_is_its_data_checksummed() {
    let c = Compute::unchecked();
    let string = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
    let scheme = Scheme::new(0, 1, *b"test", Length::Chars48).unwrap();
    assert_eq!(scheme.initial_indices(), b"s");
    assert!(scheme.derived_indices().is_empty());
    let data = data_of(string, Length::Chars48);
    let set = fresh_set(&c, &scheme, &[&data]).unwrap();
    assert_eq!(set.strings().len(), 1);
    assert_eq!(set.strings()[0].encode().as_str(), string);
    assert_eq!(set.secret().encode().as_str(), string);
}

#[test]
fn a_scheme_takes_any_threshold_and_up_to_31_strings() {
    for k in 2..=9u8 {
        for n in k..=31u8 {
            let s = Scheme::new(k, n, *b"Ad2c", Length::Chars74).unwrap();
            assert_eq!(s.identifier(), *b"ad2c");
            assert_eq!(s.initial_indices().len(), usize::from(k));
            assert_eq!(s.derived_indices().len(), usize::from(n - k));
            assert!(!s.initial_indices().contains(&b's'));
        }
        assert_eq!(
            err(Scheme::new(k, k - 1, *b"name", Length::Chars48)),
            Error::BadSplit
        );
        assert_eq!(
            err(Scheme::new(k, 32, *b"name", Length::Chars48)),
            Error::BadSplit
        );
    }
    for (k, n) in [(1, 1), (1, 3), (10, 12), (0, 2), (0, 0)] {
        assert_eq!(
            err(Scheme::new(k, n, *b"name", Length::Chars48)),
            Error::BadSplit
        );
    }
    assert_eq!(
        err(Scheme::new(2, 3, *b"nam1", Length::Chars48)),
        Error::BadIdentifier
    );
    assert_eq!(
        err(Scheme::new(2, 3, *b"namb", Length::Chars48)),
        Error::BadIdentifier
    );
}

/// Sets of every threshold, at every length made, from random initial
/// shares: every string parses, the secret has the length's seed, and
/// any k strings recover it and derive the others.
#[test]
fn fresh_sets_of_every_threshold_recover_from_any_k_strings() {
    let c = Compute::unchecked();
    let mut rng = crate::common::XorShift::new(93);
    for len in [Length::Chars48, Length::Chars74, Length::Chars127] {
        for k in 2..=9u8 {
            let n = [k, 9.max(k), 31][usize::from(k) % 3];
            let scheme = Scheme::new(k, n, *b"fr5h", len).unwrap();
            let data: Vec<Vec<u8>> = (0..k)
                .map(|_| {
                    let mut b = vec![0u8; len.data_len()];
                    rng.fill(&mut b);
                    b.iter().map(|x| x & 31).collect()
                })
                .collect();
            let refs: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
            let set = fresh_set(&c, &scheme, &refs).unwrap();
            assert_eq!(set.strings().len(), usize::from(n));
            for (s, d) in set.strings().iter().zip(&data) {
                assert_eq!(s.payload(), d.as_slice(), "the entropy stands unchanged");
            }
            let texts: Vec<String> = set
                .strings()
                .iter()
                .map(|s| s.encode().as_str().to_string())
                .collect();
            for t in &texts {
                assert_eq!(t.len(), len.chars());
                Codex32::parse(t).unwrap();
            }
            let seed = set.secret().to_seed().unwrap();
            assert_eq!(seed.expose().bytes().len(), len.seed_len());
            // The last k strings, derived ones among them.
            let last: Vec<Codex32> = texts[texts.len() - usize::from(k)..]
                .iter()
                .map(|t| Codex32::parse(t).unwrap())
                .collect();
            let back = Codex32::recover(&last).unwrap();
            assert_eq!(back.encode().as_str(), set.secret().encode().as_str());
            let first = Codex32::derive_share(&last, b'a');
            if texts.len() > usize::from(k) {
                assert_eq!(first.unwrap().encode().as_str(), texts[0]);
            }
        }
    }
}

#[test]
fn a_fresh_set_refuses_the_wrong_entropy() {
    let c = Compute::unchecked();
    let scheme = Scheme::new(2, 3, *b"name", Length::Chars48).unwrap();
    let ok = [0u8; 26];
    assert_eq!(err(fresh_set(&c, &scheme, &[&ok])), Error::WrongShareCount);
    assert_eq!(
        err(fresh_set(&c, &scheme, &[&ok, &ok[..25]])),
        Error::BadLength
    );
    let mut bad = [0u8; 26];
    bad[4] = 32;
    assert_eq!(
        err(fresh_set(&c, &scheme, &[&ok, &bad])),
        Error::BadCharacter { position: 13 }
    );
}

// ---------------------------------------------------------------------
// Codex32QR/v1-48
// ---------------------------------------------------------------------

/// The Codex32QR profile's vectors 1 to 4 (`docs/CODEX32.md` §10.3):
/// each set recovers to the stated secret, a set made from its shares'
/// data is the same set, and the seed has the stated fingerprint and,
/// where the profile prints it whole, the stated key at the stated path.
#[test]
fn codex32qr_vectors_recover_to_their_secret_and_key() {
    use faraday_bip::bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
    use faraday_bip::bitcoin::{NetworkKind, secp256k1::Secp256k1};
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let v = json(QR);
    let vectors = v["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 4);
    for case in vectors {
        let name = case["vector"].as_u64().unwrap();
        let secret_text = case["secret"].as_str().unwrap();
        let secret = Codex32::parse(secret_text).expect("upper case is read");
        assert_eq!(secret.encode().as_str(), secret_text.to_lowercase());
        let shares: Vec<&str> = case["shares"]
            .as_object()
            .unwrap()
            .values()
            .map(|s| s.as_str().unwrap())
            .collect();
        if !shares.is_empty() {
            let parsed: Vec<Codex32> = shares.iter().map(|s| Codex32::parse(s).unwrap()).collect();
            let back = Codex32::recover(&parsed).unwrap();
            assert_eq!(
                back.encode().as_str(),
                secret_text.to_lowercase(),
                "vector {name}"
            );
            let k = parsed[0].threshold();
            let ident: [u8; 4] = parsed[0].identifier();
            let scheme = Scheme::new(k, k, ident, Length::Chars48).unwrap();
            let data: Vec<Vec<u8>> = shares.iter().map(|s| data_of(s, Length::Chars48)).collect();
            let refs: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
            let set = fresh_set(&c, &scheme, &refs).unwrap();
            assert_eq!(set.secret().encode().as_str(), secret_text.to_lowercase());
        }
        let seed = secret.to_seed().unwrap();
        let master = Xpriv::new_master(NetworkKind::Test, seed.expose().bytes()).unwrap();
        assert_eq!(
            master.fingerprint(&secp).to_string(),
            case["fingerprint"].as_str().unwrap(),
            "vector {name}"
        );
        if case["xpub"]["complete"].as_bool().unwrap() {
            let path: DerivationPath = case["derivation"].as_str().unwrap().parse().unwrap();
            let child = master.derive_priv(&secp, &path).unwrap();
            assert_eq!(
                Xpub::from_priv(&secp, &child).to_string(),
                case["xpub"]["text"].as_str().unwrap(),
                "vector {name}"
            );
        }
    }
}
