//! Install and uninstall on Linux, against a temporary home: what a
//! person finds on their computer afterwards. The "running copy" and an
//! earlier Starter's Faraday Demo are small shell scripts that answer
//! `--version`.

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use faraday_core::names;
use faraday_core::strings::EN;
use faraday_core::{Program, version_line};
use faraday_starter::Starter;
use faraday_starter::api::{Reason, StarterEvent};
use faraday_starter_shell::linux::install::{self, Dirs, exec_quote};

/// A new home with what a person already has in it, and the dirs.
fn home(name: &str) -> (PathBuf, Dirs) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("starter-install-{name}"));
    let _ = fs::remove_dir_all(&root);
    for (file, text) in [
        ("Downloads/received.txt", "a received file"),
        ("faraday-sticks/BOOTSTICK/vault.ofv", "a vault"),
        (".config/other/settings", "somebody else's"),
    ] {
        let p = root.join(file);
        fs::create_dir_all(p.parent().expect("a folder")).expect("folder");
        fs::write(p, text).expect("file");
    }
    let dirs = Dirs::new(root.clone(), None);
    (root, dirs)
}

/// Every file under `root`, relative and sorted.
fn tree(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).expect("readable") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                out.push(p.strip_prefix(root).expect("under").display().to_string());
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// A script at `path` that answers `--version` with `said`.
fn answering(path: &Path, said: &str) {
    fs::create_dir_all(path.parent().expect("a folder")).expect("folder");
    fs::write(path, format!("#!/bin/sh\necho {said}\n")).expect("script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("mode");
}

/// A running copy that says it is `version`.
fn running(root: &Path, version: &str) -> PathBuf {
    let p = root.join(format!("running-{version}"));
    answering(&p, &version_line(Program::StarterApp, version));
    p
}

static OURS: LazyLock<[String; 3]> = LazyLock::new(|| {
    [
        format!(".local/bin/{}", names::LINUX_BIN),
        format!(".local/share/applications/{}.desktop", names::MENU_ID),
        format!(
            ".local/share/icons/hicolor/256x256/apps/{}.png",
            names::MENU_ID
        ),
    ]
});

/// Faraday Demo runs inside the Starter's window (F126): install writes
/// the Starter, its menu entry and its icon, and no Faraday Demo.
#[test]
fn install_writes_exactly_the_files_the_spec_lists_and_nothing_else() {
    let (root, dirs) = home("files");
    let stub = running(&root, faraday_starter::VERSION);
    let before = tree(&root);

    let place = install::install(&dirs, &stub).expect("installed");
    assert_eq!(place, format!("~/.local/bin/{}", names::LINUX_BIN));
    let mut want = before;
    want.extend(OURS.iter().cloned());
    want.sort();
    assert_eq!(tree(&root), want);

    // What a person opens is executable, and the entry names it.
    let mode = fs::metadata(root.join(&OURS[0]))
        .expect("there")
        .permissions()
        .mode();
    assert_eq!(mode & 0o111, 0o111);
    let entry = fs::read_to_string(root.join(&OURS[1])).expect("entry");
    assert!(
        entry.contains(&format!("Name={}\n", EN.starter_menu_name)),
        "{entry}"
    );
    assert!(
        entry.contains(&format!("Exec={}/{}\n", root.display(), OURS[0])),
        "{entry}"
    );
    // Not touched.
    assert_eq!(
        fs::read_to_string(root.join("Downloads/received.txt")).expect("kept"),
        "a received file"
    );
    // What is installed, as Home and Installed show it.
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, faraday_starter::VERSION);
    assert_eq!(found.items.len(), 3);
}

#[test]
fn the_menu_entry_quotes_a_path_with_a_space() {
    assert_eq!(
        exec_quote(Path::new("/opt/a b/.local/bin/faraday-starter")),
        "\"/opt/a b/.local/bin/faraday-starter\""
    );
    assert_eq!(
        exec_quote(Path::new("/opt/ab/.local/bin/faraday-starter")),
        "/opt/ab/.local/bin/faraday-starter"
    );
}

#[test]
fn uninstall_removes_exactly_what_install_put_there_and_the_cache() {
    let (root, dirs) = home("uninstall");
    let stub = running(&root, faraday_starter::VERSION);
    let before = tree(&root);
    install::install(&dirs, &stub).expect("installed");
    // Also a cached Faraday Demo from an earlier Starter's Open Faraday
    // Demo.
    let cached = dirs
        .cache_for("0.1.7")
        .join("faraday-demo-0.1.7-linux-x86_64");
    answering(&cached, &version_line(Program::DesktopApp, "0.1.7"));
    assert!(tree(&root).len() > before.len() + 3);

    install::uninstall(&dirs).expect("uninstalled");
    assert_eq!(tree(&root), before);
    assert_eq!(
        fs::read_to_string(root.join("faraday-sticks/BOOTSTICK/vault.ofv")).expect("kept"),
        "a vault"
    );
    assert_eq!(install::installed(&dirs), None);
    // A second one has nothing to do and says nothing.
    install::uninstall(&dirs).expect("nothing to remove");
}

