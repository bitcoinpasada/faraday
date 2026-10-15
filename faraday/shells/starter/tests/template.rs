//! Blank templates on this computer (`docs/STARTER.md` §3.8): the PDF lands
//! in Downloads under its name, never over a file, and Print's copy goes
//! to the temporary folder.

use faraday_starter::template::{Copy, Template};
use faraday_starter_shell::pdf::write_temp;

fn template() -> Template {
    Template {
        words: 12,
        copy: Copy::SeedQr,
        ..Template::default()
    }
}

#[test]
fn the_pdf_lands_in_downloads_and_a_second_save_is_named_two() {
    let dir = std::env::temp_dir().join(format!("faraday-template-test-{}", std::process::id()));
    let t = template();
    let pdf = t.pdf().expect("a PDF");
    let first = faraday_storage::save_download(&dir, &t.file_name(), &pdf).unwrap();
    let second = faraday_storage::save_download(&dir, &t.file_name(), &pdf).unwrap();
    assert_eq!(
        first.file_name().unwrap().to_str().unwrap(),
        "faraday-blank-template-12-words-seedqr.pdf"
    );
    assert_eq!(
        second.file_name().unwrap().to_str().unwrap(),
        "faraday-blank-template-12-words-seedqr (2).pdf"
    );
    assert_eq!(std::fs::read(&first).unwrap(), pdf);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn print_writes_the_same_pdf_to_the_temporary_folder() {
    let t = template();
    let pdf = t.pdf().expect("a PDF");
    let path = write_temp(&t.file_name(), &pdf).unwrap();
    assert!(path.starts_with(std::env::temp_dir()));
    assert_eq!(std::fs::read(&path).unwrap(), pdf);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
