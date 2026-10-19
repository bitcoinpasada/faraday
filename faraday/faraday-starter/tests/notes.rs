//! What to write down before the restart (`docs/STARTER.md` §3.4, F47):
//! what the Start page shows, and the sheet Print hands the shell to
//! open, for a maker with a page, one without, and Intel Macs.

use faraday_starter::api::{Computer, Os, Reason, StarterCommand, StarterEvent};
use faraday_starter::notes::Notes;
use faraday_starter::{Page, Starter, StarterAction};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const NOT_FOUND: &str = "If the Secure Boot setting cannot be found: exit the BIOS without saving \
                         changes. Windows starts as before.";
const NOT_FOUND_LINUX: &str = "If the Secure Boot setting cannot be found: exit the BIOS without \
                               saving changes. The computer starts as before.";

fn computer(os: Os, maker: &str, family: &str, model: &str) -> Computer {
    Computer {
        os,
        maker: maker.into(),
        model: model.into(),
        family: family.into(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }
}

fn dell() -> Computer {
    computer(Os::Windows, "Dell Inc.", "Latitude", "Latitude 5440")
}

fn unknown() -> Computer {
    computer(Os::Windows, "Example Computers", "Example", "Example 14")
}

/// An Intel Mac on macOS, by its model identifier.
fn mac(model: &str) -> Computer {
    Computer {
        secure_boot: None,
        ..computer(Os::MacOs, "Apple Inc.", "", model)
    }
}

/// On the Start page, with nothing holding the restart.
fn start_page(c: Computer) -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 4000,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    s.starter_event(StarterEvent::Computer(c));
    s.starter_event(StarterEvent::Disks(Vec::new()));
    s.press(StarterAction::Start);
    assert_eq!(s.page, Page::Start);
    s
}

fn texts(s: &mut Starter) -> Vec<String> {
    s.drawn_texts()
}

fn position(s: &mut Starter, text: &str) -> Option<usize> {
    texts(s).iter().position(|t| t.contains(text))
}

/// The value drawn right after a row's label.
fn value_after(s: &mut Starter, label: &str) -> String {
    let t = texts(s);
    let i = t
        .iter()
        .position(|x| x == label)
        .unwrap_or_else(|| panic!("no row {label}: {t:?}"));
    t[i + 1].clone()
}

/// What Print hands the shell: the file's name and the PDF.
fn printed(s: &mut Starter) -> (String, Vec<u8>) {
    while s.next_command().is_some() {}
    s.press(StarterAction::PrintNotes);
    let c: Vec<StarterCommand> = std::iter::from_fn(|| s.next_command()).collect();
    match c.as_slice() {
        [StarterCommand::PrintNotes { name, pdf }] => (name.clone(), pdf.clone()),
        other => panic!("one PrintNotes expected: {other:?}"),
    }
}

/// Whether the PDF's page draws `text` (its streams are not compressed;
/// the writer escapes parentheses and maps the arrow to ›).
fn pdf_has(pdf: &[u8], text: &str) -> bool {
    let mut want = Vec::new();
    for c in text.chars() {
        match c {
            '(' | ')' => {
                want.push(b'\\');
                want.push(c as u8);
            }
            '→' => want.push(0x9B),
            c => want.extend_from_slice(c.to_string().as_bytes()),
        }
    }
    // A wrapped line is split at spaces; each word is enough to find.
    let hay = pdf;
    want.split(|b| *b == b' ')
        .filter(|w| !w.is_empty())
        .all(|w| hay.windows(w.len()).any(|x| x == w))
}

fn rows(n: &Notes) -> Vec<(String, Option<&'static str>)> {
    n.rows.iter().map(|r| (r.label.clone(), r.value)).collect()
}

fn heads(n: &Notes) -> Vec<&'static str> {
    n.steps.iter().map(|s| s.head).collect()
}

#[test]
fn a_dell_s_page_shows_its_three_things_before_the_restart() {
    let mut s = start_page(dell());
    assert_eq!(value_after(&mut s, "Boot menu key"), "F12");
    assert_eq!(value_after(&mut s, "BIOS key"), "F2");
    assert_eq!(
        value_after(&mut s, "Secure Boot setting"),
        "Boot Configuration → Secure Boot"
    );
    let notes = position(&mut s, "Write down").expect("Write down");
    let restart = position(&mut s, "Restart into the BIOS").expect("the restart step");
    assert!(notes < restart);
    let print = position(&mut s, "Print").expect("Print");
    assert!(print < restart);
}

#[test]
fn a_dell_s_sheet_has_the_three_things_and_the_steps_in_order() {
    let notes = Notes::for_computer(&dell()).expect("notes");
    assert_eq!(notes.title, "Start this computer from the stick");
    assert_eq!(
        rows(&notes),
        [
            ("Boot menu key".to_string(), Some("F12")),
            ("BIOS key".to_string(), Some("F2")),
            (
                "Secure Boot setting".to_string(),
                Some("Boot Configuration → Secure Boot")
            ),
        ]
    );
    assert_eq!(
        heads(&notes),
        [
            "Turn Secure Boot off",
            "Start from the stick",
            "Turn Secure Boot back on before starting Windows",
        ]
    );
    assert_eq!(notes.steps[0].lines[1], NOT_FOUND);
    let mut s = start_page(dell());
    let (name, pdf) = printed(&mut s);
    assert_eq!(name, "faraday-start-from-the-stick.pdf");
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(pdf, notes.pdf());
    for t in [
        "Start this computer from the stick",
        "Boot menu key",
        "F12",
        "Boot Configuration → Secure Boot",
        "Turn Secure Boot back on before starting Windows",
        NOT_FOUND,
    ] {
        assert!(pdf_has(&pdf, t), "{t}");
    }
}

