//! Send to Faraday (`docs/COMPANION.md` §4.3, §11): a PSBT and a
//! description picked from a file come out as codes that the device's
//! own scanner (`faraday-core`'s) reads back as the same bytes, in every
//! format the screen offers; the screen stays on while they show; a file
//! that holds a secret is refused whole.

mod common;

use std::rc::Rc;

use common::*;
use faraday_companion::{Act, Companion, CompanionCommand, Page};
use faraday_core::wallet::FileKind as DeviceKind;
use faraday_core::{Action, Faraday, StorageEvent};
use osk_codec::QrMatrix;
use osk_shell_api::{App, Command, Event, FileKind};

/// What the device's scanner files from these codes, each read off the
/// phone's screen by its camera.
fn device_reads(frames: &[Rc<QrMatrix>]) -> Option<(DeviceKind, Vec<u8>)> {
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Scan);
    for f in frames {
        let (w, h, luma) = f.to_luma(3, osk_codec::qr::QUIET_ZONE);
        for d in osk_codec::decode_luma(w, h, &luma) {
            app.event(Event::Scanned { bytes: d.bytes });
        }
    }
    let item = app.inbox.last()?;
    Some((item.kind, item.bytes.clone()))
}

/// The phone with `bytes` picked from its files, on Send to Faraday.
fn picked(bytes: &[u8]) -> Companion {
    let mut c = phone();
    c.press(Act::Send);
    c.press(Act::Choose);
    let (cmds, _) = drain(&mut c);
    assert!(cmds.contains(&Command::RequestFile {
        kind: FileKind::Any
    }));
    c.event(Event::File {
        kind: FileKind::Any,
        bytes: bytes.to_vec(),
    });
    c
}

fn frames(c: &Companion) -> Vec<Rc<QrMatrix>> {
    c.sending.as_ref().expect("codes on show").frames.clone()
}

/// Every format the screen offers for what was picked, one code and
/// animated, read back by the device as `kind` and `bytes`.
fn read_back_in_every_form(c: &mut Companion, kind: DeviceKind, bytes: &[u8], formats: usize) {
    for _ in 0..formats {
        for _ in 0..2 {
            let shown = frames(c);
            let label = format!("{:?}", c.sending.as_ref().map(|s| s.made));
            assert_eq!(
                device_reads(&shown),
                Some((kind, bytes.to_vec())),
                "{label}"
            );
            if c.offers(Act::Animated) {
                c.press(Act::Animated);
            }
        }
        c.press(Act::Format);
    }
}

#[test]
fn a_psbt_picked_from_a_file_reads_back_on_the_device_as_the_same_psbt() {
    let psbt = psbt();
    let mut c = picked(&psbt);
    assert_eq!(c.page, Page::Send);
    // UR one code, UR animated, BBQr, BBQr animated.
    read_back_in_every_form(&mut c, DeviceKind::Psbt, &psbt, 2);
    // The same PSBT as base64 text.
    let text = osk_bip::base64::encode(&psbt);
    let mut c = picked(text.as_bytes());
    read_back_in_every_form(&mut c, DeviceKind::Psbt, &psbt, 1);
}

#[test]
fn a_description_picked_from_a_file_reads_back_on_the_device_as_the_same_text() {
    let d = description();
    let mut c = picked(d.as_bytes());
    assert_eq!(c.page, Page::Send);
    // UR, BBQr and numbered parts, each one code and animated.
    read_back_in_every_form(&mut c, DeviceKind::Wallet, d.as_bytes(), 3);
}

#[test]
fn a_kept_file_shows_as_codes_from_received() {
    let d = description();
    let mut c = receiving();
    show(&mut c, &code_of(d.as_bytes()));
    c.press(Act::Open);
    c.press(Act::ShowAsQr);
    assert_eq!(c.page, Page::Send);
    assert_eq!(
        device_reads(&frames(&c)),
        Some((DeviceKind::Wallet, d.into_bytes()))
    );
}

#[test]
fn the_screen_stays_on_while_a_code_shows() {
    let mut c = picked(&psbt());
    let (_, ours) = drain(&mut c);
    assert!(ours.contains(&CompanionCommand::KeepScreenOn { on: true }));
    c.press(Act::Back);
    let (_, ours) = drain(&mut c);
    assert!(ours.contains(&CompanionCommand::KeepScreenOn { on: false }));
    assert_eq!(c.page, Page::SendList);
}

#[test]
fn an_animated_code_moves_on_at_the_rate_chosen() {
    let mut c = picked(&psbt());
    c.press(Act::Animated);
    let n = frames(&c).len();
    assert!(n > 1);
    let at = |c: &Companion| c.sending.as_ref().map_or(0, |s| s.frame);
    c.event(Event::Tick { now_ms: 10_000 });
    let first = at(&c);
    c.event(Event::Tick { now_ms: 10_400 });
    assert_eq!(at(&c), first, "moved before 2 a second's 500 ms");
    c.event(Event::Tick { now_ms: 10_500 });
    assert_eq!(at(&c), (first + 1) % n);
    c.press(Act::Rate);
    c.event(Event::Tick { now_ms: 11_000 });
    let second = at(&c);
    c.event(Event::Tick { now_ms: 11_333 });
    assert_eq!(at(&c), (second + 1) % n, "3 a second is 333 ms a frame");
}

#[test]
fn a_file_too_large_for_the_envelope_is_not_sent() {
    let big: Vec<u8> = (0..300 * 1024u32).map(|i| (i % 251) as u8).collect();
    let c = picked(&big);
    assert_eq!(c.page, Page::NotSent(faraday_companion::NotSent::TooLarge));
    assert!(c.sending.is_none());
}

#[test]
fn a_file_that_holds_a_seed_is_refused_whole() {
    let words = b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\n";
    let c = picked(words);
    assert_eq!(c.page, Page::Refused(faraday_intake::Secret::Bip39Words));
    assert!(c.sending.is_none());
    assert!(
        c.texts()
            .iter()
            .any(|t| t == faraday_strings::EN.companion_refuse_words)
    );
}