#[test]
fn an_older_installed_copy_is_offered_the_update() {
    let (root, dirs) = home("update");
    let old = running(&root, "0.0.9");
    install::install(&dirs, &old).expect("installed");
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, "0.0.9");
    let mut starter = Starter::new();
    starter.starter_event(StarterEvent::Installed(Some(found)));
    assert!(starter.update_offered());

    // Updating is installing again from the newer running copy.
    let new = running(&root, faraday_starter::VERSION);
    install::install(&dirs, &new).expect("updated");
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, faraday_starter::VERSION);
    starter.starter_event(StarterEvent::Installed(Some(found)));
    assert!(!starter.update_offered());
}

/// A program of somebody else's, named `faraday-starter` or `faraday-demo`.
fn foreign(root: &Path, name: &str) -> PathBuf {
    let p = root.join(".local/bin").join(name);
    answering(&p, "somebody else's tool 1.0");
    p
}

const THEIRS: &str = "#!/bin/sh\necho somebody else's tool 1.0\n";

#[test]
fn a_faraday_starter_that_is_somebody_elses_is_not_overwritten_and_install_names_its_path() {
    let (root, dirs) = home("foreign-starter");
    let stub = running(&root, faraday_starter::VERSION);
    let theirs = foreign(&root, names::LINUX_BIN);
    let before = tree(&root);

    let r = install::install(&dirs, &stub);
    assert_eq!(
        r,
        Err(Reason::InTheWay {
            path: format!("~/.local/bin/{}", names::LINUX_BIN)
        })
    );
    // Nothing was written, and it is not reported as an installed Starter.
    assert_eq!(tree(&root), before);
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
    assert_eq!(install::installed(&dirs), None);

    // Uninstall leaves it.
    install::uninstall(&dirs).expect("nothing of ours");
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
}

#[test]
fn our_own_copy_is_replaced_on_update_and_removed_on_uninstall() {
    let (root, dirs) = home("ours");
    let old = running(&root, "0.0.9");
    install::install(&dirs, &old).expect("installed");
    let new = running(&root, faraday_starter::VERSION);
    install::install(&dirs, &new).expect("updated over our own");
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, faraday_starter::VERSION);
    assert_eq!(found.items.len(), 3);
    install::uninstall(&dirs).expect("uninstalled");
    assert!(!root.join(&OURS[0]).exists());
}

#[test]
fn a_file_that_is_not_executable_or_prints_a_near_miss_is_not_ours() {
    let (root, dirs) = home("near-miss");
    let stub = running(&root, faraday_starter::VERSION);
    let bin = root.join(&OURS[0]);
    fs::create_dir_all(bin.parent().expect("folder")).expect("folder");
    for body in [
        &format!("#!/bin/sh\necho {}\n", EN.name_faraday_starter_app),
        &format!("#!/bin/sh\necho {}s 1\n", EN.name_faraday_starter_app),
    ] {
        fs::write(&bin, body).expect("script");
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("mode");
        assert!(matches!(
            install::install(&dirs, &stub),
            Err(Reason::InTheWay { .. })
        ));
    }
    fs::write(
        &bin,
        format!("#!/bin/sh\necho {} 1\n", EN.name_faraday_starter_app),
    )
    .expect("script");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o644)).expect("mode");
    assert!(matches!(
        install::install(&dirs, &stub),
        Err(Reason::InTheWay { .. })
    ));
}

/// What an earlier install (`names::LEGACY[which]`) left: its Starter
/// answering `--version` with the words of its time, its menu entry and
/// icon, and the desktop app's entry of its time.
fn old_install(root: &Path, which: usize) -> Vec<PathBuf> {
    let l = &names::LEGACY[which];
    let said = ["Faraday hub app", "Faraday Hub"][which];
    let files = [
        (
            format!(".local/bin/{}", l.linux_bin),
            format!("#!/bin/sh\necho {said} 0.1.9\n"),
        ),
        (
            format!(".local/share/applications/{}.desktop", l.menu_id),
            String::from("[Desktop Entry]\nName=Faraday\n"),
        ),
        (
            format!(".local/share/icons/hicolor/256x256/apps/{}.png", l.menu_id),
            String::from("png"),
        ),
        (
            format!(
                ".local/share/applications/{}.desktop",
                names::MENU_DESKTOP_ID
            ),
            String::from("[Desktop Entry]\nName=Faraday desktop (testing)\n"),
        ),
    ];
    files
        .iter()
        .map(|(file, text)| {
            let p = root.join(file);
            fs::create_dir_all(p.parent().expect("folder")).expect("folder");
            fs::write(&p, text).expect("file");
            if file.contains("bin/") {
                fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("mode");
            }
            p
        })
        .collect()
}

/// The owner's own launcher, which no install touches.
fn own_launcher(root: &Path) -> PathBuf {
    let p = root.join(".local/share/applications/faraday.desktop");
    fs::create_dir_all(p.parent().expect("folder")).expect("folder");
    fs::write(&p, "[Desktop Entry]\nName=Faraday-dev\n").expect("file");
    p
}

