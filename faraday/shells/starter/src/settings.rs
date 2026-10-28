//! The installed Starter's settings file (`docs/TEXT-SIZE.md` §6,
//! `docs/STARTER.md` §5.2): one `text=` line, the Text size chosen on
//! About. It is written only while a Starter is installed, read at start,
//! and removed by Uninstall; a Starter with nothing installed writes none
//! (§5.1). Where it lives is each system's
//! ([`crate::platform::Platform::settings_file`]).

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use faraday_core::ui::TextSize;

/// The file's name, in the folder each system names.
pub const FILE: &str = "settings.txt";

/// Costs a file read: the text size the file at `path` holds; `None`
/// where there is no file, or no `text=` line naming one of the three
/// sizes.
pub fn read(path: &Path) -> Option<TextSize> {
    let body = fs::read_to_string(path).ok()?;
    body.lines()
        .filter_map(|l| l.trim().strip_prefix("text="))
        .find_map(TextSize::from_id)
}

/// The file's contents for `text`.
pub fn body(text: TextSize) -> String {
    format!("text={}\n", text.id())
}

/// Costs a file write: `text` as the file at `path`, its folder made
/// where it is missing, written beside it and renamed into place so a
/// file half written is never read.
pub fn write(path: &Path, text: TextSize) -> std::io::Result<()> {
    let folder = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(folder)?;
    let beside = folder.join(format!(".{FILE}.new"));
    let done = fs::write(&beside, body(text)).and_then(|()| fs::rename(&beside, path));
    if done.is_err() {
        let _ = fs::remove_file(&beside);
    }
    done
}

/// Costs a file removal: removes the file at `path`, then its folder
/// where that leaves it empty. A file already gone is not an error.
pub fn remove(path: &Path) -> std::io::Result<()> {
    let folder = path.parent().unwrap_or(Path::new("."));
    let _ = fs::remove_file(folder.join(format!(".{FILE}.new")));
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let _ = fs::remove_dir(folder);
    Ok(())
}
