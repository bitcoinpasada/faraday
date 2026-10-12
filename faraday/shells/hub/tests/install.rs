//! Install, uninstall and Try the desktop app on Linux, against a
//! temporary home: what a person finds on their computer afterwards.
//! The "running copy" and the desktop app are small shell scripts, so
//! the tests start what they install.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use faraday_core::strings::EN;
use faraday_hub::Hub;
use faraday_hub::api::{HubEvent, Reason};
use faraday_hub_shell::linux::install::{self, Dirs, exec_quote};
use faraday_hub_shell::payload::Payload;

const DESKTOP_SH: &[u8] = include_bytes!("vectors/desktop-app.sh");
const DESKTOP_XZ: &[u8] = include_bytes!("vectors/desktop-app.sh.xz");
const DESKTOP_XZ_SHA: &str = "a96fbc9a0f71df904781e6ca23e78bbfc0052737dfcf2f49ec289c3178dd078a";
const DESKTOP_SHA: &str = "7b89e3875dc5832ecbbfb9509cf5517200c8e48762c2d983beeaf01ed2094b56";
const XZ_NAME: &str = "faraday-desktop-0.2.0-linux-x86_64.xz";
const INNER: &str = "faraday-desktop-0.2.0-linux-x86_64";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A new home with what a person already has in it, and the dirs.
fn home(name: &str) -> (PathBuf, Dirs) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("hub-install-{name}"));
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

/// A running copy that says it is `version`.
fn running(root: &Path, version: &str) -> PathBuf {
    let p = root.join(format!("running-{version}"));
    fs::write(&p, format!("#!/bin/sh\necho Faraday hub app {version}\n")).expect("script");
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("mode");
    p
}

fn payload(xz: &[u8]) -> Payload {
    let manifest = format!(
        "# Faraday hub payload 0.2.0, commit {COMMIT}\n{XZ_NAME}\t{DESKTOP_XZ_SHA}\t{INNER}\t{DESKTOP_SHA}\t{}\t0.2.0+0123456789ab\n",
        DESKTOP_SH.len()
    );
    Payload::from_parts(
        manifest.as_bytes(),
        vec![(XZ_NAME.to_string(), xz.to_vec())],
    )
    .expect("a payload")
}

const OURS: [&str; 5] = [
    ".local/bin/faraday",
    ".local/bin/faraday-desktop",
    ".local/share/applications/app.faradayoffline.Faraday.desktop",
    ".local/share/applications/app.faradayoffline.FaradayDesktop.desktop",
    ".local/share/icons/hicolor/256x256/apps/app.faradayoffline.Faraday.png",
];

#[test]
fn install_writes_exactly_the_files_the_spec_lists_and_nothing_else() {
    let (root, dirs) = home("files");
    let stub = running(&root, faraday_hub::VERSION);
    let before = tree(&root);

    // Without the desktop app: no desktop app, no menu entry for it.
    let place = install::install(&dirs, &stub, None).expect("installed");
    assert_eq!(place, "~/.local/bin/faraday");
    let mut want = before.clone();
    want.extend([OURS[0], OURS[2], OURS[4]].iter().map(|s| s.to_string()));
    want.sort();
    assert_eq!(tree(&root), want);

    // With it.
    install::install(&dirs, &stub, Some(DESKTOP_SH)).expect("installed");
    let mut want = before;
    want.extend(OURS.iter().map(|s| s.to_string()));
    want.sort();
    assert_eq!(tree(&root), want);

    // What a person opens is executable, and the entries name them.
    for bin in [".local/bin/faraday", ".local/bin/faraday-desktop"] {
        let mode = fs::metadata(root.join(bin))
            .expect("there")
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "{bin}");
    }
    let entry = fs::read_to_string(root.join(OURS[2])).expect("entry");
    assert!(
        entry.contains(&format!("Name={}\n", EN.hub_menu_name)),
        "{entry}"
    );
    assert!(
        entry.contains(&format!("Exec={}/.local/bin/faraday\n", root.display())),
        "{entry}"
    );
    let entry = fs::read_to_string(root.join(OURS[3])).expect("entry");
    assert!(
        entry.contains(&format!("Name={}\n", EN.hub_menu_desktop_name)),
        "{entry}"
    );
    // Not touched.
    assert_eq!(
        fs::read_to_string(root.join("Downloads/received.txt")).expect("kept"),
        "a received file"
    );
    // What is installed, as Home and Installed show it.
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, faraday_hub::VERSION);
    assert_eq!(found.items.len(), 5);
}

