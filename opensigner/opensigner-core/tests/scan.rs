//! The scanner and the QR outputs through the shell contract
//! (`docs/UX.md` §4 routing table, §5 secret frame, F4; `docs/PLANNING.md`
//! §4.2 camera channel). Frames are built from the codec's own encoder,
//! and what the app draws is read back through its framebuffer and the
//! decoder: the encoder, the widget and the decoder agree on real pixels.

mod common;

use common::{ABANDON, Harness, PANEL, PHONE};
use opensigner_core::backup::BackupStep;
use opensigner_core::load::Step;
use opensigner_core::scan::{CameraRotation, ScanStage};
use opensigner_core::sign::{QrMode, Stage, Step as SignStep};
use opensigner_core::verify::{AddressResult, VerifyStage};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_codec::qr::{Ecc, Payload, QUIET_ZONE, QrMatrix};
use osk_codec::{PayloadKind, seedqr, ur};
use osk_psbt::Psbt;
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, Event, FileKind, SecureHardware, TouchPhase,
};

const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");

fn mnemonic() -> Mnemonic {
    Mnemonic::parse(Language::English, &ABANDON.join(" ")).unwrap()
}

fn demo_psbt_bytes() -> Vec<u8> {
    Psbt::parse_base64(std::str::from_utf8(DEMO).unwrap().trim())
        .unwrap()
        .to_bytes()
}

/// A camera frame picturing `matrix` at 4 px per module.
fn frame_of(matrix: &QrMatrix) -> Event {
    let (w, h, luma) = matrix.to_luma(4, QUIET_ZONE);
    Event::CameraFrame {
        width: w as u16,
        height: h as u16,
        luma,
        chroma: None,
    }
}

fn frame_bytes(bytes: &[u8]) -> Event {
    frame_of(&osk_codec::qr::encode(Payload::Bytes(bytes), Ecc::Low).unwrap())
}

/// The framebuffer as luma, for decoding what is on screen.
fn screen_luma(h: &mut Harness) -> (usize, usize, Vec<u8>) {
    let f = h.app.frame();
    let luma = f
        .rgba
        .chunks_exact(4)
        .map(|p| {
            ((u32::from(p[0]) * 299 + u32::from(p[1]) * 587 + u32::from(p[2]) * 114) / 1000) as u8
        })
        .collect();
    (usize::from(f.width), usize::from(f.height), luma)
}

/// Decodes the one QR code on screen.
fn decode_screen(h: &mut Harness) -> Vec<u8> {
    let (w, hh, luma) = screen_luma(h);
    let found = osk_codec::decode_luma(w, hh, &luma);
    assert_eq!(found.len(), 1, "one code on screen");
    found.into_iter().next().unwrap().bytes
}

