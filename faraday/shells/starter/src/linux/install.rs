//! Install and uninstall on Linux (`docs/STARTER.md` §3.5, §5.2, §5.4),
//! per user and never as root.
//!
//! The files an install puts here, and the only ones an uninstall
//! removes:
//!
//! | | |
//! |---|---|
//! | the Starter | `~/.local/bin/faraday-starter`, a copy of the running file |
//! | the menu entry | `~/.local/share/applications/app.faradayoffline.FaradayStarter.desktop` |
//! | the icon | `~/.local/share/icons/hicolor/256x256/apps/app.faradayoffline.FaradayStarter.png` |
//!
//! A desktop app an earlier Starter installed (`~/.local/bin/faraday-desktop`
//! and its menu entry) is no longer the Starter's (F30): install, update
//! and uninstall leave it as it is, and the README says how to remove it.
//!
//! Earlier installs (`names::LEGACY`: the Starter as `~/.local/bin/faraday`
//! or `faraday-starter`, with the menu entry and icon of the same old name)
//! are replaced by install and removed by uninstall, the executable only
//! when it answers `--version` as Faraday's Starter (old words included);
//! nothing else of the old names. The names are in `faraday-core/src/names.rs`.
//!
//! Folders an install had to make (`~/.local/bin` and the rest) are the
//! desktop's shared places; uninstall leaves them. The installed copy's
//! version is what `--version` prints (the build string, or the
//! version for a build without one), after "Faraday Starter ". A file of
//! the Starter's name that does not answer `--version` as a Faraday app
//! is another program's, and is left alone (§5.2).

use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use faraday_core::names;
use faraday_core::strings::EN;
use faraday_core::{Program, parse_version_output};
use faraday_starter::api::{InstallItem, Installed, Reason};

/// The icon, made by `just faraday-starter-icon` from Faraday's mark.
const ICON: &[u8] = include_bytes!("../../assets/icon.png");
/// The menu entry's and the icon's reverse-DNS name (§5.2).
const MENU: &str = names::MENU_ID;
/// How long a copy gets to say its version.
const VERSION_WAIT: Duration = Duration::from_secs(5);

/// The folder an install works in: the home folder.
#[derive(Debug, Clone)]
pub struct Dirs {
    home: PathBuf,
}

impl Dirs {
    /// The home folder `home`.
    pub fn new(home: PathBuf) -> Dirs {
        Dirs { home }
    }

    /// From the environment: `HOME`. A relative or empty value is not
    /// used.
    pub fn from_env() -> Result<Dirs, Reason> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or(Reason::NoHome)?;
        Ok(Dirs::new(home))
    }

    fn bin(&self) -> PathBuf {
        self.home.join(".local/bin").join(names::LINUX_BIN)
    }

    /// An earlier install's Starter.
    fn legacy_bin(&self, l: &names::Legacy) -> PathBuf {
        self.home.join(".local/bin").join(l.linux_bin)
    }

    /// An earlier install's files other than the executable: its menu
    /// entry and icon.
    fn legacy_extras(&self, l: &names::Legacy) -> [PathBuf; 2] {
        [
            self.menu(l.menu_id),
            self.home.join(format!(
                ".local/share/icons/hicolor/256x256/apps/{}.png",
                l.menu_id
            )),
        ]
    }

    fn menu(&self, name: &str) -> PathBuf {
        self.home
            .join(format!(".local/share/applications/{name}.desktop"))
    }

    fn icon(&self) -> PathBuf {
        self.home.join(format!(
            ".local/share/icons/hicolor/256x256/apps/{MENU}.png"
        ))
    }

    /// Every file an install puts here, in the order it writes them.
    fn files(&self) -> [(InstallItem, PathBuf); 3] {
        [
            (InstallItem::App, self.bin()),
            (InstallItem::Icon, self.icon()),
            (InstallItem::Menu, self.menu(MENU)),
        ]
    }

    /// `path` as a person reads it: under the home folder as `~/…`.
    fn shown(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.display().to_string(),
        }
    }

    fn file_error(&self, path: &Path, e: &std::io::Error) -> Reason {
        Reason::File {
            path: self.shown(path),
            detail: e.to_string(),
        }
    }
}

/// What is installed, found on disk: `None` when there is no installed
/// Starter. Runs the installed copy with `--version`; a file there that
/// does not answer as the Starter is not an installed Starter.
pub fn installed(dirs: &Dirs) -> Option<Installed> {
    let version = match faraday_of(&dirs.bin()) {
        Ok((Program::StarterApp, build)) => build,
        Ok(_) | Err(_) => return installed_legacy(dirs),
    };
    let items = dirs
        .files()
        .into_iter()
        .filter(|(item, p)| *item == InstallItem::App || p.is_file())
        .map(|(item, p)| (item, dirs.shown(&p)))
        .collect();
    Some(Installed { version, items })
}

