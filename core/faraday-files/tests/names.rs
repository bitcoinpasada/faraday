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

/// An empty name is refused in the app's words, never the operating system's message.
///
/// Invariant: An empty name is refused with the app's own words, not the operating system's message.
/// Regression: An empty name reaches the operating system and comes back in its words.
/// Owner: domain
#[test]
fn an_empty_name_is_refused_in_the_apps_words() {
    use faraday_strings::EN;
    assert_eq!(
        checked("").unwrap_err(),
        faraday_text::Text::fill(EN.files_not_a_name, &[""])
    );
}
