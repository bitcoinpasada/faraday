//! Install, uninstall and the desktop app on Linux (`docs/HUB.md` §3.5,
//! §5.2 to §5.4), per user and never as root.
//!
//! The files an install puts here, and the only ones an uninstall
//! removes besides the cache (§5.3):
//!
//! | | |
//! |---|---|
//! | the hub app | `~/.local/bin/faraday-starter`, a copy of the running file |
//! | the desktop app, if ticked | `~/.local/bin/faraday-desktop` |
//! | the menu entries | `~/.local/share/applications/app.faradayoffline.FaradayStarter.desktop` and `…FaradayDesktop.desktop` |
//! | the icon | `~/.local/share/icons/hicolor/256x256/apps/app.faradayoffline.FaradayStarter.png` |
//!
//! Earlier installs (`names::LEGACY`: the hub app as `~/.local/bin/faraday`
//! or `faraday-hub`, with the menu entry and icon of the same old name)
//! are replaced by install and removed by uninstall, the executable only
//! when it answers `--version` as Faraday's hub app (old words included);
//! nothing else of the old names. The names are in `faraday-core/src/names.rs`.
//!
//! Folders an install had to make (`~/.local/bin` and the rest) are the
//! desktop's shared places; uninstall leaves them. The installed copy's
//! version is what `--version` prints (the build string, or the
//! version for a build without one), after "Faraday Starter ". A file of
//! either executable's name that does not answer `--version` as a
//! Faraday app is another program's, and is left alone (§5.2).

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use faraday_core::names;
use faraday_core::strings::EN;
use faraday_core::{Program, parse_version_output};
use faraday_hub::api::{InstallItem, Installed, Reason};
use faraday_payload::{ImageKind, Sha256Sum};

use crate::payload::Payload;

/// The icon, made by `just faraday-hub-icon` from Faraday's mark.
const ICON: &[u8] = include_bytes!("../../assets/icon.png");
/// The menu entries' and the icon's reverse-DNS names (§5.2).
const MENU: &str = names::MENU_ID;
const MENU_DESKTOP: &str = names::MENU_DESKTOP_ID;
/// How long a copy gets to say its version, and how long a started
/// desktop app gets to fail before it counts as started.
const VERSION_WAIT: Duration = Duration::from_secs(5);
const START_WAIT: Duration = Duration::from_millis(400);

/// The folders an install works in: the home folder and the cache.
#[derive(Debug, Clone)]
pub struct Dirs {
    home: PathBuf,
    cache: PathBuf,
}

impl Dirs {
    /// `home` and the cache folder `$XDG_CACHE_HOME`, which is
    /// `home/.cache` when unset.
    pub fn new(home: PathBuf, cache: Option<PathBuf>) -> Dirs {
        let cache = cache.unwrap_or_else(|| home.join(".cache"));
        Dirs { home, cache }
    }

