//! The file list a shell that can list its files answers a request with,
//! and what the file a person taps does (`docs/DESIGN.md` §5 Menu,
//! "Files"): the same round trip as a shell that answers with the file,
//! with the choice in the middle.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::scan::ScanStage;
use opensigner_core::sign::{Stage, Step};
use opensigner_core::{ScreenKind, ids, strings};
use osk_shell_api::{App, Command, Event, FileEntry, FileKind};

const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");

/// Three transactions on a card, newest first, as a shell orders them.
fn three() -> Vec<FileEntry> {
    vec![
        FileEntry {
            name: String::from("today.psbt"),
            size: 512,
            modified: Some(1_770_000_000),
        },
        FileEntry {
            name: String::from("payroll.psbt"),
            size: 1_024,
            modified: Some(1_769_900_000),
        },
        FileEntry {
            name: String::from("old.psbt"),
            size: 2_048,
            modified: Some(1_600_000_000),
        },
    ]
}

impl Harness {
    /// The test key on regtest, then its wallet → "Sign a transaction"
    /// → "Read a file".
    fn sign_request(&mut self) {
        self.start_load(&ABANDON);
        self.finish_load(None);
        self.set_network(3);
        self.open_single_sig(0);
        self.tap(ids::WALLET_SIGN);
        assert_eq!(self.app.screen(), ScreenKind::Scan);
        self.tap(ids::SCAN_FILE);
        assert_eq!(self.app.scan_stage(), Some(ScanStage::Waiting));
    }

    /// Taps the file at `i` of the list and returns the commands it
    /// produced.
    fn tap_file(&mut self, i: usize) -> Vec<Command> {
        let (x, y) = self.center(ids::at(ids::FILES_ROW_BASE, i));
        self.app.event(Event::Touch {
            x,
            y,
            phase: osk_shell_api::TouchPhase::Down,
        });
        self.app.event(Event::Touch {
            x,
            y,
            phase: osk_shell_api::TouchPhase::Up,
        });
        self.drain()
    }
}

#[test]
fn the_files_a_shell_lists_are_the_rows_a_person_reads() {
    let mut h = Harness::new(PANEL);
    h.sign_request();
    h.send(Event::FileList {
        kind: FileKind::Any,
        entries: three(),
        place: None,
    });
    assert_eq!(h.app.screen(), ScreenKind::Files);
    let texts = h.app.texts();
    let rows: Vec<&String> = texts.iter().filter(|t| t.ends_with(".psbt")).collect();
    assert_eq!(
        rows,
        ["today.psbt", "payroll.psbt", "old.psbt"],
        "the shell's order, newest first"
    );
    // Each file's date is beside its name, to the minute, so two files
    // written the same day can be told apart.
    for date in ["2026-02-02 02:40", "2026-01-31 22:53", "2020-09-13 12:26"] {
        assert!(
            texts.iter().any(|t| t == date),
            "{date} is not on the screen: {texts:?}"
        );
    }
    assert!(
        h.app.rect_of(ids::at(ids::FILES_ROW_BASE, 3)).is_none(),
        "three files are three rows"
    );
}

#[test]
fn the_file_a_person_taps_is_the_one_the_transaction_comes_from() {
    let mut h = Harness::new(PANEL);
    h.sign_request();
    h.send(Event::FileList {
        kind: FileKind::Any,
        entries: three(),
        place: None,
    });
    let commands = h.tap_file(1);
    assert!(
        commands.contains(&Command::ReadFile {
            kind: FileKind::Any,
            name: String::from("payroll.psbt"),
        }),
        "{commands:?}"
    );
    // The list is gone and the screen that asked is waiting for the
    // bytes, which arrive as they do from any shell.
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert_eq!(h.app.scan_stage(), Some(ScanStage::Waiting));
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
}

#[test]
fn a_card_with_nothing_on_it_says_so_and_the_chevron_goes_back() {
    let mut h = Harness::new(PANEL);
    h.sign_request();
    h.send(Event::FileList {
        kind: FileKind::Any,
        entries: Vec::new(),
        place: Some(String::from("OSKDATA")),
    });
    assert_eq!(h.app.screen(), ScreenKind::Files);
    let s = &strings::EN;
    let texts = h.app.texts();
    assert!(texts.iter().any(|t| t == s.files_none), "{texts:?}");
    assert!(
        texts.iter().any(|t| t == "OSKDATA"),
        "the row does not name the partition the shell looked at: {texts:?}"
    );
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert_eq!(
        h.app.scan_stage(),
        Some(ScanStage::Camera),
        "choosing no file leaves the scanner as it was"
    );
    let texts = h.app.texts();
    assert!(
        !texts.iter().any(|t| t == s.scan_waiting),
        "the scanner is still waiting: {texts:?}"
    );
}

/// A shell with no name for where it looked says only that there is
/// nothing there.
#[test]
fn a_shell_that_cannot_name_the_medium_leaves_the_row_at_one_line() {
    let mut h = Harness::new(PANEL);
    h.sign_request();
    h.send(Event::FileList {
        kind: FileKind::Any,
        entries: Vec::new(),
        place: None,
    });
    assert_eq!(h.app.screen(), ScreenKind::Files);
    let texts = h.app.texts();
    assert_eq!(
        texts.iter().filter(|t| !t.is_empty()).count(),
        2,
        "the title and the row, and nothing under it: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == strings::EN.files_none),
        "{texts:?}"
    );
}

#[test]
fn the_scanners_read_a_file_takes_the_same_list_and_routes_what_it_reads() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::FileList {
        kind: FileKind::Any,
        entries: three(),
        place: None,
    });
    assert_eq!(h.app.screen(), ScreenKind::Files);
    let commands = h.tap_file(0);
    assert!(
        commands.contains(&Command::ReadFile {
            kind: FileKind::Any,
            name: String::from("today.psbt"),
        }),
        "{commands:?}"
    );
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Sign);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
}
