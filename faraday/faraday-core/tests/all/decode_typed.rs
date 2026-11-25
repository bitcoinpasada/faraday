//! Decode a transaction's Type it: text typed or pasted as hex or base64
//! is read as a file of the same bytes is, and nothing a file would not be
//! taken is taken from it.

use crate::common::{has, has_part, lock, offered, texts, through_fade, type_text};
use faraday_core::decode::{TYPED_MAX, TypedPress};
use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, FileKind, Key, SecureHardware};

/// The first transaction ever sent, as the hex of its bytes.
const TX: &str = "0100000001c997a5e56e104102fa209c6a852dd90660a20b2d9c352423edce25857fcd3704000000004847304402204e45e16932b8af514961a1d3a1a25fdf3f4f7732e9d624c6c61548ab5fb8cd410220181522ec8eca07de4860a4acdd12909d831cc56cbbac4622082221a8768d1d0901ffffffff0200ca9a3b00000000434104ae1a62fe09c5f51b13905f07f06b99a2f7159b2225f374cd378d71302fa28414e7aab37397f554a7df5f142c21c1b7303b8a0626f1baded5c72a704f7e6cd84cac00286bee0000000043410411db93e1dcdb8a016b49840f8c53bc1eb68a382e97b1482ecad7b148a6909a5cb2e0eaddfb84ccf9744464f82e160bfa9b8b64f9d4c03f999b8643f656b412a3ac00000000";

/// The start of the hex, short enough to stand on the field's first line.
const HEAD: &str = "0100000001c997";

/// Whether the end of `TX` is drawn: the field shows the lines at the
/// caret, which stands at the end of what was typed.
fn shown(app: &mut Faraday) -> bool {
    texts(app).concat().contains(&TX[TX.len() - 20..])
}