/// An install made under earlier names, found by its Starter.
fn installed_legacy(dirs: &Dirs) -> Option<Installed> {
    names::LEGACY.iter().find_map(|l| {
        let bin = dirs.legacy_bin(l);
        let version = match faraday_of(&bin) {
            Ok((Program::StarterApp, build)) => build,
            Ok(_) | Err(_) => return None,
        };
        let mut items = vec![(InstallItem::App, dirs.shown(&bin))];
        items.extend(
            dirs.legacy_extras(l)
                .into_iter()
                .filter(|p| p.is_file())
                .map(|p| {
                    let item = if p.extension().is_some_and(|e| e == "png") {
                        InstallItem::Icon
                    } else {
                        InstallItem::Menu
                    };
                    (item, dirs.shown(&p))
                }),
        );
        Some(Installed { version, items })
    })
}

/// Removes the earlier installs' files. One set goes when its executable
/// is ours or not there; an executable there that is another program's
/// keeps the names it came with. A file already gone is not an error.
fn remove_legacy(dirs: &Dirs) -> Result<(), Reason> {
    for l in &names::LEGACY {
        let bin = dirs.legacy_bin(l);
        if bin.exists() && !ours(&bin, Program::StarterApp) {
            continue;
        }
        for path in std::iter::once(bin).chain(dirs.legacy_extras(l)) {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(dirs.file_error(&path, &e)),
            }
        }
    }
    Ok(())
}

/// Whether `path` answers `--version` as `program`.
fn ours(path: &Path, program: Program) -> bool {
    faraday_of(path).is_ok_and(|(p, _)| p == program)
}

/// Which Faraday program `path` is, and its build string. A file that
/// is not executable, does not answer in time or prints anything else
/// is not ours.
fn faraday_of(path: &Path) -> Result<(Program, String), String> {
    let said = version_of(path)?;
    parse_version_output(&said)
        .map(|(p, build)| (p, build.to_string()))
        .ok_or_else(|| String::from("not a Faraday app"))
}

/// What `path --version` prints, when it answers within a few seconds.
fn version_of(path: &Path) -> Result<String, String> {
    let mut child = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) if status.success() => break,
            Some(status) => return Err(status.to_string()),
            None if started.elapsed() > VERSION_WAIT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(String::from("no answer"));
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    let mut said = String::new();
    child
        .stdout
        .take()
        .ok_or_else(|| String::from("no output"))?
        .read_to_string(&mut said)
        .map_err(|e| e.to_string())?;
    let line = said.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return Err(String::from("no output"));
    }
    Ok(line.to_string())
}

/// Installs: a copy of `running` as the Starter, the icon and the menu
/// entry. Each file is written beside its place and renamed into it, so
/// a copy that is running is replaced whole. The menu entry comes last,
/// so it never points at a file that is not there. Returns where the
/// Starter went.
pub fn install(dirs: &Dirs, running: &Path) -> Result<String, Reason> {
    let app = dirs.bin();
    // A file of the Starter's name that is another program's stops the
    // install before anything is written.
    if app.exists() && !ours(&app, Program::StarterApp) {
        return Err(Reason::InTheWay {
            path: dirs.shown(&app),
        });
    }
    place(dirs, &app, 0o755, |to| fs::copy(running, to).map(|_| ()))?;
    place(dirs, &dirs.icon(), 0o644, |to| fs::write(to, ICON))?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec={}\nIcon={MENU}\nTerminal=false\nCategories=Utility;\n",
        EN.starter_menu_name,
        exec_quote(&app)
    );
    place(dirs, &dirs.menu(MENU), 0o644, |to| fs::write(to, &entry))?;
    remove_legacy(dirs)?;
    Ok(dirs.shown(&app))
}

/// Writes `path` through a file beside it, then renames it into place.
fn place(
    dirs: &Dirs,
    path: &Path,
    mode: u32,
    write: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), Reason> {
    let fail = |e: std::io::Error| dirs.file_error(path, &e);
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(fail)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let beside = parent.join(format!(".{name}.new"));
    let done = write(&beside)
        .and_then(|()| fs::set_permissions(&beside, fs::Permissions::from_mode(mode)))
        .and_then(|()| fs::rename(&beside, path));
    if let Err(e) = done {
        let _ = fs::remove_file(&beside);
        return Err(fail(e));
    }
    Ok(())
}

/// `path` as a desktop entry's `Exec` value reads it: quoted when it
/// holds a character the specification reserves, with the reserved
/// characters escaped, `%` doubled, and backslashes doubled again for
/// the file's own string escaping.
pub fn exec_quote(path: &Path) -> String {
    let text = path.display().to_string();
    let reserved = " \t\n\"'\\><~|&;$*?#()`";
    let mut out = String::new();
    let quote = text.chars().any(|c| reserved.contains(c));
    if quote {
        out.push('"');
    }
    for c in text.chars() {
        match c {
            '"' | '`' | '$' | '\\' if quote => {
                out.push('\\');
                out.push(c);
            }
            '%' => out.push_str("%%"),
            _ => out.push(c),
        }
    }
    if quote {
        out.push('"');
    }
    out.replace('\\', "\\\\")
}

/// Uninstalls: removes the files install wrote, and nothing else. A file
/// already gone is not an error.
pub fn uninstall(dirs: &Dirs) -> Result<(), Reason> {
    for (item, path) in dirs.files() {
        if item == InstallItem::App && path.exists() && !ours(&path, Program::StarterApp) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(dirs.file_error(&path, &e)),
        }
    }
    remove_legacy(dirs)
}
