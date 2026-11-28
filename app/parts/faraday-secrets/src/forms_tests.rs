//! The secret forms the classifier names, one by one, with the vectors `main`'s tests use
//! (`transfer_text`, `codes`, `gpg`, `wallet_secret_text`, `psbt_v2`): each form is refused as a
//! text and held back as a file, so a regression in the shape tests of `scan` or `files` is seen.

use faraday_app_test::test_compute;
use faraday_bip::bip39::Language;
use faraday_kernel::session::FileKind;
use faraday_pgp::{Armor, armor, armor_with};
use faraday_psbt::Psbt;
use faraday_psbt::v2::PsbtV2;

use crate::{SecretKind, TextCheck, check, classify};

const SEED: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
/// 12 words of entropy zeros as SeedQR digits (4 digits a word).
const SEEDQR: &str = "000000000000000000000000000000000000000000000003";
const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
const WIF: &str = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";
const NSEC: &str = "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5";
/// BIP-93's first vector, a secret, and its second's first share.
const CODEX32_SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
const CODEX32_SHARE: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
/// A published SLIP-39 share (20 words).
const SLIP39: &str = "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed";
const OTPAUTH: &str = "otpauth://totp/Mail:me?secret=JBSWY3DPEHPK3PXP&issuer=Mail";
const MIGRATION: &str = "otpauth-migration://offline?data=CjEKCkhlbGxvId6tvu8SGEV4YW1wbGU6YWxpY2VAZXhhbXBsZS5jb20aB0V4YW1wbGUgASgBMAIQARgBIAAo2a";
/// BIP-39 vectors in two other lists (`tools/vectors/bip39`).
const SEED_SPANISH: &str =
    "ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco abierto";
const SEED_SPANISH_PLAIN: &str =
    "abaco abaco abaco abaco abaco abaco abaco abaco abaco abaco abaco abierto";
const SEED_JAPANESE: &str = "あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あおぞら";
/// paperkey 1.6's output for a Faraday GPG key (`tools/vectors/paperkey/README.md`).
const PAPERKEY: &str = include_str!("../../../../tools/vectors/paperkey/faraday-key.txt");

fn checked(text: &str) -> TextCheck {
    check(test_compute(), text)
}

fn refused(text: &str) -> bool {
    !matches!(checked(text), TextCheck::Fine)
}

/// A share sheet (a public file) with `extra` appended: a seed there makes it a secret file.
fn share_sheet(extra: &str) -> String {
    format!(
        "Name: Split\nPolicy: 2 of 3\nFormat: P2WSH\nDerivation: m/48'/1'/0'/2'\n\
         3442193E: xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8\n{extra}"
    )
}

/// Invariant: SeedQR digits (48 or 96, or in groups of four), a codex32 secret and share, a SLIP-39 share, an nsec and a TOTP setup code are each named by their own kind as a whole text, and refused inside other text.
/// Regression: One of these forms loses its shape test, so a seed's QR digits, a codex32 string, a Nostr key or a TOTP secret is shown or sent as an ordinary line.
/// Owner: app
#[test]
fn each_secret_form_is_named_by_its_kind() {
    assert_eq!(checked(SEEDQR), TextCheck::Secret(SecretKind::SeedQr));
    assert_eq!(
        checked(&SEEDQR.repeat(2)),
        TextCheck::Secret(SecretKind::SeedQr)
    );
    assert_eq!(
        checked(CODEX32_SECRET),
        TextCheck::Secret(SecretKind::Codex32Secret)
    );
    assert_eq!(
        checked(CODEX32_SHARE),
        TextCheck::Secret(SecretKind::Codex32Share)
    );
    assert_eq!(checked(SLIP39), TextCheck::Secret(SecretKind::Slip39Share));
    assert_eq!(checked(NSEC), TextCheck::Secret(SecretKind::Nostr));
    assert_eq!(checked(OTPAUTH), TextCheck::Secret(SecretKind::Totp));
    assert_eq!(checked(MIGRATION), TextCheck::Secret(SecretKind::Totp));
    for text in [SEEDQR, CODEX32_SECRET, CODEX32_SHARE, SLIP39, NSEC, OTPAUTH] {
        assert!(
            refused(&format!("Sweep {text} then wait for the rest.")),
            "{text}"
        );
    }
    // One character off is refused as a secret that does not check, never shown.
    assert!(refused(&CODEX32_SECRET.replace("czlw", "czlx")));
    assert!(refused(&XPRV.replace("33yuGBxrMPHi", "33yuGBxrMPHj")));
    assert!(refused(SLIP39.rsplit_once(' ').map_or(SLIP39, |(a, _)| a)));
    assert!(refused(&SEED.replacen(" about", "", 1)));
}