fn psbt_b64() -> String {
    let path = format!(
        "{}/../../tools/vectors/psbt/bump-core-original.psbt",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn size(app: &mut Faraday, width: u32, height: u32) {
    app.event(Event::Display(DisplayInfo {
        width: width as u16,
        height: height as u16,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// Decode opened from Tools, on a window `width` wide, with `files` held.
fn open_decode(files: &[(&str, &[u8])], width: u32, height: u32) -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, faraday_bip::keys::Network::Mainnet).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files
                .iter()
                .map(|(n, b)| (faraday_text::Text::of(n), b.to_vec()))
                .collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    size(&mut app, width, height);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == EN.catalog_decode_a_transaction)
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    through_fade(&mut app);
    assert_eq!(app.screen, Screen::Decode);
    app
}

fn typed_in(app: &mut Faraday, text: &str) {
    app.press(Action::DecodeTyped(TypedPress::Open));
    through_fade(app);
    type_text(app, text);
    app.press(Action::DecodeTyped(TypedPress::Next));
    through_fade(app);
}

/// What a decode says of the transaction, the source line apart.
fn values(app: &Faraday) -> String {
    let d = &app.decode().expect("a transaction is shown").decoded;
    format!(
        "{:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?}",
        d.txid, d.wtxid, d.version, d.locktime, d.size, d.inputs, d.outputs, d.fee
    )
}

fn as_file(name: &str, bytes: &[u8]) -> String {
    let mut app = open_decode(&[(name, bytes)], 1366, 4000);
    app.press(Action::DecodeInbox(0));
    through_fade(&mut app);
    values(&app)
}

#[test]
fn typed_hex_decodes_as_a_file_of_the_same_bytes() {
    let mut app = open_decode(&[], 1366, 4000);
    typed_in(&mut app, TX);
    assert_eq!(values(&app), as_file("tx.txt", TX.as_bytes()));
    assert!(
        app.decode()
            .unwrap()
            .decoded
            .source
            .contains(EN.decode_typed_source)
    );
}

#[test]
fn a_typed_base64_psbt_decodes_as_a_file_of_the_same_bytes() {
    let text = psbt_b64();
    let mut app = open_decode(&[], 1366, 4000);
    typed_in(&mut app, text.trim());
    assert_eq!(values(&app), as_file("p.psbt", text.as_bytes()));
}

#[test]
fn a_pasted_text_is_read_as_a_typed_one() {
    let mut app = open_decode(&[], 1366, 4000);
    app.clipboard = true;
    app.press(Action::DecodeTyped(TypedPress::Open));
    app.event(Event::Key(faraday_shell_api::Key::Paste));
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: faraday_text::Text::of(&format!("{TX}\n")),
    });
    app.press(Action::DecodeTyped(TypedPress::Next));
    assert_eq!(values(&app), as_file("tx.txt", TX.as_bytes()));
}

/// Refused as the file of those bytes is: not a transaction, nothing shown.
fn refused(text: &str) {
    let mut app = open_decode(&[], 1366, 4000);
    typed_in(&mut app, text);
    assert!(app.decode().is_none(), "{text:.20} was decoded");
    assert!(has(&mut app, EN.compare_not_a_transaction));
    let kind = wallet_kind(text.as_bytes());
    assert!(
        !matches!(
            kind,
            faraday_core::wallet::FileKind::Psbt | faraday_core::wallet::FileKind::Transaction
        ),
        "a file of it is a transaction"
    );
    assert_eq!(app.inbox().iter().count(), 0, "put in Files");
    assert!(app.outbox.is_empty(), "put in the outbox");
    // The file of the same bytes opens nothing in Decode.
    let mut file = open_decode(&[("x.txt", text.as_bytes())], 1366, 4000);
    file.press(Action::DecodeInbox(0));
    assert!(file.decode().is_none());
}

fn wallet_kind(bytes: &[u8]) -> faraday_core::wallet::FileKind {
    faraday_core::wallet::classify(&testkit::compute(), "x.txt", bytes)
}

#[test]
fn garbage_is_refused_as_a_file_is() {
    refused("hello there");
}

#[test]
fn a_truncated_transaction_is_refused_as_a_file_is() {
    refused(&TX[..TX.len() - 20]);
}

#[test]
fn hex_with_a_stray_character_is_refused_as_a_file_is() {
    let mut bad = TX.to_string();
    bad.insert(40, 'g');
    refused(&bad);
}

#[test]
fn a_vault_a_descriptor_a_key_and_seed_words_are_refused_as_a_file_is() {
    refused(&String::from_utf8_lossy(&testkit::test_vault().unwrap()));
    let descriptor = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
        .descriptor;
    refused(&descriptor);
    refused(
        "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi",
    );
    refused(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
}

/// Pasted, `text` is read as a file of its bytes is: the same or the same
/// refusal, and nothing put in Files or the outbox.
fn read_as_a_file_is(text: &str) {
    let mut app = open_decode(&[], 1366, 4000);
    app.press(Action::DecodeTyped(TypedPress::Open));
    paste(&mut app, text);
    app.press(Action::DecodeTyped(TypedPress::Next));
    through_fade(&mut app);
    let typed = app.decode().map(|_| values(&app));
    assert_eq!(app.inbox().iter().count(), 0);
    assert!(app.outbox.is_empty());
    let mut file = open_decode(&[("x.txt", text.as_bytes())], 1366, 4000);
    file.press(Action::DecodeInbox(0));
    through_fade(&mut file);
    assert_eq!(typed, file.decode().map(|_| values(&file)), "{text:.20?}");
}

#[test]
fn a_bom_crlf_and_upper_case_are_read_as_a_file_is() {
    read_as_a_file_is(&format!("\u{feff}{TX}"));
    read_as_a_file_is(&format!("{TX}\r\n"));
    read_as_a_file_is(&format!("{}\r\n{}", &TX[..40], &TX[40..]));
    read_as_a_file_is(&TX.to_uppercase());
}

#[test]
fn nothing_typed_is_refused() {
    let mut app = open_decode(&[], 1366, 4000);
    typed_in(&mut app, "");
    assert!(app.decode().is_none());
    assert!(has(&mut app, EN.compare_not_a_transaction));
}

/// `text` pasted into the open field, as the clipboard gives it.
fn paste(app: &mut Faraday, text: &str) {
    app.clipboard = true;
    app.event(Event::Key(Key::Paste));
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: faraday_text::Text::of(text),
    });
}

fn key(app: &mut Faraday, k: Key) {
    app.event(Event::Key(k));
}

/// A transaction of about 980 000 characters of hex (a data output), and
/// the same padded with spaces to exactly what the field holds.
fn big_tx() -> (String, String) {
    use faraday_bip::bitcoin::{
        Amount, ScriptBuf, Transaction, TxIn, TxOut, absolute::LockTime, consensus::encode,
        transaction::Version,
    };
    let data = 490_000u32;
    let mut script = vec![0x6a, 0x4e];
    script.extend(data.to_le_bytes());
    script.extend(std::iter::repeat_n(0x5a, data as usize));
    let tx = Transaction {
        version: Version(2),
        lock_time: LockTime::ZERO,
        input: vec![TxIn::default()],
        output: vec![TxOut {
            value: Amount::ZERO,
            script_pubkey: ScriptBuf::from_bytes(script),
        }],
    };
    let hex = encode::serialize_hex(&tx);
    assert!(hex.len() < TYPED_MAX);
    let padded = format!("{hex}{}", " ".repeat(TYPED_MAX - hex.len()));
    (hex, padded)
}

