//! The picks that list held files, opened by presses from Check and from
//! Tools (`docs/WORKSPACE.md` §2): the held file's row stands above the
//! ways in and a press on it takes the file.

use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::App;

/// A signed transaction of one input and one output, in hex.
const TX: &str = "0200000001000000000000000000000000000000000000000000000000000000000000000100000000\
    00ffffffff0100e1f505000000001600140000000000000000000000000000000000000000\
    00000000";

fn pressed_from(opener: Screen, tool: Screen, name: &str, bytes: &[u8], row: fn(usize) -> Action) {
    let mut app: Faraday = faraday_core::testkit::started();
    faraday_core::testkit::inbox_file(&mut app, name, bytes.to_vec());
    let at = app.inbox().len() - 1;
    app.press(Action::Nav(opener));
    assert_eq!(app.screen, opener);
    app.press(Action::Nav(tool));
    assert_eq!(app.screen, tool);
    let _ = app.frame();
    let file = app.hit_box(row(at)).expect("the held file's row");
    let scan = if tool == Screen::Decode {
        Action::ScanFor(faraday_core::Ask::Decode)
    } else {
        Action::Scan
    };
    let way = app.hit_box(scan).expect("the Scan way in");
    assert!(file.1 < way.1, "{opener:?}: the file above the ways in");
    app.press(row(at));
}

#[test]
fn decode_lists_the_held_transaction_first_from_check_and_from_tools() {
    for opener in [Screen::Check, Screen::Catalog] {
        pressed_from(
            opener,
            Screen::Decode,
            "spend.txn",
            TX.as_bytes(),
            Action::DecodeInbox,
        );
    }
}

#[test]
fn check_a_signed_message_lists_the_held_message_from_check_and_from_tools() {
    let message = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n\
        H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=\n\
        Pay the invoice by Friday.\n";
    for opener in [Screen::Check, Screen::Catalog] {
        pressed_from(
            opener,
            Screen::CheckMessage,
            "signed.txt",
            message.as_bytes(),
            Action::CheckMessage,
        );
    }
}
