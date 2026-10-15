//! Install and uninstall on Windows (`docs/STARTER.md` §3.5, §5.2, §5.4),
//! per user and never elevated. What an install puts here is a plan,
//! plain data ([`plan`]); the files are written with std alone, so the
//! tests apply a plan to a temporary `%LOCALAPPDATA%`, and the registry
//! is behind [`Registry`], which the Windows code answers from
//! `HKEY_CURRENT_USER` and the tests from a map.
//!
//! | | |
//! |---|---|
//! | the Starter | `%LOCALAPPDATA%\Programs\Faraday Starter\Faraday Starter.exe`, a copy of the running file |
//! | the Start menu shortcut | `Faraday Starter.lnk`, in the person's Start menu Programs folder |
//! | the uninstall entry | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Faraday Starter` |
//! | the icon | in the `.exe` |
//!
//! Earlier installs (`names::LEGACY`: folder, executable, shortcut,
//! and uninstall key) are found and removed by install and
//! uninstall, when their executable is not there or answers `--version`
//! as Faraday's Starter; one that does not leaves all its old names alone.
//!
//! A `Faraday Desktop.exe` an earlier Starter installed beside it, and its
//! shortcut, are no longer the Starter's (F30): install, update and
//! uninstall leave them, and the folder that holds one stays. A
//! `Faraday Starter.exe` that does not answer `--version` as Faraday's
//! own program is another program's, and is neither replaced nor
//! removed (§5.2).
//! A running copy cannot be deleted on Windows: an install renames the
//! old file aside first, and an uninstall run from the installed copy
//! leaves a short command that removes it once the Starter has exited
//! (§5.4).

use std::fs;
use std::path::{Path, PathBuf};

use faraday_core::strings::EN;
use faraday_core::{Program, names};
use faraday_starter::api::{InstallItem, Installed, Reason};

use super::shortcut;

/// The uninstall entry, under `HKEY_CURRENT_USER`.
pub fn uninstall_key() -> String {
    format!(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}",
        names::WIN_NAME
    )
}
/// An earlier install's uninstall entry.
fn legacy_uninstall_key(l: &names::Legacy) -> String {
    format!(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}",
        l.win_name
    )
}
/// The Starter's file name.
pub const APP: &str = names::WIN_EXE;
/// The shortcut's file name.
pub const SHORTCUT: &str = names::WIN_SHORTCUT;
/// Beside a file while it is replaced.
fn new_name() -> String {
    format!(".{}.new", names::WIN_EXE)
}
fn old_name() -> String {
    format!(".{}.old", names::WIN_EXE)
}
/// `name` beside it while it is written (`new`) or set aside (`old`).
fn beside(name: &str, suffix: &str) -> String {
    format!(".{name}.{suffix}")
}

/// The folders an install works in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// `%LOCALAPPDATA%`.
    pub local: PathBuf,
    /// The Start menu's Programs folder (`FOLDERID_Programs`).
    pub programs: PathBuf,
}

impl Dirs {
    /// `%LOCALAPPDATA%\Programs\Faraday Starter`.
    pub fn folder(&self) -> PathBuf {
        self.local.join("Programs").join(names::WIN_NAME)
    }

    /// An earlier install's folder, executable and shortcut.
    fn legacy_folder(&self, l: &names::Legacy) -> PathBuf {
        self.local.join("Programs").join(l.win_name)
    }

    fn legacy_app(&self, l: &names::Legacy) -> PathBuf {
        self.legacy_folder(l).join(l.win_exe)
    }

    fn legacy_shortcut(&self, l: &names::Legacy) -> PathBuf {
        self.programs.join(l.win_shortcut)
    }

    /// The installed Starter.
    pub fn app(&self) -> PathBuf {
        self.folder().join(APP)
    }

    /// The Start menu shortcut.
    pub fn shortcut(&self) -> PathBuf {
        self.programs.join(SHORTCUT)
    }
}

/// A registry value's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Data {
    /// `REG_SZ`.
    Sz(String),
    /// `REG_DWORD`.
    Dword(u32),
}

/// One step of an install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The running Starter copied to this place.
    CopySelf(PathBuf),
    /// A file written whole.
    Write(PathBuf, Vec<u8>),
    /// A value under [`uninstall_key`].
    Value(&'static str, Data),
}