#[test]
fn a_maker_without_a_page_reads_not_known_and_the_sheet_leaves_lines_to_write_on() {
    let mut s = start_page(unknown());
    for label in ["Boot menu key", "BIOS key", "Secure Boot setting"] {
        assert_eq!(value_after(&mut s, label), "Not known", "{label}");
    }
    let notes = Notes::for_computer(&unknown()).expect("notes");
    assert!(notes.rows.iter().all(|r| r.value.is_none()));
    assert_eq!(notes.rows.len(), 3);
    let (_, pdf) = printed(&mut s);
    assert!(!pdf_has(&pdf, "Not known"));
    // A blank is a ruled line under where the value would be: one per row.
    let lines = pdf.windows(4).filter(|w| w == b" l S").count();
    assert_eq!(lines, 3);
    assert!(pdf_has(&pdf, NOT_FOUND));
}

#[test]
fn on_linux_the_sheet_says_the_computer_starts_as_before() {
    let mut c = dell();
    c.os = Os::Linux;
    let notes = Notes::for_computer(&c).expect("notes");
    assert_eq!(notes.steps[0].lines[1], NOT_FOUND_LINUX);
    assert_eq!(notes.steps[2].head, "Turn Secure Boot back on");
    let mut s = start_page(c);
    // Wrapped over two lines at this width.
    assert!(texts(&mut s).join(" ").contains(NOT_FOUND_LINUX));
}

#[test]
fn the_page_says_what_to_do_when_the_setting_cannot_be_found_under_the_secure_boot_step() {
    let mut s = start_page(unknown());
    let off = position(&mut s, "Turn Secure Boot off").expect("the step");
    let line = position(&mut s, NOT_FOUND).expect("the line");
    let next = position(&mut s, "Start from the stick").expect("the next step");
    assert!(off < line && line < next);
}

#[test]
fn a_t2_mac_gets_the_macs_three_things_and_steps() {
    let notes = Notes::for_computer(&mac("MacBookPro15,1")).expect("notes");
    assert_eq!(notes.title, "Start this Mac from the stick");
    assert_eq!(
        rows(&notes),
        [
            ("Boot menu key".to_string(), Some("Option, held")),
            ("macOS Recovery key".to_string(), Some("Command-R, held")),
            (
                "Startup Security".to_string(),
                Some("macOS Recovery → Utilities → Startup Security Utility")
            ),
        ]
    );
    assert_eq!(
        heads(&notes),
        [
            "Lower Startup Security",
            "Start from the stick",
            "Afterwards: Startup Security back to Full",
        ]
    );
    // No BIOS on a Mac: no line about one.
    assert!(
        notes
            .steps
            .iter()
            .flat_map(|s| s.lines.iter())
            .all(|l| !l.contains("BIOS"))
    );
    let mut s = start_page(mac("MacBookPro15,1"));
    assert_eq!(value_after(&mut s, "Boot menu key"), "Option, held");
    let (_, pdf) = printed(&mut s);
    for t in [
        "Option, held",
        "Command-R, held",
        "Startup Security Utility",
    ] {
        assert!(pdf_has(&pdf, t), "{t}");
    }
}

#[test]
fn a_mac_without_t2_has_only_its_boot_key_and_one_the_table_does_not_know_is_marked() {
    let notes = Notes::for_computer(&mac("MacBookPro11,1")).expect("notes");
    assert_eq!(
        rows(&notes),
        [("Boot menu key".to_string(), Some("Option, held"))]
    );
    assert_eq!(heads(&notes), ["Start from the stick"]);
    let notes = Notes::for_computer(&mac("MacBookPro99,9")).expect("notes");
    let labels: Vec<String> = notes.rows.iter().map(|r| r.label.clone()).collect();
    assert_eq!(
        labels,
        [
            "Boot menu key",
            "macOS Recovery key, only on a Mac with the T2 chip",
            "Startup Security, only on a Mac with the T2 chip",
        ]
    );
    assert_eq!(notes.steps[0].lines[0], "Only on a Mac with the T2 chip");
}

#[test]
fn a_computer_that_cannot_start_the_stick_has_nothing_to_write_down() {
    let mut arm = unknown();
    arm.arm = true;
    assert!(Notes::for_computer(&arm).is_none());
    let mut s = start_page(arm);
    assert!(position(&mut s, "Write down").is_none());
}

#[test]
fn a_viewer_that_does_not_open_says_why_until_the_next_print() {
    let mut s = start_page(dell());
    let _ = printed(&mut s);
    s.starter_event(StarterEvent::NotesOpened(Err(Reason::Missing(
        "xdg-open".into(),
    ))));
    assert!(texts(&mut s).iter().any(|t| t.contains("xdg-open")));
    let _ = printed(&mut s);
    assert!(!texts(&mut s).iter().any(|t| t.contains("xdg-open")));
}

#[test]
fn the_sheet_names_no_path_user_or_computer() {
    let pdf = Notes::for_computer(&dell()).expect("notes").pdf();
    for word in [
        b"/Info".as_slice(),
        b"/Producer",
        b"/Author",
        b"/CreationDate",
        b"/home/",
        b"Latitude",
        b"Dell Inc.",
    ] {
        assert!(!pdf.windows(word.len()).any(|w| w == word), "{word:?}");
    }
}
