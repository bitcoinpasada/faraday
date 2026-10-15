//! A blank template's PDF on this computer (`docs/STARTER.md` §3.8): made
//! in the system's temporary folder for Print, and opened in the system's
//! PDF viewer, which prints to any printer or to a file. The PDF holds
//! nothing of any wallet. The viewer is the system's own "open with the
//! default program" (`xdg-open`, `open`, `explorer.exe`); never a print
//! verb that sends a file to the default printer unasked.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use faraday_starter::api::Reason;

fn file_reason(path: &Path, e: impl ToString) -> Reason {
    Reason::File {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

/// Writes `pdf` as `name` in a new folder of its own under the system's
/// temporary folder. Returns its path.
pub fn write_temp(name: &str, pdf: &[u8]) -> Result<PathBuf, Reason> {
    let base = std::env::temp_dir();
    let name = Path::new(name)
        .file_name()
        .ok_or_else(|| file_reason(&base, "no file name"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = base.join(format!("faraday-template-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&dir).map_err(|e| file_reason(&dir, e))?;
    let path = dir.join(name);
    std::fs::write(&path, pdf).map_err(|e| file_reason(&path, e))?;
    Ok(path)
}

/// The program that opens a file with the system's default one.
fn opener() -> PathBuf {
    #[cfg(windows)]
    {
        // From the system folder, not from the search path.
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("explorer.exe")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/usr/bin/open")
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        PathBuf::from("xdg-open")
    }
}

/// Opens a file in the system's default program for it.
pub fn open(path: &Path) -> Result<(), Reason> {
    let program = opener();
    let mut child = Command::new(&program)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Reason::Missing(program.display().to_string()),
            _ => file_reason(path, e),
        })?;
    // Reaped when it ends, without holding the Starter up.
    std::thread::spawn(move || child.wait());
    Ok(())
}