/// The install: the Starter, then the shortcut, then the uninstall
/// entry, so none points at a file that is not there. `version` is the
/// running copy's, `size` its bytes.
pub fn plan(dirs: &Dirs, version: &str, name: &str, size: u64) -> Vec<Step> {
    let app = dirs.app();
    let shown = app.display().to_string();
    let folder = dirs.folder().display().to_string();
    let mut steps = vec![
        Step::CopySelf(app.clone()),
        Step::Write(dirs.shortcut(), shortcut::lnk(&shown, &folder)),
    ];
    steps.extend([
        Step::Value("DisplayName", Data::Sz(name.to_string())),
        Step::Value("DisplayVersion", Data::Sz(version.to_string())),
        Step::Value("Publisher", Data::Sz(EN.starter_publisher.to_string())),
        Step::Value("DisplayIcon", Data::Sz(format!("{shown},0"))),
        Step::Value(
            "InstallLocation",
            Data::Sz(dirs.folder().display().to_string()),
        ),
        Step::Value(
            "UninstallString",
            Data::Sz(format!("\"{shown}\" --uninstall")),
        ),
        Step::Value(
            "EstimatedSize",
            Data::Dword(u32::try_from(size.div_ceil(1024)).unwrap_or(u32::MAX)),
        ),
        Step::Value("NoModify", Data::Dword(1)),
        Step::Value("NoRepair", Data::Dword(1)),
    ]);
    steps
}

/// The person's registry, under `HKEY_CURRENT_USER`.
pub trait Registry {
    /// Sets `name` under `key`, making the key if needed.
    fn set(&mut self, key: &str, name: &str, data: &Data) -> Result<(), String>;
    /// Deletes `key` and its values; a key not there is not an error.
    fn delete(&mut self, key: &str) -> Result<(), String>;
    /// Whether `key` is there.
    fn exists(&mut self, key: &str) -> bool;
}

/// Removes the earlier installs: each one's file and folder, shortcut
/// and uninstall key, when its executable is not there or answers as the
/// Starter.
fn remove_legacy(dirs: &Dirs, ask: Ask<'_>, reg: &mut dyn Registry) -> Result<(), Reason> {
    for l in &names::LEGACY {
        let app = dirs.legacy_app(l);
        if app.exists() && !ours(ask, &app) {
            continue;
        }
        // A copy of it that is running cannot be deleted; that is left.
        let _ = fs::remove_file(&app);
        let _ = fs::remove_dir(dirs.legacy_folder(l));
        match fs::remove_file(dirs.legacy_shortcut(l)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(file_error(&dirs.legacy_shortcut(l), &e)),
        }
        reg.delete(&legacy_uninstall_key(l)).map_err(key_error)?;
    }
    Ok(())
}

/// What a file answers to `--version`: which Faraday program and its
/// build string, or `None` for anything else.
pub type Ask<'a> = &'a dyn Fn(&Path) -> Option<(Program, String)>;

fn ours(ask: Ask<'_>, path: &Path) -> bool {
    ask(path).is_some_and(|(p, _)| p == Program::StarterApp)
}