/// Invariant: SeedQR digits copied in groups of four, one group to a line or as one run, are refused; a short run of groups is an ordinary number.
/// Regression: A SeedQR written as groups of four digits is sent as ordinary text.
/// Owner: app
#[test]
fn seedqr_digits_in_groups_of_four_are_refused() {
    let groups = "0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0003";
    assert!(refused(groups));
    assert!(refused(&format!("code: {}", groups.replace(' ', "\n"))));
    assert!(!refused("0000 0000 0000 0000 0003"));
}

/// Invariant: The 16 bytes of a 12-word seed's entropy (a CompactSeedQR) are named a CompactSeedQR by the scan.
/// Regression: A scanned CompactSeedQR is read as no secret form and goes out as an ordinary code.
/// Owner: app
#[test]
fn a_compact_seedqr_is_named_by_the_scan() {
    let c = test_compute();
    let Ok(m) = faraday_keys::words::parse(c, Language::English, SEED) else {
        panic!("a valid mnemonic");
    };
    let compact = faraday_codec::seedqr::to_entropy(&m);
    assert_eq!(
        crate::scan::secret_kind(c, compact.expose().as_bytes()),
        Some(SecretKind::CompactSeedQr)
    );
}

/// Invariant: A UR that can hold a secret is refused whole or in parts (a private key, a share, a type nobody here knows), a seed UR is named, and a UR of a PSBT or of a text with nothing secret in it is shown.
/// Regression: A UR's parts are shown or sent one by one until the message is whole, or an unknown UR type is let through.
/// Owner: app
#[test]
fn a_ur_that_can_hold_a_secret_is_refused_whole_or_in_parts() {
    use faraday_codec::ur::{Encoder, encode_single};
    let mut eckey = vec![0xa2, 0x02, 0xf5, 0x03, 0x58, 0x20];
    eckey.extend([7u8; 32]);
    assert!(refused(&encode_single("crypto-eckey", &eckey)));
    assert!(refused(&encode_single("eckey", &eckey)));
    assert!(refused(&encode_single(
        "crypto-sskr",
        &[0x58, 0x04, 1, 2, 3, 4]
    )));
    assert!(refused(
        "ur:crypto-seed/oeadgdhkwzdtfthptokigtvwnnjsqzcxknsktdsfecsbbk"
    ));

    // A secret in the parts of a UR of bytes: refused until whole and read, then by what it holds.
    let secret = format!("my key is p2wpkh:{WIF}");
    let Ok(mut enc) = Encoder::bytes(secret.as_bytes(), 20) else {
        panic!("a multi-part encoder");
    };
    assert!(enc.fragment_count() > 2);
    let parts: Vec<String> = (0..enc.fragment_count())
        .map(|_| String::from(&*enc.next_part()))
        .collect();
    assert!(refused(&parts[0]));
    assert!(refused(&parts[..parts.len() - 1].join(" ")));
    assert!(refused(&parts.join("\n")));

    // Parts of a type nobody here knows, one and all but one.
    let Ok(mut enc) = Encoder::bytes(&[9u8; 100], 20) else {
        panic!("a multi-part encoder");
    };
    let parts: Vec<String> = (0..enc.fragment_count())
        .map(|_| enc.next_part().replacen("ur:bytes/", "ur:envelope/", 1))
        .collect();
    assert!(parts.len() > 2 && parts[0].starts_with("ur:envelope/"));
    assert!(refused(&parts[0]));
    assert!(refused(&parts[..parts.len() - 1].join(" ")));

    // A PSBT's parts, and a whole text with nothing secret in it, are shown.
    let Ok(mut enc) = Encoder::psbt(&[9u8; 100], 20) else {
        panic!("a multi-part encoder");
    };
    let psbt_parts: Vec<String> = (0..enc.fragment_count())
        .map(|_| String::from(&*enc.next_part()))
        .collect();
    assert!(!refused(&psbt_parts[0]));
    let Ok(mut enc) = Encoder::bytes(b"a block header, nothing secret in it", 20) else {
        panic!("a multi-part encoder");
    };
    let plain: Vec<String> = (0..enc.fragment_count())
        .map(|_| String::from(&*enc.next_part()))
        .collect();
    assert!(!refused(&plain.join("\n")));
}

