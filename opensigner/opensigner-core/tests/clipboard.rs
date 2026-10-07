//! The clipboard: what a person can paste into the device, what they
//! can copy out of it, and what the device refuses either way.

mod common;

use common::{ABANDON, Harness, PANEL, PHONE};
use opensigner_core::tools::Tool;
use opensigner_core::{ScreenKind, ids, strings};
use osk_shell_api::{Command, Event, FileKind};

/// BIP-32 test vector 1's master extended public key.
const XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";

/// Home › Tools.
fn open_tools(h: &mut Harness) {
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    assert_eq!(h.app.screen(), ScreenKind::Tools);
}

/// Taps "Paste" on the scanner and hands the shell's answer back.
fn paste(h: &mut Harness, text: &str) {
    h.seen.clear();
    h.tap(ids::SCAN_PASTE);
    assert!(
        h.seen
            .iter()
            .any(|c| matches!(c, Command::RequestClipboard { .. })),
        "Paste asks the shell for the clipboard"
    );
    h.send(Event::Clipboard {
        kind: FileKind::Text,
        text: String::from(text),
    });
}

/// Opens one calculator's scanner.
fn open_tool(h: &mut Harness, tool: Tool) {
    let index = Tool::ALL.iter().position(|t| *t == tool).expect("a tool");
    open_tools(h);
    h.tap(ids::at(ids::TOOLS_CALC_BASE, index));
    assert_eq!(h.app.screen(), ScreenKind::Scan);
}

/// A descriptor off the clipboard reaches the checksum tool's answer.
#[test]
fn a_pasted_descriptor_reaches_the_checksum_result() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_tool(&mut h, Tool::Descriptor);
    paste(&mut h, "raw(deadbeef)#89f8spxm");
    assert_eq!(h.app.screen(), ScreenKind::ToolResult);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.tool_checksum_valid),
        "the verdict: {texts:?}"
    );
}

/// An extended public key off the clipboard reaches Convert key.
#[test]
fn a_pasted_xpub_reaches_convert_key() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_tool(&mut h, Tool::ConvertKey);
    paste(&mut h, XPUB);
    assert_eq!(h.app.screen(), ScreenKind::ToolResult);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.tool_bip32_row),
        "the key it read: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "3442193e"),
        "and the fingerprint of that very key: {texts:?}"
    );
}

/// A string the tool cannot work from opens the Type entry with the
/// text in the field and the reason under it, so a person sees what was
/// wrong.
#[test]
fn a_pasted_string_the_tool_cannot_read_opens_the_field_with_the_reason() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_tool(&mut h, Tool::ConvertKey);
    paste(&mut h, "xpub-but-not-really");
    assert_eq!(h.app.screen(), ScreenKind::Tool, "{:?}", h.app.texts());
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.tool_not_a_key),
        "the reason: {texts:?}"
    );
}

/// An address off the clipboard is checked exactly as a scanned one is.
#[test]
fn a_pasted_address_reaches_the_check_result() {
    let s = &strings::EN;
    let mut h = Harness::new(PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let ours = h
        .app
        .addresses(0, osk_bip::keys::ScriptType::NativeSegwit, false, 1)[0]
        .clone();
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    paste(&mut h, &ours);
    assert_eq!(h.app.screen(), ScreenKind::Verify);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.verify_yours_title),
        "the address is ours: {texts:?}"
    );
}

/// Words in the clear are a secret: pasting them into a calculator is
/// refused, and nothing of them reaches the tool.
#[test]
fn pasted_words_are_refused_under_a_tool() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_tool(&mut h, Tool::Hashes);
    paste(&mut h, &ABANDON.join(" "));
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.scan_reason_secret),
        "the refusal says what it refused: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("abandon")),
        "and no word of it is on the screen: {texts:?}"
    );
}

/// An extended private key on the clipboard is refused the same way.
#[test]
fn a_pasted_extended_private_key_is_refused() {
    const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_tool(&mut h, Tool::ConvertKey);
    paste(&mut h, XPRV);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert!(
        h.app.texts().iter().any(|t| t == s.scan_reason_secret),
        "{:?}",
        h.app.texts()
    );
}

