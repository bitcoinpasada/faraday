//! What a stick the Starter writes holds, judged by programs Faraday did
//! not write, where they are installed: `xz` and 7-Zip decode the same
//! `.xz` streams as Faraday's decoder (same bytes, or all refuse); gdisk's
//! `sgdisk -v` and util-linux's `sfdisk --verify` find no problem in
//! every table the Starter writes, GPT and MBR; CPython's `plistlib`
//! reads the `diskutil` answers as the plist reader does; CPython's
//! `uuid` finds the GUIDs written on a stick to be version 4 and RFC 9562
//! variant; CPython's `zlib.crc32` finds the header and partition-array CRCs of
//! a GPT the Starter writes (both copies) to be CRC-32 of the bytes they
//! cover. Without a tool a test is skipped with a note, and fails when
//! `FARADAY_JUDGES_REQUIRED` is set.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use faraday_payload::ImageKind;
use faraday_starter::api::Mode;
use faraday_starter_shell::gpt;
use faraday_starter_shell::macos::plist;
use faraday_starter_shell::writer::Job;

use crate::ids::{runs, tool};
use crate::sticks::{SECTOR, STICK, Sparse, card_image, read, sha, stick_image};
use crate::writer::{FileDisk, image_job, release_image, run, tmp};

/// 7-Zip's own program, `7zz` or the name some systems give it, `7z`; not
/// p7zip, the old fork, whose banner says so.
fn sevenzip() -> Option<PathBuf> {
    ["7zz", "7z"].iter().map(|n| tool(n)).find(|p| {
        let Ok(out) = Command::new(p).output() else {
            return false;
        };
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        let text = text.trim_start();
        text.starts_with("7-Zip ")
            && !text.starts_with("7-Zip [")
            && text.contains("Igor Pavlov")
            && !text.contains("p7zip")
    })
}

/// Whether every tool in `need` is installed; if not, says so, and
/// panics when the judges are required.
fn have(need: &[&str]) -> bool {
    let gone: Vec<&str> = need
        .iter()
        .copied()
        .filter(|n| match *n {
            "7-Zip" => sevenzip().is_none(),
            "python3" => !runs("python3", &["--version"]),
            "xz" => !runs("xz", &["--version"]),
            other => !runs(other, &["--version"]),
        })
        .collect();
    if gone.is_empty() {
        return true;
    }
    let list = gone.join(", ");
    eprintln!("{list} is not installed: the check against it is skipped");
    if std::env::var_os("FARADAY_JUDGES_REQUIRED").is_some() {
        panic!("FARADAY_JUDGES_REQUIRED is set and {list} is not installed");
    }
    false
}

fn text_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr)
}

// ---- the .xz decoders ------------------------------------------------------