impl Harness {
    fn open_scan(&mut self) {
        let (x, y) = self.center(ids::HOME_SCAN);
        self.app.event(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        self.app.event(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
        let commands = self.drain();
        assert!(commands.contains(&Command::CameraOn), "{commands:?}");
        assert_eq!(self.app.screen(), ScreenKind::Scan);
        assert_eq!(self.app.scan_stage(), Some(ScanStage::Camera));
    }

    /// Sends a frame, then the code a shell reads from it, and returns
    /// the commands they produced. A shell decodes its own frames and
    /// hands the core the payload (`Event::Scanned`); this stands in
    /// for the worker a real shell runs.
    fn frame(&mut self, event: Event) -> Vec<Command> {
        let read = match &event {
            Event::CameraFrame {
                width,
                height,
                luma,
                ..
            } => {
                opensigner_core::scan::decode_frame(usize::from(*width), usize::from(*height), luma)
            }
            _ => None,
        };
        self.app.event(event);
        if let Some(bytes) = read {
            self.app.event(Event::Scanned { bytes });
        }
        self.drain()
    }

    fn load_key(&mut self) {
        self.start_load(&ABANDON);
        self.finish_load(None);
    }

    fn regtest(&mut self) {
        self.set_network(3);
        assert_eq!(self.app.network(), Network::Regtest);
    }
}

#[test]
fn seedqr_and_compact_frames_land_on_the_confirm_with_a_language_row() {
    for compact in [false, true] {
        let mut h = Harness::new(PANEL);
        h.open_scan();
        let m = mnemonic();
        let matrix = if compact {
            seedqr::encode_compact(&m).unwrap()
        } else {
            seedqr::encode_seedqr(&m).unwrap()
        };
        let commands = h.frame(frame_of(&matrix));
        assert!(commands.contains(&Command::CameraOff), "{commands:?}");
        assert_eq!(h.app.screen(), ScreenKind::Load);
        // A decoded seed code has a valid checksum by construction, so
        // neither the language step nor a separate checksum screen is a
        // wall between the scan and the fingerprint (UX review
        // 2026-09-07, Q4).
        assert_eq!(
            h.app.load_step(),
            Some(Step::Checksum),
            "the language step is skipped"
        );
        assert!(h.app.load_suspects().is_empty());
        // The language is still one tap away, because it changes the key.
        assert!(
            h.app.rect_of(ids::LOAD_LANG_OTHER).is_some(),
            "the word-language row is on the confirm"
        );
        h.tap(ids::LOAD_LANG_OTHER);
        assert_eq!(h.app.load_step(), Some(Step::Language));
        h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
        assert_eq!(h.app.load_step(), Some(Step::Checksum));
        h.finish_load(None);
        assert_eq!(h.app.fingerprints()[0].to_hex(), *b"73c5da0a");
    }
}

#[test]
fn a_bad_seedqr_is_reported_and_scanning_continues() {
    let mut h = Harness::new(PANEL);
    h.open_scan();
    // All-zero digits: "abandon" ×12 fails the checksum.
    h.frame(frame_of(
        &osk_codec::qr::encode(Payload::Numeric(&[b'0'; 48]), Ecc::Low).unwrap(),
    ));
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Camera));
    assert!(h.app.scan_error().unwrap().contains("SeedQR"));
    // A good code afterwards still routes.
    h.frame(frame_of(&seedqr::encode_seedqr(&mnemonic()).unwrap()));
    assert_eq!(h.app.screen(), ScreenKind::Load);
}

#[test]
fn psbt_frame_opens_the_sign_summary() {
    let mut h = Harness::new(PANEL);
    h.load_key();
    h.regtest();
    h.open_scan();
    h.frame(frame_bytes(DEMO.trim_ascii()));
    assert_eq!(h.app.screen(), ScreenKind::Sign);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Summary)));
    assert_eq!(h.app.sign_inspection().unwrap().participating_keys.len(), 1);
    // Back from the summary lands on Home: there is nothing behind a
    // transaction that arrived, and the scanner is not on the stack.
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.sign_stage(), None);
}

#[test]
fn address_frame_opens_verify_with_the_result() {
    let mut h = Harness::new(PANEL);
    h.load_key();
    let ours = h.app.addresses(0, ScriptType::Taproot, true, 8)[7].clone();
    h.open_scan();
    h.frame(frame_bytes(ours.as_bytes()));
    assert_eq!(h.app.screen(), ScreenKind::Verify);
    assert_eq!(h.app.verify_stage(), Some(VerifyStage::Result));
    assert_eq!(
        h.app.verify_result(),
        Some(&AddressResult::Yours {
            fingerprint: h.app.fingerprints()[0],
            script: ScriptType::Taproot,
            change: true,
            index: 7,
        })
    );
    // Another seed's address: valid but not ours.
    let other = Mnemonic::from_entropy(Language::English, &[7u8; 16]).unwrap();
    let master = MasterKey::from_seed(&other.to_seed(b"").unwrap(), Network::Mainnet);
    let theirs = master
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .to_string();
    h.tap(ids::VERIFY_CLEAR);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.frame(frame_bytes(theirs.as_bytes()));
    assert_eq!(h.app.verify_result(), Some(&AddressResult::NotFound));
    // Our own first signet address, while the setting is mainnet.
    let signet = MasterKey::from_seed(&mnemonic().to_seed(b"").unwrap(), Network::Signet)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .to_string();
    assert!(signet.starts_with("tb1q"), "{signet}");
    h.tap(ids::VERIFY_CLEAR);
    h.frame(frame_bytes(signet.as_bytes()));
    assert_eq!(h.app.verify_result(), Some(&AddressResult::WrongNetwork));
}