#[test]
fn the_menu_entry_quotes_a_path_with_a_space() {
    assert_eq!(
        exec_quote(Path::new("/opt/a b/.local/bin/faraday")),
        "\"/opt/a b/.local/bin/faraday\""
    );
    assert_eq!(
        exec_quote(Path::new("/opt/ab/.local/bin/faraday")),
        "/opt/ab/.local/bin/faraday"
    );
}

#[test]
fn uninstall_removes_exactly_what_install_put_there_and_the_cache() {
    let (root, dirs) = home("uninstall");
    let stub = running(&root, faraday_hub::VERSION);
    let before = tree(&root);
    install::install(&dirs, &stub, Some(DESKTOP_SH)).expect("installed");
    // Also a cached desktop app from an earlier Try the desktop app.
    let cached = dirs.cache_for("0.2.0").join(INNER);
    fs::create_dir_all(cached.parent().expect("folder")).expect("folder");
    fs::write(&cached, DESKTOP_SH).expect("cached");
    assert!(tree(&root).len() > before.len() + 5);

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
fn an_older_installed_copy_is_offered_the_update_and_the_update_keeps_the_desktop_app() {
    let (root, dirs) = home("update");
    let old = running(&root, "0.0.9");
    install::install(&dirs, &old, Some(DESKTOP_SH)).expect("installed");
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, "0.0.9");
    let mut hub = Hub::new();
    hub.hub_event(HubEvent::Installed(Some(found)));
    assert!(hub.update_offered());

    // Updating is installing again from the newer running copy; the
    // desktop app that was installed stays.
    let new = running(&root, faraday_hub::VERSION);
    install::install(&dirs, &new, None).expect("updated");
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, faraday_hub::VERSION);
    assert!(root.join(OURS[1]).is_file());
    hub.hub_event(HubEvent::Installed(Some(found)));
    assert!(!hub.update_offered());
}

#[test]
fn try_the_desktop_app_runs_the_installed_one_when_there_is_one() {
    let (root, dirs) = home("installed-desktop");
    let stub = running(&root, faraday_hub::VERSION);
    install::install(&dirs, &stub, Some(DESKTOP_SH)).expect("installed");
    let path = install::desktop_app(&dirs, Some(&payload(DESKTOP_XZ))).expect("found");
    assert_eq!(path, root.join(OURS[1]));
    install::start(&dirs, &path).expect("started");
    assert_eq!(
        fs::read_to_string(format!("{}.mark", path.display())).expect("ran"),
        "started"
    );
    // Nothing was cached.
    assert!(!root.join(".cache").exists());
}

#[test]
fn try_the_desktop_app_unpacks_it_into_the_cache_and_runs_it() {
    let (root, dirs) = home("cache");
    let payload = payload(DESKTOP_XZ);
    let path = install::desktop_app(&dirs, Some(&payload)).expect("unpacked");
    assert_eq!(path, root.join(".cache/faraday/0.2.0").join(INNER));
    assert_eq!(fs::read(&path).expect("there"), DESKTOP_SH);
    assert_eq!(
        fs::metadata(&path).expect("there").permissions().mode() & 0o111,
        0o111
    );
    install::start(&dirs, &path).expect("started");
    assert_eq!(
        fs::read_to_string(format!("{}.mark", path.display())).expect("ran"),
        "started"
    );
}

#[test]
fn a_cached_desktop_app_whose_sum_changed_is_not_run_but_unpacked_again() {
    let (root, dirs) = home("tampered");
    let payload = payload(DESKTOP_XZ);
    let path = install::desktop_app(&dirs, Some(&payload)).expect("unpacked");
    let mark = PathBuf::from(format!("{}.mark", path.display()));

    // Someone replaces the cached copy with one that does something else.
    fs::write(&path, "#!/bin/sh\nprintf tampered > \"$0.mark\"\n").expect("tampered");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("mode");

    let again = install::desktop_app(&dirs, Some(&payload)).expect("unpacked again");
    assert_eq!(again, path);
    assert_eq!(fs::read(&path).expect("there"), DESKTOP_SH);
    install::start(&dirs, &path).expect("started");
    assert_eq!(fs::read_to_string(mark).expect("ran"), "started");
    let _ = root;
}

#[test]
fn a_payload_that_does_not_match_its_manifest_is_not_cached_or_run() {
    let (root, dirs) = home("damaged");
    let mut damaged = DESKTOP_XZ.to_vec();
    damaged[40] ^= 1;
    let r = install::desktop_app(&dirs, Some(&payload(&damaged)));
    assert_eq!(r, Err(Reason::PayloadSum));
    assert!(!root.join(".cache").exists());
    // And no payload at all.
    assert_eq!(install::desktop_app(&dirs, None), Err(Reason::NoDesktopApp));
}