/// A clipboard with nothing on it says so inside the viewfinder for a
/// moment and then goes back to looking for a QR.
#[test]
fn an_empty_clipboard_says_so_and_the_scanner_stays() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_tool(&mut h, Tool::Hashes);
    h.tap(ids::SCAN_PASTE);
    h.send(Event::ClipboardUnavailable {
        kind: FileKind::Text,
    });
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert!(
        h.app.texts().iter().any(|t| t == s.scan_nothing_to_paste),
        "{:?}",
        h.app.texts()
    );
    h.tick(opensigner_core::NOTICE_MS + 1);
    assert!(
        h.app.texts().iter().any(|t| t == s.scan_looking),
        "the viewfinder goes back to its state line: {:?}",
        h.app.texts()
    );
}

/// Copy puts a public string on the clipboard, and the screen says it
/// did.
#[test]
fn copy_writes_the_address_and_says_so() {
    let s = &strings::EN;
    let mut h = Harness::new(PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let ours = h
        .app
        .addresses(0, osk_bip::keys::ScriptType::NativeSegwit, false, 1)[0]
        .clone();
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    paste(&mut h, &ours);
    // §4.5 keeps the whole address on the Address screen, so the row
    // that copies it is on the Compare screen its reference row opens.
    h.tap(ids::VERIFY_ADDRESS);
    h.seen.clear();
    h.tap(ids::COPY);
    let written = h.seen.iter().find_map(|c| match c {
        Command::WriteClipboard { text, .. } => Some(text.clone()),
        _ => None,
    });
    assert_eq!(written.as_deref(), Some(ours.as_str()));
    h.send(Event::ClipboardWritten {
        kind: FileKind::Text,
    });
    assert!(
        h.app.texts().iter().any(|t| t == s.copy_done),
        "{:?}",
        h.app.texts()
    );
}

/// A secret carries no Copy row: the Secret screen has no way of
/// putting what it shows on a clipboard.
#[test]
fn a_secret_screen_has_no_copy_row() {
    let mut h = Harness::new(PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    h.tap(ids::EXPLORE_XPRV);
    let s = &strings::EN;
    assert!(
        !h.app.texts().iter().any(|t| t == s.action_copy),
        "a secret is never copied: {:?}",
        h.app.texts()
    );
    assert!(h.app.rect_of(ids::COPY).is_none(), "and there is no row");
}

/// A shell that answers a Copy with "not written" has no clipboard, and
/// from then on the Paste tile is dead.
#[test]
fn a_shell_with_no_clipboard_dims_the_paste_tile() {
    let s = &strings::EN;
    let mut h = Harness::new(PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_single_sig(0);
    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::COPY);
    h.send(Event::ClipboardNotWritten {
        kind: FileKind::Text,
    });
    assert!(
        h.app.texts().iter().any(|t| t == s.copy_none),
        "the screen says the copy did not happen: {:?}",
        h.app.texts()
    );
    h.tap(ids::BACK);
    h.tap(ids::WALLET_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    let dimmed = h.app.dimmed_tiles();
    assert!(
        dimmed.iter().any(|t| t == s.scan_paste),
        "the scanner's Paste tile is dead: {dimmed:?}"
    );
    h.seen.clear();
    h.tap_at(1, 1);
    assert!(
        !h.seen
            .iter()
            .any(|c| matches!(c, Command::RequestClipboard { .. })),
        "and nothing on the screen asks the shell for the clipboard"
    );
}

/// A codex32 string is a seed, so it is refused from the clipboard as
/// words are, whether it is the secret or a share (§16.109 rule 3).
#[test]
fn a_pasted_codex32_string_is_refused() {
    const SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
    const SHARE: &str = "ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr";
    let s = &strings::EN;
    for text in [SECRET, SHARE] {
        let mut h = Harness::new(PANEL);
        h.go_home();
        h.open_tile(2);
        assert_eq!(h.app.screen(), ScreenKind::Scan);
        paste(&mut h, text);
        assert_eq!(
            h.app.screen(),
            ScreenKind::Scan,
            "{text}: {:?}",
            h.app.texts()
        );
        assert!(
            h.app.texts().iter().any(|t| t == s.scan_reason_secret),
            "{text}: {:?}",
            h.app.texts()
        );
    }
}