#[test]
fn typed_address_is_checked() {
    let mut h = Harness::new(PHONE);
    h.load_key();
    let ours = h.app.addresses(0, ScriptType::NativeSegwit, false, 1)[0].clone();
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::SCAN_TYPE);
    assert_eq!(h.app.verify_stage(), Some(VerifyStage::Typing));
    h.type_text(&ours);
    h.key(osk_shell_api::Key::Enter);
    // The key's SegWit wallet is registered, so the address is that
    // wallet's.
    assert_eq!(
        h.app.verify_result(),
        Some(&AddressResult::Wallet {
            wallet: 0,
            change: false,
            index: 0,
        })
    );
    // The result's address is a reference row: the whole of it is one
    // tap away, on the Compare screen (§4.5).
    h.tap(ids::VERIFY_ADDRESS);
    assert!(h.app.texts().iter().any(|t| t.replace(' ', "") == ours));
    h.tap(ids::COMPARE_DONE);
    // §4.3: ✓ is dead while the text is not an address, so nothing is
    // checked and the caption line says so. The keyboard takes only
    // characters that can still lead to an address, so what stands in
    // the field is the beginning of one and stops short.
    h.tap(ids::VERIFY_CLEAR);
    h.tap(ids::SCAN_TYPE);
    h.type_text("bc1q");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.verify_stage(), Some(VerifyStage::Typing));
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.verify_invalid_title),
        "the caption line says it is not an address"
    );
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
}

#[test]
fn descriptor_and_xpub_frames_open_the_wallet_or_the_document() {
    let s = &strings::EN;
    let says = |h: &Harness, want: &str, screen: &str| {
        let texts = h.app.texts();
        assert!(
            texts.iter().any(|t| t == want),
            "{screen} never says {want:?}: {texts:?}"
        );
    };
    let mut h = Harness::new(PANEL);
    h.load_key();
    let master = MasterKey::from_seed(&mnemonic().to_seed(b"").unwrap(), Network::Mainnet);
    let account = master.account_xpub(ScriptType::NativeSegwit, 0).unwrap();
    h.open_scan();
    h.frame(frame_bytes(account.descriptor().as_bytes()));
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    // A single-key descriptor is a wallet of one key, so it is offered
    // as one rather than read as a document (§16.72).
    assert_eq!(h.app.inspect_title(), Some(s.wallet_title));
    says(&h, s.wallet_use, "the descriptor");
    // §5 Record: the checksum, what it pays to, the keys it names, and
    // the descriptor itself one tap away.
    says(&h, s.inspect_checksum, "the descriptor");
    says(&h, s.inspect_checksum_ok, "the descriptor");
    says(&h, s.script_type_row, "the descriptor");
    says(&h, s.script_segwit, "the descriptor");
    says(&h, s.row_keys, "the descriptor");
    says(&h, "73c5da0a", "the descriptor");
    // §4.4's glyph, not the MINE badge: the key row says that this
    // device holds the private key.
    assert!(
        h.app
            .row_glyphs()
            .iter()
            .any(|(label, icon)| label == s.sign_key_row && *icon == osk_ui::widgets::Icon::Wallet),
        "the key row carries the key glyph: {:?}",
        h.app.row_glyphs()
    );
    assert!(
        !h.app.texts().iter().any(|t| t == "MINE"),
        "the badge is for outputs and addresses"
    );
    // §4.5: the reference row opens the Compare screen, as structure.
    h.tap(ids::INSPECT_TEXT);
    assert!(h.app.rect_of(ids::COMPARE_KEY).is_some(), "the key inside");
    h.tap(ids::COMPARE_KEY);
    assert!(h.app.rect_of(ids::COMPARE_TEXT).is_some(), "the key whole");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);

    h.open_scan();
    h.frame(frame_bytes(account.xpub_string().as_bytes()));
    // A bare key says nothing about what it pays to, so the Choice asks
    // before the wallet it would make exists; the chevron leaves the key
    // as the document it is.
    says(&h, s.script_type_title, "the public key");
    h.tap(ids::BACK);
    assert_eq!(h.app.inspect_title(), Some(s.inspect_xpub));
    says(&h, s.confirm_network, "the public key");
    says(&h, Network::Mainnet.name(), "the public key");
    says(&h, s.row_mine, "the public key");
    says(&h, s.value_yes, "the public key");
    h.tap(ids::INSPECT_DONE);
    assert_eq!(h.app.screen(), ScreenKind::Home);

    // A descriptor whose checksum was tampered with, and a key no
    // loaded key derives.
    let mut bad = account.descriptor();
    let last = bad.pop().expect("a checksum");
    bad.push(if last == 'q' { 'p' } else { 'q' });
    h.open_scan();
    h.frame(frame_bytes(bad.as_bytes()));
    says(&h, s.inspect_checksum_bad, "a tampered descriptor");
    h.tap(ids::BACK);
    let other = MasterKey::from_seed(&mnemonic().to_seed(b"other").unwrap(), Network::Mainnet)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap();
    h.open_scan();
    h.frame(frame_bytes(other.xpub_string().as_bytes()));
    h.tap(ids::BACK);
    says(&h, s.value_no, "a public key of another seed");
}