fn file_error(path: &Path, e: &std::io::Error) -> Reason {
    Reason::File {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

fn key_error(e: String) -> Reason {
    Reason::File {
        path: format!(r"HKEY_CURRENT_USER\{}", uninstall_key()),
        detail: e,
    }
}

/// Whether `a` and `b` are one file.
fn same(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Applies `steps`: `running` is the Starter being run. A `Faraday Starter.exe`
/// already there that is not Faraday's stops it before anything is
/// written. Where the Starter went.
pub fn install(
    dirs: &Dirs,
    steps: &[Step],
    running: &Path,
    ask: Ask<'_>,
    reg: &mut dyn Registry,
) -> Result<String, Reason> {
    let app = dirs.app();
    if app.exists() && !ours(ask, &app) {
        return Err(Reason::InTheWay {
            path: app.display().to_string(),
        });
    }
    for step in steps {
        match step {
            Step::CopySelf(to) => copy_self(running, to)?,
            Step::Write(path, bytes) => {
                let fail = |e: std::io::Error| file_error(path, &e);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(fail)?;
                }
                fs::write(path, bytes).map_err(fail)?;
            }
            Step::Value(name, data) => reg.set(&uninstall_key(), name, data).map_err(key_error)?,
        }
    }
    remove_legacy(dirs, ask, reg)?;
    Ok(app.display().to_string())
}

/// Copies `running` to `to` through a file beside it.
fn copy_self(running: &Path, to: &Path) -> Result<(), Reason> {
    if same(running, to) {
        return Ok(());
    }
    put(to, |at| fs::copy(running, at).map(|_| ()))
}

/// Puts a file at `to`: `write` makes it beside `to`, and it is renamed
/// into place. A file already at `to` is renamed aside first, which
/// Windows allows while it runs, and removed when it can be.
fn put(to: &Path, write: impl FnOnce(&Path) -> std::io::Result<()>) -> Result<(), Reason> {
    let fail = |e: std::io::Error| file_error(to, &e);
    let folder = to.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(folder).map_err(fail)?;
    let name = to.file_name().unwrap_or_default().to_string_lossy();
    let (new, old) = (
        folder.join(beside(&name, "new")),
        folder.join(beside(&name, "old")),
    );
    if let Err(e) = write(&new) {
        let _ = fs::remove_file(&new);
        return Err(fail(e));
    }
    let _ = fs::remove_file(&old);
    let moved = to.exists();
    if moved && let Err(e) = fs::rename(to, &old) {
        let _ = fs::remove_file(&new);
        return Err(fail(e));
    }
    if let Err(e) = fs::rename(&new, to) {
        if moved {
            let _ = fs::rename(&old, to);
        }
        let _ = fs::remove_file(&new);
        return Err(fail(e));
    }
    let _ = fs::remove_file(&old);
    Ok(())
}

/// What is installed, found on this computer, or `None` when no Starter
/// is installed.
pub fn installed(dirs: &Dirs, ask: Ask<'_>, reg: &mut dyn Registry) -> Option<Installed> {
    let app = dirs.app();
    let version = match ask(&app) {
        Some((Program::StarterApp, build)) => build,
        _ => return installed_legacy(dirs, ask, reg),
    };
    let mut items = vec![(InstallItem::App, app.display().to_string())];
    if dirs.shortcut().is_file() {
        items.push((InstallItem::Menu, dirs.shortcut().display().to_string()));
    }
    if reg.exists(&uninstall_key()) {
        items.push((
            InstallItem::UninstallEntry,
            format!(r"HKEY_CURRENT_USER\{}", uninstall_key()),
        ));
    }
    Some(Installed { version, items })
}

/// An install made under earlier names, found by its executable.
fn installed_legacy(dirs: &Dirs, ask: Ask<'_>, reg: &mut dyn Registry) -> Option<Installed> {
    names::LEGACY.iter().find_map(|l| {
        let app = dirs.legacy_app(l);
        let version = match ask(&app) {
            Some((Program::StarterApp, build)) => build,
            _ => return None,
        };
        let mut items = vec![(InstallItem::App, app.display().to_string())];
        if dirs.legacy_shortcut(l).is_file() {
            items.push((
                InstallItem::Menu,
                dirs.legacy_shortcut(l).display().to_string(),
            ));
        }
        if reg.exists(&legacy_uninstall_key(l)) {
            items.push((
                InstallItem::UninstallEntry,
                format!(r"HKEY_CURRENT_USER\{}", legacy_uninstall_key(l)),
            ));
        }
        Some(Installed { version, items })
    })
}

/// What is left once uninstall returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Left {
    /// Nothing.
    Nothing,
    /// The running copy is the installed one: this command (the
    /// arguments for `cmd.exe`, as one line) removes it and its folder
    /// once the Starter has exited.
    AfterExit(String),
}

/// Uninstalls: the Starter (only Faraday's), the files set aside by an
/// install, the shortcut, the uninstall entry, and the folders they leave
/// empty. Nothing else.
pub fn uninstall(
    dirs: &Dirs,
    running: &Path,
    ask: Ask<'_>,
    reg: &mut dyn Registry,
) -> Result<Left, Reason> {
    let app = dirs.app();
    let folder = dirs.folder();
    let mut left = Left::Nothing;
    if app.exists() && ours(ask, &app) {
        if same(running, &app) {
            left = after_exit(&folder);
        } else {
            fs::remove_file(&app).map_err(|e| file_error(&app, &e))?;
        }
    }
    let _ = fs::remove_file(folder.join(old_name()));
    let _ = fs::remove_file(folder.join(new_name()));
    match fs::remove_file(dirs.shortcut()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(file_error(&dirs.shortcut(), &e)),
    }
    reg.delete(&uninstall_key()).map_err(key_error)?;
    remove_legacy(dirs, ask, reg)?;
    let _ = fs::remove_dir(&folder);
    let _ = fs::remove_dir(dirs.local.join("Programs"));
    Ok(left)
}

/// The command that removes the Starter from `folder` once it has
/// exited: for up to thirty seconds, once a second, it deletes
/// `Faraday Starter.exe` and removes the folder if that left it empty, and stops
/// when the folder is gone. Only that file and an empty folder. `None`
/// for a folder whose name `cmd.exe` would read as more than a name.
///
/// The second is waited by `ping` to the loopback address: `timeout`
/// ends at once when its input is not a console, as it is here, and a
/// deleted file still open elsewhere (the exiting process, a scanner)
/// stays in the folder until it is closed, so the tries need the time
/// between them.
pub fn after_exit_command(folder: &Path) -> Option<String> {
    let f = folder.display().to_string();
    if f.contains(['"', '%', '^', '\n', '\r']) {
        return None;
    }
    let app = format!("{f}\\{APP}");
    Some(format!(
        "/d /q /s /c \"for /l %i in (1,1,30) do (ping -n 2 127.0.0.1 >nul 2>&1 & del /f /q \"{app}\" >nul 2>&1 & rmdir \"{f}\" >nul 2>&1 & if not exist \"{f}\" exit /b 0)\""
    ))
}

fn after_exit(folder: &Path) -> Left {
    match after_exit_command(folder) {
        Some(c) => Left::AfterExit(c),
        None => Left::Nothing,
    }
}