#[test]
fn a_text_of_exactly_the_limit_draws_takes_a_caret_edit_and_is_read() {
    let (hex, padded) = big_tx();
    assert_eq!(padded.len(), TYPED_MAX);
    let mut app = open_decode(&[], 1366, 768);
    app.press(Action::DecodeTyped(TypedPress::Open));
    paste(&mut app, &padded);
    assert!(app.decode().is_none());
    assert!(!has(&mut app, EN.decode_typed_too_large));
    // Only the lines that show are drawn, the caret's (the last) among them.
    let drawn: usize = texts(&mut app).iter().map(|t| t.len()).sum();
    assert!(drawn < 20_000, "{drawn} characters drawn");
    // A caret edit at the start: the first character out and in again.
    key(&mut app, Key::Home);
    assert!(has_part(&mut app, &hex[..14]));
    key(&mut app, Key::Delete);
    assert!(!has_part(&mut app, &hex[..14]));
    key(&mut app, Key::Char(hex.chars().next().unwrap()));
    assert!(has_part(&mut app, &hex[..14]));
    assert!(app.decode().is_none());
    key(&mut app, Key::Enter);
    through_fade(&mut app);
    let read = values(&app);
    drop(app);
    let mut file = open_decode(&[("tx.txt", padded.as_bytes())], 1366, 768);
    file.press(Action::DecodeInbox(0));
    through_fade(&mut file);
    assert_eq!(read, values(&file));
}

#[test]
fn a_text_one_over_the_limit_is_refused_and_what_was_typed_stays() {
    let mut app = open_decode(&[], 1366, 4000);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, TX);
    paste(&mut app, &"0".repeat(TYPED_MAX + 1));
    assert!(app.decode().is_none());
    assert!(has(&mut app, EN.decode_typed_too_large));
    assert!(shown(&mut app));
}

#[test]
fn back_from_the_result_keeps_the_text() {
    let mut app = open_decode(&[], 1366, 4000);
    typed_in(&mut app, TX);
    assert!(app.decode().is_some());
    app.press(Action::DecodeAnother);
    through_fade(&mut app);
    assert!(app.decode().is_none());
    assert!(shown(&mut app));
}

#[test]
fn text_left_without_next_is_not_kept() {
    let mut app = open_decode(&[], 1366, 4000);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, TX);
    assert!(shown(&mut app));
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::DecodeTyped(TypedPress::Open));
    app.press(Action::Nav(Screen::Decode));
    assert!(!shown(&mut app));
}

#[test]
fn lock_clears_the_text_and_the_result() {
    let mut app = open_decode(&[], 1366, 4000);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, TX);
    assert!(shown(&mut app));
    lock(&mut app);
    // Looked at where the lock left it, on Decode, not after leaving it.
    assert_eq!(app.screen, Screen::Decode);
    assert!(!shown(&mut app));
    assert!(!offered(&mut app, Action::DecodeTyped(TypedPress::Next)));

    let mut app = open_decode(&[], 1366, 4000);
    typed_in(&mut app, TX);
    assert!(app.decode().is_some());
    lock(&mut app);
    assert_eq!(app.screen, Screen::Decode);
    assert!(app.decode().is_none());
    assert!(!shown(&mut app));
    // The field comes back empty.
    app.press(Action::DecodeTyped(TypedPress::Open));
    assert!(!shown(&mut app));
}

#[test]
fn enter_reads_the_text_as_next_does() {
    let mut app = open_decode(&[], 1366, 4000);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, TX);
    key(&mut app, Key::Enter);
    through_fade(&mut app);
    assert_eq!(values(&app), as_file("tx.txt", TX.as_bytes()));
}

#[test]
fn a_sheet_over_decode_takes_the_keys_and_the_paste_and_keeps_the_text() {
    let mut app = open_decode(&[], 480, 640);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, HEAD);
    app.press(Action::Guide);
    through_fade(&mut app);
    key(&mut app, Key::Enter);
    paste(&mut app, TX);
    assert!(app.decode().is_none());
    app.press(Action::GuideClose);
    through_fade(&mut app);
    assert!(has_part(&mut app, HEAD));
    assert!(!shown(&mut app));
    assert!(app.decode().is_none());
}

