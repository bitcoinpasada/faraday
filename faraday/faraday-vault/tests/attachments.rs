//! Attachments at a seal (`docs/VAULT.md` §7.1): one nothing refers to any
//! more is emptied in place and the trailing ones are left off, so an entry
//! deleted takes its attachment out of the file; every one still referred
//! to, by an entry, a history entry or a recycle bin entry, is kept.

use std::path::PathBuf;

use faraday_crypto::work::Compute;
use faraday_vault::file::{self, Change, Error, Opened, SIZES};
use faraday_vault::kdbx::items::{Body, Place};
use faraday_vault::kdbx::xml::Document;
use faraday_vault::kdbx::{self, Binaries, Binary, RANDOM_LEN, items};
use faraday_vault::records::{field, kind};
use faraday_vault::{Cost, Record};

const PASSPHRASE: &str = "attachments passphrase";
const CHEAP: Cost = Cost {
    memory_kib: 8 * 1024,
    passes: 1,
    lanes: 1,
};
const A: [u8; 16] = [1; 16];
const B: [u8; 16] = [2; 16];
const C: [u8; 16] = [3; 16];
const A64: &str = "AQEBAQEBAQEBAQEBAQEBAQ==";
const B64: &str = "AgICAgICAgICAgICAgICAg==";
const C64: &str = "AwMDAwMDAwMDAwMDAwMDAw==";
const D64: &str = "BAQEBAQEBAQEBAQEBAQEBA==";
/// The `RecycleBinUUID` of a new document.
const BIN64: &str = "AAAAAAAAAAAAAAAAAAAAAA==";

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn random(seed: u8) -> [u8; RANDOM_LEN] {
    std::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(seed))
}

fn data(n: u8) -> Vec<u8> {
    format!("attachment number {n} with bytes only it has").into_bytes()
}

fn binary(n: u8, protected: bool) -> Binary {
    Binary {
        protected,
        data: zeroize::Zeroizing::new(data(n)),
    }
}

/// An entry titled `title` with the attachments `refs` (file names `fN`).
fn entry(uuid: &str, title: &str, refs: &[&str], history: &str) -> String {
    let mut s = format!(
        "<Entry><UUID>{uuid}</UUID><String><Key>Title</Key><Value>{title}</Value></String>"
    );
    for (i, r) in refs.iter().enumerate() {
        s += &format!("<Binary><Key>f{i}-{title}</Key><Value {r}/></Binary>");
    }
    if !history.is_empty() {
        s += &format!("<History>{history}</History>");
    }
    s + "</Entry>"
}

fn by_ref(n: usize) -> String {
    format!("Ref=\"{n}\"")
}

/// A vault file whose root group holds `entries` and whose recycle bin
/// holds `bin`, with `binaries`, at `size`.
fn vault(entries: &str, bin: &str, binaries: &[Binary], size: usize) -> Vec<u8> {
    let mut doc = Document::new([9; 16], "Attached");
    items::stamp(&mut doc, 1, None).unwrap();
    let random = random(3);
    let text = String::from_utf8(doc.write(kdbx::inner_key_of(&random)).to_vec()).unwrap();
    let bin = if bin.is_empty() {
        String::new()
    } else {
        format!("<Group><UUID>{BIN64}</UUID><Name>Recycle Bin</Name>{bin}</Group>")
    };
    let text = text.replace("</Group></Root>", &format!("{entries}{bin}</Group></Root>"));
    assert!(text.contains(entries), "the entries went in");
    let contents = kdbx::Contents {
        document: text.as_bytes(),
        binaries: Binaries::all(binaries),
        public_custom_data: None,
    };
    kdbx::seal(&compute(), PASSPHRASE, CHEAP, &random, &contents, size).unwrap()
}

fn open(file: &[u8]) -> Opened {
    file::open(&compute(), file, PASSPHRASE).expect("opens")
}

/// What a seal of `file` with `changes` writes: the attachments (protected,
/// bytes) and the document, read back from the file.
type Sealed = (Vec<u8>, Vec<(bool, Vec<u8>)>, String);