/// Invariant: A paperkey, whole or among other text, is named a paperkey by the text check and the scan.
/// Regression: A GPG key's paperkey, the one backup of its secret key, goes out as a text file.
/// Owner: app
#[test]
fn a_paperkey_is_named_and_filed_as_a_secret() {
    assert_eq!(checked(PAPERKEY), TextCheck::Secret(SecretKind::Paperkey));
    assert_eq!(
        crate::scan::secret_kind(test_compute(), PAPERKEY.as_bytes()),
        Some(SecretKind::Paperkey)
    );
    assert!(refused(&format!("my key\n{PAPERKEY}")));
}

/// Invariant: A numbered file of seed words, and a seed in a BIP-39 list other than English (Spanish, Japanese), with or without its accents, is a seed.
/// Regression: A seed numbered one word to a line, or written in another list or without its accents, is shown or filed as text.
/// Owner: app
#[test]
fn a_numbered_or_other_list_seed_is_a_seed() {
    let numbered: String = SEED
        .split(' ')
        .enumerate()
        .map(|(i, w)| format!("{}. {w}\n", i + 1))
        .collect();
    assert!(refused(&format!("# my seed\n{numbered}")));
    assert_eq!(
        classify(test_compute(), "seed.txt", numbered.as_bytes()),
        FileKind::Words
    );
    for seed in [SEED_SPANISH, SEED_SPANISH_PLAIN, SEED_JAPANESE] {
        assert!(refused(seed), "{seed}");
        assert!(refused(&format!("backup: {seed} keep it")), "{seed}");
    }
    for seed in [SEED_SPANISH, SEED_JAPANESE] {
        assert_eq!(
            classify(test_compute(), "seed.txt", seed.as_bytes()),
            FileKind::Words,
            "{seed}"
        );
    }
    // Ordinary Spanish words are not a seed.
    assert!(!refused(
        "ábaco abierto abogado abono aborto abrazo abrir abuelo"
    ));
}

/// The layouts a seed hides in beside other text: after a label, in a line, wrapped, one word to
/// a line with its place, and in capitals.
fn hidden_seed_comments(seed: &str) -> Vec<String> {
    let words: Vec<&str> = seed.split_whitespace().collect();
    let wrapped: String = words
        .chunks(5)
        .map(|c| format!("# {}\n", c.join(" ")))
        .collect();
    let numbered: String = words
        .iter()
        .enumerate()
        .map(|(i, w)| format!("# {}. {w}\n", i + 1))
        .collect();
    vec![
        format!("# seed: {seed} here\n"),
        format!("# {}\n", words.join(" ")),
        wrapped,
        numbered,
        format!("# Seed: {} here\n", seed.to_uppercase()),
    ]
}