#[test]
fn the_guide_pane_on_a_section_leaves_the_keyboard_and_the_paste_to_the_field() {
    let mut app = open_decode(&[], 2560, 1440);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, HEAD);
    app.press(Action::Guide);
    through_fade(&mut app);
    paste(&mut app, TX);
    assert!(shown(&mut app));
}

#[test]
fn the_guide_find_in_the_pane_has_the_keyboard_and_the_paste() {
    let mut app = open_decode(&[], 2560, 1440);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, HEAD);
    app.press(Action::Guide);
    app.press(Action::GuideContents);
    through_fade(&mut app);
    paste(&mut app, TX);
    type_text(&mut app, "zz");
    assert!(has_part(&mut app, HEAD));
    assert!(!shown(&mut app));
    app.press(Action::GuideClose);
    through_fade(&mut app);
    assert!(has_part(&mut app, HEAD));
    assert!(!has_part(&mut app, "zz"));
}

#[test]
fn pasting_nothing_leaves_the_text_and_says_so() {
    let mut app = open_decode(&[], 1366, 4000);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, TX);
    paste(&mut app, "");
    assert!(shown(&mut app));
    assert!(has(&mut app, EN.pasted_nothing));
    key(&mut app, Key::Enter);
    through_fade(&mut app);
    assert_eq!(values(&app), as_file("tx.txt", TX.as_bytes()));
}

/// The text pasted with `tail` after it, an edit made inside it at its
/// start (the first character out, a character in), and nothing read yet.
fn edited_in_a_paste(tail: &str, how: impl Fn(&mut Faraday)) -> (Faraday, String) {
    let pasted = format!("{TX}{tail}");
    let mut app = open_decode(&[], 1366, 4000);
    app.press(Action::DecodeTyped(TypedPress::Open));
    paste(&mut app, &pasted);
    key(&mut app, Key::Home);
    how(&mut app);
    (app, pasted)
}

#[test]
fn an_edit_inside_a_pasted_text_with_line_breaks_changes_only_the_text() {
    for tail in ["\n", "\r\n", "\r\n\r\n"] {
        // Deleted and typed again: the same bytes, nothing read by itself.
        let (mut app, pasted) = edited_in_a_paste(tail, |app| {
            key(app, Key::Delete);
            key(app, Key::Char('0'));
        });
        assert!(app.decode().is_none(), "{tail:?}: read by itself");
        assert!(has_part(&mut app, HEAD), "{tail:?}");
        app.press(Action::DecodeTyped(TypedPress::Next));
        through_fade(&mut app);
        assert_eq!(
            values(&app),
            as_file("tx.txt", pasted.as_bytes()),
            "{tail:?}"
        );
        // One character out is a text a file of it would refuse, and the
        // breaks are kept.
        let (mut app, _) = edited_in_a_paste(tail, |app| key(app, Key::Delete));
        assert!(app.decode().is_none(), "{tail:?}: read by itself");
        assert!(!has_part(&mut app, HEAD), "{tail:?}");
        key(&mut app, Key::Char('0'));
        app.press(Action::DecodeTyped(TypedPress::Next));
        through_fade(&mut app);
        assert_eq!(
            values(&app),
            as_file("tx.txt", pasted.as_bytes()),
            "{tail:?}"
        );
    }
}

#[test]
fn the_text_is_edited_at_the_caret_in_the_middle() {
    let mut app = open_decode(&[], 1366, 4000);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, &format!("{}ab{}", &TX[..100], &TX[100..]));
    key(&mut app, Key::Home);
    for _ in 0..100 {
        key(&mut app, Key::Right);
    }
    key(&mut app, Key::Delete);
    key(&mut app, Key::Delete);
    assert!(app.decode().is_none());
    key(&mut app, Key::Enter);
    through_fade(&mut app);
    assert_eq!(values(&app), as_file("tx.txt", TX.as_bytes()));
}

#[test]
fn type_it_is_offered_at_compact_1366_and_1920() {
    for (w, h) in [(480, 640), (1366, 768), (1920, 1080)] {
        // None waiting, and two waiting (one would open on itself).
        let two = [("a.txt", TX.as_bytes()), ("b.txt", TX.as_bytes())];
        for files in [&[][..], &two[..]] {
            let mut app = open_decode(files, w, h);
            assert!(
                offered(&mut app, Action::DecodeTyped(TypedPress::Open)),
                "{w}x{h}"
            );
        }
    }
}