fn sealed(file: &[u8], changes: &[Change<'_>]) -> Sealed {
    let o = open(file);
    let out = file::seal(&compute(), &o, changes, &random(5), None).expect("seals");
    let k = kdbx::open(&compute(), &out, PASSPHRASE).expect("opens again");
    let held = k
        .binaries
        .iter()
        .map(|b| (b.protected, b.data.to_vec()))
        .collect();
    let doc = Document::read(&k.document, &k.inner_key).expect("reads");
    let text = String::from_utf8(doc.write(&[0u8; 64]).to_vec()).unwrap();
    (out, held, text)
}

fn three() -> Vec<Binary> {
    vec![binary(0, false), binary(1, true), binary(2, false)]
}

/// Whether `needle` is anywhere in `hay`.
fn has(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn an_entry_deleted_takes_its_attachment_out_of_the_file_and_the_others_still_read() {
    let entries = [
        entry(A64, "first", &[&by_ref(0)], ""),
        entry(B64, "second", &[&by_ref(1)], ""),
        entry(C64, "third", &[&by_ref(2)], ""),
    ]
    .concat();
    let file = vault(&entries, "", &three(), SIZES[0]);
    // Deleting the last one cuts it off.
    let (_, held, _) = sealed(&file, &[Change::Remove(C)]);
    assert_eq!(held.len(), 2);
    assert!(!held.iter().any(|(_, d)| *d == data(2)));
    // Deleting a middle one empties it in place, protection kept.
    let (out, held, _) = sealed(&file, &[Change::Remove(B)]);
    assert_eq!(held.len(), 3);
    assert_eq!(held[0], (false, data(0)));
    assert_eq!(held[1], (true, Vec::new()));
    assert_eq!(held[2], (false, data(2)));
    // The document says what it said of the others.
    let k = kdbx::open(&compute(), &out, PASSPHRASE).unwrap();
    assert!(!has(&k.document, b"second"));
    assert!(!k.binaries.iter().any(|b| has(&b.data, &data(1))));
    if let Some(text) = pykeepass_reads(&out) {
        assert!(text.contains("attachment: f0-first="), "{text}");
        assert!(
            text.contains(&format!("attachment: f0-third={}", hex(&data(2)))),
            "{text}"
        );
        assert!(
            text.contains(&format!("f0-first={}", hex(&data(0)))),
            "{text}"
        );
        assert!(!text.contains(&hex(&data(1))), "{text}");
    }
}

#[test]
fn an_attachment_still_used_by_another_entry_or_a_history_or_the_bin_is_kept() {
    let shared = [
        entry(A64, "first", &[&by_ref(0)], ""),
        entry(B64, "second", &[&by_ref(0), &by_ref(1)], ""),
    ]
    .concat();
    let file = vault(&shared, "", &three()[..2], SIZES[0]);
    let (_, held, _) = sealed(&file, &[Change::Remove(B)]);
    assert_eq!(
        held,
        vec![(false, data(0))],
        "the shared one stays, the other goes"
    );

    // Used by the history of an entry that stays.
    let old = entry(D64, "old", &[&by_ref(1)], "");
    let hist = [
        entry(A64, "first", &[&by_ref(0)], &old),
        entry(B64, "second", &[&by_ref(0)], ""),
    ]
    .concat();
    let file = vault(&hist, "", &three()[..2], SIZES[0]);
    let (_, held, _) = sealed(&file, &[Change::Remove(B)]);
    assert_eq!(held.len(), 2);
    assert_eq!(held[1], (true, data(1)));

    // Used only by an entry in the recycle bin.
    let kept = entry(A64, "first", &[&by_ref(0)], "");
    let binned = entry(D64, "binned", &[&by_ref(1)], "");
    let file = vault(
        &[kept, entry(B64, "second", &[&by_ref(2)], "")].concat(),
        &binned,
        &three(),
        SIZES[0],
    );
    let (_, held, _) = sealed(&file, &[Change::Remove(B)]);
    assert_eq!(held, vec![(false, data(0)), (true, data(1))]);
}

#[test]
fn a_reference_that_is_not_understood_keeps_every_attachment() {
    let ok = entry(A64, "first", &[&by_ref(0)], "");
    for odd in [
        "Ref=\"x\"".to_string(),
        "Ref=\" 1\"".to_string(),
        "Ref=\"-1\"".to_string(),
        "Ref=\"\"".to_string(),
    ] {
        let entries = [ok.clone(), entry(B64, "second", &[&odd], "")].concat();
        let file = vault(&entries, "", &three(), SIZES[0]);
        let (_, held, _) = sealed(&file, &[Change::Remove(C)]);
        assert_eq!(held.len(), 3, "{odd}");
        assert_eq!(held[2], (false, data(2)), "{odd}");
    }
    // A Binary that is not a Key and a Value with a Ref.
    for odd in [
        "<Binary/>",
        "<Binary><Key>k</Key></Binary>",
        "<Binary><Key>k</Key><Value>AAAA</Value></Binary>",
        "<Binary><Key>k</Key><Value Ref=\"0\"/><Value Ref=\"1\"/></Binary>",
        "<Binary><Key>k</Key><Other/></Binary>",
    ] {
        let weird = format!("<Entry><UUID>{B64}</UUID>{odd}</Entry>");
        let file = vault(&[ok.clone(), weird].concat(), "", &three(), SIZES[0]);
        let (_, held, _) = sealed(&file, &[Change::Remove(C)]);
        assert_eq!(held.len(), 3, "{odd}");
    }
}

#[test]
fn a_reference_in_another_spelling_or_to_nothing_is_read_right() {
    // Entity-encoded digits and spaces around `=` read as the number.
    let spelled = format!(
        "<Entry><UUID>{A64}</UUID><Binary><Key>k</Key><Value  Ref = '&#49;' /></Binary></Entry>"
    );
    let file = vault(&spelled, "", &three(), SIZES[0]);
    let (_, held, _) = sealed(&file, &[]);
    assert_eq!(held.len(), 2);
    assert_eq!(held[0], (false, Vec::new()));
    assert_eq!(held[1], (true, data(1)));
    // A reference past the end changes nothing for the others.
    let past = entry(A64, "past", &[&by_ref(77), &by_ref(0)], "");
    let file = vault(&past, "", &three(), SIZES[0]);
    let (_, held, _) = sealed(&file, &[]);
    assert_eq!(held, vec![(false, data(0))]);
}

#[test]
fn a_file_with_an_attachment_nothing_refers_to_loses_it_at_the_next_seal() {
    let file = vault(
        &entry(A64, "first", &[&by_ref(1)], ""),
        "",
        &three(),
        SIZES[0],
    );
    let (_, held, _) = sealed(&file, &[]);
    assert_eq!(held, vec![(false, Vec::new()), (true, data(1))]);
}

#[test]
fn deleting_the_entry_with_a_large_attachment_makes_room_in_a_full_vault() {
    let size = SIZES[0];
    let big = Binary {
        protected: false,
        data: zeroize::Zeroizing::new(vec![0x5a; size - 12_000]),
    };
    let file = vault(&entry(A64, "big", &[&by_ref(0)], ""), "", &[big], size);
    let o = open(&file);
    let text = "x".repeat(20_000);
    let body = Body::Record(Record::new(kind::NOTE).with(field::NOTE, text.as_bytes()));
    let place = Place {
        uuid: [7; 16],
        group: "Notes",
        group_uuid: [200; 16],
        title: "Note",
        now: None,
    };
    let add = Change::Put { place, body: &body };
    let c = compute();
    assert!(matches!(
        file::seal(&c, &o, std::slice::from_ref(&add), &random(5), None),
        Err(Error::Full { .. })
    ));
    let full = o.used(&c, &[]).unwrap();
    let gone = o.used(&c, &[Change::Remove(A)]).unwrap();
    assert!(gone + size / 2 < full, "{gone} bytes against {full}");
    assert_eq!(o.next_size(&c, &[Change::Remove(A)], None).unwrap(), size);
    let changes = [Change::Remove(A), add];
    let out = file::seal(&c, &o, &changes, &random(5), None).expect("fits once it is gone");
    assert_eq!(out.len(), file.len());
    assert_eq!(open(&out).read().items.len(), 1);
}

/// What pykeepass reads of `file`, or none where it is not installed.
fn pykeepass_reads(file: &[u8]) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("faraday-attachments-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("vault.kdbx");
    std::fs::write(&path, file).unwrap();
    let out = std::process::Command::new("python3")
        .args(["-I", "-B"])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/kdbx/reread.py"))
        .arg(&path)
        .arg(PASSPHRASE)
        .arg("more")
        .output();
    let _ = std::fs::remove_dir_all(&dir);
    let out = out.ok()?;
    let err = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() && (err.contains("ModuleNotFoundError") || err.contains("ImportError"))
    {
        eprintln!("pykeepass is not installed: skipped");
        return None;
    }
    assert!(out.status.success(), "pykeepass: {err}");
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// `keepassxc-cli` with the passphrase on stdin; none where it is not
/// installed (said, not silent).
fn keepassxc(
    args: &[&str],
    file: &std::path::Path,
    after: &[&str],
) -> Option<std::process::Output> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("keepassxc-cli")
        .args(args)
        .arg(file)
        .args(after)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let _ = child
        .stdin
        .take()?
        .write_all(format!("{PASSPHRASE}\n").as_bytes());
    child.wait_with_output().ok()
}

#[test]
fn keepassxc_reads_a_file_with_an_attachment_emptied_in_the_middle() {
    let entries = [
        entry(A64, "first", &[&by_ref(0)], ""),
        entry(B64, "second", &[&by_ref(1)], ""),
        entry(C64, "third", &[&by_ref(2)], ""),
    ]
    .concat();
    let file = vault(&entries, "", &three(), SIZES[0]);
    let (out, held, _) = sealed(&file, &[Change::Remove(B)]);
    assert_eq!(held[1], (true, Vec::new()));
    let dir = std::env::temp_dir().join(format!("faraday-attachments-xc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("vault.kdbx");
    std::fs::write(&path, out).unwrap();
    let listed = keepassxc(&["ls", "-q", "-R"], &path, &[]);
    let Some(listed) = listed else {
        eprintln!("keepassxc-cli is not installed: skipped");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    };
    let text = String::from_utf8_lossy(&listed.stdout).into_owned();
    assert!(listed.status.success(), "{text} {:?}", listed.stderr);
    assert!(text.contains("first") && text.contains("third"), "{text}");
    for (title, n) in [("first", 0u8), ("third", 2)] {
        let name = format!("f0-{title}");
        let got = keepassxc(
            &["attachment-export", "-q", "--stdout"],
            &path,
            &[title, &name],
        )
        .expect("runs");
        assert!(got.status.success(), "{:?}", got.stderr);
        assert_eq!(got.stdout, data(n), "{title}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
