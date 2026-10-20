//! Staging a release (`faraday/tools/stage-release.sh`, `docs/DEMO.md`
//! §9.2) over test builds: what the owner signs holds the stick and Pi
//! images and Faraday Desktop's two files under their testnet-only names
//! (F61), what each Starter embeds holds the images only, and a payload
//! holding a desktop app is refused by name. The builds are small
//! stand-ins carrying a build ID where the script reads one; the script
//! needs xz, strings and mtools, as on the build machine.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use faraday_payload::Manifest;

const VERSION: &str = "0.9.0";
/// The kernel release string a build carries, as the script reads it.
const RELEASE: &str = "6.6.84-faraday-0.9.0-test";

fn fresh(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("stage-release-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("folder");
    root
}

fn run(program: &str, args: &[&str]) -> Output {
    let out = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_err(program);
    assert!(
        out.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

trait OrErr<T> {
    fn unwrap_or_err(self, what: &str) -> T;
}

impl<T> OrErr<T> for std::io::Result<T> {
    fn unwrap_or_err(self, what: &str) -> T {
        self.unwrap_or_else(|e| panic!("{what}: {e} (it is needed to stage a release)"))
    }
}

/// The stick and Pi builds under `builds/`, as the image builds leave
/// them: the stick's kernel release string in its bytes, the Pi's in
/// `faraday-build.txt` on its boot partition, the first MBR partition.
fn builds(root: &Path) -> PathBuf {
    let builds = root.join("builds");
    fs::create_dir_all(builds.join("stick")).expect("folder");
    fs::create_dir_all(builds.join("pi")).expect("folder");
    let mut stick = vec![0x5au8; 300_000];
    let text = format!("Linux version {RELEASE} (gcc) #1 SMP");
    stick[100_000..100_000 + text.len()].copy_from_slice(text.as_bytes());
    fs::write(builds.join("stick/faraday-x86_64-uefi.img"), &stick).expect("stick");

    let part = root.join("boot.vfat");
    let part_s = part.to_str().expect("a path");
    run("mformat", &["-C", "-f", "1440", "-i", part_s, "::"]);
    let note = root.join("faraday-build.txt");
    fs::write(&note, format!("{RELEASE}\n")).expect("note");
    run(
        "mcopy",
        &[
            "-i",
            part_s,
            note.to_str().expect("a path"),
            "::faraday-build.txt",
        ],
    );
    let mut pi = vec![0u8; 512];
    // Partition 1: FAT32 with LBA, starting at sector 1.
    pi[446 + 4] = 0x0c;
    pi[454..458].copy_from_slice(&1u32.to_le_bytes());
    pi[510] = 0x55;
    pi[511] = 0xaa;
    pi.extend(fs::read(&part).expect("partition"));
    fs::write(builds.join("pi/faraday-pi3-waveshare-28dpi.img"), &pi).expect("pi");
    builds
}

/// Faraday Desktop's two builds under `builds/`, as the container builds
/// leave them, each carrying `build` as its build ID.
fn desktop_builds(builds: &Path, build: &str) {
    fs::create_dir_all(builds.join("linux")).expect("folder");
    fs::create_dir_all(builds.join("windows")).expect("folder");
    let app = format!("Faraday Desktop {build}\n").repeat(100);
    fs::write(builds.join("linux/faraday"), &app).expect("linux");
    fs::write(builds.join("windows/faraday.exe"), &app).expect("windows");
}

const DESKTOP_LINUX: &str = "faraday-desktop-0.9.0-testnet-only-linux-x86_64";
const DESKTOP_WINDOWS: &str = "faraday-desktop-0.9.0-testnet-only-windows-x86_64.exe";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/stage-release.sh")
}

fn stage(args: &[&str]) -> Output {
    Command::new("bash")
        .arg(script())
        .args(args)
        .output()
        .unwrap_or_err("bash")
}

/// A first pass over the test builds. Returns the release and payload
/// folders.
fn first_pass(root: &Path) -> (PathBuf, PathBuf) {
    let builds = builds(root);
    desktop_builds(&builds, "0.9.0+0123456789ab.test");
    let (out, payload) = (root.join("release"), root.join("payload"));
    let done = stage(&[
        VERSION,
        "--any-build",
        "--builds",
        builds.to_str().expect("a path"),
        "--out",
        out.to_str().expect("a path"),
        "--payload",
        payload.to_str().expect("a path"),
    ]);
    assert!(
        done.status.success(),
        "staging failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
    (out, payload)
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .expect("a folder")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn a_staged_release_lists_the_desktop_files_and_its_payloads_hold_only_the_images() {
    let root = fresh("staged");
    let (out, payload) = first_pass(&root);
    let stick = format!("faraday-stick-{VERSION}-x86_64-uefi.img");
    let pi = format!("faraday-pi-{VERSION}-pi3-waveshare-28dpi.img");

    let sums = fs::read_to_string(out.join("SHA256SUMS")).expect("SHA256SUMS");
    let listed: Vec<&str> = sums
        .lines()
        .skip(1)
        .filter_map(|l| l.split_once("  ").map(|(_, name)| name))
        .collect();
    assert_eq!(
        listed,
        [
            format!("{stick}.xz"),
            format!("{pi}.xz"),
            stick.clone(),
            pi.clone(),
            DESKTOP_LINUX.to_string(),
            DESKTOP_WINDOWS.to_string(),
        ]
    );
    assert_eq!(
        names(&out),
        [
            "SHA256SUMS",
            DESKTOP_LINUX,
            DESKTOP_WINDOWS,
            &format!("{pi}.xz"),
            &format!("{stick}.xz")
        ]
    );
    // Each published as built, and the Linux one runs.
    let built = fs::read(root.join("builds/linux/faraday")).expect("build");
    assert_eq!(fs::read(out.join(DESKTOP_LINUX)).expect("staged"), built);
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(out.join(DESKTOP_LINUX))
        .expect("staged")
        .permissions()
        .mode();
    assert_eq!(mode & 0o111, 0o111);
    // The sums check out from the release folder, as a person runs them.
    let checked = Command::new("sha256sum")
        .args(["-c", "--quiet", "--ignore-missing", "SHA256SUMS"])
        .current_dir(&out)
        .output()
        .unwrap_or_err("sha256sum");
    assert!(checked.status.success(), "{checked:?}");

    for system in ["linux", "windows"] {
        let dir = payload.join(system);
        assert_eq!(
            names(&dir),
            ["MANIFEST", &format!("{pi}.xz"), &format!("{stick}.xz")],
            "{system}"
        );
        let m = Manifest::parse(&fs::read(dir.join("MANIFEST")).expect("MANIFEST"))
            .expect("a manifest");
        let files: Vec<&str> = m.files().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            files,
            [format!("{stick}.xz"), format!("{pi}.xz")],
            "{system}"
        );
    }
}

#[test]
fn staging_refuses_a_payload_holding_a_desktop_app_and_names_it() {
    let root = fresh("refused");
    let (out, payload) = first_pass(&root);
    // A desktop app put in a Starter's payload, which no Starter carries.
    let left = payload.join(format!("linux/{DESKTOP_LINUX}"));
    fs::write(&left, "a desktop app").expect("file");
    // A Starter built against the payload: it carries its MANIFEST's
    // first line.
    let first = fs::read_to_string(payload.join("linux/MANIFEST"))
        .expect("MANIFEST")
        .lines()
        .next()
        .expect("a first line")
        .to_string();
    let starter = root.join("faraday-starter");
    fs::write(&starter, format!("a Starter\n{first}\n")).expect("starter");

    let done = stage(&[
        VERSION,
        "--starter",
        starter.to_str().expect("a path"),
        "--out",
        out.to_str().expect("a path"),
        "--payload",
        payload.to_str().expect("a path"),
    ]);
    assert!(
        !done.status.success(),
        "a payload with a desktop app was staged"
    );
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains(DESKTOP_LINUX), "{said}");
    // The Starter was not added; what the first pass staged stays.
    assert!(
        !out.join(format!("Faraday-Starter-{VERSION}-Linux.tar.gz"))
            .exists()
    );
    assert!(out.join("SHA256SUMS").is_file());
}

#[test]
fn a_release_refuses_a_desktop_test_build_and_names_it() {
    let root = fresh("test-desktop");
    let builds = builds(&root);
    desktop_builds(&builds, "0.9.0+0123456789ab.test");
    let done = stage(&[
        VERSION,
        "--builds",
        builds.to_str().expect("a path"),
        "--out",
        root.join("release").to_str().expect("a path"),
        "--payload",
        root.join("payload").to_str().expect("a path"),
    ]);
    assert!(
        !done.status.success(),
        "a test build was staged as a release"
    );
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("linux/faraday is a test build"), "{said}");
    assert!(!root.join("release").exists());
}