/// Bytes from a fixed seed, so every run judges the same streams.
fn noise(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

/// What each of the three decoders made of a stream: the SHA-256 and
/// size of the bytes it gave, or `None` where it refused.
type Verdict = Option<String>;

fn faraday_decodes(stream: &[u8]) -> Verdict {
    faraday_payload::hash_image(Cursor::new(stream), ImageKind::Xz)
        .ok()
        .map(|s| format!("{} {}", s.sha256, s.size))
}

fn decoded_by(program: &Path, args: &[&str], file: &Path) -> Verdict {
    let out = Command::new(program)
        .args(args)
        .arg(file)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("a decoder");
    out.status
        .success()
        .then(|| format!("{} {}", sha(&out.stdout), out.stdout.len()))
}

#[test]
fn xz_streams_good_and_damaged_are_decoded_or_refused_as_xz_and_7_zip_do() {
    if !have(&["xz", "7-Zip"]) {
        return;
    }
    let seven = sevenzip().expect("7-Zip");
    let dir = tmp("outside-xz");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    let mut payloads: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![7],
        noise(1, 1000),
        noise(2, 70_000),
        noise(3, 300_000),
        b"stick image ".repeat(20_000),
        vec![0; 1 << 20],
    ];
    let mut bases: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, data) in payloads.drain(..).enumerate() {
        let raw = dir.join(format!("raw-{i}"));
        fs::write(&raw, &data).expect("written");
        for (preset, check) in [
            ("-0", "crc32"),
            ("-6", "crc64"),
            ("-1", "sha256"),
            ("-3", "none"),
        ] {
            let made = Command::new("xz")
                .args(["-c", preset, &format!("--check={check}")])
                .arg(&raw)
                .output()
                .expect("xz");
            assert!(made.status.success());
            bases.push((format!("{i}{preset}{check}"), made.stdout));
        }
    }
    // Each stream whole, cut short, with one byte changed, and with a
    // second stream or other bytes after it.
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, s) in &bases {
        cases.push((format!("{name} whole"), s.clone()));
        let n = s.len();
        let mut cuts = vec![0, 1, 6, 11, 12, 13, n / 2, n.saturating_sub(13)];
        cuts.extend([n.saturating_sub(12), n.saturating_sub(1)]);
        for cut in cuts {
            if cut < n {
                cases.push((format!("{name} cut at {cut}"), s[..cut].to_vec()));
            }
        }
        let seed = n as u64 * 31 + 5;
        let spots = noise(seed, 24);
        for (k, b) in spots.iter().enumerate() {
            let at = (usize::from(*b) * 251 + k * 4099) % n;
            let mut c = s.clone();
            c[at] ^= 1 << (k % 8);
            cases.push((format!("{name} bit {} of byte {at}", k % 8), c));
        }
        let mut two = s.clone();
        two.extend_from_slice(s);
        cases.push((format!("{name} twice"), two));
        let mut more = s.clone();
        more.extend_from_slice(b"junk");
        cases.push((format!("{name} then junk"), more));
        let mut zeros = s.clone();
        zeros.extend_from_slice(&[0; 4]);
        cases.push((format!("{name} then four zero bytes"), zeros));
    }
    let mut differ = Vec::new();
    let (mut good, mut refused) = (0, 0);
    for (n, (name, stream)) in cases.iter().enumerate() {
        let file = dir.join(format!("case-{n}.xz"));
        fs::write(&file, stream).expect("written");
        let ours = faraday_decodes(stream);
        let xz = decoded_by(Path::new("xz"), &["-dc"], &file);
        let seven_z = decoded_by(&seven, &["x", "-so", "-bd", "-y"], &file);
        if ours.is_some() {
            good += 1;
        } else {
            refused += 1;
        }
        if ours != xz || ours != seven_z {
            differ.push(format!(
                "{name}: Faraday {ours:?}, xz {xz:?}, 7-Zip {seven_z:?}"
            ));
        }
        fs::remove_file(&file).expect("removed");
    }
    assert!(good > 20 && refused > 100, "{good} read, {refused} refused");
    assert!(
        differ.is_empty(),
        "{} differ:\n{}",
        differ.len(),
        differ.join("\n")
    );
}

// ---- the partition tables --------------------------------------------------

fn write_stick(name: &str, size: u64) -> PathBuf {
    let image = stick_image();
    let (job, source) = release_image(&image);
    let path = tmp(name);
    Sparse::new(size).write(&path);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::Whole);
    assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
    path
}

fn write_card(name: &str) -> PathBuf {
    let image = card_image(0x5A5A_0001);
    let (img, source) = image_job(&format!("{name}.img"), &image);
    let path = tmp(name);
    Sparse::new(STICK).write(&path);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::Whole);
    assert_eq!(run(&order, &Job::Image(img), &mut disk).0, Ok(()));
    path
}

#[test]
fn the_tables_written_on_sticks_and_cards_pass_sgdisk_and_sfdisk() {
    if !have(&["sgdisk", "sfdisk"]) {
        return;
    }
    let mut disks = vec![
        (
            "a Faraday stick of 1 GB",
            write_stick("outside-gpt-1", 1 << 30),
        ),
        (
            "a Faraday stick of 256 GB",
            write_stick("outside-gpt-256", 256 << 30),
        ),
        ("a Faraday stick", write_stick("outside-gpt", STICK)),
    ];
    disks.push(("a Pi card, MBR", write_card("outside-mbr")));
    for (what, path) in &disks {
        let sf = Command::new(tool("sfdisk"))
            .arg("--verify")
            .arg(path)
            .output()
            .expect("sfdisk");
        let sg = Command::new(tool("sgdisk"))
            .arg("-v")
            .arg(path)
            .output()
            .expect("sgdisk");
        let (sf_said, sg_said) = (text_of(&sf), text_of(&sg));
        assert!(
            sf.status.success() && sf_said.contains("No errors"),
            "{what}: sfdisk: {sf_said}"
        );
        assert!(
            sg.status.success() && sg_said.contains("No problems found"),
            "{what}: sgdisk: {sg_said}"
        );
    }
}

