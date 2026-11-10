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
        text: text.to_string(),
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
