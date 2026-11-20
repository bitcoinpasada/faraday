//! Two codex32 shares of BIP 93's vector 2 typed back on the backup's
//! codex32 item, checked and counted towards the rebuild of the key: no
//! plain copy of either string, or of its data symbols, is freed.

use crate::common::type_text;
use crate::{arm, finish, finish_left};
use faraday_bip::keys::ScriptType;
use faraday_core::codex32_backup::Press;
use faraday_core::{Action, Faraday, StorageEvent, plan, plan::Item, testkit};
use faraday_keys::texts::Spec;
use faraday_leak_scan::Form;
use faraday_shell_api::{App, EntropyBytes, Event, Key};
use faraday_ui::keyboard::OskPress;
use faraday_vault::codex32::{SetFacts, State};

pub(crate) const A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
pub(crate) const C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// What a share is held as: its text, the text after `ms1` as typed on
/// the codex32 keyboard, and its data symbols (5-bit values).
pub(crate) fn forms_of(name: &'static str, s: &str) -> [Form; 3] {
    let typed = &s[3..];
    let symbols = typed
        .bytes()
        .map(|b| {
            CHARSET
                .iter()
                .position(|&c| c == b)
                .expect("a bech32 character") as u8
        })
        .collect();
    [
        Form::new(name, s.as_bytes().to_vec()),
        Form::new("share text after ms1", typed.as_bytes().to_vec()),
        Form::new("share data symbols", symbols),
    ]
}

/// Vector 2's key loaded from A and C, its set told, and Back up opened
/// on a single-key wallet over it.
pub(crate) fn key_and_backup(app: &mut Faraday) {
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    for s in [A, C] {
        type_text(app, s);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    app.session.keys.set_codex32(
        0,
        Some(SetFacts {
            threshold: 2,
            identifier: *b"name",
            length: 48,
            unknown: 0,
            strings: vec![
                (b'a', State::Made),
                (b'c', State::Made),
                (b'd', State::Made),
            ],
        }),
    );
    let c = testkit::compute();
    let text = app
        .session
        .keys
        .key_text(&c, 0, &Spec::Single(ScriptType::NativeSegwit, 0))
        .unwrap()
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("shares-wallet.txt"),
            format!("wpkh({}/<0;1>/*)", &*text).into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(app, 0);
    app.press(Action::Backup(0));
}

/// As [`key_and_backup`], the codex32 item open.
fn at_the_item() -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    key_and_backup(&mut app);
    app.press(Action::BChecklist);
    let n = Item::Codex32(0);
    if app.backup().unwrap().open == Some(Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    if app.backup().unwrap().open != Some(n) {
        app.press(Action::BStep(n));
    }
    app
}

fn type_back(app: &mut Faraday, s: &str) {
    for ch in s[3..].chars() {
        app.press(Action::BC32(Press::Key(OskPress::Char(ch))));
    }
}

#[test]
fn shares_typed_back_and_counted_towards_the_rebuild_leave_no_plain_copy() {
    let mut app = at_the_item();
    let mut forms = Vec::new();
    forms.extend(forms_of("share A text", A));
    forms.extend(forms_of("share C text", C));
    let armed = arm(&forms);
    app.press(Action::BC32(Press::Open(
        plan::codex32::index_of(b'a').unwrap() as u8,
    )));
    type_back(&mut app, A);
    app.press(Action::BC32(Press::Next));
    type_back(&mut app, C);
    app.press(Action::BC32(Press::Next));
    assert_eq!(
        app.backup().unwrap().c32.as_ref().unwrap().rebuilds,
        Some(true),
        "the two shares rebuild the key"
    );
    finish("codex32 shares typed back", armed, false);
}

/// A backup is a flow in progress (`docs/FLOWS.md`, Flows in progress):
/// the arrow, Escape and the mouse's back leave its screen and keep what
/// was typed back until the flow is cancelled or Lock. Cancel ends it.
#[test]
fn shares_typed_back_are_gone_once_the_backup_is_cancelled() {
    let mut app = at_the_item();
    let mut forms = Vec::new();
    forms.extend(forms_of("share A text", A));
    forms.extend(forms_of("share C text", C));
    let armed = arm(&forms);
    app.press(Action::BC32(Press::Open(
        plan::codex32::index_of(b'a').unwrap() as u8,
    )));
    type_back(&mut app, A);
    app.press(Action::BC32(Press::Next));
    type_back(&mut app, C);
    app.press(Action::BC32(Press::Next));
    app.press(Action::CancelFlow);
    app.press(Action::CancelFlowNow);
    let _ = app.frame();
    assert!(app.backup().is_none(), "Cancel did not end the backup");
    finish_left("codex32 shares, backup cancelled", armed, &[]);
}