#[test]
fn words_frame_shows_the_caution_then_load() {
    let mut h = Harness::new(PANEL);
    h.open_scan();
    h.frame(frame_bytes(ABANDON.join(" ").as_bytes()));
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert_eq!(h.app.scan_stage(), Some(ScanStage::WordsCaution));
    // Back returns to scanning; the code again, then Continue.
    h.tap(ids::BACK);
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Camera));
    h.frame(frame_bytes(ABANDON.join(" ").as_bytes()));
    h.tap(ids::SCAN_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Load);
    // Only the English list holds every one of these words, so the
    // language step is not asked (UX review 2026-09-07, Q4).
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
    h.finish_load(None);
    assert_eq!(h.app.fingerprints()[0].to_hex(), *b"73c5da0a");
}

#[test]
fn unknown_frame_offers_treat_as() {
    let mut h = Harness::new(PANEL);
    h.open_scan();
    h.frame(frame_bytes(b"hello, world"));
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Unknown));
    assert!(h.app.rect_of(ids::SCAN_AS_PSBT).is_some());
    h.tap(ids::SCAN_AS_TEXT);
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    assert_eq!(h.app.inspect_title(), Some(strings::EN.inspect_text));
    h.tap(ids::BACK);
    h.open_scan();
    h.frame(frame_bytes(b"hello, world"));
    h.tap(ids::SCAN_AS_PSBT);
    assert_eq!(h.app.screen(), ScreenKind::Sign);
    assert_eq!(h.app.sign_stage(), Some(Stage::ParseError));
}

#[test]
fn camera_unavailable_offers_the_file_fallback() {
    let mut h = Harness::new(PANEL);
    h.load_key();
    h.regtest();
    h.open_scan();
    h.send(Event::CameraUnavailable);
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Unavailable));
    let (x, y) = h.center(ids::SCAN_FILE);
    h.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    h.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    let commands = h.drain();
    assert!(
        commands.contains(&Command::RequestFile {
            kind: FileKind::Any
        }),
        "{commands:?}"
    );
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Waiting));
    // Closing the picker is not a failure: the scanner goes back to what
    // it was doing and says nothing about it (§2.1).
    h.send(Event::FileCancelled {
        kind: FileKind::Any,
    });
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Unavailable));
    assert!(h.app.scan_error().is_none());
    // A device with no file channel leaves the same screen behind.
    h.tap(ids::SCAN_FILE);
    h.send(Event::FileUnavailable {
        kind: FileKind::Any,
    });
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Unavailable));
    assert!(h.app.scan_error().is_none());
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Sign);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Summary)));
    // Leaving a scanner whose camera never came sends no CameraOff.
    let mut h = Harness::new(PANEL);
    h.open_scan();
    h.send(Event::CameraUnavailable);
    let (x, y) = h.center(ids::BACK);
    h.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    h.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    let commands = h.drain();
    assert!(!commands.contains(&Command::CameraOff), "{commands:?}");
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