/// A valid French seed of one word with two accents, written with only some of its accents:
/// the first accent left off a composed word, and off a decomposed one.
fn half_accented_french_seeds() -> Vec<String> {
    let c = test_compute();
    let l = Language::French;
    let Some(word) =
        (0..2048u16).find(|&i| l.word_nfkd(i).chars().filter(|c| !c.is_ascii()).count() >= 2)
    else {
        panic!("a two-accent word");
    };
    let (shown, nfkd) = (l.word_display(word), l.word_nfkd(word));
    let Some(last) = (0..2048u16).map(|i| l.word_display(i)).find(|w| {
        faraday_keys::words::parse(c, l, &format!("{} {w}", [shown; 11].join(" "))).is_ok()
    }) else {
        panic!("a closing word");
    };
    let first_accent = shown.chars().position(|c| !c.is_ascii()).unwrap_or(0);
    let composed: String = shown
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let plain = match c {
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'à' | 'â' => 'a',
                'î' | 'ï' => 'i',
                'ô' => 'o',
                'ù' | 'û' | 'ü' => 'u',
                'ç' => 'c',
                other => other,
            };
            if i == first_accent { plain } else { c }
        })
        .collect();
    let mut dropped = false;
    let decomposed: String = nfkd
        .chars()
        .filter(|c| {
            let mark = ('\u{0300}'..='\u{036F}').contains(c);
            !(mark && !std::mem::replace(&mut dropped, true))
        })
        .collect();
    [composed, decomposed]
        .into_iter()
        .map(|w| format!("{} {last}", [w.as_str(); 11].join(" ")))
        .collect()
}

/// Invariant: A seed in another list, written without its accents or with only some of them, beside other text in a share sheet or a signed message, makes the file a secret one; ordinary words in a comment keep the sheet public.
/// Regression: A Spanish or French seed with its accents left off, in a share sheet's comment or below a signed message, is sent as a public file.
/// Owner: app
#[test]
fn an_accentless_or_half_accented_seed_beside_other_text_is_a_secret_file() {
    const SIGNATURE: &str =
        "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";
    let c = test_compute();
    assert_eq!(
        classify(c, "share.txt", share_sheet("").as_bytes()),
        FileKind::Share
    );
    let mut seeds = vec![
        SEED_SPANISH.to_string(),
        SEED_SPANISH_PLAIN.to_string(),
        SEED_JAPANESE.to_string(),
    ];
    seeds.extend(half_accented_french_seeds());
    for seed in seeds {
        for extra in hidden_seed_comments(&seed) {
            assert_ne!(
                classify(c, "share.txt", share_sheet(&extra).as_bytes()),
                FileKind::Share,
                "{extra}"
            );
            let signed =
                format!("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{extra}");
            assert_ne!(
                classify(c, "message.txt", signed.as_bytes()),
                FileKind::Message,
                "{extra}"
            );
        }
    }
    assert_eq!(
        classify(
            c,
            "share.txt",
            share_sheet("# ábaco abierto abogado abono aborto abrazo abrir abuelo\n").as_bytes()
        ),
        FileKind::Share
    );
}

/// Invariant: A GPG public key whose armour header or user ID holds a seed (in English, without its accents or in capitals) is not a public key file; one with neither is.
/// Regression: A GPG key block carrying a seed in its Comment header is sent as a public key.
/// Owner: app
#[test]
fn a_seed_in_a_gpg_header_or_user_id_is_not_a_public_key() {
    let c = test_compute();
    let mut k = faraday_pgp::Key {
        primary_seed: [0x11; 32],
        created: 1_780_000_000,
        subkey_seed: [0x22; 32],
        subkey_created: 1_780_000_000,
        user_ids: vec![faraday_text::Text::of("Test Person <test@example.com>")],
        expiry: 0,
    };
    let cert = k.certificate(1_780_000_000);
    let public = armor(Armor::PublicKey, &cert);
    assert_eq!(classify(c, "k.asc", public.as_bytes()), FileKind::Gpg);
    for seed in [
        SEED,
        SEED_SPANISH_PLAIN,
        "ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABANDON ABOUT",
        XPRV,
    ] {
        let with_header = armor_with(Armor::PublicKey, &cert, Some(seed));
        assert_ne!(
            classify(c, "k.asc", with_header.as_bytes()),
            FileKind::Gpg,
            "{seed}"
        );
        k.user_ids = vec![faraday_text::Text::of(seed)];
        let seeded = armor(Armor::PublicKey, &k.certificate(1_780_000_000));
        assert_ne!(
            classify(c, "k.asc", seeded.as_bytes()),
            FileKind::Gpg,
            "{seed}"
        );
    }
}

