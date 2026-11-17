//! A note saved in a backup plan reads back as written: text a note cannot
//! keep as it is, is refused, never kept as other text.

use faraday_plan::Note;

const MOVED: &[u8] = include_bytes!("../../fuzz/corpus/plan_text/moved-unclean.txt");
const VAULTNAME: &[u8] = include_bytes!("../../fuzz/corpus/plan_text/vaultname-unclean.txt");

fn round_trips(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    if let Some(note) = Note::from_text(&text, 4) {
        assert_eq!(Note::from_text(&note.to_text(), 4), Some(note));
    }
}

#[test]
fn a_moved_note_with_stray_whitespace_reads_back_as_read() {
    round_trips(MOVED);
}

#[test]
fn a_vault_name_with_a_line_break_reads_back_as_read() {
    round_trips(VAULTNAME);
}

#[test]
fn a_note_as_the_plan_writes_it_reads_back() {
    for text in [
        "moved 2026-10-10",
        "vaultname 1 Home safe",
        "holder 9a6a2580 Alice's Coldcard",
    ] {
        let note = Note::from_text(text, 4).expect("reads");
        assert_eq!(&*note.to_text(), text);
    }
}