#[test]
fn multipart_ur_over_three_frames_shows_progress_then_routes() {
    let mut h = Harness::new(PANEL);
    h.load_key();
    h.regtest();
    let psbt = demo_psbt_bytes();
    // A fragment length that splits this transaction into three parts.
    let mut enc = ur::Encoder::psbt(&psbt, 140).unwrap();
    assert_eq!(enc.fragment_count(), 3);
    let parts: Vec<Event> = (0..3)
        .map(|_| {
            let text = enc.next_part().to_ascii_uppercase();
            frame_of(
                &osk_codec::qr::encode(Payload::Alphanumeric(text.as_bytes()), Ecc::Low).unwrap(),
            )
        })
        .collect();
    h.open_scan();
    // Out of order: the last part first.
    h.frame(parts[2].clone());
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert_eq!(h.app.scan_progress(), Some((1, 3)));
    // The same part again changes nothing.
    h.frame(parts[2].clone());
    assert_eq!(h.app.scan_progress(), Some((1, 3)));
    h.frame(parts[0].clone());
    assert_eq!(h.app.scan_progress(), Some((2, 3)));
    let commands = h.frame(parts[1].clone());
    assert!(commands.contains(&Command::CameraOff), "{commands:?}");
    assert_eq!(h.app.screen(), ScreenKind::Sign);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Summary)));
}

#[test]
fn scan_rows_inside_load_and_sign_open_the_scanner() {
    let mut h = Harness::new(PANEL);
    h.open_load();
    assert_eq!(h.app.load_step(), Some(Step::Source));
    // The source step is a choice: the tap checks the scan row and
    // Continue opens the scanner.
    h.tap(ids::LOAD_SOURCE_SCAN);
    assert_eq!(h.app.screen(), ScreenKind::Load, "the tap only checks");
    h.tap(ids::LOAD_SOURCE_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Keys, "back where Load started");
    h.load_key();
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.key(osk_shell_api::Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
}

#[test]
fn sign_result_qr_pages_cycle_ur_parts_on_tick_and_decode() {
    let mut h = Harness::new(PHONE);
    h.load_key();
    h.regtest();
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    for _ in 0..4 {
        h.tap(ids::SIGN_CONTINUE);
    }
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Result)));
    h.tap(ids::SIGN_QR);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Qr)));
    // §4.9: the square is the class's side, and at that side one code
    // for this transaction is denser than a camera resolves, so the page
    // opens on the animated parts and the toggle is dimmed with the
    // reason rather than offering a code that cannot be read.
    assert_eq!(h.app.sign_qr_mode(), Some(QrMode::Ur));
    assert_eq!(h.app.qr_visible(), Some(true));
    assert!(
        h.app.rect_of(ids::SIGN_QR_ANIMATED).is_none(),
        "a forced toggle is not a control"
    );
    assert!(
        h.app.texts().iter().any(|t| t == strings::EN.sign_qr_dense),
        "and it says why: {:?}",
        h.app.texts()
    );
    let (part, total) = h.app.sign_qr_part().unwrap();
    assert_eq!(part, 1);
    assert!(total >= 2, "{total}");
    // Parts advance every 250 ms, not before.
    h.tick(100);
    assert_eq!(h.app.sign_qr_part(), Some((1, total)));
    h.tick(200);
    assert_eq!(h.app.sign_qr_part(), Some((2, total)));
    // Collect the frames off the screen until the decoder completes.
    let mut d = ur::Decoder::new();
    let mut decoded_parts = 0;
    while !d.is_complete() && decoded_parts < 3 * total {
        let shown = decode_screen(&mut h);
        let text = std::str::from_utf8(&shown).unwrap().to_owned();
        assert!(text.starts_with("UR:CRYPTO-PSBT/"), "{text}");
        d.receive(&text).unwrap();
        decoded_parts += 1;
        h.tick(250);
    }
    let Some(ur::Message::Psbt(bytes)) = d.into_message() else {
        panic!("a PSBT message");
    };
    let signed = Psbt::parse_bytes(&bytes).unwrap();
    assert!(osk_psbt::is_complete(&signed));
    // Back returns to the result and drops the page.
    h.tap(ids::BACK);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Result)));
    assert_eq!(h.app.sign_qr_mode(), None);
}