#[test]
fn a_result_has_a_way_back_that_is_never_decode_itself() {
    // Another held file chosen in the pane over a file's result.
    let two = [("a.txt", TX.as_bytes()), ("b.txt", TX.as_bytes())];
    let mut app = open_decode(&two, 1366, 768);
    app.press(Action::DecodeInbox(0));
    through_fade(&mut app);
    app.press(Action::DecodeInbox(1));
    through_fade(&mut app);
    assert_ne!(app.decode().unwrap().back, Screen::Decode);
    // Type it from a result, read again.
    let mut app = open_decode(&[], 1366, 768);
    typed_in(&mut app, TX);
    app.press(Action::DecodeTyped(TypedPress::Open));
    through_fade(&mut app);
    app.press(Action::DecodeTyped(TypedPress::Next));
    through_fade(&mut app);
    assert_ne!(app.decode().unwrap().back, Screen::Decode);
}

#[test]
fn type_it_is_reached_by_scrolling_under_many_waiting_files_on_a_small_panel() {
    let names: Vec<String> = (0..16).map(|k| format!("{k}.txt")).collect();
    let files: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), TX.as_bytes())).collect();
    let mut app = open_decode(&files, 480, 640);
    let _ = app.frame();
    let (w, h) = app.size();
    let open = Action::DecodeTyped(TypedPress::Open);
    for _ in 0..12 {
        if app
            .where_offered(open)
            .is_some_and(|(_, y)| f32::from(y) < h)
        {
            break;
        }
        app.event(Event::Scroll {
            x: (w / 2.0) as u16,
            y: (h / 2.0) as u16,
            dy: 120,
        });
        let _ = app.frame();
    }
    let y = app.where_offered(open).expect("offered").1;
    assert!(f32::from(y) < h, "Type it is below the window at {y}");
}

#[test]
fn with_the_keyboard_up_on_a_small_panel_the_field_and_next_can_be_reached_above_it() {
    let names: Vec<String> = (0..16).map(|k| format!("{k}.txt")).collect();
    let files: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), TX.as_bytes())).collect();
    let mut app = open_decode(&files, 480, 640);
    app.press(Action::DecodeTyped(TypedPress::Open));
    type_text(&mut app, HEAD);
    let _ = app.frame();
    let top = app.keyboard_top().expect("the keyboard is up");
    let (w, h) = app.size();
    let next = Action::DecodeTyped(TypedPress::Next);
    for _ in 0..12 {
        if app.where_offered(next).is_some_and(|(_, y)| y < top) {
            break;
        }
        app.event(Event::Scroll {
            x: (w / 2.0) as u16,
            y: (h / 4.0) as u16,
            dy: 120,
        });
        let _ = app.frame();
    }
    let y = app.where_offered(next).expect("Next is offered").1;
    assert!(y < top, "Next at {y} is under the keyboard at {top}");
}

#[test]
fn the_character_at_the_caret_is_drawn_wherever_it_stands_in_a_wrapped_text() {
    let mut app = open_decode(&[], 1366, 768);
    app.press(Action::DecodeTyped(TypedPress::Open));
    through_fade(&mut app);
    type_text(&mut app, TX);
    key(&mut app, Key::Home);
    for k in 0..TX.len() - 8 {
        let drawn = texts(&mut app).concat();
        // The stretch of the hex drawn: from its first 16 characters on.
        let from = (0..TX.len() - 16)
            .find(|&s| drawn.contains(&TX[s..s + 16]))
            .expect("some of the hex is drawn");
        let mut to = from + 16;
        while to < TX.len() && drawn.contains(&TX[from..to + 1]) {
            to += 1;
        }
        assert!(from <= k && k < to, "the caret at {k} is out of sight");
        key(&mut app, Key::Right);
    }
}

#[test]
fn a_paste_puts_the_caret_at_its_end_and_selects_nothing() {
    let mut app = open_decode(&[], 1366, 768);
    app.press(Action::DecodeTyped(TypedPress::Open));
    through_fade(&mut app);
    type_text(&mut app, HEAD);
    key(&mut app, Key::Home);
    app.event(Event::Shift { held: true });
    key(&mut app, Key::End);
    app.event(Event::Shift { held: false });
    paste(&mut app, TX);
    key(&mut app, Key::Char('f'));
    assert!(
        texts(&mut app)
            .concat()
            .contains(&format!("{}f", &TX[TX.len() - 10..]))
    );
}
