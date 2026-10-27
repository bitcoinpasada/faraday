//! Staging a release (`faraday/tools/stage-release.sh`, `docs/STARTER.md`
//! §1.3, §5.3) over test builds: what the owner signs holds the stick and
//! Pi images and the stick's boot program, published on its own for
//! signing on Faraday (F134), each Starter's payload holds the images and its MANIFEST,
//! and no Faraday Demo file is needed or staged anywhere: it is compiled
//! into the Starter (F126). A release folder holding a desktop app is
//! refused by name. The builds are small stand-ins carrying a build ID
//! where the script reads one; the script needs xz, strings and mtools, as
//! on the build machine.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use faraday_payload::Manifest;

const VERSION: &str = "0.9.0";
/// The kernel release string a build carries, as the script reads it.
const RELEASE: &str = "6.6.84-faraday-0.9.0-test";
/// The boot program as the release publishes it (F134).
const BOOT: &str = "faraday-0.9.0-BOOTX64.EFI";

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

/// The stick's boot program stand-in: a kernel carrying the release
/// string, as `EFI/BOOT/BOOTX64.EFI` is.
fn kernel() -> Vec<u8> {
    kernel_of(RELEASE)
}

/// A kernel stand-in carrying `release`.
fn kernel_of(release: &str) -> Vec<u8> {
    let mut kernel = vec![0x5au8; 4096];
    kernel[..2].copy_from_slice(b"MZ");
    let text = format!("Linux version {release} (gcc) #1 SMP");
    kernel[64..64 + text.len()].copy_from_slice(text.as_bytes());
    kernel
}

/// The stick and Pi builds under `builds/`, as the image builds leave
/// them: the stick a GPT disk whose first partition, FAT, holds
/// `EFI/BOOT/BOOTX64.EFI`, the kernel with its release string; the Pi's
/// release string in `faraday-build.txt` on its boot partition, the first
/// MBR partition.
fn builds(root: &Path) -> PathBuf {
    builds_with(root, &kernel())
}

/// The builds with `boot` as the stick's boot program.
fn builds_with(root: &Path, boot: &[u8]) -> PathBuf {
    let builds = root.join("builds");
    fs::create_dir_all(builds.join("stick")).expect("folder");
    fs::create_dir_all(builds.join("pi")).expect("folder");
    let esp = root.join("esp.vfat");
    let esp_s = esp.to_str().expect("a path");
    run("mformat", &["-C", "-f", "1440", "-i", esp_s, "::"]);
    run("mmd", &["-i", esp_s, "::EFI", "::EFI/BOOT"]);
    let k = root.join("kernel");
    fs::write(&k, boot).expect("kernel");
    run(
        "mcopy",
        &[
            "-i",
            esp_s,
            k.to_str().expect("a path"),
            "::EFI/BOOT/BOOTX64.EFI",
        ],
    );
    // A protective MBR, the GPT header at LBA 1 naming its entries at LBA
    // 2, and the first entry starting at LBA 34, where the partition is.
    let mut stick = vec![0u8; 34 * 512];
    stick[446 + 4] = 0xee;
    stick[510] = 0x55;
    stick[511] = 0xaa;
    stick[512..520].copy_from_slice(b"EFI PART");
    stick[512 + 72..512 + 80].copy_from_slice(&2u64.to_le_bytes());
    stick[1024..1040].fill(0x28);
    stick[1024 + 32..1024 + 40].copy_from_slice(&34u64.to_le_bytes());
    // The release string the image builds leave outside the boot program
    // too, ahead of the partition, where the script reads the stick's
    // build ID first.
    let text = format!("Linux version {RELEASE} (gcc) #1 SMP");
    stick[4096..4096 + text.len()].copy_from_slice(text.as_bytes());
    stick.extend(fs::read(&esp).expect("partition"));
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

/// With no Faraday Demo built at all, each payload holds the two images
/// and its MANIFEST, and the release holds the images and SHA256SUMS.
#[test]
fn each_payload_holds_the_images_alone_and_staging_needs_no_faraday_demo() {
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
            BOOT.to_string(),
            stick.clone(),
            pi.clone(),
        ]
    );
    // No desktop file in the release folder.
    assert_eq!(
        names(&out),
        [
            "SHA256SUMS",
            BOOT,
            &format!("{pi}.xz"),
            &format!("{stick}.xz")
        ]
    );
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