/// A seed code is a secret, so its screens offer no picture of it
/// (`docs/PLANNING.md` §16.134 rule 4): not while it is hidden, not while
/// it is held open, and not on CompactSeedQR.
#[test]
fn a_seed_code_offers_no_png() {
    for row in [ids::BACKUP_SEEDQR, ids::BACKUP_COMPACT] {
        let mut h = Harness::new(PANEL);
        h.load_key();
        h.open_key(0);
        h.tap(ids::DETAIL_BACKUP);
        h.tap(row);
        assert_eq!(h.app.screen(), ScreenKind::Backup);
        let no_row = |h: &Harness| {
            h.app.rect_of(ids::SAVE_PNG).is_none()
                && !h
                    .app
                    .texts()
                    .iter()
                    .any(|t| t == strings::EN.action_save_png)
        };
        assert!(no_row(&h), "hidden: {:?}", h.app.texts());
        let point = h.press(ids::CREATE_REVEAL);
        h.tick(100);
        assert_eq!(h.app.qr_visible(), Some(true));
        assert!(no_row(&h), "held open: {:?}", h.app.texts());
        h.release(point);
    }
}

#[test]
fn seedqr_secret_frame_is_blank_until_held_on_every_size() {
    for display in [PANEL, PHONE] {
        let mut h = Harness::new(display);
        h.load_key();
        h.open_key(0);
        h.tap(ids::DETAIL_BACKUP);
        h.tap(ids::BACKUP_SEEDQR);
        assert_eq!(h.app.screen(), ScreenKind::Backup);
        assert_eq!(h.app.backup_step(), Some(BackupStep::SeedQr));
        assert_eq!(h.app.qr_visible(), Some(false), "blank until held");
        let (w, hh, luma) = screen_luma(&mut h);
        assert!(
            osk_codec::decode_luma(w, hh, &luma).is_empty(),
            "nothing to scan"
        );
        let point = h.press(ids::CREATE_REVEAL);
        h.tick(100);
        assert_eq!(h.app.qr_visible(), Some(true));
        let shown = decode_screen(&mut h);
        let expected = seedqr::to_digits(&mnemonic());
        assert_eq!(shown, expected.expose().as_bytes());
        h.release(point);
        assert_eq!(h.app.qr_visible(), Some(false));
        // §5 Secret has no bottom action: the chevron is the way out.
        h.tap(ids::BACK);
        assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
        h.tap(ids::BACKUP_COMPACT);
        assert_eq!(h.app.backup_step(), Some(BackupStep::CompactSeedQr));
        let point = h.press(ids::CREATE_REVEAL);
        h.tick(100);
        let shown = decode_screen(&mut h);
        let entropy = mnemonic().entropy();
        assert_eq!(shown, entropy.expose().as_bytes());
        h.release(point);
        assert_eq!(h.app.qr_visible(), Some(false));
    }
}

#[test]
fn export_shows_a_descriptor_or_key_qr_that_decodes() {
    let mut h = Harness::new(PANEL);
    h.load_key();
    h.open_single_sig(0);
    h.tap(ids::WALLET_EXPORT);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    // The menu states what the export is; the last row opens the QR
    // screen a coordinator is shown (§5 QR).
    h.tap(ids::EXPORT_SHOW);
    assert_eq!(h.app.qr_visible(), Some(true), "public: never blank");
    let shown = decode_screen(&mut h);
    assert_eq!(
        osk_codec::classify(&shown),
        PayloadKind::Descriptor {
            checksum: Some(true)
        }
    );
    assert!(shown.starts_with(b"wpkh([73c5da0a/84h/0h/0h]"));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Export, "Back leaves the code");
    // A wallet pays to the script type it was added at, so the Taproot
    // addresses of the same key are a wallet of their own.
    h.go_home();
    h.add_single_sig(0, 3);
    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::EXPORT_SHOW);
    let shown = decode_screen(&mut h);
    assert!(shown.starts_with(b"tr([73c5da0a/86h/0h/0h]"));
    h.tap(ids::BACK);
    // SLIP-132 has no Taproot form, so its row is not on the list and
    // Continue cannot land on it.
    h.tap(ids::EXPORT_FORMAT);
    assert!(h.app.rect_of(ids::at(ids::PICK_BASE, 2)).is_none());
    h.choose(ids::at(ids::PICK_BASE, 1), ids::PICK_CONTINUE);
    h.tap(ids::EXPORT_SHOW);
    let shown = decode_screen(&mut h);
    assert_eq!(osk_codec::classify(&shown), PayloadKind::Xpub);
    h.tap(ids::BACK);
    // The export string is a reference row: the whole of it is on the
    // Compare screen (§4.5).
    h.tap(ids::EXPORT_TEXT);
    assert!(h.app.rect_of(ids::COMPARE_TEXT).is_some());
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
}