/// A program of somebody else's, named `faraday` or `faraday-desktop`.
fn foreign(root: &Path, name: &str) -> PathBuf {
    let p = root.join(".local/bin").join(name);
    fs::create_dir_all(p.parent().expect("folder")).expect("folder");
    fs::write(&p, "#!/bin/sh\necho somebody else's tool 1.0\n").expect("script");
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("mode");
    p
}

const THEIRS: &str = "#!/bin/sh\necho somebody else's tool 1.0\n";

#[test]
fn a_faraday_that_is_somebody_elses_is_not_overwritten_and_install_names_its_path() {
    let (root, dirs) = home("foreign-hub");
    let stub = running(&root, faraday_hub::VERSION);
    let theirs = foreign(&root, "faraday");
    let before = tree(&root);

    let r = install::install(&dirs, &stub, Some(DESKTOP_SH));
    assert_eq!(
        r,
        Err(Reason::InTheWay {
            path: String::from("~/.local/bin/faraday")
        })
    );
    // Nothing was written, and it is not reported as an installed hub app.
    assert_eq!(tree(&root), before);
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
    assert_eq!(install::installed(&dirs), None);

    // Uninstall leaves it.
    install::uninstall(&dirs).expect("nothing of ours");
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
}

#[test]
fn a_faraday_desktop_that_is_somebody_elses_is_not_overwritten_or_removed() {
    let (root, dirs) = home("foreign-desktop");
    let stub = running(&root, faraday_hub::VERSION);
    let theirs = foreign(&root, "faraday-desktop");
    let before = tree(&root);

    // Without the desktop app ticked, the hub app installs beside it.
    install::install(&dirs, &stub, None).expect("installed");
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
    let found = install::installed(&dirs).expect("installed");
    assert!(
        found
            .items
            .iter()
            .all(|(_, shown)| shown != "~/.local/bin/faraday-desktop"),
        "{found:?}"
    );

    // With it ticked, install says the file is in the way.
    let r = install::install(&dirs, &stub, Some(DESKTOP_SH));
    assert_eq!(
        r,
        Err(Reason::InTheWay {
            path: String::from("~/.local/bin/faraday-desktop")
        })
    );
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);

    // Try the desktop app does not run it; it unpacks its own.
    let path = install::desktop_app(&dirs, Some(&payload(DESKTOP_XZ))).expect("unpacked");
    assert_ne!(path, theirs);

    install::uninstall(&dirs).expect("uninstalled");
    assert_eq!(fs::read_to_string(&theirs).expect("kept"), THEIRS);
    let mut left = tree(&root);
    left.retain(|f| !f.starts_with(".cache"));
    let mut want = before;
    want.sort();
    assert_eq!(left, want);
}

#[test]
fn our_own_copies_are_replaced_on_update_and_removed_on_uninstall() {
    let (root, dirs) = home("ours");
    let old = running(&root, "0.0.9");
    install::install(&dirs, &old, Some(DESKTOP_SH)).expect("installed");
    let new = running(&root, faraday_hub::VERSION);
    install::install(&dirs, &new, Some(DESKTOP_SH)).expect("updated over our own");
    let found = install::installed(&dirs).expect("installed");
    assert_eq!(found.version, faraday_hub::VERSION);
    assert_eq!(found.items.len(), 5);
    install::uninstall(&dirs).expect("uninstalled");
    assert!(!root.join(OURS[0]).exists());
    assert!(!root.join(OURS[1]).exists());
}

#[test]
fn a_file_that_is_not_executable_or_prints_a_near_miss_is_not_ours() {
    let (root, dirs) = home("near-miss");
    let stub = running(&root, faraday_hub::VERSION);
    let bin = root.join(OURS[0]);
    fs::create_dir_all(bin.parent().expect("folder")).expect("folder");
    for body in [
        "#!/bin/sh\necho Faraday hub app\n",
        "#!/bin/sh\necho Faraday hub appliance 1\n",
    ] {
        fs::write(&bin, body).expect("script");
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("mode");
        assert!(matches!(
            install::install(&dirs, &stub, None),
            Err(Reason::InTheWay { .. })
        ));
    }
    fs::write(&bin, "#!/bin/sh\necho Faraday hub app 1\n").expect("script");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o644)).expect("mode");
    assert!(matches!(
        install::install(&dirs, &stub, None),
        Err(Reason::InTheWay { .. })
    ));
}