/// CPython's `zlib.crc32` over a GPT header (its CRC field as zero) and
/// the partition array it names, against the two CRC fields. Each
/// argument is `<header hex>:<array hex>`.
const GPT_CRCS: &str = r#"
import sys, zlib, struct
for arg in sys.argv[1:]:
    header, array = (bytes.fromhex(x) for x in arg.split(":"))
    assert header[:8] == b"EFI PART", "signature"
    size = struct.unpack_from("<I", header, 12)[0]
    crc = struct.unpack_from("<I", header, 16)[0]
    zeroed = header[:16] + bytes(4) + header[20:size]
    assert zlib.crc32(zeroed) == crc, "header CRC"
    count, entry, array_crc = struct.unpack_from("<III", header, 80)
    assert len(array) == count * entry, "array length"
    assert zlib.crc32(array) == array_crc, "array CRC"
    print("ok")
"#;

#[test]
fn the_crcs_in_the_gpt_written_on_a_stick_are_crc_32_as_python_works_it_out() {
    if !have(&["python3"]) {
        return;
    }
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let mut args = Vec::new();
    for (name, size) in [("outside-crc-1", 1u64 << 30), ("outside-crc", STICK)] {
        let path = write_stick(name, size);
        let u32_at = |b: &[u8], i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        let u64_at = |b: &[u8], i: usize| u64::from_le_bytes(b[i..i + 8].try_into().unwrap());
        // The primary header at LBA 1 and the backup where it says.
        let primary = read(&path, SECTOR, 92);
        let backup = read(&path, u64_at(&primary, 32) * SECTOR, 92);
        for header in [primary, backup] {
            let array_at = u64_at(&header, 72) * SECTOR;
            let len = u64::from(u32_at(&header, 80)) * u64::from(u32_at(&header, 84));
            let array = read(&path, array_at, len as usize);
            args.push(format!("{}:{}", hex(&header), hex(&array)));
        }
    }
    let out = Command::new("python3")
        .args(["-I", "-c", GPT_CRCS])
        .args(&args)
        .output()
        .expect("python3");
    let said = text_of(&out);
    assert!(out.status.success(), "{said}");
    assert_eq!(
        said.lines().filter(|l| *l == "ok").count(),
        args.len(),
        "{said}"
    );
}

// ---- the property lists ----------------------------------------------------

/// Every `diskutil ... -plist` answer in the fixture files, by title.
fn diskutil_answers() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for file in ["intel-mac.txt", "usb-start.txt"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/macos")
            .join(file);
        let text = fs::read_to_string(path).expect("answers");
        let mut current: Option<(String, String)> = None;
        for line in text.lines() {
            if let Some(title) = line.strip_prefix("=== ") {
                out.extend(current.take());
                current = title
                    .contains("-plist")
                    .then(|| (format!("{file}: {title}"), String::new()));
            } else if let Some((_, body)) = current.as_mut() {
                body.push_str(line);
                body.push('\n');
            }
        }
        out.extend(current);
    }
    out
}

