//! Staging a release (`faraday/tools/stage-release.sh`, `docs/STARTER.md`
//! §1.3, §5.3) over test builds: what the owner signs holds the stick and
//! Pi images and no desktop file, each Starter's payload holds the images
//! and Faraday Desktop for its own system (F83), and a release folder
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
/// The build ID that release string gives, which each desktop app carries.
const BUILD: &str = "0.9.0-test";

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

const DESKTOP_LINUX: &str = "faraday-desktop-0.9.0-linux-x86_64";
const DESKTOP_WINDOWS: &str = "faraday-desktop-0.9.0-windows-x86_64.exe";

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
    desktop_builds(&builds, BUILD);
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
fn each_payload_holds_the_images_and_its_own_desktop_app_and_the_release_holds_none() {
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
        ]
    );
    // No standalone desktop file in the release folder.
    assert_eq!(
        names(&out),
        ["SHA256SUMS", &format!("{pi}.xz"), &format!("{stick}.xz")]
    );
    // The sums check out from the release folder, as a person runs them.
    let checked = Command::new("sha256sum")
        .args(["-c", "--quiet", "--ignore-missing", "SHA256SUMS"])
        .current_dir(&out)
        .output()
        .unwrap_or_err("sha256sum");
    assert!(checked.status.success(), "{checked:?}");

    for (system, inner, build) in [
        ("linux", DESKTOP_LINUX, "builds/linux/faraday"),
        ("windows", DESKTOP_WINDOWS, "builds/windows/faraday.exe"),
    ] {
        let dir = payload.join(system);
        let xz = format!("{inner}.xz");
        assert_eq!(
            names(&dir),
            [
                "MANIFEST",
                xz.as_str(),
                &format!("{pi}.xz"),
                &format!("{stick}.xz")
            ],
            "{system}"
        );
        let m = Manifest::parse(&fs::read(dir.join("MANIFEST")).expect("MANIFEST"))
            .expect("a manifest");
        let files: Vec<&str> = m.files().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            files,
            [format!("{stick}.xz"), format!("{pi}.xz"), xz.clone()],
            "{system}"
        );
        // The manifest line holds the desktop app as built, and the .xz
        // decompresses to it.
        let built = fs::read(root.join(build)).expect("build");
        let line = m.files().last().expect("the desktop line");
        assert_eq!(line.inner_name, inner);
        assert_eq!(line.inner_size, built.len() as u64);
        let unpacked = Command::new("xz")
            .args(["-d", "-c"])
            .arg(dir.join(&xz))
            .output()
            .unwrap_or_err("xz");
        assert_eq!(unpacked.stdout, built, "{system}");
    }
}

#[test]
fn staging_refuses_a_release_folder_holding_a_desktop_app_and_names_it() {
    let root = fresh("refused");
    let (out, payload) = first_pass(&root);
    // A desktop app left in the release folder, which is not published.
    let left = out.join(format!(
        "faraday-desktop-{VERSION}-testnet-only-linux-x86_64"
    ));
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
        "a release folder with a desktop app was staged"
    );
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("faraday-desktop-"), "{said}");
    // The Starter was not added; what the first pass staged stays.
    assert!(
        !out.join(format!("Faraday-Starter-{VERSION}-Linux.tar.gz"))
            .exists()
    );
    assert!(out.join("SHA256SUMS").is_file());
}

#[test]
fn a_desktop_app_that_does_not_carry_the_images_build_id_is_refused_and_named() {
    let root = fresh("other-build");
    let builds = builds(&root);
    desktop_builds(&builds, "0.9.0+0123456789ab.test");
    let done = stage(&[
        VERSION,
        "--any-build",
        "--builds",
        builds.to_str().expect("a path"),
        "--out",
        root.join("release").to_str().expect("a path"),
        "--payload",
        root.join("payload").to_str().expect("a path"),
    ]);
    assert!(!done.status.success(), "another build was staged");
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(
        said.contains("linux/faraday does not carry the build ID"),
        "{said}"
    );
    assert!(!root.join("release").exists());
}