/// Invariant: A key file with a private key copied in groups of four on lines is named as holding a secret; the same file with no private key is a plain key file.
/// Regression: An extended private key broken into groups of four beside a public key goes out as a public key file.
/// Owner: app
#[test]
fn a_private_key_in_groups_of_four_makes_a_key_file_secret() {
    let key = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";
    let groups: Vec<&str> = XPRV
        .as_bytes()
        .chunks(4)
        .filter_map(|g| std::str::from_utf8(g).ok())
        .collect();
    let c = test_compute();
    let file = format!("{key}\n{}\n", groups.join("\n"));
    assert_eq!(
        classify(c, "keys.txt", file.as_bytes()),
        FileKind::KeyWithSecret
    );
    assert_ne!(
        classify(c, "keys.txt", format!("{key}\n").as_bytes()),
        FileKind::KeyWithSecret
    );
}

/// BIP-370's smallest valid version 2 PSBT (`tools/vectors/psbt-bips`): one input, two outputs.
const PSBT_V2: &str = "70736274ff01020402000000010401010105010201fb040200000000010e200b0ad921419c1c8719735d72dc739f9ea9e0638d1fe4c1eef0f9944084815fc8010f0400000000000103080008af2f000000000104160014c430f64c4756da310dbd1a085572ef299926272c000103088bbdeb0b0000000001041600144dd193ac964a56ac1b9e1cca8454fe2f474f851300";
/// BIP-370's vector with no locktime that holds for both inputs: it parses, and does not read as
/// version 0.
const PSBT_V2_NO_LOCKTIME: &str = "70736274ff0102040200000001030400000000010401020105010101fb040200000000010e200f758dbfbd4da7c16c8a3309c3c81e1100f561ea646db5b01752c485e1bdde9f010f04010000000112041027000000010e203a1b3b3c837d6489ea7a31d8e6c7dd503c001bef3e06958e7574808d68ca78a5010f04000000000111048c8dc462000103084f9335770000000001041600140b1352cacd03cf6aa1b7f3c8d6388671b34a5e1100";
/// The first valid vector of BIP-375, a version 2 PSBT with silent payment fields.
const PSBT_SILENT: &str = "cHNidP8B+wQCAAAAAQIEAgAAAAEEAQEBBQEBAQYBAAABDiAYpxdmOwurFLEqGncTI/8eQHndUy5d0T4o6hCBxwCYSgEPBAAAAAABAQqghgEAAAAAAAFSIgICyBe7dSGvw16pbzv7Jw5utQ3f+lVgYnuWH+wA8pllCL9HMEQCIHLzeIUvuENyYHLyUHbw53Vg7UwuBHFm7mpibHW/2znWAiAhKq5fCVdONB9YhvX/8y9XFuq5AwpKU3nmAWtqTULcJAEBAwQBAAAAIgYCyBe7dSGvw16pbzv7Jw5utQ3f+lVgYnuWH+wA8pllCL8IAAAAgAAAAAABEAT+////Ih0Cekh/wZ+3aYd7h0LW6hgRjzxOcrHqjG3mAqetSkHb4GghA+yk/xG3KOLg9gzmIilDpv9VudlfYnv5qZ0IS8hy1QpbIh4Cekh/wZ+3aYd7h0LW6hgRjzxOcrHqjG3mAqetSkHb4GhAihOzmFVF9yvW6JcUrrkJs+NUqEKpu4tWzQ7e0h34oZlZizEiikngvX6VzhBT98WyistUOmhwdgDjzomCLuMgIQABAwgYcwEAAAAAAAEEIlEgMm31D+Cge3rLcgcL6ztjLrmtFbppXMHlqmqgB7YUb9gBCUICekh/wZ+3aYd7h0LW6hgRjzxOcrHqjG3mAqetSkHb4GgDYeGx6d5eQssgB/fKVLng1X7ROTj61W0/GeV1E6j84DkA";