#[test]
fn install_replaces_an_earlier_install_and_uninstall_removes_one() {
    for which in 0..names::LEGACY.len() {
        let (root, dirs) = home(&format!("old-names-{which}"));
        let old = old_install(&root, which);
        let launcher = own_launcher(&root);
        let stub = running(&root, faraday_starter::VERSION);

        // The old install is found as installed, by its old Starter.
        let found = install::installed(&dirs).expect("old install found");
        assert_eq!(found.version, "0.1.9");

        // Install puts the new names there and takes the old ones away.
        install::install(&dirs, &stub).expect("installed");
        for p in &old {
            assert!(!p.exists(), "{}", p.display());
        }
        for f in OURS.iter() {
            assert!(root.join(f).is_file(), "{f}");
        }
        assert!(launcher.is_file());

        // Uninstall of an earlier install alone.
        let (root, dirs) = home(&format!("old-names-uninstall-{which}"));
        let old = old_install(&root, which);
        let launcher = own_launcher(&root);
        install::uninstall(&dirs).expect("uninstalled");
        for p in &old {
            assert!(!p.exists(), "{}", p.display());
        }
        assert!(launcher.is_file());
        assert_eq!(install::installed(&dirs), None);
    }
}

#[test]
fn a_faraday_that_is_somebody_elses_stays_beside_the_new_install() {
    let (root, dirs) = home("foreign-old-name");
    let theirs = foreign(&root, "faraday");
    let launcher = own_launcher(&root);
    let stub = running(&root, faraday_starter::VERSION);

    install::install(&dirs, &stub).expect("installed");
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
    assert!(launcher.is_file());
    install::uninstall(&dirs).expect("uninstalled");
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
    assert!(launcher.is_file());
}

/// Faraday Demo as an earlier Starter installed it beside itself, under
/// the names `l` gives, answering `--version` with `said`: its file, its
/// menu entry, and the cache it ran from.
fn old_demo(root: &Path, dirs: &Dirs, l: &names::LegacyDesktop, said: &str) -> [PathBuf; 3] {
    let bin = root.join(".local/bin").join(l.linux_bin);
    let entry = root.join(format!(".local/share/applications/{}.desktop", l.menu_id));
    let cached = dirs
        .cache_for("0.1.7")
        .join("faraday-demo-0.1.7-linux-x86_64");
    answering(&bin, said);
    answering(&cached, said);
    fs::create_dir_all(entry.parent().expect("a folder")).expect("folder");
    fs::write(&entry, "[Desktop Entry]\nName=Faraday Demo\n").expect("entry");
    [bin, entry, cached]
}

/// What an earlier Starter's Faraday Demo answers `--version` with, under
/// the names `l` gives: Faraday Demo's words, or before them Faraday
/// Desktop's.
fn demo_words(l: &names::LegacyDesktop) -> String {
    if l.linux_bin == "faraday-demo" {
        version_line(Program::DesktopApp, "0.1.7")
    } else {
        String::from("Faraday Desktop 0.1.9")
    }
}

/// Faraday Demo runs inside the Starter's window (F126): install and
/// update remove the Faraday Demo an earlier Starter installed beside
/// itself, under its name of either era, with its menu entry and the
/// cache it ran from, when it answers `--version` as Faraday's; uninstall
/// alone removes it too.
#[test]
fn an_earlier_starters_faraday_demo_is_removed_by_install_and_uninstall() {
    for (i, l) in names::LEGACY_DESKTOP.iter().enumerate() {
        let (root, dirs) = home(&format!("old-demo-{i}"));
        let before = tree(&root);
        let old = old_demo(&root, &dirs, l, &demo_words(l));
        let stub = running(&root, faraday_starter::VERSION);
        install::install(&dirs, &stub).expect("installed");
        for p in &old {
            assert!(!p.exists(), "{}", p.display());
        }
        let mut want = before.clone();
        want.extend(OURS.iter().cloned());
        want.push(format!("running-{}", faraday_starter::VERSION));
        want.sort();
        assert_eq!(tree(&root), want);

        // Uninstall alone finds it too.
        let (root, dirs) = home(&format!("old-demo-uninstall-{i}"));
        let before = tree(&root);
        let old = old_demo(&root, &dirs, l, &demo_words(l));
        install::uninstall(&dirs).expect("uninstalled");
        for p in &old {
            assert!(!p.exists(), "{}", p.display());
        }
        assert_eq!(tree(&root), before);
    }
}

/// A file of Faraday Demo's names that does not answer as Faraday's is
/// another program's: install and uninstall leave it.
#[test]
fn a_faraday_demo_that_is_somebody_elses_stays_beside_the_new_install() {
    for (i, l) in names::LEGACY_DESKTOP.iter().enumerate() {
        let (root, dirs) = home(&format!("foreign-demo-{i}"));
        let theirs = foreign(&root, l.linux_bin);
        let stub = running(&root, faraday_starter::VERSION);

        install::install(&dirs, &stub).expect("installed");
        assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
        let found = install::installed(&dirs).expect("installed");
        assert!(
            found
                .items
                .iter()
                .all(|(_, shown)| !shown.ends_with(l.linux_bin)),
            "{found:?}"
        );
        install::uninstall(&dirs).expect("uninstalled");
        assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
    }
}
