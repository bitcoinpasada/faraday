//! The names a file may be given on a stick: ones every system that reads
//! a FAT volume can open, so a file received under a name Windows cannot
//! hold is refused rather than written.

use faraday_files::{checked, checked_for_write};

#[test]
fn a_name_windows_cannot_open_is_refused() {
    for bad in [
        "wallet:backup.psbt",
        "a<b.txt",
        "a>b.txt",
        "a\"b.txt",
        "a|b.txt",
        "a?b.txt",
        "a*b.txt",
        "tab\there.txt",
        "ends-in-dot.",
        "ends-in-space.txt ",
        "CON",
        "con.txt",
        "Prn.psbt",
        "AUX",
        "nul.json",
        "COM1",
        "com9.txt",
        "LPT1.psbt",
        "lpt9",
    ] {
        assert!(checked_for_write(bad).is_err(), "{bad:?} was accepted");
    }
}

#[test]
fn ordinary_names_are_taken() {
    for good in [
        "savings-unsigned.psbt",
        "wallet.txt",
        "console.txt",
        "COM10.txt",
        "com.txt",
        "auxiliary",
        "a b.psbt",
    ] {
        assert_eq!(checked_for_write(good), Ok(good));
        assert_eq!(checked(good), Ok(good));
    }
}

/// A file already on a stick under a name Windows cannot open is still
/// read and removed: only a name being written is held to its rules.
#[test]
fn a_name_already_on_a_stick_is_still_read_and_removed() {
    for name in ["aux.psbt", "CON", "a:b.txt", "ends-in-dot."] {
        assert_eq!(checked(name), Ok(name), "{name}");
    }
    assert!(checked(".hidden").is_err());
    assert!(checked("a/b").is_err());
}

/// A folder standing for a stick says why it refused in the app's words,
/// never the operating system's message: a file that is not there, and a
/// rename onto a name another file has.
#[test]
fn a_folder_refuses_in_the_apps_words() {
    use faraday_files::{Dir, Place};
    use faraday_strings::EN;
    let dir = std::env::temp_dir().join(format!("faraday-files-words-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.psbt"), b"a").unwrap();
    std::fs::write(dir.join("b.psbt"), b"b").unwrap();
    let mut place = Dir(dir.clone());
    assert_eq!(
        place.read("gone.psbt", 1024),
        Err(EN.fat_not_found.to_string())
    );
    assert_eq!(
        place.rename("a.psbt", "b.psbt"),
        Err(faraday_strings::fill(EN.files_already_there, &["b.psbt"]))
    );
    assert_eq!(
        checked("").unwrap_err(),
        faraday_strings::fill(EN.files_not_a_name, &[""])
    );
    let _ = std::fs::remove_dir_all(&dir);
}