fn from_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

fn is_psbt_file(bytes: &[u8]) -> bool {
    classify(test_compute(), "a.psbt", bytes) == FileKind::Psbt
}

/// Invariant: A PSBT is a PSBT file only as the device reads one: version 0 or version 2, binary or base64, with no BIP-375 silent payment field and, for version 2, readable as version 0; any other falls through to text or other.
/// Regression: A version 0 PSBT with a silent payment field, a version 2 one with such a field, or a version 2 one that does not read as version 0 is listed as a PSBT the device then refuses to sign.
/// Owner: app
#[test]
fn a_psbt_the_device_refuses_is_not_a_psbt_file() {
    let c = test_compute();
    let v2_bytes = from_hex(PSBT_V2);
    let Ok(v2) = PsbtV2::parse_bytes(c, &v2_bytes) else {
        panic!("a valid version 2 PSBT");
    };
    let Ok(v0) = v2.to_v0(c) else {
        panic!("a version 0 form");
    };
    // As read: version 2 and version 0, binary and base64.
    for form in [
        v2_bytes.clone(),
        faraday_bip::base64::encode(&v2_bytes).into_bytes(),
        v0.to_bytes(),
        v0.to_base64().into_bytes(),
    ] {
        assert!(is_psbt_file(&form));
    }

    // A version 0 PSBT with one BIP-375 field, in each map.
    for (place, ty) in [
        ("global", 0x07),
        ("global", 0x08),
        ("input", 0x1d),
        ("input", 0x1e),
        ("output", 0x09),
        ("output", 0x0a),
    ] {
        let mut with = v0.clone();
        let unknown = match place {
            "global" => &mut with.inner_mut().unknown,
            "input" => &mut with.inner_mut().inputs[0].unknown,
            _ => &mut with.inner_mut().outputs[0].unknown,
        };
        unknown.insert(
            faraday_bip::bitcoin::psbt::raw::Key {
                type_value: ty,
                key: vec![],
            },
            vec![0],
        );
        for form in [with.to_bytes(), with.to_base64().into_bytes()] {
            assert!(Psbt::parse_bytes(c, &with.to_bytes()).is_ok());
            assert!(!is_psbt_file(&form), "v0 {place} {ty:#x}");
        }
    }

    // A version 2 PSBT with one BIP-375 field, and BIP-375's own vector.
    let mut with = v2.clone();
    with.global.set(vec![0x07], vec![0]);
    assert!(with.has_silent_fields());
    assert!(!is_psbt_file(&with.to_bytes()));
    let silent = faraday_bip::base64::decode(PSBT_SILENT);
    let Ok(silent) = silent else {
        panic!("a base64 vector");
    };
    for form in [silent.to_vec(), PSBT_SILENT.as_bytes().to_vec()] {
        assert!(!is_psbt_file(&form));
    }

    // A version 2 PSBT that parses and does not read as version 0: BIP-370's vector whose
    // inputs' locktimes cannot both hold.
    let unreadable = from_hex(PSBT_V2_NO_LOCKTIME);
    let Ok(parsed) = PsbtV2::parse_bytes(c, &unreadable) else {
        panic!("the vector parses as version 2");
    };
    assert!(!parsed.has_silent_fields() && parsed.to_v0(c).is_err());
    for form in [
        unreadable.clone(),
        faraday_bip::base64::encode(&unreadable).into_bytes(),
    ] {
        assert!(!is_psbt_file(&form));
    }
}