/// A value written as the Python below writes it.
fn canon(v: &plist::Value) -> String {
    use plist::Value;
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    match v {
        Value::Dict(d) => {
            let items: Vec<String> = d
                .iter()
                .map(|(k, v)| format!("{}={}", hex(k.as_bytes()), canon(v)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
        Value::Array(a) => format!("[{}]", a.iter().map(canon).collect::<Vec<_>>().join(",")),
        Value::String(s) => format!("s:{}", hex(s.as_bytes())),
        Value::Integer(i) => format!("i:{i}"),
        Value::Real(r) => format!("r:{:016x}", r.to_bits()),
        Value::Bool(b) => format!("b:{}", u8::from(*b)),
        Value::Data(d) => format!("d:{}", hex(d)),
        Value::Date(d) => format!("D:{d}"),
    }
}

const PLISTLIB: &str = r#"
import datetime, os, plistlib, struct, sys
def enc(v):
    if isinstance(v, bool): return 'b:%d' % v
    if isinstance(v, int): return 'i:%d' % v
    if isinstance(v, float): return 'r:' + struct.pack('>d', v).hex()
    if isinstance(v, str): return 's:' + v.encode().hex()
    if isinstance(v, bytes): return 'd:' + v.hex()
    if isinstance(v, datetime.datetime): return 'D:' + v.strftime('%Y-%m-%dT%H:%M:%SZ')
    if isinstance(v, list): return '[' + ','.join(enc(x) for x in v) + ']'
    if isinstance(v, dict): return '{' + ','.join(k.encode().hex() + '=' + enc(v[k]) for k in sorted(v)) + '}'
    raise ValueError(type(v))
for name in sorted(os.listdir(sys.argv[1])):
    data = open(os.path.join(sys.argv[1], name), 'rb').read()
    try:
        print(name + '\t' + enc(plistlib.loads(data, fmt=plistlib.FMT_XML)))
    except Exception:
        print(name + '\trefused')
"#;

#[test]
fn the_answers_diskutil_gives_are_read_as_python_reads_them_or_refused_by_both() {
    if !have(&["python3"]) {
        return;
    }
    let dir = tmp("outside-plist");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    let mut inputs: Vec<String> = Vec::new();
    for (_, body) in diskutil_answers() {
        // The answer whole, and cut short at a spread of places.
        let end = body.rfind("</plist>").expect("closed");
        for at in (0..end).step_by(61) {
            inputs.push(body[..at].to_string());
        }
        inputs.push(body.replace("<integer>", "<integer>5x"));
        inputs.push(body.replace("&amp;", "&nbsp;"));
        inputs.push(body);
    }
    for (n, text) in inputs.iter().enumerate() {
        fs::write(dir.join(format!("{n:06}")), text).expect("written");
    }
    let out = Command::new("python3")
        .args(["-I", "-c", PLISTLIB])
        .arg(&dir)
        .output()
        .expect("python3");
    assert!(out.status.success(), "{}", text_of(&out));
    let python = String::from_utf8_lossy(&out.stdout).to_string();
    let lines: Vec<&str> = python.lines().collect();
    assert_eq!(lines.len(), inputs.len());
    let (mut read_by_both, mut refused_by_both) = (0, 0);
    for (n, (text, line)) in inputs.iter().zip(&lines).enumerate() {
        let theirs = line.split_once('\t').expect("a result").1;
        let ours = plist::parse(text).ok().map(|v| canon(&v));
        match (&ours, theirs) {
            (None, "refused") => refused_by_both += 1,
            (Some(a), b) if a == b => read_by_both += 1,
            _ => panic!("input {n}: Faraday {ours:?}, plistlib {theirs}\n{text}"),
        }
    }
    assert!(read_by_both >= 10 && refused_by_both > 50);
}

// ---- the GUIDs -------------------------------------------------------------

const UUIDS: &str = r#"
import sys, uuid
bad = 0
for arg in sys.argv[1:]:
    raw, shown = arg.split('=')
    u = uuid.UUID(bytes_le=bytes.fromhex(raw))
    ok = u.version == 4 and u.variant == uuid.RFC_4122 and str(u).upper() == shown
    print(shown, 'ok' if ok else 'BAD %s version %s variant %s' % (u, u.version, u.variant))
"#;

#[test]
fn the_ids_drawn_for_a_stick_are_version_4_uuids_as_python_reads_them() {
    if !have(&["python3"]) {
        return;
    }
    let mut args: Vec<String> = Vec::new();
    for n in 0..6 {
        let path = write_stick(&format!("outside-uuid-{n}"), STICK);
        let file = fs::File::open(&path).expect("open");
        let table = gpt::read(
            &mut |at, buf| {
                use std::os::unix::fs::FileExt;
                file.read_exact_at(buf, at)
            },
            STICK,
        )
        .expect("a GPT whose CRCs hold");
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        // As the disk holds them: the header's disk GUID, each entry's
        // unique GUID, in the mixed-endian form of the GPT.
        args.push(format!(
            "{}={}",
            hex(&read(&path, SECTOR + 56, 16)),
            table.disk_guid
        ));
        for (i, part) in table.parts.iter().enumerate() {
            let at = 2 * SECTOR + 128 * i as u64 + 16;
            args.push(format!("{}={}", hex(&read(&path, at, 16)), part.guid));
        }
    }
    let out = Command::new("python3")
        .args(["-I", "-c", UUIDS])
        .args(&args)
        .output()
        .expect("python3");
    let said = text_of(&out);
    assert!(out.status.success(), "{said}");
    assert!(
        said.lines().count() == args.len() && said.lines().all(|l| l.ends_with(" ok")),
        "{said}"
    );
    let mut shown: Vec<&str> = said.lines().collect();
    shown.sort_unstable();
    shown.dedup();
    assert_eq!(shown.len(), args.len(), "every ID differs");
}