/// `docs/DESIGN.md` §4.9: "The square shows the camera's latest frame
/// ... scaled to cover the square and cropped to it." A frame that
/// decodes nothing still redraws the preview, which is the whole point:
/// the user aims the camera by what the square shows.
#[test]
fn a_frame_draws_itself_inside_the_square() {
    const GREY: u8 = 160;
    let mut h = Harness::new(PANEL);
    h.open_scan();
    assert_eq!(h.app.scan_preview(), None, "nothing before the first frame");
    let square = h.app.rect_of(ids::VIEWFINDER).expect("the viewfinder");
    let (w, _, before) = screen_luma(&mut h);
    let at = |luma: &[u8], x: i32, y: i32| luma[y as usize * w + x as usize];
    let middle = (square.x + square.w / 2, square.y + square.h / 3);
    assert_ne!(at(&before, middle.0, middle.1), GREY, "an empty square");

    h.frame(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: vec![GREY; 640 * 480],
        chroma: None,
    });
    assert_eq!(
        h.app.scan_stage(),
        Some(ScanStage::Camera),
        "a frame with no QR keeps scanning"
    );
    assert_eq!(
        h.app.scan_preview(),
        Some((320, 240)),
        "reduced so the shorter side is under the maximum"
    );
    let (_, _, after) = screen_luma(&mut h);
    assert_eq!(
        at(&after, middle.0, middle.1),
        GREY,
        "the frame fills the square"
    );
    assert_ne!(
        at(&after, square.x + 1, square.y + 1),
        GREY,
        "the corner bracket is drawn over it"
    );
}

/// §4.9: the square shows the frame "in colour where the camera gives
/// colour and in grey where it does not". The chroma reaches the drawn
/// square, and the same luma without it stays grey.
#[test]
fn a_frame_with_chroma_draws_the_square_in_colour() {
    // BT.601 limited-range red: luma 81, Cb 90, Cr 240.
    let (width, height) = (640usize, 480usize);
    let pairs = (width / 2) * (height / 2);
    let mut h = Harness::new(PANEL);
    h.open_scan();
    let square = h.app.rect_of(ids::VIEWFINDER).expect("the viewfinder");
    // A third of the way down, clear of the state band at the bottom.
    let (x, y) = (square.x + square.w / 2, square.y + square.h / 3);
    let centre = |h: &mut Harness| {
        let f = h.app.frame();
        let at = (y as usize * usize::from(f.width) + x as usize) * 4;
        (f.rgba[at], f.rgba[at + 1], f.rgba[at + 2])
    };

    h.frame(Event::CameraFrame {
        width: width as u16,
        height: height as u16,
        luma: vec![81; width * height],
        chroma: Some([90u8, 240].repeat(pairs)),
    });
    let (r, g, b) = centre(&mut h);
    assert!(r > 200 && g < 60 && b < 60, "red, not grey: {r},{g},{b}");

    h.frame(Event::CameraFrame {
        width: width as u16,
        height: height as u16,
        luma: vec![81; width * height],
        chroma: None,
    });
    assert_eq!(centre(&mut h), (81, 81, 81), "grey without chroma");

    // A chroma plane that is not the one this luma calls for is dropped
    // rather than half-drawn.
    h.frame(Event::CameraFrame {
        width: width as u16,
        height: height as u16,
        luma: vec![81; width * height],
        chroma: Some(vec![90; 4]),
    });
    assert_eq!(centre(&mut h), (81, 81, 81));
}

/// A frame may picture a SeedQR, so the preview lives exactly as long as
/// the scanner does: it goes when the camera turns out to be absent,
/// when the screen is left, and it is never kept off the Scan screen.
#[test]
fn the_preview_lives_only_while_the_scanner_does() {
    let frame = |h: &mut Harness| {
        h.frame(Event::CameraFrame {
            width: 320,
            height: 240,
            luma: vec![120; 320 * 240],
            chroma: None,
        });
    };

    let mut h = Harness::new(PANEL);
    h.open_scan();
    frame(&mut h);
    assert!(h.app.scan_preview().is_some());
    h.send(Event::CameraUnavailable);
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Unavailable));
    assert_eq!(h.app.scan_preview(), None, "no camera, no preview");

    let mut h = Harness::new(PANEL);
    h.open_scan();
    frame(&mut h);
    let (x, y) = h.center(ids::BACK);
    h.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    h.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    let commands = h.drain();
    assert!(commands.contains(&Command::CameraOff), "{commands:?}");
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.scan_preview(), None, "left with the screen");

    // A frame that arrives after the camera was told to stop is dropped
    // whole: it is not decoded and it is not kept.
    frame(&mut h);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.scan_preview(), None, "nothing stored off the screen");
}

