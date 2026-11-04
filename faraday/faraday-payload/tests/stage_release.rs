//! Staging a release (`faraday/tools/stage-release.sh`, `docs/STARTER.md`
//! §1.3, §5.3, `docs/IMAGES.md` §4) over test builds: what the owner signs
//! holds every image `faraday/builds.toml` releases, the PC stick's boot
//! programs, published on their own for signing on Faraday (F134), and the
//! release's image list, whose lines and `SHA256SUMS` check a download, the
//! image inside it and a card or stick already written; images of two
//! builds are refused. The Starter carries no image, only that list: a
//! Starter built without it is not added. No Faraday Demo file is
//! needed or staged anywhere: it is compiled into the Starter (F126). A
//! release folder holding a desktop app is refused by name. The builds are
//! small stand-ins carrying a build ID where the script reads one; the
//! script needs python3, xz, strings and mtools, as on the build machine.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

const VERSION: &str = "0.9.0";
/// The kernel release string a build carries, as the script reads it.
const RELEASE: &str = "6.6.84-faraday-0.9.0-test";
/// The boot programs as the release publishes them (F134, IMAGES §4).
const BOOT: &str = "faraday-0.9.0-BOOTX64.EFI";

/// One image a release publishes: where its build leaves it under
/// `builds/`, its release name without `.xz`, and its line in `IMAGES`
/// after the two sums (platform, screen, the boards it starts).
struct Image {
    built: &'static str,
    released: &'static str,
    listed: [&'static str; 3],
}

const PI_BOARDS: &str = "Pi 3B, Pi 3B+, Pi Zero 2 W";
const PC_BOARDS: &str = "a 64-bit PC with UEFI firmware";

/// Every image a release publishes, in `builds.toml`'s order.
const IMAGES: [Image; 2] = [
    Image {
        built: "stick/faraday-x86_64-uefi.img",
        released: "faraday-stick-0.9.0-x86_64-uefi.img",
        listed: ["x86_64-uefi", "efi-framebuffer", PC_BOARDS],
    },
    Image {
        built: "pi/faraday-pi3-any.img",
        released: "faraday-pi-0.9.0.img",
        listed: ["pi3", "any", PI_BOARDS],
    },
];

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

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
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

/// The builds under `builds/`, as the image builds leave them: each stick
/// a GPT disk whose first partition, FAT, holds `EFI/BOOT/BOOTX64.EFI`,
/// the kernel with its release string; each Pi image's release string in
/// `faraday-build.txt` on its boot partition, the first MBR partition.
fn builds(root: &Path) -> PathBuf {
    builds_with(root, &kernel(), RELEASE)
}

/// The builds with `boot` as the stick's boot program, and the Pi
/// image carrying `pi`.
fn builds_with(root: &Path, boot: &[u8], pi: &str) -> PathBuf {
    let builds = root.join("builds");
    fs::create_dir_all(builds.join("stick")).expect("folder");
    fs::create_dir_all(builds.join("pi")).expect("folder");
    fs::write(builds.join(IMAGES[0].built), stick_image(root, boot)).expect("stick");
    fs::write(
        builds.join(IMAGES[1].built),
        pi_image(root, IMAGES[1].built, pi),
    )
    .expect("pi");
    builds
}

/// A stick image whose boot program is `boot`.
fn stick_image(root: &Path, boot: &[u8]) -> Vec<u8> {
    let esp = root.join("esp.vfat");
    let _ = fs::remove_file(&esp);
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
    let esp = fs::read(&esp).expect("partition");
    // A protective MBR, the GPT header at LBA 1 naming its entries at LBA
    // 2, and the first entry from LBA 34, where the partition is, to its
    // last sector.
    let last = 34 + esp.len() as u64 / 512 - 1;
    let mut stick = vec![0u8; 34 * 512];
    stick[446 + 4] = 0xee;
    stick[510] = 0x55;
    stick[511] = 0xaa;
    stick[512..520].copy_from_slice(b"EFI PART");
    stick[512 + 72..512 + 80].copy_from_slice(&2u64.to_le_bytes());
    stick[1024..1040].fill(0x28);
    stick[1024 + 32..1024 + 40].copy_from_slice(&34u64.to_le_bytes());
    stick[1024 + 40..1024 + 48].copy_from_slice(&last.to_le_bytes());
    // The release string the image builds leave outside the boot program
    // too, ahead of the partition, where the script reads the stick's
    // build ID first.
    let text = format!("Linux version {RELEASE} (gcc) #1 SMP");
    stick[4096..4096 + text.len()].copy_from_slice(text.as_bytes());
    stick.extend(esp);
    // A data partition after it, which a person's use changes.
    stick.extend([0x44u8; 4096]);
    stick
}

/// A Pi image carrying `release`, its boot partition holding `name` too so
/// that each image differs.
fn pi_image(root: &Path, name: &str, release: &str) -> Vec<u8> {
    let part = root.join("boot.vfat");
    let _ = fs::remove_file(&part);
    let part_s = part.to_str().expect("a path");
    run("mformat", &["-C", "-f", "1440", "-i", part_s, "::"]);
    let note = root.join("faraday-build.txt");
    fs::write(&note, format!("{release}\n")).expect("note");
    let panel = root.join("panel.txt");
    fs::write(&panel, name).expect("panel");
    for (file, on) in [(&note, "::faraday-build.txt"), (&panel, "::panel.txt")] {
        run("mcopy", &["-i", part_s, file.to_str().expect("a path"), on]);
    }
    let part = fs::read(&part).expect("partition");
    let mut pi = vec![0u8; 512];
    // Partition 1: FAT32 with LBA, from sector 1, its length in sectors.
    pi[446 + 4] = 0x0c;
    pi[454..458].copy_from_slice(&1u32.to_le_bytes());
    pi[458..462].copy_from_slice(&(part.len() as u32 / 512).to_le_bytes());
    pi[510] = 0x55;
    pi[511] = 0xaa;
    pi.extend(part);
    pi.extend([0x44u8; 4096]);
    pi
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

/// A first pass over `builds`. Returns what the script did.
fn stage_builds(root: &Path, builds: &Path) -> Output {
    stage(&[
        VERSION,
        "--any-build",
        "--builds",
        builds.to_str().expect("a path"),
        "--out",
        root.join("release").to_str().expect("a path"),
    ])
}

/// A first pass over the test builds. Returns the release folder.
fn first_pass(root: &Path) -> PathBuf {
    let builds = builds(root);
    let done = stage_builds(root, &builds);
    assert!(
        done.status.success(),
        "staging failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
    root.join("release")
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .expect("a folder")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// The lines of `SHA256SUMS` after its first: (sum, name).
fn sums(out: &Path) -> Vec<(String, String)> {
    fs::read_to_string(out.join("SHA256SUMS"))
        .expect("SHA256SUMS")
        .lines()
        .skip(1)
        .map(|l| {
            let (sum, name) = l.split_once("  ").expect("a sum and a name");
            (sum.to_string(), name.to_string())
        })
        .collect()
}

/// The image inside a published `.xz`, as a person decompresses it.
fn decompressed(xz: &Path) -> Vec<u8> {
    run("xz", &["-d", "-c", xz.to_str().expect("a path")]).stdout
}

/// The bytes of a written card or stick's first partition, as a person
/// reads its device (`/dev/sdX1`): the stand-ins' layout.
fn boot_partition(image: &[u8]) -> &[u8] {
    if &image[512..520] == b"EFI PART" {
        let first = u64::from_le_bytes(image[1056..1064].try_into().expect("8")) as usize;
        let last = u64::from_le_bytes(image[1064..1072].try_into().expect("8")) as usize;
        &image[first * 512..(last + 1) * 512]
    } else {
        let first = u32::from_le_bytes(image[454..458].try_into().expect("4")) as usize;
        let count = u32::from_le_bytes(image[458..462].try_into().expect("4")) as usize;
        &image[first * 512..(first + count) * 512]
    }
}

/// With no Faraday Demo built at all, the release holds every image, the
/// boot program, `IMAGES` and `SHA256SUMS`, and nothing is staged
/// beside the release folder.
#[test]
fn the_release_holds_the_images_and_staging_needs_no_faraday_demo() {
    let root = fresh("staged");
    let out = first_pass(&root);

    let listed: Vec<String> = sums(&out).into_iter().map(|(_, name)| name).collect();
    let mut want: Vec<String> = IMAGES
        .iter()
        .map(|i| format!("{}.xz", i.released))
        .collect();
    want.extend([BOOT.to_string(), "IMAGES".into()]);
    want.extend(IMAGES.iter().map(|i| i.released.to_string()));
    want.extend(
        IMAGES
            .iter()
            .map(|i| format!("{}-boot.vfat", i.released.trim_end_matches(".img"))),
    );
    assert_eq!(listed, want);
    // No desktop file in the release folder: the images, the boot
    // programs, the list and the sums.
    let mut files: Vec<String> = IMAGES
        .iter()
        .map(|i| format!("{}.xz", i.released))
        .collect();
    files.extend([BOOT, "IMAGES", "SHA256SUMS"].map(String::from));
    files.sort();
    assert_eq!(names(&out), files);
    // The sums check out from the release folder, as a person runs them.
    let checked = Command::new("sha256sum")
        .args(["-c", "--quiet", "--ignore-missing", "SHA256SUMS"])
        .current_dir(&out)
        .output()
        .unwrap_or_err("sha256sum");
    assert!(checked.status.success(), "{checked:?}");

    // No payload folder is made beside the release.
    assert!(!root.join("payload").exists());
}

/// The release's image list: one line per image, its sums those of the
/// published `.xz` and of the image inside it, and its platform, screen,
/// build and boards; and a card or stick written from it checks against
/// `SHA256SUMS` by its boot partition alone, whatever its data partition
/// now holds.
#[test]
fn the_image_list_and_the_sums_check_each_image_and_a_written_card() {
    let root = fresh("list");
    let out = first_pass(&root);
    let list = fs::read_to_string(out.join("IMAGES")).expect("IMAGES");
    let mut lines = list.lines();
    assert!(
        lines
            .next()
            .expect("a first line")
            .starts_with(&format!("# Faraday {VERSION} images, commit ")),
        "{list}"
    );
    let lines: Vec<Vec<&str>> = lines.map(|l| l.split('\t').collect()).collect();
    assert_eq!(lines.len(), IMAGES.len(), "{list}");
    let sums = sums(&out);
    for (line, image) in lines.iter().zip(&IMAGES) {
        let xz = out.join(format!("{}.xz", image.released));
        let inner = decompressed(&xz);
        assert_eq!(line[0], format!("{}.xz", image.released));
        assert_eq!(
            line[1],
            sha256(&fs::read(&xz).expect("the .xz")),
            "{}",
            line[0]
        );
        assert_eq!(line[2], sha256(&inner), "{}", line[0]);
        assert_eq!(line[3..6], image.listed, "{}", line[0]);
        assert_eq!(line[6], inner.len().to_string(), "{}", line[0]);
        assert_eq!(line[7], format!("{VERSION}-test"), "{}", line[0]);
        let want_boot = if image.released.starts_with("faraday-stick-") {
            sha256(boot_partition(&inner))
        } else {
            "-".to_string()
        };
        assert_eq!(line[8], want_boot, "{}", line[0]);

        let mut card = inner.clone();
        let data = card.len() - 4096;
        card[data..].fill(0x77);
        let boot = format!("{}-boot.vfat", image.released.trim_end_matches(".img"));
        assert!(
            sums.contains(&(sha256(boot_partition(&card)), boot.clone())),
            "{boot} is not in SHA256SUMS"
        );
    }
}

/// Images that carry two build IDs are not one release: nothing is staged,
/// and the refusal names each image's.
#[test]
fn staging_refuses_images_of_different_builds() {
    let root = fresh("two-builds");
    let builds = builds_with(&root, &kernel(), "6.6.84-faraday-0.8.0-test");
    let done = stage_builds(&root, &builds);
    assert!(!done.status.success());
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("not one build"), "{said}");
    assert!(said.contains("pi3-any 0.8.0-test"), "{said}");
    assert!(said.contains("stick 0.9.0-test"), "{said}");
    assert!(!root.join("release").exists(), "nothing staged");
}

/// The boot programs a release publishes (F134): each stick image's
/// `EFI/BOOT/BOOTX64.EFI` byte for byte, beside the images and listed in
/// `SHA256SUMS` with its SHA-256, so the sums Faraday checks before
/// signing it name it.
#[test]
fn staging_publishes_the_boot_program_and_lists_it_in_the_sums() {
    let root = fresh("boot");
    let out = first_pass(&root);
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
}

#[test]
fn staging_refuses_a_boot_program_of_another_build() {
    let root = fresh("other-build");
    let builds = builds_with(&root, &kernel_of("6.6.84-faraday-0.8.0-test"), RELEASE);
    let out = root.join("release");
    let done = stage_builds(&root, &builds);
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
    let done = stage_builds(&root, &builds);
    assert!(!done.status.success());
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("has no GPT"), "{said}");
    assert!(!out.exists(), "nothing staged");
}

#[test]
fn staging_refuses_a_release_folder_holding_a_desktop_app_and_names_it() {
    let root = fresh("refused");
    let out = first_pass(&root);
    // A desktop app left in the release folder, which is not published.
    let left = out.join(format!("faraday-demo-{VERSION}-testnet-only-linux-x86_64"));
    fs::write(&left, "a desktop app").expect("file");
    let starter = starter_with(&root, &out);

    let done = stage(&[
        VERSION,
        "--starter",
        starter.to_str().expect("a path"),
        "--out",
        out.to_str().expect("a path"),
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

/// A Starter built against the release's image list carries it; one built
/// without it, or against another release's, is not added. What the
/// second pass adds is listed in `SHA256SUMS`.
#[test]
fn a_starter_is_added_only_if_it_carries_this_releases_image_list() {
    let root = fresh("second");
    let out = first_pass(&root);
    let tar = format!("Faraday-Starter-{VERSION}-Linux.tar.gz");

    let bare = root.join("bare-starter");
    fs::write(&bare, "a Starter built without an image list\n").expect("starter");
    let done = stage(&[
        VERSION,
        "--starter",
        bare.to_str().expect("a path"),
        "--out",
        out.to_str().expect("a path"),
    ]);
    assert!(!done.status.success());
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("image list"), "{said}");
    assert!(!out.join(&tar).exists());

    let starter = starter_with(&root, &out);
    let done = stage(&[
        VERSION,
        "--starter",
        starter.to_str().expect("a path"),
        "--out",
        out.to_str().expect("a path"),
    ]);
    assert!(
        done.status.success(),
        "adding the Starter failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(out.join(&tar).is_file());
    assert!(sums(&out).iter().any(|(_, n)| *n == tar));
    assert!(!root.join("payload").exists());
}

/// A stand-in Starter built against the release's `IMAGES`: it holds
/// the list's first line, which names the release and the commit.
fn starter_with(root: &Path, out: &Path) -> PathBuf {
    let first = fs::read_to_string(out.join("IMAGES"))
        .expect("IMAGES")
        .lines()
        .next()
        .expect("a first line")
        .to_string();
    let starter = root.join("faraday-starter");
    fs::write(&starter, format!("a Starter\n{first}\n")).expect("starter");
    starter
}

/// A trial staging (`--any-build`) stages no cross-check record, even
/// one that a real `just crosscheck --fresh` left where staging reads it.
#[test]
fn a_trial_staging_takes_no_cross_check_record() {
    let root = fresh("record");
    let builds = builds(&root);
    let record = root.join("crosscheck-record.txt");
    fs::write(&record, "commit 0\nresult: every answer agreed\n").expect("file");
    let out = root.join("release");
    let done = stage(&[
        VERSION,
        "--any-build",
        "--crosscheck",
        record.to_str().expect("a path"),
        "--builds",
        builds.to_str().expect("a path"),
        "--out",
        out.to_str().expect("a path"),
    ]);
    assert!(
        done.status.success(),
        "staging failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(!names(&out).iter().any(|n| n.starts_with("crosscheck-")));
    assert!(!sums(&out).iter().any(|(_, n)| n.starts_with("crosscheck-")));
}