    /// From the environment: `HOME` and `XDG_CACHE_HOME`. A relative
    /// or empty value is not used.
    pub fn from_env() -> Result<Dirs, Reason> {
        let absolute = |name: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
        };
        let home = absolute("HOME").ok_or(Reason::NoHome)?;
        Ok(Dirs::new(home, absolute("XDG_CACHE_HOME")))
    }

    fn bin(&self) -> PathBuf {
        self.home.join(".local/bin").join(names::LINUX_BIN)
    }

    /// An earlier install's hub app.
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

    fn desktop_bin(&self) -> PathBuf {
        self.home.join(".local/bin").join(names::LINUX_DESKTOP_BIN)
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

    /// The cache folder for one release (§5.3).
    pub fn cache_for(&self, version: &str) -> PathBuf {
        self.cache.join("faraday").join(version)
    }

    /// Every file an install puts here, in the order it writes them.
    fn files(&self) -> [(InstallItem, PathBuf); 5] {
        [
            (InstallItem::App, self.bin()),
            (InstallItem::Desktop, self.desktop_bin()),
            (InstallItem::Icon, self.icon()),
            (InstallItem::Menu, self.menu(MENU)),
            (InstallItem::DesktopMenu, self.menu(MENU_DESKTOP)),
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
/// hub app. Runs the installed copy with `--version`; a file there that
/// does not answer as the hub app is not an installed hub app.
pub fn installed(dirs: &Dirs) -> Option<Installed> {
    let version = match faraday_of(&dirs.bin()) {
        Ok((Program::HubApp, build)) => build,
        Ok(_) | Err(_) => return installed_legacy(dirs),
    };
    let items = dirs
        .files()
        .into_iter()
        .filter(|(item, p)| match item {
            InstallItem::App => true,
            InstallItem::Desktop => ours(p, Program::DesktopApp),
            _ => p.is_file(),
        })
        .map(|(item, p)| (item, dirs.shown(&p)))
        .collect();
    Some(Installed { version, items })
}

/// An install made under earlier names, found by its hub app.
fn installed_legacy(dirs: &Dirs) -> Option<Installed> {
    names::LEGACY.iter().find_map(|l| {
        let bin = dirs.legacy_bin(l);
        let version = match faraday_of(&bin) {
            Ok((Program::HubApp, build)) => build,
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
        if bin.exists() && !ours(&bin, Program::HubApp) {
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

/// Installs: a copy of `running` as the hub app, `desktop` (the
/// decompressed, checked desktop app) when given, the icon and the menu
/// entries. Each file is written beside its place and renamed into it,
/// so a copy that is running is replaced whole. The menu entries come
/// last, so none points at a file that is not there. Returns where the
/// hub app went.
pub fn install(dirs: &Dirs, running: &Path, desktop: Option<&[u8]>) -> Result<String, Reason> {
    let app = dirs.bin();
    // A file of either name that is another program's stops the install
    // before anything is written.
    let desktop_bin = dirs.desktop_bin();
    for (path, program, wanted) in [
        (&app, Program::HubApp, true),
        (&desktop_bin, Program::DesktopApp, desktop.is_some()),
    ] {
        if wanted && path.exists() && !ours(path, program) {
            return Err(Reason::InTheWay {
                path: dirs.shown(path),
            });
        }
    }
    place(dirs, &app, 0o755, |to| fs::copy(running, to).map(|_| ()))?;
    if let Some(bytes) = desktop {
        place(dirs, &dirs.desktop_bin(), 0o755, |to| fs::write(to, bytes))?;
    }
    place(dirs, &dirs.icon(), 0o644, |to| fs::write(to, ICON))?;
    let entry = |name: &str, comment: Option<&str>, exec: &Path| {
        let comment = comment.map_or_else(String::new, |c| format!("Comment={c}\n"));
        format!(
            "[Desktop Entry]\nType=Application\nName={name}\n{comment}Exec={}\nIcon={MENU}\nTerminal=false\nCategories=Utility;\n",
            exec_quote(exec)
        )
    };
    place(dirs, &dirs.menu(MENU), 0o644, |to| {
        fs::write(to, entry(EN.hub_menu_name, None, &app))
    })?;
    if desktop.is_some() || ours(&dirs.desktop_bin(), Program::DesktopApp) {
        place(dirs, &dirs.menu(MENU_DESKTOP), 0o644, |to| {
            fs::write(
                to,
                entry(
                    EN.hub_menu_desktop_name,
                    Some(EN.hub_menu_desktop_comment),
                    &dirs.desktop_bin(),
                ),
            )
        })?;
    }
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

/// Uninstalls: removes the files install wrote and the desktop app's
/// cache, and nothing else. A file already gone is not an error.
pub fn uninstall(dirs: &Dirs) -> Result<(), Reason> {
    for (item, path) in dirs.files() {
        let program = match item {
            InstallItem::App => Some(Program::HubApp),
            InstallItem::Desktop => Some(Program::DesktopApp),
            _ => None,
        };
        if program.is_some_and(|p| path.exists() && !ours(&path, p)) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(dirs.file_error(&path, &e)),
        }
    }
    remove_legacy(dirs)?;
    clear_cache(dirs)
}

/// Removes `faraday/X.Y.Z/` folders from the cache folder, then
/// `faraday/` if nothing else is in it.
fn clear_cache(dirs: &Dirs) -> Result<(), Reason> {
    let root = dirs.cache.join("faraday");
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let release = !name.is_empty() && name.split('.').all(|p| p.parse::<u32>().is_ok());
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if release && is_dir {
            fs::remove_dir_all(entry.path()).map_err(|e| dirs.file_error(&entry.path(), &e))?;
        }
    }
    let _ = fs::remove_dir(&root);
    Ok(())
}

/// The desktop app to run (§5.3): the installed one if there is one;
/// otherwise the payload's, in the cache. A cached copy is hashed on
/// every start and decompressed again when its SHA-256 is not the
/// manifest's, or its executable bit is gone.
pub fn desktop_app(dirs: &Dirs, payload: Option<&Payload>) -> Result<PathBuf, Reason> {
    if ours(&dirs.desktop_bin(), Program::DesktopApp) {
        return Ok(dirs.desktop_bin());
    }
    let payload = payload.ok_or(Reason::NoDesktopApp)?;
    let line = payload.desktop_line().ok_or(Reason::NoDesktopApp)?;
    let name = Path::new(&line.inner_name).file_name().map(PathBuf::from);
    let name = name
        .filter(|n| n.as_os_str() == line.inner_name.as_str())
        .ok_or(Reason::NoDesktopApp)?;
    let folder = dirs.cache_for(payload.manifest.version());
    let cached = folder.join(name);
    if cached_is_good(&cached, &line.inner_sha256) {
        return Ok(cached);
    }
    let (_, bytes) = payload.desktop_app()?;
    place(dirs, &cached, 0o755, |to| {
        let mut f = fs::File::create(to)?;
        f.write_all(&bytes)?;
        f.sync_all()
    })?;
    // What is on disk now is what runs: hashed again.
    match sum_of(&cached) {
        Some(sum) if sum == line.inner_sha256 => Ok(cached),
        other => Err(Reason::CacheSum(
            other.map_or_else(|| String::from("unreadable"), |s| s.to_string()),
        )),
    }
}

fn sum_of(path: &Path) -> Option<Sha256Sum> {
    let f = fs::File::open(path).ok()?;
    faraday_payload::hash_image(std::io::BufReader::new(f), ImageKind::Plain)
        .ok()
        .map(|s| s.sha256)
}

fn cached_is_good(path: &Path, want: &Sha256Sum) -> bool {
    let executable = fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o100 != 0);
    executable && sum_of(path).as_ref() == Some(want)
}

/// Starts `path` on its own, in a process group of its own so closing
/// the hub app does not end it, and says why not when it ends at once
/// with a failure.
pub fn start(dirs: &Dirs, path: &Path) -> Result<(), Reason> {
    let fail = |detail: String| Reason::Start {
        path: dirs.shown(path),
        detail,
    };
    let mut child = Command::new(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| fail(e.to_string()))?;
    let started = Instant::now();
    while started.elapsed() < START_WAIT {
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => return Err(fail(status.to_string())),
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(fail(e.to_string())),
        }
    }
    // Reaped when it ends, so it does not linger as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