/// §4.9: the camera-rotation setting turns every frame before the
/// preview and the decode, because a device someone built mounts its
/// camera whichever way the case allows.
#[test]
fn the_camera_rotation_setting_turns_every_frame() {
    let mut h = Harness::new(PANEL);
    h.set_setting(
        ids::SETTINGS_CAMERA_ROTATION_ROW,
        ids::SETTINGS_CAMERA_ROTATION_BASE,
        1,
    );
    assert_eq!(h.app.camera_rotation(), CameraRotation::Deg90);
    let texts = h.app.texts();
    let ninety = strings::EN.settings_camera_rotation_90;
    assert!(texts.iter().any(|t| t == ninety), "{texts:?}");
    h.tap(ids::BACK);

    h.open_scan();
    // A landscape frame reaches the preview as a portrait one: the turn
    // happened before anything read it.
    h.frame(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: vec![90; 640 * 480],
        chroma: None,
    });
    assert_eq!(h.app.scan_preview(), Some((240, 320)));

    // And the decoder reads the turned frame, not the one that arrived.
    let matrix = osk_codec::qr::encode(Payload::Bytes(b"hello, world"), Ecc::Low).unwrap();
    let (w, hh, luma) = matrix.to_luma(4, QUIET_ZONE);
    let (w, hh, luma) = opensigner_core::scan::rotate(w, hh, &luma, CameraRotation::Deg270);
    h.frame(Event::CameraFrame {
        width: w,
        height: hh,
        luma,
        chroma: None,
    });
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Unknown), "it decoded");
}

/// §4.9: a device whose camera the platform keeps upright has no
/// rotation to apply, so a settings card carried over from a device
/// whose camera was mounted sideways leaves its frames as they came.
#[test]
fn a_fixed_camera_leaves_every_frame_as_it_came() {
    let mut h = Harness::new(DisplayInfo {
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
        ..PANEL
    });
    h.send(Event::Settings {
        bytes: b"opensigner-settings 1\ncamera_rotation=270\nfirst_run_done=on\n".to_vec(),
    });
    assert_eq!(h.app.camera_rotation(), CameraRotation::Deg0);

    h.open_scan();
    h.frame(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: vec![90; 640 * 480],
        chroma: None,
    });
    assert_eq!(h.app.scan_preview(), Some((320, 240)));
}

/// §4.9: a code is the shell's to read, and the core routes what the
/// shell hands it exactly as it routed what it used to find itself. A
/// frame is the preview and nothing else, so a frame picturing a code
/// routes nothing on its own, and a code that arrives once the scanner
/// has been left is ignored, as a frame is.
#[test]
fn a_code_the_shell_read_routes_and_a_frame_alone_does_not() {
    let matrix = osk_codec::qr::encode(Payload::Bytes(b"hello, world"), Ecc::Low).unwrap();
    let (w, hh, luma) = matrix.to_luma(4, QUIET_ZONE);

    let mut h = Harness::new(PANEL);
    h.open_scan();
    h.app.event(Event::CameraFrame {
        width: w as u16,
        height: hh as u16,
        luma: luma.clone(),
        chroma: None,
    });
    h.drain();
    assert_eq!(
        h.app.scan_stage(),
        Some(ScanStage::Camera),
        "the frame is the preview; nothing was read from it"
    );
    assert_eq!(h.app.scan_preview(), Some((w as u16, hh as u16)));

    h.send(Event::Scanned {
        bytes: b"hello, world".to_vec(),
    });
    assert_eq!(
        h.app.scan_stage(),
        Some(ScanStage::Unknown),
        "the code the shell read is routed"
    );

    // Off the scan screen a code is dropped, as a frame is: the camera
    // has been told to stop and a frame may still be in flight.
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    h.send(Event::Scanned {
        bytes: b"hello, world".to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Home, "ignored off the scanner");
}
