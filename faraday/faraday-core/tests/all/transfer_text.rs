//! Transfer's Send text in the online app: text typed or pasted is shown
//! as codes the device reads back as it was written, except a secret
//! (F236).

use faraday_core::scan_read::Kind;
use faraday_core::transfer::TextPress;
use faraday_core::wallet::Session;
use faraday_core::{Action, Faraday, QrFormat, Screen, testkit};
use faraday_draw::canvas::InkKind;
use faraday_qr::{Arrived, Assembler, Step};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, FileKind, SecureHardware};

/// The online app at 1280 × 800 on Transfer.
fn on_transfer() -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
    app.clipboard = true;
    app.session = Session::on(testkit::NET);
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::Transfer));
    app.settle();
    app
}

/// `text` as Paste brings it from the clipboard, then Show as QR.
fn shown(text: &str) -> Faraday {
    let mut app = on_transfer();
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: faraday_text::Text::of(text),
    });
    app.press(Action::TransferText(TextPress::Show));
    app
}

/// What the screen says now.
fn said(app: &mut Faraday) -> Vec<String> {
    app.drawn_ink()
        .iter()
        .filter_map(|i| match &i.kind {
            InkKind::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The text of every code the QR sheet shows, as a camera reads them.
fn read_frames(app: &Faraday) -> Vec<String> {
    let c = testkit::compute();
    app.qr()
        .expect("no QR sheet")
        .frames
        .iter()
        .map(|f| {
            let png = faraday_codec::png::qr_png(f, 4);
            let codes = faraday_files::qr_in_png(&c, &png).expect("a PNG");
            assert_eq!(codes.len(), 1);
            String::from_utf8(codes[0].clone()).expect("text")
        })
        .collect()
}

fn lower_first(s: &str) -> String {
    let mut it = s.chars();
    it.next()
        .map(|c| c.to_lowercase().chain(it).collect())
        .unwrap_or_default()
}

const HEADER: &str = "0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c";
const SEED: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const SEEDQR: &str = "000000000000000000000000000000000000000000000003";
const WIF: &str = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";
const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
const NSEC: &str = "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5";
const CODEX32: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";

#[test]
fn a_block_header_shows_as_one_code_the_device_reads_back_as_written() {
    let mut app = shown(HEADER);
    let frames = read_frames(&app);
    assert_eq!(frames, [HEADER]);
    assert_eq!(
        faraday_qr::classify_text(&frames[0]),
        Arrived::Text(HEADER.to_string())
    );
    app.press(Action::Cancel);
}

#[test]
fn text_typed_is_shown_as_it_is_with_its_ends_trimmed() {
    let mut app = on_transfer();
    for c in "  hello world \n".chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::TransferText(TextPress::Show));
    assert_eq!(read_frames(&app), ["hello world"]);
}

#[test]
fn a_hash_of_64_hex_digits_is_shown() {
    let hash = "00000000000000000003".to_string() + &"ab".repeat(22);
    assert_eq!(hash.len(), 64);
    let app = shown(&hash);
    assert_eq!(read_frames(&app), [hash]);
}

#[test]
fn each_secret_alone_is_refused_with_its_name() {
    for (text, kind) in [
        (SEED, Some(Kind::SeedWords)),
        (SEEDQR, None),
        (WIF, Some(Kind::Wif)),
        (XPRV, Some(Kind::Xprv)),
        (NSEC, None),
        (CODEX32, None),
    ] {
        let mut app = shown(text);
        assert!(app.qr().is_none(), "{text}: shown");
        let lines = said(&mut app);
        let line = match kind {
            Some(k) => format!("Not shown: this is {}", lower_first(k.label())),
            None => "Not shown: this is ".to_string(),
        };
        assert!(
            lines.iter().any(|l| l.starts_with(&line)),
            "{text}: not named in {lines:?}"
        );
    }
}

#[test]
fn each_secret_inside_a_sentence_is_refused_with_its_name() {
    for (text, kind) in [
        (SEED, Some(Kind::SeedWords)),
        (WIF, Some(Kind::Wif)),
        (XPRV, Some(Kind::Xprv)),
        (NSEC, None),
        (CODEX32, None),
        (SEEDQR, None),
    ] {
        let sentence = format!("Please keep this safe for me: {text} and thanks.");
        let mut app = shown(&sentence);
        assert!(app.qr().is_none(), "{text}: shown");
        let lines = said(&mut app);
        let line = match kind {
            Some(k) => format!("Not shown: it holds {}", lower_first(k.label())),
            None => "Not shown: it holds ".to_string(),
        };
        assert!(
            lines.iter().any(|l| l.starts_with(&line)),
            "{text}: not named in {lines:?}"
        );
    }
}

#[test]
fn text_over_16_kib_is_refused() {
    let long = "lorem ipsum ".repeat(1500);
    assert!(long.len() > 16 * 1024);
    let mut app = shown(&long);
    assert!(app.qr().is_none());
    assert!(said(&mut app).iter().any(|l| l.starts_with("Over ")));
}

/// 2 KiB of ordinary lines.
fn two_kib() -> String {
    let text: String = (0..100)
        .map(|i| format!("line {i}: block {:08x} ok\n", i * 7919))
        .collect();
    assert!(text.len() > 2000);
    text.trim().to_string()
}

#[test]
fn a_2_kib_text_goes_through_ur_bytes_into_the_devices_reader() {
    let text = two_kib();
    let app = shown(&text);
    let frames = read_frames(&app);
    assert!(frames.len() > 1, "animated");
    let mut decoder = faraday_codec::ur::Decoder::new();
    for f in &frames {
        decoder.receive(f).expect("a part");
        if decoder.is_complete() {
            break;
        }
    }
    assert!(decoder.is_complete());
    match decoder.into_message() {
        Some(faraday_codec::ur::Message::Bytes(b)) => assert_eq!(b, text.as_bytes()),
        other => panic!("not the text: {other:?}"),
    }
}

#[test]
fn a_2_kib_text_goes_through_bbqr_into_the_devices_assembler() {
    let text = two_kib();
    let mut app = shown(&text);
    let bbqr = QrFormat::ALL
        .iter()
        .position(|f| *f == QrFormat::Bbqr)
        .expect("BBQr");
    app.press(Action::QrFormat(u8::try_from(bbqr).unwrap()));
    let frames = read_frames(&app);
    assert!(frames.len() > 1, "animated");
    let mut assembler = Assembler::default();
    let mut done = None;
    for f in &frames {
        if let Step::Done(a) = assembler.feed(f) {
            done = Some(a);
            break;
        }
    }
    assert_eq!(done, Some(Arrived::Text(text)));
}

const OTPAUTH: &str = "otpauth://totp/Mail:me?secret=JBSWY3DPEHPK3PXP&issuer=Mail";
const MIGRATION: &str = "otpauth-migration://offline?data=CjEKCkhlbGxvId6tvu8SGEV4YW1wbGU6YWxpY2VAZXhhbXBsZS5jb20aB0V4YW1wbGUgASgBMAIQARgBIAAo2a";
const SLIP39: &str = "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed";
const PEM: &str = concat!(
    "-----BEGIN OPENSSH ",
    "PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmU=\n-----END OPENSSH ",
    "PRIVATE KEY-----"
);
const PGP: &str = concat!(
    "-----BEGIN PGP ",
    "PRIVATE KEY BLOCK-----\n\nlQOYBF==\n-----END PGP ",
    "PRIVATE KEY BLOCK-----"
);

/// Whether the text is refused, by the line under the field.
fn refused(text: &str) -> bool {
    let mut app = shown(text);
    let no_code = app.qr().is_none();
    let line = said(&mut app)
        .iter()
        .any(|l| l.starts_with("Not shown") || l.starts_with("Over "));
    assert_eq!(no_code, line, "{text}: code and line disagree");
    no_code
}

#[test]
fn more_secrets_are_refused_whole_and_inside_a_sentence() {
    let hd_private = {
        let mut cbor = faraday_qr::cbor::head(5, 1);
        cbor.extend(faraday_qr::cbor::head(0, 2));
        cbor.push(0xf5);
        faraday_codec::ur::encode_single("crypto-hdkey", &cbor)
    };
    let seed_ur = faraday_codec::ur::encode_single("seed", &[0xa0]);
    let crypto_seed_ur = faraday_codec::ur::encode_single("crypto-seed", &[0xa0]);
    for secret in [
        OTPAUTH,
        MIGRATION,
        PEM,
        PGP,
        SLIP39,
        &seed_ur,
        &crypto_seed_ur,
        &hd_private,
        &format!("wpkh({XPRV}/0/*)"),
        &format!("wpkh([d34db33f/84h/0h/0h]{XPRV}/0/*)"),
    ] {
        assert!(refused(secret), "{secret}: shown");
        assert!(
            refused(&format!("Please keep this safe: {secret} and thanks.")),
            "{secret}: shown in a sentence"
        );
        assert!(refused(&format!("\"{secret}\"")), "{secret}: shown quoted");
        assert!(
            refused(&format!("({secret})")),
            "{secret}: shown in brackets"
        );
    }
}

#[test]
fn a_secret_joined_to_its_name_or_label_is_refused() {
    let words = SEED;
    for text in [
        format!("MNEMONIC=\"{words}\""),
        format!("{{\"mnemonic\":\"{words}\"}}"),
        format!("seed={words}"),
        format!("p2wpkh:{WIF}"),
        format!("p2wpkh-p2sh:{WIF}"),
        format!("{{\"bc1qexample\":\"p2wpkh:{WIF}\"}}"),
        format!("nostr:{NSEC}"),
        format!("key:{XPRV}"),
        format!("url:{OTPAUTH}"),
        format!("url:{MIGRATION}"),
        format!(
            "ur:{{{}}}",
            faraday_codec::ur::encode_single("seed", &[0xa0]).as_str()
        ),
    ] {
        assert!(refused(&text), "{text}: shown");
    }
}

#[test]
fn a_secret_after_a_label_is_found_whatever_punctuation_is_round_it() {
    let hex = "0c1e24e5917779d297e14d45f14e1a1a";
    for value in [
        format!("p2wpkh:{WIF}"),
        format!("share:{CODEX32}"),
        format!("entropy:{hex}"),
        format!("key:{WIF}"),
    ] {
        for (before, after) in [
            ("`", "`"),
            ("", "."),
            ("", "!"),
            ("", "?"),
            ("*", "*"),
            ("_", "_"),
            ("", "."),
            ("(", ")."),
            ("\"", "\","),
        ] {
            let wrapped = format!("{before}{value}{after}");
            assert!(refused(&wrapped), "{wrapped}: shown");
            assert!(refused(&format!("Sweep {wrapped}")), "end: {wrapped}");
            assert!(
                refused(&format!("Sweep {wrapped} then wait for the rest.")),
                "middle: {wrapped}"
            );
        }
    }
}

#[test]
fn a_secret_one_character_off_is_refused() {
    for text in [
        // A WIF and a codex32 string with a changed last character, an
        // extended private key with a changed end, eleven and nineteen words.
        "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWm",
        "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlq",
        &XPRV.replace("33yuGBxrMPHi", "33yuGBxrMPHj"),
        &SEED.replacen(" about", "", 1),
        SLIP39.rsplit_once(' ').unwrap().0,
    ] {
        assert!(refused(text), "{text}: shown");
    }
}

#[test]
fn a_seed_with_a_word_misspelt_anywhere_or_in_four_letter_form_is_refused() {
    // One misspelt word, at each place of twelve.
    let words: Vec<&str> = SEED.split(' ').collect();
    for at in 0..words.len() {
        let mut typo = words.clone();
        let bad = format!("{}n", words[at]);
        typo[at] = &bad;
        let text = typo.join(" ");
        assert!(refused(&text), "{text}: shown");
        assert!(
            refused(&format!("Keep this for me: {text}, thanks.")),
            "{text}"
        );
    }
    assert!(refused(
        "abandon abandon abandon abandon abandon abandonn abandon abandon abandon abandon abandon about"
    ));
    // Twenty SLIP-39 words with one off the list, in the middle.
    let mut slip: Vec<&str> = SLIP39.split(' ').collect();
    slip[10] = "zzzzzz";
    assert!(refused(&slip.join(" ")));
    // The first four letters of each word, as on a steel plate.
    assert!(refused(
        "aban aban aban aban aban aban aban aban aban aban aban abou"
    ));
    assert!(refused(
        "Aban ABAN aban aban aban aban aban aban aban aban aban ABOU and"
    ));
    // Ordinary sentences are shown.
    assert!(!refused(
        "The quick brown fox jumps over the lazy dog while the rest of us wait for the bus to town."
    ));
    assert!(!refused(
        "the cat sat on the mat and the dog ran to a hat of the man"
    ));
    assert!(!refused(
        "aban aban aban aban aban aban aban aban aban aban aban"
    ));
}

#[test]
fn bare_hex_of_entropy_or_a_seed_length_is_refused_and_64_digits_is_shown() {
    let hex = |n: usize| "a1b2c3d4".repeat(n / 8 + 1)[..n].to_string();
    for n in [32, 40, 48, 56, 128] {
        assert!(refused(&hex(n)), "{n} hex digits shown");
        assert!(refused(&format!("0x{}", hex(n))), "0x{n} shown");
    }
    for n in [31, 64, 66, 100] {
        assert!(!refused(&hex(n)), "{n} hex digits refused");
    }
}

#[test]
fn a_numbered_file_of_seed_words_is_refused_as_seed_words() {
    // A numbered file of words reads as seed words, not as a private key.
    let file: String = SEED
        .split(' ')
        .enumerate()
        .map(|(i, w)| format!("{}. {w}\n", i + 1))
        .collect();
    let mut app = shown(&format!("# my seed\n{file}"));
    assert!(app.qr().is_none());
    assert!(
        said(&mut app)
            .iter()
            .any(|l| l.starts_with("Not shown: this is a seed's words")),
        "{:?}",
        said(&mut app)
    );
}

#[test]
fn a_secret_in_a_urs_parts_is_refused_until_the_message_is_whole_and_read() {
    let secret = format!("my key is p2wpkh:{WIF}");
    let mut enc = faraday_codec::ur::Encoder::bytes(secret.as_bytes(), 20).unwrap();
    assert!(enc.fragment_count() > 2);
    let parts: Vec<String> = (0..enc.fragment_count())
        .map(|_| String::from(&*enc.next_part()))
        .collect();
    // A part, then all but one: the message is not whole.
    assert!(refused(&parts[0]));
    assert!(refused(&parts[..parts.len() - 1].join(" ")));
    // All of them: the text inside is read, and it holds a key.
    assert!(refused(&parts.join("\n")));
    // A text with nothing secret in it is shown.
    let mut enc =
        faraday_codec::ur::Encoder::bytes(b"a block header, nothing secret in it", 20).unwrap();
    let parts: Vec<String> = (0..enc.fragment_count())
        .map(|_| String::from(&*enc.next_part()))
        .collect();
    assert!(!refused(&parts.join("\n")));
}

#[test]
fn a_ur_that_can_hold_a_secret_is_refused_whole_or_in_parts_unless_known_public() {
    // A whole eckey (a private key) and a whole sskr share.
    let mut eckey = vec![0xa2, 0x02, 0xf5, 0x03, 0x58, 0x20];
    eckey.extend([7u8; 32]);
    assert!(refused(&faraday_codec::ur::encode_single(
        "crypto-eckey",
        &eckey
    )));
    assert!(refused(&faraday_codec::ur::encode_single("eckey", &eckey)));
    assert!(refused(&faraday_codec::ur::encode_single(
        "crypto-sskr",
        &[0x58, 0x04, 1, 2, 3, 4]
    )));
    // Parts of a type nobody here knows (an envelope), one and all but one.
    let mut enc = faraday_codec::ur::Encoder::bytes(&[9u8; 100], 20).unwrap();
    let parts: Vec<String> = (0..enc.fragment_count())
        .map(|_| enc.next_part().replacen("ur:bytes/", "ur:envelope/", 1))
        .collect();
    assert!(parts.len() > 2 && parts[0].starts_with("ur:envelope/"));
    assert!(refused(&parts[0]));
    assert!(refused(&parts[..parts.len() - 1].join(" ")));
    // Parts of a PSBT are public: shown.
    let mut enc = faraday_codec::ur::Encoder::psbt(&[9u8; 100], 20).unwrap();
    let psbt_parts: Vec<String> = (0..enc.fragment_count())
        .map(|_| String::from(&*enc.next_part()))
        .collect();
    assert!(!refused(&psbt_parts[0]));
}

#[test]
fn a_zero_width_space_or_soft_hyphen_does_not_hide_a_secret() {
    for gap in ["\u{200B}", "\u{00AD}"] {
        assert!(refused(&WIF.replacen("Ki", &format!("K{gap}i"), 1)));
        assert!(refused(&SEED.replacen(
            "abandon abandon",
            &format!("aban{gap}don abandon"),
            1
        )));
        assert!(refused(&format!("{gap}{XPRV}")));
    }
}

#[test]
fn seedqr_digits_in_groups_of_four_are_refused() {
    let groups = "0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0003";
    assert!(refused(groups));
    assert!(refused(&format!("code: {}", groups.replace(' ', "\n"))));
    assert!(!refused("0000 0000 0000 0000 0003"));
}

#[test]
fn a_paste_over_what_the_field_holds_is_refused_not_cut() {
    let long = format!("{}\n{SEED}", " ".repeat(70 * 1024));
    let mut app = shown(&long);
    assert!(app.qr().is_none());
    assert!(said(&mut app).iter().any(|l| l.starts_with("Over ")));
    let text = format!("{}{HEADER}", " ".repeat(70 * 1024));
    assert!(refused(&text));
}

#[test]
fn text_changed_after_a_show_is_checked_again_at_the_next() {
    let mut app = on_transfer();
    for c in "hello".chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::TransferText(TextPress::Show));
    assert!(app.qr().is_some());
    app.press(Action::Cancel);
    app.press(Action::TransferText(TextPress::Clear));
    for c in SEED.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::TransferText(TextPress::Show));
    assert!(app.qr().is_none());
}
#[test]
fn seed_words_of_every_list_in_any_form_are_refused_alone_and_in_a_sentence() {
    let mut forms = crate::common::seed_forms();
    for run in [crate::common::KOREAN_RUN, crate::common::FRENCH_RUN] {
        forms.push(("run".into(), "composed", run.to_string()));
    }
    // Japanese words are separated by the ideographic space, as BIP-39
    // gives them.
    {
        use faraday_bip::bip39::{Language, Mnemonic};
        let m = Mnemonic::from_entropy(Language::Japanese, &[0x5a; 16]).expect("entropy");
        let words: Vec<&str> = m
            .indices()
            .iter()
            .map(|&i| Language::Japanese.word_display(i))
            .collect();
        forms.push((
            "Japanese".into(),
            "ideographic space",
            words.join("\u{3000}"),
        ));
    }
    for (lang, form, text) in forms {
        // A valid seed of a list is named a seed; a run that is no seed is
        // named by its checksum.
        let valid = lang != "run";
        let mut app = shown(&text);
        assert!(app.qr().is_none(), "{lang} {form}: shown alone");
        assert!(
            said(&mut app).iter().any(|l| if valid {
                l.starts_with("Not shown: this is a seed's words")
            } else {
                l.starts_with("Not shown: this is a seed's words")
                    || l.starts_with("Not shown: this is words that fail")
            }),
            "{lang} {form}: not named alone"
        );
        let mut app = shown(&format!(
            "Please keep this safe for me: {text}, and thanks."
        ));
        assert!(app.qr().is_none(), "{lang} {form}: shown in a sentence");
        assert!(
            said(&mut app).iter().any(|l| if valid {
                l.starts_with("Not shown: it holds a seed's words")
            } else {
                l.starts_with("Not shown: it holds a seed's words")
                    || l.starts_with("Not shown: it holds words that fail")
            }),
            "{lang} {form}: not named in a sentence"
        );
    }
}