/// The boot program a release publishes (F134): the stick image's
/// `EFI/BOOT/BOOTX64.EFI` byte for byte, beside the images and listed in
/// `SHA256SUMS` with its SHA-256, so the sums Faraday checks before
/// signing it name it; and not in the Starter's payload.
#[test]
fn staging_publishes_the_boot_program_and_lists_it_in_the_sums() {
    let root = fresh("boot");
    let (out, payload) = first_pass(&root);
    let published = fs::read(out.join(BOOT)).expect("the boot program is published");
    assert_eq!(published, kernel());
    let sha = Command::new("sha256sum")
        .arg(out.join(BOOT))
        .output()
        .unwrap_or_err("sha256sum");
    let sha = String::from_utf8_lossy(&sha.stdout)
        .split_whitespace()
        .next()
        .expect("a sum")
        .to_string();
    let sums = fs::read_to_string(out.join("SHA256SUMS")).expect("SHA256SUMS");
    assert!(
        sums.lines().any(|l| l == format!("{sha}  {BOOT}")),
        "{sums}"
    );
    assert!(sums.starts_with(&format!("# Faraday {VERSION}, commit ")));
    for system in ["linux", "windows"] {
        assert!(!payload.join(system).join(BOOT).exists(), "{system}");
    }
}

#[test]
fn staging_refuses_a_boot_program_of_another_build() {
    let root = fresh("other-build");
    let builds = builds_with(&root, &kernel_of("6.6.84-faraday-0.8.0-test"));
    let out = root.join("release");
    let done = stage(&[
        VERSION,
        "--any-build",
        "--builds",
        builds.to_str().expect("a path"),
        "--out",
        out.to_str().expect("a path"),
        "--payload",
        root.join("payload").to_str().expect("a path"),
    ]);
    assert!(!done.status.success());
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(
        said.contains("does not carry the build ID 0.9.0-test"),
        "{said}"
    );
    assert!(!out.exists(), "nothing staged");
}

#[test]
fn staging_refuses_a_stick_image_without_a_boot_program() {
    let root = fresh("no-boot");
    let builds = builds(&root);
    // The stick image's partition table wiped: no GPT, no boot program.
    let stick = builds.join("stick/faraday-x86_64-uefi.img");
    let mut bytes = fs::read(&stick).expect("stick");
    bytes[512..520].fill(0);
    fs::write(&stick, bytes).expect("stick");
    let out = root.join("release");
    let done = stage(&[
        VERSION,
        "--any-build",
        "--builds",
        builds.to_str().expect("a path"),
        "--out",
        out.to_str().expect("a path"),
        "--payload",
        root.join("payload").to_str().expect("a path"),
    ]);
    assert!(!done.status.success());
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("has no GPT"), "{said}");
    assert!(!out.exists(), "nothing staged");
}

#[test]
fn staging_refuses_a_release_folder_holding_a_desktop_app_and_names_it() {
    let root = fresh("refused");
    let (out, payload) = first_pass(&root);
    // A desktop app left in the release folder, which is not published.
    let left = out.join(format!("faraday-demo-{VERSION}-testnet-only-linux-x86_64"));
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
    assert!(said.contains("faraday-demo-"), "{said}");
    // The Starter was not added; what the first pass staged stays.
    assert!(
        !out.join(format!("Faraday-Starter-{VERSION}-Linux.tar.gz"))
            .exists()
    );
    assert!(out.join("SHA256SUMS").is_file());
}
